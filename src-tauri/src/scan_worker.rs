//! Scan Worker - Background thread for note scanning
//!
//! # Architecture
//!
//! ```text
//! NoteRepo::update() ──┬──► ScanQueue ──► ScanWorker Thread
//!                      │                       │
//!                      │                       ▼
//!                      │              ┌─────────────────┐
//!                      │              │ 1. Read note    │
//!                      │              │ 2. Run scanner  │
//!                      │              │ 3. Cache spans  │
//!                      │              └─────────────────┘
//!                      │                       │
//!                      └───────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! // Start worker (usually in app initialization)
//! let queue = Arc::new(ScanQueue::new());
//! let worker = ScanWorker::start(queue.clone(), process_note_scan);
//!
//! // Queue scans (from NoteRepo::update)
//! queue.push(note_id);
//!
//! // On shutdown
//! worker.stop();
//! ```

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cozo::DbInstance;
use serde::{Deserialize, Serialize};

// =============================================================================
// SCAN QUEUE - Thread-safe, deduplicated FIFO
// =============================================================================

/// Thread-safe queue for note IDs to scan
/// 
/// Features:
/// - FIFO ordering
/// - Deduplication (same note ID won't be queued twice)
pub struct ScanQueue {
    /// Ordered list of note IDs to process
    queue: Mutex<Vec<String>>,
    /// Set for O(1) dedup checks
    pending: Mutex<HashSet<String>>,
}

impl ScanQueue {
    /// Create a new empty queue
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            pending: Mutex::new(HashSet::new()),
        }
    }

    /// Push a note ID to the queue (deduplicated)
    pub fn push(&self, note_id: String) {
        let mut pending = self.pending.lock().unwrap();
        
        // Skip if already queued
        if pending.contains(&note_id) {
            return;
        }
        
        pending.insert(note_id.clone());
        drop(pending);
        
        let mut queue = self.queue.lock().unwrap();
        queue.push(note_id);
    }

    /// Pop the next note ID from the queue
    pub fn pop(&self) -> Option<String> {
        let mut queue = self.queue.lock().unwrap();
        
        if queue.is_empty() {
            return None;
        }
        
        let note_id = queue.remove(0);
        drop(queue);
        
        let mut pending = self.pending.lock().unwrap();
        pending.remove(&note_id);
        
        Some(note_id)
    }

    /// Get the number of items in the queue
    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ScanQueue {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// DECORATION SPAN RECORDS
// =============================================================================

/// A single decoration span for highlighting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecorationSpanRecord {
    pub start: u32,
    pub end: u32,
    pub kind: String,
    pub entity_id: Option<String>,
    pub label: String,
    pub styling: Option<String>,
}

// =============================================================================
// DECORATION CACHE - CozoDB persistence
// =============================================================================

/// Cache for decoration spans in CozoDB
pub struct DecorationCache;

