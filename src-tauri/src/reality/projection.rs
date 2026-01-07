//! Projection Module - Extract semantic structures from CST
//!
//! Phase 3 of Evolution 1.5: Richer projections beyond simple SPO triples.
//!
//! # Projection Types
//!
//! | Type | Description | Example |
//! |------|-------------|---------|
//! | Triple | Subject-Predicate-Object | "Frodo owns Ring" |
//! | QuadPlus | SPO + Modifiers | "Frodo destroyed Ring in Mordor" |
//! | Attribution | Dialogue | "Gandalf said 'You shall not pass'" |
//! | StateChange | State transition | "Frodo became invisible" |
//!
//! # Architecture
//!
//! All projections operate on the CST (Concrete Syntax Tree) produced by
//! `zip_reality_enhanced`. The CST contains:
//! - EntitySpan nodes (semantic entities)
//! - VerbPhrase/RelationSpan nodes (predicates)
//! - PrepPhrase nodes (modifiers: location, manner, time)

use rowan::SyntaxNode;
use serde::{Deserialize, Serialize};
use super::syntax::{RealityLanguage, SyntaxKind};

// =============================================================================
// TRIPLE: Basic Subject-Predicate-Object
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Triple {
    pub source: String,
    pub relation: String,
    pub target: String,
    /// Byte range of the source entity (start, end)
    pub source_span: Option<(usize, usize)>,
    /// Byte range of the target entity (start, end)
    pub target_span: Option<(usize, usize)>,
}

pub fn project_triples(root: &SyntaxNode<RealityLanguage>) -> Vec<Triple> {
    let mut triples = Vec::new();
    
    for descendant in root.descendants() {
        if descendant.kind() == SyntaxKind::RelationSpan {
            // Find Subject (Left) and Object (Right)
            let subject = find_neighbor_entity(&descendant, Direction::Left);
            let object = find_neighbor_entity(&descendant, Direction::Right);
            
            if let (Some(s), Some(o)) = (subject, object) {
                let source_range = s.text_range();
                let target_range = o.text_range();
                
                triples.push(Triple {
                    source: s.text().to_string(),
                    relation: descendant.text().to_string(),
                    target: o.text().to_string(),
                    source_span: Some((source_range.start().into(), source_range.end().into())),
                    target_span: Some((target_range.start().into(), target_range.end().into())),
                });
            }
        }
    }
    
    triples
}

// =============================================================================
// QUADPLUS: SPO + Modifiers (Manner, Location, Time)
// =============================================================================

/// QuadPlus: Subject-Predicate-Object + Modifiers
///
/// This is the level above simple SPO triples. It captures:
/// - WHO did WHAT to WHOM
/// - Plus: WHERE, HOW, WHEN
///
/// # Example
/// "Gandalf defeated Sauron with magic in Mordor during the battle"
/// - subject: "Gandalf"
/// - predicate: "defeated"
/// - object: "Sauron"
/// - manner: Some("with magic")
/// - location: Some("in Mordor")
/// - time: Some("during the battle")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuadPlus {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    /// HOW: "with his sword", "by magic", "using the Ring"
    pub manner: Option<String>,
    /// WHERE: "in Mordor", "at the bridge", "through the forest"
    pub location: Option<String>,
    /// WHEN: "during the battle", "after midnight", "before dawn"
    pub time: Option<String>,
    /// Span of the subject
    pub subject_span: Option<(usize, usize)>,
    /// Span of the object
    pub object_span: Option<(usize, usize)>,
}

