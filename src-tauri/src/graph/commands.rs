//! Graph Registry Tauri Commands
//!
//! Exposes GraphRegistry and ScannerBridge to the TypeScript frontend.
//! Commands follow the incremental migration pattern - new commands
//! alongside existing ones, allowing gradual adoption.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::graph::{
    GraphRegistry, ScannerBridge, NodeInput, NodeFilter, NodeKind, 
    CreatedBy, EdgeInput, Direction, ScannedMention, ScannedRelation, 
    RelationSource,
};

// FST-NER commands for reactive updates
use crate::ner::commands::{fst_add_entity, fst_remove_entity};

// =============================================================================
// Global State
// =============================================================================

/// Get the path for persistent graph storage
fn get_graph_db_path() -> String {
    // Use platform-appropriate data directory
    let data_dir = dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    
    let varant_dir = data_dir.join("varant");
    
    // Create directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&varant_dir) {
        log::warn!("[GraphRegistry] Could not create data dir: {}, falling back to in-memory", e);
        return String::new();
    }
    
    varant_dir.join("graph.db").to_string_lossy().to_string()
}

/// Global GraphRegistry instance (SQLite-backed CozoDB)
pub static GRAPH_REGISTRY: Lazy<Mutex<GraphRegistry>> = Lazy::new(|| {
    let path = get_graph_db_path();
    
    if path.is_empty() {
        log::info!("[GraphRegistry] Initializing with in-memory storage (fallback)...");
        Mutex::new(GraphRegistry::in_memory().expect("Failed to create in-memory GraphRegistry"))
    } else {
        log::info!("[GraphRegistry] Initializing with SQLite at: {}", path);
        match GraphRegistry::new(&path) {
            Ok(registry) => Mutex::new(registry),
            Err(e) => {
                log::error!("[GraphRegistry] SQLite init failed ({}), falling back to in-memory", e);
                Mutex::new(GraphRegistry::in_memory().expect("Failed to create in-memory GraphRegistry"))
            }
        }
    }
});

/// Global ScannerBridge for smart hydration
pub static SCANNER_BRIDGE: Lazy<Mutex<ScannerBridge>> = Lazy::new(|| {
    log::info!("[ScannerBridge] Initializing...");
    Mutex::new(ScannerBridge::new())
});


// =============================================================================
// Request/Response Types (JSON serializable)
// =============================================================================

#[taurpc::ipc_type]
pub struct RegisterNodeRequest {
    pub label: String,
    pub kind: String,
    pub source_note: String,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default = "default_created_by")]
    pub created_by: String,
}

fn default_created_by() -> String { "user".to_string() }

#[taurpc::ipc_type]
pub struct NodeResponse {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub mention_count: i64,
    pub aliases: Vec<String>,
    pub is_new: bool,
}

#[taurpc::ipc_type]
pub struct CreateEdgeRequest {
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub source_note: Option<String>,
}

#[taurpc::ipc_type]
pub struct EdgeResponse {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub confidence: f64,
}

#[taurpc::ipc_type]
pub struct IngestRequest {
    pub source_note: String,
    #[serde(default)]
    pub mentions: Vec<MentionInput>,
    #[serde(default)]
    pub relations: Vec<RelationInput>,
}

#[taurpc::ipc_type]
pub struct MentionInput {
    pub entity_label: String,
    pub entity_kind: String,
}

#[taurpc::ipc_type]
pub struct RelationInput {
    pub head_label: String,
    pub tail_label: String,
    pub relation_type: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_confidence() -> f64 { 0.8 }
fn default_source() -> String { "extraction".to_string() }

#[taurpc::ipc_type]
pub struct IngestResponse {
    pub entities_created: usize,
    pub entities_updated: usize,
    pub edges_created: usize,
    pub edges_updated: usize,
}

#[taurpc::ipc_type]
pub struct HydrateResponse {
    pub entity_count: usize,
    pub needs_hydration: bool,
    pub entities: Option<Vec<EntityDef>>,
}

#[taurpc::ipc_type]
pub struct EntityDef {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub aliases: Vec<String>,
}

// =============================================================================
// Node Commands
// =============================================================================

/// Register a node (entity) - returns existing if found
#[tauri::command]
pub fn graph_register_node(request: RegisterNodeRequest) -> Result<NodeResponse, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let input = NodeInput {
        label: request.label,
        kind: NodeKind::from_str(&request.kind),
        source_note: request.source_note,
        subtype: request.subtype,
        aliases: request.aliases,
        created_by: CreatedBy::from_str(&request.created_by),
        metadata: None,
    };
    
