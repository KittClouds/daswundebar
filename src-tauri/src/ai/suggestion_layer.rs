//! Suggestion Layer - In-memory buffer for NER suggestions pending user review
//!
//! # Architecture
//!
//! ```text
//! NerService ──► SuggestionLayer ──► CozoDB (inferred_entities)
//!                     │                        │
//!                     │  In-Memory Buffer      │  Persisted
//!                     │  (pending review)      │  (accepted/rejected)
//!                     └────────────────────────┘
//! ```
//!
//! Suggestions flow:
//! 1. NER inference produces InferredEntity with confidence
//! 2. High confidence (≥0.90) → auto-promote to entities relation
//! 3. Medium confidence (0.60-0.89) → store in suggestion layer
//! 4. User accepts → promote to entities, mark as accepted
//! 5. User rejects → mark as rejected (kept for training data)

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use cozo::DbInstance;
use serde::{Serialize, Deserialize};

use crate::ai::ner_service::InferredEntity;
use crate::ai::ner_postprocess::NerPostProcessor;

// =============================================================================
// Label → EntityKind Mapping
// =============================================================================

/// Maps NER labels to application EntityKind values.
///
/// Returns the canonical EntityKind string, or returns the label uppercased
/// if no mapping exists (allows custom labels to pass through).
pub fn map_label_to_kind(label: &str) -> String {
    match label.to_lowercase().as_str() {
        // Person/Character
        "person" | "per" | "character" => "CHARACTER".to_string(),
        // Location/Place
        "location" | "loc" | "place" | "gpe" => "LOCATION".to_string(),
        // Organization → NPC (groups/factions)
        "organization" | "org" => "NPC".to_string(),
        // Items/Objects
        "item" | "object" | "artifact" | "product" => "ITEM".to_string(),
        // Events
        "event" | "evt" => "EVENT".to_string(),
        // Concepts
        "concept" | "idea" | "misc" => "CONCEPT".to_string(),
        // Creatures
        "creature" | "monster" | "animal" => "CREATURE".to_string(),
        // Time expressions (from TemporalCortex)
        "date" | "time" | "duration" => "EVENT".to_string(),
        // Default: uppercase the label (allows custom labels)
        _ => label.to_uppercase(),
    }
}

// =============================================================================
// Types
// =============================================================================

/// A suggestion pending user review
#[taurpc::ipc_type]
pub struct Suggestion {
    /// Unique ID
    pub id: String,
    /// World/workspace ID
    pub world_id: String,
    /// Source note ID
    pub source_note_id: String,
    /// The detected text span
    pub text: String,
    /// Original label from input
    pub label: String,
    /// Inferred entity type (e.g., "CHARACTER", "LOCATION")
    pub entity_type: String,
    /// Start byte offset in source text
    pub byte_start: usize,
    /// End byte offset in source text
    pub byte_end: usize,
    /// Model confidence (0.0 - 1.0)
    pub confidence: f32,
    /// When the inference was made
    pub inferred_at: f64,
}

/// Status of a suggestion after review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
}

impl SuggestionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SuggestionStatus::Pending => "pending",
            SuggestionStatus::Accepted => "accepted",
            SuggestionStatus::Rejected => "rejected",
        }
    }
}

/// Result of accepting a suggestion - includes full entity info for TS sync
#[taurpc::ipc_type]
pub struct AcceptResult {
    pub suggestion_id: String,
    pub promoted_entity_id: String,
    pub label: String,
    pub kind: String,
    pub source_note_id: String,
    pub is_new: bool,
}

// =============================================================================
// SuggestionLayer
// =============================================================================

/// In-memory buffer for pending NER suggestions
///
/// Thread-safe via RwLock. Suggestions are keyed by (note_id, suggestion_id).
pub struct SuggestionLayer {
    /// Pending suggestions: note_id -> Vec<Suggestion>
    pending: RwLock<HashMap<String, Vec<Suggestion>>>,
}