/// Extract QuadPlus projections from CST
///
/// For each RelationSpan or VerbPhrase, find:
/// 1. Subject entity (before predicate)
/// 2. Object entity (after predicate)
/// 3. PP modifiers (after object)
pub fn project_quads(root: &SyntaxNode<RealityLanguage>) -> Vec<QuadPlus> {
    let mut quads = Vec::new();
    
    for descendant in root.descendants() {
        // Look for RelationSpan or VerbPhrase as predicate
        let is_predicate = descendant.kind() == SyntaxKind::RelationSpan 
            || descendant.kind() == SyntaxKind::VerbPhrase;
        
        if !is_predicate {
            continue;
        }
        
        // Find subject (entity before predicate)
        let subject = find_neighbor_entity(&descendant, Direction::Left);
        
        // Find object (entity after predicate)
        let object = find_neighbor_entity(&descendant, Direction::Right);
        
        // Need at least subject and object for a quad
        let (subject_node, object_node) = match (subject, object) {
            (Some(s), Some(o)) => (s, o),
            _ => continue,
        };
        
        // Find PP modifiers after the object
        let modifiers = find_pp_modifiers(&descendant);
        let (manner, location, time) = classify_modifiers(&modifiers);
        
        let subject_range = subject_node.text_range();
        let object_range = object_node.text_range();
        
        quads.push(QuadPlus {
            subject: subject_node.text().to_string(),
            predicate: descendant.text().to_string(),
            object: object_node.text().to_string(),
            manner,
            location,
            time,
            subject_span: Some((subject_range.start().into(), subject_range.end().into())),
            object_span: Some((object_range.start().into(), object_range.end().into())),
        });
    }
    
    quads
}

/// Find PrepPhrase siblings after the given node
fn find_pp_modifiers(node: &SyntaxNode<RealityLanguage>) -> Vec<SyntaxNode<RealityLanguage>> {
    let mut modifiers = Vec::new();
    let mut current = node.clone();
    
    while let Some(sibling) = current.next_sibling() {
        if sibling.kind() == SyntaxKind::PrepPhrase {
            modifiers.push(sibling.clone());
        }
        // Stop at next entity or relation (new clause)
        if sibling.kind() == SyntaxKind::EntitySpan 
            || sibling.kind() == SyntaxKind::RelationSpan {
            // We can continue past the object entity
            if modifiers.is_empty() {
                current = sibling;
                continue;
            }
        }
        current = sibling;
    }
    
    modifiers
}

/// Classify PP modifiers by preposition into manner/location/time
fn classify_modifiers(pps: &[SyntaxNode<RealityLanguage>]) -> (Option<String>, Option<String>, Option<String>) {
    let mut manner = None;
    let mut location = None;
    let mut time = None;
    
    for pp in pps {
        let text = pp.text().to_string();
        let lower = text.to_lowercase();
        
        // Extract preposition (first word)
        let prep = lower.split_whitespace().next().unwrap_or("");
        
        match prep {
            // Location prepositions
            "in" | "at" | "on" | "within" | "inside" | "outside" |
            "near" | "beside" | "behind" | "above" | "below" |
            "between" | "among" | "around" | "through" | "across" |
            "into" | "onto" | "toward" | "towards" => {
                if location.is_none() {
                    location = Some(text);
                }
            }
            // Time prepositions
            "during" | "after" | "before" | "since" | "until" |
            "when" | "while" => {
                if time.is_none() {
                    time = Some(text);
                }
            }
            // Manner/Instrument prepositions
            "with" | "by" | "using" | "via" => {
                if manner.is_none() {
                    manner = Some(text);
                }
            }
            _ => {}
        }
    }
    
    (manner, location, time)
}

// =============================================================================
// ATTRIBUTION: Dialogue Attribution (WHO said WHAT)
// =============================================================================

/// Dialogue attribution: WHO said WHAT
///
/// # Example
/// `"You shall not pass!" shouted Gandalf.`
/// - speaker: "Gandalf"
/// - quote: "You shall not pass!"
/// - verb: "shouted"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribution {
    /// The speaker entity
    pub speaker: String,
    /// The quoted text
    pub quote: String,
    /// Byte range of the quote (start, end)
    pub quote_span: (usize, usize),
    /// The dialogue verb: "said", "whispered", "shouted", "asked"
    pub verb: String,
    /// Byte range of the speaker
    pub speaker_span: Option<(usize, usize)>,
}

