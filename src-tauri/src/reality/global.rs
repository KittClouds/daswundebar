//! Global Knowledge Graph: Multi-document graph synthesis
//!
//! Phase 3.5.1-3.5.2 of Evolution 1.5: Cross-Document Analysis
//!
//! This module provides:
//! - GlobalGraph: Unified concept graph spanning multiple documents
//! - DocumentContext: Per-document metadata and synapse links
//! - Multi-document processing with entity unification

use std::collections::HashMap;
use super::graph::{ConceptGraph, ConceptNode, ConceptEdge};
use super::synapse::SynapseBridge;
use super::unification::EntityUnifier;
use super::engine::RealityEngine;
use super::parser::SemanticSpan;
use super::projection::Projection;

// =============================================================================
// DocumentContext
// =============================================================================

/// Represents a processed document with its local context
#[derive(Debug)]
pub struct DocumentContext {
    /// Document/note ID
    pub doc_id: String,
    /// Local synapse links (text↔graph for this document)
    pub synapse: SynapseBridge,
    /// Entity IDs found in this document
    pub entities: Vec<String>,
    /// Wikilink targets from this document
    pub wikilinks: Vec<String>,
}

impl DocumentContext {
    /// Create a new document context
    pub fn new(doc_id: impl Into<String>) -> Self {
        Self {
            doc_id: doc_id.into(),
            synapse: SynapseBridge::new(),
            entities: Vec::new(),
            wikilinks: Vec::new(),
        }
    }
}

// =============================================================================
// ProcessDocResult
// =============================================================================

/// Result from processing a document
#[derive(Debug, Clone, Default)]
pub struct ProcessDocResult {
    pub doc_id: String,
    pub projections_count: usize,
    pub nodes_merged: usize,
    pub edges_merged: usize,
    pub new_entities: usize,
}

// =============================================================================
// MergeStats (internal)
// =============================================================================

#[derive(Debug, Clone, Default)]
struct MergeStats {
    nodes_merged: usize,
    edges_merged: usize,
    new_entities: usize,
}

// =============================================================================
// GlobalGraph
// =============================================================================

/// Global knowledge graph spanning multiple documents
/// 
/// This is the main entry point for cross-document analysis.
/// It maintains:
/// - A unified ConceptGraph with merged entities
/// - Per-document contexts with local synapse links
/// - An EntityUnifier for alias resolution
pub struct GlobalGraph {
    /// The unified concept graph
    graph: ConceptGraph,
    /// Document contexts keyed by doc_id
    documents: HashMap<String, DocumentContext>,
    /// Entity unifier for alias resolution
    unifier: EntityUnifier,
}

impl Default for GlobalGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalGraph {
    // -------------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------------
    
    /// Create a new empty global graph
    pub fn new() -> Self {
        Self {
            graph: ConceptGraph::new(),
            documents: HashMap::new(),
            unifier: EntityUnifier::new(),
        }
    }
    
    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------
    
    /// Get the unified graph (immutable)
    pub fn graph(&self) -> &ConceptGraph {
        &self.graph
    }
    
    /// Get the unified graph (mutable)
    pub fn graph_mut(&mut self) -> &mut ConceptGraph {
        &mut self.graph
    }
    
    /// Get the entity unifier (immutable)
    pub fn unifier(&self) -> &EntityUnifier {
        &self.unifier
    }
    
    /// Get the entity unifier (mutable)
    pub fn unifier_mut(&mut self) -> &mut EntityUnifier {
        &mut self.unifier
    }
    
    /// Get document context by ID
    pub fn document(&self, doc_id: &str) -> Option<&DocumentContext> {
        self.documents.get(doc_id)
    }
    
    /// Get all document IDs
    pub fn document_ids(&self) -> Vec<&str> {
        self.documents.keys().map(|s| s.as_str()).collect()
    }
    
