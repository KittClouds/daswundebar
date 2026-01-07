//! StructuredRelationExtractor - Structure-Based Relation Extraction (Native Tauri)
//!
//! Uses Chunker output (NP/VP/PP) to extract relations based on sentence structure.
//!
//! # Architecture
//!
//! Instead of scanning text for pattern strings, we:
//! 1. Chunk the text into NP/VP/PP phrases
//! 2. Find Subject-Verb-Object patterns around VPs
//! 3. Map entities to subjects/objects based on positional proximity
//! 4. Handle passive voice by detecting PP("by X") and flipping S/O

use serde::{Deserialize, Serialize};

use super::chunker::{Chunk, ChunkKind, ChunkResult, Chunker, TextRange};
use super::relation::EntitySpan;
use super::verb_morphology::VerbLexicon;

// =============================================================================
// Core Types
// =============================================================================

/// A structured relation extracted from sentence structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredRelation {
    /// The subject entity (WHO/WHAT does the action)
    pub subject: String,
    /// Subject entity ID (if known)
    pub subject_id: Option<String>,
    /// Subject position in text
    pub subject_span: TextRange,
    
    /// The verb/predicate (WHAT action)
    pub predicate: String,
    /// Predicate position in text
    pub predicate_span: TextRange,
    /// Normalized relation type (e.g., "DEFEATED", "LOVES")
    pub relation_type: String,
    
    /// The object entity (WHO/WHAT receives the action)
    pub object: Option<String>,
    /// Object entity ID (if known)
    pub object_id: Option<String>,
    /// Object position in text
    pub object_span: Option<TextRange>,
    
    /// Modifiers extracted from PP chunks
    pub modifiers: Vec<RelationModifier>,
    
    /// Whether this was derived from passive voice transformation
    pub passive_transformed: bool,
    
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
}

/// A modifier attached to a relation (from PP chunks)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationModifier {
    /// Type of modifier (LOCATION, MANNER, TIME, INSTRUMENT)
    pub modifier_type: ModifierType,
    /// The modifier text
    pub text: String,
    /// Position in source text
    pub span: TextRange,
    /// The preposition that introduced this modifier
    pub preposition: String,
}

/// Types of relation modifiers (extracted from PP analysis)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierType {
    /// WHERE: "in Mordor", "at the bridge"
    Location,
    /// HOW: "with his sword", "by magic"
    Manner,
    /// WHEN: "during the battle", "after midnight"
    Time,
    /// WITH WHAT: "with Sting", "using the Ring"
    Instrument,
    /// Generic/unknown
    Other,
}

impl ModifierType {
    /// Infer modifier type from preposition
    pub fn from_preposition(prep: &str) -> Self {
        let prep_lower = prep.to_lowercase();
        match prep_lower.as_str() {
            // Location
            "in" | "at" | "on" | "within" | "inside" | "outside" |
            "near" | "beside" | "behind" | "above" | "below" |
            "between" | "among" | "around" | "through" | "across" |
            "into" | "onto" | "toward" | "towards" => ModifierType::Location,
            
            // Time
            "during" | "after" | "before" | "since" | "until" |
            "when" | "while" => ModifierType::Time,
            
            // Manner/Instrument
            "with" | "by" | "using" | "via" => ModifierType::Manner,
            
            // Default
            _ => ModifierType::Other,
        }
    }
}

/// Internal representation of SVO pattern before entity resolution
#[derive(Debug, Clone)]
pub struct SVOPattern {
    /// Entity that is the subject (before VP)
    pub subject: EntitySpan,
    /// The verb phrase chunk
    pub verb: Chunk,
    /// Entity that is the object (after VP), if any
    pub object: Option<EntitySpan>,
    /// PP/ADJP modifiers between or after S-V-O
    pub modifiers: Vec<Chunk>,
    /// Whether this pattern has passive voice markers
    pub is_passive: bool,
}

/// Statistics from structured relation extraction
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredRelationStats {
    pub chunks_processed: usize,
    pub vp_count: usize,
    pub svo_patterns_found: usize,
    pub passive_transformations: usize,
    pub relations_extracted: usize,
    pub timing_us: u64,
}

// =============================================================================
// StructuredRelationExtractor
// =============================================================================

/// Structure-based relation extractor using Chunker output
///
/// Instead of matching pattern strings, this uses sentence structure:
/// - NP before VP = likely subject
/// - NP after VP = likely object  
/// - PP after VP = modifiers (location, manner, time)
pub struct StructuredRelationExtractor {
    /// The chunker for NP/VP/PP detection
    chunker: Chunker,
    /// Unified verb lexicon with morphology + semantics
    lexicon: VerbLexicon,
    /// Passive voice auxiliary verbs
    passive_auxiliaries: Vec<String>,
    /// Maximum character distance between entity and VP
    max_distance: usize,
}