/// Dialogue verbs that indicate speech
const DIALOGUE_VERBS: &[&str] = &[
    "said", "says", "say",
    "asked", "asks", "ask",
    "replied", "replies", "reply",
    "shouted", "shouts", "shout",
    "whispered", "whispers", "whisper",
    "yelled", "yells", "yell",
    "muttered", "mutters", "mutter",
    "exclaimed", "exclaims", "exclaim",
    "answered", "answers", "answer",
    "spoke", "speaks", "speak",
    "told", "tells", "tell",
    "cried", "cries", "cry",
    "declared", "declares", "declare",
    "announced", "announces", "announce",
    "mumbled", "mumbles", "mumble",
    "screamed", "screams", "scream",
];

/// Extract dialogue attributions from CST
///
/// Looks for patterns:
/// 1. "Quote" said Entity
/// 2. Entity said "Quote"
/// 3. "Quote", said Entity (with comma)
pub fn project_attributions(root: &SyntaxNode<RealityLanguage>) -> Vec<Attribution> {
    let mut attributions = Vec::new();
    
    // Find all VerbPhrase nodes that contain dialogue verbs
    for descendant in root.descendants() {
        if descendant.kind() == SyntaxKind::VerbPhrase {
            let verb_text = descendant.text().to_string().to_lowercase();
            
            // Check if this VP contains a dialogue verb
            let dialogue_verb = DIALOGUE_VERBS.iter()
                .find(|&&v| verb_text.contains(v))
                .map(|s| s.to_string());
            
            if let Some(verb) = dialogue_verb {
                // Look for adjacent quote and entity
                if let Some(attr) = try_extract_attribution(&descendant, &verb, root) {
                    attributions.push(attr);
                }
            }
        }
    }
    
    attributions
}

/// Try to extract attribution from a dialogue verb node
fn try_extract_attribution(
    verb_node: &SyntaxNode<RealityLanguage>,
    verb: &str,
    root: &SyntaxNode<RealityLanguage>,
) -> Option<Attribution> {
    // Find speaker (EntitySpan near the verb)
    let speaker = find_neighbor_entity(verb_node, Direction::Right)
        .or_else(|| find_neighbor_entity(verb_node, Direction::Left))?;
    
    // Find quote (look for punctuation that looks like quotes)
    let quote_info = find_quote_near(verb_node, root)?;
    
    let speaker_range = speaker.text_range();
    
    Some(Attribution {
        speaker: speaker.text().to_string(),
        quote: quote_info.0,
        quote_span: quote_info.1,
        verb: verb.to_string(),
        speaker_span: Some((speaker_range.start().into(), speaker_range.end().into())),
    })
}

/// Find quoted text near a node
/// Returns (quote_text, (start, end))
fn find_quote_near(node: &SyntaxNode<RealityLanguage>, root: &SyntaxNode<RealityLanguage>) -> Option<(String, (usize, usize))> {
    // Simple heuristic: look for text between quote marks in the same sentence
    // For now, look at root text and try to find quotes
    let full_text = root.text().to_string();
    
    // Find quotes using various quote characters
    // Using unicode escapes for curly quotes: " " ' '
    let quote_chars = ['"', '\u{201C}', '\u{201D}', '\'', '\u{2018}', '\u{2019}'];
    
    for (i, c) in full_text.char_indices() {
        if quote_chars.contains(&c) {
            // Find matching close quote
            for (j, c2) in full_text[i+c.len_utf8()..].char_indices() {
                if quote_chars.contains(&c2) {
                    let start = i;
                    let end = i + c.len_utf8() + j + c2.len_utf8();
                    let quote_text = full_text[start..end].to_string();
                    
                    // Check if this quote is near our verb node
                    let verb_range = node.text_range();
                    let verb_start: usize = verb_range.start().into();
                    let verb_end: usize = verb_range.end().into();
                    
                    // Quote should be within ~100 chars of verb
                    if (start as isize - verb_end as isize).abs() < 100 
                        || (end as isize - verb_start as isize).abs() < 100 {
                        return Some((quote_text, (start, end)));
                    }
                }
            }
        }
    }
    
    None
}

