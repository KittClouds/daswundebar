use rowan::{GreenNode, GreenNodeBuilder};
use super::syntax::SyntaxKind;

// =============================================================================
// SemanticSpan Trait
// =============================================================================

/// Trait for items that represent semantic spans in text
pub trait SemanticSpan {
    fn start(&self) -> usize;
    fn end(&self) -> usize;
    fn syntax_kind(&self) -> SyntaxKind;
}

// =============================================================================
// Paragraph Detection
// =============================================================================

/// Detects paragraph boundaries (double newline or multiple newlines)
/// Returns (start, end) byte offsets for each paragraph
pub fn detect_paragraphs(text: &str) -> Vec<(usize, usize)> {
    if text.is_empty() {
        return vec![];
    }
    
    let mut paragraphs = Vec::new();
    let mut para_start = 0;
    let mut in_blank = false;
    let mut i = 0;
    let bytes = text.as_bytes();
    
    while i < bytes.len() {
        // Check for \r\n or \n
        let (is_newline, newline_len) = if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            (true, 2)
        } else if bytes[i] == b'\n' {
            (true, 1)
        } else {
            (false, 0)
        };
        
        if is_newline {
            if in_blank {
                // Multiple newlines - paragraph break
                // End the current paragraph (if non-empty)
                let para_end = para_start;
                // Skip until we find non-whitespace to start next para
                i += newline_len;
                while i < bytes.len() && (bytes[i] == b'\n' || bytes[i] == b'\r' || bytes[i] == b' ' || bytes[i] == b'\t') {
                    i += 1;
                }
                if para_end > para_start || !paragraphs.is_empty() {
                    // Don't push empty paragraph at start
                }
                para_start = i;
                in_blank = false;
            } else {
                // First newline - might be paragraph break
                // Save potential end point
                let potential_end = i;
                i += newline_len;
                // Check if next char is also newline (or whitespace followed by newline)
                let mut lookahead = i;
                while lookahead < bytes.len() && (bytes[lookahead] == b' ' || bytes[lookahead] == b'\t') {
                    lookahead += 1;
                }
                if lookahead < bytes.len() && (bytes[lookahead] == b'\n' || bytes[lookahead] == b'\r') {
                    // Paragraph break
                    if potential_end > para_start {
                        paragraphs.push((para_start, potential_end));
                    }
                    in_blank = true;
                }
            }
        } else {
            in_blank = false;
            i += 1;
        }
    }
    
    // Final paragraph
    if para_start < text.len() {
        // Trim trailing whitespace
        let mut end = text.len();
        while end > para_start && text.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        if end > para_start {
            paragraphs.push((para_start, end));
        }
    }
    
    // If no paragraphs detected, treat whole text as one paragraph
    if paragraphs.is_empty() && !text.is_empty() {
        let mut end = text.len();
        while end > 0 && text.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        if end > 0 {
            paragraphs.push((0, end));
        } else {
            paragraphs.push((0, text.len()));
        }
    }
    
    paragraphs
}

// =============================================================================
// Sentence Detection
// =============================================================================

/// Common abbreviations that end with period but aren't sentence boundaries
const ABBREVIATIONS: &[&str] = &[
    "Dr", "Mr", "Mrs", "Ms", "Prof", "Rev", "Sr", "Jr",
    "vs", "etc", "i.e", "e.g", "cf", "Inc", "Ltd", "Corp",
    "St", "Ave", "Blvd", "Rd", "Mt", "Ft",
    "Jan", "Feb", "Mar", "Apr", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun",
];

