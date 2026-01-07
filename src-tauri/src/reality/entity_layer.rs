//! EntityLayer - Lossless Span Storage
//!
//! Stores ALL entity spans from Scanner without deduplication.
//! This is the "source of truth" layer that preserves every mention.
//!
//! # Design
//!
//! | Field       | Purpose                                    |
//! |-------------|-------------------------------------------|
//! | id          | Normalized entity ID (e.g., "frodo")      |
//! | label       | Original text ("Frodo")                   |
//! | kind        | Entity type (CHARACTER, LOCATION, etc.)   |
//! | spans       | ALL occurrences with context              |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Core Types
// =============================================================================

/// Record of a single span occurrence in a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanRecord {
    /// Document ID where this span occurs
    pub doc_id: String,
    /// Start byte offset in document
    pub start: usize,
    /// End byte offset in document
    pub end: usize,
    /// Surrounding context (for disambiguation/display)
    pub context: String,
}

/// All information about a single entity across documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRecord {
    /// Normalized entity ID (lowercase, underscores)
    pub id: String,
    /// Original label as written
    pub label: String,
    /// Entity kind (CHARACTER, LOCATION, etc.)
    pub kind: String,
    /// All span occurrences (lossless - not deduplicated)
    pub spans: Vec<SpanRecord>,
}

impl EntityRecord {
    /// Create a new entity record
    pub fn new(id: String, label: String, kind: String) -> Self {
        Self {
            id,
            label,
            kind,
            spans: Vec::new(),
        }
    }
    
    /// Total number of mentions
    pub fn frequency(&self) -> usize {
        self.spans.len()
    }
    
    /// Get the first mention (if any)
    pub fn first_mention(&self) -> Option<&SpanRecord> {
        self.spans.first()
    }
}

// =============================================================================
// EntityLayer
// =============================================================================

/// Lossless storage for all entity spans
///
/// Unlike the graph which deduplicates nodes, this layer stores
/// EVERY span occurrence for complete provenance tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityLayer {
    /// Entity ID -> EntityRecord (with all spans)
    entities: HashMap<String, EntityRecord>,
    /// Total spans recorded (for stats)
    total_spans: usize,
}

impl EntityLayer {
    /// Create a new empty EntityLayer
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Record a span occurrence
    ///
    /// This is the main entry point. It:
    /// 1. Normalizes the entity ID
    /// 2. Creates or updates the EntityRecord
    /// 3. Appends the new SpanRecord (never deduplicated)
    pub fn record_span(
        &mut self,
        label: &str,
        kind: &str,
        start: usize,
        end: usize,
        doc_id: &str,
        text: &str,
    ) {
        let id = normalize_id(label);
        
        let record = self.entities.entry(id.clone()).or_insert_with(|| {
            EntityRecord::new(id, label.to_string(), kind.to_string())
        });
        
        record.spans.push(SpanRecord {
            doc_id: doc_id.to_string(),
            start,
            end,
            context: extract_context(text, start, end),
        });
        
        self.total_spans += 1;
    }
    
    /// Get number of unique entities
    pub fn unique_entities(&self) -> usize {
        self.entities.len()
    }
    
    /// Get total number of spans recorded
    pub fn total_spans(&self) -> usize {
        self.total_spans
    }
    
    /// Get all spans for an entity
    pub fn all_spans_for(&self, entity_id: &str) -> &[SpanRecord] {
        self.entities.get(entity_id)
            .map(|e| e.spans.as_slice())
            .unwrap_or(&[])
    }
    
    /// Get an entity record by ID
    pub fn get_entity(&self, entity_id: &str) -> Option<&EntityRecord> {
        self.entities.get(entity_id)
    }
    
    /// Iterate over all entities
    pub fn iter(&self) -> impl Iterator<Item = (&String, &EntityRecord)> {
        self.entities.iter()
    }
    
    /// Get all entity IDs
    pub fn entity_ids(&self) -> impl Iterator<Item = &String> {
        self.entities.keys()
    }
    
    /// Clear all data
    pub fn clear(&mut self) {
        self.entities.clear();
        self.total_spans = 0;
    }
    