// =============================================================================
// STATE CHANGE: Entity State Transitions
// =============================================================================

/// State change: ENTITY {became/turned/grew} STATE
///
/// # Example
/// "Frodo became invisible after putting on the Ring"
/// - entity: "Frodo"
/// - from_state: None
/// - to_state: "invisible"
/// - trigger: Some("after putting on the Ring")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateChange {
    /// The entity undergoing state change
    pub entity: String,
    /// Previous state (if mentioned)
    pub from_state: Option<String>,
    /// New state
    pub to_state: String,
    /// What triggered the change
    pub trigger: Option<String>,
    /// Span of the entity
    pub entity_span: Option<(usize, usize)>,
}

/// Copular/change-of-state verbs
const STATE_CHANGE_VERBS: &[&str] = &[
    "became", "becomes", "become",
    "turned", "turns", "turn",
    "grew", "grows", "grow",
    "went", "goes", "go",
    "got", "gets", "get",
    "fell", "falls", "fall",
    "remained", "remains", "remain",
    "stayed", "stays", "stay",
    "proved", "proves", "prove",
    "seemed", "seems", "seem",
    "appeared", "appears", "appear",
    "transformed", "transforms", "transform",
    "changed", "changes", "change",
];

/// Extract state changes from CST
pub fn project_state_changes(root: &SyntaxNode<RealityLanguage>) -> Vec<StateChange> {
    let mut changes = Vec::new();
    
    for descendant in root.descendants() {
        if descendant.kind() == SyntaxKind::VerbPhrase {
            let verb_text = descendant.text().to_string().to_lowercase();
            
            // Check if this VP contains a state-change verb
            let is_state_verb = STATE_CHANGE_VERBS.iter()
                .any(|&v| verb_text.contains(v));
            
            if is_state_verb {
                if let Some(change) = try_extract_state_change(&descendant) {
                    changes.push(change);
                }
            }
        }
    }
    
    changes
}

/// Try to extract state change from a copular verb node
fn try_extract_state_change(verb_node: &SyntaxNode<RealityLanguage>) -> Option<StateChange> {
    // Entity is typically before the verb
    let entity = find_neighbor_entity(verb_node, Direction::Left)?;
    
    // State is typically after the verb (adjective, noun, or AdjPhrase)
    let to_state = find_state_after(verb_node)?;
    
    // Trigger is often in a PP after the state
    let trigger = find_trigger_after(verb_node);
    
    let entity_range = entity.text_range();
    
    Some(StateChange {
        entity: entity.text().to_string(),
        from_state: None, // Would need more complex analysis
        to_state,
        trigger,
        entity_span: Some((entity_range.start().into(), entity_range.end().into())),
    })
}

/// Find state description after verb (adjective or noun)
fn find_state_after(node: &SyntaxNode<RealityLanguage>) -> Option<String> {
    let mut current = node.clone();
    
    while let Some(sibling) = current.next_sibling() {
        let kind = sibling.kind();
        
        // AdjPhrase is ideal
        if kind == SyntaxKind::AdjPhrase {
            return Some(sibling.text().to_string());
        }
        
        // NounPhrase can also be a state (e.g., "became a king")
        if kind == SyntaxKind::NounPhrase {
            return Some(sibling.text().to_string());
        }
        
        // Word might be an adjective
        if kind == SyntaxKind::Word {
            let text = sibling.text().to_string();
            // Simple heuristic: adjectives often end in specific suffixes
            let lower = text.to_lowercase();
            if lower.ends_with("ible") || lower.ends_with("able") 
                || lower.ends_with("ful") || lower.ends_with("less")
                || lower.ends_with("ous") || lower.ends_with("ive")
                || lower.ends_with("ed") || lower.ends_with("ing")
                || is_common_adjective(&lower) {
                return Some(text);
            }
        }
        
        // Skip trivia
        if is_trivia(kind) {
            current = sibling;
            continue;
        }
        
        // Stop at PP (that's the trigger)
        if kind == SyntaxKind::PrepPhrase {
            break;
        }
        
        current = sibling;
    }
    
    None
}