impl Default for StructuredRelationExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuredRelationExtractor {
    /// Create a new extractor with default configuration
    pub fn new() -> Self {
        Self {
            chunker: Chunker::new(),
            lexicon: VerbLexicon::new(),
            passive_auxiliaries: vec![
                "was".to_string(),
                "were".to_string(),
                "been".to_string(),
                "being".to_string(),
                "is".to_string(),
                "are".to_string(),
                "got".to_string(),
                "gets".to_string(),
            ],
            max_distance: 100,
        }
    }

    /// Extract structured relations from text using sentence structure
    pub fn extract_structured(
        &self,
        text: &str,
        entities: &[EntitySpan],
    ) -> Vec<StructuredRelation> {
        let chunk_result = self.chunker.chunk(text);
        self.extract_from_chunks(text, entities, &chunk_result)
    }

    /// Extract relations given pre-computed chunks
    pub fn extract_from_chunks(
        &self,
        text: &str,
        entities: &[EntitySpan],
        chunk_result: &ChunkResult,
    ) -> Vec<StructuredRelation> {
        if entities.is_empty() {
            return Vec::new();
        }

        let svo_patterns = self.find_svo_patterns(&chunk_result.chunks, entities, text);
        
        svo_patterns
            .into_iter()
            .filter_map(|pattern| self.pattern_to_relation(pattern, text))
            .collect()
    }

    /// Extract with statistics
    pub fn extract_with_stats(
        &self,
        text: &str,
        entities: &[EntitySpan],
    ) -> (Vec<StructuredRelation>, StructuredRelationStats) {
        let start = std::time::Instant::now();
        
        let chunk_result = self.chunker.chunk(text);
        let vp_count = chunk_result.chunks.iter()
            .filter(|c| c.kind == ChunkKind::VerbPhrase)
            .count();
        
        let svo_patterns = self.find_svo_patterns(&chunk_result.chunks, entities, text);
        let passive_count = svo_patterns.iter().filter(|p| p.is_passive).count();
        
        let relations: Vec<_> = svo_patterns
            .iter()
            .filter_map(|pattern| self.pattern_to_relation(pattern.clone(), text))
            .collect();

        let stats = StructuredRelationStats {
            chunks_processed: chunk_result.chunks.len(),
            vp_count,
            svo_patterns_found: svo_patterns.len(),
            passive_transformations: passive_count,
            relations_extracted: relations.len(),
            timing_us: start.elapsed().as_micros() as u64,
        };

        (relations, stats)
    }

    /// Find SVO patterns by matching entities to VP chunks
    pub fn find_svo_patterns(
        &self,
        chunks: &[Chunk],
        entities: &[EntitySpan],
        text: &str,
    ) -> Vec<SVOPattern> {
        let mut patterns = Vec::new();
        
        // Sort entities by position for efficient lookup
        let mut sorted_entities = entities.to_vec();
        sorted_entities.sort_by_key(|e| e.start);

        // Find all VP chunks
        let vp_chunks: Vec<_> = chunks.iter()
            .filter(|c| c.kind == ChunkKind::VerbPhrase)
            .collect();

        for vp in &vp_chunks {
            // Get sentence boundaries around this VP
            let (sent_start, sent_end) = self.find_sentence_bounds(text, vp.range.start);

            // Find subject: nearest entity ending before VP starts, WITHIN SAME SENTENCE
            let subject = self.find_nearest_entity_before_in_range(
                &sorted_entities,
                vp.range.start,
                sent_start,
            );

            // Find object: nearest entity starting after VP ends, WITHIN SAME SENTENCE
            let object = self.find_nearest_entity_after_in_range(
                &sorted_entities,
                vp.range.end,
                sent_end,
            );

            // Need at least a subject
            let subject = match subject {
                Some(s) => s,
                None => continue,
            };

            // Collect PP modifiers after the VP (within sentence)
            let modifiers = self.collect_modifiers_in_range(chunks, vp.range.end, sent_end);

            // Check for passive voice
            let is_passive = self.detect_passive(vp, &modifiers, text);

            let mut pattern = SVOPattern {
                subject: subject.clone(),
                verb: (*vp).clone(),
                object: object.cloned(),
                modifiers,
                is_passive,
            };

            // Handle passive voice transformation
            if is_passive {
                if let Some(transformed) = self.handle_passive(&pattern, text) {
                    pattern = transformed;
                }
            }

            patterns.push(pattern);
        }

        patterns
    }

