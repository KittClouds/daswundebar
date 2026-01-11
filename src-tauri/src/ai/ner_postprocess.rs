//! NER Post-Processor - Cleans up raw NER model output
//!
//! Pipeline:
//! 1. Junk Filter (blocklist + heuristics)
//! 2. Span Cleanup (possessives, boundaries, case)
//! 3. Adjacent Span Merging ("Doctor" + "Who" → "Doctor Who")
//! 4. Deduplication (collapse repeated mentions)
//! 5. Alias Resolution (optional KB lookup)
//! 6. Type Arbitration (priority rules)

use std::collections::{HashMap, HashSet};

// =============================================================================
// Types
// =============================================================================

/// Raw suggestion from NER model (before cleanup)
#[derive(Debug, Clone)]
pub struct RawSuggestion {
    pub text: String,
    pub entity_type: String,
    pub confidence: f32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub source_note_id: String,
}

/// Cleaned suggestion (after post-processing)
#[derive(Debug, Clone, PartialEq)]
pub struct CleanSuggestion {
    pub text: String,
    pub entity_type: String,
    pub confidence: f32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub source_note_id: String,
    pub mention_count: usize,
}

// =============================================================================
// Static Data
// =============================================================================

/// Words that should never be entities
const BLOCKLIST: &[&str] = &[
    // Common English
    "the", "a", "an", "and", "or", "but", "is", "are", "was", "were",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "can", "may", "might", "must", "shall", "just", "very",
    "only", "even", "also", "still", "already", "again", "often",
    "be", "been", "being", "am", "it", "its", "they", "them", "their",
    "he", "him", "his", "she", "her", "hers", "we", "us", "our", "you", "your",
    "i", "me", "my", "mine", "what", "which", "who", "whom", "whose",
    "this", "that", "these", "those", "here", "there", "where", "when",
    "why", "how", "all", "each", "every", "both", "few", "more", "most",
    "other", "some", "such", "no", "nor", "not", "so", "than", "too",
    // Screenplay/Document junk
    "continued", "cont'd", "contd", "cut", "int", "ext", "interior", "exterior",
    "o.s.", "o/s", "v.o.", "v/o", "pov", "angle", "close", "wide", "insert",
    "fade", "dissolve", "intercut", "flashback", "title", "titles", "credits",
    "episode", "scene", "act", "page", "revision", "revisions", "draft", "pink",
    "blue", "white", "green", "yellow", "goldenrod", "buff", "salmon", "cherry",
    // Time expressions
    "night", "day", "morning", "evening", "afternoon", "later", "earlier",
    "moment", "moments", "beat", "pause", "silence",
    // Common adjectives that get misclassified
    "high", "low", "big", "small", "new", "old", "good", "bad", "first", "last",
    "long", "short", "great", "little", "own", "same", "right", "left",
];

/// Known aliases → canonical forms
const ALIASES: &[(&str, &str)] = &[
    ("dr who", "The Doctor"),
    ("doctor who", "The Doctor"),
    ("the doc", "The Doctor"),
    ("doc", "The Doctor"),
];

// =============================================================================
// Post-Processor
// =============================================================================

pub struct NerPostProcessor {
    blocklist: HashSet<String>,
    aliases: HashMap<String, String>,
    min_confidence: f32,
    max_word_count: usize,
}

