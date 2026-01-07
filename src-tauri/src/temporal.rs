//! TemporalCortex - Temporal Pattern Detection (Native Tauri)
//!
//! Detects temporal expressions in O(n) time using Aho-Corasick
//! for Unicode-safe matching. Supports custom calendar integration.
//!
//! # Categories (126+ patterns)
//! - WEEKDAY: monday, tue, wed, etc.
//! - MONTH: january, jan, etc.
//! - TIME_OF_DAY: morning, dusk, midnight, etc.
//! - NARRATIVE_MARKER: chapter, scene, act, etc.
//! - RELATIVE: "later that day", "the next morning", etc.
//! - CONNECTOR: before, after, during, etc.
//! - ERA: "third age", ad, bc, stardate, etc.

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==================== TYPE DEFINITIONS ====================

/// Kind of temporal pattern detected
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum TemporalKind {
    Weekday,
    Month,
    TimeOfDay,
    NarrativeMarker,
    Relative,
    Connector,
    Era,
    Custom,
}

impl TemporalKind {
    fn as_str(&self) -> &'static str {
        match self {
            TemporalKind::Weekday => "WEEKDAY",
            TemporalKind::Month => "MONTH",
            TemporalKind::TimeOfDay => "TIME_OF_DAY",
            TemporalKind::NarrativeMarker => "NARRATIVE_MARKER",
            TemporalKind::Relative => "RELATIVE",
            TemporalKind::Connector => "CONNECTOR",
            TemporalKind::Era => "ERA",
            TemporalKind::Custom => "CUSTOM",
        }
    }
    
    fn confidence(&self) -> f64 {
        match self {
            TemporalKind::NarrativeMarker => 0.95,
            TemporalKind::Weekday => 0.90,
            TemporalKind::Month => 0.90,
            TemporalKind::Era => 0.90,
            TemporalKind::TimeOfDay => 0.85,
            TemporalKind::Relative => 0.80,
            TemporalKind::Connector => 0.70,
            TemporalKind::Custom => 0.85,
        }
    }
}

/// Metadata extracted from temporal mentions
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TemporalMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekday_index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month_index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub narrative_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub era_year: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub era_name: Option<String>,
}

/// A single temporal mention result
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TemporalMention {
    pub kind: String,
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<TemporalMetadata>,
}

/// Check if a match is at word boundaries
fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    
    // Convert byte positions to char positions
    let mut byte_pos = 0;
    let mut start_char_idx = None;
    let mut end_char_idx = None;
    
    for (i, c) in text_chars.iter().enumerate() {
        if byte_pos == start {
            start_char_idx = Some(i);
        }
        if byte_pos == end {
            end_char_idx = Some(i);
        }
        byte_pos += c.len_utf8();
    }
    if end == text.len() {
        end_char_idx = Some(text_chars.len());
    }
    
    let start_char = start_char_idx.unwrap_or(0);
    let end_char = end_char_idx.unwrap_or(text_chars.len());
    
    // Check character before start
    if start_char > 0 {
        let before = text_chars[start_char - 1];
        if before.is_alphanumeric() {
            return false;
        }
    }
    
    // Check character after end
    if end_char < text_chars.len() {
        let after = text_chars[end_char];
        if after.is_alphanumeric() {
            return false;
        }
    }
    
    true
}

/// Scan result with statistics
#[derive(Serialize, Deserialize)]
pub struct TemporalScanResult {
    pub mentions: Vec<TemporalMention>,
    pub stats: TemporalScanStats,
}

#[derive(Serialize, Deserialize)]
pub struct TemporalScanStats {
    pub patterns_matched: usize,
    pub scan_time_ms: f64,
}

// ==================== DICTIONARIES ====================

#[derive(Clone)]
struct PatternMeta {
    kind: TemporalKind,
    weekday_idx: Option<u8>,
    month_idx: Option<u8>,
    direction: Option<String>,
}

// Weekdays
const WEEKDAYS: &[(&str, u8)] = &[
    ("monday", 0), ("mon", 0),
    ("tuesday", 1), ("tue", 1),
    ("wednesday", 2), ("wed", 2),
    ("thursday", 3), ("thu", 3),
    ("friday", 4), ("fri", 4),
    ("saturday", 5), ("sat", 5),
    ("sunday", 6), ("sun", 6),
];

// Months
const MONTHS: &[(&str, u8)] = &[
    ("january", 0), ("jan", 0),
    ("february", 1), ("feb", 1),
    ("march", 2), ("mar", 2),
    ("april", 3), ("apr", 3),
    ("may", 4),
    ("june", 5), ("jun", 5),
    ("july", 6), ("jul", 6),
    ("august", 7), ("aug", 7),
    ("september", 8), ("sep", 8),
    ("october", 9), ("oct", 9),
    ("november", 10), ("nov", 10),
    ("december", 11), ("dec", 11),
];

