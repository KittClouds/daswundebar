//! Dialogue Attributor - Speaker identification (Native Tauri)
//!
//! Identifies speakers for dialogue chunks using heuristics:
//! 1. **Quote Before**: `"Run!" shouted Gandalf.`
//! 2. **Quote After**: `Frodo said, "No."`
//! 3. **Implicit**: Uses context/turn-taking if no explicit attribution

use serde::{Deserialize, Serialize};

use super::chunker::{Chunk, ChunkKind};
use super::resolver::{EntityId, Resolver};

// =============================================================================
// Types
// =============================================================================

/// Position of quote relative to narration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuotePosition {
    /// Quote comes before Narration: "Run," he said
    Before,
    /// Quote comes after Narration: He said, "Run"
    After,
}

// =============================================================================
// DialogueAttributor
// =============================================================================

/// Identifies speakers for dialogue segments
pub struct DialogueAttributor;

impl Default for DialogueAttributor {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogueAttributor {
    pub fn new() -> Self {
        DialogueAttributor
    }

    /// Attempt to identify the speaker of a dialogue segment
    ///
    /// `chunks`: The chunks of the surrounding narration (before/after the quote)
    /// `quote_position`: Whether the quote is before or after the narration
    /// `resolver`: The coreference resolver with entity context
    #[allow(dead_code)]
    pub fn attribute_speaker(
        chunks: &[Chunk],
        _quote_position: QuotePosition,
        resolver: &mut Resolver,
        text: &str,
    ) -> Option<EntityId> {
        // Both positions use the same strategy: find NP that resolves to entity
        Self::find_speaker_in_chunks(chunks, resolver, text)
    }

    fn find_speaker_in_chunks(
        chunks: &[Chunk],
        resolver: &mut Resolver,
        text: &str,
    ) -> Option<EntityId> {
        for chunk in chunks {
            if chunk.kind == ChunkKind::NounPhrase {
                let np_text = chunk.text(text);
                if let Some(id) = resolver.resolve(np_text) {
                    return Some(id);
                }
            }
        }
        None
    }

    /// Main API for speaker attribution
    ///
    /// Takes text, chunks, quote position, and resolver to identify speaker
    pub fn attribute_simple(
        text: &str,
        chunks: &[Chunk],
        _quote_pos: QuotePosition,
        resolver: &mut Resolver,
    ) -> Option<EntityId> {
        // Strategy:
        // 1. Find all NPs
        // 2. Resolve them to entities
        // 3. Return first resolved Entity

        for chunk in chunks {
            if chunk.kind == ChunkKind::NounPhrase {
                let np_text = chunk.text(text);
                if let Some(id) = resolver.resolve(np_text) {
                    return Some(id);
                }
            }
        }

        None
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::Chunker;
    use crate::resolver::Gender;

    fn setup() -> (Chunker, Resolver) {
        let chunker = Chunker::new();
        let mut resolver = Resolver::new();
        resolver.register_entity("e1", "Gandalf", Gender::Male, vec![]);
        resolver.register_entity("e2", "Frodo", Gender::Male, vec![]);
        (chunker, resolver)
    }

    #[test]
    fn test_quote_after_subject() {
        // Frodo said ...
        let (chunker, mut resolver) = setup();
        let text = "Frodo said";
        let res = chunker.chunk(text);

        let speaker = DialogueAttributor::attribute_simple(
            text,
            &res.chunks,
            QuotePosition::After,
            &mut resolver,
        );

        assert_eq!(speaker.as_deref(), Some("e2"));
    }

    #[test]
    fn test_quote_before_subject() {
        // ... shouted Gandalf
        let (chunker, mut resolver) = setup();
        let text = "shouted Gandalf";
        let res = chunker.chunk(text);

        let speaker = DialogueAttributor::attribute_simple(
            text,
            &res.chunks,
            QuotePosition::Before,
            &mut resolver,
        );

        assert_eq!(speaker.as_deref(), Some("e1"));
    }

    #[test]
    fn test_pronoun_attribution() {
        // He asked ...
        let (chunker, mut resolver) = setup();

        // Context: Gandalf just mentioned
        resolver.observe_mention("e1");

        let text = "He asked";
        let res = chunker.chunk(text);

        let speaker = DialogueAttributor::attribute_simple(
            text,
            &res.chunks,
            QuotePosition::After,
            &mut resolver,
        );

        assert_eq!(
            speaker.as_deref(),
            Some("e1"),
            "Should resolve 'He' to Gandalf"
        );
    }
}
