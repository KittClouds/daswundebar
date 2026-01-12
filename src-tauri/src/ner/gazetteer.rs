//! Gazetteer - Entity name matching via Aho-Corasick
//!
//! Wraps ImplicitCortex to provide gazetteer functionality for the FST-NER pipeline.
//! Matches known entity names and aliases in O(n) time.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Types
// =============================================================================

/// Priority levels for gazetteer matches
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MatchPriority {
    /// Low priority (auto-generated aliases)
    Low = 1,
    /// Medium priority (user-defined aliases)
    Medium = 2,
    /// High priority (canonical entity names)
    High = 3,
    /// Exact match from registry
    Exact = 4,
}

/// A gazetteer match result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GazetteerMatch {
    /// Matched text from source
    pub text: String,
    /// Entity kind (CHARACTER, LOCATION, etc.)
    pub kind: String,
    /// Canonical entity label (resolved from alias)
    pub canonical_label: String,
    /// Entity ID (if known)
    pub entity_id: Option<String>,
    /// Match priority
    pub priority: MatchPriority,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Start byte offset
    pub byte_start: usize,
    /// End byte offset
    pub byte_end: usize,
}

/// Entry in the gazetteer
#[derive(Debug, Clone)]
struct GazetteerEntry {
    pattern: String,
    kind: String,
    canonical_label: String,
    entity_id: Option<String>,
    priority: MatchPriority,
    confidence: f64,
}

// =============================================================================
// Gazetteer
// =============================================================================

/// Gazetteer matcher using Aho-Corasick
pub struct Gazetteer {
    /// Pattern entries (indexed by pattern ID from AC)
    entries: Vec<GazetteerEntry>,
    /// Aho-Corasick automaton
    automaton: Option<aho_corasick::AhoCorasick>,
    /// Patterns pending build
    pending_patterns: Vec<String>,
    /// Whether automaton needs rebuild
    dirty: bool,
}

impl Default for Gazetteer {
    fn default() -> Self {
        Self::new()
    }
}

impl Gazetteer {
    pub fn new() -> Self {
        Gazetteer {
            entries: Vec::new(),
            automaton: None,
            pending_patterns: Vec::new(),
            dirty: false,
        }
    }

    /// Add an entry to the gazetteer
    pub fn add(
        &mut self,
        pattern: &str,
        kind: &str,
        canonical_label: &str,
        entity_id: Option<&str>,
        priority: MatchPriority,
        confidence: f64,
    ) {
        // Store lowercase for case-insensitive matching
        let lower_pattern = pattern.to_lowercase();
        
        self.entries.push(GazetteerEntry {
            pattern: lower_pattern.clone(),
            kind: kind.to_string(),
            canonical_label: canonical_label.to_string(),
            entity_id: entity_id.map(|s| s.to_string()),
            priority,
            confidence,
        });
        
        self.pending_patterns.push(lower_pattern);
        self.dirty = true;
    }

    /// Add an entity with auto-generated aliases
    pub fn add_entity(&mut self, id: &str, label: &str, kind: &str, aliases: &[String]) {
        // Add canonical name with high priority
        self.add(label, kind, label, Some(id), MatchPriority::High, 1.0);
        
        // Add aliases with medium priority
        for alias in aliases {
            self.add(alias, kind, label, Some(id), MatchPriority::Medium, 0.9);
        }
    }

    /// Build the automaton (must call before matching)
    pub fn build(&mut self) -> Result<(), String> {
        if !self.dirty && self.automaton.is_some() {
            return Ok(());
        }

        if self.pending_patterns.is_empty() {
            self.automaton = None;
            self.dirty = false;
            return Ok(());
        }

        use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .ascii_case_insensitive(true)
            .build(&self.pending_patterns)
            .map_err(|e| format!("Failed to build gazetteer: {}", e))?;

        self.automaton = Some(ac);
        self.dirty = false;
        Ok(())
    }

    /// Find all gazetteer matches in text
    pub fn find_matches(&self, text: &str) -> Vec<GazetteerMatch> {
        let ac = match &self.automaton {
            Some(ac) => ac,
            None => return Vec::new(),
        };

        let lower_text = text.to_lowercase();
        let mut matches = Vec::new();

        for mat in ac.find_iter(&lower_text) {
            let pattern_id = mat.pattern().as_usize();
            if let Some(entry) = self.entries.get(pattern_id) {
                // Validate word boundaries
                if !is_word_boundary(&lower_text, mat.start(), mat.end()) {
                    continue;
                }

                // Get original text (preserving case)
                let matched_text = &text[mat.start()..mat.end()];

                matches.push(GazetteerMatch {
                    text: matched_text.to_string(),
                    kind: entry.kind.clone(),
                    canonical_label: entry.canonical_label.clone(),
                    entity_id: entry.entity_id.clone(),
                    priority: entry.priority,
                    confidence: entry.confidence,
                    byte_start: mat.start(),
                    byte_end: mat.end(),
                });
            }
        }

        // Dedupe overlapping matches (keep higher priority/longer)
        dedupe_overlapping(matches)
    }

    /// Get pattern count
    pub fn pattern_count(&self) -> usize {
        self.entries.len()
    }

    /// Clear all patterns
    pub fn clear(&mut self) {
        self.entries.clear();
        self.pending_patterns.clear();
        self.automaton = None;
        self.dirty = false;
    }

