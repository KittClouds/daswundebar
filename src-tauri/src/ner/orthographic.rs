//! Orthographic Classifier - Shape-based token classification
//!
//! Classifies tokens by capitalization patterns for NER heuristics.
//! InitCap mid-sentence is a strong entity signal.

use serde::{Deserialize, Serialize};

// =============================================================================
// Types
// =============================================================================

/// Orthographic shape classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrthographicShape {
    /// All uppercase: "NASA", "MORDOR", "WHO"
    AllCaps,
    /// Initial capital: "Zorian", "Cyoria", "Sally"
    InitCap,
    /// Mixed case (internal caps): "McDonald", "iPhone", "MacBook"
    MixedCaps,
    /// All lowercase: "the", "wizard", "ancient"
    AllLower,
    /// Contains digits: "Chapter3", "M16", "2nd"
    HasDigit,
    /// Single character
    SingleChar,
    /// Punctuation only
    Punctuation,
    /// Unknown/other
    Other,
}

impl OrthographicShape {
    /// Get a short code for the shape
    pub fn code(&self) -> &'static str {
        match self {
            OrthographicShape::AllCaps => "AA",
            OrthographicShape::InitCap => "Aa",
            OrthographicShape::MixedCaps => "aA",
            OrthographicShape::AllLower => "aa",
            OrthographicShape::HasDigit => "d",
            OrthographicShape::SingleChar => "x",
            OrthographicShape::Punctuation => ".",
            OrthographicShape::Other => "?",
        }
    }

    /// Is this shape likely to be an entity?
    pub fn is_entity_candidate(&self) -> bool {
        matches!(
            self,
            OrthographicShape::AllCaps
                | OrthographicShape::InitCap
                | OrthographicShape::MixedCaps
        )
    }
}

// =============================================================================
// Classifier
// =============================================================================

/// Orthographic shape classifier
pub struct OrthographicClassifier {
    /// Minimum confidence for entity candidacy
    _min_confidence: f32,
}

impl Default for OrthographicClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl OrthographicClassifier {
    pub fn new() -> Self {
        OrthographicClassifier {
            _min_confidence: 0.5,
        }
    }

    /// Classify a single token's orthographic shape
    pub fn classify(&self, text: &str) -> OrthographicShape {
        if text.is_empty() {
            return OrthographicShape::Other;
        }

        let chars: Vec<char> = text.chars().collect();

        // Single character
        if chars.len() == 1 {
            let c = chars[0];
            if c.is_ascii_punctuation() || is_unicode_punctuation(c) {
                return OrthographicShape::Punctuation;
            }
            return OrthographicShape::SingleChar;
        }

        // Check for digits
        if chars.iter().any(|c| c.is_ascii_digit()) {
            return OrthographicShape::HasDigit;
        }

        // Get only alphabetic characters for case analysis
        let alpha_chars: Vec<char> = chars.iter().filter(|c| c.is_alphabetic()).copied().collect();

        if alpha_chars.is_empty() {
            // No alphabetic chars (punctuation-only or special)
            if chars.iter().all(|c| c.is_ascii_punctuation() || is_unicode_punctuation(*c)) {
                return OrthographicShape::Punctuation;
            }
            return OrthographicShape::Other;
        }

        let all_upper = alpha_chars.iter().all(|c| c.is_uppercase());
        let all_lower = alpha_chars.iter().all(|c| c.is_lowercase());
        let first_upper = alpha_chars.first().map(|c| c.is_uppercase()).unwrap_or(false);

        if all_upper {
            return OrthographicShape::AllCaps;
        }

        if all_lower {
            return OrthographicShape::AllLower;
        }

        if first_upper {
            // Check for mixed case (internal capitals)
            let rest = &alpha_chars[1..];
            if rest.iter().any(|c| c.is_uppercase()) {
                return OrthographicShape::MixedCaps;
            }
            return OrthographicShape::InitCap;
        }

        // lowercase start but has uppercase later (e.g., "iPhone")
        if alpha_chars.iter().skip(1).any(|c| c.is_uppercase()) {
            return OrthographicShape::MixedCaps;
        }

        OrthographicShape::Other
    }

    /// Classify a sequence of tokens and return shapes
    pub fn classify_sequence(&self, tokens: &[&str]) -> Vec<OrthographicShape> {
        tokens.iter().map(|t| self.classify(t)).collect()
    }

    /// Check if a token at position is likely an entity based on context
    pub fn is_likely_entity(&self, text: &str, is_sentence_start: bool) -> bool {
        let shape = self.classify(text);

        match shape {
            // AllCaps is always suspicious (could be acronym or emphasis)
            OrthographicShape::AllCaps => text.len() >= 2,

            // InitCap at sentence start is ambiguous
            OrthographicShape::InitCap => !is_sentence_start,

            // MixedCaps is almost always an entity (McDonald, iPhone)
            OrthographicShape::MixedCaps => true,

            // Other shapes are not entity candidates
            _ => false,
        }
    }