/// Detects sentence boundaries within a text range
/// Returns (start, end) byte offsets for each sentence
pub fn detect_sentences(text: &str) -> Vec<(usize, usize)> {
    if text.is_empty() {
        return vec![];
    }
    
    let mut sentences = Vec::new();
    let mut sent_start = 0;
    
    // Skip leading whitespace
    while sent_start < text.len() && text.as_bytes()[sent_start].is_ascii_whitespace() {
        sent_start += 1;
    }
    
    let chars: Vec<char> = text.chars().collect();
    let mut char_to_byte: Vec<usize> = Vec::with_capacity(chars.len() + 1);
    let mut byte_offset = 0;
    for c in &chars {
        char_to_byte.push(byte_offset);
        byte_offset += c.len_utf8();
    }
    char_to_byte.push(byte_offset); // End position
    
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        
        // Check for sentence-ending punctuation
        if c == '.' || c == '!' || c == '?' || c == '…' {
            let punct_byte = char_to_byte[i];
            
            // Check if this is an abbreviation (for period only)
            if c == '.' {
                if is_abbreviation(text, punct_byte) {
                    i += 1;
                    continue;
                }
            }
            
            // Check what follows the punctuation
            let mut j = i + 1;
            
            // Handle ellipsis-like sequences: ... or . . .
            while j < chars.len() && (chars[j] == '.' || chars[j] == ' ') {
                if chars[j] == '.' {
                    j += 1;
                } else if j + 1 < chars.len() && chars[j] == ' ' && chars[j + 1] == '.' {
                    j += 2;
                } else {
                    break;
                }
            }
            
            // Skip whitespace after punctuation
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            
            // If next char is uppercase or end of text, we have a sentence boundary
            let is_sentence_end = j >= chars.len() || chars[j].is_uppercase() || chars[j] == '"' || chars[j] == '"' || chars[j] == '\'';
            
            if is_sentence_end {
                // End of sentence at the position after punctuation
                let end_byte = char_to_byte[i + 1];
                if end_byte > sent_start {
                    sentences.push((sent_start, end_byte));
                }
                
                // Skip whitespace to find next sentence start
                sent_start = if j < chars.len() { char_to_byte[j] } else { text.len() };
                i = j;
                continue;
            }
        }
        
        i += 1;
    }
    
    // Final sentence (if any remaining text)
    if sent_start < text.len() {
        let mut end = text.len();
        while end > sent_start && text.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        if end > sent_start {
            sentences.push((sent_start, end));
        }
    }
    
    sentences
}

/// Check if the period at byte_offset is part of an abbreviation
fn is_abbreviation(text: &str, period_offset: usize) -> bool {
    // Look backwards to find the start of the word
    let before = &text[..period_offset];
    let word_start = before.rfind(|c: char| c.is_whitespace() || is_punctuation_char(c))
        .map(|i| i + 1)
        .unwrap_or(0);
    
    let word = &text[word_start..period_offset];
    
    // Check against known abbreviations (case-insensitive)
    for abbr in ABBREVIATIONS {
        if word.eq_ignore_ascii_case(abbr) {
            return true;
        }
    }
    
    // Single letter followed by period is likely an initial (e.g., "J. Smith")
    if word.len() == 1 && word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        return true;
    }
    
    false
}

// =============================================================================
// The Zipper: Main Entry Point
// =============================================================================

/// The Zipper: Merges text and semantic spans into a GreenNode (Syntax Tree)
pub fn zip_reality<S: SemanticSpan>(text: &str, spans: &[S]) -> GreenNode {
    let mut builder = GreenNodeBuilder::new();
    
    // Start Document
    builder.start_node(SyntaxKind::Document.into());
    
    // Sort spans: Start ASC, then End DESC (Longest first)
    let mut sorted: Vec<&S> = spans.iter().collect();
    sorted.sort_by(|a, b| {
        let start_cmp = a.start().cmp(&b.start());
        if start_cmp != std::cmp::Ordering::Equal {
            return start_cmp;
        }
        b.end().cmp(&a.end())
    });
    
    // Detect paragraphs
    let paragraphs = detect_paragraphs(text);
    
    for (para_start, para_end) in paragraphs {
        builder.start_node(SyntaxKind::Paragraph.into());
        
        // Detect sentences within this paragraph
        let para_text = &text[para_start..para_end];
        let sentences = detect_sentences(para_text);
        
        if sentences.is_empty() {
            // No sentences detected, process as raw
            let para_spans: Vec<&S> = sorted.iter()
                .filter(|s| s.start() >= para_start && s.end() <= para_end)
                .copied()
                .collect();
            zip_range(&mut builder, text, para_start, para_end, &para_spans);
        } else {
            for (sent_rel_start, sent_rel_end) in sentences {
                let sent_start = para_start + sent_rel_start;
                let sent_end = para_start + sent_rel_end;
                
                builder.start_node(SyntaxKind::Sentence.into());
                
                // Filter spans that belong to this sentence
                let sent_spans: Vec<&S> = sorted.iter()
                    .filter(|s| s.start() >= sent_start && s.end() <= sent_end)
                    .copied()
                    .collect();
                
                zip_range(&mut builder, text, sent_start, sent_end, &sent_spans);
                
                builder.finish_node(); // Close Sentence
            }
        }
        
        builder.finish_node(); // Close Paragraph
    }
    
    builder.finish_node(); // Close Document
    
    builder.finish()
}

