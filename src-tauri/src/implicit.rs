//! ImplicitCortex - Entity Name Matching in Plain Text (Native Tauri)
//!
//! Uses Aho-Corasick for O(n) detection of entity names and aliases.
//! Designed to find implicit mentions like "Frodo" in prose text.
//!
//! **Smart Alias Generation**: Automatically derives searchable aliases from
//! entity names. "Monkey D. Luffy" → "Luffy" (0.90), "D. Luffy" (0.85).

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use serde::{Deserialize, Serialize};

// =============================================================================
// Types
// =============================================================================

/// Entity definition for hydration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDefinition {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub aliases: Vec<String>,
}

/// A detected implicit entity mention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitMention {
    pub entity_id: String,
    pub entity_label: String,
    pub entity_kind: String,
    pub matched_text: String,
    pub start: usize,
    pub end: usize,
    pub is_alias_match: bool,
    /// Confidence score: 1.0 for exact match, lower for auto-generated aliases
    pub confidence: f64,
}

/// Metadata for each pattern in the automaton
#[derive(Debug, Clone)]
struct PatternMeta {
    entity_id: String,
    entity_label: String,
    entity_kind: String,
    _pattern_text: String,
    is_alias: bool,
    /// Confidence for this pattern (1.0 for primary label, lower for aliases)
    confidence: f64,
}

// =============================================================================
// Smart Alias Generation
// =============================================================================

/// Minimum length for auto-generated aliases to avoid false positives
const MIN_ALIAS_LEN: usize = 3;

/// Generate aliases from an entity name based on its kind
/// Returns (alias, confidence) pairs
fn generate_aliases(label: &str, kind: &str) -> Vec<(String, f64)> {
    let parts: Vec<&str> = label.split_whitespace().collect();
    let mut aliases: Vec<(String, f64)> = Vec::new();
    
    // Skip single-word names - no aliases to generate
    if parts.len() <= 1 {
        return aliases;
    }
    
    // Kind-specific alias generation
    match kind.to_uppercase().as_str() {
        "CHARACTER" | "PERSON" => {
            // Characters get FIRST name (calling name) AND surname treatment
            // "Zorian Kazinski" → "Zorian" (0.95, first name is common calling), "Kazinski" (0.90)
            match parts.len() {
                2 => {
                    // "First Last" → "First" (0.95), "Last" (0.90)
                    let first = parts[0];
                    let last = parts[1];
                    
                    // First name (calling name) - highest priority for characters
                    if first.len() >= MIN_ALIAS_LEN && !is_title_or_honorific(first) {
                        aliases.push((first.to_string(), 0.95));
                    }
                    // Last name (surname)
                    if last.len() >= MIN_ALIAS_LEN && !is_title_or_honorific(last) {
                        aliases.push((last.to_string(), 0.90));
                    }
                }
                3 => {
                    // "First Middle Last" → "First", "Last", "Middle Last"
                    let first = parts[0];
                    let last = parts[2];
                    let middle_last = format!("{} {}", parts[1], parts[2]);
                    
                    // First name (calling name)
                    if first.len() >= MIN_ALIAS_LEN && !is_title_or_honorific(first) {
                        aliases.push((first.to_string(), 0.95));
                    }
                    // Last name
                    if last.len() >= MIN_ALIAS_LEN && !is_title_or_honorific(last) {
                        aliases.push((last.to_string(), 0.90));
                    }
                    // Middle + Last
                    if middle_last.len() >= MIN_ALIAS_LEN {
                        aliases.push((middle_last, 0.85));
                    }
                }
                _ => {
                    // Longer names: first word, last word, last two words
                    let first = parts[0];
                    let last = parts[parts.len() - 1];
                    let last_two = format!("{} {}", parts[parts.len() - 2], last);
                    
                    // First name
                    if first.len() >= MIN_ALIAS_LEN && !is_title_or_honorific(first) {
                        aliases.push((first.to_string(), 0.90));
                    }
                    // Last name
                    if last.len() >= MIN_ALIAS_LEN && !is_title_or_honorific(last) {
                        aliases.push((last.to_string(), 0.85));
                    }
                    if last_two.len() >= MIN_ALIAS_LEN {
                        aliases.push((last_two, 0.80));
                    }
                }
            }
        }
        "FACTION" | "ORGANIZATION" | "GROUP" => {
            // Factions: last word if descriptive, acronyms
            let last = parts[parts.len() - 1];
            if last.len() >= MIN_ALIAS_LEN && !is_common_word(last) {
                aliases.push((last.to_string(), 0.75));
            }
            
            // Generate acronym if 3+ words
            if parts.len() >= 3 {
                let acronym: String = parts.iter()
                    .filter(|p| !is_common_word(p))
                    .filter_map(|p| p.chars().next())
                    .collect::<String>()
                    .to_uppercase();
                if acronym.len() >= 2 {
                    aliases.push((acronym, 0.70));
                }
            }
        }
        _ => {
            // Default: just the last word for multi-word names
            let last = parts[parts.len() - 1];
            if last.len() >= MIN_ALIAS_LEN && !is_common_word(last) {
                aliases.push((last.to_string(), 0.80));
            }
        }
    }
    
    aliases
}