    /// Generate a shape signature for a sequence of words
    /// Useful for pattern matching (e.g., "Aa Aa" = two InitCap words)
    pub fn sequence_signature(&self, tokens: &[&str]) -> String {
        tokens
            .iter()
            .map(|t| self.classify(t).code())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Check if character is Unicode punctuation
fn is_unicode_punctuation(c: char) -> bool {
    matches!(
        c,
        '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}' |
        '\u{2013}' | '\u{2014}' | '\u{2026}' |
        '\u{00AB}' | '\u{00BB}' | '\u{00A1}' | '\u{00BF}' |
        '\u{2039}' | '\u{203A}' | '\u{201E}' | '\u{201A}'
    )
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_caps() {
        let c = OrthographicClassifier::new();
        assert_eq!(c.classify("NASA"), OrthographicShape::AllCaps);
        assert_eq!(c.classify("MORDOR"), OrthographicShape::AllCaps);
        assert_eq!(c.classify("WHO"), OrthographicShape::AllCaps);
    }

    #[test]
    fn test_init_cap() {
        let c = OrthographicClassifier::new();
        assert_eq!(c.classify("Zorian"), OrthographicShape::InitCap);
        assert_eq!(c.classify("Sally"), OrthographicShape::InitCap);
        assert_eq!(c.classify("Cyoria"), OrthographicShape::InitCap);
    }

    #[test]
    fn test_mixed_caps() {
        let c = OrthographicClassifier::new();
        assert_eq!(c.classify("McDonald"), OrthographicShape::MixedCaps);
        assert_eq!(c.classify("iPhone"), OrthographicShape::MixedCaps);
        assert_eq!(c.classify("MacBook"), OrthographicShape::MixedCaps);
    }

    #[test]
    fn test_all_lower() {
        let c = OrthographicClassifier::new();
        assert_eq!(c.classify("wizard"), OrthographicShape::AllLower);
        assert_eq!(c.classify("the"), OrthographicShape::AllLower);
        assert_eq!(c.classify("ancient"), OrthographicShape::AllLower);
    }

    #[test]
    fn test_has_digit() {
        let c = OrthographicClassifier::new();
        assert_eq!(c.classify("Chapter3"), OrthographicShape::HasDigit);
        assert_eq!(c.classify("M16"), OrthographicShape::HasDigit);
        assert_eq!(c.classify("2nd"), OrthographicShape::HasDigit);
    }

    #[test]
    fn test_punctuation() {
        let c = OrthographicClassifier::new();
        assert_eq!(c.classify("."), OrthographicShape::Punctuation);
        assert_eq!(c.classify("!"), OrthographicShape::Punctuation);
        assert_eq!(c.classify("—"), OrthographicShape::Punctuation);
    }

    #[test]
    fn test_single_char() {
        let c = OrthographicClassifier::new();
        assert_eq!(c.classify("I"), OrthographicShape::SingleChar);
        assert_eq!(c.classify("a"), OrthographicShape::SingleChar);
    }

    #[test]
    fn test_entity_candidate() {
        let c = OrthographicClassifier::new();

        // Entity candidates
        assert!(c.classify("NASA").is_entity_candidate());
        assert!(c.classify("Sally").is_entity_candidate());
        assert!(c.classify("McDonald").is_entity_candidate());

        // Non-candidates
        assert!(!c.classify("wizard").is_entity_candidate());
        assert!(!c.classify("the").is_entity_candidate());
        assert!(!c.classify("42").is_entity_candidate());
    }

    #[test]
    fn test_is_likely_entity() {
        let c = OrthographicClassifier::new();

        // InitCap at sentence start - NOT likely (could be regular word)
        assert!(!c.is_likely_entity("The", true));

        // InitCap mid-sentence - likely entity
        assert!(c.is_likely_entity("Zorian", false));

        // AllCaps is always suspicious
        assert!(c.is_likely_entity("NASA", true));
        assert!(c.is_likely_entity("WHO", false));

        // MixedCaps is almost always entity
        assert!(c.is_likely_entity("McDonald", true));
        assert!(c.is_likely_entity("iPhone", false));
    }

    #[test]
    fn test_sequence_signature() {
        let c = OrthographicClassifier::new();
        let tokens = vec!["The", "ancient", "wizard", "Gandalf"];
        let sig = c.sequence_signature(&tokens);
        assert_eq!(sig, "Aa aa aa Aa");
    }

    #[test]
    fn test_hyphenated_name() {
        let c = OrthographicClassifier::new();
        // Hyphenated names like "Sally-Anne" have internal capital 'A', so MixedCaps
        assert_eq!(c.classify("Sally-Anne"), OrthographicShape::MixedCaps);
        // Single-part hyphenated is InitCap
        assert_eq!(c.classify("O'Brien"), OrthographicShape::InitCap);
    }

    #[test]
    fn test_contraction() {
        let c = OrthographicClassifier::new();
        // Contractions with lowercase
        assert_eq!(c.classify("don't"), OrthographicShape::AllLower);
        // Contractions with InitCap
        assert_eq!(c.classify("I'll"), OrthographicShape::InitCap);
    }
}