    /// Count of documents processed
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty() && self.graph.is_empty()
    }
    
    // -------------------------------------------------------------------------
    // Document Processing
    // -------------------------------------------------------------------------
    
    /// Process a document and merge into global graph
    /// 
    /// This is the main entry point for cross-document processing.
    /// 1. Runs RealityEngine on the document
    /// 2. Merges nodes/edges into global graph (via entity unification)
    /// 3. Stores document context for local synapse queries
    pub fn process_document<S: SemanticSpan>(
        &mut self,
        doc_id: &str,
        text: &str,
        spans: &[S],
    ) -> ProcessDocResult {
        // Create local engine for this document
        let mut engine = RealityEngine::new();
        let (projections, _stats) = engine.process_with_projections(text, spans);
        
        // Extract entity IDs and wikilinks before merging
        let entities = self.extract_entity_ids(&projections);
        let wikilinks = self.extract_wikilinks(&projections);
        
        // Merge into global graph
        let merge_stats = self.merge_graph(engine.graph());
        
        // Create document context with its own synapse
        // We need to move the synapse from the engine
        let mut doc_ctx = DocumentContext::new(doc_id);
        doc_ctx.entities = entities;
        doc_ctx.wikilinks = wikilinks;
        
        // Copy synapse links from engine
        // Since SynapseBridge doesn't impl Clone, we rebuild from iterator
        for (range, entity_id, node_idx) in engine.synapse().iter() {
            doc_ctx.synapse.link(range, entity_id.to_string(), node_idx);
        }
        
        let projections_count = projections.len();
        self.documents.insert(doc_id.to_string(), doc_ctx);
        
        ProcessDocResult {
            doc_id: doc_id.to_string(),
            projections_count,
            nodes_merged: merge_stats.nodes_merged,
            edges_merged: merge_stats.edges_merged,
            new_entities: merge_stats.new_entities,
        }
    }
    
    // -------------------------------------------------------------------------
    // Graph Queries
    // -------------------------------------------------------------------------
    
    /// Get entity→documents mapping (which docs mention each entity)
    pub fn entity_documents(&self) -> HashMap<String, Vec<String>> {
        let mut mapping: HashMap<String, Vec<String>> = HashMap::new();
        
        for (doc_id, ctx) in &self.documents {
            for entity_id in &ctx.entities {
                // Resolve through unifier to get canonical ID
                let canonical = self.unifier.resolve(entity_id);
                mapping
                    .entry(canonical)
                    .or_default()
                    .push(doc_id.clone());
            }
        }
        
        // Deduplicate
        for docs in mapping.values_mut() {
            docs.sort();
            docs.dedup();
        }
        
        mapping
    }
    
    /// Get documents linked by wikilinks from a specific document
    pub fn linked_documents(&self, doc_id: &str) -> Vec<String> {
        self.documents
            .get(doc_id)
            .map(|ctx| ctx.wikilinks.clone())
            .unwrap_or_default()
    }
    
    /// Find documents that share entities with the given document
    pub fn related_documents(&self, doc_id: &str) -> Vec<String> {
        let Some(ctx) = self.documents.get(doc_id) else {
            return Vec::new();
        };
        
        let mut related: HashMap<String, usize> = HashMap::new();
        let entity_docs = self.entity_documents();
        
        for entity_id in &ctx.entities {
            let canonical = self.unifier.resolve(entity_id);
            if let Some(docs) = entity_docs.get(&canonical) {
                for other_doc in docs {
                    if other_doc != doc_id {
                        *related.entry(other_doc.clone()).or_default() += 1;
                    }
                }
            }
        }
        
        // Sort by shared entity count (descending)
        let mut result: Vec<_> = related.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result.into_iter().map(|(doc, _)| doc).collect()
    }
    
    // -------------------------------------------------------------------------
    // Maintenance
    // -------------------------------------------------------------------------
    
    /// Remove a document from the global graph
    /// 
    /// Note: This only removes the document context. Entities added to the
    /// global graph are NOT removed (they may be referenced by other docs).
    pub fn remove_document(&mut self, doc_id: &str) -> bool {
        self.documents.remove(doc_id).is_some()
    }
    
    /// Clear all documents and the graph
    pub fn clear(&mut self) {
        self.documents.clear();
        self.graph.clear();
        self.unifier.clear();
    }
    
    // -------------------------------------------------------------------------
    // Private Helpers
    // -------------------------------------------------------------------------
    
    /// Merge a local ConceptGraph into the global graph
    fn merge_graph(&mut self, local: &ConceptGraph) -> MergeStats {
        let mut stats = MergeStats::default();
        
        // Merge nodes (use unifier to resolve aliases)
        for node in local.nodes() {
            let canonical_id = self.unifier.resolve(&node.id);
            
            // Check if node already exists
            if self.graph.get_node(&canonical_id).is_none() {
                self.graph.ensure_node(ConceptNode::new(
                    &canonical_id,
                    &node.label,
                    &node.kind,
                ));
                stats.new_entities += 1;
            }
            stats.nodes_merged += 1;
        }
        
        // Merge edges
        for (source, target, edge) in local.edges() {
            let canonical_source = self.unifier.resolve(&source.id);
            let canonical_target = self.unifier.resolve(&target.id);
            
            // Add edge (may create duplicates - could dedupe later)
            if self.graph.add_edge(&canonical_source, &canonical_target, edge.clone()).is_some() {
                stats.edges_merged += 1;
            }
        }
        
        stats
    }
    
    /// Extract entity IDs from projections
    fn extract_entity_ids(&self, projections: &[Projection]) -> Vec<String> {
        let mut entities = Vec::new();
        
        for proj in projections {
            match proj {
                Projection::Triple(t) => {
                    entities.push(t.source.trim().to_lowercase());
                    entities.push(t.target.trim().to_lowercase());
                }
                Projection::Quad(q) => {
                    entities.push(q.subject.trim().to_lowercase());
                    entities.push(q.object.trim().to_lowercase());
                }
                Projection::Attribution(a) => {
                    entities.push(a.speaker.trim().to_lowercase());
                }
                Projection::StateChange(s) => {
                    entities.push(s.entity.trim().to_lowercase());
                }
            }
        }
        
        // Deduplicate
        entities.sort();
        entities.dedup();
        entities
    }
    
    /// Extract wikilink targets from projections
    /// 
    /// Note: Currently projections don't capture wikilinks directly.
    /// This will be enhanced when SyntaxRealityBridge is implemented.
    fn extract_wikilinks(&self, _projections: &[Projection]) -> Vec<String> {
        // TODO: Implement when wikilink projections are added
        Vec::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::syntax::SyntaxKind;
    
    // Test span helper
    struct TestSpan {
        start: usize,
        end: usize,
        kind: SyntaxKind,
    }
    
    impl SemanticSpan for TestSpan {
        fn start(&self) -> usize { self.start }
        fn end(&self) -> usize { self.end }
        fn syntax_kind(&self) -> SyntaxKind { self.kind }
    }
    
    fn entity(start: usize, end: usize, _label: &str) -> TestSpan {
        TestSpan { start, end, kind: SyntaxKind::EntitySpan }
    }
    
    fn relation(start: usize, end: usize) -> TestSpan {
        TestSpan { start, end, kind: SyntaxKind::RelationSpan }
    }
    
    // -------------------------------------------------------------------------
    // Basic Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_global_graph_new() {
        let global = GlobalGraph::new();
        assert_eq!(global.document_count(), 0);
        assert!(global.graph().is_empty());
        assert!(global.is_empty());
    }
    
    #[test]
    fn test_global_graph_default() {
        let global = GlobalGraph::default();
        assert!(global.is_empty());
    }
    
    #[test]
    fn test_document_context_new() {
        let ctx = DocumentContext::new("doc1");
        assert_eq!(ctx.doc_id, "doc1");
        assert!(ctx.entities.is_empty());
        assert!(ctx.wikilinks.is_empty());
    }
    
    // -------------------------------------------------------------------------
    // Document Processing Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_process_single_document() {
        let mut global = GlobalGraph::new();
        
        let text = "Frodo owns Ring.";
        let spans = vec![
            entity(0, 5, "Frodo"),
            relation(6, 10),
            entity(11, 15, "Ring"),
        ];
        
        let result = global.process_document("doc1", text, &spans);
        
        assert_eq!(result.doc_id, "doc1");
        assert!(result.nodes_merged >= 2, "Should merge at least 2 nodes");
        assert_eq!(global.document_count(), 1);
        assert!(global.document("doc1").is_some());
    }
    
    #[test]
    fn test_process_multiple_documents() {
        let mut global = GlobalGraph::new();
        
        // Document 1
        let text1 = "Frodo owns Ring.";
        let spans1 = vec![
            entity(0, 5, "Frodo"),
            relation(6, 10),
            entity(11, 15, "Ring"),
        ];
        global.process_document("doc1", text1, &spans1);
        
        // Document 2
        let text2 = "Frodo travels.";
        let spans2 = vec![
            entity(0, 5, "Frodo"),
            relation(6, 13),
        ];
        global.process_document("doc2", text2, &spans2);
        
        assert_eq!(global.document_count(), 2);
        assert!(global.document("doc1").is_some());
        assert!(global.document("doc2").is_some());
        
        // Frodo should only exist once in global graph (merged)
        let frodo_count = global.graph().nodes()
            .filter(|n| n.label.to_lowercase() == "frodo")
            .count();
        assert_eq!(frodo_count, 1, "Frodo should be unified");
    }
    
    #[test]
    fn test_entity_documents_mapping() {
        let mut global = GlobalGraph::new();
        
        // Doc1 has Frodo owns Ring (full triple pattern)
        let text1 = "Frodo owns Ring.";
        let spans1 = vec![
            entity(0, 5, "Frodo"),
            relation(6, 10),
            entity(11, 15, "Ring"),
        ];
        global.process_document("doc1", text1, &spans1);
        
        // Doc2 also has Frodo loves Sam
        let text2 = "Frodo loves Sam.";
        let spans2 = vec![
            entity(0, 5, "Frodo"),
            relation(6, 11),
            entity(12, 15, "Sam"),
        ];
        global.process_document("doc2", text2, &spans2);
        
        let entity_docs = global.entity_documents();
        let frodo_docs = entity_docs.get("frodo");
        
        assert!(frodo_docs.is_some(), "Should have frodo in entity_docs. Keys: {:?}", entity_docs.keys().collect::<Vec<_>>());
        let frodo_docs = frodo_docs.unwrap();
        assert!(frodo_docs.contains(&"doc1".to_string()));
        assert!(frodo_docs.contains(&"doc2".to_string()));
    }
    
    // -------------------------------------------------------------------------
    // Entity Unification Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_unifier_integration() {
        let mut global = GlobalGraph::new();
        
        // Add alias before processing
        global.unifier_mut().add_alias("strider", "aragorn");
        
        // Doc1 mentions Aragorn loves Arwen
        let text1 = "Aragorn loves Arwen.";
        let spans1 = vec![
            entity(0, 7, "Aragorn"),
            relation(8, 13),
            entity(14, 19, "Arwen"),
        ];
        global.process_document("doc1", text1, &spans1);
        
        // Doc2 mentions Strider (alias) leads Rangers
        let text2 = "Strider leads Rangers.";
        let spans2 = vec![
            entity(0, 7, "Strider"),
            relation(8, 13),
            entity(14, 21, "Rangers"),
        ];
        global.process_document("doc2", text2, &spans2);
        
        // Should both resolve to "aragorn" in entity_documents
        let entity_docs = global.entity_documents();
        let aragorn_docs = entity_docs.get("aragorn");
        
        assert!(aragorn_docs.is_some(), "Should have aragorn after unification. Keys: {:?}", entity_docs.keys().collect::<Vec<_>>());
        assert_eq!(aragorn_docs.unwrap().len(), 2, "Both docs should map to aragorn");
    }
    
    // -------------------------------------------------------------------------
    // Maintenance Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_remove_document() {
        let mut global = GlobalGraph::new();
        
        let text = "Frodo owns Ring.";
        let spans = vec![entity(0, 5, "Frodo")];
        global.process_document("doc1", text, &spans);
        
        assert_eq!(global.document_count(), 1);
        
        let removed = global.remove_document("doc1");
        
        assert!(removed);
        assert_eq!(global.document_count(), 0);
        assert!(global.document("doc1").is_none());
        
        // Graph nodes still exist (not cleaned up)
        assert!(!global.graph().is_empty());
    }
    
    #[test]
    fn test_clear() {
        let mut global = GlobalGraph::new();
        
        let text = "Frodo owns Ring.";
        let spans = vec![entity(0, 5, "Frodo")];
        global.process_document("doc1", text, &spans);
        
        assert!(!global.is_empty());
        
        global.clear();
        
        assert!(global.is_empty());
        assert!(global.graph().is_empty());
        assert_eq!(global.document_count(), 0);
    }
    
    #[test]
    fn test_document_ids() {
        let mut global = GlobalGraph::new();
        
        global.process_document("doc1", "Text", &[] as &[TestSpan]);
        global.process_document("doc2", "Text", &[] as &[TestSpan]);
        global.process_document("doc3", "Text", &[] as &[TestSpan]);
        
        let ids = global.document_ids();
        assert_eq!(ids.len(), 3);
    }
    
    #[test]
    fn test_related_documents() {
        let mut global = GlobalGraph::new();
        
        // Doc1 has Frodo owns Ring
        let text1 = "Frodo owns Ring.";
        let spans1 = vec![
            entity(0, 5, "Frodo"),
            relation(6, 10),
            entity(11, 15, "Ring"),
        ];
        global.process_document("doc1", text1, &spans1);
        
        // Doc2 also has Frodo loves Sam
        let text2 = "Frodo loves Sam.";
        let spans2 = vec![
            entity(0, 5, "Frodo"),
            relation(6, 11),
            entity(12, 15, "Sam"),
        ];
        global.process_document("doc2", text2, &spans2);
        
        // Doc3 has only Gandalf helps Bilbo (no shared entities)
        let text3 = "Gandalf helps Bilbo.";
        let spans3 = vec![
            entity(0, 7, "Gandalf"),
            relation(8, 13),
            entity(14, 19, "Bilbo"),
        ];
        global.process_document("doc3", text3, &spans3);
        
        // Doc1 should be related to Doc2 (shared Frodo)
        let related = global.related_documents("doc1");
        assert!(related.contains(&"doc2".to_string()), "Doc1 should be related to Doc2 via shared Frodo. Related: {:?}", related);
        assert!(!related.contains(&"doc3".to_string()), "Doc1 should not be related to Doc3");
    }
}