/// Check if a word is a common title/honorific that shouldn't be an alias
fn is_title_or_honorific(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "mr" | "mrs" | "ms" | "dr" | "prof" | "sir" | "lord" | "lady" 
        | "king" | "queen" | "prince" | "princess" | "captain" | "general"
        | "the" | "of" | "and" | "jr" | "sr" | "ii" | "iii" | "iv"
    )
}

/// Check if a word is too common to be a useful alias
fn is_common_word(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "the" | "of" | "and" | "or" | "a" | "an" | "in" | "on" | "at" | "to" | "for"
        | "is" | "are" | "was" | "were" | "be" | "been" | "being"
    )
}

// =============================================================================
// ImplicitCortex
// =============================================================================

/// Entity name matcher using Aho-Corasick
pub struct ImplicitCortex {
    automaton: Option<AhoCorasick>,
    pattern_meta: Vec<PatternMeta>,
    pending_patterns: Vec<String>,
    needs_rebuild: bool,
}

impl Default for ImplicitCortex {
    fn default() -> Self {
        Self::new()
    }
}

impl ImplicitCortex {
    pub fn new() -> Self {
        Self {
            automaton: None,
            pattern_meta: Vec::new(),
            pending_patterns: Vec::new(),
            needs_rebuild: true,
        }
    }

    /// Get the number of patterns
    pub fn pattern_count(&self) -> usize {
        self.pattern_meta.len()
    }

    /// Hydrate with entity definitions (auto-generates aliases)
    pub fn hydrate(&mut self, entities: Vec<EntityDefinition>) {
        self.pattern_meta.clear();
        self.pending_patterns.clear();

        for entity in entities {
            // Add primary label (confidence 1.0)
            let label_lower = entity.label.to_lowercase();
            self.pattern_meta.push(PatternMeta {
                entity_id: entity.id.clone(),
                entity_label: entity.label.clone(),
                entity_kind: entity.kind.clone(),
                _pattern_text: label_lower.clone(),
                is_alias: false,
                confidence: 1.0,
            });
            self.pending_patterns.push(label_lower);

            // Add explicitly provided aliases (confidence 0.95)
            for alias in &entity.aliases {
                let alias_lower = alias.to_lowercase();
                self.pattern_meta.push(PatternMeta {
                    entity_id: entity.id.clone(),
                    entity_label: entity.label.clone(),
                    entity_kind: entity.kind.clone(),
                    _pattern_text: alias_lower.clone(),
                    is_alias: true,
                    confidence: 0.95,
                });
                self.pending_patterns.push(alias_lower);
            }
            
            // Auto-generate additional aliases based on name structure
            let auto_aliases = generate_aliases(&entity.label, &entity.kind);
            for (alias, confidence) in auto_aliases {
                let alias_lower = alias.to_lowercase();
                // Avoid duplicates
                if !self.pending_patterns.contains(&alias_lower) {
                    self.pattern_meta.push(PatternMeta {
                        entity_id: entity.id.clone(),
                        entity_label: entity.label.clone(),
                        entity_kind: entity.kind.clone(),
                        _pattern_text: alias_lower.clone(),
                        is_alias: true,
                        confidence,
                    });
                    self.pending_patterns.push(alias_lower);
                }
            }
        }

        self.needs_rebuild = true;
    }

