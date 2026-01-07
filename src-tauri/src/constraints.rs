//! Constraints - Ref validation and uniqueness enforcement (Native Tauri)
//!
//! # Functionality
//! - Validate refs (required fields, kind-specific rules)
//! - Enforce uniqueness (deduplicate by composite key)
//! - Predicate validation (allowed predicates per entity kind)
//! - Scope filtering

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// PREDICATE RULES
// =============================================================================

/// Get allowed predicates for an entity kind
fn get_predicate_rules(entity_kind: &str) -> Option<&'static [&'static str]> {
    match entity_kind {
        "CHARACTER" | "PERSON" => Some(&[
            "KNOWS",
            "LOVES",
            "HATES",
            "RELATED_TO",
            "WORKS_WITH",
            "MENTORS",
            "RIVALS",
        ]),
        "LOCATION" => Some(&["CONTAINS", "NEAR", "CONNECTED_TO", "PART_OF"]),
        "ORGANIZATION" => Some(&["OWNS", "EMPLOYS", "ALLIED_WITH", "RIVALS", "PART_OF"]),
        "EVENT" => Some(&["INVOLVES", "CAUSES", "PRECEDES", "FOLLOWS"]),
        "ITEM" => Some(&["BELONGS_TO", "CREATED_BY", "USED_BY", "PART_OF"]),
        "CONCEPT" => Some(&["RELATED_TO", "IMPLIES", "CONTRADICTS", "PART_OF"]),
        _ => None,
    }
}