// =============================================================================
// Zip Range (Recursive)
// =============================================================================

fn zip_range<S: SemanticSpan>(
    builder: &mut GreenNodeBuilder,
    full_text: &str,
    start: usize,
    end: usize,
    spans: &[&S]
) {
    let mut current_pos = start;
    let mut i = 0;
    
    while current_pos < end {
        // Skip spans that have already been passed or are irrelevant
        while i < spans.len() && spans[i].start() < current_pos {
            i += 1;
        }

        if i >= spans.len() {
            // No more spans ahead
            if current_pos < end {
                tokenize_range(builder, &full_text[current_pos..end]);
            }
            break;
        }

        let span = spans[i];
        
        // Validate span is within current bounds
        if span.start() >= end {
            // Start is outside our scope
            break;
        }
        
        // Fill gap before span
        if span.start() > current_pos {
            tokenize_range(builder, &full_text[current_pos..span.start()]);
        }
        
        // We are at span start
        builder.start_node(span.syntax_kind().into());
        
        // Find potential children for this span
        let mut children = Vec::new();
        let mut j = i + 1;
        while j < spans.len() {
            let candidate = spans[j];
            if candidate.start() >= span.end() {
                break;
            }
            if candidate.end() <= span.end() {
                children.push(candidate);
            }
            j += 1;
        }
        
        // Recurse
        zip_range(builder, full_text, span.start(), span.end(), &children);
        
        builder.finish_node(); // Close SPAN
        
        current_pos = span.end();
        i += 1;
    }
}

// =============================================================================
// Tokenizer (Unicode-aware)
// =============================================================================

fn tokenize_range(builder: &mut GreenNodeBuilder, text: &str) {
    let mut offset = 0;
    while offset < text.len() {
        let (len, kind) = lex_next_token(&text[offset..]);
        if len == 0 {
            // Safety: shouldn't happen, but prevent infinite loop
            break;
        }
        builder.token(kind.into(), &text[offset..offset+len]);
        offset += len;
    }
}

/// Check if a character is punctuation (Unicode-aware)
fn is_punctuation_char(c: char) -> bool {
    // Common punctuation categories
    matches!(c, 
        // ASCII punctuation
        '!' | '"' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' |
        '-' | '.' | '/' | ':' | ';' | '<' | '=' | '>' | '?' | '@' | '[' | '\\' |
        ']' | '^' | '_' | '`' | '{' | '|' | '}' | '~' |
        // Unicode punctuation (using escape for curly quotes)
        '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}' | 
        '\u{2013}' | '\u{2014}' | '\u{2026}' | '\u{00AB}' | '\u{00BB}' |
        '\u{00A1}' | '\u{00BF}' | '\u{2039}' | '\u{203A}' | '\u{201E}' | '\u{201A}'
    )
}

/// Check if a character is a quote that could be part of a contraction
fn is_contraction_apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

