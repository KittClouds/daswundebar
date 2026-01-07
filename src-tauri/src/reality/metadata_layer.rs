//! MetadataLayer - Derived Entity Properties
//!
//! Computes aggregate metadata from the EntityLayer:
//! - Frequency (total mentions)
//! - First mention (where introduced)
//! - Aliases (different surface forms)
//! - Importance score (derived from graph position)
//!
//! This layer is recomputed on demand from the EntityLayer,
//! serving as a materialized view for fast queries.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::entity_layer::{EntityLayer, EntityRecord};

// =============================================================================
// Core Types
// =============================================================================

/// Aggregated metadata for a single entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    /// Entity ID
    pub entity_id: String,
    /// Total number of mentions across all documents
    pub frequency: usize,
    /// Document where entity was first mentioned
    pub first_mention_doc: String,
    /// Byte offset of first mention
    pub first_mention_offset: usize,
    /// Alternative surface forms (aliases) for this entity
    pub aliases: HashSet<String>,
    /// Computed importance score (0.0 - 1.0)
    pub importance: f64,
    /// Documents this entity appears in
    pub documents: HashSet<String>,
}

impl EntityMetadata {
    /// Create new metadata from an entity record
    pub fn from_record(id: &str, record: &EntityRecord) -> Self {
        let first_span = record.spans.first();
        
        let documents: HashSet<String> = record.spans
            .iter()
            .map(|s| s.doc_id.clone())
            .collect();
        
        Self {
            entity_id: id.to_string(),
            frequency: record.spans.len(),
            first_mention_doc: first_span.map(|s| s.doc_id.clone()).unwrap_or_default(),
            first_mention_offset: first_span.map(|s| s.start).unwrap_or(0),
            aliases: HashSet::new(),
            importance: 0.0,
            documents,
        }
    }
    
    /// Check if entity spans multiple documents
    pub fn is_cross_document(&self) -> bool {
        self.documents.len() > 1
    }
    
    /// Add an alias for this entity
    pub fn add_alias(&mut self, alias: String) {
        if alias != self.entity_id {
            self.aliases.insert(alias);
        }
    }
}

// =============================================================================
// MetadataLayer
// =============================================================================

/// Derived metadata layer - computed from EntityLayer
///
/// This is a materialized view that caches computed properties.
/// Call `recompute()` to refresh from EntityLayer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataLayer {
    /// Entity ID -> Metadata
    metadata: HashMap<String, EntityMetadata>,
    /// Last computation timestamp (for staleness checking)
    last_computed_at: Option<u64>,
}

impl MetadataLayer {
    /// Create a new empty MetadataLayer
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Recompute all metadata from EntityLayer
    ///
    /// This is the main entry point. Call after EntityLayer changes.
    pub fn compute_from_entity_layer(&mut self, entity_layer: &EntityLayer) {
        self.metadata.clear();
        
        for (id, record) in entity_layer.iter() {
            if !record.spans.is_empty() {
                self.metadata.insert(
                    id.clone(),
                    EntityMetadata::from_record(id, record),
                );
            }
        }
        
        // Compute importance scores (normalized frequency)
        self.compute_importance_scores();
        
        // Record computation time
        self.last_computed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
    }
    
    /// Compute normalized importance scores
    fn compute_importance_scores(&mut self) {
        if self.metadata.is_empty() {
            return;
        }
        
        let max_freq = self.metadata
            .values()
            .map(|m| m.frequency)
            .max()
            .unwrap_or(1) as f64;
        
        for meta in self.metadata.values_mut() {
            // Simple importance = normalized frequency
            // Can be enhanced with graph centrality in future
            meta.importance = (meta.frequency as f64) / max_freq;
        }
    }
    
    /// Get metadata for an entity
    pub fn get(&self, entity_id: &str) -> Option<&EntityMetadata> {
        self.metadata.get(entity_id)
    }
    
    /// Get mutable metadata for an entity
    pub fn get_mut(&mut self, entity_id: &str) -> Option<&mut EntityMetadata> {
        self.metadata.get_mut(entity_id)
    }
    
    /// Iterate over all metadata
    pub fn iter(&self) -> impl Iterator<Item = (&String, &EntityMetadata)> {
        self.metadata.iter()
    }
    
    /// Get entities sorted by frequency (descending)
    pub fn by_frequency(&self) -> Vec<&EntityMetadata> {
        let mut sorted: Vec<_> = self.metadata.values().collect();
        sorted.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        sorted
    }
    
