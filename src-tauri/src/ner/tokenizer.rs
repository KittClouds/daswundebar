//! Tokenizer - Unicode-aware text tokenization with byte offsets
//!
//! Produces tokens with precise byte boundaries for NER span reporting.
//! Handles contractions, Unicode punctuation, and mixed-case words.

use serde::{Deserialize, Serialize};

// =============================================================================
// Types
// =============================================================================

/// Token kind classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenKind {
    /// Alphabetic word (may include contractions like "don't")
    Word,
    /// Numeric sequence (digits, may include decimal point)
    Number,
    /// Mixed alphanumeric (e.g., "COVID-19", "B52")
    Mixed,
    /// Punctuation character
    Punctuation,
    /// Whitespace (space, tab, newline)
    Whitespace,
}

/// A single token with byte offsets
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    /// The token text
    pub text: String,
    /// Token classification
    pub kind: TokenKind,
    /// Start byte offset in source text
    pub byte_start: usize,
    /// End byte offset in source text (exclusive)
    pub byte_end: usize,
}

impl Token {
    pub fn new(text: impl Into<String>, kind: TokenKind, byte_start: usize, byte_end: usize) -> Self {
        Token {
            text: text.into(),
            kind,
            byte_start,
            byte_end,
        }
    }

    /// Get the length in bytes
    pub fn len(&self) -> usize {
        self.byte_end - self.byte_start
    }

    /// Check if token is empty
    pub fn is_empty(&self) -> bool {
        self.byte_start == self.byte_end
    }
}

// =============================================================================
// Tokenizer
// =============================================================================

/// Unicode-aware tokenizer with byte offset tracking
pub struct Tokenizer {
    /// Whether to include whitespace tokens in output
    include_whitespace: bool,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer {
    /// Create a new tokenizer (excludes whitespace by default)
    pub fn new() -> Self {
        Tokenizer {
            include_whitespace: false,
        }
    }

    /// Create a tokenizer that includes whitespace tokens
    pub fn with_whitespace() -> Self {
        Tokenizer {
            include_whitespace: true,
        }
    }

    /// Tokenize text into tokens with byte offsets
    pub fn tokenize(&self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut offset = 0;

        while offset < text.len() {
            let (len, kind) = lex_next(&text[offset..]);
            if len == 0 {
                // Safety: shouldn't happen, but prevent infinite loop
                break;
            }

            if kind != TokenKind::Whitespace || self.include_whitespace {
                tokens.push(Token::new(
                    &text[offset..offset + len],
                    kind,
                    offset,
                    offset + len,
                ));
            }

            offset += len;
        }

        tokens
    }

    /// Tokenize and return only word tokens (for NER)
    pub fn tokenize_words(&self, text: &str) -> Vec<Token> {
        self.tokenize(text)
            .into_iter()
            .filter(|t| t.kind == TokenKind::Word || t.kind == TokenKind::Mixed)
            .collect()
    }
}

// =============================================================================
// Lexer Core (adapted from reality/parser.rs)
// =============================================================================

/// Check if a character is punctuation (Unicode-aware)
fn is_punctuation_char(c: char) -> bool {
    matches!(c, 
        // ASCII punctuation
        '!' | '"' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' |
        '-' | '.' | '/' | ':' | ';' | '<' | '=' | '>' | '?' | '@' | '[' | '\\' |
        ']' | '^' | '_' | '`' | '{' | '|' | '}' | '~' |
        // Unicode punctuation
        '\u{201C}' | '\u{201D}' | // " "
        '\u{2018}' | '\u{2019}' | // ' '
        '\u{2013}' | '\u{2014}' | // – —
        '\u{2026}' |              // …
        '\u{00AB}' | '\u{00BB}' | // « »
        '\u{00A1}' | '\u{00BF}' | // ¡ ¿
        '\u{2039}' | '\u{203A}' | // ‹ ›
        '\u{201E}' | '\u{201A}'   // „ ‚
    )
}

/// Check if a character is a quote that could be part of a contraction
fn is_contraction_apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}' // straight or curly apostrophe
}