// =============================================================================
// TYPES
// =============================================================================

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Ref structure for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefInput {
    pub id: String,
    pub kind: String,
    pub target: String,
    pub source_note_id: String,
    pub predicate: Option<String>,
    pub scope_type: String,
    pub scope_path: Option<String>,
    pub positions: Vec<RefPosition>,
    pub payload: Option<RefPayload>,
    pub attributes: Option<serde_json::Value>,
    pub created_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefPosition {
    pub note_id: String,
    pub offset: usize,
    pub length: usize,
    pub context_before: Option<String>,
    pub context_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefPayload {
    pub entity_kind: Option<String>,
    pub subject_kind: Option<String>,
    pub subject_label: Option<String>,
    pub object_kind: Option<String>,
    pub object_label: Option<String>,
    pub aliases: Option<Vec<String>>,
}

// =============================================================================
// CONSTRAINT ENGINE
// =============================================================================

/// Constraint validation engine
#[derive(Debug, Clone, Default)]
pub struct ConstraintEngine;

impl ConstraintEngine {
    pub fn new() -> Self {
        Self
    }

    /// Validate a single ref
    pub fn validate(&self, ref_input: &RefInput) -> ConstraintResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic validation
        if ref_input.id.is_empty() {
            errors.push("Ref must have an id".to_string());
        }
        if ref_input.kind.is_empty() {
            errors.push("Ref must have a kind".to_string());
        }
        if ref_input.target.is_empty() {
            errors.push("Ref must have a target".to_string());
        }
        if ref_input.source_note_id.is_empty() {
            errors.push("Ref must have a source_note_id".to_string());
        }

        // Kind-specific validation
        match ref_input.kind.as_str() {
            "entity" => self.validate_entity(ref_input, &mut errors, &mut warnings),
            "triple" => self.validate_triple(ref_input, &mut errors, &mut warnings),
            _ => {}
        }

        ConstraintResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    fn validate_entity(
        &self,
        ref_input: &RefInput,
        _errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        // Check for very short labels
        if ref_input.target.len() < 2 {
            warnings.push(format!(
                "Entity label \"{}\" is very short",
                ref_input.target
            ));
        }

        // Check for suspicious characters
        if ref_input
            .target
            .chars()
            .any(|c| matches!(c, '<' | '>' | '{' | '}' | '[' | ']' | '|'))
        {
            warnings.push(format!(
                "Entity label \"{}\" contains suspicious characters",
                ref_input.target
            ));
        }
    }

    fn validate_triple(
        &self,
        ref_input: &RefInput,
        errors: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) {
        if let Some(payload) = &ref_input.payload {
            if payload.subject_kind.is_none() || payload.subject_label.is_none() {
                errors.push("Triple ref must have subject".to_string());
            }
            if payload.object_kind.is_none() || payload.object_label.is_none() {
                errors.push("Triple ref must have object".to_string());
            }

            // Validate predicate is allowed for subject kind
            if let (Some(subject_kind), Some(predicate)) =
                (&payload.subject_kind, &ref_input.predicate)
            {
                if !self.validate_predicate(subject_kind, predicate) {
                    if let Some(allowed) = get_predicate_rules(subject_kind) {
                        warnings.push(format!(
                            "Predicate \"{}\" is unusual for {}. Expected: {}",
                            predicate,
                            subject_kind,
                            allowed.join(", ")
                        ));
                    }
                }
            }
        }

        if ref_input.predicate.is_none() {
            errors.push("Triple ref must have predicate".to_string());
        }
    }

    /// Validate predicate for entity kind
    pub fn validate_predicate(&self, entity_kind: &str, predicate: &str) -> bool {
        match get_predicate_rules(entity_kind) {
            Some(allowed) => allowed.contains(&predicate),
            None => true, // Unknown kinds allow any predicate
        }
    }

    /// Get allowed predicates for an entity kind
    pub fn get_allowed_predicates(&self, entity_kind: &str) -> Vec<String> {
        get_predicate_rules(entity_kind)
            .map(|v| v.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    /// Enforce uniqueness across refs
    pub fn enforce_uniqueness(&self, refs: Vec<RefInput>) -> Vec<RefInput> {
        let mut seen: HashMap<String, RefInput> = HashMap::new();

        for ref_input in refs {
            let key = self.get_unique_key(&ref_input);

            if let Some(existing) = seen.get_mut(&key) {
                // Merge positions
                existing.positions.extend(ref_input.positions);
                // Update last seen
                existing.last_seen_at = existing.last_seen_at.max(ref_input.last_seen_at);
            } else {
                seen.insert(key, ref_input);
            }
        }

        seen.into_values().collect()
    }

    /// Generate a unique key for deduplication
    fn get_unique_key(&self, ref_input: &RefInput) -> String {
        let scope_path = ref_input.scope_path.as_deref().unwrap_or("");

        match ref_input.kind.as_str() {
            "entity" => {
                let entity_kind = ref_input
                    .payload
                    .as_ref()
                    .and_then(|p| p.entity_kind.as_deref())
                    .unwrap_or("UNKNOWN");
                format!(
                    "entity:{}:{}:{}",
                    entity_kind,
                    ref_input.target.to_lowercase(),
                    scope_path
                )
            }
            "wikilink" => format!(
                "wikilink:{}:{}",
                ref_input.target.to_lowercase(),
                scope_path
            ),
            "backlink" => format!(
                "backlink:{}:{}",
                ref_input.target.to_lowercase(),
                scope_path
            ),
            "tag" => format!("tag:{}", ref_input.target.to_lowercase()),
            "mention" => format!("mention:{}", ref_input.target.to_lowercase()),
            "triple" => format!(
                "triple:{}:{}",
                ref_input.target,
                ref_input.predicate.as_deref().unwrap_or("")
            ),
            "temporal" => {
                let offset = ref_input.positions.first().map(|p| p.offset).unwrap_or(0);
                format!(
                    "temporal:{}:{}:{}",
                    ref_input.target, ref_input.source_note_id, offset
                )
            }
            _ => format!(
                "{}:{}:{}",
                ref_input.kind, ref_input.target, ref_input.source_note_id
            ),
        }
    }

    /// Filter refs by scope
    pub fn filter_by_scope(
        &self,
        refs: Vec<RefInput>,
        scope_type: &str,
        scope_path: Option<&str>,
    ) -> Vec<RefInput> {
        refs.into_iter()
            .filter(|r| match scope_type {
                "note" => r.scope_type == "note" && r.scope_path.as_deref() == scope_path,
                "folder" => {
                    if let (Some(ref_path), Some(filter_path)) = (&r.scope_path, scope_path) {
                        ref_path.starts_with(filter_path)
                    } else {
                        true
                    }
                }
                "vault" => true,
                _ => true,
            })
            .collect()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ref(id: &str, kind: &str, target: &str) -> RefInput {
        RefInput {
            id: id.to_string(),
            kind: kind.to_string(),
            target: target.to_string(),
            source_note_id: "note1".to_string(),
            predicate: None,
            scope_type: "note".to_string(),
            scope_path: None,
            positions: vec![RefPosition {
                note_id: "note1".to_string(),
                offset: 0,
                length: 10,
                context_before: None,
                context_after: None,
            }],
            payload: None,
            attributes: None,
            created_at: 1000,
            last_seen_at: 1000,
        }
    }

    #[test]
    fn test_validate_valid_ref() {
        let engine = ConstraintEngine::new();
        let ref_input = make_ref("ref1", "entity", "Jon");

        let result = engine.validate(&ref_input);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_missing_id() {
        let engine = ConstraintEngine::new();
        let mut ref_input = make_ref("", "entity", "Jon");
        ref_input.id = "".to_string();

        let result = engine.validate(&ref_input);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("id")));
    }

    #[test]
    fn test_validate_missing_kind() {
        let engine = ConstraintEngine::new();
        let mut ref_input = make_ref("ref1", "", "Jon");
        ref_input.kind = "".to_string();

        let result = engine.validate(&ref_input);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("kind")));
    }

    #[test]
    fn test_validate_missing_target() {
        let engine = ConstraintEngine::new();
        let mut ref_input = make_ref("ref1", "entity", "");
        ref_input.target = "".to_string();

        let result = engine.validate(&ref_input);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("target")));
    }

    #[test]
    fn test_validate_entity_short_label_warning() {
        let engine = ConstraintEngine::new();
        let ref_input = make_ref("ref1", "entity", "X");

        let result = engine.validate(&ref_input);
        assert!(result.valid); // Warning, not error
        assert!(result.warnings.iter().any(|w| w.contains("short")));
    }

    #[test]
    fn test_validate_entity_suspicious_chars_warning() {
        let engine = ConstraintEngine::new();
        let ref_input = make_ref("ref1", "entity", "Jon<script>");

        let result = engine.validate(&ref_input);
        assert!(result.valid); // Warning, not error
        assert!(result.warnings.iter().any(|w| w.contains("suspicious")));
    }

    #[test]
    fn test_validate_predicate_allowed() {
        let engine = ConstraintEngine::new();
        assert!(engine.validate_predicate("CHARACTER", "KNOWS"));
        assert!(engine.validate_predicate("LOCATION", "CONTAINS"));
    }

    #[test]
    fn test_validate_predicate_not_in_list() {
        let engine = ConstraintEngine::new();
        // OWNS is not in CHARACTER's allowed list
        assert!(!engine.validate_predicate("CHARACTER", "OWNS"));
    }

    #[test]
    fn test_validate_predicate_unknown_kind() {
        let engine = ConstraintEngine::new();
        // Unknown kinds allow any predicate
        assert!(engine.validate_predicate("UNKNOWN_KIND", "ANYTHING"));
    }

    #[test]
    fn test_get_allowed_predicates() {
        let engine = ConstraintEngine::new();
        let allowed = engine.get_allowed_predicates("CHARACTER");
        assert!(allowed.contains(&"KNOWS".to_string()));
        assert!(allowed.contains(&"LOVES".to_string()));
        assert!(!allowed.contains(&"OWNS".to_string()));
    }

    #[test]
    fn test_enforce_uniqueness_deduplicates() {
        let engine = ConstraintEngine::new();

        let ref1 = make_ref("ref1", "entity", "Jon");
        let mut ref2 = make_ref("ref2", "entity", "Jon");
        ref2.positions = vec![RefPosition {
            note_id: "note2".to_string(),
            offset: 50,
            length: 3,
            context_before: None,
            context_after: None,
        }];

        let refs = vec![ref1, ref2];
        let unique = engine.enforce_uniqueness(refs);

        // Should merge into one ref
        assert_eq!(unique.len(), 1);
        // Should have positions from both
        assert_eq!(unique[0].positions.len(), 2);
    }

    #[test]
    fn test_enforce_uniqueness_different_kinds() {
        let engine = ConstraintEngine::new();

        let ref1 = make_ref("ref1", "entity", "Jon");
        let ref2 = make_ref("ref2", "wikilink", "Jon");

        let refs = vec![ref1, ref2];
        let unique = engine.enforce_uniqueness(refs);

        // Different kinds = different refs
        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn test_filter_by_scope_note() {
        let engine = ConstraintEngine::new();

        let mut ref1 = make_ref("ref1", "entity", "Jon");
        ref1.scope_path = Some("note1".to_string());

        let mut ref2 = make_ref("ref2", "entity", "Arya");
        ref2.scope_path = Some("note2".to_string());

        let refs = vec![ref1, ref2];
        let filtered = engine.filter_by_scope(refs, "note", Some("note1"));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].target, "Jon");
    }

    #[test]
    fn test_filter_by_scope_folder() {
        let engine = ConstraintEngine::new();

        let mut ref1 = make_ref("ref1", "entity", "Jon");
        ref1.scope_path = Some("chapter1/section1".to_string());

        let mut ref2 = make_ref("ref2", "entity", "Arya");
        ref2.scope_path = Some("chapter2/section1".to_string());

        let refs = vec![ref1, ref2];
        let filtered = engine.filter_by_scope(refs, "folder", Some("chapter1"));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].target, "Jon");
    }
}
