//! NarrativeGraph - Standalone Dependency Parser (Native Tauri)
//!
//! A composable NLP engine that chunks text, attaches dependencies,
//! resolves coreferences, and attributes dialogue speakers.
//!
//! # Architecture
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     NarrativeGraph                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐  │
//! │  │  Chunker    │ →  │   Attacher   │ →  │   Resolver    │  │
//! │  │  (NP/VP/PP) │    │  (head-find) │    │ (coref/alias) │  │
//! │  └─────────────┘    └──────────────┘    └───────────────┘  │
//! │         ↓                                                   │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │              DialogueAttributor                      │   │
//! │  │    (speaker identification for quotes)              │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};

use super::attacher::{Attacher, Dependency};
use super::chunker::{Chunk, ChunkStats, Chunker};
use super::dialogue::{DialogueAttributor, QuotePosition};
use super::resolver::{EntityId, Gender, Resolver};

// =============================================================================
// Result Types
// =============================================================================

/// Complete analysis result from NarrativeGraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeResult {
    /// Detected phrase chunks
    pub chunks: Vec<Chunk>,
    /// Dependency links between chunks
    pub dependencies: Vec<Dependency>,
    /// Root chunk index (usually main verb)
    pub root_idx: Option<usize>,
    /// Statistics
    pub stats: NarrativeStats,
}

/// Performance and count statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeStats {
    pub chunk_count: usize,
    pub dependency_count: usize,
    pub noun_phrases: usize,
    pub verb_phrases: usize,
    pub prep_phrases: usize,
    pub timing_us: u64,
}

/// Entity hydration input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInput {
    pub id: String,
    pub name: String,
    pub gender: String, // "male", "female", "neutral", "plural", "unknown"
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub kind: String,
}

impl EntityInput {
    fn to_gender(&self) -> Gender {
        match self.gender.to_lowercase().as_str() {
            "male" | "m" => Gender::Male,
            "female" | "f" => Gender::Female,
            "neutral" | "n" | "it" => Gender::Neutral,
            "plural" | "p" | "they" => Gender::Plural,
            _ => Gender::Unknown,
        }
    }
}

// =============================================================================
// NarrativeGraph Facade
// =============================================================================

/// Standalone NLP engine for narrative text analysis
///
/// Combines chunking, dependency attachment, coreference resolution,
/// and dialogue attribution into a single, composable API.
pub struct NarrativeGraph {
    chunker: Chunker,
    attacher: Attacher,
    resolver: Resolver,
    #[allow(dead_code)]
    attributor: DialogueAttributor,
}

impl Default for NarrativeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl NarrativeGraph {
    /// Create a new NarrativeGraph engine
    pub fn new() -> Self {
        NarrativeGraph {
            chunker: Chunker::new(),
            attacher: Attacher::new(),
            resolver: Resolver::new(),
            attributor: DialogueAttributor::new(),
        }
    }

    /// Full analysis pipeline: chunk → attach → return structured result
    pub fn analyze(&self, text: &str) -> NarrativeResult {
        let start = std::time::Instant::now();

        // Step 1: Chunk
        let chunk_result = self.chunker.chunk(text);

        // Step 2: Attach dependencies
        let dep_graph = self.attacher.attach(&chunk_result.chunks);

        // Step 3: Build stats
        let chunk_stats = ChunkStats::from_chunks(&chunk_result.chunks, chunk_result.tokens.len());

        // Capture counts before moving
        let dep_count = dep_graph.dependencies.len();
        let chunk_count = chunk_result.chunks.len();

        NarrativeResult {
            chunks: dep_graph.chunks,
            dependencies: dep_graph.dependencies,
            root_idx: dep_graph.root_idx,
            stats: NarrativeStats {
                chunk_count,
                dependency_count: dep_count,
                noun_phrases: chunk_stats.noun_phrases,
                verb_phrases: chunk_stats.verb_phrases,
                prep_phrases: chunk_stats.prep_phrases,
                timing_us: start.elapsed().as_micros() as u64,
            },
        }
    }