    /// Find sentence boundaries around a position
    fn find_sentence_bounds(&self, text: &str, position: usize) -> (usize, usize) {
        let bytes = text.as_bytes();
        const MAX_BACKWARD: usize = 200;
        const MAX_FORWARD: usize = 300;
        
        // Find sentence start
        let mut start = position;
        let mut soft_boundary: Option<usize> = None;
        let search_limit = position.saturating_sub(MAX_BACKWARD);
        
        while start > search_limit {
            let ch = bytes[start - 1];
            if ch == b'.' || ch == b'?' || ch == b'!' {
                break;
            } else if ch == b'\n' && soft_boundary.is_none() {
                soft_boundary = Some(start);
            }
            start -= 1;
        }
        
        if start == search_limit && start > 0 {
            if let Some(soft) = soft_boundary {
                start = soft;
            }
        }
        
        // Find sentence end
        let mut end = position;
        let end_limit = (position + MAX_FORWARD).min(bytes.len());
        
        while end < end_limit {
            let ch = bytes[end];
            if ch == b'.' || ch == b'?' || ch == b'!' || ch == b'\n' {
                end += 1;
                break;
            }
            end += 1;
        }
        
        end = end.min(bytes.len());
        
        (start, end)
    }

    /// Find the nearest entity that ends before the given position, within sentence bounds
    fn find_nearest_entity_before_in_range<'a>(
        &self,
        entities: &'a [EntitySpan],
        position: usize,
        sentence_start: usize,
    ) -> Option<&'a EntitySpan> {
        entities
            .iter()
            .filter(|e| e.end <= position && e.start >= sentence_start)
            .max_by_key(|e| e.end)
    }

    /// Find the nearest entity that starts after the given position, within sentence bounds
    fn find_nearest_entity_after_in_range<'a>(
        &self,
        entities: &'a [EntitySpan],
        position: usize,
        sentence_end: usize,
    ) -> Option<&'a EntitySpan> {
        entities
            .iter()
            .filter(|e| e.start >= position && e.end <= sentence_end)
            .min_by_key(|e| e.start)
    }

    /// Collect PP modifier chunks that follow the VP, within sentence bounds
    fn collect_modifiers_in_range(&self, chunks: &[Chunk], vp_end: usize, sentence_end: usize) -> Vec<Chunk> {
        chunks
            .iter()
            .filter(|c| {
                c.kind == ChunkKind::PrepPhrase && 
                c.range.start >= vp_end &&
                c.range.end <= sentence_end
            })
            .cloned()
            .collect()
    }

    /// Detect if VP is in passive voice
    fn detect_passive(&self, vp: &Chunk, modifiers: &[Chunk], text: &str) -> bool {
        for modifier_range in &vp.modifiers {
            let modifier_text = modifier_range.slice(text).to_lowercase();
            if self.passive_auxiliaries.contains(&modifier_text) {
                let has_by_pp = modifiers.iter().any(|pp| {
                    let pp_text = pp.head.slice(text).to_lowercase();
                    pp_text == "by"
                });
                
                if has_by_pp {
                    return true;
                }
                
                let verb_text = vp.head.slice(text).to_lowercase();
                if verb_text.ends_with("ed") || verb_text.ends_with("en") {
                    return true;
                }
            }
        }
        false
    }

    /// Handle passive voice by flipping subject/object
    fn handle_passive(&self, pattern: &SVOPattern, text: &str) -> Option<SVOPattern> {
        if !pattern.is_passive {
            return None;
        }

        // Find the "by" PP to extract the agent
        let _agent_pp = pattern.modifiers.iter().find(|pp| {
            let prep_text = pp.head.slice(text).to_lowercase();
            prep_text == "by"
        })?;

        // For MVP: we'll just return None and let caller handle
        None
    }

    /// Convert SVO pattern to structured relation
    fn pattern_to_relation(&self, pattern: SVOPattern, text: &str) -> Option<StructuredRelation> {
        let verb_text = pattern.verb.head.slice(text);
        let verb_lower = verb_text.to_lowercase();

        // Get relation type from lexicon, or use verb itself uppercased
        let relation_type = self.lexicon
            .get_relation(&verb_lower)
            .map(|s| s.to_string())
            .unwrap_or_else(|| verb_lower.to_uppercase());

        // Convert PP modifiers to RelationModifiers
        let modifiers: Vec<_> = pattern.modifiers.iter().map(|pp| {
            let prep_text = pp.head.slice(text);
            let full_text = pp.range.slice(text);
            RelationModifier {
                modifier_type: ModifierType::from_preposition(prep_text),
                text: full_text.to_string(),
                span: pp.range,
                preposition: prep_text.to_string(),
            }
        }).collect();

        let confidence = self.calculate_confidence(&pattern);

        Some(StructuredRelation {
            subject: pattern.subject.label.clone(),
            subject_id: pattern.subject.entity_id.clone(),
            subject_span: TextRange::new(pattern.subject.start, pattern.subject.end),
            
            predicate: verb_text.to_string(),
            predicate_span: pattern.verb.range,
            relation_type,
            
            object: pattern.object.as_ref().map(|o| o.label.clone()),
            object_id: pattern.object.as_ref().and_then(|o| o.entity_id.clone()),
            object_span: pattern.object.as_ref().map(|o| TextRange::new(o.start, o.end)),
            
            modifiers,
            passive_transformed: pattern.is_passive,
            confidence,
        })
    }

    /// Calculate confidence score for a pattern
    fn calculate_confidence(&self, pattern: &SVOPattern) -> f64 {
        let mut confidence: f64 = 0.5;

        if pattern.object.is_some() {
            confidence += 0.3;
        }
        
        if pattern.is_passive {
            confidence -= 0.1;
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Set maximum distance for entity-VP matching
    pub fn set_max_distance(&mut self, distance: usize) {
        self.max_distance = distance;
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(label: &str, start: usize, end: usize) -> EntitySpan {
        EntitySpan {
            label: label.to_string(),
            entity_id: Some(format!("id_{}", label.to_lowercase())),
            start,
            end,
            kind: Some("CHARACTER".to_string()),
        }
    }

    fn extract(text: &str, entities: &[EntitySpan]) -> Vec<StructuredRelation> {
        let extractor = StructuredRelationExtractor::new();
        extractor.extract_structured(text, entities)
    }

    #[test]
    fn test_simple_svo() {
        // "Gandalf defeated Sauron"
        //  ^^^^^^^          ^^^^^^
        //  0-7              17-23
        let text = "Gandalf defeated Sauron";
        let entities = vec![
            make_entity("Gandalf", 0, 7),
            make_entity("Sauron", 17, 23),
        ];

        let relations = extract(text, &entities);

        assert_eq!(relations.len(), 1, "Expected 1 relation, got {}", relations.len());
        let rel = &relations[0];
        assert_eq!(rel.subject, "Gandalf");
        assert_eq!(rel.object, Some("Sauron".to_string()));
        assert_eq!(rel.relation_type, "DEFEATED");
    }

    #[test]
    fn test_intransitive_verb() {
        let text = "Frodo walked";
        let entities = vec![
            make_entity("Frodo", 0, 5),
        ];

        let relations = extract(text, &entities);

        assert_eq!(relations.len(), 1);
        let rel = &relations[0];
        assert_eq!(rel.subject, "Frodo");
        assert!(rel.object.is_none());
    }

    #[test]
    fn test_unknown_verb_uses_verb_as_type() {
        // "Bilbo burglarized Smaug"
        let text = "Bilbo burglarized Smaug";
        let entities = vec![
            make_entity("Bilbo", 0, 5),
            make_entity("Smaug", 18, 23),
        ];

        let relations = extract(text, &entities);

        assert_eq!(relations.len(), 1);
        let rel = &relations[0];
        assert_eq!(rel.relation_type, "BURGLARIZED");
    }

    #[test]
    fn test_multiple_vps() {
        let text = "Gandalf defeated Sauron. Frodo saved Sam.";
        let entities = vec![
            make_entity("Gandalf", 0, 7),
            make_entity("Sauron", 17, 23),
            make_entity("Frodo", 25, 30),
            make_entity("Sam", 37, 40),
        ];

        let relations = extract(text, &entities);

        assert!(relations.len() >= 2, "Expected at least 2 relations, got {}", relations.len());
    }

    #[test]
    fn test_passive_voice_detection() {
        let text = "Sauron was defeated by Gandalf";
        let entities = vec![
            make_entity("Sauron", 0, 6),
            make_entity("Gandalf", 23, 30),
        ];

        let extractor = StructuredRelationExtractor::new();
        let chunk_result = extractor.chunker.chunk(text);
        let patterns = extractor.find_svo_patterns(&chunk_result.chunks, &entities, text);

        assert!(!patterns.is_empty(), "Should find SVO pattern");
        let pattern = &patterns[0];
        assert!(pattern.is_passive, "Should detect passive voice");
    }
}