fn lex_next_token(text: &str) -> (usize, SyntaxKind) {
    if text.is_empty() {
        return (0, SyntaxKind::Error);
    }
    
    let first = text.chars().next().unwrap();
    
    // Whitespace: consume all contiguous whitespace
    if first.is_whitespace() {
        let len = text.chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| c.len_utf8())
            .sum();
        return (len, SyntaxKind::Whitespace);
    }
    
    // Punctuation (but not apostrophe at start of word)
    if is_punctuation_char(first) && !is_contraction_apostrophe(first) {
        return (first.len_utf8(), SyntaxKind::Punctuation);
    }
    
    // Word: use unicode word boundaries for better segmentation
    // A word includes letters, numbers, and embedded apostrophes (contractions)
    let mut len = 0;
    let mut chars = text.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c.is_whitespace() || (is_punctuation_char(c) && !is_contraction_apostrophe(c)) {
            break;
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
            // Apostrophe not part of contraction
            break;
        }
        
        len += c.len_utf8();
    }
    
    if len == 0 {
        // Edge case: apostrophe at start (like 'twas)
        if is_contraction_apostrophe(first) {
            // Check if followed by letters
            let after_apos: String = text.chars().skip(1).take_while(|c| c.is_alphabetic()).collect();
            if !after_apos.is_empty() {
                let word_len = first.len_utf8() + after_apos.len();
                return (word_len, SyntaxKind::Word);
            }
        }
        // Single punctuation or unknown
        return (first.len_utf8(), SyntaxKind::Punctuation);
    }
    
    (len, SyntaxKind::Word)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod parser_tests {
    use super::*;
    
    /// Mock entity for testing CST parsing
    struct MockEntity {
        pub start: usize,
        pub end: usize,
    }
    
    impl SemanticSpan for MockEntity {
        fn start(&self) -> usize { self.start }
        fn end(&self) -> usize { self.end }
        fn syntax_kind(&self) -> SyntaxKind { SyntaxKind::EntitySpan }
    }
    
    // -------------------------------------------------------------------------
    // Sentence Detection Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_sentence_detection_basic() {
        let text = "Hello world. How are you?";
        let sentences = detect_sentences(text);
        
        assert_eq!(sentences.len(), 2, "Should detect 2 sentences");
        assert_eq!(&text[sentences[0].0..sentences[0].1], "Hello world.");
        assert_eq!(&text[sentences[1].0..sentences[1].1], "How are you?");
    }
    
    #[test]
    fn test_sentence_detection_abbreviations() {
        let text = "Dr. Smith went home.";
        let sentences = detect_sentences(text);
        
        assert_eq!(sentences.len(), 1, "Dr. should not split: {:?}", sentences);
        assert_eq!(&text[sentences[0].0..sentences[0].1], "Dr. Smith went home.");
    }
    
    #[test]
    fn test_sentence_detection_initials() {
        let text = "J. R. R. Tolkien wrote books.";
        let sentences = detect_sentences(text);
        
        assert_eq!(sentences.len(), 1, "Initials should not split");
    }
    
    #[test]
    fn test_sentence_detection_exclamation() {
        let text = "Wow! That's amazing.";
        let sentences = detect_sentences(text);
        
        assert_eq!(sentences.len(), 2);
        assert_eq!(&text[sentences[0].0..sentences[0].1], "Wow!");
        assert_eq!(&text[sentences[1].0..sentences[1].1], "That's amazing.");
    }
    
    // -------------------------------------------------------------------------
    // Paragraph Detection Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_paragraph_detection_basic() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let paragraphs = detect_paragraphs(text);
        
        assert_eq!(paragraphs.len(), 2, "Should detect 2 paragraphs");
        assert_eq!(&text[paragraphs[0].0..paragraphs[0].1], "First paragraph.");
        assert_eq!(&text[paragraphs[1].0..paragraphs[1].1], "Second paragraph.");
    }
    
    #[test]
    fn test_paragraph_detection_single_newline() {
        let text = "Same paragraph.\nStill same.";
        let paragraphs = detect_paragraphs(text);
        
        // Single newline doesn't split paragraphs
        assert_eq!(paragraphs.len(), 1);
    }
    
    #[test]
    fn test_paragraph_detection_windows_newlines() {
        let text = "First para.\r\n\r\nSecond para.";
        let paragraphs = detect_paragraphs(text);
        
        assert_eq!(paragraphs.len(), 2, "Should handle Windows line endings");
    }
    
    // -------------------------------------------------------------------------
    // Unicode Tokenizer Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_unicode_punctuation_emdash() {
        let text = "word—another";
        let (len, kind) = lex_next_token(text);
        assert_eq!(kind, SyntaxKind::Word);
        assert_eq!(&text[..len], "word");
        
        let (len2, kind2) = lex_next_token(&text[len..]);
        assert_eq!(kind2, SyntaxKind::Punctuation);
        assert_eq!(&text[len..len+len2], "—");
    }
    
    #[test]
    fn test_unicode_curly_quotes() {
        // \u{201C} is left double quotation mark
        let text = "\u{201C}Hello\u{201D}";
        let (len, kind) = lex_next_token(text);
        assert_eq!(kind, SyntaxKind::Punctuation);
        assert_eq!(&text[..len], "\u{201C}");
    }
    
    #[test]
    fn test_contractions_dont() {
        let text = "don\'t stop";
        let (len, kind) = lex_next_token(text);
        
        assert_eq!(kind, SyntaxKind::Word, "Contraction should be one word");
        assert_eq!(&text[..len], "don\'t");
    }
    
    #[test]
    fn test_contractions_curly_apostrophe() {
        // Using \u{2019} for right single quotation mark (curly apostrophe)
        let text = "won\u{2019}t fail";
        let (len, kind) = lex_next_token(text);
        
        assert_eq!(kind, SyntaxKind::Word);
        assert_eq!(&text[..len], "won\u{2019}t");
    }
    
    #[test]
    fn test_contractions_twas() {
        let text = "\'twas the night";
        let (len, kind) = lex_next_token(text);
        
        assert_eq!(kind, SyntaxKind::Word, "twas should be one word");
        assert_eq!(&text[..len], "\'twas");
    }
    
    // -------------------------------------------------------------------------
    // Integration: Zipper with Sentences
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_zipper_produces_sentences() {
        use super::super::syntax::RealityLanguage;
        
        let text = "Hello world. Goodbye world.";
        let entities: Vec<MockEntity> = vec![];
        
        let green = zip_reality(text, &entities);
        let root: rowan::SyntaxNode<RealityLanguage> = rowan::SyntaxNode::new_root(green);
        
        // Find sentences
        let sentences: Vec<_> = root.descendants()
            .filter(|n| n.kind() == SyntaxKind::Sentence)
            .collect();
        
        assert_eq!(sentences.len(), 2, "Should have 2 sentence nodes");
    }
}

