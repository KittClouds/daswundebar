//! Pipeline - Unified FST-NER processing pipeline
//!
//! Combines all stages:
//! 1. Tokenizer → tokens with byte offsets
//! 2. Gazetteer → known entity matches
//! 3. Rules → contextual pattern matches
//! 4. Resolver → conflict resolution
//!
//! # Performance Target
//! < 2ms for 10KB text, < 0.5ms per keystroke

use serde::{Deserialize, Serialize};
use super::tokenizer::{Token, Tokenizer};
use super::orthographic::{OrthographicClassifier, OrthographicShape};
use super::gazetteer::{Gazetteer, GazetteerMatch, MatchPriority};
use super::rules::{RuleEngine, RuleMatch};

// =============================================================================
// Types
// =============================================================================

/// Source of an entity match
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchSource {
    /// From gazetteer (known entities)
    Gazetteer,
    /// From contextual rules
    Rule,
    /// From orthographic heuristics
    Orthographic,
}

/// A unified entity match from the pipeline
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityMatch {
    /// Matched text
    pub text: String,
    /// Entity kind (CHARACTER, LOCATION, etc.)
    pub kind: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Start byte offset
    pub byte_start: usize,
    /// End byte offset  
    pub byte_end: usize,
    /// Match source
    pub source: MatchSource,
    /// Canonical label (if resolved from alias)
    pub canonical_label: Option<String>,
    /// Entity ID (if known from registry)
    pub entity_id: Option<String>,
}

/// Pipeline result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Entity matches
    pub matches: Vec<EntityMatch>,
    /// Processing time in microseconds
    pub timing_us: u64,
    /// Token count
    pub token_count: usize,
}

// =============================================================================
// Pipeline
// =============================================================================

