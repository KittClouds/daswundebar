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
pub mod graph;    // Graph Registry - Unified nodes + edges (CozoDB)
mod rag;      // RAG Pipeline - Embeddings + HNSW (CozoDB native)
mod resorank; // ResoRank - BM25F + Proximity scoring (ported from kittcore)
mod blueprint; // Blueprint Hub - Entity/Relationship type definitions
mod time_registry; // Time Registry - Change history tracking (V2 Phase 2)
mod scan_worker; // Scan Worker - Background note scanning (Phase 1)
mod ai;          // AI Subsystem - NER and Inference (Phase 1 NER)
mod ner;         // FST-NER Engine - Hot-path NER (Phase 1)
mod crossdoc;    // Cross-Doc Entity Linking (Phase 1)
mod mention;     // Unified Mention Contract - Single type for all extractors
mod api;         // TauRPC API - Typed IPC layer (Phase 2.2)

// (SurrealDB REMOVED - migrated to CozoDB content_repos.rs)

#[cfg(test)]
mod benchmark_tests;

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

// ResoRank scorer (BM25F + proximity scoring)
static RESORANK: Lazy<Mutex<resorank::ResoRankScorer>> = Lazy::new(|| {
    log::info!("Initializing ResoRank scorer...");
    let corpus_stats = resorank::CorpusStatistics::default();
    Mutex::new(resorank::ResoRankScorer::with_defaults(corpus_stats))
});

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

