//! RelationEngine: CST + Graph-Based Relationship Extraction (Native Tauri)
//!
//! This module replaces the old pattern-dictionary-based RelationCortex.
//!
//! # Architecture (A+D Hybrid)
//!
//! ## Layer 1: Explicit Triples
//! Handled by `TripleCortex` - parses `[X] (REL) [Y]` and `[[X->REL->Y]]` syntax.
//!
//! ## Layer 2: CST Projection
//! Uses `StructuredRelationExtractor` to find Subject-Verb-Object patterns.
//!
//! ## Layer 3: Graph Inference
//! Uses graph algorithms to infer additional relationships.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use super::chunker::ChunkResult;
use super::structured_relation::StructuredRelationExtractor;

// =============================================================================
// Types
// =============================================================================

/// A detected relationship between two entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelation {
    pub head_entity: String,
    pub head_start: usize,
    pub head_end: usize,
    pub tail_entity: String,
    pub tail_start: usize,
    pub tail_end: usize,
    pub relation_type: String,
    pub pattern_matched: String,
    pub pattern_start: usize,
    pub pattern_end: usize,
    pub confidence: f64,
}

/// Entity span input for relation extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySpan {
    pub label: String,
    pub entity_id: Option<String>,
    pub start: usize,
    pub end: usize,
    pub kind: Option<String>,
}

/// Source of a relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationSource {
    /// From explicit syntax
    Explicit,
    /// From CST projection (SVO patterns)
    CST,
    /// From graph algorithms
    Inferred,
}

/// Unified relation with source tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedRelation {
    pub head: String,
    pub head_id: Option<String>,
    pub tail: String,
    pub tail_id: Option<String>,
    pub relation_type: String,
    pub source: RelationSource,
    pub confidence: f32,
    pub span: Option<(usize, usize)>,
    pub verb_text: Option<String>,
}

impl Default for UnifiedRelation {
    fn default() -> Self {
        Self {
            head: String::new(),
            head_id: None,
            tail: String::new(),
            tail_id: None,
            relation_type: String::new(),
            source: RelationSource::CST,
            confidence: 0.0,
            span: None,
            verb_text: None,
        }
    }
}

impl UnifiedRelation {
    /// Convert to backward-compatible ExtractedRelation
    pub fn to_extracted(&self) -> ExtractedRelation {
        ExtractedRelation {
            head_entity: self.head.clone(),
            head_start: 0,
            head_end: 0,
            tail_entity: self.tail.clone(),
            tail_start: 0,
            tail_end: 0,
            relation_type: self.relation_type.clone(),
            pattern_matched: self.verb_text.clone().unwrap_or_default(),
            pattern_start: self.span.map(|(s, _)| s).unwrap_or(0),
            pattern_end: self.span.map(|(_, e)| e).unwrap_or(0),
            confidence: self.confidence as f64,
        }
    }
}

/// Statistics from relation extraction
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationStats {
    pub cst_count: usize,
    pub inferred_count: usize,
    pub total_count: usize,
    pub time_us: u64,
}

// =============================================================================
// Type Aliases for Backward Compatibility
// =============================================================================

/// Type alias for backward compatibility
pub type RelationCortex = RelationEngine;

// =============================================================================
// RelationEngine
// =============================================================================

/// Relationship extraction engine using CST and Graph algorithms
pub struct RelationEngine {
    cst_confidence_threshold: f32,
    enable_inference: bool,
}

impl Default for RelationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationEngine {
    /// Create a new RelationEngine
    pub fn new() -> Self {
        Self {
            cst_confidence_threshold: 0.5,
            enable_inference: true,
        }
    }

    /// Set CST confidence threshold
    pub fn set_cst_threshold(&mut self, threshold: f32) {
        self.cst_confidence_threshold = threshold;
    }

    /// Enable/disable graph inference
    pub fn set_inference_enabled(&mut self, enabled: bool) {
        self.enable_inference = enabled;
    }

