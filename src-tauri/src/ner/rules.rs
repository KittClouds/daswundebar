//! Rule Engine - Contextual pattern rules for NER
//!
//! Implements Brill-style contextual rules for entity recognition.
//! Rules are applied in priority order to annotate entity spans.
//!
//! # Example Rules
//!
//! ```text
//! Rule: PersonAfterTitle
//! Pattern: [title] [InitCap]+
//! Action: annotate(CHARACTER)
//!
//! Rule: LocationAfterPreposition  
//! Pattern: [in|at|from|to] [InitCap]+
//! Action: annotate(LOCATION)
//! ```

use serde::{Deserialize, Serialize};
use super::orthographic::{OrthographicClassifier, OrthographicShape};
use super::tokenizer::{Token, TokenKind};

// =============================================================================
// Types
// =============================================================================

/// Condition for matching a token
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenCondition {
    /// Match specific text (case-insensitive)
    Text(String),
    /// Match any text in list
    TextIn(Vec<String>),
    /// Match orthographic shape
    Shape(OrthographicShape),
    /// Match orthographic shape BUT exclude specific texts (case-insensitive)
    ShapeNotIn(OrthographicShape, Vec<String>),
    /// Match token kind
    Kind(TokenKind),
    /// Match any token
    Any,
}

impl TokenCondition {
    /// Check if condition matches a token
    pub fn matches(&self, token: &Token, classifier: &OrthographicClassifier) -> bool {
        match self {
            TokenCondition::Text(t) => token.text.eq_ignore_ascii_case(t),
            TokenCondition::TextIn(texts) => {
                texts.iter().any(|t| token.text.eq_ignore_ascii_case(t))
            }
            TokenCondition::Shape(shape) => classifier.classify(&token.text) == *shape,
            TokenCondition::ShapeNotIn(shape, exclude) => {
                classifier.classify(&token.text) == *shape
                    && !exclude.iter().any(|t| token.text.eq_ignore_ascii_case(t))
            }
            TokenCondition::Kind(kind) => token.kind == *kind,
            TokenCondition::Any => true,
        }
    }
}

/// A pattern element (with quantifier)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternElement {
    pub condition: TokenCondition,
    /// Minimum occurrences
    pub min: usize,
    /// Maximum occurrences (None = unlimited)
    pub max: Option<usize>,
    /// Should this element be included in the annotation span?
    pub capture: bool,
}

impl PatternElement {
    /// Single required match
    pub fn one(condition: TokenCondition) -> Self {
        PatternElement {
            condition,
            min: 1,
            max: Some(1),
            capture: true,
        }
    }

    /// One or more matches
    pub fn one_or_more(condition: TokenCondition) -> Self {
        PatternElement {
            condition,
            min: 1,
            max: None,
            capture: true,
        }
    }

    /// Zero or one match (optional)
    pub fn optional(condition: TokenCondition) -> Self {
        PatternElement {
            condition,
            min: 0,
            max: Some(1),
            capture: true,
        }
    }

    /// Context-only (not included in span)
    pub fn context(condition: TokenCondition) -> Self {
        PatternElement {
            condition,
            min: 1,
            max: Some(1),
            capture: false,
        }
    }
}

/// A contextual rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Rule name (for debugging)
    pub name: String,
    /// Priority (higher = applied first)
    pub priority: u32,
    /// Pattern elements to match
    pub pattern: Vec<PatternElement>,
    /// Entity kind to annotate
    pub entity_kind: String,
    /// Confidence adjustment
    pub confidence: f64,
}

/// A matched rule result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleMatch {
    /// Rule that matched
    pub rule_name: String,
    /// Matched text
    pub text: String,
    /// Entity kind
    pub kind: String,
    /// Confidence
    pub confidence: f64,
    /// Start byte offset
    pub byte_start: usize,
    /// End byte offset
    pub byte_end: usize,
}

// =============================================================================
// Rule Engine
// =============================================================================