    /// Remove all entries for a specific entity ID
    /// 
    /// Use this when an entity is deleted from the registry.
    /// Triggers rebuild of the Aho-Corasick automaton on next build() call.
    pub fn remove_entity(&mut self, id: &str) {
        let before_count = self.entries.len();
        
        // Remove all entries matching this entity ID
        self.entries.retain(|e| e.entity_id.as_deref() != Some(id));
        
        let removed_count = before_count - self.entries.len();
        
        if removed_count > 0 {
            // Rebuild pending_patterns from remaining entries
            self.pending_patterns = self.entries.iter()
                .map(|e| e.pattern.clone())
                .collect();
            self.dirty = true;
            
            log::debug!("[Gazetteer] Removed {} patterns for entity '{}'", removed_count, id);
        }
    }

    /// Check if gazetteer is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Check if match is at word boundaries
fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    // Check start boundary
    if start > 0 {
        let before = text[..start].chars().last();
        if let Some(c) = before {
            if c.is_alphanumeric() {
                return false;
            }
        }
    }

    // Check end boundary
    if end < text.len() {
        let after = text[end..].chars().next();
        if let Some(c) = after {
            if c.is_alphanumeric() {
                return false;
            }
        }
    }

    true
}

/// Remove overlapping matches, keeping higher priority/longer ones
fn dedupe_overlapping(mut matches: Vec<GazetteerMatch>) -> Vec<GazetteerMatch> {
    if matches.len() <= 1 {
        return matches;
    }

    // Sort by: start ASC, then length DESC, then priority DESC
    matches.sort_by(|a, b| {
        a.byte_start
            .cmp(&b.byte_start)
            .then_with(|| (b.byte_end - b.byte_start).cmp(&(a.byte_end - a.byte_start)))
            .then_with(|| b.priority.cmp(&a.priority))
    });

    let mut result = Vec::new();
    let mut last_end = 0;

    for m in matches {
        if m.byte_start >= last_end {
            last_end = m.byte_end;
            result.push(m);
        }
    }

    result
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_match() {
        let mut gaz = Gazetteer::new();
        gaz.add("Zorian", "CHARACTER", "Zorian", Some("ent_1"), MatchPriority::High, 1.0);
        gaz.build().unwrap();

        let matches = gaz.find_matches("Zorian walked into the room.");
        
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "Zorian");
        assert_eq!(matches[0].kind, "CHARACTER");
        assert_eq!(matches[0].byte_start, 0);
    }

    #[test]
    fn test_case_insensitive() {
        let mut gaz = Gazetteer::new();
        gaz.add("Zorian", "CHARACTER", "Zorian", None, MatchPriority::High, 1.0);
        gaz.build().unwrap();

        let matches = gaz.find_matches("ZORIAN walked, then zorian spoke.");
        
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].text, "ZORIAN");
        assert_eq!(matches[1].text, "zorian");
    }

    #[test]
    fn test_word_boundary() {
        let mut gaz = Gazetteer::new();
        gaz.add("or", "MISC", "Or", None, MatchPriority::High, 1.0);
        gaz.build().unwrap();

        // Should NOT match "or" inside "Zorian"
        let matches = gaz.find_matches("Zorian or the door");
        
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "or");
        assert_eq!(matches[0].byte_start, 7); // " or "
    }

    #[test]
    fn test_multiple_entities() {
        let mut gaz = Gazetteer::new();
        gaz.add("Zorian", "CHARACTER", "Zorian", None, MatchPriority::High, 1.0);
        gaz.add("Cyoria", "LOCATION", "Cyoria", None, MatchPriority::High, 1.0);
        gaz.build().unwrap();

        let matches = gaz.find_matches("Zorian lived in Cyoria.");
        
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].kind, "CHARACTER");
        assert_eq!(matches[1].kind, "LOCATION");
    }

    #[test]
    fn test_overlapping_longest_wins() {
        let mut gaz = Gazetteer::new();
        gaz.add("Sally", "CHARACTER", "Sally", None, MatchPriority::High, 1.0);
        gaz.add("Sally Sparrow", "CHARACTER", "Sally Sparrow", None, MatchPriority::High, 1.0);
        gaz.build().unwrap();

        let matches = gaz.find_matches("Sally Sparrow walked.");
        
        // Should keep "Sally Sparrow" (longer), not "Sally"
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "Sally Sparrow");
    }

    #[test]
    fn test_alias_resolution() {
        let mut gaz = Gazetteer::new();
        gaz.add_entity(
            "ent_doctor",
            "The Doctor",
            "CHARACTER",
            &["Doctor Who".to_string(), "Doc".to_string()],
        );
        gaz.build().unwrap();

        let matches = gaz.find_matches("The Doctor met Doc.");
        
        assert_eq!(matches.len(), 2);
        // Both should resolve to canonical "The Doctor"
        assert_eq!(matches[0].canonical_label, "The Doctor");
        assert_eq!(matches[1].canonical_label, "The Doctor");
    }

    #[test]
    fn test_priority_order() {
        let mut gaz = Gazetteer::new();
        // Add same pattern with different priorities
        gaz.add("wizard", "CLASS", "Wizard", None, MatchPriority::Low, 0.5);
        gaz.add("wizard", "CHARACTER", "The Wizard", None, MatchPriority::High, 1.0);
        gaz.build().unwrap();

        let matches = gaz.find_matches("The wizard appeared.");
        
        // Higher priority should win
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].kind, "CHARACTER");
    }

    #[test]
    fn test_empty_gazetteer() {
        let mut gaz = Gazetteer::new();
        gaz.build().unwrap();

        let matches = gaz.find_matches("Hello world");
        assert!(matches.is_empty());
    }
}
