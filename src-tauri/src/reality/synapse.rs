//! Synapse Bridge: Bidirectional mapping between text spans and graph nodes
//!
//! This module provides the critical bridge between the CST (Rowan) and
//! the semantic graph (petgraph). It allows:
//! - Clicking a graph node → highlight all text occurrences
//! - Clicking text → navigate to graph node
//! - Incremental updates when text changes

use rowan::TextRange;
use rustworkx_core::petgraph::graph::NodeIndex;
use std::collections::HashMap;

// =============================================================================
// SynapseBridge
// =============================================================================

/// Bidirectional map: Text spans ↔ Graph nodes
/// 
/// This is the "secret sauce" that connects the document view to the graph view.
/// Each text span (a range of characters) can be linked to a graph node.
/// Each graph node can have multiple text occurrences (same entity mentioned
/// multiple times in the document).
#[derive(Debug, Default)]
pub struct SynapseBridge {
    /// TextRange → (entity_id, node_index)
    /// Allows: "What entity is at this text position?"
    span_to_node: HashMap<TextRange, (String, NodeIndex)>,
    
    /// entity_id → all text occurrences
    /// Allows: "Where does this entity appear in the text?"
    node_to_spans: HashMap<String, Vec<TextRange>>,
}

impl SynapseBridge {
    /// Create a new empty synapse bridge
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Register a link between a text span and a graph node
    /// 
    /// # Arguments
    /// * `range` - The text range (start..end in bytes)
    /// * `entity_id` - The entity ID (matches graph node ID)
    /// * `index` - The petgraph NodeIndex
    pub fn link(&mut self, range: TextRange, entity_id: String, index: NodeIndex) {
        // Forward map: span → node
        self.span_to_node.insert(range, (entity_id.clone(), index));
        
        // Reverse map: node → spans
        self.node_to_spans
            .entry(entity_id)
            .or_default()
            .push(range);
    }
    
    /// Link using u32 offsets (convenience for WASM interop)
    pub fn link_offsets(&mut self, start: u32, end: u32, entity_id: String, index: NodeIndex) {
        let range = TextRange::new(start.into(), end.into());
        self.link(range, entity_id, index);
    }
    
    /// Clear all links (for re-scan)
    pub fn clear(&mut self) {
        self.span_to_node.clear();
        self.node_to_spans.clear();
    }
    
    /// Get node info from an exact text range
    pub fn node_for_range(&self, range: TextRange) -> Option<(&str, NodeIndex)> {
        self.span_to_node
            .get(&range)
            .map(|(id, idx)| (id.as_str(), *idx))
    }
    
    /// Get node info from any text position (offset in bytes)
    /// 
    /// Finds the first span that contains this offset.
    pub fn node_at(&self, offset: u32) -> Option<(&str, NodeIndex)> {
        let offset_pos = rowan::TextSize::from(offset);
        
        for (range, (entity_id, node_index)) in &self.span_to_node {
            if range.contains(offset_pos) {
                return Some((entity_id.as_str(), *node_index));
            }
        }
        None
    }
    