/// Contextual rule engine
pub struct RuleEngine {
    /// Rules sorted by priority (descending)
    rules: Vec<Rule>,
    /// Orthographic classifier
    classifier: OrthographicClassifier,
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleEngine {
    pub fn new() -> Self {
        let mut engine = RuleEngine {
            rules: Vec::new(),
            classifier: OrthographicClassifier::new(),
        };
        engine.load_default_rules();
        engine
    }

    /// Add a rule
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
        // Keep sorted by priority (descending)
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Apply rules to tokens, return matches
    pub fn apply(&self, tokens: &[Token]) -> Vec<RuleMatch> {
        let mut matches = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            // Skip non-word tokens
            if tokens[i].kind == TokenKind::Punctuation || tokens[i].kind == TokenKind::Whitespace {
                i += 1;
                continue;
            }

            // Try each rule in priority order
            let mut matched = false;
            for rule in &self.rules {
                if let Some((span_start, span_end, consumed)) = self.try_match_rule(rule, tokens, i) {
                    let text: String = tokens[span_start..span_end]
                        .iter()
                        .map(|t| t.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");

                    matches.push(RuleMatch {
                        rule_name: rule.name.clone(),
                        text,
                        kind: rule.entity_kind.clone(),
                        confidence: rule.confidence,
                        byte_start: tokens[span_start].byte_start,
                        byte_end: tokens[span_end - 1].byte_end,
                    });

                    i += consumed;
                    matched = true;
                    break;
                }
            }

            if !matched {
                i += 1;
            }
        }

        matches
    }

    /// Try to match a rule at position, returns (span_start, span_end, tokens_consumed)
    fn try_match_rule(
        &self,
        rule: &Rule,
        tokens: &[Token],
        start: usize,
    ) -> Option<(usize, usize, usize)> {
        let mut pos = start;
        let mut span_start: Option<usize> = None;
        let mut span_end: Option<usize> = None;

        for element in &rule.pattern {
            let mut count = 0;

            while pos < tokens.len() {
                // Skip whitespace between tokens
                if tokens[pos].kind == TokenKind::Whitespace {
                    pos += 1;
                    continue;
                }

                if element.condition.matches(&tokens[pos], &self.classifier) {
                    count += 1;

                    if element.capture {
                        if span_start.is_none() {
                            span_start = Some(pos);
                        }
                        span_end = Some(pos + 1);
                    }

                    pos += 1;

                    // Check max
                    if let Some(max) = element.max {
                        if count >= max {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }

            // Check min requirement
            if count < element.min {
                return None;
            }
        }

        match (span_start, span_end) {
            (Some(s), Some(e)) if e > s => Some((s, e, pos - start)),
            _ => None,
        }
    }

    /// Load default narrative rules
    fn load_default_rules(&mut self) {
        // Rule: Title + InitCap+ → CHARACTER
        self.add_rule(Rule {
            name: "PersonAfterTitle".to_string(),
            priority: 100,
            pattern: vec![
                PatternElement::context(TokenCondition::TextIn(vec![
                    "lord".to_string(),
                    "lady".to_string(),
                    "sir".to_string(),
                    "dr".to_string(),
                    "doctor".to_string(),
                    "professor".to_string(),
                    "king".to_string(),
                    "queen".to_string(),
                    "prince".to_string(),
                    "princess".to_string(),
                    "captain".to_string(),
                    "master".to_string(),
                ])),
                PatternElement::one_or_more(TokenCondition::Shape(OrthographicShape::InitCap)),
            ],
            entity_kind: "CHARACTER".to_string(),
            confidence: 0.85,
        });

        // Rule: Prep + InitCap+ → LOCATION (weaker)
        self.add_rule(Rule {
            name: "LocationAfterPrep".to_string(),
            priority: 80,
            pattern: vec![
                PatternElement::context(TokenCondition::TextIn(vec![
                    "in".to_string(),
                    "at".to_string(),
                    "from".to_string(),
                    "to".to_string(),
                    "near".to_string(),
                    "through".to_string(),
                ])),
                PatternElement::one_or_more(TokenCondition::Shape(OrthographicShape::InitCap)),
            ],
            entity_kind: "LOCATION".to_string(),
            confidence: 0.70,
        });

        // Rule: "the" + InitCap+ → CHARACTER/ITEM (epithets like "The Doctor")
        self.add_rule(Rule {
            name: "EpithetWithThe".to_string(),
            priority: 75,
            pattern: vec![
                PatternElement::one(TokenCondition::Text("the".to_string())),
                PatternElement::one_or_more(TokenCondition::Shape(OrthographicShape::InitCap)),
            ],
            entity_kind: "CHARACTER".to_string(),
            confidence: 0.65,
        });

        // Rule: AllCaps (2+ chars) → FACTION/ORG
        self.add_rule(Rule {
            name: "AcronymEntity".to_string(),
            priority: 70,
            pattern: vec![
                PatternElement::one(TokenCondition::Shape(OrthographicShape::AllCaps)),
            ],
            entity_kind: "FACTION".to_string(),
            confidence: 0.60,
        });

        // Rule: Standalone InitCap (not at sentence start) → CHARACTER
        // Excludes common stopwords that are often capitalized but aren't entities
        self.add_rule(Rule {
            name: "StandaloneInitCap".to_string(),
            priority: 50,
            pattern: vec![
                PatternElement::one_or_more(TokenCondition::ShapeNotIn(
                    OrthographicShape::InitCap,
                    vec![
                        // Common function words often capitalized at sentence start
                        "the".to_string(), "this".to_string(), "that".to_string(),
                        "however".to_string(), "when".to_string(), "where".to_string(),
                        "what".to_string(), "which".to_string(), "while".to_string(),
                        "also".to_string(), "just".to_string(), "from".to_string(),
                        "with".to_string(), "have".to_string(), "been".to_string(),
                        "then".to_string(), "than".to_string(), "there".to_string(),
                        "here".to_string(), "some".to_string(), "such".to_string(),
                        "only".to_string(), "even".to_string(), "well".to_string(),
                        "very".to_string(), "much".to_string(), "many".to_string(),
                    ],
                )),
            ],
            entity_kind: "CHARACTER".to_string(),
            confidence: 0.55,
        });
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Clear all rules
    pub fn clear(&mut self) {
        self.rules.clear();
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tokenizer::Tokenizer;

    fn tokenize(text: &str) -> Vec<Token> {
        Tokenizer::new().tokenize(text)
    }

    #[test]
    fn test_title_pattern() {
        let engine = RuleEngine::new();
        let tokens = tokenize("Lord Voldemort appeared.");
        let matches = engine.apply(&tokens);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].rule_name, "PersonAfterTitle");
        assert_eq!(matches[0].text, "Voldemort");
        assert_eq!(matches[0].kind, "CHARACTER");
    }

    #[test]
    fn test_location_pattern() {
        let engine = RuleEngine::new();
        let tokens = tokenize("He traveled to Mordor.");
        let matches = engine.apply(&tokens);

        // Should match "Mordor" as LOCATION
        let loc_match = matches.iter().find(|m| m.kind == "LOCATION");
        assert!(loc_match.is_some());
        assert_eq!(loc_match.unwrap().text, "Mordor");
    }

    #[test]
    fn test_epithet_pattern() {
        let engine = RuleEngine::new();
        let tokens = tokenize("The Doctor arrived.");
        let matches = engine.apply(&tokens);

        // Should match "The Doctor"
        let char_match = matches.iter().find(|m| m.text.contains("Doctor"));
        assert!(char_match.is_some());
    }

    #[test]
    fn test_acronym_pattern() {
        let engine = RuleEngine::new();
        let tokens = tokenize("He joined NASA yesterday.");
        let matches = engine.apply(&tokens);

        let faction_match = matches.iter().find(|m| m.text == "NASA");
        assert!(faction_match.is_some());
        assert_eq!(faction_match.unwrap().kind, "FACTION");
    }

    #[test]
    fn test_multi_word_name() {
        let engine = RuleEngine::new();
        let tokens = tokenize("Doctor Sally Sparrow arrived.");
        let matches = engine.apply(&tokens);

        // Should match "Sally Sparrow" after "Doctor"
        let char_match = matches.iter().find(|m| m.rule_name == "PersonAfterTitle");
        assert!(char_match.is_some());
        assert!(char_match.unwrap().text.contains("Sally"));
    }

    #[test]
    fn test_priority_order() {
        let engine = RuleEngine::new();
        // "Doctor Smith" should match PersonAfterTitle (high priority)
        // not StandaloneInitCap (low priority)
        let tokens = tokenize("Doctor Smith came.");
        let matches = engine.apply(&tokens);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].rule_name, "PersonAfterTitle");
    }

    #[test]
    fn test_no_match() {
        let engine = RuleEngine::new();
        let tokens = tokenize("the wizard walked slowly.");
        let matches = engine.apply(&tokens);

        // All lowercase, no entity patterns
        assert!(matches.is_empty());
    }

    #[test]
    fn test_condition_text_in() {
        let cond = TokenCondition::TextIn(vec!["in".to_string(), "at".to_string()]);
        let classifier = OrthographicClassifier::new();
        
        let tok_in = Token::new("in", TokenKind::Word, 0, 2);
        let tok_at = Token::new("at", TokenKind::Word, 0, 2);
        let tok_on = Token::new("on", TokenKind::Word, 0, 2);

        assert!(cond.matches(&tok_in, &classifier));
        assert!(cond.matches(&tok_at, &classifier));
        assert!(!cond.matches(&tok_on, &classifier));
    }
}