    /// Hydrate with known entities for coreference resolution
    pub fn hydrate_entities(&mut self, entities: Vec<EntityInput>) {
        for input in entities {
            self.resolver
                .register_entity(&input.id, &input.name, input.to_gender(), input.aliases);
        }
    }

    /// Hydrate a single entity
    pub fn hydrate_entity(&mut self, id: &str, name: &str, gender: Gender, aliases: Vec<String>) {
        self.resolver.register_entity(id, name, gender, aliases);
    }

    /// Resolve a pronoun or alias to an entity ID
    pub fn resolve(&mut self, text: &str) -> Option<String> {
        self.resolver.resolve(text)
    }

    /// Record an explicit mention (updates coreference context)
    pub fn observe_mention(&mut self, entity_id: &str) {
        self.resolver.observe_mention(entity_id);
    }

    /// Attribute speaker for dialogue
    pub fn attribute_speaker(&mut self, text: &str, quote_pos: QuotePosition) -> Option<EntityId> {
        let chunk_result = self.chunker.chunk(text);
        DialogueAttributor::attribute_simple(text, &chunk_result.chunks, quote_pos, &mut self.resolver)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_instance() {
        let _ng = NarrativeGraph::new();
        assert!(true);
    }

    #[test]
    fn test_analyze_simple() {
        let ng = NarrativeGraph::new();
        let result = ng.analyze("The wizard walked through the forest.");

        assert!(result.stats.chunk_count > 0, "Should have chunks");
        assert!(result.stats.noun_phrases >= 1, "Should have at least one NP");
        assert!(result.stats.verb_phrases >= 1, "Should have at least one VP");
    }

    #[test]
    fn test_analyze_with_dependencies() {
        let ng = NarrativeGraph::new();
        let result = ng.analyze("Frodo found the ring.");

        assert!(
            result.stats.chunk_count >= 1,
            "Should produce at least 1 chunk"
        );
    }

    #[test]
    fn test_hydrate_and_resolve() {
        let mut ng = NarrativeGraph::new();
        ng.hydrate_entity(
            "e1",
            "Gandalf",
            Gender::Male,
            vec!["Mithrandir".to_string()],
        );

        // Resolve by name
        assert_eq!(ng.resolve("Gandalf").as_deref(), Some("e1"));

        // Resolve by alias
        assert_eq!(ng.resolve("Mithrandir").as_deref(), Some("e1"));
    }

    #[test]
    fn test_pronoun_resolution_with_context() {
        let mut ng = NarrativeGraph::new();
        ng.hydrate_entity("e1", "Gandalf", Gender::Male, vec![]);
        ng.observe_mention("e1");

        assert_eq!(ng.resolve("He").as_deref(), Some("e1"));
    }

    #[test]
    fn test_dialogue_attribution() {
        let mut ng = NarrativeGraph::new();
        ng.hydrate_entity("e1", "Gandalf", Gender::Male, vec![]);

        let speaker = ng.attribute_speaker("shouted Gandalf", QuotePosition::Before);
        assert_eq!(speaker.as_deref(), Some("e1"));
    }

    #[test]
    fn test_full_narrative_flow() {
        let mut ng = NarrativeGraph::new();

        // Hydrate entities
        ng.hydrate_entity(
            "gandalf",
            "Gandalf",
            Gender::Male,
            vec!["the wizard".to_string()],
        );
        ng.hydrate_entity("frodo", "Frodo", Gender::Male, vec![]);

        // Analyze text
        let _result = ng.analyze("Gandalf walked slowly. The wizard stopped.");

        // Observe first mention
        ng.observe_mention("gandalf");

        // Resolve pronoun
        assert_eq!(ng.resolve("He").as_deref(), Some("gandalf"));

        // Resolve alias
        assert_eq!(ng.resolve("the wizard").as_deref(), Some("gandalf"));
    }

    #[test]
    fn test_performance() {
        let ng = NarrativeGraph::new();
        let text = "The ancient wizard slowly walked through the dark forest. \
                   He was searching for the hidden tower where the dragon lived.";

        let result = ng.analyze(text);

        // Should complete in under 5ms
        assert!(
            result.stats.timing_us < 5000,
            "Should analyze quickly, took {}us",
            result.stats.timing_us
        );
    }
}