// Narrative markers
const NARRATIVE_MARKERS: &[&str] = &[
    "chapter", "ch.", "scene", "act", "part", "book",
    "episode", "ep.", "sequence", "prologue", "epilogue", "interlude",
];

// Relative phrases
const RELATIVE_PHRASES: &[&str] = &[
    "later that day", "later that night", "later that evening", "later that morning",
    "that morning", "that afternoon", "that evening", "that night",
    "earlier that day", "earlier that morning", "earlier that evening",
    "the next day", "the next morning", "the next evening", "the next night",
    "the next week", "the next month", "the next year",
    "next morning", "next evening", "next night", "next week", "next month", "next year",
    "the following day", "the following morning", "the following evening",
    "the following week", "the following month", "the following year",
    "the previous day", "the previous morning", "the previous evening",
    "the day before", "the night before", "the week before",
    "meanwhile", "at the same time", "simultaneously", "in the meantime",
    "at that moment", "at that very moment", "just then",
    "moments later", "hours later", "days later", "weeks later", "months later", "years later",
    "a moment later", "an hour later", "a day later", "a week later", "a month later", "a year later",
    "some time later", "shortly after", "shortly before",
    "long ago", "once upon a time", "in the beginning", "at the end",
    "eventually", "soon", "finally", "at last", "in time",
    "ages ago", "not long after", "before long",
];

// Time of day
const TIME_OF_DAY: &[&str] = &[
    "morning", "afternoon", "evening", "night", "midnight", "noon", "midday",
    "dawn", "dusk", "twilight", "sunrise", "sunset", "nightfall", "daybreak",
    "early morning", "late morning", "early afternoon", "late afternoon",
    "late evening", "late night",
];

// Temporal connectors
const CONNECTORS_BEFORE: &[&str] = &[
    "before", "prior to", "preceding", "just before", "right before",
    "immediately before", "long before",
];

const CONNECTORS_AFTER: &[&str] = &[
    "after", "following", "just after", "right after",
    "immediately after", "long after", "ever since",
];

const CONNECTORS_CONCURRENT: &[&str] = &[
    "during", "while", "throughout", "in the middle of",
];

const CONNECTORS_NEUTRAL: &[&str] = &[
    "when", "until", "since", "at the start of", "at the end of",
    "by the time", "as soon as",
];

// Era markers
const ERA_MARKERS: &[&str] = &[
    "third age", "second age", "first age", "fourth age", "fifth age",
    "year", "stardate", "epoch", "era", "age of", "millennium",
    "ad", "bc", "bce", "ce", "a.d.", "b.c.", "b.c.e.", "c.e.",
];

// ==================== MAIN IMPLEMENTATION ====================

/// TemporalCortex - Temporal expression detector
pub struct TemporalCortex {
    automaton: Option<AhoCorasick>,
    pattern_meta: Vec<PatternMeta>,
    pending_patterns: Vec<String>,
    pending_meta: Vec<PatternMeta>,
    custom_era_names: Vec<String>,
    number_re: Regex,
}

impl Default for TemporalCortex {
    fn default() -> Self {
        Self::new()
    }
}

impl TemporalCortex {
    pub fn new() -> Self {
        let mut cortex = Self {
            automaton: None,
            pattern_meta: Vec::new(),
            pending_patterns: Vec::new(),
            pending_meta: Vec::new(),
            custom_era_names: Vec::new(),
            number_re: Regex::new(r"^(?:\s+(?:of|in))?\s*(\d+(?:\.\d+)?)").unwrap(),
        };
        
        cortex.load_default_dictionary();
        cortex.build_internal();
        
        cortex
    }
    
    fn load_default_dictionary(&mut self) {
        // Weekdays
        for (pattern, idx) in WEEKDAYS {
            self.add_pattern_internal(pattern, TemporalKind::Weekday, Some(*idx), None, None);
        }
        
        // Months
        for (pattern, idx) in MONTHS {
            self.add_pattern_internal(pattern, TemporalKind::Month, None, Some(*idx), None);
        }
        
        // Time of day
        for pattern in TIME_OF_DAY {
            self.add_pattern_internal(pattern, TemporalKind::TimeOfDay, None, None, None);
        }
        
        // Narrative markers
        for pattern in NARRATIVE_MARKERS {
            self.add_pattern_internal(pattern, TemporalKind::NarrativeMarker, None, None, None);
        }
        
        // Relative phrases
        for pattern in RELATIVE_PHRASES {
            self.add_pattern_internal(pattern, TemporalKind::Relative, None, None, None);
        }
        
        // Connectors
        for pattern in CONNECTORS_BEFORE {
            self.add_pattern_internal(pattern, TemporalKind::Connector, None, None, Some("before".to_string()));
        }
        for pattern in CONNECTORS_AFTER {
            self.add_pattern_internal(pattern, TemporalKind::Connector, None, None, Some("after".to_string()));
        }
        for pattern in CONNECTORS_CONCURRENT {
            self.add_pattern_internal(pattern, TemporalKind::Connector, None, None, Some("concurrent".to_string()));
        }
        for pattern in CONNECTORS_NEUTRAL {
            self.add_pattern_internal(pattern, TemporalKind::Connector, None, None, None);
        }
        
        // Era markers
        for pattern in ERA_MARKERS {
            self.add_pattern_internal(pattern, TemporalKind::Era, None, None, None);
        }
    }
    