    let result = registry.register_node(input)
        .map_err(|e| e.to_string())?;
    
    // Reactive FST update: add new entity to gazetteer
    if result.is_new {
        if let Err(e) = fst_add_entity(
            &result.node.id,
            &result.node.label,
            result.node.kind.as_str(),
            result.node.aliases.clone(),
        ) {
            log::warn!("[GraphRegistry] FST add_entity failed: {}", e);
        }
    }
    
    Ok(NodeResponse {
        id: result.node.id,
        label: result.node.label,
        kind: result.node.kind.as_str().to_string(),
        mention_count: result.node.mention_count,
        aliases: result.node.aliases,
        is_new: result.is_new,
    })
}

/// Get a node by ID
#[tauri::command]
pub fn graph_get_node(id: String) -> Result<Option<NodeResponse>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let node = registry.get_node(&id).map_err(|e| e.to_string())?;
    
    Ok(node.map(|n| NodeResponse {
        id: n.id,
        label: n.label,
        kind: n.kind.as_str().to_string(),
        mention_count: n.mention_count,
        aliases: n.aliases,
        is_new: false,
    }))
}

/// Find a node by label (case-insensitive, checks aliases)
#[tauri::command]
pub fn graph_find_node(label: String) -> Result<Option<NodeResponse>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let node = registry.find_node(&label).map_err(|e| e.to_string())?;
    
    Ok(node.map(|n| NodeResponse {
        id: n.id,
        label: n.label,
        kind: n.kind.as_str().to_string(),
        mention_count: n.mention_count,
        aliases: n.aliases,
        is_new: false,
    }))
}

/// Get all nodes, optionally filtered by kind
#[tauri::command]
pub fn graph_get_nodes(kind: Option<String>) -> Result<Vec<NodeResponse>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let filter = NodeFilter {
        kind: kind.map(|k| NodeKind::from_str(&k)),
        ..Default::default()
    };
    
    let nodes = registry.get_nodes(filter).map_err(|e| e.to_string())?;
    
    Ok(nodes.into_iter().map(|n| NodeResponse {
        id: n.id,
        label: n.label,
        kind: n.kind.as_str().to_string(),
        mention_count: n.mention_count,
        aliases: n.aliases,
        is_new: false,
    }).collect())
}

/// Delete a node (cascades to edges)
#[tauri::command]
pub fn graph_delete_node(id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    // Reactive FST update: remove entity from gazetteer
    if let Err(e) = fst_remove_entity(&id) {
        log::warn!("[GraphRegistry] FST remove_entity failed: {}", e);
    }
    
    registry.delete_node(&id).map_err(|e| e.to_string())
}

/// Clear all nodes and edges from the graph
/// Returns the number of nodes deleted
#[tauri::command]
pub fn graph_clear_all() -> Result<usize, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    let mut bridge = SCANNER_BRIDGE.lock().map_err(|e| e.to_string())?;
    
    let deleted = registry.clear_all().map_err(|e| e.to_string())?;
    bridge.invalidate(); // Force re-hydration next time
    
    log::info!("[GraphRegistry] Cleared {} nodes via IPC", deleted);
    Ok(deleted)
}

// =============================================================================
// Edge Commands
// =============================================================================

/// Create an edge between two nodes
#[tauri::command]
pub fn graph_create_edge(request: CreateEdgeRequest) -> Result<EdgeResponse, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let input = EdgeInput {
        source_id: request.source_id,
        target_id: request.target_id,
        edge_type: request.edge_type,
        inverse_type: None,
        bidirectional: false,
        weight: 1.0,
        confidence: request.confidence.unwrap_or(1.0),
        created_by: CreatedBy::User,
        source_note: request.source_note,
        metadata: None,
    };
    
    let edge = registry.create_edge(input).map_err(|e| e.to_string())?;
    
    Ok(EdgeResponse {
        id: edge.id,
        source_id: edge.source_id,
        target_id: edge.target_id,
        edge_type: edge.edge_type,
        confidence: edge.confidence,
    })
}