    /// Build (no-op for new architecture, kept for compatibility)
    pub fn build(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Pattern count (returns 0, no pattern dictionary)
    pub fn pattern_count(&self) -> usize {
        0
    }

    /// Extract relations using CST projection
    pub fn project_from_cst(
        &self,
        text: &str,
        entities: &[EntitySpan],
    ) -> Vec<UnifiedRelation> {
        if entities.is_empty() || text.is_empty() {
            return Vec::new();
        }

        let extractor = StructuredRelationExtractor::new();
        let (structured_relations, _stats) = extractor.extract_with_stats(text, entities);

        structured_relations
            .into_iter()
            .filter(|sr| sr.confidence >= self.cst_confidence_threshold as f64)
            .map(|sr| {
                UnifiedRelation {
                    head: sr.subject.clone(),
                    head_id: sr.subject_id.clone(),
                    tail: sr.object.clone().unwrap_or_default(),
                    tail_id: sr.object_id.clone(),
                    relation_type: sr.relation_type.clone(),
                    source: RelationSource::CST,
                    confidence: sr.confidence as f32,
                    span: Some((sr.predicate_span.start, sr.predicate_span.end)),
                    verb_text: Some(sr.predicate.clone()),
                }
            })
            .collect()
    }

    /// Infer relations from graph structure
    pub fn infer_from_graph(
        &self,
        edges: &[(String, String, String)],
        _entities: &[EntitySpan],
    ) -> Vec<UnifiedRelation> {
        if !self.enable_inference || edges.is_empty() {
            return Vec::new();
        }

        let mut inferred = Vec::new();
        
        // Build adjacency lists
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        let mut edge_set: HashSet<(String, String)> = HashSet::new();

        for (head, tail, _rel) in edges {
            adjacency.entry(head.clone()).or_default().push(tail.clone());
            adjacency.entry(tail.clone()).or_default().push(head.clone());
            
            let ordered = if head < tail {
                (head.clone(), tail.clone())
            } else {
                (tail.clone(), head.clone())
            };
            edge_set.insert(ordered);
        }

        // 1. Community Detection
        let communities = self.detect_simple_communities(&adjacency);
        
        for community in &communities {
            if community.len() < 2 {
                continue;
            }
            
            for (i, a) in community.iter().enumerate() {
                for b in community.iter().skip(i + 1) {
                    let ordered = if a < b {
                        (a.clone(), b.clone())
                    } else {
                        (b.clone(), a.clone())
                    };
                    
                    if !edge_set.contains(&ordered) {
                        inferred.push(UnifiedRelation {
                            head: a.clone(),
                            head_id: None,
                            tail: b.clone(),
                            tail_id: None,
                            relation_type: "ASSOCIATED_WITH".to_string(),
                            source: RelationSource::Inferred,
                            confidence: 0.4,
                            span: None,
                            verb_text: None,
                        });
                    }
                }
            }
        }

        // 2. Two-Hop Path Analysis
        for (a, neighbors_a) in &adjacency {
            for b in neighbors_a {
                if let Some(neighbors_b) = adjacency.get(b) {
                    for c in neighbors_b {
                        if c == a {
                            continue;
                        }
                        
                        let ordered = if a < c {
                            (a.clone(), c.clone())
                        } else {
                            (c.clone(), a.clone())
                        };
                        
                        if !edge_set.contains(&ordered) {
                            let already_inferred = inferred.iter().any(|r| {
                                (r.head == *a && r.tail == *c) || 
                                (r.head == *c && r.tail == *a)
                            });
                            
                            if !already_inferred {
                                inferred.push(UnifiedRelation {
                                    head: a.clone(),
                                    head_id: None,
                                    tail: c.clone(),
                                    tail_id: None,
                                    relation_type: "CONNECTED_VIA".to_string(),
                                    source: RelationSource::Inferred,
                                    confidence: 0.3,
                                    span: None,
                                    verb_text: Some(b.clone()),
                                });
                            }
                        }
                    }
                }
            }
        }

        // 3. Hub Detection
        let avg_degree: f32 = if adjacency.is_empty() {
            0.0
        } else {
            adjacency.values().map(|v| v.len() as f32).sum::<f32>() / adjacency.len() as f32
        };
        
        let hub_threshold = (avg_degree * 2.0).max(4.0) as usize;
        
        for (node, neighbors) in &adjacency {
            if neighbors.len() >= hub_threshold {
                for neighbor in neighbors {
                    let has_explicit = edges.iter().any(|(h, t, _)| {
                        (h == node && t == neighbor) || (h == neighbor && t == node)
                    });
                    
                    if !has_explicit {
                        inferred.push(UnifiedRelation {
                            head: neighbor.clone(),
                            head_id: None,
                            tail: node.clone(),
                            tail_id: None,
                            relation_type: "ORBITS".to_string(),
                            source: RelationSource::Inferred,
                            confidence: 0.35,
                            span: None,
                            verb_text: None,
                        });
                    }
                }
            }
        }

        inferred
    }

    /// Simple community detection using connected components
    fn detect_simple_communities(&self, adjacency: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut communities: Vec<Vec<String>> = Vec::new();

        for start in adjacency.keys() {
            if visited.contains(start) {
                continue;
            }

            let mut component = Vec::new();
            let mut queue = VecDeque::new();
            
            queue.push_back(start.clone());
            visited.insert(start.clone());

            while let Some(node) = queue.pop_front() {
                component.push(node.clone());
                
                if let Some(neighbors) = adjacency.get(&node) {
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            visited.insert(neighbor.clone());
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }

            if component.len() >= 2 {
                communities.push(component);
            }
        }

        communities
    }

    /// Full extraction pipeline
    pub fn extract(
        &self,
        text: &str,
        entities: &[EntitySpan],
        existing_edges: &[(String, String, String)],
    ) -> (Vec<UnifiedRelation>, RelationStats) {
        let start = std::time::Instant::now();
        let mut all_relations = Vec::new();
        let mut stats = RelationStats::default();

        // Layer 2: CST Projection
        let cst_relations = self.project_from_cst(text, entities);
        stats.cst_count = cst_relations.len();
        all_relations.extend(cst_relations);

        // Layer 3: Graph Inference
        let inferred_relations = self.infer_from_graph(existing_edges, entities);
        stats.inferred_count = inferred_relations.len();
        all_relations.extend(inferred_relations);

        stats.total_count = all_relations.len();
        stats.time_us = start.elapsed().as_micros() as u64;

        (all_relations, stats)
    }

    /// Extract relations using pre-computed chunks (FAST - avoids re-chunking)
    pub fn project_from_chunks(
        &self,
        text: &str,
        entities: &[EntitySpan],
        chunks: &ChunkResult,
    ) -> Vec<UnifiedRelation> {
        if entities.is_empty() || text.is_empty() {
            return Vec::new();
        }

        let extractor = StructuredRelationExtractor::new();
        let structured_relations = extractor.extract_from_chunks(text, entities, chunks);

        structured_relations
            .into_iter()
            .filter(|sr| sr.confidence >= self.cst_confidence_threshold as f64)
            .map(|sr| {
                UnifiedRelation {
                    head: sr.subject.clone(),
                    head_id: sr.subject_id.clone(),
                    tail: sr.object.clone().unwrap_or_default(),
                    tail_id: sr.object_id.clone(),
                    relation_type: sr.relation_type.clone(),
                    source: RelationSource::CST,
                    confidence: sr.confidence as f32,
                    span: Some((sr.predicate_span.start, sr.predicate_span.end)),
                    verb_text: Some(sr.predicate.clone()),
                }
            })
            .collect()
    }

    /// Full extraction pipeline with pre-computed chunks (FAST - avoids re-chunking)
    pub fn extract_with_chunks(
        &self,
        text: &str,
        entities: &[EntitySpan],
        existing_edges: &[(String, String, String)],
        chunks: &ChunkResult,
    ) -> (Vec<UnifiedRelation>, RelationStats) {
        let start = std::time::Instant::now();
        let mut all_relations = Vec::new();
        let mut stats = RelationStats::default();

        // Layer 2: CST Projection (using pre-computed chunks)
        let cst_relations = self.project_from_chunks(text, entities, chunks);
        stats.cst_count = cst_relations.len();
        all_relations.extend(cst_relations);

        // Layer 3: Graph Inference
        let inferred_relations = self.infer_from_graph(existing_edges, entities);
        stats.inferred_count = inferred_relations.len();
        all_relations.extend(inferred_relations);

        stats.total_count = all_relations.len();
        stats.time_us = start.elapsed().as_micros() as u64;

        (all_relations, stats)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Normalize a verb phrase to UPPER_SNAKE_CASE relation type
pub fn normalize_verb(verb: &str) -> String {
    verb.trim()
        .to_uppercase()
        .replace(' ', "_")
        .replace('-', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Find an entity that contains the given span
pub fn find_entity_in_span<'a>(
    entities: &'a [EntitySpan],
    start: usize,
    end: usize,
) -> Option<&'a EntitySpan> {
    entities.iter().find(|e| {
        e.start <= start && e.end >= end
        || (start <= e.start && end >= e.end)
        || (start < e.end && end > e.start)
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_source_serializes() {
        let source = RelationSource::CST;
        let json = serde_json::to_string(&source).unwrap();
        assert!(json.contains("CST"));
    }

    #[test]
    fn test_unified_relation_default() {
        let rel = UnifiedRelation::default();
        assert!(rel.head.is_empty());
        assert_eq!(rel.source, RelationSource::CST);
        assert_eq!(rel.confidence, 0.0);
    }

    #[test]
    fn test_unified_to_extracted_conversion() {
        let unified = UnifiedRelation {
            head: "Luffy".to_string(),
            tail: "Kaido".to_string(),
            relation_type: "DEFEATED".to_string(),
            source: RelationSource::CST,
            confidence: 0.9,
            span: Some((10, 20)),
            verb_text: Some("defeated".to_string()),
            ..Default::default()
        };

        let extracted = unified.to_extracted();
        assert_eq!(extracted.head_entity, "Luffy");
        assert_eq!(extracted.tail_entity, "Kaido");
        assert_eq!(extracted.relation_type, "DEFEATED");
    }

    #[test]
    fn test_relation_engine_creation() {
        let engine = RelationEngine::new();
        assert!(engine.enable_inference);
        assert_eq!(engine.cst_confidence_threshold, 0.5);
    }

    #[test]
    fn test_normalize_verb() {
        assert_eq!(normalize_verb("defeated"), "DEFEATED");
        assert_eq!(normalize_verb("is a friend of"), "IS_A_FRIEND_OF");
        assert_eq!(normalize_verb("co-leads"), "CO_LEADS");
    }

    #[test]
    fn test_find_entity_in_span() {
        let entities = vec![
            EntitySpan {
                label: "Luffy".to_string(),
                entity_id: None,
                start: 0,
                end: 5,
                kind: Some("CHARACTER".to_string()),
            },
            EntitySpan {
                label: "Kaido".to_string(),
                entity_id: None,
                start: 15,
                end: 20,
                kind: Some("CHARACTER".to_string()),
            },
        ];

        let found = find_entity_in_span(&entities, 0, 5);
        assert!(found.is_some());
        assert_eq!(found.unwrap().label, "Luffy");
    }

    #[test]
    fn test_project_from_cst_svo() {
        let engine = RelationEngine::new();
        let text = "Luffy defeated Kaido.";
        let entities = vec![
            EntitySpan {
                label: "Luffy".to_string(),
                entity_id: None,
                start: 0,
                end: 5,
                kind: Some("CHARACTER".to_string()),
            },
            EntitySpan {
                label: "Kaido".to_string(),
                entity_id: None,
                start: 15,
                end: 20,
                kind: Some("CHARACTER".to_string()),
            },
        ];

        let relations = engine.project_from_cst(text, &entities);
        
        assert!(!relations.is_empty(), "CST projection should find SVO relations");
        
        let defeated_rel = relations.iter().find(|r| r.relation_type == "DEFEATED");
        assert!(defeated_rel.is_some(), "Should find DEFEATED relation");
    }

    #[test]
    fn test_extract_full_pipeline() {
        let engine = RelationEngine::new();
        let text = "Luffy defeated Kaido.";
        let entities = vec![
            EntitySpan {
                label: "Luffy".to_string(),
                entity_id: None,
                start: 0,
                end: 5,
                kind: Some("CHARACTER".to_string()),
            },
            EntitySpan {
                label: "Kaido".to_string(),
                entity_id: None,
                start: 15,
                end: 20,
                kind: Some("CHARACTER".to_string()),
            },
        ];
        let edges: Vec<(String, String, String)> = vec![];

        let (relations, stats) = engine.extract(text, &entities, &edges);
        
        assert!(stats.total_count > 0);
        assert!(!relations.is_empty());
    }
}
