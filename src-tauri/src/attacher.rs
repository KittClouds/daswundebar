//! Dependency Attacher - Links chunks into a dependency graph (Native Tauri)
//!
//! Uses a non-greedy scoring strategy to resolve relationships:
//! - Subject-Verb (nsubj)
//! - Verb-Object (obj)
//! - PP-Attachment (nmod)
//!
//! # Universal Dependencies (Simplified)
//!
//! | Relation | From (Head) | To (Dep) | Example |
//! |----------|-------------|----------|---------|
//! | nsubj    | Verb        | Noun     | walked ← Frodo |
//! | obj      | Verb        | Noun     | carried → ring |
//! | nmod     | Noun        | Noun/PP  | ring → finger (on) |
//! | amod     | Noun        | AdjPh    | wizard ← ancient |
//! | advmod   | Verb        | AdvPh    | walked ← slowly |
//!
//! # Scoring Strategy
//!
//! Score = Proximity + SemanticFit + DirectionalPreference

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::chunker::{Chunk, ChunkKind, TextRange};

// =============================================================================
// Core Types
// =============================================================================

/// Types of dependencies between chunks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyKind {
    /// Nominal subject: "The **wizard** walked" (walked -> wizard)
    NSubj,
    /// Direct object: "found the **ring**" (found -> ring)
    Obj,
    /// Nominal modifier (usually PP): "walked **in the forest**"
    /// or "Book **of Spells**"
    NMod,
    /// Adjective modifier: "**Happy**, the wizard smiled"
    AMod,
    /// Adverbial modifier: "walked **slowly**"
    AdvMod,
    /// Root of the sentence (usually the main verb)
    Root,
    /// Conjunction / Other
    Unknown,
}

/// A resolved dependency link
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dependency {
    /// Index of the head chunk in the chunk list
    pub head_idx: usize,
    /// Index of the dependent chunk in the chunk list
    pub dep_idx: usize,
    /// The type of relationship
    pub kind: DependencyKind,
    /// The confidence score of this attachment
    pub score: f64,
}

impl Dependency {
    pub fn new(head_idx: usize, dep_idx: usize, kind: DependencyKind, score: f64) -> Self {
        Dependency {
            head_idx,
            dep_idx,
            kind,
            score,
        }
    }
}

/// Parsed sentence with dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub chunks: Vec<Chunk>,
    pub dependencies: Vec<Dependency>,
    pub root_idx: Option<usize>,
}

// =============================================================================
// Attacher Implementation
// =============================================================================

/// Dependency Attacher - builds dependency graphs from chunks
pub struct Attacher {
    proximity_weight: f64,
    #[allow(dead_code)]
    semantic_weight: f64,
}

impl Default for Attacher {
    fn default() -> Self {
        Self::new()
    }
}

impl Attacher {
    pub fn new() -> Self {
        Attacher {
            proximity_weight: 1.0,
            semantic_weight: 1.0,
        }
    }

    /// Build dependency graph from chunks
    pub fn attach(&self, chunks: &[Chunk]) -> DependencyGraph {
        let mut dependencies = Vec::new();
        let mut attached_indices = HashSet::new();

        // 1. Find Root (usually the first main verb phrase)
        let root_idx = chunks
            .iter()
            .position(|c| c.kind == ChunkKind::VerbPhrase);

        // 2. For each chunk, find its best head
        for (i, chunk) in chunks.iter().enumerate() {
            // Skip the root itself
            if Some(i) == root_idx {
                continue;
            }

            // Find best attachment
            if let Some(best_dep) = self.find_best_attachment(i, chunk, chunks, &attached_indices) {
                attached_indices.insert(best_dep.dep_idx);
                dependencies.push(best_dep);
            }
        }

        DependencyGraph {
            chunks: chunks.to_vec(),
            dependencies,
            root_idx,
        }
    }

    /// Score all potential parents and return the best one
    fn find_best_attachment(
        &self,
        child_idx: usize,
        child: &Chunk,
        chunks: &[Chunk],
        _attached: &HashSet<usize>,
    ) -> Option<Dependency> {
        let mut best_score = -1.0;
        let mut best_dep = None;

        for (parent_idx, parent) in chunks.iter().enumerate() {
            if child_idx == parent_idx {
                continue;
            }

            let score = self.score_attachment(child_idx, child, parent_idx, parent);
            let kind = self.determine_relation(child.kind, parent.kind, child_idx < parent_idx);

            if let Some(k) = kind {
                if score > best_score {
                    best_score = score;
                    best_dep = Some(Dependency::new(parent_idx, child_idx, k, score));
                }
            }
        }

        best_dep
    }

