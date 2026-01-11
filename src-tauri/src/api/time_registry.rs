//! Time Registry API - Entity and edge change history

use crate::time_registry::commands as cmd;

// ============================================================================
// API Trait
// ============================================================================

#[taurpc::procedures(path = "time_registry", export_to = "../src/bindings.ts")]
pub trait TimeRegistryApi {
    async fn init() -> Result<(), String>;
    async fn record_entity_change(params: String) -> Result<(), String>;
    async fn get_entity_history(entity_id: String) -> Result<String, String>;
    async fn record_edge_change(params: String) -> Result<(), String>;
    async fn get_edge_history(source_id: String, target_id: String) -> Result<String, String>;
}

// ============================================================================
// Implementation
// ============================================================================

#[derive(Clone, Default)]
pub struct TimeRegistryApiImpl;

#[taurpc::resolvers]
impl TimeRegistryApi for TimeRegistryApiImpl {
    async fn init(self) -> Result<(), String> {
        cmd::time_registry_init()
    }
    
    async fn record_entity_change(self, params: String) -> Result<(), String> {
        let input: cmd::RecordChangeInput = serde_json::from_str(&params)
            .map_err(|e| format!("Failed to parse params: {}", e))?;
        cmd::time_registry_record_entity_change(input)
    }
    
    async fn get_entity_history(self, entity_id: String) -> Result<String, String> {
        let result = cmd::time_registry_get_entity_history(entity_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn record_edge_change(self, params: String) -> Result<(), String> {
        let input: cmd::RecordEdgeChangeInput = serde_json::from_str(&params)
            .map_err(|e| format!("Failed to parse params: {}", e))?;
        cmd::time_registry_record_edge_change(input)
    }
    
    async fn get_edge_history(self, source_id: String, target_id: String) -> Result<String, String> {
        let result = cmd::time_registry_get_edge_history(source_id, target_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
}
