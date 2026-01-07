//! TripleCortex - Triple Syntax Extraction (Native Tauri)
//!
//! Extracts explicit relationship triples from text using multiple syntaxes:
//! - Wiki-style: [[Source->PREDICATE->Target]]
//! - Forward arrow: Source ->RELATION-> Target
//! - Backward arrow: Target <-RELATION<- Source
//! - Bidirectional: A <->RELATION<-> B
//! - Parenthesized: [KIND|Label] (RELATION) [KIND|Label]
//! - Arrow with kinds: [KIND|Label] ->RELATION-> [KIND|Label]

use regex::Regex;
use serde::{Deserialize, Serialize};

// =============================================================================
// Types
// =============================================================================

/// An extracted triple relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedTriple {
    pub source: String,
    pub predicate: String,
    pub target: String,
    pub start: usize,
    pub end: usize,
    pub raw_text: String,
    /// Optional: entity kind for source (if explicit syntax used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    /// Optional: entity kind for target (if explicit syntax used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<String>,
}

// =============================================================================
// TripleCortex
// =============================================================================

/// Triple syntax extractor - supports multiple syntaxes
pub struct TripleCortex {
    // Pattern 1: [[source->predicate->target]] (wiki-style)
    wiki_regex: Regex,
    // Pattern 2: Source ->RELATION-> Target (forward arrow)
    forward_regex: Regex,
    // Pattern 3: Target <-RELATION<- Source (backward arrow)
    backward_regex: Regex,
    // Pattern 4: A <->RELATION<-> B (bidirectional)
    bidir_regex: Regex,
    // Pattern 5: [KIND|Label] (RELATION) [KIND|Label] (parenthesized)
    paren_regex: Regex,
    // Pattern 6: [KIND|Label] ->RELATION-> [KIND|Label] (arrow with kinds)
    arrow_kind_regex: Regex,
}

impl Default for TripleCortex {
    fn default() -> Self {
        Self::new()
    }
}

impl TripleCortex {
    pub fn new() -> Self {
        // Pattern 1: [[source->pred->target]] (wiki-style)
        let wiki_regex = Regex::new(
            r"\[\[\s*([^\[\]>]+?)\s*->\s*([^\[\]>]+?)\s*->\s*([^\[\]>]+?)\s*\]\]"
        ).expect("Wiki triple regex should compile");

        // Pattern 2: Source ->RELATION-> Target (forward arrow)
        let forward_regex = Regex::new(
            r"(\S[^<>\[\]\n]*?)\s*->([A-Z][A-Z0-9_]*)->\s*([^<>\[\]\n]*\S)"
        ).expect("Forward arrow regex should compile");

        // Pattern 3: Target <-RELATION<- Source (backward arrow)
        let backward_regex = Regex::new(
            r"(\S[^<>\[\]\n]*?)\s*<-([A-Z][A-Z0-9_]*)<-\s*([^<>\[\]\n]*\S)"
        ).expect("Backward arrow regex should compile");

        // Pattern 4: A <->RELATION<-> B (bidirectional)
        let bidir_regex = Regex::new(
            r"(\S[^<>\[\]\n]*?)\s*<->([A-Z][A-Z0-9_]*)<->\s*([^<>\[\]\n]*\S)"
        ).expect("Bidirectional arrow regex should compile");

        // Pattern 5: [KIND|Label] (RELATION) [KIND|Label] (parenthesized)
        let paren_regex = Regex::new(
            r"(?m)\[([A-Z_]+)\|([^\]]+)\][\s\n]*\(([A-Za-z][A-Za-z0-9_]*)\)[\s\n]*\[([A-Z_]+)\|([^\]]+)\]"
        ).expect("Parenthesized triple regex should compile");

        // Pattern 6: [KIND|Label] ->RELATION-> [KIND|Label] (arrow with kinds)
        let arrow_kind_regex = Regex::new(
            r"\[([A-Z_]+)\|([^\]]+)\]\s*->([A-Z][A-Z0-9_]*)->\s*\[([A-Z_]+)\|([^\]]+)\]"
        ).expect("Arrow-kind triple regex should compile");

        Self { wiki_regex, forward_regex, backward_regex, bidir_regex, paren_regex, arrow_kind_regex }
    }