    fn add_pattern_internal(
        &mut self,
        pattern: &str,
        kind: TemporalKind,
        weekday_idx: Option<u8>,
        month_idx: Option<u8>,
        direction: Option<String>,
    ) {
        self.pending_patterns.push(pattern.to_lowercase());
        self.pending_meta.push(PatternMeta {
            kind,
            weekday_idx,
            month_idx,
            direction,
        });
    }
    
    fn build_internal(&mut self) {
        if self.pending_patterns.is_empty() {
            return;
        }
        
        let pma = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .ascii_case_insensitive(true)
            .build(&self.pending_patterns)
            .expect("Failed to build TemporalCortex automaton");
        
        self.automaton = Some(pma);
        self.pattern_meta = self.pending_meta.clone();
    }
    
    /// Hydrate with custom calendar terms
    pub fn hydrate_calendar(
        &mut self,
        months: Vec<String>,
        weekdays: Vec<String>,
        eras: Vec<String>,
    ) -> Result<(), String> {
        // Clear and reset
        self.custom_era_names.clear();
        self.pending_patterns.clear();
        self.pending_meta.clear();
        self.automaton = None;
        
        // Add custom months
        for (idx, month) in months.iter().enumerate() {
            if month.len() >= 2 {
                let lower = month.to_lowercase();
                self.add_pattern_internal(&lower, TemporalKind::Custom, None, Some(idx as u8), None);
            }
        }
        
        // Add custom weekdays
        for (idx, weekday) in weekdays.iter().enumerate() {
            if weekday.len() >= 2 {
                let lower = weekday.to_lowercase();
                self.add_pattern_internal(&lower, TemporalKind::Custom, Some(idx as u8), None, None);
            }
        }
        
        // Add custom eras
        for era in eras {
            if era.len() >= 2 {
                let lower = era.to_lowercase();
                self.custom_era_names.push(lower.clone());
                self.add_pattern_internal(&lower, TemporalKind::Era, None, None, None);
            }
        }
        
        // Reload default dictionary and build
        self.load_default_dictionary();
        self.build_internal();
        
        Ok(())
    }
    
    pub fn is_ready(&self) -> bool {
        self.automaton.is_some()
    }
    
    /// Scan text for temporal mentions
    pub fn scan(&self, text: &str) -> TemporalScanResult {
        let start = std::time::Instant::now();
        
        let pma = match self.automaton.as_ref() {
            Some(p) => p,
            None => return TemporalScanResult {
                mentions: vec![],
                stats: TemporalScanStats { patterns_matched: 0, scan_time_ms: 0.0 },
            },
        };
        
        let mut mentions: Vec<TemporalMention> = Vec::new();
        
        for m in pma.find_iter(text) {
            let start_pos = m.start();
            let end_pos = m.end();
            
            if !is_word_boundary(text, start_pos, end_pos) {
                continue;
            }
            
            let pattern_id = m.pattern().as_usize();
            let meta = &self.pattern_meta[pattern_id];
            let matched_text = &text[start_pos..end_pos];
            
            let metadata = self.extract_metadata(meta, text, end_pos);
            
            mentions.push(TemporalMention {
                kind: meta.kind.as_str().to_string(),
                text: matched_text.to_string(),
                start: start_pos,
                end: end_pos,
                confidence: meta.kind.confidence(),
                metadata: if metadata.weekday_index.is_some() 
                    || metadata.month_index.is_some()
                    || metadata.narrative_number.is_some()
                    || metadata.direction.is_some()
                    || metadata.era_year.is_some()
                    || metadata.era_name.is_some() 
                {
                    Some(metadata)
                } else {
                    None
                },
            });
        }
        
        mentions = self.dedupe_overlapping(mentions);
        
        TemporalScanResult {
            stats: TemporalScanStats {
                patterns_matched: mentions.len(),
                scan_time_ms: start.elapsed().as_secs_f64() * 1000.0,
            },
            mentions,
        }
    }
    
