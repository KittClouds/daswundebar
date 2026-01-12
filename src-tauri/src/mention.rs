//! Unified Mention Contract
//!
//! Single type for all entity mentions regardless of source:
//! - Regex patterns (user tags)
//! - Aho-Corasick (ImplicitCortex)
//! - FST-NER (rules + orthographic)
//! - ML NER (GLiNER)
//!
//! This prevents source-specific types from leaking across the codebase.

use serde::{Deserialize, Serialize};

// =============================================================================
// Mention Source
// =============================================================================

/// Source of a mention detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MentionSource {
    /// User-typed syntax: [CHARACTER|Jon]
    UserTag,
    /// Aho-Corasick match on registered entity name/alias
    Implicit,
    /// FST-NER gazetteer match (registered entity)
    FstGazetteer,
    /// FST-NER rule match (contextual pattern)
    FstRule,
    /// FST-NER orthographic match (shape-based heuristic)
    FstOrthographic,
    /// ML model inference (GLiNER, etc.)
    MlNer,
    /// Temporal expression (not an entity, but uses same pipeline)
    Temporal,
}

impl MentionSource {
    /// Priority for conflict resolution (higher = wins)
    pub fn priority(&self) -> u8 {
        match self {
            MentionSource::UserTag => 100,      // King
            MentionSource::Implicit => 80,      // Registered entity
            MentionSource::FstGazetteer => 70,  // FST found registered
            MentionSource::FstRule => 60,       // FST rule matched
            MentionSource::MlNer => 50,         // ML inference
            MentionSource::FstOrthographic => 40, // Shape heuristic
            MentionSource::Temporal => 30,      // Time expression
        }
    }

    /// Human-readable name
    pub fn as_str(&self) -> &'static str {
        match self {
            MentionSource::UserTag => "user_tag",
            MentionSource::Implicit => "implicit",
            MentionSource::FstGazetteer => "fst_gazetteer",
            MentionSource::FstRule => "fst_rule",
            MentionSource::FstOrthographic => "fst_orthographic",
            MentionSource::MlNer => "ml_ner",
            MentionSource::Temporal => "temporal",
        }
    }
}

// =============================================================================
// Mention
// =============================================================================

/// A single detected entity mention in a document.
/// 
/// This is the **universal contract** for all extractors.
/// By the time data leaves any extractor, it MUST be in this format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mention {
    /// Document/note ID where this mention was found
    pub doc_id: String,
    
    /// Start byte offset in the document
    pub start: usize,
    
    /// End byte offset in the document (exclusive)
    pub end: usize,
    
    /// The actual surface text that was matched
    pub surface: String,
    
    /// Entity kind/label (CHARACTER, LOCATION, etc.)
    pub label: String,
    
    /// How this mention was detected
    pub source: MentionSource,
    
    /// Confidence score (0.0-1.0), if available
    pub score: Option<f64>,
    
    /// Rule ID that triggered this match (for FST rules)
    pub rule_id: Option<String>,
    
    /// Canonical entity ID if this mention is linked to a known entity
    pub entity_id: Option<String>,
    
    /// Canonical label if different from surface text
    pub canonical: Option<String>,
}

impl Mention {
    /// Create a new mention with required fields
    pub fn new(
        doc_id: impl Into<String>,
        start: usize,
        end: usize,
        surface: impl Into<String>,
        label: impl Into<String>,
        source: MentionSource,
    ) -> Self {
        Self {
            doc_id: doc_id.into(),
            start,
            end,
            surface: surface.into(),
            label: label.into(),
            source,
            score: None,
            rule_id: None,
            entity_id: None,
            canonical: None,
        }
    }

    /// Builder: set confidence score
    pub fn with_score(mut self, score: f64) -> Self {
        self.score = Some(score);
        self
    }

    /// Builder: set rule ID
    pub fn with_rule_id(mut self, rule_id: impl Into<String>) -> Self {
        self.rule_id = Some(rule_id.into());
        self
    }

    /// Builder: link to entity
    pub fn with_entity(mut self, entity_id: impl Into<String>, canonical: impl Into<String>) -> Self {
        self.entity_id = Some(entity_id.into());
        self.canonical = Some(canonical.into());
        self
    }

    /// Get effective label (canonical if available, else surface)
    pub fn effective_label(&self) -> &str {
        self.canonical.as_deref().unwrap_or(&self.surface)
    }

    /// Check if this mention overlaps with another
    pub fn overlaps(&self, other: &Mention) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Length in bytes
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Is this mention empty?
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

// =============================================================================
// Suggestion
// =============================================================================

/// A grouped/merged view of mentions for the same entity.
/// 
/// Used for the UI suggestion panel - shows one row per unique entity
/// with all its mentions aggregated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Canonical label for this entity
    pub canonical: String,
    
    /// Entity kind (CHARACTER, LOCATION, etc.)
    pub label: String,
    
