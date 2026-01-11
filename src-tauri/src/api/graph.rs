//! Graph API - Entity registry, edges, and hydration
//!
//! Delegates directly to existing graph commands.
//! Uses JSON string passthrough for request types.

use crate::graph::commands;

// Import request types for parsing (not re-exported to avoid specta issues)
use commands::{RegisterNodeRequest, CreateEdgeRequest, IngestRequest};

// ============================================================================
// API Trait
// ============================================================================

#[taurpc::procedures(path = "graph", export_to = "../src/bindings.ts")]
pub trait GraphApi {
    // Node CRUD - uses same request types as original
    async fn register_node(request: String) -> Result<String, String>;
    async fn get_node(id: String) -> Result<Option<String>, String>;
    async fn find_node(label: String) -> Result<Option<String>, String>;
    async fn get_nodes(kind: Option<String>) -> Result<String, String>;
    async fn delete_node(id: String) -> Result<bool, String>;
    async fn clear_all() -> Result<usize, String>;
    
    // Edge CRUD
    async fn create_edge(request: String) -> Result<String, String>;
    async fn get_edges(node_id: String, direction: Option<String>) -> Result<String, String>;
    async fn delete_edge(id: String) -> Result<bool, String>;
    
    // Hydration
    async fn get_entities_for_hydration() -> Result<String, String>;
    async fn get_all_entities() -> Result<String, String>;
    async fn invalidate_hydration() -> Result<(), String>;
    
    // Ingest
    async fn ingest_scan_result(request: String) -> Result<String, String>;
    
    // Stats
    async fn stats() -> Result<String, String>;
}

// ============================================================================
// Implementation
// ============================================================================

#[derive(Clone, Default)]
pub struct GraphApiImpl;

#[taurpc::resolvers]
impl GraphApi for GraphApiImpl {
    async fn register_node(self, request: String) -> Result<String, String> {
        let req: RegisterNodeRequest = serde_json::from_str(&request)
            .map_err(|e| format!("Failed to parse request: {}", e))?;
        let result = commands::graph_register_node(req)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_node(self, id: String) -> Result<Option<String>, String> {
        let result = commands::graph_get_node(id)?;
        match result {
            Some(node) => Ok(Some(serde_json::to_string(&node).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn find_node(self, label: String) -> Result<Option<String>, String> {
        let result = commands::graph_find_node(label)?;
        match result {
            Some(node) => Ok(Some(serde_json::to_string(&node).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn get_nodes(self, kind: Option<String>) -> Result<String, String> {
        let result = commands::graph_get_nodes(kind)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_node(self, id: String) -> Result<bool, String> {
        commands::graph_delete_node(id)
    }
    
    async fn clear_all(self) -> Result<usize, String> {
        commands::graph_clear_all()
    }
    
    async fn create_edge(self, request: String) -> Result<String, String> {
        let req: CreateEdgeRequest = serde_json::from_str(&request)
            .map_err(|e| format!("Failed to parse request: {}", e))?;
        let result = commands::graph_create_edge(req)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_edges(self, node_id: String, direction: Option<String>) -> Result<String, String> {
        let result = commands::graph_get_edges(node_id, direction)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_edge(self, id: String) -> Result<bool, String> {
        commands::graph_delete_edge(id)
    }
    
    async fn get_entities_for_hydration(self) -> Result<String, String> {
        let result = commands::graph_get_entities_for_hydration()?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_all_entities(self) -> Result<String, String> {
        let result = commands::graph_get_all_entities()?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn invalidate_hydration(self) -> Result<(), String> {
        commands::graph_invalidate_hydration()
    }
    
    async fn ingest_scan_result(self, request: String) -> Result<String, String> {
        let req: IngestRequest = serde_json::from_str(&request)
            .map_err(|e| format!("Failed to parse request: {}", e))?;
        let result = commands::graph_ingest_scan_result(req)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn stats(self) -> Result<String, String> {
        let result = commands::graph_stats()?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
}
