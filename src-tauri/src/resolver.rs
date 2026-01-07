//! Coreference Resolver - Pronoun and alias resolution (Native Tauri)
//!
//! Resolves pronouns ("he", "she") and aliases ("the wizard") to canonical entities.
//! Uses a Narrative Context to track recency, gender, and scene presence.
//!
//! # Heuristics
//! 1. Pronoun Gender Match: "he" -> Male, "she" -> Female
//! 2. Recency: Prefer most recently mentioned compatible entity
//! 3. Salience: (Future) Main characters have higher weight
//! 4. Scene Context: (Future) Only resolve to characters present in scene

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use super::chunker::TextRange;

// =============================================================================
// Core Types
// =============================================================================

pub type EntityId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Neutral, // It/Thing
    Plural,  // They/Them
    Unknown,
}

impl Default for Gender {
    fn default() -> Self {
        Gender::Unknown
    }
}

/// A resolved chain of mentions for a single entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct MentionChain {
    pub canonical_id: EntityId,
    /// Locations of all mentions in the text
    pub mentions: Vec<TextRange>,
}

/// Metadata about a known entity (for resolution context)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    pub id: EntityId,
    pub name: String,
    pub gender: Gender,
    pub aliases: Vec<String>,
    pub kind: String, // "CHARACTER", "LOCATION", etc.
}

// =============================================================================
// Narrative Context
// =============================================================================

/// Tracks the state of the narrative for resolution
#[derive(Debug, Clone, Default)]
pub struct NarrativeContext {
    /// Stack of recently mentioned entities (most recent at front)
    history: VecDeque<EntityId>,
    /// Metadata for all known entities
    registry: HashMap<EntityId, EntityMetadata>,
    /// Max history size to track
    max_history: usize,

    // Scene context fields
    #[allow(dead_code)]
    pub scene_id: Option<String>,
    pub active_characters: Vec<EntityId>,
    pub speaker: Option<EntityId>,
    pub in_dialogue: bool,
}

impl NarrativeContext {
    pub fn new() -> Self {
        NarrativeContext {
            history: VecDeque::new(),
            registry: HashMap::new(),
            max_history: 10,
            scene_id: None,
            active_characters: Vec::new(),
            speaker: None,
            in_dialogue: false,
        }
    }

    #[allow(dead_code)]
    pub fn set_scene(&mut self, scene_id: Option<String>) {
        self.scene_id = scene_id;
    }

    #[allow(dead_code)]
    pub fn set_speaker(&mut self, speaker_id: Option<EntityId>) {
        self.speaker = speaker_id;
    }

    #[allow(dead_code)]
    pub fn set_dialogue_state(&mut self, in_dialogue: bool) {
        self.in_dialogue = in_dialogue;
    }

    #[allow(dead_code)]
    pub fn add_active_character(&mut self, entity_id: EntityId) {
        if !self.active_characters.contains(&entity_id) {
            self.active_characters.push(entity_id);
        }
    }

    pub fn register(&mut self, entity: EntityMetadata) {
        self.registry.insert(entity.id.clone(), entity);
    }

    /// Record a mention of an entity, moving it to the front of history
    pub fn push_mention(&mut self, entity_id: &str) {
        // Remove existing occurrence to update freshness
        if let Some(idx) = self.history.iter().position(|id| id == entity_id) {
            self.history.remove(idx);
        }

        self.history.push_front(entity_id.to_string());

        if self.history.len() > self.max_history {
            self.history.pop_back();
        }
    }

    /// Find the most recent entity matching the gender criteria
    pub fn find_most_recent(&self, gender: Gender) -> Option<&EntityId> {
        for id in &self.history {
            if let Some(meta) = self.registry.get(id) {
                if self.genders_compatible(meta.gender, gender) {
                    return Some(id);
                }
            }
        }
        None
    }

    fn genders_compatible(&self, entity_gender: Gender, pronoun_gender: Gender) -> bool {
        match (entity_gender, pronoun_gender) {
            (g1, g2) if g1 == g2 => true,
            (_, Gender::Unknown) => true,
            (Gender::Unknown, _) => true,
            (_, Gender::Plural) => true,
            _ => false,
        }
    }
}

// =============================================================================
// Resolver Implementation
// =============================================================================