    /// Extract all triples from text using all supported syntaxes
    pub fn extract(&self, text: &str) -> Vec<ExtractedTriple> {
        let mut triples = Vec::new();
        let mut matched_ranges: Vec<(usize, usize)> = Vec::new();

        // Pattern 5: [KIND|Label] (RELATION) [KIND|Label] (parenthesized - HIGHEST PRIORITY)
        for cap in self.paren_regex.captures_iter(text) {
            if let Some(triple) = Self::extract_kind_triple(&cap) {
                matched_ranges.push((triple.start, triple.end));
                triples.push(triple);
            }
        }

        // Pattern 6: [KIND|Label] ->RELATION-> [KIND|Label] (arrow with kinds)
        for cap in self.arrow_kind_regex.captures_iter(text) {
            if let Some(triple) = Self::extract_kind_triple(&cap) {
                let start = triple.start;
                let end = triple.end;
                let overlaps = matched_ranges.iter().any(|(s, e)| {
                    (start >= *s && start < *e) || (end > *s && end <= *e)
                });
                if !overlaps {
                    matched_ranges.push((start, end));
                    triples.push(triple);
                }
            }
        }

        // Pattern 1: [[source->pred->target]] (wiki-style)
        for cap in self.wiki_regex.captures_iter(text) {
            if let Some(triple) = Self::extract_wiki_triple(&cap) {
                let start = triple.start;
                let end = triple.end;
                let overlaps = matched_ranges.iter().any(|(s, e)| {
                    (start >= *s && start < *e) || (end > *s && end <= *e)
                });
                if !overlaps {
                    matched_ranges.push((start, end));
                    triples.push(triple);
                }
            }
        }

        // Helper: check if a position should be skipped
        let should_skip = |start: usize, end: usize| -> bool {
            matched_ranges.iter().any(|(s, e)| {
                (start >= *s && start < *e) || (end > *s && end <= *e)
            })
        };

        // Pattern 2: Source ->RELATION-> Target (forward arrow)
        for cap in self.forward_regex.captures_iter(text) {
            if let Some(triple) = Self::extract_arrow_triple(&cap, false) {
                if !should_skip(triple.start, triple.end) {
                    triples.push(triple);
                }
            }
        }

        // Pattern 3: Target <-RELATION<- Source (backward arrow - swap source/target)
        for cap in self.backward_regex.captures_iter(text) {
            if let Some(triple) = Self::extract_arrow_triple(&cap, true) {
                if !should_skip(triple.start, triple.end) {
                    triples.push(triple);
                }
            }
        }

        // Pattern 4: A <->RELATION<-> B (bidirectional - emit both directions)
        for cap in self.bidir_regex.captures_iter(text) {
            if let Some(triple) = Self::extract_arrow_triple(&cap, false) {
                if !should_skip(triple.start, triple.end) {
                    // Forward direction
                    triples.push(triple.clone());
                    // Reverse direction
                    triples.push(ExtractedTriple {
                        source: triple.target.clone(),
                        target: triple.source.clone(),
                        ..triple
                    });
                }
            }
        }

        // Sort by position and dedupe
        triples.sort_by_key(|t| t.start);
        triples.dedup_by(|a, b| a.start == b.start && a.source == b.source && a.target == b.target);
        triples
    }

    fn extract_wiki_triple(cap: &regex::Captures) -> Option<ExtractedTriple> {
        let full_match = cap.get(0)?;
        let source = cap.get(1)?.as_str().trim();
        let predicate = cap.get(2)?.as_str().trim();
        let target = cap.get(3)?.as_str().trim();

        if source.is_empty() || predicate.is_empty() || target.is_empty() {
            return None;
        }

        Some(ExtractedTriple {
            source: source.to_string(),
            predicate: predicate.to_string(),
            target: target.to_string(),
            start: full_match.start(),
            end: full_match.end(),
            raw_text: full_match.as_str().to_string(),
            source_kind: None,
            target_kind: None,
        })
    }

    /// Extract kind-annotated triple: [KIND|Label] (REL) [KIND|Label] or [KIND|Label] ->REL-> [KIND|Label]
    fn extract_kind_triple(cap: &regex::Captures) -> Option<ExtractedTriple> {
        let full_match = cap.get(0)?;
        let source_kind = cap.get(1)?.as_str().trim();
        let source_label = cap.get(2)?.as_str().trim();
        let predicate = cap.get(3)?.as_str().trim();
        let target_kind = cap.get(4)?.as_str().trim();
        let target_label = cap.get(5)?.as_str().trim();

        if source_label.is_empty() || predicate.is_empty() || target_label.is_empty() {
            return None;
        }

        Some(ExtractedTriple {
            source: source_label.to_string(),
            predicate: predicate.to_string(),
            target: target_label.to_string(),
            start: full_match.start(),
            end: full_match.end(),
            raw_text: full_match.as_str().to_string(),
            source_kind: Some(source_kind.to_string()),
            target_kind: Some(target_kind.to_string()),
        })
    }

