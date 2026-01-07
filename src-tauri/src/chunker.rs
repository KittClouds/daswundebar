//! Chunker - NP/VP/PP Detection (Native Tauri)
//!
//! Phase 1 of the dependency parser for worldbuilders.
//! Identifies noun phrases, verb phrases, prepositional phrases via
//! rule-based head-finding (no neural models).
//!
//! # Design
//!
//! | Pattern           | Head       | Example                    |
//! |-------------------|------------|----------------------------|
//! | Det? Adj* Noun+   | Last noun  | "the old grey **wizard**"  |
//! | Aux? Adv* Verb    | Verb       | "was slowly **walking**"   |
//! | Prep NP           | Prep       | "**through** the forest"   |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;

use super::verb_morphology::VerbMorphology;

// =============================================================================
// Core Types
// =============================================================================

/// Kind of phrase chunk detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChunkKind {
    /// Noun phrase: "the ancient wizard", "a dark forest"
    NounPhrase,
    /// Verb phrase: "was walking slowly", "quickly ran"
    VerbPhrase,
    /// Prepositional phrase: "through the forest", "in the tower"
    PrepPhrase,
    /// Adjective phrase: "incredibly powerful", "very old"
    AdjPhrase,
    /// Relative clause: "who lived in the tower"
    Clause,
}

impl ChunkKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChunkKind::NounPhrase => "NP",
            ChunkKind::VerbPhrase => "VP",
            ChunkKind::PrepPhrase => "PP",
            ChunkKind::AdjPhrase => "ADJP",
            ChunkKind::Clause => "CLAUSE",
        }
    }
}

/// Text range (byte offsets)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "TextRange: start must be <= end");
        TextRange { start, end }
    }

    pub fn from_range(range: Range<usize>) -> Self {
        TextRange::new(range.start, range.end)
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Extract the text slice from a source string
    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    /// Check if this range contains another range
    pub fn contains(&self, other: &TextRange) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// Check if this range overlaps with another
    pub fn overlaps(&self, other: &TextRange) -> bool {
        self.start < other.end && other.start < self.end
    }
}

impl From<Range<usize>> for TextRange {
    fn from(range: Range<usize>) -> Self {
        TextRange::from_range(range)
    }
}

/// A detected phrase chunk
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    /// Type of phrase
    pub kind: ChunkKind,
    /// Full span of the chunk (byte offsets)
    pub range: TextRange,
    /// The head word's span (the main word of the phrase)
    pub head: TextRange,
    /// Spans of modifier words (adjectives, adverbs, determiners, etc.)
    pub modifiers: Vec<TextRange>,
}

impl Chunk {
    pub fn new(kind: ChunkKind, range: TextRange, head: TextRange) -> Self {
        Chunk {
            kind,
            range,
            head,
            modifiers: Vec::new(),
        }
    }

    pub fn with_modifiers(mut self, modifiers: Vec<TextRange>) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Get the head word text from source
    pub fn head_text<'a>(&self, source: &'a str) -> &'a str {
        self.head.slice(source)
    }

    /// Get the full chunk text from source
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        self.range.slice(source)
    }
}

/// Part of speech tag (simplified for chunking)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum POS {
    // Nominal
    Noun,
    Pronoun,
    ProperNoun,
    
    // Verbal
    Verb,
    Auxiliary,
    Modal,
    
    // Modifiers
    Adjective,
    Adverb,
    
    // Function words
    Determiner,
    Preposition,
    Conjunction,
    
    // Relative/WH
    RelativePronoun,
    
    // Punctuation & other
    Punctuation,
    Other,
}

impl POS {
    pub fn is_nominal(&self) -> bool {
        matches!(self, POS::Noun | POS::Pronoun | POS::ProperNoun)
    }

    pub fn is_verbal(&self) -> bool {
        matches!(self, POS::Verb | POS::Auxiliary | POS::Modal)
    }

    pub fn is_modifier(&self) -> bool {
        matches!(self, POS::Adjective | POS::Adverb)
    }
}