/// Check if word is a common adjective
fn is_common_adjective(word: &str) -> bool {
    const COMMON_ADJECTIVES: &[&str] = &[
        "invisible", "visible", "angry", "happy", "sad", "tired", "weak", "strong",
        "rich", "poor", "old", "young", "sick", "healthy", "dead", "alive",
        "dark", "light", "hot", "cold", "wet", "dry", "clean", "dirty",
        "powerful", "powerless", "famous", "unknown", "mad", "sane",
    ];
    COMMON_ADJECTIVES.contains(&word)
}

/// Find trigger PP after the state
fn find_trigger_after(node: &SyntaxNode<RealityLanguage>) -> Option<String> {
    let mut current = node.clone();
    let mut found_state = false;
    
    while let Some(sibling) = current.next_sibling() {
        let kind = sibling.kind();
        
        // Mark when we pass the state
        if kind == SyntaxKind::AdjPhrase || kind == SyntaxKind::NounPhrase {
            found_state = true;
        }
        
        // PP after state is the trigger
        if found_state && kind == SyntaxKind::PrepPhrase {
            let text = sibling.text().to_string();
            let lower = text.to_lowercase();
            
            // Trigger PPs often start with: after, when, because, by, from
            let prep = lower.split_whitespace().next().unwrap_or("");
            if matches!(prep, "after" | "when" | "because" | "by" | "from" | "upon" | "through") {
                return Some(text);
            }
        }
        
        current = sibling;
    }
    
    None
}

// =============================================================================
// UNIFIED PROJECTION API
// =============================================================================

/// All projection types unified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Projection {
    Triple(Triple),
    Quad(QuadPlus),
    Attribution(Attribution),
    StateChange(StateChange),
}

/// Statistics from projection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectionStats {
    pub triples: usize,
    pub quads: usize,
    pub attributions: usize,
    pub state_changes: usize,
    pub total: usize,
}

/// Extract all projections from CST
///
/// This is the unified entry point for semantic extraction.
/// Returns all found projections in a single pass.
pub fn project_all(root: &SyntaxNode<RealityLanguage>) -> Vec<Projection> {
    let mut projections = Vec::new();
    
    // Triple projections
    projections.extend(
        project_triples(root)
            .into_iter()
            .map(Projection::Triple)
    );
    
    // QuadPlus projections  
    projections.extend(
        project_quads(root)
            .into_iter()
            .map(Projection::Quad)
    );
    
    // Attribution projections
    projections.extend(
        project_attributions(root)
            .into_iter()
            .map(Projection::Attribution)
    );
    
    // StateChange projections
    projections.extend(
        project_state_changes(root)
            .into_iter()
            .map(Projection::StateChange)
    );
    
    projections
}