    /// Get entities sorted by importance (descending)
    pub fn by_importance(&self) -> Vec<&EntityMetadata> {
        let mut sorted: Vec<_> = self.metadata.values().collect();
        sorted.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));
        sorted
    }
    
    /// Get entities that appear in multiple documents
    pub fn cross_document_entities(&self) -> Vec<&EntityMetadata> {
        self.metadata
            .values()
            .filter(|m| m.is_cross_document())
            .collect()
    }
    
    /// Get entities from a specific document
    pub fn entities_in_doc(&self, doc_id: &str) -> Vec<&EntityMetadata> {
        self.metadata
            .values()
            .filter(|m| m.documents.contains(doc_id))
            .collect()
    }
    
    /// Total number of entities with metadata
    pub fn len(&self) -> usize {
        self.metadata.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }
    
    /// Clear all metadata
    pub fn clear(&mut self) {
        self.metadata.clear();
        self.last_computed_at = None;
    }
    
    /// Add an alias for an entity
    pub fn add_alias(&mut self, entity_id: &str, alias: &str) {
        if let Some(meta) = self.metadata.get_mut(entity_id) {
            meta.add_alias(alias.to_string());
        }
    }
    
    /// Check if metadata is stale (older than given seconds)
    pub fn is_stale(&self, max_age_secs: u64) -> bool {
        match self.last_computed_at {
            None => true,
            Some(computed) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                now.saturating_sub(computed) > max_age_secs
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reality::entity_layer::EntityLayer;

    #[test]
    fn test_metadata_layer_basic() {
        let mut entity_layer = EntityLayer::new();
        entity_layer.record_span("Frodo", "CHARACTER", 0, 5, "doc1", "Frodo owns the Ring.");
        entity_layer.record_span("Ring", "ITEM", 16, 20, "doc1", "Frodo owns the Ring.");
        
        let mut metadata_layer = MetadataLayer::new();
        metadata_layer.compute_from_entity_layer(&entity_layer);
        
        assert_eq!(metadata_layer.len(), 2);
        
        let frodo = metadata_layer.get("frodo").unwrap();
        assert_eq!(frodo.frequency, 1);
        assert_eq!(frodo.first_mention_doc, "doc1");
    }

    #[test]
    fn test_metadata_frequency() {
        let mut entity_layer = EntityLayer::new();
        entity_layer.record_span("Frodo", "CHARACTER", 0, 5, "doc1", "Frodo");
        entity_layer.record_span("Frodo", "CHARACTER", 10, 15, "doc1", "Frodo again");
        entity_layer.record_span("Sam", "CHARACTER", 0, 3, "doc1", "Sam");
        
        let mut metadata_layer = MetadataLayer::new();
        metadata_layer.compute_from_entity_layer(&entity_layer);
        
        let frodo = metadata_layer.get("frodo").unwrap();
        assert_eq!(frodo.frequency, 2);
        
        let sam = metadata_layer.get("sam").unwrap();
        assert_eq!(sam.frequency, 1);
        
        // Frodo should be more important
        assert!(frodo.importance > sam.importance);
    }

    #[test]
    fn test_metadata_cross_document() {
        let mut entity_layer = EntityLayer::new();
        entity_layer.record_span("Frodo", "CHARACTER", 0, 5, "chapter1", "Frodo");
        entity_layer.record_span("Frodo", "CHARACTER", 0, 5, "chapter2", "Frodo");
        entity_layer.record_span("Sam", "CHARACTER", 0, 3, "chapter1", "Sam");
        
        let mut metadata_layer = MetadataLayer::new();
        metadata_layer.compute_from_entity_layer(&entity_layer);
        
        let frodo = metadata_layer.get("frodo").unwrap();
        assert!(frodo.is_cross_document());
        assert_eq!(frodo.documents.len(), 2);
        
        let sam = metadata_layer.get("sam").unwrap();
        assert!(!sam.is_cross_document());
        
        // Cross-document query
        let cross_doc = metadata_layer.cross_document_entities();
        assert_eq!(cross_doc.len(), 1);
        assert_eq!(cross_doc[0].entity_id, "frodo");
    }

    #[test]
    fn test_metadata_by_frequency() {
        let mut entity_layer = EntityLayer::new();
        for _ in 0..5 { entity_layer.record_span("Frodo", "CHARACTER", 0, 5, "doc", "Frodo"); }
        for _ in 0..3 { entity_layer.record_span("Sam", "CHARACTER", 0, 3, "doc", "Sam"); }
        for _ in 0..1 { entity_layer.record_span("Gandalf", "CHARACTER", 0, 7, "doc", "Gandalf"); }
        
        let mut metadata_layer = MetadataLayer::new();
        metadata_layer.compute_from_entity_layer(&entity_layer);
        
        let sorted = metadata_layer.by_frequency();
        assert_eq!(sorted[0].entity_id, "frodo");
        assert_eq!(sorted[1].entity_id, "sam");
        assert_eq!(sorted[2].entity_id, "gandalf");
    }

    #[test]
    fn test_metadata_aliases() {
        let mut entity_layer = EntityLayer::new();
        entity_layer.record_span("Frodo", "CHARACTER", 0, 5, "doc", "Frodo");
        
        let mut metadata_layer = MetadataLayer::new();
        metadata_layer.compute_from_entity_layer(&entity_layer);
        
        metadata_layer.add_alias("frodo", "Mr. Frodo");
        metadata_layer.add_alias("frodo", "Ringbearer");
        
        let frodo = metadata_layer.get("frodo").unwrap();
        assert_eq!(frodo.aliases.len(), 2);
        assert!(frodo.aliases.contains("Mr. Frodo"));
        assert!(frodo.aliases.contains("Ringbearer"));
    }

    #[test]
    fn test_metadata_entities_in_doc() {
        let mut entity_layer = EntityLayer::new();
        entity_layer.record_span("Frodo", "CHARACTER", 0, 5, "chapter1", "Frodo");
        entity_layer.record_span("Sam", "CHARACTER", 0, 3, "chapter1", "Sam");
        entity_layer.record_span("Gandalf", "CHARACTER", 0, 7, "chapter2", "Gandalf");
        
        let mut metadata_layer = MetadataLayer::new();
        metadata_layer.compute_from_entity_layer(&entity_layer);
        
        let ch1_entities = metadata_layer.entities_in_doc("chapter1");
        assert_eq!(ch1_entities.len(), 2);
        
        let ch2_entities = metadata_layer.entities_in_doc("chapter2");
        assert_eq!(ch2_entities.len(), 1);
        assert_eq!(ch2_entities[0].entity_id, "gandalf");
    }
}
