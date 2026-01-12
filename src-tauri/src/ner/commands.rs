//! FST-NER Commands - Tauri IPC for hot-path NER
//!
//! Provides fast entity recognition for keystroke-level highlighting.

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::pipeline::{Pipeline, PipelineResult, EntityMatch, MatchSource};
use super::gazetteer::MatchPriority;

// =============================================================================
// Global Pipeline Instance
// =============================================================================

/// Global FST-NER pipeline (reusable, fast)
static FST_PIPELINE: Lazy<RwLock<Pipeline>> = Lazy::new(|| {
    log::info!("[FST-NER] Initializing pipeline...");
    RwLock::new(Pipeline::new())
});

// =============================================================================
// Types for IPC
// =============================================================================

/// Entity definition for hydration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDef {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub aliases: Vec<String>,
}

/// Scan result for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FstScanResult {
    pub matches: Vec<FstMatch>,
    pub timing_us: u64,
    pub token_count: usize,
}

/// Single match for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FstMatch {
    pub text: String,
    pub kind: String,
    pub confidence: f64,
    pub byte_start: usize,
    pub byte_end: usize,
    pub source: String,
    pub entity_id: Option<String>,
    pub canonical_label: Option<String>,
}

impl From<EntityMatch> for FstMatch {
    fn from(m: EntityMatch) -> Self {
        FstMatch {
            text: m.text,
            kind: m.kind,
            confidence: m.confidence,
            byte_start: m.byte_start,
            byte_end: m.byte_end,
            source: match m.source {
                MatchSource::Gazetteer => "gazetteer".to_string(),
                MatchSource::Rule => "rule".to_string(),
                MatchSource::Orthographic => "orthographic".to_string(),
            },
            entity_id: m.entity_id,
            canonical_label: m.canonical_label,
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Scan text for entities (hot path - designed for keystroke-level speed)
pub fn fst_scan(text: &str) -> FstScanResult {
    let pipeline = FST_PIPELINE.read();
    let result = pipeline.process(text);

    FstScanResult {
        matches: result.matches.into_iter().map(FstMatch::from).collect(),
        timing_us: result.timing_us,
        token_count: result.token_count,
    }
}

/// Hydrate the pipeline with known entities
pub fn fst_hydrate(entities: Vec<EntityDef>) -> Result<usize, String> {
    let mut pipeline = FST_PIPELINE.write();
    let gaz = pipeline.gazetteer_mut();

    let count = entities.len();
    for entity in entities {
        gaz.add_entity(&entity.id, &entity.label, &entity.kind, &entity.aliases);
    }

    pipeline.build()?;
    
    log::info!("[FST-NER] Hydrated with {} entities", count);
    Ok(count)
}

/// Clear the gazetteer (for re-hydration)
pub fn fst_clear() {
    let mut pipeline = FST_PIPELINE.write();
    pipeline.gazetteer_mut().clear();
    log::info!("[FST-NER] Cleared gazetteer");
}

/// Add a single entity to the gazetteer
pub fn fst_add_entity(id: &str, label: &str, kind: &str, aliases: Vec<String>) -> Result<(), String> {
    let mut pipeline = FST_PIPELINE.write();
    pipeline.gazetteer_mut().add_entity(id, label, kind, &aliases);
    pipeline.build()
}

/// Remove an entity from the gazetteer
/// 
/// Removes all patterns (canonical label + aliases) associated with this entity ID.
/// Triggers rebuild of the Aho-Corasick automaton.
pub fn fst_remove_entity(id: &str) -> Result<(), String> {
    let mut pipeline = FST_PIPELINE.write();
    pipeline.gazetteer_mut().remove_entity(id);
    pipeline.build()
}

/// Get pipeline stats
pub fn fst_stats() -> FstStats {
    let pipeline = FST_PIPELINE.read();
    FstStats {
        gazetteer_patterns: pipeline.gazetteer_pattern_count(),
        rule_count: pipeline.rule_count(),
    }
}

/// Pipeline statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FstStats {
    pub gazetteer_patterns: usize,
    pub rule_count: usize,
}

// =============================================================================
// Tauri Commands
// =============================================================================

/// Scan text for entities (Tauri command)
#[tauri::command]
pub fn ner_fst_scan(text: String) -> Result<FstScanResult, String> {
    Ok(fst_scan(&text))
}

/// Hydrate with entities (Tauri command)
#[tauri::command]
pub fn ner_fst_hydrate(entities: Vec<EntityDef>) -> Result<usize, String> {
    fst_hydrate(entities)
}

/// Clear gazetteer (Tauri command)
#[tauri::command]
pub fn ner_fst_clear() -> Result<(), String> {
    fst_clear();
    Ok(())
}

/// Add single entity (Tauri command)
#[tauri::command]
pub fn ner_fst_add_entity(
    id: String,
    label: String,
    kind: String,
    aliases: Vec<String>,
) -> Result<(), String> {
    fst_add_entity(&id, &label, &kind, aliases)
}

/// Remove entity from gazetteer (Tauri command)
/// 
/// Removes all patterns for this entity ID and rebuilds the automaton.
#[tauri::command]
pub fn ner_fst_remove_entity(id: String) -> Result<(), String> {
    fst_remove_entity(&id)
}

/// Get stats (Tauri command)
#[tauri::command]
pub fn ner_fst_stats() -> Result<FstStats, String> {
    Ok(fst_stats())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fst_scan_basic() {
        // Clear any previous state
        fst_clear();
        
        let result = fst_scan("Lord Voldemort appeared.");
        
        // Should find via rules
        assert!(!result.matches.is_empty());
    }

    #[test]
    fn test_fst_hydrate_and_scan() {
        fst_clear();
        
        let entities = vec![
            EntityDef {
                id: "ent_1".to_string(),
                label: "Zorian".to_string(),
                kind: "CHARACTER".to_string(),
                aliases: vec!["Zor".to_string()],
            },
        ];
        
        fst_hydrate(entities).unwrap();
        
        let result = fst_scan("Zorian walked. Then Zor spoke.");
        
        // Should find both Zorian and Zor
        assert!(result.matches.iter().any(|m| m.text == "Zorian"));
        assert!(result.matches.iter().any(|m| m.text == "Zor"));
    }

    #[test]
    fn test_fst_add_entity() {
        fst_clear();
        
        fst_add_entity(
            "ent_c",
            "Cyoria",
            "LOCATION",
            vec!["The City".to_string()],
        ).unwrap();
        
        let result = fst_scan("He lived in Cyoria.");
        
        let cyoria = result.matches.iter().find(|m| m.text == "Cyoria");
        assert!(cyoria.is_some());
        assert_eq!(cyoria.unwrap().kind, "LOCATION");
    }

    #[test]
    fn test_fst_stats() {
        fst_clear();
        
        let stats_before = fst_stats();
        
        fst_add_entity("e1", "Test", "CHARACTER", vec![]).unwrap();
        
        let stats_after = fst_stats();
        
        // Should have more patterns after adding
        assert!(stats_after.gazetteer_patterns > stats_before.gazetteer_patterns);
    }

    #[test]
    fn test_fst_remove_entity() {
        fst_clear();
        
        // Add entity
        fst_add_entity(
            "ent_remove_test",
            "Removable",
            "CHARACTER",
            vec!["Alias1".to_string()],
        ).unwrap();
        
        // Verify it's found
        let result = fst_scan("Removable walked with Alias1.");
        assert!(result.matches.iter().any(|m| m.text == "Removable"));
        assert!(result.matches.iter().any(|m| m.text == "Alias1"));
        
        // Remove entity
        fst_remove_entity("ent_remove_test").unwrap();
        
        // Verify it's no longer found (gazetteer match gone)
        let result_after = fst_scan("Removable walked with Alias1.");
        let gaz_matches: Vec<_> = result_after.matches.iter()
            .filter(|m| m.source == "gazetteer" && (m.text == "Removable" || m.text == "Alias1"))
            .collect();
        assert!(gaz_matches.is_empty(), "Entity should be removed from gazetteer: {:?}", gaz_matches);
    }
}