/// Lex the next token from text, returns (byte_length, kind)
fn lex_next(text: &str) -> (usize, TokenKind) {
    if text.is_empty() {
        return (0, TokenKind::Punctuation);
    }

    let first = text.chars().next().unwrap();

    // Whitespace: consume all contiguous whitespace
    if first.is_whitespace() {
        let len = text
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| c.len_utf8())
            .sum();
        return (len, TokenKind::Whitespace);
    }

    // Pure number sequence
    if first.is_ascii_digit() {
        let mut len = 0;
        let mut has_alpha = false;
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c.is_ascii_digit() || c == '.' || c == ',' {
                len += c.len_utf8();
            } else if c.is_alphabetic() {
                // Mixed alphanumeric like "B52" or "COVID-19"
                has_alpha = true;
                len += c.len_utf8();
            } else if c == '-' {
                // Check if followed by digit (e.g., date range) or letter
                if let Some(&next) = chars.peek() {
                    if next.is_alphanumeric() {
                        len += c.len_utf8();
                        has_alpha = true;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        return (len, if has_alpha { TokenKind::Mixed } else { TokenKind::Number });
    }

    // Punctuation (but not apostrophe at start of word like 'twas)
    if is_punctuation_char(first) && !is_contraction_apostrophe(first) {
        return (first.len_utf8(), TokenKind::Punctuation);
    }

    // Word: includes letters, embedded apostrophes (contractions), hyphens
    let mut len = 0;
    let mut has_digit = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_whitespace() || (is_punctuation_char(c) && !is_contraction_apostrophe(c) && c != '-') {
            break;
        }

        if c.is_ascii_digit() {
            has_digit = true;
        }

        // Handle apostrophes/quotes - include if followed by a letter (contraction)
        if is_contraction_apostrophe(c) {
            if let Some(&next) = chars.peek() {
                if next.is_alphabetic() {
                    // It's a contraction like "don't"
                    len += c.len_utf8();
                    continue;
                }
            }
            // Apostrophe not part of contraction (possessive ending)
            break;
        }

        // Handle hyphen - include if followed by alphanumeric (compound word)
        if c == '-' {
            if let Some(&next) = chars.peek() {
                if next.is_alphanumeric() {
                    len += c.len_utf8();
                    continue;
                }
            }
            break;
        }

        len += c.len_utf8();
    }

    if len == 0 {
        // Edge case: apostrophe at start (like 'twas)
        if is_contraction_apostrophe(first) {
            let after_apos: String = text.chars().skip(1).take_while(|c| c.is_alphabetic()).collect();
            if !after_apos.is_empty() {
                let word_len = first.len_utf8() + after_apos.len();
                return (word_len, TokenKind::Word);
            }
        }
        // Single punctuation or unknown
        return (first.len_utf8(), TokenKind::Punctuation);
    }

    (len, if has_digit { TokenKind::Mixed } else { TokenKind::Word })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_words() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("hello world");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "hello");
        assert_eq!(tokens[0].kind, TokenKind::Word);
        assert_eq!(tokens[0].byte_start, 0);
        assert_eq!(tokens[0].byte_end, 5);

        assert_eq!(tokens[1].text, "world");
        assert_eq!(tokens[1].byte_start, 6);
    }

    #[test]
    fn test_punctuation() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Hello, world!");

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].text, "Hello");
        assert_eq!(tokens[1].text, ",");
        assert_eq!(tokens[1].kind, TokenKind::Punctuation);
        assert_eq!(tokens[2].text, "world");
        assert_eq!(tokens[3].text, "!");
    }

    #[test]
    fn test_contractions() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("don't stop");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "don't");
        assert_eq!(tokens[0].kind, TokenKind::Word);
    }

    #[test]
    fn test_curly_apostrophe() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("won\u{2019}t fail");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "won\u{2019}t");
        assert_eq!(tokens[0].kind, TokenKind::Word);
    }

    #[test]
    fn test_twas() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("'twas the night");

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "'twas");
        assert_eq!(tokens[0].kind, TokenKind::Word);
    }

    #[test]
    fn test_numbers() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("42 items 3.14");

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "42");
        assert_eq!(tokens[0].kind, TokenKind::Number);
        assert_eq!(tokens[2].text, "3.14");
        assert_eq!(tokens[2].kind, TokenKind::Number);
    }

    #[test]
    fn test_mixed_alphanumeric() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("COVID-19 B52");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "COVID-19");
        assert_eq!(tokens[0].kind, TokenKind::Mixed);
        assert_eq!(tokens[1].text, "B52");
        assert_eq!(tokens[1].kind, TokenKind::Mixed);
    }

    #[test]
    fn test_unicode_emdash() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("word—another");

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "word");
        assert_eq!(tokens[1].text, "—");
        assert_eq!(tokens[1].kind, TokenKind::Punctuation);
        assert_eq!(tokens[2].text, "another");
    }

    #[test]
    fn test_curly_quotes() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("\u{201C}Hello\u{201D}");

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "\u{201C}");
        assert_eq!(tokens[0].kind, TokenKind::Punctuation);
        assert_eq!(tokens[1].text, "Hello");
        assert_eq!(tokens[2].text, "\u{201D}");
    }

    #[test]
    fn test_hyphenated_name() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Sally-Anne walked");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "Sally-Anne");
        assert_eq!(tokens[0].kind, TokenKind::Word);
    }

    #[test]
    fn test_byte_offsets_unicode() {
        let tokenizer = Tokenizer::new();
        // "café" has 5 bytes (é is 2 bytes in UTF-8)
        let tokens = tokenizer.tokenize("café time");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "café");
        assert_eq!(tokens[0].byte_start, 0);
        assert_eq!(tokens[0].byte_end, 5); // 'c'=1, 'a'=1, 'f'=1, 'é'=2
        assert_eq!(tokens[1].byte_start, 6);
    }

    #[test]
    fn test_with_whitespace() {
        let tokenizer = Tokenizer::with_whitespace();
        let tokens = tokenizer.tokenize("a b");

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1].kind, TokenKind::Whitespace);
    }

    #[test]
    fn test_possessive_split() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Sally's book");

        // Note: Possessives like "Sally's" are kept as single token (same as contractions)
        // This matches behavior of "don't" → single token
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "Sally's");
        assert_eq!(tokens[1].text, "book");
    }
}
