use rowan::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    // Top Level
    Document = 0,
    Section,
    Paragraph,
    Sentence,

    // Semantics
    EntitySpan, // Wrapping an entity
    ConceptSpan,
    RelationSpan,
    
    // NEW: Link types (from UnifiedScanner)
    WikilinkSpan,     // [[Target]] or [[Target|Display]]
    BacklinkSpan,     // <<Target>>
    TripleSpan,       // [Entity] (relation) [Entity]
    TemporalSpan,     // "on March 15th", "during the war"


    // NEW: Phrase-level nodes (from Chunker)
    NounPhrase,      // "the ancient wizard"
    VerbPhrase,      // "was slowly walking"
    PrepPhrase,      // "through the forest"
    AdjPhrase,       // "incredibly powerful"
    
    // NEW: Clause-level nodes
    MainClause,           // Primary clause in sentence
    SubordinateClause,    // "who lived in the tower", "because he was tired"

    // Leaves
    Word,
    Whitespace,
    Punctuation,
    
    // Catch-all
    Error,
    
    // Technical
    __Last,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealityLanguage;

impl Language for RealityLanguage {
    type Kind = SyntaxKind;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        unsafe { std::mem::transmute(raw.0) }
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}