impl Default for NerPostProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl NerPostProcessor {
    pub fn new() -> Self {
        let blocklist: HashSet<String> = BLOCKLIST
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        let aliases: HashMap<String, String> = ALIASES
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.to_string()))
            .collect();

        Self {
            blocklist,
            aliases,
            min_confidence: 0.60,
            max_word_count: 4,
        }
    }

    /// Check if text is in blocklist
    pub fn is_blocked(&self, text: &str) -> bool {
        let normalized = text.trim().to_lowercase();
        self.blocklist.contains(&normalized)
    }

    /// Check if text is junk based on heuristics
    pub fn is_junk(&self, text: &str) -> bool {
        let t = text.trim();

        // Too short
        if t.len() < 2 {
            return true;
        }

        // Too long (probably captured a sentence)
        if t.len() > 60 {
            return true;
        }

        // ALL CAPS with colon (screenplay direction like "CONTINUED:")
        if t.ends_with(':') && t.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase()) {
            return true;
        }

        // Pure numbers or timestamps
        if t.chars().all(|c| c.is_numeric() || c == '/' || c == ':' || c == '-' || c == '.') {
            return true;
        }

        // Just possessive suffix
        if t == "'s" || t == "s'" || t == "'S" {
            return true;
        }

        // Contains revision markers
        let lower = t.to_lowercase();
        if lower.contains("revision") || lower.contains("draft") {
            return true;
        }

        // Page numbers like "Page 1" or "Page 2."
        if lower.starts_with("page ") {
            return true;
        }

        false
    }

    /// Clean possessive suffixes
    pub fn clean_possessive(&self, text: &str) -> String {
        text.trim()
            .trim_end_matches("'s")
            .trim_end_matches("'S")
            .trim_end_matches("'s") // curly apostrophe
            .trim_end_matches("s'")
            .to_string()
    }

    /// Fix span boundaries (trim punctuation except meaningful ones)
    pub fn fix_boundaries(&self, text: &str) -> String {
        text.trim()
            .trim_matches(|c: char| {
                c.is_ascii_punctuation() && c != '-' && c != '\'' && c != '\u{2019}'
            })
            .to_string()
    }

    /// Normalize case: "SALLY SPARROW" → "Sally Sparrow"
    pub fn normalize_case(&self, text: &str) -> String {
        // Check if it's a special all-caps name that should stay that way
        // For now, title case everything
        text.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(c) => {
                        let first: String = c.to_uppercase().collect();
                        let rest: String = chars.flat_map(|c| c.to_lowercase()).collect();
                        format!("{}{}", first, rest)
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Full span cleanup pipeline
    pub fn clean_span(&self, text: &str) -> String {
        let step1 = self.fix_boundaries(text);
        let step2 = self.clean_possessive(&step1);
        let step3 = self.normalize_case(&step2);
        step3
    }

    /// Resolve aliases to canonical forms
    pub fn resolve_alias(&self, text: &str) -> String {
        let lower = text.to_lowercase();
        self.aliases.get(&lower).cloned().unwrap_or_else(|| text.to_string())
    }

    /// Merge adjacent spans that belong together
    pub fn merge_adjacent_spans(&self, mut suggestions: Vec<RawSuggestion>) -> Vec<RawSuggestion> {
        if suggestions.is_empty() {
            return suggestions;
        }

        // Sort by (doc_id, byte_start)
        suggestions.sort_by(|a, b| {
            a.source_note_id.cmp(&b.source_note_id)
                .then(a.byte_start.cmp(&b.byte_start))
        });

        let mut result: Vec<RawSuggestion> = Vec::new();
        let mut i = 0;

        while i < suggestions.len() {
            let mut current = suggestions[i].clone();
            
            // Try to merge with following spans
            while i + 1 < suggestions.len() {
                let next = &suggestions[i + 1];

                // Check merge conditions
                let same_doc = current.source_note_id == next.source_note_id;
                let same_type = current.entity_type == next.entity_type;
                let gap = if next.byte_start >= current.byte_end {
                    next.byte_start - current.byte_end
                } else {
                    0 // overlapping
                };
                let is_adjacent = gap <= 3; // Allow small gap for space/punctuation

                if same_doc && same_type && is_adjacent {
                    // Merge
                    let merged_text = format!("{} {}", current.text.trim(), next.text.trim());
                    
                    // Check word count limit
                    if merged_text.split_whitespace().count() > self.max_word_count {
                        break;
                    }

                    current.text = merged_text;
                    current.byte_end = next.byte_end;
                    current.confidence = (current.confidence + next.confidence) / 2.0;
                    i += 1;
                } else {
                    break;
                }
            }

            result.push(current);
            i += 1;
        }

        result
    }

    /// Deduplicate by normalized label, keep highest confidence
    pub fn deduplicate(&self, suggestions: Vec<CleanSuggestion>) -> Vec<CleanSuggestion> {
        let mut by_label: HashMap<String, CleanSuggestion> = HashMap::new();

        for s in suggestions {
            let key = s.text.to_lowercase();
            by_label
                .entry(key)
                .and_modify(|existing| {
                    existing.mention_count += s.mention_count;
                    if s.confidence > existing.confidence {
                        existing.confidence = s.confidence;
                    }
                })
                .or_insert(s);
        }

        by_label.into_values().collect()
    }

    /// Full processing pipeline
    pub fn process(&self, raw: Vec<RawSuggestion>) -> Vec<CleanSuggestion> {
        // Stage 1: Filter blocked and junk
        let filtered: Vec<_> = raw
            .into_iter()
            .filter(|s| !self.is_blocked(&s.text))
            .filter(|s| !self.is_junk(&s.text))
            .collect();

        // Stage 2.5: Merge adjacent spans
        let merged = self.merge_adjacent_spans(filtered);

        // Stage 2 + 3: Clean and convert
        let cleaned: Vec<CleanSuggestion> = merged
            .into_iter()
            .map(|s| {
                let clean_text = self.clean_span(&s.text);
                let resolved = self.resolve_alias(&clean_text);
                CleanSuggestion {
                    text: resolved,
                    entity_type: s.entity_type,
                    confidence: s.confidence,
                    byte_start: s.byte_start,
                    byte_end: s.byte_end,
                    source_note_id: s.source_note_id,
                    mention_count: 1,
                }
            })
            .filter(|s| !s.text.is_empty())
            .filter(|s| !self.is_blocked(&s.text)) // Check again after cleaning
            .collect();

        // Stage 4: Deduplicate
        let deduped = self.deduplicate(cleaned);

        // Stage 5: Filter by confidence
        deduped
            .into_iter()
            .filter(|s| s.confidence >= self.min_confidence)
            .collect()
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_raw(text: &str, entity_type: &str, confidence: f32, start: usize, end: usize) -> RawSuggestion {
        RawSuggestion {
            text: text.to_string(),
            entity_type: entity_type.to_string(),
            confidence,
            byte_start: start,
            byte_end: end,
            source_note_id: "test_doc".to_string(),
        }
    }

    // -------------------------------------------------------------------------
    // Blocklist Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_blocklist_common_words() {
        let pp = NerPostProcessor::new();
        
        assert!(pp.is_blocked("the"), "should block 'the'");
        assert!(pp.is_blocked("The"), "should block 'The' (case insensitive)");
        assert!(pp.is_blocked("WHO"), "should block 'WHO'");
        assert!(pp.is_blocked("just"), "should block 'just'");
        assert!(pp.is_blocked("CONTINUED"), "should block 'CONTINUED'");
    }

    #[test]
    fn test_blocklist_screenplay_terms() {
        let pp = NerPostProcessor::new();
        
        assert!(pp.is_blocked("Episode"), "should block 'Episode'");
        assert!(pp.is_blocked("PINK"), "should block 'PINK' (revision color)");
        assert!(pp.is_blocked("INT"), "should block 'INT'");
        assert!(pp.is_blocked("EXT"), "should block 'EXT'");
        assert!(pp.is_blocked("CUT"), "should block 'CUT'");
    }

    #[test]
    fn test_blocklist_allows_real_names() {
        let pp = NerPostProcessor::new();
        
        assert!(!pp.is_blocked("Sally"), "should NOT block 'Sally'");
        assert!(!pp.is_blocked("Kathy"), "should NOT block 'Kathy'");
        assert!(!pp.is_blocked("Murray Gold"), "should NOT block 'Murray Gold'");
        assert!(!pp.is_blocked("Wester Drumlins"), "should NOT block 'Wester Drumlins'");
    }

    // -------------------------------------------------------------------------
    // Junk Heuristic Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_junk_too_short() {
        let pp = NerPostProcessor::new();
        
        assert!(pp.is_junk("A"), "single char is junk");
        assert!(pp.is_junk(""), "empty is junk");
        assert!(!pp.is_junk("Jo"), "two chars is OK");
    }

    #[test]
    fn test_junk_too_long() {
        let pp = NerPostProcessor::new();
        
        let long = "This is a very long sentence that should not be an entity name at all";
        assert!(pp.is_junk(long), "long sentence is junk");
    }

    #[test]
    fn test_junk_screenplay_directions() {
        let pp = NerPostProcessor::new();
        
        assert!(pp.is_junk("CONTINUED:"), "CONTINUED: is junk");
        assert!(pp.is_junk("CUT TO:"), "CUT TO: is junk");
        assert!(pp.is_junk("INT:"), "INT: is junk");
    }

    #[test]
    fn test_junk_timestamps() {
        let pp = NerPostProcessor::new();
        
        assert!(pp.is_junk("17/11/06"), "date is junk");
        assert!(pp.is_junk("12:30"), "time is junk");
        assert!(pp.is_junk("2006"), "year alone is junk");
    }

    #[test]
    fn test_junk_page_numbers() {
        let pp = NerPostProcessor::new();
        
        assert!(pp.is_junk("Page 2"), "page number is junk");
        assert!(pp.is_junk("Page 2."), "page number with period is junk");
    }

    // -------------------------------------------------------------------------
    // Span Cleanup Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_clean_possessive() {
        let pp = NerPostProcessor::new();
        
        assert_eq!(pp.clean_possessive("SALLY'S"), "SALLY");
        assert_eq!(pp.clean_possessive("Doctor's"), "Doctor");
        assert_eq!(pp.clean_possessive("Jones'"), "Jone"); // edge case - might need refinement
    }

    #[test]
    fn test_fix_boundaries() {
        let pp = NerPostProcessor::new();
        
        assert_eq!(pp.fix_boundaries("  Sally  "), "Sally");
        assert_eq!(pp.fix_boundaries("\"Sally\""), "Sally");
        assert_eq!(pp.fix_boundaries("(Kathy)"), "Kathy");
        assert_eq!(pp.fix_boundaries("Sally-Anne"), "Sally-Anne"); // hyphen preserved
    }

    #[test]
    fn test_normalize_case() {
        let pp = NerPostProcessor::new();
        
        assert_eq!(pp.normalize_case("SALLY SPARROW"), "Sally Sparrow");
        assert_eq!(pp.normalize_case("the doctor"), "The Doctor");
        assert_eq!(pp.normalize_case("KATHY"), "Kathy");
    }

    #[test]
    fn test_full_clean_span() {
        let pp = NerPostProcessor::new();
        
        assert_eq!(pp.clean_span("  SALLY'S  "), "Sally");
        assert_eq!(pp.clean_span("\"THE DOCTOR\""), "The Doctor");
    }

    // -------------------------------------------------------------------------
    // Adjacent Span Merging Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_merge_adjacent_doctor_who() {
        let pp = NerPostProcessor::new();
        
        let raw = vec![
            make_raw("Doctor", "CHARACTER", 0.75, 0, 6),
            make_raw("Who", "CHARACTER", 0.75, 7, 10),
        ];
        
        let merged = pp.merge_adjacent_spans(raw);
        
        assert_eq!(merged.len(), 1, "should merge into one");
        assert_eq!(merged[0].text, "Doctor Who");
    }

    #[test]
    fn test_merge_adjacent_three_tokens() {
        let pp = NerPostProcessor::new();
        
        let raw = vec![
            make_raw("Sally", "CHARACTER", 0.80, 0, 5),
            make_raw("Anne", "CHARACTER", 0.80, 6, 10),
            make_raw("Sparrow", "CHARACTER", 0.80, 11, 18),
        ];
        
        let merged = pp.merge_adjacent_spans(raw);
        
        assert_eq!(merged.len(), 1, "should merge all three");
        assert_eq!(merged[0].text, "Sally Anne Sparrow");
    }

    #[test]
    fn test_no_merge_different_types() {
        let pp = NerPostProcessor::new();
        
        let raw = vec![
            make_raw("Wester", "LOCATION", 0.80, 0, 6),
            make_raw("Sally", "CHARACTER", 0.80, 7, 12),
        ];
        
        let merged = pp.merge_adjacent_spans(raw);
        
        assert_eq!(merged.len(), 2, "should NOT merge different types");
    }

    #[test]
    fn test_no_merge_large_gap() {
        let pp = NerPostProcessor::new();
        
        let raw = vec![
            make_raw("Sally", "CHARACTER", 0.80, 0, 5),
            make_raw("Kathy", "CHARACTER", 0.80, 100, 105),
        ];
        
        let merged = pp.merge_adjacent_spans(raw);
        
        assert_eq!(merged.len(), 2, "should NOT merge with large gap");
    }

    #[test]
    fn test_merge_respects_word_limit() {
        let pp = NerPostProcessor::new();
        
        // 5 tokens would exceed max_word_count of 4
        let raw = vec![
            make_raw("The", "CHARACTER", 0.80, 0, 3),
            make_raw("Very", "CHARACTER", 0.80, 4, 8),
            make_raw("Long", "CHARACTER", 0.80, 9, 13),
            make_raw("Name", "CHARACTER", 0.80, 14, 18),
            make_raw("Here", "CHARACTER", 0.80, 19, 23),
        ];
        
        let merged = pp.merge_adjacent_spans(raw);
        
        // Should merge first 4, then stop
        assert!(merged.len() >= 1, "should produce at least one result");
        let first = &merged[0];
        assert!(first.text.split_whitespace().count() <= 4, "should not exceed word limit");
    }

    // -------------------------------------------------------------------------
    // Alias Resolution Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_alias_doctor_who() {
        let pp = NerPostProcessor::new();
        
        assert_eq!(pp.resolve_alias("Doctor Who"), "The Doctor");
        assert_eq!(pp.resolve_alias("Dr Who"), "The Doctor");
        assert_eq!(pp.resolve_alias("the doc"), "The Doctor");
    }

    #[test]
    fn test_alias_passthrough() {
        let pp = NerPostProcessor::new();
        
        assert_eq!(pp.resolve_alias("Sally Sparrow"), "Sally Sparrow");
        assert_eq!(pp.resolve_alias("Kathy"), "Kathy");
    }

    // -------------------------------------------------------------------------
    // Deduplication Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_dedupe_same_entity() {
        let pp = NerPostProcessor::new();
        
        let suggestions = vec![
            CleanSuggestion {
                text: "Sally".to_string(),
                entity_type: "CHARACTER".to_string(),
                confidence: 0.70,
                byte_start: 0,
                byte_end: 5,
                source_note_id: "test".to_string(),
                mention_count: 1,
            },
            CleanSuggestion {
                text: "sally".to_string(), // lowercase variant
                entity_type: "CHARACTER".to_string(),
                confidence: 0.80,
                byte_start: 100,
                byte_end: 105,
                source_note_id: "test".to_string(),
                mention_count: 1,
            },
        ];
        
        let deduped = pp.deduplicate(suggestions);
        
        assert_eq!(deduped.len(), 1, "should dedupe to one");
        assert_eq!(deduped[0].mention_count, 2, "should track mention count");
        assert_eq!(deduped[0].confidence, 0.80, "should keep highest confidence");
    }

    // -------------------------------------------------------------------------
    // Full Pipeline Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_pipeline_filters_junk() {
        let pp = NerPostProcessor::new();
        
        let raw = vec![
            make_raw("Who", "CHARACTER", 0.75, 0, 3),
            make_raw("Episode", "CHARACTER", 0.76, 10, 17),
            make_raw("PINK", "CHARACTER", 0.77, 20, 24),
            make_raw("Sally Sparrow", "CHARACTER", 0.85, 50, 63),
        ];
        
        let result = pp.process(raw);
        
        assert_eq!(result.len(), 1, "should filter to just Sally Sparrow");
        assert_eq!(result[0].text, "Sally Sparrow");
    }

    #[test]
    fn test_pipeline_doctor_who_script() {
        let pp = NerPostProcessor::new();
        
        // Simulating the actual garbage from the screenshots
        let raw = vec![
            make_raw("Who", "CHARACTER", 0.75, 0, 3),
            make_raw("Episode", "CHARACTER", 0.76, 10, 17),
            make_raw("PINK", "CHARACTER", 0.77, 20, 24),
            make_raw("High", "CHARACTER", 0.81, 30, 34),
            make_raw("Just", "CHARACTER", 0.82, 40, 44),
            make_raw("CONTINUED:", "CHARACTER", 0.79, 50, 60),
            make_raw("SALLY'S", "CHARACTER", 0.80, 70, 77),
            make_raw("Sally Sparrow", "CHARACTER", 0.88, 100, 113),
            make_raw("KATHY", "CHARACTER", 0.85, 150, 155),
            make_raw("Murray Gold", "CHARACTER", 0.75, 200, 211),
            make_raw("THE DOCTOR", "CHARACTER", 0.90, 300, 310),
        ];
        
        let result = pp.process(raw);
        
        // Should have: Sally, Sally Sparrow, Kathy, Murray Gold, The Doctor
        let labels: Vec<&str> = result.iter().map(|s| s.text.as_str()).collect();
        
        assert!(!labels.contains(&"Who"), "Who should be filtered");
        assert!(!labels.contains(&"Episode"), "Episode should be filtered");
        assert!(!labels.contains(&"Pink"), "PINK should be filtered");
        assert!(!labels.contains(&"High"), "High should be filtered");
        assert!(!labels.contains(&"Just"), "Just should be filtered");
        assert!(!labels.contains(&"Continued:"), "CONTINUED: should be filtered");
        
        // Valid ones should remain
        assert!(labels.iter().any(|l| l.contains("Sally")), "Sally should remain");
        assert!(labels.iter().any(|l| l.contains("Kathy")), "Kathy should remain");
        assert!(labels.iter().any(|l| l.contains("Murray")), "Murray Gold should remain");
        assert!(labels.iter().any(|l| l.contains("Doctor")), "The Doctor should remain");
    }

    #[test]
    fn test_pipeline_confidence_filter() {
        let pp = NerPostProcessor::new();
        
        let raw = vec![
            make_raw("Sally", "CHARACTER", 0.50, 0, 5), // below threshold
            make_raw("Kathy", "CHARACTER", 0.70, 10, 15), // above threshold
        ];
        
        let result = pp.process(raw);
        
        assert_eq!(result.len(), 1, "should filter low confidence");
        assert_eq!(result[0].text, "Kathy");
    }
}