    /// Entity ID if already registered in the graph
    pub entity_id: Option<String>,
    
    /// All mentions of this entity in the document
    pub mentions: Vec<Mention>,
    
    /// Aggregated ranking score (for sorting in UI)
    pub rank_score: f64,
}

impl Suggestion {
    /// Create a new suggestion from a set of mentions
    pub fn from_mentions(mentions: Vec<Mention>) -> Option<Self> {
        if mentions.is_empty() {
            return None;
        }

        // Use the highest-priority mention for canonical info
        let best = mentions.iter()
            .max_by_key(|m| (m.source.priority(), m.score.map(|s| (s * 1000.0) as i32).unwrap_or(0)))
            .unwrap();

        let canonical = best.canonical.clone()
            .unwrap_or_else(|| best.surface.clone());

        let label = best.label.clone();
        let entity_id = mentions.iter()
            .find_map(|m| m.entity_id.clone());

        // Rank score: weighted by source priority and mention count
        let rank_score = mentions.iter()
            .map(|m| {
                let base = m.source.priority() as f64;
                let conf = m.score.unwrap_or(0.8);
                base * conf
            })
            .sum::<f64>() / mentions.len() as f64;

        Some(Self {
            canonical,
            label,
            entity_id,
            mentions,
            rank_score,
        })
    }

    /// Number of times this entity is mentioned
    pub fn mention_count(&self) -> usize {
        self.mentions.len()
    }

    /// Is this entity already registered?
    pub fn is_registered(&self) -> bool {
        self.entity_id.is_some()
    }

    /// Get all unique sources that detected this entity
    pub fn sources(&self) -> Vec<MentionSource> {
        let mut sources: Vec<MentionSource> = self.mentions.iter()
            .map(|m| m.source)
            .collect();
        sources.sort_by_key(|s| std::cmp::Reverse(s.priority()));
        sources.dedup();
        sources
    }
}

// =============================================================================
// Mention Merger
// =============================================================================

/// Utilities for merging mentions from multiple sources
pub struct MentionMerger;

impl MentionMerger {
    /// Merge overlapping mentions, keeping higher-priority source
    pub fn dedupe_overlapping(mut mentions: Vec<Mention>) -> Vec<Mention> {
        if mentions.len() <= 1 {
            return mentions;
        }

        // Sort by: start ASC, length DESC, priority DESC
        mentions.sort_by(|a, b| {
            a.start.cmp(&b.start)
                .then_with(|| b.len().cmp(&a.len()))
                .then_with(|| b.source.priority().cmp(&a.source.priority()))
        });

        let mut result = Vec::with_capacity(mentions.len());
        let mut last_end = 0;

        for m in mentions {
            if m.start >= last_end {
                last_end = m.end;
                result.push(m);
            }
            // Skip overlapping lower-priority mentions
        }

        result
    }

    /// Group mentions by canonical label into Suggestions
    pub fn group_into_suggestions(mentions: Vec<Mention>) -> Vec<Suggestion> {
        use std::collections::HashMap;

        let mut groups: HashMap<String, Vec<Mention>> = HashMap::new();

        for m in mentions {
            let key = m.effective_label().to_lowercase();
            groups.entry(key).or_default().push(m);
        }

        let mut suggestions: Vec<Suggestion> = groups
            .into_values()
            .filter_map(Suggestion::from_mentions)
            .collect();

        // Sort by rank score descending
        suggestions.sort_by(|a, b| b.rank_score.partial_cmp(&a.rank_score).unwrap_or(std::cmp::Ordering::Equal));

        suggestions
    }
}

// =============================================================================
// Conversions from existing types
// =============================================================================

impl From<&crate::implicit::ImplicitMention> for Mention {
    fn from(m: &crate::implicit::ImplicitMention) -> Self {
        Mention {
            doc_id: String::new(), // Must be set by caller
            start: m.start,
            end: m.end,
            surface: m.matched_text.clone(),
            label: m.entity_kind.clone(),
            source: MentionSource::Implicit,
            score: Some(m.confidence),
            rule_id: None,
            entity_id: Some(m.entity_id.clone()),
            canonical: Some(m.entity_label.clone()),
        }
    }
}

impl From<&crate::ner::pipeline::EntityMatch> for Mention {
    fn from(m: &crate::ner::pipeline::EntityMatch) -> Self {
        let source = match m.source {
            crate::ner::pipeline::MatchSource::Gazetteer => MentionSource::FstGazetteer,
            crate::ner::pipeline::MatchSource::Rule => MentionSource::FstRule,
            crate::ner::pipeline::MatchSource::Orthographic => MentionSource::FstOrthographic,
        };

        Mention {
            doc_id: String::new(), // Must be set by caller
            start: m.byte_start,
            end: m.byte_end,
            surface: m.text.clone(),
            label: m.kind.clone(),
            source,
            score: Some(m.confidence),
            rule_id: None,
            entity_id: m.entity_id.clone(),
            canonical: m.canonical_label.clone(),
        }
    }
}