/// A tagged token
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub text: String,
    pub pos: POS,
    pub range: TextRange,
}

impl Token {
    pub fn new(text: impl Into<String>, pos: POS, range: TextRange) -> Self {
        Token {
            text: text.into(),
            pos,
            range,
        }
    }
}

/// Result of chunking a text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResult {
    pub chunks: Vec<Chunk>,
    pub tokens: Vec<Token>,
    /// Time taken in microseconds
    pub timing_us: u64,
}

/// Statistics from chunking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkStats {
    pub noun_phrases: usize,
    pub verb_phrases: usize,
    pub prep_phrases: usize,
    pub adj_phrases: usize,
    pub clauses: usize,
    pub token_count: usize,
}

impl ChunkStats {
    pub fn from_chunks(chunks: &[Chunk], token_count: usize) -> Self {
        let mut stats = ChunkStats {
            noun_phrases: 0,
            verb_phrases: 0,
            prep_phrases: 0,
            adj_phrases: 0,
            clauses: 0,
            token_count,
        };
        for chunk in chunks {
            match chunk.kind {
                ChunkKind::NounPhrase => stats.noun_phrases += 1,
                ChunkKind::VerbPhrase => stats.verb_phrases += 1,
                ChunkKind::PrepPhrase => stats.prep_phrases += 1,
                ChunkKind::AdjPhrase => stats.adj_phrases += 1,
                ChunkKind::Clause => stats.clauses += 1,
            }
        }
        stats
    }
}

// =============================================================================
// Chunker Implementation
// =============================================================================

/// Rule-based phrase chunker
/// 
/// Uses simple POS patterns to identify NP, VP, PP without neural models.
pub struct Chunker {
    /// Precompiled word -> POS lookup (common words)
    lexicon: HashMap<String, POS>,
    /// Verb morphology for expanded verb recognition
    verb_morphology: VerbMorphology,
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunker {
    /// Create a new Chunker with default English lexicon
    pub fn new() -> Self {
        let mut chunker = Chunker {
            lexicon: HashMap::new(),
            verb_morphology: VerbMorphology::new(),
        };
        chunker.load_default_lexicon();
        chunker
    }

    /// Chunk text (native Rust API)
    pub fn chunk(&self, text: &str) -> ChunkResult {
        let start = std::time::Instant::now();
        
        // Step 1: Tokenize
        let token_ranges = self.tokenize(text);
        
        // Step 2: Tag POS
        let tokens = self.tag_tokens(&token_ranges, text);
        
        // Step 3: Chunk
        let chunks = self.find_chunks(&tokens, text);
        
        ChunkResult {
            chunks,
            tokens,
            timing_us: start.elapsed().as_micros() as u64,
        }
    }

    /// Get chunking statistics
    pub fn get_stats(&self, text: &str) -> ChunkStats {
        let result = self.chunk(text);
        ChunkStats::from_chunks(&result.chunks, result.tokens.len())
    }

    /// Tokenize text into word boundaries
    fn tokenize(&self, text: &str) -> Vec<TextRange> {
        let mut tokens = Vec::new();
        let mut start: Option<usize> = None;
        
        for (i, c) in text.char_indices() {
            if c.is_alphanumeric() || c == '\'' || c == '-' {
                if start.is_none() {
                    start = Some(i);
                }
            } else {
                if let Some(s) = start.take() {
                    tokens.push(TextRange::new(s, i));
                }
                if c.is_ascii_punctuation() {
                    tokens.push(TextRange::new(i, i + c.len_utf8()));
                }
            }
        }
        if let Some(s) = start {
            tokens.push(TextRange::new(s, text.len()));
        }
        
        tokens
    }

    /// Tag tokens with POS
    fn tag_tokens(&self, token_ranges: &[TextRange], text: &str) -> Vec<Token> {
        token_ranges
            .iter()
            .map(|range| {
                let word = range.slice(text);
                let pos = self.lookup_pos(word);
                Token::new(word, pos, *range)
            })
            .collect()
    }

    /// Lookup POS for a word
    fn lookup_pos(&self, word: &str) -> POS {
        let lower = word.to_lowercase();
        
        // Check lexicon first
        if let Some(pos) = self.lexicon.get(&lower) {
            return *pos;
        }
        
        // Check verb morphology for main verbs not in lexicon
        if self.verb_morphology.is_verb(&lower) {
            return POS::Verb;
        }
        
        // Heuristic rules for unknown words
        self.infer_pos(word)
    }

    /// Infer POS for unknown words using heuristics
    fn infer_pos(&self, word: &str) -> POS {
        let lower = word.to_lowercase();
        
        // Punctuation
        if word.len() == 1 && word.chars().next().map(|c| c.is_ascii_punctuation()).unwrap_or(false) {
            return POS::Punctuation;
        }
        
        // Proper noun heuristic: starts with uppercase
        if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            return POS::ProperNoun;
        }
        
        // Common suffixes
        if lower.ends_with("ly") {
            return POS::Adverb;
        }
        if lower.ends_with("ing") || lower.ends_with("ed") || lower.ends_with("en") {
            return POS::Verb;
        }
        if lower.ends_with("ness") || lower.ends_with("tion") || lower.ends_with("ment") 
            || lower.ends_with("ity") || lower.ends_with("er") || lower.ends_with("or") {
            return POS::Noun;
        }
        if lower.ends_with("ful") || lower.ends_with("less") || lower.ends_with("ous") 
            || lower.ends_with("ive") || lower.ends_with("able") || lower.ends_with("ible") {
            return POS::Adjective;
        }
        
        // Default: noun
        POS::Noun
    }