    fn extract_metadata(&self, meta: &PatternMeta, text: &str, end_pos: usize) -> TemporalMetadata {
        let mut metadata = TemporalMetadata::default();
        
        if let Some(idx) = meta.weekday_idx {
            metadata.weekday_index = Some(idx);
        }
        
        if let Some(idx) = meta.month_idx {
            metadata.month_index = Some(idx);
        }
        
        if let Some(ref dir) = meta.direction {
            metadata.direction = Some(dir.clone());
        }
        
        // Look for narrative number
        if meta.kind == TemporalKind::NarrativeMarker {
            let limit = std::cmp::min(end_pos + 15, text.len());
            let after = &text[end_pos..limit];
            if let Some(cap) = self.number_re.captures(after) {
                if let Some(num) = cap.get(1) {
                    if let Ok(n) = num.as_str().parse::<u32>() {
                        metadata.narrative_number = Some(n);
                    }
                }
            }
        }
        
        // Look for era year
        if meta.kind == TemporalKind::Era {
            let limit = std::cmp::min(end_pos + 20, text.len());
            let after = &text[end_pos..limit];
            if let Some(cap) = self.number_re.captures(after) {
                if let Some(num) = cap.get(1) {
                    if let Ok(n) = num.as_str().parse::<f64>() {
                        metadata.era_year = Some(n);
                    }
                }
            }
            // Era name
            let start = end_pos.saturating_sub(30);
            let matched_lower = text[start..end_pos].to_lowercase();
            for era in &self.custom_era_names {
                if matched_lower.ends_with(era) {
                    metadata.era_name = Some(era.clone());
                    break;
                }
            }
            for era in ERA_MARKERS {
                if matched_lower.ends_with(era) {
                    metadata.era_name = Some(era.to_string());
                    break;
                }
            }
        }
        
        metadata
    }
    
    fn dedupe_overlapping(&self, mut mentions: Vec<TemporalMention>) -> Vec<TemporalMention> {
        if mentions.is_empty() {
            return mentions;
        }
        
        mentions.sort_by(|a, b| {
            if a.start != b.start {
                a.start.cmp(&b.start)
            } else {
                (b.end - b.start).cmp(&(a.end - a.start))
            }
        });
        
        let mut result = Vec::with_capacity(mentions.len());
        let mut last_end = 0;
        
        for mention in mentions {
            if mention.start >= last_end {
                last_end = mention.end;
                result.push(mention);
            }
        }
        
        result
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekday_detection() {
        let cortex = TemporalCortex::new();
        assert!(cortex.is_ready());
        
        let result = cortex.scan("on monday we meet");
        assert_eq!(result.mentions.len(), 1);
        assert_eq!(result.mentions[0].text, "monday");
        assert_eq!(result.mentions[0].kind, "WEEKDAY");
    }

    #[test]
    fn test_relative_phrase() {
        let cortex = TemporalCortex::new();
        let result = cortex.scan("later that day the hero arrived");
        
        assert!(result.mentions.iter().any(|m| m.text == "later that day"));
    }

    #[test]
    fn test_narrative_marker_with_number() {
        let cortex = TemporalCortex::new();
        let result = cortex.scan("chapter 5 begins here");
        
        assert!(result.mentions.iter().any(|m| m.text == "chapter"));
        let chapter = result.mentions.iter().find(|m| m.text == "chapter").unwrap();
        assert_eq!(chapter.metadata.as_ref().unwrap().narrative_number, Some(5));
    }

    #[test]
    fn test_era_marker() {
        let cortex = TemporalCortex::new();
        let result = cortex.scan("in the third age 3019");
        
        assert!(result.mentions.iter().any(|m| m.text == "third age"));
        let era = result.mentions.iter().find(|m| m.text == "third age").unwrap();
        assert_eq!(era.metadata.as_ref().unwrap().era_year, Some(3019.0));
    }

    #[test]
    fn test_connector_direction() {
        let cortex = TemporalCortex::new();
        let result = cortex.scan("before the battle, he trained");
        
        let before = result.mentions.iter().find(|m| m.text == "before").unwrap();
        assert_eq!(before.metadata.as_ref().unwrap().direction, Some("before".to_string()));
    }

    #[test]
    fn test_word_boundary() {
        let cortex = TemporalCortex::new();
        // "sunday" inside "sundays" should NOT match
        let result = cortex.scan("studying on sundays");
        // Should match "sun" inside "sundays"? No - word boundary check should prevent
        // Actually "sundays" ending with "s" would pass boundary after, let's check
        // The word boundary fn checks before and after the match
        assert!(result.mentions.is_empty() || 
                !result.mentions.iter().any(|m| m.text == "sun" || m.text == "sunday"));
    }
}