    /// Get all text occurrences of an entity
    /// 
    /// Returns empty slice if entity not found.
    pub fn spans_of(&self, entity_id: &str) -> &[TextRange] {
        self.node_to_spans
            .get(entity_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
    
    /// Get all spans as (start, end) tuples for an entity (WASM-friendly)
    pub fn span_offsets_of(&self, entity_id: &str) -> Vec<(u32, u32)> {
        self.spans_of(entity_id)
            .iter()
            .map(|r| (r.start().into(), r.end().into()))
            .collect()
    }
    
    /// Check if a span is linked
    pub fn contains_range(&self, range: TextRange) -> bool {
        self.span_to_node.contains_key(&range)
    }
    
    /// Check if an entity has any linked spans
    pub fn contains_entity(&self, entity_id: &str) -> bool {
        self.node_to_spans.contains_key(entity_id)
    }
    
    /// Count of unique spans (forward links)
    pub fn link_count(&self) -> usize {
        self.span_to_node.len()
    }
    
    /// Count of unique entities (reverse map entries)
    pub fn entity_count(&self) -> usize {
        self.node_to_spans.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.span_to_node.is_empty()
    }
    
    /// Iterate over all (range, entity_id, node_index) tuples
    pub fn iter(&self) -> impl Iterator<Item = (TextRange, &str, NodeIndex)> {
        self.span_to_node
            .iter()
            .map(|(range, (id, idx))| (*range, id.as_str(), *idx))
    }
    
    /// Get all entity IDs that have linked spans
    pub fn entity_ids(&self) -> impl Iterator<Item = &str> {
        self.node_to_spans.keys().map(|s| s.as_str())
    }
    
    /// Remove all spans for a specific entity
    pub fn unlink_entity(&mut self, entity_id: &str) {
        if let Some(spans) = self.node_to_spans.remove(entity_id) {
            for span in spans {
                self.span_to_node.remove(&span);
            }
        }
    }
    
    /// Remove a specific span link
    pub fn unlink_range(&mut self, range: TextRange) {
        if let Some((entity_id, _)) = self.span_to_node.remove(&range) {
            if let Some(spans) = self.node_to_spans.get_mut(&entity_id) {
                spans.retain(|r| *r != range);
                if spans.is_empty() {
                    self.node_to_spans.remove(&entity_id);
                }
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
    use rustworkx_core::petgraph::graph::NodeIndex;
    
    fn make_range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }
    
    fn node_idx(i: u32) -> NodeIndex {
        NodeIndex::new(i as usize)
    }
    
    // -------------------------------------------------------------------------
    // Basic Link Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_synapse_link_and_query() {
        let mut synapse = SynapseBridge::new();
        
        // Link "Frodo" at positions 0..5 to node index 0
        let range = make_range(0, 5);
        synapse.link(range, "frodo".to_string(), node_idx(0));
        
        // Query forward: range → node
        let result = synapse.node_for_range(range);
        assert!(result.is_some());
        let (id, idx) = result.unwrap();
        assert_eq!(id, "frodo");
        assert_eq!(idx, node_idx(0));
        
        // Query reverse: entity → spans
        let spans = synapse.spans_of("frodo");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0], range);
    }
    
    #[test]
    fn test_synapse_multiple_occurrences() {
        let mut synapse = SynapseBridge::new();
        
        // Same entity appears at multiple positions
        // "Frodo went to Mordor. Frodo was brave."
        //  ^^^^^                  ^^^^^
        //  0..5                   22..27
        
        synapse.link(make_range(0, 5), "frodo".to_string(), node_idx(0));
        synapse.link(make_range(22, 27), "frodo".to_string(), node_idx(0));
        
        // Should have 2 links total
        assert_eq!(synapse.link_count(), 2);
        
        // But only 1 unique entity
        assert_eq!(synapse.entity_count(), 1);
        
        // Entity should have 2 spans
        let spans = synapse.spans_of("frodo");
        assert_eq!(spans.len(), 2);
        assert!(spans.contains(&make_range(0, 5)));
        assert!(spans.contains(&make_range(22, 27)));
    }
    
    #[test]
    fn test_synapse_clear() {
        let mut synapse = SynapseBridge::new();
        
        synapse.link(make_range(0, 5), "a".to_string(), node_idx(0));
        synapse.link(make_range(10, 15), "b".to_string(), node_idx(1));
        synapse.link(make_range(20, 25), "c".to_string(), node_idx(2));
        
        assert_eq!(synapse.link_count(), 3);
        assert!(!synapse.is_empty());
        
        synapse.clear();
        
        assert_eq!(synapse.link_count(), 0);
        assert!(synapse.is_empty());
        assert!(synapse.spans_of("a").is_empty());
    }
    
    #[test]
    fn test_synapse_offset_lookup() {
        let mut synapse = SynapseBridge::new();
        
        // "Hello Frodo and Sam"
        //        ^^^^^     ^^^
        //        6..11     16..19
        
        synapse.link(make_range(6, 11), "frodo".to_string(), node_idx(0));
        synapse.link(make_range(16, 19), "sam".to_string(), node_idx(1));
        
        // Click at position 8 (inside "Frodo")
        let result = synapse.node_at(8);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "frodo");
        
        // Click at position 17 (inside "Sam")
        let result = synapse.node_at(17);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "sam");
        
        // Click at position 3 (inside "Hello" - no entity)
        let result = synapse.node_at(3);
        assert!(result.is_none());
        