/// FST-NER processing pipeline
pub struct Pipeline {
    tokenizer: Tokenizer,
    gazetteer: Gazetteer,
    rules: RuleEngine,
    classifier: OrthographicClassifier,
    /// Minimum confidence threshold
    min_confidence: f64,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Pipeline {
            tokenizer: Tokenizer::new(),
            gazetteer: Gazetteer::new(),
            rules: RuleEngine::new(),
            classifier: OrthographicClassifier::new(),
            min_confidence: 0.35, // Allow orthographic matches (0.40) through
        }
    }

    /// Get mutable reference to gazetteer for hydration
    pub fn gazetteer_mut(&mut self) -> &mut Gazetteer {
        &mut self.gazetteer
    }

    /// Get mutable reference to rules for customization
    pub fn rules_mut(&mut self) -> &mut RuleEngine {
        &mut self.rules
    }

    /// Get gazetteer pattern count
    pub fn gazetteer_pattern_count(&self) -> usize {
        self.gazetteer.pattern_count()
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.rule_count()
    }

    /// Build the pipeline (call after adding gazetteer entries)
    pub fn build(&mut self) -> Result<(), String> {
        self.gazetteer.build()
    }

    /// Process text and return entity matches
    pub fn process(&self, text: &str) -> PipelineResult {
        let start = std::time::Instant::now();

        // Stage 1: Tokenize
        let tokens = self.tokenizer.tokenize(text);
        let token_count = tokens.len();

        // Stage 2: Gazetteer matches (highest priority)
        let gaz_matches = self.gazetteer.find_matches(text);

        // Stage 3: Rule matches
        let rule_matches = self.rules.apply(&tokens);

        // Stage 4: Orthographic fallback (lowest priority)
        let ortho_matches = self.orthographic_scan(&tokens);

        // Stage 5: Merge and resolve conflicts
        let all_candidates = self.merge_candidates(gaz_matches, rule_matches, ortho_matches);
        let resolved = self.resolve_conflicts(all_candidates);

        // Stage 6: Filter by confidence
        let matches: Vec<EntityMatch> = resolved
            .into_iter()
            .filter(|m| m.confidence >= self.min_confidence)
            .collect();

        PipelineResult {
            matches,
            timing_us: start.elapsed().as_micros() as u64,
            token_count,
        }
    }

    /// Stopwords to filter from orthographic detection
    const STOPWORDS: &'static [&'static str] = &[
        "the", "this", "that", "however", "when", "where", "what", "which",
        "while", "also", "just", "from", "with", "have", "been", "being",
        "would", "could", "should", "about", "after", "before", "other",
        "such", "these", "those", "their", "there", "then", "than", "some",
        "only", "even", "well", "very", "much", "many", "most", "more",
        "into", "over", "under", "again", "further", "once", "here",
    ];

    /// Orthographic-only entity detection (fallback)
    fn orthographic_scan(&self, tokens: &[Token]) -> Vec<EntityMatch> {
        let mut matches = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            let token = &tokens[i];
            let shape = self.classifier.classify(&token.text);

            // Only consider entity candidates
            if !shape.is_entity_candidate() {
                i += 1;
                continue;
            }

            // Skip stopwords - common words that are often capitalized but aren't entities
            if Self::STOPWORDS.contains(&token.text.to_lowercase().as_str()) {
                i += 1;
                continue;
            }

            // Check if this is likely an entity (not at sentence start)
            // For now, we use a simple heuristic: first token is likely sentence start
            let is_sentence_start = i == 0 || self.is_after_sentence_end(&tokens, i);

            if shape == OrthographicShape::InitCap && is_sentence_start {
                i += 1;
                continue;
            }

            // Collect consecutive InitCap tokens
            let start_idx = i;
            let mut end_idx = i + 1;

            while end_idx < tokens.len() {
                let next_shape = self.classifier.classify(&tokens[end_idx].text);
                if next_shape == OrthographicShape::InitCap {
                    end_idx += 1;
                } else {
                    break;
                }
            }

            let text: String = tokens[start_idx..end_idx]
                .iter()
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            matches.push(EntityMatch {
                text,
                kind: "UNKNOWN".to_string(), // Orthographic doesn't determine type
                confidence: 0.40, // Low confidence for pure orthographic
                byte_start: tokens[start_idx].byte_start,
                byte_end: tokens[end_idx - 1].byte_end,
                source: MatchSource::Orthographic,
                canonical_label: None,
                entity_id: None,
            });

            i = end_idx;
        }

        matches
    }

    /// Check if position is after sentence-ending punctuation
    fn is_after_sentence_end(&self, tokens: &[Token], pos: usize) -> bool {
        if pos == 0 {
            return true;
        }

        // Look back for sentence-ending punctuation
        for i in (0..pos).rev() {
            let t = &tokens[i];
            if t.text == "." || t.text == "!" || t.text == "?" {
                return true;
            }
            // Skip whitespace
            if t.kind != super::tokenizer::TokenKind::Whitespace {
                return false;
            }
        }

        false
    }

    /// Merge candidates from all sources
    fn merge_candidates(
        &self,
        gaz: Vec<GazetteerMatch>,
        rules: Vec<RuleMatch>,
        ortho: Vec<EntityMatch>,
    ) -> Vec<EntityMatch> {
        let mut all = Vec::new();

        // Gazetteer matches (highest priority)
        for m in gaz {
            all.push(EntityMatch {
                text: m.text,
                kind: m.kind,
                confidence: m.confidence,
                byte_start: m.byte_start,
                byte_end: m.byte_end,
                source: MatchSource::Gazetteer,
                canonical_label: Some(m.canonical_label),
                entity_id: m.entity_id,
            });
        }

        // Rule matches
        for m in rules {
            all.push(EntityMatch {
                text: m.text,
                kind: m.kind,
                confidence: m.confidence,
                byte_start: m.byte_start,
                byte_end: m.byte_end,
                source: MatchSource::Rule,
                canonical_label: None,
                entity_id: None,
            });
        }

        // Orthographic matches (lowest priority)
        all.extend(ortho);

        all
    }

    /// Resolve overlapping matches
    fn resolve_conflicts(&self, mut candidates: Vec<EntityMatch>) -> Vec<EntityMatch> {
        if candidates.len() <= 1 {
            return candidates;
        }

        // Sort by: start ASC, length DESC, source priority DESC, confidence DESC
        candidates.sort_by(|a, b| {
            a.byte_start
                .cmp(&b.byte_start)
                .then_with(|| (b.byte_end - b.byte_start).cmp(&(a.byte_end - a.byte_start)))
                .then_with(|| source_priority(b.source).cmp(&source_priority(a.source)))
                .then_with(|| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut result = Vec::new();
        let mut last_end = 0;

        for m in candidates {
            if m.byte_start >= last_end {
                last_end = m.byte_end;
                result.push(m);
            }
        }

        result
    }
}

/// Get priority value for match source
fn source_priority(source: MatchSource) -> u8 {
    match source {
        MatchSource::Gazetteer => 3,
        MatchSource::Rule => 2,
        MatchSource::Orthographic => 1,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_pipeline_with_entities() -> Pipeline {
        let mut pipeline = Pipeline::new();
        pipeline.gazetteer_mut().add(
            "Zorian",
            "CHARACTER",
            "Zorian",
            Some("ent_1"),
            MatchPriority::High,
            1.0,
        );
        pipeline.gazetteer_mut().add(
            "Cyoria",
            "LOCATION",
            "Cyoria",
            Some("ent_2"),
            MatchPriority::High,
            1.0,
        );
        pipeline.build().unwrap();
        pipeline
    }

    #[test]
    fn test_gazetteer_match() {
        let pipeline = create_pipeline_with_entities();
        let result = pipeline.process("Zorian walked through Cyoria.");

        assert_eq!(result.matches.len(), 2);
        
        let zorian = result.matches.iter().find(|m| m.text == "Zorian");
        assert!(zorian.is_some());
        assert_eq!(zorian.unwrap().source, MatchSource::Gazetteer);
    }

    #[test]
    fn test_rule_match() {
        let pipeline = Pipeline::new();
        let result = pipeline.process("Lord Voldemort appeared.");

        // Should match via PersonAfterTitle rule
        let voldemort = result.matches.iter().find(|m| m.text.contains("Voldemort"));
        assert!(voldemort.is_some());
        assert_eq!(voldemort.unwrap().source, MatchSource::Rule);
    }

    #[test]
    fn test_gazetteer_beats_rule() {
        let mut pipeline = Pipeline::new();
        pipeline.gazetteer_mut().add(
            "Voldemort",
            "VILLAIN",
            "Lord Voldemort",
            Some("ent_v"),
            MatchPriority::High,
            1.0,
        );
        pipeline.build().unwrap();

        let result = pipeline.process("Voldemort appeared.");

        // Gazetteer match should win over rule
        let voldemort = result.matches.iter().find(|m| m.text == "Voldemort");
        assert!(voldemort.is_some());
        assert_eq!(voldemort.unwrap().source, MatchSource::Gazetteer);
        assert_eq!(voldemort.unwrap().kind, "VILLAIN");
    }

    #[test]
    fn test_orthographic_fallback() {
        let pipeline = Pipeline::new();
        // "Sally" mid-sentence should be detected (via Rule or Orthographic)
        let result = pipeline.process("Then Sally arrived.");

        // Sally can be matched by StandaloneInitCap rule OR orthographic fallback
        let sally = result.matches.iter().find(|m| m.text == "Sally");
        assert!(sally.is_some(), "Sally should be detected, got: {:?}", result.matches);
    }

    #[test]
    fn test_sentence_start_filtered() {
        let pipeline = Pipeline::new();
        // "The" at sentence start should not be detected
        let result = pipeline.process("The wizard walked.");

        // "wizard" is lowercase, shouldn't match
        // "The" is sentence start, shouldn't match as entity
        // Only rule matches might fire
        let the_match = result.matches.iter().find(|m| m.text == "The" && m.source == MatchSource::Orthographic);
        assert!(the_match.is_none());
    }

    #[test]
    fn test_overlapping_resolution() {
        let mut pipeline = Pipeline::new();
        pipeline.gazetteer_mut().add("Sally", "CHARACTER", "Sally", None, MatchPriority::High, 1.0);
        pipeline.gazetteer_mut().add("Sally Sparrow", "CHARACTER", "Sally Sparrow", None, MatchPriority::High, 1.0);
        pipeline.build().unwrap();

        let result = pipeline.process("Sally Sparrow walked.");

        // Should keep longer "Sally Sparrow"
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].text, "Sally Sparrow");
    }

    #[test]
    fn test_performance_small() {
        let pipeline = create_pipeline_with_entities();
        let text = "Zorian walked through Cyoria.";
        let result = pipeline.process(text);

        // Should be very fast for small text
        assert!(result.timing_us < 1000, "Should complete in < 1ms");
    }

    #[test]
    fn test_empty_text() {
        let pipeline = Pipeline::new();
        let result = pipeline.process("");

        assert!(result.matches.is_empty());
        assert_eq!(result.token_count, 0);
    }

    #[test]
    fn test_confidence_filtering() {
        let mut pipeline = Pipeline::new();
        pipeline.min_confidence = 0.80;

        // Orthographic matches have 0.40 confidence, should be filtered
        let result = pipeline.process("Then Sally arrived.");

        // No gazetteer, rules might match but low confidence ortho should be gone
        let ortho_matches: Vec<_> = result.matches.iter()
            .filter(|m| m.source == MatchSource::Orthographic)
            .collect();
        assert!(ortho_matches.is_empty());
    }
}