impl Default for SuggestionLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl SuggestionLayer {
    /// Create a new suggestion layer
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
        }
    }

    /// Add a suggestion from NER inference
    /// Returns Some(id) if suggestion was added, None if filtered out by post-processor
    pub fn add_suggestion(
        &self,
        world_id: &str,
        source_note_id: &str,
        entity: &InferredEntity,
    ) -> Option<String> {
        // Post-process: check if this should be filtered
        let postprocessor = NerPostProcessor::new();
        
        // Early exit if blocked or junk
        if postprocessor.is_blocked(&entity.text) {
            log::debug!(
                "[SuggestionLayer] Blocked: '{}' (blocklist)",
                entity.text
            );
            return None;
        }
        
        if postprocessor.is_junk(&entity.text) {
            log::debug!(
                "[SuggestionLayer] Blocked: '{}' (junk heuristics)",
                entity.text
            );
            return None;
        }
        
        // Clean the text
        let cleaned_text = postprocessor.clean_span(&entity.text);
        
        // Check if cleaned text is too short or blocked
        if cleaned_text.len() < 2 || postprocessor.is_blocked(&cleaned_text) {
            log::debug!(
                "[SuggestionLayer] Blocked after cleanup: '{}' -> '{}'",
                entity.text,
                cleaned_text
            );
            return None;
        }
        
        let id = format!("infer_{}", uuid::Uuid::new_v4());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let suggestion = Suggestion {
            id: id.clone(),
            world_id: world_id.to_string(),
            source_note_id: source_note_id.to_string(),
            text: cleaned_text, // Use cleaned text
            label: entity.label.clone(),
            entity_type: map_label_to_kind(&entity.label),
            byte_start: entity.start,
            byte_end: entity.end,
            confidence: entity.confidence,
            inferred_at: now,
        };

        let mut pending = self.pending.write();
        pending
            .entry(source_note_id.to_string())
            .or_default()
            .push(suggestion);

        log::debug!(
            "[SuggestionLayer] Added suggestion {} for note {} ('{}')",
            id,
            source_note_id,
            entity.text
        );

        Some(id)
    }

    /// Get all pending suggestions for a note
    pub fn get_suggestions(&self, note_id: &str) -> Vec<Suggestion> {
        let pending = self.pending.read();
        pending.get(note_id).cloned().unwrap_or_default()
    }

    /// Get all pending suggestions across all notes
    pub fn get_all_suggestions(&self) -> Vec<Suggestion> {
        let pending = self.pending.read();
        pending.values().flatten().cloned().collect()
    }

    /// Get a specific suggestion by ID
    pub fn get_suggestion(&self, suggestion_id: &str) -> Option<Suggestion> {
        let pending = self.pending.read();
        for suggestions in pending.values() {
            if let Some(s) = suggestions.iter().find(|s| s.id == suggestion_id) {
                return Some(s.clone());
            }
        }
        None
    }

    /// Accept a suggestion and promote to entity
    ///
    /// This:
    /// 1. Removes from pending buffer
    /// 2. Creates entity in entities relation
    /// 3. Updates inferred_entities with accepted status
    pub fn accept(
        &self,
        db: &DbInstance,
        suggestion_id: &str,
    ) -> Result<AcceptResult, String> {
        // Find and remove the suggestion
        let suggestion = {
            let mut pending = self.pending.write();
            let mut found = None;
            let mut found_note_id = None;

            for (note_id, suggestions) in pending.iter_mut() {
                if let Some(pos) = suggestions.iter().position(|s| s.id == suggestion_id) {
                    found = Some(suggestions.remove(pos));
                    found_note_id = Some(note_id.clone());
                    break;
                }
            }

            // Clean up empty entries
            if let Some(note_id) = found_note_id {
                if pending.get(&note_id).map(|v| v.is_empty()).unwrap_or(false) {
                    pending.remove(&note_id);
                }
            }

            found.ok_or_else(|| format!("Suggestion not found: {}", suggestion_id))?
        };

        // Create entity in entities relation
        let entity_id = format!("ent_{}", uuid::Uuid::new_v4());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let create_entity_query = format!(
            r#"
            ?[id, world_id, label, entity_kind, entity_subtype, note_id, folder_id, is_active, aliases, attributes, created_at, updated_at] <- [[
                $id, $world_id, $label, $entity_kind, '', $note_id, '', true, '[]', '{{}}', $now, $now
            ]]
            :put entities {{ id, world_id, label, entity_kind, entity_subtype, note_id, folder_id, is_active, aliases, attributes, created_at, updated_at }}
            "#
        );

        let mut params = std::collections::BTreeMap::new();
        params.insert("id".to_string(), cozo::DataValue::from(entity_id.clone()));
        params.insert("world_id".to_string(), cozo::DataValue::from(suggestion.world_id.clone()));
        params.insert("label".to_string(), cozo::DataValue::from(suggestion.text.clone()));
        params.insert("entity_kind".to_string(), cozo::DataValue::from(suggestion.entity_type.clone()));
        params.insert("note_id".to_string(), cozo::DataValue::from(suggestion.source_note_id.clone()));
        params.insert("now".to_string(), cozo::DataValue::from(now));

        db.run_script(&create_entity_query, params, cozo::ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to create entity: {}", e))?;

        // Record in inferred_entities as accepted
        self.persist_inference(db, &suggestion, SuggestionStatus::Accepted, &entity_id)?;

        log::info!(
            "[SuggestionLayer] Accepted suggestion {} -> entity {}",
            suggestion_id,
            entity_id
        );

        Ok(AcceptResult {
            suggestion_id: suggestion_id.to_string(),
            promoted_entity_id: entity_id,
            label: suggestion.text.clone(),
            kind: suggestion.entity_type.clone(),
            source_note_id: suggestion.source_note_id.clone(),
            is_new: true, // Always new when accepting - merging would be different path
        })
    }

    /// Reject a suggestion
    ///
    /// Removes from pending buffer and records rejection in inferred_entities
    pub fn reject(&self, db: &DbInstance, suggestion_id: &str) -> Result<(), String> {
        // Find and remove the suggestion
        let suggestion = {
            let mut pending = self.pending.write();
            let mut found = None;
            let mut found_note_id = None;

            for (note_id, suggestions) in pending.iter_mut() {
                if let Some(pos) = suggestions.iter().position(|s| s.id == suggestion_id) {
                    found = Some(suggestions.remove(pos));
                    found_note_id = Some(note_id.clone());
                    break;
                }
            }

            // Clean up empty entries
            if let Some(note_id) = found_note_id {
                if pending.get(&note_id).map(|v| v.is_empty()).unwrap_or(false) {
                    pending.remove(&note_id);
                }
            }

            found.ok_or_else(|| format!("Suggestion not found: {}", suggestion_id))?
        };

        // Record in inferred_entities as rejected
        self.persist_inference(db, &suggestion, SuggestionStatus::Rejected, "")?;

        log::info!("[SuggestionLayer] Rejected suggestion {}", suggestion_id);

        Ok(())
    }

    /// Clear all pending suggestions for a note
    pub fn clear_note(&self, note_id: &str) {
        let mut pending = self.pending.write();
        pending.remove(note_id);
    }

    /// Clear all pending suggestions
    pub fn clear_all(&self) {
        let mut pending = self.pending.write();
        pending.clear();
    }

    /// Get count of pending suggestions
    pub fn pending_count(&self) -> usize {
        let pending = self.pending.read();
        pending.values().map(|v| v.len()).sum()
    }

    /// Persist an inference result to CozoDB
    fn persist_inference(
        &self,
        db: &DbInstance,
        suggestion: &Suggestion,
        status: SuggestionStatus,
        promoted_entity_id: &str,
    ) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        let query = r#"
            ?[id, world_id, source_note_id, text, label, entity_type, byte_start, byte_end, confidence, status, promoted_entity_id, inferred_at, reviewed_at] <- [[
                $id, $world_id, $source_note_id, $text, $label, $entity_type, $byte_start, $byte_end, $confidence, $status, $promoted_entity_id, $inferred_at, $reviewed_at
            ]]
            :put inferred_entities { id, world_id, source_note_id, text, label, entity_type, byte_start, byte_end, confidence, status, promoted_entity_id, inferred_at, reviewed_at }
        "#;

        let mut params = std::collections::BTreeMap::new();
        params.insert("id".to_string(), cozo::DataValue::from(suggestion.id.clone()));
        params.insert("world_id".to_string(), cozo::DataValue::from(suggestion.world_id.clone()));
        params.insert("source_note_id".to_string(), cozo::DataValue::from(suggestion.source_note_id.clone()));
        params.insert("text".to_string(), cozo::DataValue::from(suggestion.text.clone()));
        params.insert("label".to_string(), cozo::DataValue::from(suggestion.label.clone()));
        params.insert("entity_type".to_string(), cozo::DataValue::from(suggestion.entity_type.clone()));
        params.insert("byte_start".to_string(), cozo::DataValue::from(suggestion.byte_start as i64));
        params.insert("byte_end".to_string(), cozo::DataValue::from(suggestion.byte_end as i64));
        params.insert("confidence".to_string(), cozo::DataValue::from(suggestion.confidence as f64));
        params.insert("status".to_string(), cozo::DataValue::from(status.as_str()));
        params.insert("promoted_entity_id".to_string(), cozo::DataValue::from(promoted_entity_id));
        params.insert("inferred_at".to_string(), cozo::DataValue::from(suggestion.inferred_at));
        params.insert("reviewed_at".to_string(), cozo::DataValue::from(now));

        db.run_script(query, params, cozo::ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to persist inference: {}", e))?;

        Ok(())
    }
}