/// Resolver for pronouns and aliases
pub struct Resolver {
    context: NarrativeContext,
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver {
    pub fn new() -> Self {
        Resolver {
            context: NarrativeContext::new(),
        }
    }

    /// Resolve a pronoun or alias to an entity ID
    pub fn resolve(&mut self, text: &str) -> Option<String> {
        let is_pronoun = self.is_pronoun(text);

        if is_pronoun {
            let gender = self.infer_pronoun_gender(text);
            self.context.find_most_recent(gender).cloned()
        } else {
            // Check for direct alias match
            let text_lower = text.to_lowercase();
            for meta in self.context.registry.values() {
                if meta.name.to_lowercase() == text_lower
                    || meta.aliases.iter().any(|a| a.to_lowercase() == text_lower)
                {
                    return Some(meta.id.clone());
                }
            }
            None
        }
    }

    /// Updates context with an explicit mention (e.g. found by NER)
    pub fn observe_mention(&mut self, entity_id: &str) {
        self.context.push_mention(entity_id);
    }

    /// Register a known entity for resolution
    pub fn register_entity(
        &mut self,
        id: &str,
        name: &str,
        gender: Gender,
        aliases: Vec<String>,
    ) {
        self.context.register(EntityMetadata {
            id: id.to_string(),
            name: name.to_string(),
            gender,
            aliases,
            kind: "CHARACTER".to_string(),
        });
    }

    fn is_pronoun(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        matches!(
            lower.as_str(),
            "he" | "him"
                | "his"
                | "she"
                | "her"
                | "hers"
                | "it"
                | "its"
                | "they"
                | "them"
                | "their"
        )
    }

    fn infer_pronoun_gender(&self, text: &str) -> Gender {
        match text.to_lowercase().as_str() {
            "he" | "him" | "his" => Gender::Male,
            "she" | "her" | "hers" => Gender::Female,
            "it" | "its" => Gender::Neutral,
            "they" | "them" | "their" => Gender::Plural,
            _ => Gender::Unknown,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_resolver() -> Resolver {
        let mut resolver = Resolver::new();
        resolver.register_entity(
            "e1",
            "Gandalf",
            Gender::Male,
            vec!["Mithrandir".to_string(), "The Wizard".to_string()],
        );
        resolver.register_entity(
            "e2",
            "Galadriel",
            Gender::Female,
            vec!["Lady of Light".to_string()],
        );
        resolver.register_entity(
            "e3",
            "The Ring",
            Gender::Neutral,
            vec!["My Precious".to_string()],
        );
        resolver
    }

    #[test]
    fn test_pronoun_resolution_simple() {
        let mut r = setup_resolver();

        // "Gandalf walked. He stopped."
        r.observe_mention("e1"); // Gandalf mentioned

        let resolved = r.resolve("He");
        assert_eq!(
            resolved.as_deref(),
            Some("e1"),
            "Should resolve 'He' to most recent male (Gandalf)"
        );
    }

    #[test]
    fn test_pronoun_gender_switch() {
        let mut r = setup_resolver();

        r.observe_mention("e1"); // Gandalf
        r.observe_mention("e2"); // Galadriel

        // History: [Galadriel, Gandalf]

        // "She" -> Galadriel
        assert_eq!(r.resolve("She").as_deref(), Some("e2"));

        // "He" -> Gandalf (skips Galadriel due to gender)
        assert_eq!(r.resolve("He").as_deref(), Some("e1"));
    }

    #[test]
    fn test_alias_resolution() {
        let r = setup_resolver();
        let mut r = r;

        // Exact name
        assert_eq!(r.resolve("Gandalf").as_deref(), Some("e1"));

        // Alias
        assert_eq!(r.resolve("Mithrandir").as_deref(), Some("e1"));
        assert_eq!(r.resolve("The Wizard").as_deref(), Some("e1"));
    }

    #[test]
    fn test_neutral_pronoun() {
        let mut r = setup_resolver();
        r.observe_mention("e3"); // The Ring

        assert_eq!(r.resolve("It").as_deref(), Some("e3"));
    }

    #[test]
    fn test_stack_update_order() {
        let mut r = setup_resolver();

        r.observe_mention("e1"); // Gandalf
        r.observe_mention("e1"); // Gandalf again

        assert_eq!(r.resolve("He").as_deref(), Some("e1"));

        // Add another male
        r.register_entity("e4", "Frodo", Gender::Male, vec![]);
        r.observe_mention("e4"); // Frodo

        // History: [Frodo, Gandalf]
        assert_eq!(
            r.resolve("He").as_deref(),
            Some("e4"),
            "Should prefer Frodo (most recent)"
        );
    }
}