    /// Build the automaton
    pub fn build(&mut self) -> Result<(), String> {
        if self.pending_patterns.is_empty() {
            self.automaton = None;
            return Ok(());
        }

        let automaton = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .ascii_case_insensitive(true)
            .build(&self.pending_patterns)
            .map_err(|e| format!("Failed to build automaton: {}", e))?;

        self.automaton = Some(automaton);
        self.needs_rebuild = false;
        Ok(())
    }

    /// Find all implicit entity mentions in text
    pub fn find_mentions(&self, text: &str) -> Vec<ImplicitMention> {
        let automaton = match &self.automaton {
            Some(a) => a,
            None => return vec![],
        };

        if text.is_empty() {
            return vec![];
        }

        let mut mentions: Vec<ImplicitMention> = Vec::new();

        for mat in automaton.find_iter(text) {
            let pattern_id = mat.pattern().as_usize();
            if let Some(meta) = self.pattern_meta.get(pattern_id) {
                mentions.push(ImplicitMention {
                    entity_id: meta.entity_id.clone(),
                    entity_label: meta.entity_label.clone(),
                    entity_kind: meta.entity_kind.clone(),
                    matched_text: text[mat.start()..mat.end()].to_string(),
                    start: mat.start(),
                    end: mat.end(),
                    is_alias_match: meta.is_alias,
                    confidence: meta.confidence,
                });
            }
        }

        // Dedupe overlapping matches (LeftmostLongest already handles most cases)
        self.dedupe_overlapping(mentions)
    }

    /// Remove overlapping matches, keeping longer ones
    fn dedupe_overlapping(&self, mut mentions: Vec<ImplicitMention>) -> Vec<ImplicitMention> {
        if mentions.len() <= 1 {
            return mentions;
        }

        // Sort by start position, then by length (longer first)
        mentions.sort_by(|a, b| {
            a.start.cmp(&b.start)
                .then_with(|| (b.end - b.start).cmp(&(a.end - a.start)))
        });

        let mut result: Vec<ImplicitMention> = Vec::new();
        let mut last_end = 0;

        for mention in mentions {
            // Only add if it doesn't overlap with previous
            if mention.start >= last_end {
                last_end = mention.end;
                result.push(mention);
            }
        }

        result
    }

    /// Add a single entity (for incremental updates)
    pub fn add_entity(&mut self, entity: EntityDefinition) {
        let label_lower = entity.label.to_lowercase();
        self.pattern_meta.push(PatternMeta {
            entity_id: entity.id.clone(),
            entity_label: entity.label.clone(),
            entity_kind: entity.kind.clone(),
            _pattern_text: label_lower.clone(),
            is_alias: false,
            confidence: 1.0,
        });
        self.pending_patterns.push(label_lower);

        for alias in &entity.aliases {
            let alias_lower = alias.to_lowercase();
            self.pattern_meta.push(PatternMeta {
                entity_id: entity.id.clone(),
                entity_label: entity.label.clone(),
                entity_kind: entity.kind.clone(),
                _pattern_text: alias_lower.clone(),
                is_alias: true,
                confidence: 0.95,
            });
            self.pending_patterns.push(alias_lower);
        }

        // Auto-generate additional aliases based on name structure
        let auto_aliases = generate_aliases(&entity.label, &entity.kind);
        for (alias, confidence) in auto_aliases {
            let alias_lower = alias.to_lowercase();
            // Avoid duplicates
            if !self.pending_patterns.contains(&alias_lower) {
                self.pattern_meta.push(PatternMeta {
                    entity_id: entity.id.clone(),
                    entity_label: entity.label.clone(),
                    entity_kind: entity.kind.clone(),
                    _pattern_text: alias_lower.clone(),
                    is_alias: true,
                    confidence,
                });
                self.pending_patterns.push(alias_lower);
            }
        }

        self.needs_rebuild = true;
    }

