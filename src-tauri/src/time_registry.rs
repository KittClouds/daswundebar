/**
 * Time Registry - Change History Storage
 * 
 * Tracks changes to entities and edges over time.
 * Provides time-travel capabilities for the knowledge graph.
 * 
 * Not to be confused with TemporalCortex which detects temporal expressions.
 */

use std::collections::BTreeMap;
use cozo::DbInstance;
use serde::{Deserialize, Serialize};

// =============================================================================
// Data Models
// =============================================================================

#[taurpc::ipc_type]
pub struct HistoryEntry {
    pub entry_id: String,
    pub entity_id: String,
    pub action: String, // create | update | delete
    pub data_json: String,
    pub timestamp: i64,
}

#[taurpc::ipc_type]
pub struct EdgeHistoryEntry {
    pub entry_id: String,
    pub source_id: String,
    pub target_id: String,
    pub action: String, // create | update | delete
    pub data_json: String,
    pub timestamp: i64,
}

// =============================================================================
// Schema Initialization
// =============================================================================

pub fn init_time_registry_schema(db: &DbInstance) -> Result<(), String> {
    // Entity history table
    let entity_history_query = r#"
        :create entity_history {
            entry_id: String =>
            entity_id: String,
            action: String,
            data_json: String,
            timestamp: Int
        }
    "#;

    // Edge history table
    let edge_history_query = r#"
        :create edge_history {
            entry_id: String =>
            source_id: String,
            target_id: String,
            action: String,
            data_json: String,
            timestamp: Int
        }
    "#;

    // Helper to check if relation exists
    let relation_exists = |db: &DbInstance, name: &str| -> bool {
        let query = "::relations".to_string();
        match db.run_script(&query, BTreeMap::new(), cozo::ScriptMutability::Immutable) {
            Ok(result) => {
                for row in result.rows {
                    if let Some(cozo::DataValue::Str(rel_name)) = row.first() {
                        if rel_name.as_str() == name {
                            return true;
                        }
                    }
                }
                false
            }
            Err(_) => false,
        }
    };

    // Create relations if not exist
    let schemas = [
        ("entity_history", entity_history_query),
        ("edge_history", edge_history_query),
    ];

    for (name, query) in schemas {
        if !relation_exists(db, name) {
            db.run_script(query, BTreeMap::new(), cozo::ScriptMutability::Mutable)
                .map_err(|e| format!("Failed to create {}: {}", name, e))?;
            log::info!("[TimeRegistry] Created relation: {}", name);
        }
    }

    log::info!("[TimeRegistry] Schema initialized");
    Ok(())
}

// =============================================================================
// Entity History CRUD
// =============================================================================

pub fn record_entity_change(
    db: &DbInstance,
    entity_id: &str,
    action: &str,
    data_json: &str,
) -> Result<(), String> {
    let entry_id = format!("eh-{}", uuid::Uuid::new_v4());
    let timestamp = chrono::Utc::now().timestamp();

    let query = r#"
        ?[entry_id, entity_id, action, data_json, timestamp] <- [[
            $entry_id, $entity_id, $action, $data_json, $timestamp
        ]]
        :put entity_history {
            entry_id => entity_id, action, data_json, timestamp
        }
    "#;

    let mut params = BTreeMap::new();
    params.insert("entry_id".to_string(), cozo::DataValue::Str(entry_id.into()));
    params.insert("entity_id".to_string(), cozo::DataValue::Str(entity_id.into()));
    params.insert("action".to_string(), cozo::DataValue::Str(action.into()));
    params.insert("data_json".to_string(), cozo::DataValue::Str(data_json.into()));
    params.insert("timestamp".to_string(), cozo::DataValue::Num(cozo::Num::Int(timestamp)));

    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to record entity change: {}", e))?;

    Ok(())
}

pub fn get_entity_history(db: &DbInstance, entity_id: &str) -> Result<Vec<HistoryEntry>, String> {
    let query = r#"
        ?[entry_id, entity_id, action, data_json, timestamp] := 
            *entity_history{entry_id, entity_id, action, data_json, timestamp},
            entity_id == $id
        :order timestamp
    "#;

    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(entity_id.into()));

    let result = db.run_script(query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get entity history: {}", e))?;

    let mut entries = Vec::new();
    for row in &result.rows {
        entries.push(HistoryEntry {
            entry_id: row[0].get_str().unwrap_or_default().to_string(),
            entity_id: row[1].get_str().unwrap_or_default().to_string(),
            action: row[2].get_str().unwrap_or_default().to_string(),
            data_json: row[3].get_str().unwrap_or_default().to_string(),
            timestamp: row[4].get_int().unwrap_or(0),
        });
    }

    Ok(entries)
}

// =============================================================================
// Edge History CRUD
// =============================================================================