impl DecorationCache {
    /// Persist decoration spans for a note
    pub fn persist(
        db: &DbInstance,
        note_id: &str,
        content_hash: &str,
        spans: &[DecorationSpanRecord],
    ) -> Result<(), String> {
        use std::collections::BTreeMap;
        use cozo::{DataValue, ScriptMutability};

        // First, delete any existing cache for this note
        let delete_query = r#"
            ?[note_id] <- [[$note_id]]
            :rm decoration_spans { note_id }
        "#;
        
        let mut delete_params = BTreeMap::new();
        delete_params.insert("note_id".to_string(), DataValue::Str(note_id.into()));
        
        let _ = db.run_script(delete_query, delete_params, ScriptMutability::Mutable);
        
        // Serialize spans to JSON
        let spans_json = serde_json::to_string(spans)
            .map_err(|e| format!("Failed to serialize spans: {}", e))?;
        
        let now = chrono::Utc::now().timestamp() as f64;
        
        // Insert new cache entry
        let insert_query = r#"
            ?[note_id, content_hash, spans_json, created_at] <- [[
                $note_id, $content_hash, $spans_json, $created_at
            ]]
            :put decoration_spans { note_id, content_hash, spans_json, created_at }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("note_id".to_string(), DataValue::Str(note_id.into()));
        params.insert("content_hash".to_string(), DataValue::Str(content_hash.into()));
        params.insert("spans_json".to_string(), DataValue::Str(spans_json.into()));
        params.insert("created_at".to_string(), DataValue::from(now));
        
        db.run_script(insert_query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to persist decoration cache: {}", e))?;
        
        Ok(())
    }

    /// Get cached decoration spans for a note
    pub fn get(
        db: &DbInstance,
        note_id: &str,
        content_hash: &str,
    ) -> Result<Option<Vec<DecorationSpanRecord>>, String> {
        use std::collections::BTreeMap;
        use cozo::{DataValue, ScriptMutability};

        let query = r#"
            ?[spans_json] := 
                *decoration_spans[note_id, content_hash, spans_json, _],
                note_id = $note_id,
                content_hash = $content_hash
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("note_id".to_string(), DataValue::Str(note_id.into()));
        params.insert("content_hash".to_string(), DataValue::Str(content_hash.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to query decoration cache: {}", e))?;
        
        if result.rows.is_empty() {
            return Ok(None);
        }
        
        let spans_json = match &result.rows[0][0] {
            DataValue::Str(s) => s.to_string(),
            _ => return Err("Invalid spans_json type".to_string()),
        };
        
        let spans: Vec<DecorationSpanRecord> = serde_json::from_str(&spans_json)
            .map_err(|e| format!("Failed to deserialize spans: {}", e))?;
        
        Ok(Some(spans))
    }

    /// Delete cached spans for a note
    pub fn invalidate(db: &DbInstance, note_id: &str) -> Result<(), String> {
        use std::collections::BTreeMap;
        use cozo::{DataValue, ScriptMutability};

        let query = r#"
            ?[note_id] <- [[$note_id]]
            :rm decoration_spans { note_id }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("note_id".to_string(), DataValue::Str(note_id.into()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to invalidate decoration cache: {}", e))?;
        
        Ok(())
    }

    /// Clear ALL cached decoration spans (used on entity hydration)
    pub fn clear_all(db: &DbInstance) -> Result<usize, String> {
        use std::collections::BTreeMap;
        use cozo::ScriptMutability;

        // First count how many we're deleting
        let count_query = r#"
            ?[count(note_id)] := *decoration_spans[note_id, _, _, _]
        "#;
        
        let count_result = db.run_script(count_query, BTreeMap::new(), ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to count decoration cache: {}", e))?;
        
        let count = count_result.rows.first()
            .and_then(|row| row.first())
            .and_then(|v| match v {
                cozo::DataValue::Num(cozo::Num::Int(i)) => Some(*i as usize),
                _ => None,
            })
            .unwrap_or(0);

        // Delete all entries
        let delete_query = r#"
            ?[note_id] := *decoration_spans[note_id, _, _, _]
            :rm decoration_spans { note_id }
        "#;
        
        db.run_script(delete_query, BTreeMap::new(), ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to clear decoration cache: {}", e))?;
        
        log::info!("[DecorationCache] Cleared {} cached decoration spans", count);
        Ok(count)
    }
}

// =============================================================================
// SCAN WORKER - Background processing thread
// =============================================================================

/// Handle to the background scan worker thread
pub struct ScanWorker {
    /// Thread handle
    handle: Option<JoinHandle<()>>,
    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
}

impl ScanWorker {
    /// Start a new scan worker thread
    ///
    /// # Arguments
    /// * `queue` - The scan queue to process
    /// * `processor` - Function to call for each note ID
    pub fn start<F>(queue: Arc<ScanQueue>, processor: F) -> Self
    where
        F: Fn(&str) + Send + 'static,
    {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let handle = thread::spawn(move || {
            log::info!("[ScanWorker] Started");
            
            while !shutdown_clone.load(Ordering::SeqCst) {
                if let Some(note_id) = queue.pop() {
                    log::debug!("[ScanWorker] Processing note: {}", note_id);
                    processor(&note_id);
                } else {
                    // No work, sleep briefly
                    thread::sleep(Duration::from_millis(50));
                }
            }
            
            log::info!("[ScanWorker] Shutdown complete");
        });

        Self {
            handle: Some(handle),
            shutdown,
        }
    }

    /// Check if worker is running
    pub fn is_running(&self) -> bool {
        self.handle.as_ref().map_or(false, |h| !h.is_finished())
    }

    /// Stop the worker and wait for it to finish
    pub fn stop(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

// =============================================================================
// CONTENT HASH
// =============================================================================

/// Compute a hash of content for cache invalidation
pub fn compute_content_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

// =============================================================================
// NOTE SCAN PROCESSOR
// =============================================================================

/// Process a single note scan
/// 
/// This is the main processing function called by the worker:
/// 1. Read note from CozoDB
/// 2. Run scanner pipeline
/// 3. Persist decoration spans
/// 4. Persist graph data (nodes + edges)
pub fn process_note_scan(db: &DbInstance, note_id: &str) {
    log::debug!("[ScanWorker] Processing note: {}", note_id);
    
    // 1. Read note from CozoDB
    let note = match crate::graph::content_repos::NoteRepo::get(db, "default", note_id) {
        Ok(Some(note)) => note,
        Ok(None) => {
            log::warn!("[ScanWorker] Note not found: {}", note_id);
            return;
        }
        Err(e) => {
            log::error!("[ScanWorker] Failed to read note {}: {}", note_id, e);
            return;
        }
    };
    
    // 2. Run scanner pipeline
    let mut conductor = crate::CONDUCTOR.lock();
    let result = match conductor.scan(&note.content, &[]) {
        Some(result) => result,
        None => {
            log::warn!("[ScanWorker] Conductor not ready, skipping note: {}", note_id);
            return;
        }
    };
    
    // 3. Convert scan result to decoration spans
    let mut spans = Vec::new();
    
    // Add implicit entities
    for entity in &result.implicit {
        spans.push(DecorationSpanRecord {
            start: entity.start as u32,
            end: entity.end as u32,
            kind: "implicit".to_string(),
            entity_id: Some(entity.entity_id.clone()),
            label: entity.entity_label.clone(),
            styling: None,
        });
    }
    
    // Add unified spans (from scanner module, not unified_relations)
    // unified_relations is for CST inference, we want the scanner spans here
    for span in &result.triples {
        spans.push(DecorationSpanRecord {
            start: span.start as u32,
            end: span.end as u32,
            kind: "triple".to_string(),
            entity_id: None,
            label: span.raw_text.clone(),
            styling: None,
        });
    }
    
    // Add temporal spans
    for temporal in &result.temporal {
        spans.push(DecorationSpanRecord {
            start: temporal.start as u32,
            end: temporal.end as u32,
            kind: "temporal".to_string(),
            entity_id: None,
            label: temporal.text.clone(),
            styling: None,
        });
    }
    
    // 4. Persist decoration spans
    let content_hash = compute_content_hash(&note.content);
    if let Err(e) = DecorationCache::persist(db, note_id, &content_hash, &spans) {
        log::error!("[ScanWorker] Failed to cache decorations for {}: {}", note_id, e);
    } else {
        log::debug!(
            "[ScanWorker] Cached {} decoration spans for note {}",
            spans.len(),
            note_id
        );
    }
    
    // 5. Persist graph data (nodes + edges) using CozoGraph
    persist_graph_data(db, note_id, &result);
}

/// Persist extracted entities and relationships to the graph
fn persist_graph_data(db: &DbInstance, note_id: &str, result: &crate::document::ScanResult) {
    use crate::graph::cozo_graph::{CozoGraph, NodeData, EdgeData};
    use std::collections::HashSet;
    
    let graph = CozoGraph::new(db);
    let mut nodes_created = 0usize;
    let mut edges_created = 0usize;
    let mut seen_nodes: HashSet<String> = HashSet::new();
    
    // Extract entities from triples and ensure they exist as nodes
    for triple in &result.triples {
        // Source entity
        if !seen_nodes.contains(&triple.source) {
            let source_kind = triple.source_kind.as_deref().unwrap_or("ENTITY");
            let node = NodeData {
                id: generate_node_id(&triple.source),
                label: triple.source.clone(),
                normalized: triple.source.trim().to_lowercase(),
                kind: source_kind.to_string(),
                subtype: None,
                source_note: note_id.to_string(),
                metadata: None,
            };
            if let Err(e) = graph.ensure_node(&node) {
                log::warn!("[ScanWorker] Failed to persist source node '{}': {}", triple.source, e);
            } else {
                nodes_created += 1;
            }
            seen_nodes.insert(triple.source.clone());
        }
        
        // Target entity
        if !seen_nodes.contains(&triple.target) {
            let target_kind = triple.target_kind.as_deref().unwrap_or("ENTITY");
            let node = NodeData {
                id: generate_node_id(&triple.target),
                label: triple.target.clone(),
                normalized: triple.target.trim().to_lowercase(),
                kind: target_kind.to_string(),
                subtype: None,
                source_note: note_id.to_string(),
                metadata: None,
            };
            if let Err(e) = graph.ensure_node(&node) {
                log::warn!("[ScanWorker] Failed to persist target node '{}': {}", triple.target, e);
            } else {
                nodes_created += 1;
            }
            seen_nodes.insert(triple.target.clone());
        }
        
        // Create edge
        let edge = EdgeData {
            id: format!("{}:{}:{}", generate_node_id(&triple.source), triple.predicate, generate_node_id(&triple.target)),
            source_id: generate_node_id(&triple.source),
            target_id: generate_node_id(&triple.target),
            edge_type: triple.predicate.clone(),
            weight: 1.0,
            confidence: 1.0,
            source_note: Some(note_id.to_string()),
            metadata: None,
        };
        if let Err(e) = graph.add_edge(&edge) {
            log::warn!("[ScanWorker] Failed to persist edge '{}->{}': {}", triple.source, triple.target, e);
        } else {
            edges_created += 1;
        }
    }
    
    // Also persist unified_relations as edges (these are CST-inferred)
    for rel in &result.unified_relations {
        // Ensure head and tail nodes exist
        if !seen_nodes.contains(&rel.head) {
            let node = NodeData {
                id: generate_node_id(&rel.head),
                label: rel.head.clone(),
                normalized: rel.head.trim().to_lowercase(),
                kind: "ENTITY".to_string(),
                subtype: None,
                source_note: note_id.to_string(),
                metadata: None,
            };
            let _ = graph.ensure_node(&node);
            seen_nodes.insert(rel.head.clone());
        }
        
        if !seen_nodes.contains(&rel.tail) {
            let node = NodeData {
                id: generate_node_id(&rel.tail),
                label: rel.tail.clone(),
                normalized: rel.tail.trim().to_lowercase(),
                kind: "ENTITY".to_string(),
                subtype: None,
                source_note: note_id.to_string(),
                metadata: None,
            };
            let _ = graph.ensure_node(&node);
            seen_nodes.insert(rel.tail.clone());
        }
        
        // Create edge for unified relation
        let edge = EdgeData {
            id: format!("{}:{}:{}", generate_node_id(&rel.head), rel.relation_type, generate_node_id(&rel.tail)),
            source_id: generate_node_id(&rel.head),
            target_id: generate_node_id(&rel.tail),
            edge_type: rel.relation_type.clone(),
            weight: rel.confidence as f64,
            confidence: rel.confidence as f64,
            source_note: Some(note_id.to_string()),
            metadata: None,
        };
        let _ = graph.add_edge(&edge);
    }
    
    if nodes_created > 0 || edges_created > 0 {
        log::debug!(
            "[ScanWorker] Persisted {} nodes, {} edges for note {}",
            nodes_created,
            edges_created,
            note_id
        );
    }
}

/// Generate a deterministic node ID from a label
fn generate_node_id(label: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let normalized = label.trim().to_lowercase();
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    format!("node_{:016x}", hasher.finish())
}

// =============================================================================
// HELPER: Create test DB
// =============================================================================

#[cfg(test)]
pub fn create_test_db() -> DbInstance {
    let db = DbInstance::new("mem", "", Default::default())
        .expect("Failed to create test DB");
    
    // Initialize schema
    crate::graph::schema::init_schema(&db)
        .expect("Failed to initialize schema");
    
    // Initialize decoration_spans relation
    let query = r#"
        :create decoration_spans {
            note_id: String,
            content_hash: String,
            spans_json: String,
            created_at: Float
        }
    "#;
    
    let _ = db.run_script(query, Default::default(), cozo::ScriptMutability::Mutable);
    
    db
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_push_pop_basic() {
        let queue = ScanQueue::new();
        
        queue.push("note_001".to_string());
        queue.push("note_002".to_string());
        
        assert_eq!(queue.pop(), Some("note_001".to_string()));
        assert_eq!(queue.pop(), Some("note_002".to_string()));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_queue_deduplication() {
        let queue = ScanQueue::new();
        
        queue.push("note_001".to_string());
        queue.push("note_001".to_string()); // Duplicate
        queue.push("note_002".to_string());
        queue.push("note_001".to_string()); // Another duplicate
        
        // Should only get each ID once (but order preserved for first occurrence)
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop(), Some("note_001".to_string()));
        assert_eq!(queue.pop(), Some("note_002".to_string()));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_queue_length() {
        let queue = ScanQueue::new();
        
        assert_eq!(queue.len(), 0);
        queue.push("note_001".to_string());
        assert_eq!(queue.len(), 1);
        queue.push("note_002".to_string());
        assert_eq!(queue.len(), 2);
        queue.pop();
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_content_hash() {
        let hash1 = compute_content_hash("Hello, World!");
        let hash2 = compute_content_hash("Hello, World!");
        let hash3 = compute_content_hash("Different content");
        
        assert_eq!(hash1, hash2, "Same content should have same hash");
        assert_ne!(hash1, hash3, "Different content should have different hash");
    }

    #[test]
    fn test_worker_processes_queue() {
        use std::sync::atomic::AtomicUsize;
        
        let queue = Arc::new(ScanQueue::new());
        let processed = Arc::new(AtomicUsize::new(0));
        let processed_clone = processed.clone();
        
        let processor = move |_note_id: &str| {
            processed_clone.fetch_add(1, Ordering::SeqCst);
        };
        
        let worker = ScanWorker::start(queue.clone(), processor);
        
        queue.push("note_001".to_string());
        queue.push("note_002".to_string());
        queue.push("note_003".to_string());
        
        std::thread::sleep(Duration::from_millis(200));
        
        assert_eq!(processed.load(Ordering::SeqCst), 3);
        
        worker.stop();
    }

    #[test]
    fn test_worker_graceful_shutdown() {
        let queue = Arc::new(ScanQueue::new());
        let processor = |_note_id: &str| {};
        
        let worker = ScanWorker::start(queue.clone(), processor);
        
        assert!(worker.is_running());
        worker.stop();
        // Should complete without hanging
    }
}