    /// Clear all patterns
    pub fn clear(&mut self) {
        self.pattern_meta.clear();
        self.pending_patterns.clear();
        self.automaton = None;
        self.needs_rebuild = true;
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: &str, label: &str, kind: &str, aliases: Vec<&str>) -> EntityDefinition {
        EntityDefinition {
            id: id.to_string(),
            label: label.to_string(),
            kind: kind.to_string(),
            aliases: aliases.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_hydrate_and_find_single_entity() {
        let mut cortex = ImplicitCortex::new();
        cortex.hydrate(vec![entity("char_001", "Frodo", "CHARACTER", vec![])]);
        cortex.build().unwrap();

        let mentions = cortex.find_mentions("Frodo went to the market");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].entity_id, "char_001");
        assert_eq!(mentions[0].entity_label, "Frodo");
        assert_eq!(mentions[0].start, 0);
        assert_eq!(mentions[0].end, 5);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let mut cortex = ImplicitCortex::new();
        cortex.hydrate(vec![entity("char_001", "Gandalf", "CHARACTER", vec![])]);
        cortex.build().unwrap();

        let mentions = cortex.find_mentions("GANDALF spoke to gandalf");
        assert_eq!(mentions.len(), 2);
    }

    #[test]
    fn test_alias_matching() {
        let mut cortex = ImplicitCortex::new();
        cortex.hydrate(vec![entity(
            "char_001",
            "Aragorn",
            "CHARACTER",
            vec!["Strider", "Elessar"],
        )]);
        cortex.build().unwrap();

        let mentions = cortex.find_mentions("Strider is also known as Aragorn");
        assert_eq!(mentions.len(), 2);

        let alias_match = mentions.iter().find(|m| m.matched_text.to_lowercase() == "strider");
        assert!(alias_match.is_some());
        assert!(alias_match.unwrap().is_alias_match);
    }

    #[test]
    fn test_overlapping_keeps_longer() {
        let mut cortex = ImplicitCortex::new();
        cortex.hydrate(vec![
            entity("loc_001", "New York", "LOCATION", vec![]),
            entity("loc_002", "York", "LOCATION", vec![]),
        ]);
        cortex.build().unwrap();

        let mentions = cortex.find_mentions("I visited New York yesterday");
        // Should only get "New York", not "York" inside it
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].entity_id, "loc_001");
    }

    #[test]
    fn test_smart_alias_luffy() {
        let mut cortex = ImplicitCortex::new();
        cortex.hydrate(vec![entity("char_001", "Monkey D. Luffy", "CHARACTER", vec![])]);
        cortex.build().unwrap();

        // Should find "Luffy" as an auto-generated alias
        let mentions = cortex.find_mentions("Luffy defeated Kaido");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].entity_label, "Monkey D. Luffy");
        assert!(mentions[0].is_alias_match);
        assert!(mentions[0].confidence < 1.0);
    }

    #[test]
    fn test_faction_acronym() {
        let mut cortex = ImplicitCortex::new();
        cortex.hydrate(vec![entity("fac_001", "Straw Hat Pirates", "FACTION", vec![])]);
        cortex.build().unwrap();

        // Should find "SHP" as an auto-generated acronym
        let mentions = cortex.find_mentions("SHP is the best crew");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].entity_label, "Straw Hat Pirates");
    }

    #[test]
    fn test_confidence_scores() {
        let mut cortex = ImplicitCortex::new();
        cortex.hydrate(vec![entity("char_001", "Monkey D. Luffy", "CHARACTER", vec!["Straw Hat"])]);
        cortex.build().unwrap();

        // Full name = 1.0
        let full = cortex.find_mentions("Monkey D. Luffy");
        assert_eq!(full[0].confidence, 1.0);

        // Explicit alias = 0.95
        let explicit = cortex.find_mentions("Straw Hat");
        assert_eq!(explicit[0].confidence, 0.95);

        // Auto-generated surname = 0.90
        let auto = cortex.find_mentions("Luffy");
        assert!((auto[0].confidence - 0.90).abs() < 0.01);
    }
}