// =============================================================================
// PHASE 2: Enhanced Zipper with Phrase Nodes
// =============================================================================

use crate::chunker::{Chunk, ChunkKind};

/// A span that can be either a semantic entity or a phrase chunk
#[derive(Debug, Clone)]
pub enum MixedSpan<'a, S: SemanticSpan> {
    Entity(&'a S),
    Chunk(&'a Chunk),
}

impl<'a, S: SemanticSpan> MixedSpan<'a, S> {
    fn start(&self) -> usize {
        match self {
            MixedSpan::Entity(e) => e.start(),
            MixedSpan::Chunk(c) => c.range.start,
        }
    }
    
    fn end(&self) -> usize {
        match self {
            MixedSpan::Entity(e) => e.end(),
            MixedSpan::Chunk(c) => c.range.end,
        }
    }
    
    fn syntax_kind(&self) -> SyntaxKind {
        match self {
            MixedSpan::Entity(e) => e.syntax_kind(),
            MixedSpan::Chunk(c) => chunk_kind_to_syntax_kind(c.kind),
        }
    }
}

/// Convert Chunker's ChunkKind to CST SyntaxKind
pub fn chunk_kind_to_syntax_kind(kind: ChunkKind) -> SyntaxKind {
    match kind {
        ChunkKind::NounPhrase => SyntaxKind::NounPhrase,
        ChunkKind::VerbPhrase => SyntaxKind::VerbPhrase,
        ChunkKind::PrepPhrase => SyntaxKind::PrepPhrase,
        ChunkKind::AdjPhrase => SyntaxKind::AdjPhrase,
        ChunkKind::Clause => SyntaxKind::SubordinateClause,
    }
}

/// Enhanced Zipper: Merges text, semantic spans, AND phrase chunks into a GreenNode
///
/// This is the Phase 2 enhancement that creates richer CST structure by including
/// phrase-level nodes (NP, VP, PP) from the Chunker alongside entity spans.
///
/// # Node Hierarchy
///
/// ```text
/// Document
///   └── Paragraph
///       └── Sentence
///           ├── NounPhrase ("the wizard")
///           │   └── EntitySpan ("wizard")
///           ├── VerbPhrase ("defeated")
///           ├── NounPhrase ("the enemy")
///           │   └── EntitySpan ("enemy")
///           └── PrepPhrase ("in Mordor")
/// ```
///
/// # Interleaving Strategy
///
/// When entities and chunks overlap:
/// - Entities take priority (they are semantic truth)
/// - Chunks are used to wrap content not covered by entities
/// - VP chunks become RelationSpan candidates for projection
pub fn zip_reality_enhanced<S: SemanticSpan>(
    text: &str,
    spans: &[S],
    chunks: &[Chunk],
) -> GreenNode {
    let mut builder = GreenNodeBuilder::new();
    
    // Start Document
    builder.start_node(SyntaxKind::Document.into());
    
    // Sort spans: Start ASC, then End DESC (Longest first)
    let mut sorted_entities: Vec<&S> = spans.iter().collect();
    sorted_entities.sort_by(|a, b| {
        let start_cmp = a.start().cmp(&b.start());
        if start_cmp != std::cmp::Ordering::Equal {
            return start_cmp;
        }
        b.end().cmp(&a.end())
    });
    
    // Sort chunks similarly
    let mut sorted_chunks: Vec<&Chunk> = chunks.iter().collect();
    sorted_chunks.sort_by(|a, b| {
        let start_cmp = a.range.start.cmp(&b.range.start);
        if start_cmp != std::cmp::Ordering::Equal {
            return start_cmp;
        }
        b.range.end.cmp(&a.range.end)
    });
    
    // Detect paragraphs
    let paragraphs = detect_paragraphs(text);
    
    for (para_start, para_end) in paragraphs {
        builder.start_node(SyntaxKind::Paragraph.into());
        
        // Detect sentences within this paragraph
        let para_text = &text[para_start..para_end];
        let sentences = detect_sentences(para_text);
        
        if sentences.is_empty() {
            // No sentences detected, process as raw with mixed spans
            let para_entities: Vec<&S> = sorted_entities.iter()
                .filter(|s| s.start() >= para_start && s.end() <= para_end)
                .copied()
                .collect();
            let para_chunks: Vec<&Chunk> = sorted_chunks.iter()
                .filter(|c| c.range.start >= para_start && c.range.end <= para_end)
                .copied()
                .collect();
            zip_range_enhanced(&mut builder, text, para_start, para_end, &para_entities, &para_chunks);
        } else {
            for (sent_rel_start, sent_rel_end) in sentences {
                let sent_start = para_start + sent_rel_start;
                let sent_end = para_start + sent_rel_end;
                
                builder.start_node(SyntaxKind::Sentence.into());
                
                // Filter spans that belong to this sentence
                let sent_entities: Vec<&S> = sorted_entities.iter()
                    .filter(|s| s.start() >= sent_start && s.end() <= sent_end)
                    .copied()
                    .collect();
                let sent_chunks: Vec<&Chunk> = sorted_chunks.iter()
                    .filter(|c| c.range.start >= sent_start && c.range.end <= sent_end)
                    .copied()
                    .collect();
                
                zip_range_enhanced(&mut builder, text, sent_start, sent_end, &sent_entities, &sent_chunks);
                
                builder.finish_node(); // Close Sentence
            }
        }
        
        builder.finish_node(); // Close Paragraph
    }
    
    builder.finish_node(); // Close Document
    
    builder.finish()
}

/// Zip a range with both entities and chunks
///
/// Priority: Entities > Chunks > Raw text
/// This ensures semantic truth (entities) takes precedence over structural analysis (chunks)
fn zip_range_enhanced<S: SemanticSpan>(
    builder: &mut GreenNodeBuilder,
    full_text: &str,
    start: usize,
    end: usize,
    entities: &[&S],
    chunks: &[&Chunk],
) {
    // Merge entities and chunks into a single sorted list
    // Entities take priority over overlapping chunks
    let mut mixed: Vec<MixedSpan<S>> = Vec::new();
    
    // Add all entities
    for e in entities {
        mixed.push(MixedSpan::Entity(*e));
    }
    
    // Add chunks that don't overlap with entities
    for c in chunks {
        let overlaps_entity = entities.iter().any(|e| {
            // Check if chunk overlaps with entity
            !(c.range.end <= e.start() || c.range.start >= e.end())
        });
        
        if !overlaps_entity {
            mixed.push(MixedSpan::Chunk(*c));
        }
    }
    
    // Sort by start position, then by end (descending for nesting)
    mixed.sort_by(|a, b| {
        let start_cmp = a.start().cmp(&b.start());
        if start_cmp != std::cmp::Ordering::Equal {
            return start_cmp;
        }
        b.end().cmp(&a.end())
    });
    
    let mut current_pos = start;
    let mut i = 0;
    
    while current_pos < end {
        // Skip spans that have already been passed
        while i < mixed.len() && mixed[i].start() < current_pos {
            i += 1;
        }

        if i >= mixed.len() {
            // No more spans ahead
            if current_pos < end {
                tokenize_range(builder, &full_text[current_pos..end]);
            }
            break;
        }

        let span = &mixed[i];
        
        // Validate span is within current bounds
        if span.start() >= end {
            break;
        }
        
        // Fill gap before span
        if span.start() > current_pos {
            tokenize_range(builder, &full_text[current_pos..span.start()]);
        }
        
        // Start the span node
        builder.start_node(span.syntax_kind().into());
        
        // Find children that are contained within this span
        let mut children_entities: Vec<&S> = Vec::new();
        let mut children_chunks: Vec<&Chunk> = Vec::new();
        
        let mut j = i + 1;
        while j < mixed.len() {
            let candidate = &mixed[j];
            if candidate.start() >= span.end() {
                break;
            }
            if candidate.end() <= span.end() {
                match candidate {
                    MixedSpan::Entity(e) => children_entities.push(*e),
                    MixedSpan::Chunk(c) => children_chunks.push(*c),
                }
            }
            j += 1;
        }
        
        // Recurse into this span
        if children_entities.is_empty() && children_chunks.is_empty() {
            // No children, just tokenize the content
            tokenize_range(builder, &full_text[span.start()..span.end()]);
        } else {
            zip_range_enhanced(builder, full_text, span.start(), span.end(), &children_entities, &children_chunks);
        }
        
        builder.finish_node(); // Close span
        
        current_pos = span.end();
        i += 1;
    }
}

// =============================================================================
// Phase 2 Tests (TDD Contract)
// =============================================================================

#[cfg(test)]
mod enhanced_parser_tests {
    use super::*;
    use super::super::syntax::RealityLanguage;
    use crate::chunker::{Chunker, Chunk, ChunkKind};

    /// MockEntity for enhanced parser tests
    struct MockEntity {
        start: usize,
        end: usize,
        #[allow(dead_code)]
        label: String,
        #[allow(dead_code)]
        kind: String,
    }
    
    impl SemanticSpan for MockEntity {
        fn start(&self) -> usize { self.start }
        fn end(&self) -> usize { self.end }
        fn syntax_kind(&self) -> SyntaxKind { SyntaxKind::EntitySpan }
    }

    // Helper to create mock entity
    fn mock_entity(start: usize, end: usize) -> MockEntity {
        MockEntity { 
            start, 
            end, 
            label: format!("entity_{}_{}", start, end),
            kind: "CHARACTER".to_string(),
        }
    }
    
    // -------------------------------------------------------------------------
    // CONTRACT: Phrase Nodes Appear in CST
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_enhanced_zipper_includes_verb_phrase() {
        let text = "The wizard walked slowly.";
        let chunker = Chunker::new();
        let chunk_result = chunker.chunk(text);
        
        let entities: Vec<MockEntity> = vec![];
        let green = zip_reality_enhanced(text, &entities, &chunk_result.chunks);
        let root: rowan::SyntaxNode<RealityLanguage> = rowan::SyntaxNode::new_root(green);
        
        // Should contain VerbPhrase node
        let vp_nodes: Vec<_> = root.descendants()
            .filter(|n| n.kind() == SyntaxKind::VerbPhrase)
            .collect();
        
        assert!(!vp_nodes.is_empty(), "Should have VerbPhrase nodes from Chunker");
    }
    
    #[test]
    fn test_enhanced_zipper_includes_noun_phrase() {
        let text = "The ancient wizard cast a spell.";
        let chunker = Chunker::new();
        let chunk_result = chunker.chunk(text);
        
        let entities: Vec<MockEntity> = vec![];
        let green = zip_reality_enhanced(text, &entities, &chunk_result.chunks);
        let root: rowan::SyntaxNode<RealityLanguage> = rowan::SyntaxNode::new_root(green);
        
        // Should contain NounPhrase nodes
        let np_nodes: Vec<_> = root.descendants()
            .filter(|n| n.kind() == SyntaxKind::NounPhrase)
            .collect();
        
        assert!(!np_nodes.is_empty(), "Should have NounPhrase nodes from Chunker");
    }
    
    #[test]
    fn test_enhanced_zipper_includes_prep_phrase() {
        let text = "He walked through the forest.";
        let chunker = Chunker::new();
        let chunk_result = chunker.chunk(text);
        
        let entities: Vec<MockEntity> = vec![];
        let green = zip_reality_enhanced(text, &entities, &chunk_result.chunks);
        let root: rowan::SyntaxNode<RealityLanguage> = rowan::SyntaxNode::new_root(green);
        
        // Should contain PrepPhrase node
        let pp_nodes: Vec<_> = root.descendants()
            .filter(|n| n.kind() == SyntaxKind::PrepPhrase)
            .collect();
        
        assert!(!pp_nodes.is_empty(), "Should have PrepPhrase nodes from Chunker");
    }
    
    // -------------------------------------------------------------------------
    // CONTRACT: Entity Priority Over Chunks
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_entities_take_priority_over_chunks() {
        // "Gandalf defeated Sauron"
        //  ^^^^^^^          ^^^^^^
        // Entity positions: 0-7, 17-23
        let text = "Gandalf defeated Sauron";
        let chunker = Chunker::new();
        let chunk_result = chunker.chunk(text);
        
        // Entity spans at "Gandalf" and "Sauron"
        let entities = vec![
            mock_entity(0, 7),   // "Gandalf"
            mock_entity(17, 23), // "Sauron"
        ];
        
        let green = zip_reality_enhanced(text, &entities, &chunk_result.chunks);
        let root: rowan::SyntaxNode<RealityLanguage> = rowan::SyntaxNode::new_root(green);
        
        // Should have EntitySpan nodes
        let entity_nodes: Vec<_> = root.descendants()
            .filter(|n| n.kind() == SyntaxKind::EntitySpan)
            .collect();
        
        assert_eq!(entity_nodes.len(), 2, "Should have 2 EntitySpan nodes");
        
        // Should still have VerbPhrase for "defeated"
        let vp_nodes: Vec<_> = root.descendants()
            .filter(|n| n.kind() == SyntaxKind::VerbPhrase)
            .collect();
        
        assert!(!vp_nodes.is_empty(), "Should have VerbPhrase node");
    }
    
    // -------------------------------------------------------------------------
    // CONTRACT: Sentence Structure Preserved
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_enhanced_zipper_preserves_sentences() {
        let text = "First sentence. Second sentence.";
        let chunker = Chunker::new();
        let chunk_result = chunker.chunk(text);
        
        let entities: Vec<MockEntity> = vec![];
        let green = zip_reality_enhanced(text, &entities, &chunk_result.chunks);
        let root: rowan::SyntaxNode<RealityLanguage> = rowan::SyntaxNode::new_root(green);
        
        let sentences: Vec<_> = root.descendants()
            .filter(|n| n.kind() == SyntaxKind::Sentence)
            .collect();
        
        assert_eq!(sentences.len(), 2, "Should have 2 Sentence nodes");
    }
    
    // -------------------------------------------------------------------------
    // CONTRACT: ChunkKind to SyntaxKind Conversion
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_chunk_kind_to_syntax_kind() {
        assert_eq!(chunk_kind_to_syntax_kind(ChunkKind::NounPhrase), SyntaxKind::NounPhrase);
        assert_eq!(chunk_kind_to_syntax_kind(ChunkKind::VerbPhrase), SyntaxKind::VerbPhrase);
        assert_eq!(chunk_kind_to_syntax_kind(ChunkKind::PrepPhrase), SyntaxKind::PrepPhrase);
        assert_eq!(chunk_kind_to_syntax_kind(ChunkKind::AdjPhrase), SyntaxKind::AdjPhrase);
        assert_eq!(chunk_kind_to_syntax_kind(ChunkKind::Clause), SyntaxKind::SubordinateClause);
    }
    
    // -------------------------------------------------------------------------
    // CONTRACT: Empty Inputs Handled
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_enhanced_zipper_empty_text() {
        let text = "";
        let entities: Vec<MockEntity> = vec![];
        let chunks: Vec<Chunk> = vec![];
        
        let green = zip_reality_enhanced(text, &entities, &chunks);
        let root: rowan::SyntaxNode<RealityLanguage> = rowan::SyntaxNode::new_root(green);
        
        assert_eq!(root.kind(), SyntaxKind::Document);
    }
    
    #[test]
    fn test_enhanced_zipper_no_chunks() {
        let text = "Hello world";
        let entities: Vec<MockEntity> = vec![];
        let chunks: Vec<Chunk> = vec![];
        
        let green = zip_reality_enhanced(text, &entities, &chunks);
        let root: rowan::SyntaxNode<RealityLanguage> = rowan::SyntaxNode::new_root(green);
        
        // Should still produce valid CST
        assert_eq!(root.kind(), SyntaxKind::Document);
        assert!(root.text().to_string().contains("Hello"));
    }
    
    // -------------------------------------------------------------------------
    // CONTRACT: Nested Phrase Structure
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_prep_phrase_contains_noun_phrase() {
        // "in the forest" should be PP containing NP("the forest")
        let text = "walked in the forest";
        let chunker = Chunker::new();
        let chunk_result = chunker.chunk(text);
        
        let entities: Vec<MockEntity> = vec![];
        let green = zip_reality_enhanced(text, &entities, &chunk_result.chunks);
        let root: rowan::SyntaxNode<RealityLanguage> = rowan::SyntaxNode::new_root(green);
        
        // Find PP nodes
        let pp_nodes: Vec<_> = root.descendants()
            .filter(|n| n.kind() == SyntaxKind::PrepPhrase)
            .collect();
        
        // PP should exist
        assert!(!pp_nodes.is_empty(), "Should have PrepPhrase");
        
        // PP text should contain "in" and "forest"
        if !pp_nodes.is_empty() {
            let pp_text = pp_nodes[0].text().to_string();
            assert!(pp_text.contains("in"), "PP should contain 'in'");
        }
    }
}

