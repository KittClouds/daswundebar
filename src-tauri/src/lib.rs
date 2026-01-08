//! Varant (KittClouds Native) - Tauri Backend
//!
//! Phase 4: Real scanner integration with actual pattern matching.

mod scanner;
mod implicit;
mod temporal;
mod triple;
mod verb_morphology;
mod chunker;
mod relation;
mod structured_relation;
mod incremental;
mod relation_schema;
mod document;
mod conductor;
mod relation_filter;
mod attacher;
mod resolver;
mod dialogue;
mod constraints;
mod narrative;
mod change;
mod reality;  // Reality Engine - Graph + CST (ported from kittcore)
mod graph;    // Graph Registry - Unified nodes + edges (CozoDB)
mod rag;      // RAG Pipeline - Embeddings + HNSW (CozoDB native)

use once_cell::sync::Lazy;
use parking_lot::Mutex;  // FAST mutex (already in deps)

// Global scanner instance (compiled regex patterns are expensive, reuse)
static SCANNER: Lazy<scanner::UnifiedScanner> = Lazy::new(|| {
    log::info!("Initializing UnifiedScanner...");
    scanner::UnifiedScanner::new()
});

// Global ImplicitCortex for entity name matching
static IMPLICIT_CORTEX: Lazy<Mutex<implicit::ImplicitCortex>> = Lazy::new(|| {
    log::info!("Initializing ImplicitCortex...");
    Mutex::new(implicit::ImplicitCortex::new())
});

// Global TemporalCortex for temporal expression detection
static TEMPORAL_CORTEX: Lazy<temporal::TemporalCortex> = Lazy::new(|| {
    log::info!("Initializing TemporalCortex...");
    temporal::TemporalCortex::new()
});

// Global TripleCortex for explicit triple extraction
static TRIPLE_CORTEX: Lazy<triple::TripleCortex> = Lazy::new(|| {
    log::info!("Initializing TripleCortex...");
    triple::TripleCortex::new()
});

// ResoRank index (in-memory for now)
static RESORANK_INDEX: Lazy<Mutex<std::collections::HashMap<String, String>>> = 
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

// Global RelationEngine for CST + Graph relation extraction
static RELATION_ENGINE: Lazy<relation::RelationEngine> = Lazy::new(|| {
    log::info!("Initializing RelationEngine...");
    relation::RelationEngine::new()
});

// Global ScanConductor - the unified scanning API
static CONDUCTOR: Lazy<Mutex<conductor::ScanConductor>> = Lazy::new(|| {
    log::info!("Initializing ScanConductor...");
    let mut conductor = conductor::ScanConductor::new();
    conductor.init();
    Mutex::new(conductor)
});

// ============================================================================
// Scan Result Types (combined output)
// ============================================================================

#[derive(serde::Serialize, Default)]
struct CombinedScanResult {
    // From UnifiedScanner (explicit patterns)
    spans: Vec<scanner::DecorationSpan>,
    // From ImplicitCortex (entity name matching)
    implicit: Vec<implicit::ImplicitMention>,
    // From TemporalCortex (temporal expressions)
    temporal: Vec<temporal::TemporalMention>,
    // From TripleCortex (explicit triples)
    triples: Vec<triple::ExtractedTriple>,
    // Stats
    stats: ScanStats,
}