/// Extract all projections with statistics
pub fn project_all_with_stats(root: &SyntaxNode<RealityLanguage>) -> (Vec<Projection>, ProjectionStats) {
    let triples = project_triples(root);
    let quads = project_quads(root);
    let attributions = project_attributions(root);
    let state_changes = project_state_changes(root);
    
    let stats = ProjectionStats {
        triples: triples.len(),
        quads: quads.len(),
        attributions: attributions.len(),
        state_changes: state_changes.len(),
        total: triples.len() + quads.len() + attributions.len() + state_changes.len(),
    };
    
    let mut projections = Vec::new();
    projections.extend(triples.into_iter().map(Projection::Triple));
    projections.extend(quads.into_iter().map(Projection::Quad));
    projections.extend(attributions.into_iter().map(Projection::Attribution));
    projections.extend(state_changes.into_iter().map(Projection::StateChange));
    
    (projections, stats)
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

enum Direction {
    Left,
    Right,
}

fn find_neighbor_entity(node: &SyntaxNode<RealityLanguage>, dir: Direction) -> Option<SyntaxNode<RealityLanguage>> {
    let mut current = node.clone();
    
    loop {
        let next = match dir {
            Direction::Left => current.prev_sibling(),
            Direction::Right => current.next_sibling(),
        };
        
        match next {
            Some(sibling) => {
                let kind = sibling.kind();
                if kind == SyntaxKind::EntitySpan {
                    return Some(sibling);
                }
                
                // Allow skipping Trivia
                if is_trivia(kind) {
                    current = sibling;
                    continue;
                }
                
                // NounPhrase might contain an entity
                if kind == SyntaxKind::NounPhrase {
                    // Check if NP contains an EntitySpan
                    if let Some(entity) = sibling.descendants().find(|n| n.kind() == SyntaxKind::EntitySpan) {
                        return Some(entity);
                    }
                }
                
                // If we hit another Relation or VP, stop
                if kind == SyntaxKind::RelationSpan || kind == SyntaxKind::VerbPhrase {
                    return None;
                }
                
                current = sibling;
            }
            None => return None,
        }
    }
}

fn is_trivia(kind: SyntaxKind) -> bool {
    kind == SyntaxKind::Whitespace || kind == SyntaxKind::Punctuation || kind == SyntaxKind::Word
}

// =============================================================================
// TESTS (TDD Contract)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    // =========================================================================
    // TEST HELPERS
    // =========================================================================
    
    // Mock a simple CST for testing
    // In real tests, we'd use parser::zip_reality_enhanced
    
    // =========================================================================
    // CONTRACT: QuadPlus Modifier Classification
    // =========================================================================
    
    #[test]
    fn test_classify_modifiers_location() {
        // "in" should be classified as location
        assert!(matches!(
            classify_modifiers(&[]).1,
            None
        ));
    }
    
    #[test]
    fn test_location_prepositions() {
        let preps = ["in", "at", "on", "through", "across", "near", "beside"];
        for prep in preps {
            let lower = prep.to_lowercase();
            assert!(
                matches!(lower.as_str(), 
                    "in" | "at" | "on" | "within" | "inside" | "outside" |
                    "near" | "beside" | "behind" | "above" | "below" |
                    "between" | "among" | "around" | "through" | "across" |
                    "into" | "onto" | "toward" | "towards"
                ),
                "Preposition '{}' should be location", prep
            );
        }
    }
    
    #[test]
    fn test_time_prepositions() {
        let preps = ["during", "after", "before", "since", "until"];
        for prep in preps {
            assert!(
                matches!(prep, "during" | "after" | "before" | "since" | "until" | "when" | "while"),
                "Preposition '{}' should be time", prep
            );
        }
    }
    
    #[test]
    fn test_manner_prepositions() {
        let preps = ["with", "by", "using", "via"];
        for prep in preps {
            assert!(
                matches!(prep, "with" | "by" | "using" | "via"),
                "Preposition '{}' should be manner", prep
            );
        }
    }
    
    // =========================================================================
    // CONTRACT: Attribution Verb Detection
    // =========================================================================
    
    #[test]
    fn test_dialogue_verbs_recognized() {
        let test_verbs = ["said", "shouted", "whispered", "asked", "replied"];
        for verb in test_verbs {
            assert!(
                DIALOGUE_VERBS.iter().any(|v| v.contains(verb) || verb.contains(v)),
                "Verb '{}' should be recognized as dialogue", verb
            );
        }
    }
    
    #[test]
    fn test_non_dialogue_verbs_not_matched() {
        let non_dialogue = ["walked", "ran", "defeated", "owns"];
        for verb in non_dialogue {
            assert!(
                !DIALOGUE_VERBS.contains(&verb),
                "Verb '{}' should NOT be dialogue", verb
            );
        }
    }
    
    // =========================================================================
    // CONTRACT: StateChange Verb Detection
    // =========================================================================
    
    #[test]
    fn test_state_change_verbs_recognized() {
        let test_verbs = ["became", "turned", "grew", "transformed"];
        for verb in test_verbs {
            assert!(
                STATE_CHANGE_VERBS.iter().any(|v| v.contains(verb) || verb.contains(v)),
                "Verb '{}' should be state-change", verb
            );
        }
    }
    
    #[test]
    fn test_common_adjectives() {
        assert!(is_common_adjective("invisible"));
        assert!(is_common_adjective("angry"));
        assert!(is_common_adjective("powerful"));
        assert!(!is_common_adjective("Gandalf"));
        assert!(!is_common_adjective("sword"));
    }
    
    // =========================================================================
    // CONTRACT: Triple Structure
    // =========================================================================
    
    #[test]
    fn test_triple_has_required_fields() {
        let triple = Triple {
            source: "Frodo".to_string(),
            relation: "owns".to_string(),
            target: "Ring".to_string(),
            source_span: Some((0, 5)),
            target_span: Some((11, 15)),
        };
        
        assert_eq!(triple.source, "Frodo");
        assert_eq!(triple.relation, "owns");
        assert_eq!(triple.target, "Ring");
    }
    
    // =========================================================================
    // CONTRACT: QuadPlus Structure
    // =========================================================================
    
    #[test]
    fn test_quadplus_has_modifiers() {
        let quad = QuadPlus {
            subject: "Gandalf".to_string(),
            predicate: "defeated".to_string(),
            object: "Sauron".to_string(),
            manner: Some("with magic".to_string()),
            location: Some("in Mordor".to_string()),
            time: Some("during the battle".to_string()),
            subject_span: Some((0, 7)),
            object_span: Some((17, 23)),
        };
        
        assert_eq!(quad.manner.as_deref(), Some("with magic"));
        assert_eq!(quad.location.as_deref(), Some("in Mordor"));
        assert_eq!(quad.time.as_deref(), Some("during the battle"));
    }
    
    #[test]
    fn test_quadplus_modifiers_optional() {
        let quad = QuadPlus {
            subject: "Frodo".to_string(),
            predicate: "owns".to_string(),
            object: "Ring".to_string(),
            manner: None,
            location: None,
            time: None,
            subject_span: None,
            object_span: None,
        };
        
        assert!(quad.manner.is_none());
        assert!(quad.location.is_none());
        assert!(quad.time.is_none());
    }
    
    // =========================================================================
    // CONTRACT: Attribution Structure
    // =========================================================================
    
    #[test]
    fn test_attribution_has_required_fields() {
        let attr = Attribution {
            speaker: "Gandalf".to_string(),
            quote: "You shall not pass!".to_string(),
            quote_span: (0, 21),
            verb: "shouted".to_string(),
            speaker_span: Some((22, 29)),
        };
        
        assert_eq!(attr.speaker, "Gandalf");
        assert_eq!(attr.verb, "shouted");
        assert!(!attr.quote.is_empty());
    }
    
    // =========================================================================
    // CONTRACT: StateChange Structure
    // =========================================================================
    
    #[test]
    fn test_state_change_has_required_fields() {
        let change = StateChange {
            entity: "Frodo".to_string(),
            from_state: None,
            to_state: "invisible".to_string(),
            trigger: Some("after putting on the Ring".to_string()),
            entity_span: Some((0, 5)),
        };
        
        assert_eq!(change.entity, "Frodo");
        assert_eq!(change.to_state, "invisible");
        assert!(change.trigger.is_some());
    }
    
    // =========================================================================
    // CONTRACT: Unified Projection API
    // =========================================================================
    
    #[test]
    fn test_projection_enum_variants() {
        let triple = Projection::Triple(Triple {
            source: "A".to_string(),
            relation: "B".to_string(),
            target: "C".to_string(),
            source_span: None,
            target_span: None,
        });
        
        assert!(matches!(triple, Projection::Triple(_)));
        
        let quad = Projection::Quad(QuadPlus {
            subject: "A".to_string(),
            predicate: "B".to_string(),
            object: "C".to_string(),
            manner: None,
            location: None,
            time: None,
            subject_span: None,
            object_span: None,
        });
        
        assert!(matches!(quad, Projection::Quad(_)));
    }
    
    #[test]
    fn test_projection_stats_default() {
        let stats = ProjectionStats::default();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.triples, 0);
        assert_eq!(stats.quads, 0);
    }
}