/// Get edges connected to a node
#[tauri::command]
pub fn graph_get_edges(node_id: String, direction: Option<String>) -> Result<Vec<EdgeResponse>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let dir = match direction.as_deref() {
        Some("outgoing") => Direction::Outgoing,
        Some("incoming") => Direction::Incoming,
        _ => Direction::Both,
    };
    
    let edges = registry.get_edges(&node_id, dir).map_err(|e| e.to_string())?;
    
    Ok(edges.into_iter().map(|e| EdgeResponse {
        id: e.id,
        source_id: e.source_id,
        target_id: e.target_id,
        edge_type: e.edge_type,
        confidence: e.confidence,
    }).collect())
}

/// Delete an edge
#[tauri::command]
pub fn graph_delete_edge(id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    registry.delete_edge(&id).map_err(|e| e.to_string())
}

// =============================================================================
// Scanner Bridge Commands
// =============================================================================

/// Smart hydration - only returns entities if set has changed
/// 
/// This is the key command that breaks the circular hydration loop.
/// Returns None if entities haven't changed, Some(entities) if scanner needs update.
#[tauri::command]
pub fn graph_get_entities_for_hydration() -> Result<HydrateResponse, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    let mut bridge = SCANNER_BRIDGE.lock().map_err(|e| e.to_string())?;
    
    let result = bridge.get_entities_if_changed(&registry)
        .map_err(|e| e.to_string())?;
    
    match result {
        Some(entities) => {
            let count = entities.len();
            let defs: Vec<EntityDef> = entities.into_iter()
                .map(|e| EntityDef {
                    id: e.id,
                    label: e.label,
                    kind: e.kind,
                    aliases: e.aliases,
                })
                .collect();
            
            Ok(HydrateResponse {
                entity_count: count,
                needs_hydration: true,
                entities: Some(defs),
            })
        }
        None => {
            // No change, skip hydration
            Ok(HydrateResponse {
                entity_count: 0,
                needs_hydration: false,
                entities: None,
            })
        }
    }
}

/// Force get all entities (bypass change detection)
#[tauri::command]
pub fn graph_get_all_entities() -> Result<Vec<EntityDef>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    let bridge = SCANNER_BRIDGE.lock().map_err(|e| e.to_string())?;
    
    let entities = bridge.get_all_entities(&registry)
        .map_err(|e| e.to_string())?;
    
    Ok(entities.into_iter()
        .map(|e| EntityDef {
            id: e.id,
            label: e.label,
            kind: e.kind,
            aliases: e.aliases,
        })
        .collect())
}

/// Ingest scan results into the graph
/// 
/// This is called after scanning a document. It:
/// 1. Registers new entities from mentions
/// 2. Creates edges from relations
/// 3. Invalidates hydration cache if new entities were added
#[tauri::command]
pub fn graph_ingest_scan_result(request: IngestRequest) -> Result<IngestResponse, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    let mut bridge = SCANNER_BRIDGE.lock().map_err(|e| e.to_string())?;
    
    let mentions: Vec<ScannedMention> = request.mentions.into_iter()
        .map(|m| ScannedMention {
            entity_label: m.entity_label,
            entity_kind: m.entity_kind,
            source_note: request.source_note.clone(),
        })
        .collect();
    
    let relations: Vec<ScannedRelation> = request.relations.into_iter()
        .map(|r| ScannedRelation {
            head_label: r.head_label,
            tail_label: r.tail_label,
            relation_type: r.relation_type,
            confidence: r.confidence,
            source: match r.source.as_str() {
                "explicit" => RelationSource::Explicit,
                "cst" => RelationSource::CstInference,
                _ => RelationSource::GraphInference,
            },
        })
        .collect();
    
    let result = bridge.ingest_scan_result(&registry, mentions, relations, &request.source_note)
        .map_err(|e| e.to_string())?;
    
    Ok(IngestResponse {
        entities_created: result.entities_created,
        entities_updated: result.entities_updated,
        edges_created: result.edges_created,
        edges_updated: result.edges_updated,
    })
}

/// Invalidate hydration cache (force next hydration)
#[tauri::command]
pub fn graph_invalidate_hydration() -> Result<(), String> {
    let mut bridge = SCANNER_BRIDGE.lock().map_err(|e| e.to_string())?;
    bridge.invalidate();
    log::info!("[ScannerBridge] Hydration cache invalidated");
    Ok(())
}

/// Get graph statistics
#[tauri::command]
pub fn graph_stats() -> Result<serde_json::Value, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    let stats = registry.stats().map_err(|e| e.to_string())?;
    
    Ok(serde_json::json!({
        "node_count": stats.node_count,
        "edge_count": stats.edge_count,
        "avg_degree": stats.avg_degree,
        "density": stats.density,
        "component_count": stats.component_count,
    }))
}