    /// Find chunks using tagged tokens
    fn find_chunks(&self, tokens: &[Token], _text: &str) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut i = 0;
        
        while i < tokens.len() {
            // Skip punctuation
            if tokens[i].pos == POS::Punctuation {
                i += 1;
                continue;
            }
            
            // Try each pattern in priority order
            if let Some((chunk, consumed)) = self.try_prep_phrase(tokens, i) {
                chunks.push(chunk);
                i += consumed;
            } else if let Some((chunk, consumed)) = self.try_verb_phrase(tokens, i) {
                chunks.push(chunk);
                i += consumed;
            } else if let Some((chunk, consumed)) = self.try_noun_phrase(tokens, i) {
                chunks.push(chunk);
                i += consumed;
            } else if let Some((chunk, consumed)) = self.try_adj_phrase(tokens, i) {
                chunks.push(chunk);
                i += consumed;
            } else if let Some((chunk, consumed)) = self.try_clause(tokens, i) {
                chunks.push(chunk);
                i += consumed;
            } else {
                i += 1;
            }
        }
        
        chunks
    }

    /// Try to match a noun phrase: Det? Adj* Noun+
    fn try_noun_phrase(&self, tokens: &[Token], start: usize) -> Option<(Chunk, usize)> {
        let mut i = start;
        let mut modifiers = Vec::new();
        
        // Optional determiner
        if i < tokens.len() && tokens[i].pos == POS::Determiner {
            modifiers.push(tokens[i].range);
            i += 1;
        }
        
        // Zero or more adjectives
        while i < tokens.len() && tokens[i].pos == POS::Adjective {
            modifiers.push(tokens[i].range);
            i += 1;
        }
        
        // One or more nouns (compound nouns)
        let noun_start = i;
        while i < tokens.len() && tokens[i].pos.is_nominal() {
            i += 1;
        }
        
        if i > noun_start {
            let head = tokens[i - 1].range;
            let range = TextRange::new(tokens[start].range.start, tokens[i - 1].range.end);
            let chunk = Chunk::new(ChunkKind::NounPhrase, range, head)
                .with_modifiers(modifiers);
            Some((chunk, i - start))
        } else {
            None
        }
    }

    /// Try to match a verb phrase: Aux? Adv* Verb Adv*
    fn try_verb_phrase(&self, tokens: &[Token], start: usize) -> Option<(Chunk, usize)> {
        let mut i = start;
        let mut modifiers = Vec::new();
        let mut head_idx = None;
        
        // Optional auxiliary
        if i < tokens.len() && (tokens[i].pos == POS::Auxiliary || tokens[i].pos == POS::Modal) {
            modifiers.push(tokens[i].range);
            i += 1;
        }
        
        // Pre-verb adverbs
        while i < tokens.len() && tokens[i].pos == POS::Adverb {
            modifiers.push(tokens[i].range);
            i += 1;
        }
        
        // Main verb (required)
        if i < tokens.len() && tokens[i].pos == POS::Verb {
            head_idx = Some(i);
            i += 1;
        } else {
            return None;
        }
        
        // Post-verb adverbs
        while i < tokens.len() && tokens[i].pos == POS::Adverb {
            modifiers.push(tokens[i].range);
            i += 1;
        }
        
        let head_idx = head_idx?;
        let head = tokens[head_idx].range;
        let range = TextRange::new(tokens[start].range.start, tokens[i - 1].range.end);
        let chunk = Chunk::new(ChunkKind::VerbPhrase, range, head)
            .with_modifiers(modifiers);
        Some((chunk, i - start))
    }

    /// Try to match a prepositional phrase: Prep NP
    fn try_prep_phrase(&self, tokens: &[Token], start: usize) -> Option<(Chunk, usize)> {
        if start >= tokens.len() || tokens[start].pos != POS::Preposition {
            return None;
        }
        
        let prep = &tokens[start];
        let np_start = start + 1;
        
        // Must have a following NP
        let (np, np_consumed) = self.try_noun_phrase(tokens, np_start)?;
        
        let range = TextRange::new(prep.range.start, np.range.end);
        let mut modifiers = vec![np.head];
        modifiers.extend(np.modifiers);
        
        let chunk = Chunk::new(ChunkKind::PrepPhrase, range, prep.range)
            .with_modifiers(modifiers);
        Some((chunk, 1 + np_consumed))
    }

    /// Try to match an adjective phrase: Adv* Adj
    fn try_adj_phrase(&self, tokens: &[Token], start: usize) -> Option<(Chunk, usize)> {
        let mut i = start;
        let mut modifiers = Vec::new();
        
        // Intensifier adverbs
        while i < tokens.len() && tokens[i].pos == POS::Adverb {
            modifiers.push(tokens[i].range);
            i += 1;
        }
        
        // Must have at least one adjective
        if i >= tokens.len() || tokens[i].pos != POS::Adjective {
            return None;
        }
        
        let head = tokens[i].range;
        i += 1;
        
        // Only make ADJP if there are intensifiers
        if modifiers.is_empty() {
            return None;
        }
        
        let range = TextRange::new(tokens[start].range.start, tokens[i - 1].range.end);
        let chunk = Chunk::new(ChunkKind::AdjPhrase, range, head)
            .with_modifiers(modifiers);
        Some((chunk, i - start))
    }

    /// Try to match a relative clause: RelPronoun VP | RelPronoun VP NP
    fn try_clause(&self, tokens: &[Token], start: usize) -> Option<(Chunk, usize)> {
        if start >= tokens.len() || tokens[start].pos != POS::RelativePronoun {
            return None;
        }
        
        let rel = &tokens[start];
        let mut i = start + 1;
        
        // Must have a VP
        let (vp, vp_consumed) = self.try_verb_phrase(tokens, i)?;
        i += vp_consumed;
        
        // Optional NP after VP
        let mut end = vp.range.end;
        if let Some((np, np_consumed)) = self.try_noun_phrase(tokens, i) {
            end = np.range.end;
            i += np_consumed;
        }
        
        let range = TextRange::new(rel.range.start, end);
        let chunk = Chunk::new(ChunkKind::Clause, range, vp.head)
            .with_modifiers(vec![rel.range]);
        Some((chunk, i - start))
    }

    /// Load the default English lexicon
    fn load_default_lexicon(&mut self) {
        // Determiners
        for word in ["the", "a", "an", "this", "that", "these", "those", "my", "your", 
                     "his", "her", "its", "our", "their", "some", "any", "no", "every",
                     "each", "all", "both", "few", "many", "much", "most", "other"] {
            self.lexicon.insert(word.to_string(), POS::Determiner);
        }
        
        // Prepositions
        for word in ["in", "on", "at", "to", "for", "with", "by", "from", "of", "about",
                     "into", "through", "during", "before", "after", "above", "below",
                     "between", "under", "over", "against", "among", "around", "behind",
                     "beside", "beyond", "near", "toward", "towards", "upon", "within",
                     "without", "across", "along", "inside", "outside", "throughout"] {
            self.lexicon.insert(word.to_string(), POS::Preposition);
        }
        
        // Auxiliaries
        for word in ["is", "are", "was", "were", "be", "been", "being", "am",
                     "have", "has", "had", "having", "do", "does", "did", "doing"] {
            self.lexicon.insert(word.to_string(), POS::Auxiliary);
        }
        
        // Modals
        for word in ["can", "could", "will", "would", "shall", "should", "may", "might", "must"] {
            self.lexicon.insert(word.to_string(), POS::Modal);
        }
        
        // Conjunctions
        for word in ["and", "or", "but", "nor", "yet", "so", "for", "because", "although",
                     "while", "if", "unless", "until", "since", "when", "where", "whether"] {
            self.lexicon.insert(word.to_string(), POS::Conjunction);
        }
        
        // Pronouns
        for word in ["i", "you", "he", "she", "it", "we", "they", "me", "him", "her", "us", "them",
                     "myself", "yourself", "himself", "herself", "itself", "ourselves", "themselves"] {
            self.lexicon.insert(word.to_string(), POS::Pronoun);
        }
        
        // Relative pronouns
        for word in ["who", "whom", "whose", "which", "that"] {
            self.lexicon.insert(word.to_string(), POS::RelativePronoun);
        }
        
        // Common adjectives
        for word in ["old", "new", "good", "bad", "great", "small", "large", "big", "little",
                     "young", "long", "short", "high", "low", "early", "late", "first", "last",
                     "ancient", "dark", "bright", "powerful", "mighty", "wise", "evil", "grey",
                     "black", "white", "red", "blue", "green", "golden", "silver"] {
            self.lexicon.insert(word.to_string(), POS::Adjective);
        }
        
        // Common adverbs
        for word in ["very", "quite", "rather", "really", "too", "so", "just", "only",
                     "now", "then", "here", "there", "always", "never", "often", "sometimes",
                     "slowly", "quickly", "suddenly", "finally", "already", "still", "even"] {
            self.lexicon.insert(word.to_string(), POS::Adverb);
        }
        
        // Common verbs
        for word in ["go", "went", "gone", "going", "come", "came", "coming",
                     "say", "said", "saying", "see", "saw", "seen", "seeing",
                     "know", "knew", "known", "knowing", "take", "took", "taken", "taking",
                     "get", "got", "getting", "make", "made", "making",
                     "walk", "walked", "walking", "run", "ran", "running",
                     "live", "lived", "living", "speak", "spoke", "spoken", "speaking",
                     "fight", "fought", "fighting", "kill", "killed", "killing",
                     "love", "loved", "loving", "hate", "hated", "hating",
                     "rule", "ruled", "ruling", "serve", "served", "serving"] {
            self.lexicon.insert(word.to_string(), POS::Verb);
        }
        
        // Common nouns (narrative-specific)
        for word in ["wizard", "king", "queen", "knight", "dragon", "sword", "castle",
                     "forest", "tower", "ring", "magic", "battle", "kingdom", "throne",
                     "warrior", "mage", "elf", "dwarf", "orc", "goblin", "troll",
                     "man", "woman", "child", "hero", "villain", "stranger", "lord", "lady"] {
            self.lexicon.insert(word.to_string(), POS::Noun);
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn find_chunk_by_kind(chunks: &[Chunk], kind: ChunkKind) -> Option<&Chunk> {
        chunks.iter().find(|c| c.kind == kind)
    }

    #[test]
    fn test_text_range_basic() {
        let range = TextRange::new(0, 5);
        assert_eq!(range.len(), 5);
        assert!(!range.is_empty());
    }

    #[test]
    fn test_text_range_slice() {
        let text = "hello world";
        let range = TextRange::new(0, 5);
        assert_eq!(range.slice(text), "hello");
    }

    #[test]
    fn test_np_simple_noun() {
        let chunker = Chunker::new();
        let result = chunker.chunk("wizard");
        let np = find_chunk_by_kind(&result.chunks, ChunkKind::NounPhrase);
        assert!(np.is_some(), "Should find NP for 'wizard'");
    }

    #[test]
    fn test_np_det_noun() {
        let chunker = Chunker::new();
        let text = "the wizard";
        let result = chunker.chunk(text);
        let np = find_chunk_by_kind(&result.chunks, ChunkKind::NounPhrase);
        assert!(np.is_some(), "Should find NP for 'the wizard'");
        let np = np.unwrap();
        assert_eq!(np.head_text(text), "wizard");
        assert_eq!(np.text(text), "the wizard");
    }

    #[test]
    fn test_np_det_adj_noun() {
        let chunker = Chunker::new();
        let text = "the ancient wizard";
        let result = chunker.chunk(text);
        let np = find_chunk_by_kind(&result.chunks, ChunkKind::NounPhrase);
        assert!(np.is_some());
        let np = np.unwrap();
        assert_eq!(np.head_text(text), "wizard");
        assert_eq!(np.text(text), "the ancient wizard");
    }

    #[test]
    fn test_vp_simple_verb() {
        let chunker = Chunker::new();
        let text = "walked";
        let result = chunker.chunk(text);
        let vp = find_chunk_by_kind(&result.chunks, ChunkKind::VerbPhrase);
        assert!(vp.is_some(), "Should find VP for 'walked'");
    }

    #[test]
    fn test_vp_aux_verb() {
        let chunker = Chunker::new();
        let text = "was walking";
        let result = chunker.chunk(text);
        let vp = find_chunk_by_kind(&result.chunks, ChunkKind::VerbPhrase);
        assert!(vp.is_some(), "Should find VP for 'was walking'");
        let vp = vp.unwrap();
        assert_eq!(vp.head_text(text), "walking");
    }

    #[test]
    fn test_pp_prep_np() {
        let chunker = Chunker::new();
        let text = "in the forest";
        let result = chunker.chunk(text);
        let pp = find_chunk_by_kind(&result.chunks, ChunkKind::PrepPhrase);
        assert!(pp.is_some(), "Should find PP for 'in the forest'");
        let pp = pp.unwrap();
        assert_eq!(pp.head_text(text), "in");
        assert_eq!(pp.text(text), "in the forest");
    }

    #[test]
    fn test_full_sentence() {
        let chunker = Chunker::new();
        let text = "the old wizard walked through the dark forest";
        let result = chunker.chunk(text);
        
        let nps: Vec<_> = result.chunks.iter().filter(|c| c.kind == ChunkKind::NounPhrase).collect();
        let vps: Vec<_> = result.chunks.iter().filter(|c| c.kind == ChunkKind::VerbPhrase).collect();
        let pps: Vec<_> = result.chunks.iter().filter(|c| c.kind == ChunkKind::PrepPhrase).collect();
        
        assert!(!nps.is_empty(), "Should find noun phrases");
        assert!(!vps.is_empty(), "Should find verb phrases");
        assert!(!pps.is_empty(), "Should find prep phrases");
    }
}
