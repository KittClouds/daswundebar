//! Core API - Version, scanning, and relation extraction
//!
//! Delegates directly to existing Tauri commands.

// ============================================================================
// Types (re-export from parent crate)
// ============================================================================

#[taurpc::ipc_type]
pub struct HealthStatus {
    pub ok: bool,
    pub version: String,
}

// ============================================================================
// API Trait
// ============================================================================

#[taurpc::procedures(path = "core", export_to = "../src/bindings.ts")]
pub trait CoreApi {
    // Basics
    async fn version() -> String;
    async fn greet(name: String) -> String;
    async fn health() -> HealthStatus;
    
    // Scanning - returns JSON strings (matching existing API)
    async fn unified_scan(text: String, entities_json: String) -> Result<String, String>;
    async fn hydrate_entities(entities_json: String) -> Result<String, String>;
    async fn extract_triples(text: String) -> Result<String, String>;
    async fn scan_temporal(text: String) -> Result<String, String>;
    async fn scan_syntax(text: String) -> Result<String, String>;
    async fn extract_relations(text: String, entities_json: String) -> Result<String, String>;
    
    // ResoRank - returns JSON (matches existing)
    async fn resorank_search(query: String, limit: usize) -> Result<String, String>;
    async fn resorank_index(doc_id: String, title: String, content: String) -> Result<bool, String>;
    async fn resorank_clear() -> Result<(), String>;
    async fn resorank_stats() -> Result<String, String>;
    
    // Conductor
    async fn conductor_hydrate(entities_json: String) -> Result<String, String>;
    async fn conductor_status() -> Result<String, String>;
    async fn conductor_reset() -> Result<String, String>;
    
    // Scan Worker
    async fn get_decoration_spans(note_id: String, content_hash: String) -> Result<Option<String>, String>;
    async fn queue_note_scan(note_id: String) -> Result<(), String>;
    async fn scan_queue_status() -> Result<String, String>;
    async fn invalidate_all_decoration_spans() -> Result<usize, String>;
}

// ============================================================================
// Implementation
// ============================================================================

#[derive(Clone, Default)]
pub struct CoreApiImpl;

#[taurpc::resolvers]
impl CoreApi for CoreApiImpl {
    async fn version(self) -> String {
        crate::version()
    }
    
    async fn greet(self, name: String) -> String {
        crate::greet(name)
    }
    
    async fn health(self) -> HealthStatus {
        HealthStatus {
            ok: true,
            version: format!("varant v{}", env!("CARGO_PKG_VERSION")),
        }
    }
    
    async fn unified_scan(self, text: String, entities_json: String) -> Result<String, String> {
        crate::unified_scan(text, entities_json)
    }
    
    async fn hydrate_entities(self, entities_json: String) -> Result<String, String> {
        crate::hydrate_entities(entities_json)
    }
    
    async fn extract_triples(self, text: String) -> Result<String, String> {
        crate::extract_triples(text)
    }
    
    async fn scan_temporal(self, text: String) -> Result<String, String> {
        crate::scan_temporal(text)
    }
    
    async fn scan_syntax(self, text: String) -> Result<String, String> {
        crate::scan_syntax(text)
    }
    
    async fn extract_relations(self, text: String, entities_json: String) -> Result<String, String> {
        crate::extract_relations(text, entities_json)
    }
    
    async fn resorank_search(self, query: String, limit: usize) -> Result<String, String> {
        let results = crate::resorank_search(query, limit)?;
        serde_json::to_string(&results).map_err(|e| e.to_string())
    }
    
    async fn resorank_index(self, doc_id: String, title: String, content: String) -> Result<bool, String> {
        crate::resorank_index(doc_id, title, content)
    }
    
    async fn resorank_clear(self) -> Result<(), String> {
        crate::resorank_clear()
    }
    
    async fn resorank_stats(self) -> Result<String, String> {
        let stats = crate::resorank_stats()?;
        serde_json::to_string(&stats).map_err(|e| e.to_string())
    }
    
    async fn conductor_hydrate(self, entities_json: String) -> Result<String, String> {
        crate::conductor_hydrate(entities_json)
    }
    
    async fn conductor_status(self) -> Result<String, String> {
        crate::conductor_status()
    }
    
    async fn conductor_reset(self) -> Result<String, String> {
        crate::conductor_reset()
    }
    
    async fn get_decoration_spans(self, note_id: String, content_hash: String) -> Result<Option<String>, String> {
        crate::get_decoration_spans(note_id, content_hash)
    }
    
    async fn queue_note_scan(self, note_id: String) -> Result<(), String> {
        crate::queue_note_scan(note_id)?;
        Ok(())
    }
    
    async fn scan_queue_status(self) -> Result<String, String> {
        crate::scan_queue_status()
    }
    
    async fn invalidate_all_decoration_spans(self) -> Result<usize, String> {
        crate::invalidate_all_decoration_spans()
    }
}