pub fn record_edge_change(
    db: &DbInstance,
    source_id: &str,
    target_id: &str,
    action: &str,
    data_json: &str,
) -> Result<(), String> {
    let entry_id = format!("edh-{}", uuid::Uuid::new_v4());
    let timestamp = chrono::Utc::now().timestamp();

    let query = r#"
        ?[entry_id, source_id, target_id, action, data_json, timestamp] <- [[
            $entry_id, $source_id, $target_id, $action, $data_json, $timestamp
        ]]
        :put edge_history {
            entry_id => source_id, target_id, action, data_json, timestamp
        }
    "#;

    let mut params = BTreeMap::new();
    params.insert("entry_id".to_string(), cozo::DataValue::Str(entry_id.into()));
    params.insert("source_id".to_string(), cozo::DataValue::Str(source_id.into()));
    params.insert("target_id".to_string(), cozo::DataValue::Str(target_id.into()));
    params.insert("action".to_string(), cozo::DataValue::Str(action.into()));
    params.insert("data_json".to_string(), cozo::DataValue::Str(data_json.into()));
    params.insert("timestamp".to_string(), cozo::DataValue::Num(cozo::Num::Int(timestamp)));

    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to record edge change: {}", e))?;

    Ok(())
}

pub fn get_edge_history(
    db: &DbInstance,
    source_id: &str,
    target_id: &str,
) -> Result<Vec<EdgeHistoryEntry>, String> {
    let query = r#"
        ?[entry_id, source_id, target_id, action, data_json, timestamp] := 
            *edge_history{entry_id, source_id, target_id, action, data_json, timestamp},
            source_id == $source_id,
            target_id == $target_id
        :order timestamp
    "#;

    let mut params = BTreeMap::new();
    params.insert("source_id".to_string(), cozo::DataValue::Str(source_id.into()));
    params.insert("target_id".to_string(), cozo::DataValue::Str(target_id.into()));

    let result = db.run_script(query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get edge history: {}", e))?;

    let mut entries = Vec::new();
    for row in &result.rows {
        entries.push(EdgeHistoryEntry {
            entry_id: row[0].get_str().unwrap_or_default().to_string(),
            source_id: row[1].get_str().unwrap_or_default().to_string(),
            target_id: row[2].get_str().unwrap_or_default().to_string(),
            action: row[3].get_str().unwrap_or_default().to_string(),
            data_json: row[4].get_str().unwrap_or_default().to_string(),
            timestamp: row[5].get_int().unwrap_or(0),
        });
    }

    Ok(entries)
}

// =============================================================================
// Tauri Commands
// =============================================================================

pub mod commands {
    use super::*;
    use crate::graph::commands::GRAPH_REGISTRY;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RecordChangeInput {
        pub entity_id: String,
        pub action: String,
        pub data: serde_json::Value,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RecordEdgeChangeInput {
        pub source_id: String,
        pub target_id: String,
        pub action: String,
        pub data: serde_json::Value,
    }

    /// Initialize time registry schema
    #[tauri::command]
    pub fn time_registry_init() -> Result<(), String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        init_time_registry_schema(registry.db())
    }

    /// Record an entity change
    #[tauri::command]
    pub fn time_registry_record_entity_change(input: RecordChangeInput) -> Result<(), String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        let data_json = serde_json::to_string(&input.data).unwrap_or_else(|_| "{}".to_string());
        record_entity_change(registry.db(), &input.entity_id, &input.action, &data_json)
    }

    /// Get entity history
    #[tauri::command]
    pub fn time_registry_get_entity_history(entity_id: String) -> Result<Vec<HistoryEntry>, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        get_entity_history(registry.db(), &entity_id)
    }

    /// Record an edge change
    #[tauri::command]
    pub fn time_registry_record_edge_change(input: RecordEdgeChangeInput) -> Result<(), String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        let data_json = serde_json::to_string(&input.data).unwrap_or_else(|_| "{}".to_string());
        record_edge_change(registry.db(), &input.source_id, &input.target_id, &input.action, &data_json)
    }

    /// Get edge history
    #[tauri::command]
    pub fn time_registry_get_edge_history(source_id: String, target_id: String) -> Result<Vec<EdgeHistoryEntry>, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        get_edge_history(registry.db(), &source_id, &target_id)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> DbInstance {
        DbInstance::new("mem", "", Default::default()).unwrap()
    }

    #[test]
    fn test_time_registry_init() {
        let db = create_test_db();
        let result = init_time_registry_schema(&db);
        assert!(result.is_ok());
        
        // Running twice should be idempotent
        let result2 = init_time_registry_schema(&db);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_entity_history() {
        let db = create_test_db();
        init_time_registry_schema(&db).unwrap();

        // Record a creation
        record_entity_change(
            &db,
            "entity-123",
            "create",
            r#"{"name": "Test Entity"}"#,
        ).unwrap();

        // Record an update
        record_entity_change(
            &db,
            "entity-123",
            "update",
            r#"{"name": "Updated Entity"}"#,
        ).unwrap();

        // Get history
        let history = get_entity_history(&db, "entity-123").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].action, "create");
        assert_eq!(history[1].action, "update");
    }

    #[test]
    fn test_edge_history() {
        let db = create_test_db();
        init_time_registry_schema(&db).unwrap();

        // Record edge creation
        record_edge_change(
            &db,
            "source-1",
            "target-1",
            "create",
            r#"{"type": "knows"}"#,
        ).unwrap();

        // Get history
        let history = get_edge_history(&db, "source-1", "target-1").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].action, "create");
    }
}