    /// The Core Scoring Function
    fn score_attachment(
        &self,
        child_idx: usize,
        child: &Chunk,
        parent_idx: usize,
        parent: &Chunk,
    ) -> f64 {
        let mut score = 0.0;

        // 1. Proximity: Closer is better
        let dist = (child_idx as i32 - parent_idx as i32).abs() as f64;
        score += (10.0 / dist) * self.proximity_weight;

        // 2. Directional Preference (English is largely SVO)
        let is_before = child_idx < parent_idx;

        match (child.kind, parent.kind) {
            // Subject usually comes before Verb
            (ChunkKind::NounPhrase, ChunkKind::VerbPhrase) => {
                if is_before {
                    score += 5.0;
                // Subject
                } else {
                    score += 4.0; // Object (Verb NP)
                }
            }
            // PP usually attaches to preceding Noun or Verb
            (ChunkKind::PrepPhrase, _) => {
                if !is_before {
                    score += 5.0;
                }
            }
            _ => {}
        }

        // 3. Semantic preference: PP prefers Verb over Noun if close
        if child.kind == ChunkKind::PrepPhrase && parent.kind == ChunkKind::VerbPhrase {
            score += 1.0;
        }

        score
    }

    /// Determine valid dependency kind based on chunk types and order
    fn determine_relation(
        &self,
        child_type: ChunkKind,
        parent_type: ChunkKind,
        child_is_before: bool,
    ) -> Option<DependencyKind> {
        match (parent_type, child_type) {
            // VERB -> NOUN
            (ChunkKind::VerbPhrase, ChunkKind::NounPhrase) => {
                if child_is_before {
                    Some(DependencyKind::NSubj)
                } else {
                    Some(DependencyKind::Obj)
                }
            }

            // VERB -> ADVERB (or ADJ phrase acting adverbially)
            (ChunkKind::VerbPhrase, ChunkKind::AdjPhrase) => Some(DependencyKind::AdvMod),

            // NOUN -> PP (Modifier) "Book of Spells"
            (ChunkKind::NounPhrase, ChunkKind::PrepPhrase) => Some(DependencyKind::NMod),

            // VERB -> PP (Modifier) "Walked in forest"
            (ChunkKind::VerbPhrase, ChunkKind::PrepPhrase) => Some(DependencyKind::NMod),

            // NOUN -> CLAUSE "The man who lived"
            (ChunkKind::NounPhrase, ChunkKind::Clause) => Some(DependencyKind::NMod),

            _ => None,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_chunk(kind: ChunkKind, _text: &str) -> Chunk {
        let range = TextRange::new(0, 10);
        Chunk::new(kind, range, range)
    }

    fn attach(chunks: &[Chunk]) -> DependencyGraph {
        let attacher = Attacher::new();
        attacher.attach(chunks)
    }

    #[test]
    fn test_simple_svo() {
        // "Frodo found the ring"
        let chunks = vec![
            mk_chunk(ChunkKind::NounPhrase, "Frodo"),
            mk_chunk(ChunkKind::VerbPhrase, "found"),
            mk_chunk(ChunkKind::NounPhrase, "the ring"),
        ];

        let graph = attach(&chunks);

        assert_eq!(graph.root_idx, Some(1));

        // Frodo -> found (nsubj)
        let sub = graph
            .dependencies
            .iter()
            .find(|d| d.dep_idx == 0)
            .expect("Frodo should be subject");
        assert_eq!(sub.head_idx, 1);
        assert_eq!(sub.kind, DependencyKind::NSubj);

        // ring -> found (obj)
        let obj = graph
            .dependencies
            .iter()
            .find(|d| d.dep_idx == 2)
            .expect("Ring should be object");
        assert_eq!(obj.head_idx, 1);
        assert_eq!(obj.kind, DependencyKind::Obj);
    }

    #[test]
    fn test_pp_attachment_verb() {
        // "Walked in the forest"
        let chunks = vec![
            mk_chunk(ChunkKind::VerbPhrase, "Walked"),
            mk_chunk(ChunkKind::PrepPhrase, "in the forest"),
        ];

        let graph = attach(&chunks);

        let pp = graph.dependencies.iter().find(|d| d.dep_idx == 1).unwrap();
        assert_eq!(pp.head_idx, 0);
        assert_eq!(pp.kind, DependencyKind::NMod);
    }

    #[test]
    fn test_pp_attachment_noun() {
        // "Book of spells"
        let chunks = vec![
            mk_chunk(ChunkKind::NounPhrase, "Book"),
            mk_chunk(ChunkKind::PrepPhrase, "of spells"),
        ];

        let graph = attach(&chunks);

        let pp = graph.dependencies.iter().find(|d| d.dep_idx == 1).unwrap();
        assert_eq!(pp.head_idx, 0);
        assert_eq!(pp.kind, DependencyKind::NMod);
    }

    #[test]
    fn test_ambiguity_preference() {
        // "The man saw the girl with the telescope"
        // Classic PP-attachment ambiguity
        let chunks = vec![
            mk_chunk(ChunkKind::NounPhrase, "The man"),
            mk_chunk(ChunkKind::VerbPhrase, "saw"),
            mk_chunk(ChunkKind::NounPhrase, "the girl"),
            mk_chunk(ChunkKind::PrepPhrase, "with the telescope"),
        ];

        let graph = attach(&chunks);

        // Expect PP(3) to attach to NP(2) due to proximity
        let pp = graph.dependencies.iter().find(|d| d.dep_idx == 3).unwrap();
        assert_eq!(pp.head_idx, 2, "Proximity should prefer 'girl' as head");
    }
}