/// Convert from GLiNER/AI Suggestion (cold path ML NER)
impl From<&crate::ai::suggestion_layer::Suggestion> for Mention {
    fn from(s: &crate::ai::suggestion_layer::Suggestion) -> Self {
        Mention {
            doc_id: s.source_note_id.clone(),
            start: s.byte_start,
            end: s.byte_end,
            surface: s.text.clone(),
            label: s.entity_type.clone(),
            source: MentionSource::MlNer,
            score: Some(s.confidence as f64),
            rule_id: None,
            entity_id: None, // Not registered until accepted
            canonical: None,
        }
    }
}

/// Convert from TemporalMention
impl From<&crate::temporal::TemporalMention> for Mention {
    fn from(t: &crate::temporal::TemporalMention) -> Self {
        Mention {
            doc_id: String::new(), // Must be set by caller
            start: t.start,
            end: t.end,
            surface: t.text.clone(),
            label: "TEMPORAL".to_string(),
            source: MentionSource::Temporal,
            score: Some(t.confidence),
            rule_id: None,
            entity_id: None,
            canonical: None,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mention_builder() {
        let m = Mention::new("note_1", 0, 5, "Frodo", "CHARACTER", MentionSource::Implicit)
            .with_score(0.95)
            .with_entity("ent_1", "Frodo Baggins");

        assert_eq!(m.surface, "Frodo");
        assert_eq!(m.effective_label(), "Frodo Baggins");
        assert_eq!(m.score, Some(0.95));
    }

    #[test]
    fn test_source_priority() {
        assert!(MentionSource::UserTag.priority() > MentionSource::Implicit.priority());
        assert!(MentionSource::Implicit.priority() > MentionSource::FstRule.priority());
        assert!(MentionSource::FstRule.priority() > MentionSource::FstOrthographic.priority());
    }

    #[test]
    fn test_mention_overlaps() {
        let m1 = Mention::new("doc", 0, 5, "Frodo", "CHARACTER", MentionSource::Implicit);
        let m2 = Mention::new("doc", 3, 8, "Frodo", "CHARACTER", MentionSource::FstRule);
        let m3 = Mention::new("doc", 10, 15, "Gimli", "CHARACTER", MentionSource::Implicit);

        assert!(m1.overlaps(&m2));
        assert!(!m1.overlaps(&m3));
    }

    #[test]
    fn test_dedupe_keeps_higher_priority() {
        let mentions = vec![
            Mention::new("doc", 0, 5, "Frodo", "CHARACTER", MentionSource::FstOrthographic),
            Mention::new("doc", 0, 5, "Frodo", "CHARACTER", MentionSource::UserTag),
            Mention::new("doc", 10, 15, "Gimli", "CHARACTER", MentionSource::Implicit),
        ];

        let deduped = MentionMerger::dedupe_overlapping(mentions);
        
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].source, MentionSource::UserTag);
        assert_eq!(deduped[1].surface, "Gimli");
    }

    #[test]
    fn test_group_into_suggestions() {
        // All three have the SAME effective_label (surface text, no canonical override)
        let mentions = vec![
            Mention::new("doc", 0, 5, "Frodo", "CHARACTER", MentionSource::Implicit)
                .with_entity("ent_1", "Frodo"), // canonical = surface
            Mention::new("doc", 20, 25, "Frodo", "CHARACTER", MentionSource::FstRule),
            Mention::new("doc", 10, 15, "Gimli", "CHARACTER", MentionSource::Implicit),
        ];

        let suggestions = MentionMerger::group_into_suggestions(mentions);
        
        // 2 groups: "frodo" (2 mentions) and "gimli" (1 mention)
        assert_eq!(suggestions.len(), 2);
        
        // Find the Frodo suggestion
        let frodo = suggestions.iter().find(|s| s.canonical == "Frodo").unwrap();
        assert_eq!(frodo.mention_count(), 2);
        assert!(frodo.is_registered());
    }

    #[test]
    fn test_suggestion_sources() {
        let mentions = vec![
            Mention::new("doc", 0, 5, "Frodo", "CHARACTER", MentionSource::Implicit),
            Mention::new("doc", 20, 25, "Frodo", "CHARACTER", MentionSource::FstRule),
            Mention::new("doc", 40, 45, "Frodo", "CHARACTER", MentionSource::FstOrthographic),
        ];

        let suggestion = Suggestion::from_mentions(mentions).unwrap();
        let sources = suggestion.sources();
        
        // Should be sorted by priority (highest first)
        assert_eq!(sources[0], MentionSource::Implicit);
        assert_eq!(sources[1], MentionSource::FstRule);
        assert_eq!(sources[2], MentionSource::FstOrthographic);
    }
}