    /// Clear spans for a specific document (for re-processing)
    pub fn clear_doc(&mut self, doc_id: &str) {
        let mut spans_removed = 0;
        
        for record in self.entities.values_mut() {
            let before = record.spans.len();
            record.spans.retain(|s| s.doc_id != doc_id);
            spans_removed += before - record.spans.len();
        }
        
        // Remove entities with no remaining spans
        self.entities.retain(|_, r| !r.spans.is_empty());
        
        self.total_spans = self.total_spans.saturating_sub(spans_removed);
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Normalize entity ID (lowercase, replace spaces with underscores)
pub fn normalize_id(label: &str) -> String {
    label.trim().to_lowercase().replace(' ', "_")
}

/// Extract surrounding context for a span
fn extract_context(text: &str, start: usize, end: usize) -> String {
    const CONTEXT_SIZE: usize = 50;
    
    // Find safe boundaries (don't split UTF-8)
    let context_start = start.saturating_sub(CONTEXT_SIZE);
    let context_end = (end + CONTEXT_SIZE).min(text.len());
    
    // Adjust to char boundaries (stable implementation)
    let safe_start = floor_char_boundary(text, context_start);
    let safe_end = ceil_char_boundary(text, context_end);
    
    text[safe_start..safe_end].to_string()
}

/// Find the largest byte offset <= pos that is a char boundary (stable version)
fn floor_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }
    let mut i = pos;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Find the smallest byte offset >= pos that is a char boundary (stable version)
fn ceil_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }
    let mut i = pos;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_layer_basic() {
        let mut layer = EntityLayer::new();
        
        layer.record_span("Frodo", "CHARACTER", 0, 5, "doc1", "Frodo owns the Ring.");
        
        assert_eq!(layer.unique_entities(), 1);
        assert_eq!(layer.total_spans(), 1);
        
        let spans = layer.all_spans_for("frodo");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 5);
    }

    #[test]
    fn test_entity_layer_multiple_mentions() {
        let mut layer = EntityLayer::new();
        
        let text = "Frodo owns the Ring. Sam helps Frodo.";
        layer.record_span("Frodo", "CHARACTER", 0, 5, "doc1", text);
        layer.record_span("Ring", "ITEM", 16, 20, "doc1", text);
        layer.record_span("Sam", "CHARACTER", 22, 25, "doc1", text);
        layer.record_span("Frodo", "CHARACTER", 32, 37, "doc1", text);
        
        // 3 unique entities, 4 total spans
        assert_eq!(layer.unique_entities(), 3);
        assert_eq!(layer.total_spans(), 4);
        
        // Frodo has 2 mentions
        let frodo_spans = layer.all_spans_for("frodo");
        assert_eq!(frodo_spans.len(), 2);
        assert_eq!(frodo_spans[0].start, 0);
        assert_eq!(frodo_spans[1].start, 32);
    }

    #[test]
    fn test_entity_layer_cross_document() {
        let mut layer = EntityLayer::new();
        
        layer.record_span("Frodo", "CHARACTER", 0, 5, "chapter1", "Frodo left.");
        layer.record_span("Frodo", "CHARACTER", 0, 5, "chapter2", "Frodo returned.");
        
        assert_eq!(layer.unique_entities(), 1);
        assert_eq!(layer.total_spans(), 2);
        
        let frodo = layer.get_entity("frodo").unwrap();
        assert_eq!(frodo.spans[0].doc_id, "chapter1");
        assert_eq!(frodo.spans[1].doc_id, "chapter2");
    }

    #[test]
    fn test_entity_layer_clear_doc() {
        let mut layer = EntityLayer::new();
        
        layer.record_span("Frodo", "CHARACTER", 0, 5, "doc1", "Frodo");
        layer.record_span("Frodo", "CHARACTER", 0, 5, "doc2", "Frodo");
        layer.record_span("Sam", "CHARACTER", 0, 3, "doc1", "Sam");
        
        assert_eq!(layer.total_spans(), 3);
        
        // Clear doc1
        layer.clear_doc("doc1");
        
        // Frodo still exists (doc2), Sam removed
        assert_eq!(layer.unique_entities(), 1);
        assert_eq!(layer.total_spans(), 1);
        assert!(layer.get_entity("sam").is_none());
    }

    #[test]
    fn test_normalize_id() {
        assert_eq!(normalize_id("Frodo Baggins"), "frodo_baggins");
        assert_eq!(normalize_id("  GANDALF  "), "gandalf");
        assert_eq!(normalize_id("The One Ring"), "the_one_ring");
    }

    #[test]
    fn test_extract_context() {
        let text = "In a hole in the ground there lived a hobbit.";
        let context = extract_context(text, 20, 26); // "ground"
        
        // Should include surrounding text
        assert!(context.contains("ground"));
        assert!(context.len() > 6);
    }
}