// =============================================================================
// Static Query Functions
// =============================================================================

/// Get all inferred entities for a note from CozoDB
pub fn get_inferred_entities(
    db: &DbInstance,
    note_id: &str,
) -> Result<Vec<Suggestion>, String> {
    let query = r#"
        ?[id, world_id, source_note_id, text, label, entity_type, byte_start, byte_end, confidence, status, inferred_at] :=
            *inferred_entities[id, world_id, source_note_id, text, label, entity_type, byte_start, byte_end, confidence, status, _, inferred_at, _],
            source_note_id = $note_id
    "#;

    let mut params = std::collections::BTreeMap::new();
    params.insert("note_id".to_string(), cozo::DataValue::from(note_id));

    let result = db
        .run_script(query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get inferred entities: {}", e))?;

    let mut suggestions = Vec::new();
    for row in result.rows {
        if row.len() >= 11 {
            suggestions.push(Suggestion {
                id: row[0].get_str().unwrap_or_default().to_string(),
                world_id: row[1].get_str().unwrap_or_default().to_string(),
                source_note_id: row[2].get_str().unwrap_or_default().to_string(),
                text: row[3].get_str().unwrap_or_default().to_string(),
                label: row[4].get_str().unwrap_or_default().to_string(),
                entity_type: row[5].get_str().unwrap_or_default().to_string(),
                byte_start: row[6].get_int().unwrap_or(0) as usize,
                byte_end: row[7].get_int().unwrap_or(0) as usize,
                confidence: row[8].get_float().unwrap_or(0.0) as f32,
                inferred_at: row[10].get_float().unwrap_or(0.0),
            });
        }
    }

    Ok(suggestions)
}

/// Get pending inferred entities (not yet accepted/rejected)
pub fn get_pending_inferred_entities(
    db: &DbInstance,
    note_id: &str,
) -> Result<Vec<Suggestion>, String> {
    let query = r#"
        ?[id, world_id, source_note_id, text, label, entity_type, byte_start, byte_end, confidence, inferred_at] :=
            *inferred_entities[id, world_id, source_note_id, text, label, entity_type, byte_start, byte_end, confidence, status, _, inferred_at, _],
            source_note_id = $note_id,
            status = 'pending'
    "#;

    let mut params = std::collections::BTreeMap::new();
    params.insert("note_id".to_string(), cozo::DataValue::from(note_id));

    let result = db
        .run_script(query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get pending inferred entities: {}", e))?;

    let mut suggestions = Vec::new();
    for row in result.rows {
        if row.len() >= 10 {
            suggestions.push(Suggestion {
                id: row[0].get_str().unwrap_or_default().to_string(),
                world_id: row[1].get_str().unwrap_or_default().to_string(),
                source_note_id: row[2].get_str().unwrap_or_default().to_string(),
                text: row[3].get_str().unwrap_or_default().to_string(),
                label: row[4].get_str().unwrap_or_default().to_string(),
                entity_type: row[5].get_str().unwrap_or_default().to_string(),
                byte_start: row[6].get_int().unwrap_or(0) as usize,
                byte_end: row[7].get_int().unwrap_or(0) as usize,
                confidence: row[8].get_float().unwrap_or(0.0) as f32,
                inferred_at: row[9].get_float().unwrap_or(0.0),
            });
        }
    }

    Ok(suggestions)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entity() -> InferredEntity {
        InferredEntity {
            text: "Zorian".to_string(),
            label: "CHARACTER".to_string(),
            start: 10,
            end: 16,
            confidence: 0.85,
        }
    }

    fn create_test_db() -> DbInstance {
        let db = DbInstance::new("mem", "", Default::default()).unwrap();
        // Initialize schema
        crate::graph::schema::init_schema(&db).unwrap();
        db
    }

    #[test]
    fn test_add_suggestion() {
        let layer = SuggestionLayer::new();
        let entity = create_test_entity();
        
        let id = layer.add_suggestion("world1", "note1", &entity).unwrap();
        
        assert!(!id.is_empty());
        assert!(id.starts_with("infer_"));
        
        let suggestions = layer.get_suggestions("note1");
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].text, "Zorian");
    }

    #[test]
    fn test_get_suggestions_empty() {
        let layer = SuggestionLayer::new();
        let suggestions = layer.get_suggestions("nonexistent");
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_multiple_suggestions_per_note() {
        let layer = SuggestionLayer::new();
        
        let e1 = InferredEntity {
            text: "Zorian".to_string(),
            label: "CHARACTER".to_string(),
            start: 0,
            end: 6,
            confidence: 0.9,
        };
        let e2 = InferredEntity {
            text: "Cyoria".to_string(),
            label: "LOCATION".to_string(),
            start: 20,
            end: 26,
            confidence: 0.8,
        };
        
        layer.add_suggestion("world1", "note1", &e1);
        layer.add_suggestion("world1", "note1", &e2);
        
        let suggestions = layer.get_suggestions("note1");
        assert_eq!(suggestions.len(), 2);
    }

    #[test]
    fn test_pending_count() {
        let layer = SuggestionLayer::new();
        let entity = create_test_entity();
        
        assert_eq!(layer.pending_count(), 0);
        
        layer.add_suggestion("world1", "note1", &entity);
        assert_eq!(layer.pending_count(), 1);
        
        layer.add_suggestion("world1", "note2", &entity);
        assert_eq!(layer.pending_count(), 2);
    }

    #[test]
    fn test_clear_note() {
        let layer = SuggestionLayer::new();
        let entity = create_test_entity();
        
        layer.add_suggestion("world1", "note1", &entity);
        layer.add_suggestion("world1", "note2", &entity);
        
        layer.clear_note("note1");
        
        assert!(layer.get_suggestions("note1").is_empty());
        assert_eq!(layer.get_suggestions("note2").len(), 1);
    }

    #[test]
    fn test_clear_all() {
        let layer = SuggestionLayer::new();
        let entity = create_test_entity();
        
        layer.add_suggestion("world1", "note1", &entity);
        layer.add_suggestion("world1", "note2", &entity);
        
        layer.clear_all();
        
        assert_eq!(layer.pending_count(), 0);
    }

    #[test]
    fn test_accept_creates_entity() {
        let db = create_test_db();
        let layer = SuggestionLayer::new();
        let entity = create_test_entity();
        
        let id = layer.add_suggestion("world1", "note1", &entity).unwrap();
        
        let result = layer.accept(&db, &id);
        assert!(result.is_ok());
        
        let accept_result = result.unwrap();
        assert_eq!(accept_result.suggestion_id, id);
        assert!(accept_result.promoted_entity_id.starts_with("ent_"));
        
        // Suggestion should be removed from pending
        assert!(layer.get_suggestions("note1").is_empty());
    }

    #[test]
    fn test_reject_removes_from_pending() {
        let db = create_test_db();
        let layer = SuggestionLayer::new();
        let entity = create_test_entity();
        
        let id = layer.add_suggestion("world1", "note1", &entity).unwrap();
        
        let result = layer.reject(&db, &id);
        assert!(result.is_ok());
        
        // Suggestion should be removed from pending
        assert!(layer.get_suggestions("note1").is_empty());
    }

    #[test]
    fn test_accept_nonexistent_fails() {
        let db = create_test_db();
        let layer = SuggestionLayer::new();
        
        let result = layer.accept(&db, "nonexistent_id");
        assert!(result.is_err());
    }
}