    /// Extract arrow-style triple: Source ->REL-> Target or Target <-REL<- Source
    fn extract_arrow_triple(cap: &regex::Captures, swap: bool) -> Option<ExtractedTriple> {
        let full_match = cap.get(0)?;
        let first = cap.get(1)?.as_str().trim();
        let predicate = cap.get(2)?.as_str().trim();
        let second = cap.get(3)?.as_str().trim();

        if first.is_empty() || predicate.is_empty() || second.is_empty() {
            return None;
        }

        // For backward arrows, swap: source=second, target=first
        let (source, target) = if swap {
            (second, first)
        } else {
            (first, second)
        };

        Some(ExtractedTriple {
            source: source.to_string(),
            predicate: predicate.to_string(),
            target: target.to_string(),
            start: full_match.start(),
            end: full_match.end(),
            raw_text: full_match.as_str().to_string(),
            source_kind: None,
            target_kind: None,
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_wiki_triple() {
        let cortex = TripleCortex::new();
        let triples = cortex.extract("[[Frodo->OWNS->Ring]]");

        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].source, "Frodo");
        assert_eq!(triples[0].predicate, "OWNS");
        assert_eq!(triples[0].target, "Ring");
    }

    #[test]
    fn test_whitespace_handling() {
        let cortex = TripleCortex::new();
        let triples = cortex.extract("[[ Frodo  ->  OWNS  ->  Ring ]]");

        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].source, "Frodo");
    }

    #[test]
    fn test_position_tracking() {
        let cortex = TripleCortex::new();
        let text = "Some text [[A->B->C]] more text";
        let triples = cortex.extract(text);

        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].start, 10);
        assert_eq!(triples[0].end, 21);
    }

    #[test]
    fn test_forward_arrow() {
        let cortex = TripleCortex::new();
        let triples = cortex.extract("Luffy ->CAPTAIN_OF-> Straw Hats");

        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].source, "Luffy");
        assert_eq!(triples[0].predicate, "CAPTAIN_OF");
        assert_eq!(triples[0].target, "Straw Hats");
    }

    #[test]
    fn test_backward_arrow() {
        let cortex = TripleCortex::new();
        let triples = cortex.extract("Straw Hats <-CAPTAIN_OF<- Luffy");

        assert_eq!(triples.len(), 1);
        // Backward swaps: source=Luffy, target=Straw Hats
        assert_eq!(triples[0].source, "Luffy");
        assert_eq!(triples[0].target, "Straw Hats");
    }

    #[test]
    fn test_bidirectional_arrow() {
        let cortex = TripleCortex::new();
        let triples = cortex.extract("Luffy <->ALLIES<-> Zoro");

        // Bidirectional emits 2 triples
        assert_eq!(triples.len(), 2);
        assert_eq!(triples[0].source, "Luffy");
        assert_eq!(triples[0].target, "Zoro");
        assert_eq!(triples[1].source, "Zoro");
        assert_eq!(triples[1].target, "Luffy");
    }

    #[test]
    fn test_parenthesized_triple() {
        let cortex = TripleCortex::new();
        let triples = cortex.extract("[CHARACTER|Lilith] (DISCOVERS) [LOCATION|Ancient Facility]");

        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].source, "Lilith");
        assert_eq!(triples[0].predicate, "DISCOVERS");
        assert_eq!(triples[0].target, "Ancient Facility");
        assert_eq!(triples[0].source_kind, Some("CHARACTER".to_string()));
        assert_eq!(triples[0].target_kind, Some("LOCATION".to_string()));
    }

    #[test]
    fn test_mixed_case_predicate() {
        let cortex = TripleCortex::new();
        let triples = cortex.extract("[CHARACTER|Usopp] (ADMires) [FACTION|Giant Warrior Pirates]");

        assert_eq!(triples.len(), 1);
        assert_eq!(triples[0].predicate, "ADMires");
    }

    #[test]
    fn test_multiple_triples() {
        let cortex = TripleCortex::new();
        let text = "[[Frodo->OWNS->Ring]] and [[Sam->FRIEND_OF->Frodo]]";
        let triples = cortex.extract(text);

        assert_eq!(triples.len(), 2);
    }

    #[test]
    fn test_empty_text() {
        let cortex = TripleCortex::new();
        assert!(cortex.extract("").is_empty());
    }

    #[test]
    fn test_no_triples() {
        let cortex = TripleCortex::new();
        assert!(cortex.extract("Just some regular text without triples").is_empty());
    }
}