        // Click at position 13 (inside "and" - no entity)
        let result = synapse.node_at(13);
        assert!(result.is_none());
    }
    
    // -------------------------------------------------------------------------
    // Edge Cases
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_synapse_boundary_conditions() {
        let mut synapse = SynapseBridge::new();
        
        // Entity at 10..15
        synapse.link(make_range(10, 15), "entity".to_string(), node_idx(0));
        
        // Position 9 (before) should not match
        assert!(synapse.node_at(9).is_none());
        
        // Position 10 (start, inclusive) should match
        assert!(synapse.node_at(10).is_some());
        
        // Position 14 (inside) should match
        assert!(synapse.node_at(14).is_some());
        
        // Position 15 (end, exclusive) should NOT match
        assert!(synapse.node_at(15).is_none());
    }
    
    #[test]
    fn test_synapse_overlapping_spans() {
        let mut synapse = SynapseBridge::new();
        
        // Overlapping entities (outer and inner)
        // "University of California"
        //  ^^^^^^^^^^^^^^^^^^^^^^^  (0..24) - outer
        //               ^^^^^^^^^^  (14..24) - inner (California)
        
        synapse.link(make_range(0, 24), "uc".to_string(), node_idx(0));
        synapse.link(make_range(14, 24), "california".to_string(), node_idx(1));
        
        // Position 5 (inside outer only) - could match outer
        let result = synapse.node_at(5);
        assert!(result.is_some());
        
        // Position 18 (inside both) - matches whichever is found first
        let result = synapse.node_at(18);
        assert!(result.is_some());
    }
    
    // -------------------------------------------------------------------------
    // Unlink Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_synapse_unlink_entity() {
        let mut synapse = SynapseBridge::new();
        
        synapse.link(make_range(0, 5), "frodo".to_string(), node_idx(0));
        synapse.link(make_range(10, 15), "frodo".to_string(), node_idx(0));
        synapse.link(make_range(20, 25), "sam".to_string(), node_idx(1));
        
        assert_eq!(synapse.link_count(), 3);
        
        // Remove all Frodo links
        synapse.unlink_entity("frodo");
        
        assert_eq!(synapse.link_count(), 1);
        assert!(synapse.spans_of("frodo").is_empty());
        assert!(!synapse.spans_of("sam").is_empty());
    }
    
    #[test]
    fn test_synapse_unlink_range() {
        let mut synapse = SynapseBridge::new();
        
        synapse.link(make_range(0, 5), "frodo".to_string(), node_idx(0));
        synapse.link(make_range(10, 15), "frodo".to_string(), node_idx(0));
        
        assert_eq!(synapse.spans_of("frodo").len(), 2);
        
        // Remove just one span
        synapse.unlink_range(make_range(0, 5));
        
        assert_eq!(synapse.link_count(), 1);
        assert_eq!(synapse.spans_of("frodo").len(), 1);
        assert_eq!(synapse.spans_of("frodo")[0], make_range(10, 15));
    }
    
    // -------------------------------------------------------------------------
    // Iteration Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_synapse_iteration() {
        let mut synapse = SynapseBridge::new();
        
        synapse.link(make_range(0, 5), "a".to_string(), node_idx(0));
        synapse.link(make_range(10, 15), "b".to_string(), node_idx(1));
        
        let collected: Vec<_> = synapse.iter().collect();
        
        assert_eq!(collected.len(), 2);
    }
    
    #[test]
    fn test_synapse_entity_ids() {
        let mut synapse = SynapseBridge::new();
        
        synapse.link(make_range(0, 5), "frodo".to_string(), node_idx(0));
        synapse.link(make_range(10, 15), "sam".to_string(), node_idx(1));
        synapse.link(make_range(20, 25), "frodo".to_string(), node_idx(0));
        
        let ids: Vec<_> = synapse.entity_ids().collect();
        
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"frodo"));
        assert!(ids.contains(&"sam"));
    }
    
    // -------------------------------------------------------------------------
    // WASM Convenience Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_synapse_offset_convenience() {
        let mut synapse = SynapseBridge::new();
        
        synapse.link_offsets(10, 20, "entity".to_string(), node_idx(0));
        
        assert_eq!(synapse.link_count(), 1);
        
        let offsets = synapse.span_offsets_of("entity");
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0], (10, 20));
    }
}