// Global ScanQueue for background note scanning
static SCAN_QUEUE: Lazy<std::sync::Arc<scan_worker::ScanQueue>> = Lazy::new(|| {
    log::info!("Initializing ScanQueue...");
    std::sync::Arc::new(scan_worker::ScanQueue::new())
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

/// ResoRank search - BM25F with proximity scoring
#[tauri::command]
fn resorank_search(query: String, limit: usize) -> Result<Vec<resorank::SearchResult>, String> {
    let mut scorer = RESORANK.lock();
    
    // Tokenize query (simple whitespace split, lowercase)
    let query_tokens: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .filter(|s| s.len() > 1)
        .map(|s| s.to_string())
        .collect();
    
    if query_tokens.is_empty() {
        return Ok(vec![]);
    }
    
    let results = scorer.search(&query_tokens, limit);
    Ok(results)
}

/// ResoRank index document - add to BM25F index
#[tauri::command]
fn resorank_index(doc_id: String, title: String, content: String) -> Result<bool, String> {
    let mut scorer = RESORANK.lock();
    
    // Simple tokenizer (whitespace + lowercase)
    let tokenize = |text: &str| -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    };
    
    let title_tokens = tokenize(&title);
    let content_tokens = tokenize(&content);
    let all_tokens: Vec<&str> = title_tokens.iter()
        .chain(content_tokens.iter())
        .map(|s| s.as_str())
        .collect();
    
    if all_tokens.is_empty() {
        return Ok(false);
    }
    
    // Build document metadata
    let mut doc_meta = resorank::DocumentMetadata::new();
    doc_meta.set_field_length(0, title_tokens.len() as u32);  // Field 0 = title
    doc_meta.set_field_length(1, content_tokens.len() as u32); // Field 1 = content
    
    // Build token metadata with field info and segment masks
    let mut token_map = std::collections::HashMap::new();
    let total_len = all_tokens.len();
    let max_segments = 16u32;
    
    for (position, token) in all_tokens.iter().enumerate() {
        let field_id = if position < title_tokens.len() { 0 } else { 1 };
        let field_length = if field_id == 0 { title_tokens.len() } else { content_tokens.len() };
        
        let entry = token_map.entry(token.to_string()).or_insert_with(|| {
            resorank::TokenMetadata::new(1) // Corpus doc freq = 1 for now (simplified)
        });
        
        // Add field occurrence
        entry.add_field_occurrence(field_id, 1, field_length as u32);
        
        // Calculate segment mask (which 1/16th of doc is this token in?)
        let segment = ((position as f32 / total_len as f32) * max_segments as f32) as u32;
        if segment < 32 {
            entry.segment_mask |= 1 << segment;
        }
    }
    
    scorer.index_document(&doc_id, doc_meta, token_map, false);
    log::debug!("ResoRank: indexed doc {} ({} title, {} content tokens)", 
        doc_id, title_tokens.len(), content_tokens.len());
    
    Ok(true)
}

/// ResoRank clear index
#[tauri::command]
fn resorank_clear() -> Result<(), String> {
    let mut scorer = RESORANK.lock();
    scorer.clear();
    log::info!("ResoRank: cleared index");
    Ok(())
}

/// ResoRank get stats
#[tauri::command]
fn resorank_stats() -> Result<serde_json::Value, String> {
    let scorer = RESORANK.lock();
    let stats = scorer.stats();
    Ok(serde_json::json!({
        "documentCount": stats.document_count,
        "termCount": stats.term_count,
        "idfCacheSize": stats.idf_cache_size,
        "entropyCacheSize": stats.entropy_cache_size,
    }))
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

// NOTE: conductor_scan and conductor_scan_force removed in Phase 4
// Replaced by ScanWorker background scanning + get_decoration_spans

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
// Scan Worker Commands (Phase 1: Background Scanning)
// ============================================================================

/// Get cached decoration spans for a note
#[tauri::command]
fn get_decoration_spans(
    note_id: String,
    content_hash: String,
) -> Result<Option<String>, String> {
    let registry = graph::commands::GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    let db = registry.db();
    
    match scan_worker::DecorationCache::get(db, &note_id, &content_hash) {
        Ok(Some(spans)) => {
            let json = serde_json::to_string(&spans)
                .map_err(|e| format!("Serialization error: {}", e))?;
            Ok(Some(json))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Queue a note for background scanning
#[tauri::command]
fn queue_note_scan(note_id: String) -> Result<(), String> {
    SCAN_QUEUE.push(note_id.clone());
    log::debug!("[ScanWorker] Queued note for scan: {}", note_id);
    Ok(())
}

/// Get scan queue status
#[tauri::command]
fn scan_queue_status() -> Result<String, String> {
    Ok(format!(
        r#"{{"queue_length": {}}}"#,
        SCAN_QUEUE.len()
    ))
}

/// Invalidate all cached decoration spans (called on entity hydration)
#[tauri::command]
fn invalidate_all_decoration_spans() -> Result<usize, String> {
    let registry = graph::commands::GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    let db = registry.db();
    scan_worker::DecorationCache::clear_all(db)
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
        // NOTE: TauRPC is enabled but frontend still uses legacy invoke() calls.
        // Keep both until frontend migrates to TauRPC bindings.
        // TauRPC router: api::create_router().into_handler()
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
            resorank_clear,
            resorank_stats,
            conductor_hydrate,
            conductor_status,
            conductor_reset,
            get_decoration_spans,
            queue_note_scan,
            scan_queue_status,
            invalidate_all_decoration_spans,
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
            graph::commands::graph_invalidate_hydration,
            graph::commands::graph_ingest_scan_result,
            graph::commands::graph_stats,
            rag::commands::rag_init_embedder,
            rag::commands::rag_embed,
            rag::commands::rag_embedder_ready,
            rag::commands::rag_chunk_text,
            rag::commands::rag_index_note,
            rag::commands::rag_search,
            rag::commands::rag_get_chunks,
            rag::commands::rag_delete_note_chunks,
            blueprint::commands::blueprint_init,
            blueprint::commands::blueprint_create,
            blueprint::commands::blueprint_get,
            blueprint::commands::blueprint_list,
            blueprint::commands::blueprint_update,
            blueprint::commands::blueprint_delete,
            blueprint::commands::blueprint_version_create,
            blueprint::commands::blueprint_version_list,
            blueprint::commands::blueprint_version_delete,
            blueprint::commands::blueprint_entity_type_create,
            blueprint::commands::blueprint_entity_type_list,
            blueprint::commands::blueprint_entity_type_delete,
            blueprint::commands::blueprint_field_create,
            blueprint::commands::blueprint_field_list,
            blueprint::commands::blueprint_field_delete,
            blueprint::commands::blueprint_relationship_type_create,
            blueprint::commands::blueprint_relationship_type_list,
            blueprint::commands::blueprint_relationship_type_delete,
            time_registry::commands::time_registry_init,
            time_registry::commands::time_registry_record_entity_change,
            time_registry::commands::time_registry_get_entity_history,
            time_registry::commands::time_registry_record_edge_change,
            time_registry::commands::time_registry_get_edge_history,
            graph::content_commands::cozo_create_note,
            graph::content_commands::cozo_get_note,
            graph::content_commands::cozo_list_notes,
            graph::content_commands::cozo_update_note,
            graph::content_commands::cozo_delete_note,
            graph::content_commands::cozo_create_folder,
            graph::content_commands::cozo_get_folder,
            graph::content_commands::cozo_list_folders,
            graph::content_commands::cozo_get_folder_tree,
            graph::content_commands::cozo_update_folder,
            graph::content_commands::cozo_delete_folder,
            graph::content_commands::cozo_create_network,
            graph::content_commands::cozo_get_network,
            graph::content_commands::cozo_list_networks,
            graph::content_commands::cozo_delete_network,
            graph::content_commands::cozo_create_entity,
            graph::content_commands::cozo_get_entity,
            graph::content_commands::cozo_list_entities_by_kind,
            graph::content_commands::cozo_list_entities,
            graph::content_commands::cozo_delete_entity,
            graph::content_commands::cozo_create_relationship,
            graph::content_commands::cozo_get_relationship,
            graph::content_commands::cozo_get_entity_relationships,
            graph::content_commands::cozo_delete_relationship,
            graph::content_commands::cozo_create_cal_event,
            graph::content_commands::cozo_get_cal_event,
            graph::content_commands::cozo_list_cal_events,
            graph::content_commands::cozo_list_cal_events_by_month,
            graph::content_commands::cozo_delete_cal_event,
            graph::content_commands::cozo_create_period,
            graph::content_commands::cozo_get_period,
            graph::content_commands::cozo_list_periods,
            graph::content_commands::cozo_get_period_children,
            graph::content_commands::cozo_delete_period,
            graph::content_commands::cozo_create_binding,
            graph::content_commands::cozo_get_binding,
            graph::content_commands::cozo_list_bindings,
            graph::content_commands::cozo_list_bindings_by_entity,
            graph::content_commands::cozo_delete_binding,
            ai::commands::ner_get_model_status,
            ai::commands::ner_download_model,
            ai::commands::ner_request_analysis,
            ai::commands::ner_get_suggestions,
            ai::commands::ner_get_all_suggestions,
            ai::commands::ner_accept_suggestion,
            ai::commands::ner_reject_suggestion,
            ai::commands::ner_get_settings,
            ai::commands::ner_update_settings,
            ai::commands::ner_pending_count,
            ai::commands::ner_clear_note_suggestions,
            ai::commands::ner_add_suggestion,
            // FST-NER Hot-path commands
            ner::commands::ner_fst_scan,
            ner::commands::ner_fst_hydrate,
            ner::commands::ner_fst_clear,
            ner::commands::ner_fst_add_entity,
            ner::commands::ner_fst_stats,
            // CrossDoc Commands
            crossdoc::commands::crossdoc_find_duplicates,
            crossdoc::commands::crossdoc_find_similar,
            crossdoc::commands::crossdoc_stats,
            crossdoc::commands::crossdoc_embed_all_missing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