#[derive(serde::Serialize, Default)]
struct ScanStats {
    explicit_count: usize,
    implicit_count: usize,
    temporal_count: usize,
    triple_count: usize,
    scan_time_us: u64,
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Greet function for testing Tauri IPC
#[tauri::command]
fn greet(name: String) -> String {
    format!("Hello, {}! Varant (Tauri) backend is ready.", name)
}

/// Get version information
#[tauri::command]
fn version() -> String {
    format!("varant v{}", env!("CARGO_PKG_VERSION"))
}

/// Unified document scan - combines explicit patterns + implicit + temporal + triples
#[tauri::command]
fn unified_scan(text: String, _entities_json: String) -> Result<String, String> {
    let start = std::time::Instant::now();
    
    // Phase 1: Explicit pattern matching (wikilinks, entities, tags, etc.)
    let explicit_result = SCANNER.scan(&text);
    
    // Phase 2: Implicit entity matching (Aho-Corasick)
    let implicit_mentions = {
        let cortex = IMPLICIT_CORTEX.lock();
        cortex.find_mentions(&text)
    };
    
    // Phase 3: Temporal expression detection
    let temporal_result = TEMPORAL_CORTEX.scan(&text);
    
    // Phase 4: Explicit triple extraction
    let extracted_triples = TRIPLE_CORTEX.extract(&text);
    
    // Combine results
    let result = CombinedScanResult {
        stats: ScanStats {
            explicit_count: explicit_result.spans.len(),
            implicit_count: implicit_mentions.len(),
            temporal_count: temporal_result.mentions.len(),
            triple_count: extracted_triples.len(),
            scan_time_us: start.elapsed().as_micros() as u64,
        },
        spans: explicit_result.spans,
        implicit: implicit_mentions,
        temporal: temporal_result.mentions,
        triples: extracted_triples,
    };
    
    serde_json::to_string(&result)
        .map_err(|e| format!("Serialization error: {}", e))
}

/// Hydrate entities for implicit matching
#[tauri::command]
fn hydrate_entities(entities_json: String) -> Result<String, String> {
    // Parse entity definitions
    let entities: Vec<implicit::EntityDefinition> = serde_json::from_str(&entities_json)
        .map_err(|e| format!("Failed to parse entities: {}", e))?;
    
    let count = entities.len();
    
    // Hydrate the ImplicitCortex
    {
        let mut cortex = IMPLICIT_CORTEX.lock();
        cortex.hydrate(entities);
        cortex.build().map_err(|e| format!("Failed to build automaton: {}", e))?;
        log::info!("Hydrated ImplicitCortex with {} entities ({} patterns)", 
            count, cortex.pattern_count());
    }
    
    Ok(format!("Hydrated {} entities", count))
}

/// Extract triples from text - REAL implementation using TripleCortex
#[tauri::command]
fn extract_triples(text: String) -> Result<String, String> {
    let triples = TRIPLE_CORTEX.extract(&text);
    serde_json::to_string(&triples)
        .map_err(|e| format!("Serialization error: {}", e))
}

/// Scan temporal expressions - REAL implementation
#[tauri::command]
fn scan_temporal(text: String) -> Result<String, String> {
    let result = TEMPORAL_CORTEX.scan(&text);
    serde_json::to_string(&result.mentions)
        .map_err(|e| format!("Serialization error: {}", e))
}

/// Extract relations using CST + Graph inference
#[tauri::command]
fn extract_relations(text: String, entities_json: String) -> Result<String, String> {
    // Parse entity spans
    let entities: Vec<relation::EntitySpan> = serde_json::from_str(&entities_json)
        .map_err(|e| format!("Failed to parse entities: {}", e))?;
    
    // Extract relations (no existing edges for now)
    let (relations, stats) = RELATION_ENGINE.extract(&text, &entities, &[]);
    
    log::debug!("Extracted {} relations ({} CST, {} inferred) in {}μs", 
        stats.total_count, stats.cst_count, stats.inferred_count, stats.time_us);
    
    serde_json::to_string(&relations)
        .map_err(|e| format!("Serialization error: {}", e))
}

/// Scan document syntax
#[tauri::command]
fn scan_syntax(_text: String) -> Result<String, String> {
    // TODO: Port syntax scanner
    Ok("[]".to_string())
}

/// ResoRank search (in-memory for now)
#[tauri::command]
fn resorank_search(query: String, limit: usize) -> Result<String, String> {
    let index = RESORANK_INDEX.lock();
    
    // Simple substring search
    let query_lower = query.to_lowercase();
    let mut results: Vec<(&String, &String)> = index.iter()
        .filter(|(_, content)| content.to_lowercase().contains(&query_lower))
        .take(limit)
        .collect();
    
    // Sort by match position (earlier = better)
    results.sort_by(|(_, a), (_, b)| {
        let pos_a = a.to_lowercase().find(&query_lower).unwrap_or(usize::MAX);
        let pos_b = b.to_lowercase().find(&query_lower).unwrap_or(usize::MAX);
        pos_a.cmp(&pos_b)
    });
    
    let result: Vec<_> = results.iter()
        .map(|(id, content)| serde_json::json!({
            "id": id,
            "score": 1.0,
            "snippet": content.chars().take(200).collect::<String>()
        }))
        .collect();
    
    serde_json::to_string(&result)
        .map_err(|e| format!("Serialization error: {}", e))
}

/// ResoRank index document
#[tauri::command]
fn resorank_index(doc_id: String, content: String) -> Result<String, String> {
    let mut index = RESORANK_INDEX.lock();
    index.insert(doc_id.clone(), content);
    log::debug!("Indexed document: {}", doc_id);
    Ok("Indexed".to_string())
}

// ============================================================================
// ScanConductor Commands (Unified Scanner API)
// ============================================================================

/// Hydrate the ScanConductor with entity definitions
#[tauri::command]
fn conductor_hydrate(entities_json: String) -> Result<String, String> {
    let entities: Vec<implicit::EntityDefinition> = serde_json::from_str(&entities_json)
        .map_err(|e| format!("Failed to parse entities: {}", e))?;
    
    let count = entities.len();
    
    {
        let mut conductor = CONDUCTOR.lock();
        conductor.hydrate_entities(entities)
            .map_err(|e| format!("Failed to hydrate: {}", e))?;
        log::info!("Conductor hydrated with {} entities", count);
    }
    
    Ok(format!("{{\"hydrated\": {}}}", count))
}

/// Full document scan using ScanConductor
#[tauri::command]
fn conductor_scan(text: String, entities_json: String) -> Result<String, String> {
    let total_start = std::time::Instant::now();
    
    // Parse optional entity spans
    let parse_start = std::time::Instant::now();
    let external_spans: Vec<relation::EntitySpan> = if entities_json.is_empty() || entities_json == "[]" {
        vec![]
    } else {
        serde_json::from_str(&entities_json).unwrap_or_default()
    };
    let parse_time = parse_start.elapsed();
    
    // Acquire lock
    let lock_start = std::time::Instant::now();
    let mut conductor = CONDUCTOR.lock();
    let lock_time = lock_start.elapsed();
    
    // Perform scan
    let scan_start = std::time::Instant::now();
    match conductor.scan(&text, &external_spans) {
        Some(result) => {
            let scan_time = scan_start.elapsed();
            
            // Serialize result
            let serialize_start = std::time::Instant::now();
            let json = serde_json::to_string(&result)
                .map_err(|e| format!("Serialization error: {}", e))?;
            let serialize_time = serialize_start.elapsed();
            
            let total_time = total_start.elapsed();
            
            // Log detailed timing breakdown
            log::debug!(
                "[conductor_scan] text={}chars parse={}µs lock={}µs scan={}µs serialize={}µs total={}µs | implicit={} unified={} triples={}",
                text.len(),
                parse_time.as_micros(),
                lock_time.as_micros(),
                scan_time.as_micros(),
                serialize_time.as_micros(),
                total_time.as_micros(),
                result.stats.implicit_found,
                result.stats.unified_found,
                result.stats.triples_found,
            );
            
            Ok(json)
        }
        None => {
            Err("Conductor not ready - call conductor_hydrate first".to_string())
        }
    }
}

/// Force scan even if conductor is not ready (for debugging)
#[tauri::command]
fn conductor_scan_force(text: String, entities_json: String) -> Result<String, String> {
    let external_spans: Vec<relation::EntitySpan> = if entities_json.is_empty() || entities_json == "[]" {
        vec![]
    } else {
        serde_json::from_str(&entities_json).unwrap_or_default()
    };
    
    let mut conductor = CONDUCTOR.lock();
    let result = conductor.scan_force(&text, &external_spans);
    
    serde_json::to_string(&result)
        .map_err(|e| format!("Serialization error: {}", e))
}

/// Get conductor status
#[tauri::command]
fn conductor_status() -> Result<String, String> {
    let conductor = CONDUCTOR.lock();
    let stats = conductor.incremental_stats();
    
    Ok(format!(
        r#"{{"state": "{}", "entity_count": {}, "incremental_scans": {}, "full_rescans": {}, "avg_dirty_ratio": {:.4}}}"#,
        conductor.state_name(),
        conductor.entity_count(),
        stats.incremental_count,
        stats.full_rescan_count,
        stats.avg_dirty_ratio
    ))
}

/// Reset conductor state
#[tauri::command]
fn conductor_reset() -> Result<String, String> {
    let mut conductor = CONDUCTOR.lock();
    conductor.reset();
    log::info!("Conductor reset");
    Ok(r#"{"reset": true}"#.to_string())
}

// ============================================================================
// Tauri App Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            
            // Force scanner initialization on startup
            let _ = &*SCANNER;
            let _ = &*IMPLICIT_CORTEX;
            let _ = &*TEMPORAL_CORTEX;
            let _ = &*TRIPLE_CORTEX;
            let _ = &*RELATION_ENGINE;
            let _ = &*CONDUCTOR;
            
            // Initialize Graph Registry
            let _ = &*graph::commands::GRAPH_REGISTRY;
            let _ = &*graph::commands::SCANNER_BRIDGE;
            
            log::info!("Varant backend initialized (Phase 6 - Unified Graph Registry)");
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            version,
            unified_scan,
            hydrate_entities,
            extract_triples,
            scan_temporal,
            scan_syntax,
            extract_relations,
            resorank_search,
            resorank_index,
            conductor_hydrate,
            conductor_scan,
            conductor_scan_force,
            conductor_status,
            conductor_reset,
            // Graph Registry commands (Phase 2.1)
            graph::commands::graph_register_node,
            graph::commands::graph_get_node,
            graph::commands::graph_find_node,
            graph::commands::graph_get_nodes,
            graph::commands::graph_delete_node,
            graph::commands::graph_clear_all,
            graph::commands::graph_create_edge,
            graph::commands::graph_get_edges,
            graph::commands::graph_delete_edge,
            graph::commands::graph_get_entities_for_hydration,
            graph::commands::graph_get_all_entities,
            graph::commands::graph_ingest_scan_result,
            graph::commands::graph_invalidate_hydration,
            graph::commands::graph_stats,
            // RAG Pipeline commands (Phase 3 - Embeddings + HNSW)
            rag::commands::rag_init_embedder,
            rag::commands::rag_embed,
            rag::commands::rag_embedder_ready,
            rag::commands::rag_chunk_text,
            rag::commands::rag_index_note,
            rag::commands::rag_search,
            rag::commands::rag_get_chunks,
            rag::commands::rag_delete_note_chunks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


