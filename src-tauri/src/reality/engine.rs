//! Unified Reality Engine
//!
//! This module combines all reality components into a single cohesive engine:
//! - CST (Rowan) for syntax tree operations
//! - Graph for semantic relationships  
//! - Synapse for text↔graph bridging
//! - Interner for string deduplication
//!
//! Phase 4 of Evolution 1.5: Integration with richer projections

use rowan::GreenNode;

use super::syntax::{RealityLanguage, SyntaxKind};
use super::parser::{zip_reality, SemanticSpan};
use super::projection::{
    project_triples, project_quads, project_attributions, project_state_changes,
    project_all, project_all_with_stats,
    Triple, QuadPlus, Attribution, StateChange, Projection, ProjectionStats,
};
use super::graph::{ConceptGraph, ConceptNode, ConceptEdge, EdgeKind};
use super::synapse::SynapseBridge;

/// Type alias for Rowan syntax node with our language
pub type SyntaxNode = rowan::SyntaxNode<RealityLanguage>;

// =============================================================================
// RealityEngine
// =============================================================================

/// The unified reality engine
/// 
/// Holds: CST (Rowan) + Graph + Synapse (bridge)
/// 
/// This is the main entry point for processing documents and querying
/// the semantic graph.
#[derive(Default)]
pub struct RealityEngine {
    /// Current document's CST root (GreenNode is immutable, cheap to clone)
    green: Option<GreenNode>,
    
    /// The semantic graph (concepts and relationships)
    graph: ConceptGraph,
    
    /// Text ↔ Graph bridge
    synapse: SynapseBridge,
    
    /// Statistics for the last process operation
    last_stats: Option<ProcessStats>,
}

/// Statistics from processing a document
#[derive(Debug, Clone, Default)]
pub struct ProcessStats {
    pub triples_extracted: usize,
    pub quads_extracted: usize,
    pub attributions_extracted: usize,
    pub state_changes_extracted: usize,
    pub nodes_created: usize,
    pub edges_created: usize,
    pub synapse_links: usize,
}

impl RealityEngine {
    /// Create a new empty engine
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Process a document with semantic spans
    /// 
    /// This is the main entry point. It:
    /// 1. Builds the CST from text + spans
    /// 2. Extracts triples from the CST
    /// 3. Updates the graph with new concepts/relationships
    /// 4. Populates the synapse bridge for text↔graph linking
    /// 
    /// Returns the number of triples extracted.
    pub fn process<S: SemanticSpan>(&mut self, text: &str, spans: &[S]) -> usize {
        // Clear previous state
        self.synapse.clear();
        
        // 1. Build CST
        self.green = Some(zip_reality(text, spans));
        
        // 2. Extract triples
        let root = self.syntax_root().expect("Just built the tree");
        let triples = project_triples(&root);
        
        // 3. Update graph and synapse
        let mut stats = ProcessStats::default();
        stats.triples_extracted = triples.len();
        
        for triple in &triples {
            self.ingest_triple(triple, &mut stats);
        }
        
        // Also link entities that aren't in triples
        self.link_standalone_entities(&root, &mut stats);
        
        self.last_stats = Some(stats.clone());
        
        stats.triples_extracted
    }
    
    /// Clear all state (graph, synapse, CST)
    pub fn clear(&mut self) {
        self.green = None;
        self.graph.clear();
        self.synapse.clear();
        self.last_stats = None;
    }
    
    /// Get the syntax tree root (if processed)
    pub fn syntax_root(&self) -> Option<SyntaxNode> {
        self.green.as_ref().map(|g| SyntaxNode::new_root(g.clone()))
    }
    
    /// Get a reference to the graph
    pub fn graph(&self) -> &ConceptGraph {
        &self.graph
    }
    
    /// Get a mutable reference to the graph
    pub fn graph_mut(&mut self) -> &mut ConceptGraph {
        &mut self.graph
    }
    
    /// Get a reference to the synapse bridge
    pub fn synapse(&self) -> &SynapseBridge {
        &self.synapse
    }
    
    /// Get stats from the last process operation
    pub fn last_stats(&self) -> Option<&ProcessStats> {
        self.last_stats.as_ref()
    }
    
    /// Get engine stats (current state)
    pub fn stats(&self) -> EngineStats {
        EngineStats {
            has_cst: self.green.is_some(),
            node_count: self.graph.node_count(),
            edge_count: self.graph.edge_count(),
            synapse_links: self.synapse.link_count(),
        }
    }
    
    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------
    
    /// Ingest a triple into the graph and synapse
    fn ingest_triple(&mut self, triple: &Triple, stats: &mut ProcessStats) {
        // Create or get source node
        let source_id = self.make_entity_id(&triple.source);
        let source_node = ConceptNode::new(&source_id, &triple.source, "Entity");
        let _source_idx = self.graph.ensure_node(source_node);
        if self.graph.node_count() > stats.nodes_created {
            stats.nodes_created = self.graph.node_count();
        }
        
        // Create or get target node
        let target_id = self.make_entity_id(&triple.target);
        let target_node = ConceptNode::new(&target_id, &triple.target, "Entity");
        let _target_idx = self.graph.ensure_node(target_node);
        stats.nodes_created = self.graph.node_count();
        
        // Create edge
        let edge = ConceptEdge::unweighted(&triple.relation);
        if self.graph.add_edge(&source_id, &target_id, edge).is_some() {
            stats.edges_created += 1;
        }
        
        // Link source span to graph
        if let Some((start, end)) = triple.source_span {
            self.synapse.link_offsets(
                start as u32,
                end as u32,
                source_id.clone(),
            );
            stats.synapse_links += 1;
        }
        
        // Link target span to graph
        if let Some((start, end)) = triple.target_span {
            self.synapse.link_offsets(
                start as u32,
                end as u32,
                target_id,
            );
            stats.synapse_links += 1;
        }
    }
    
    /// Link standalone entities (those not in triples) to the synapse
    fn link_standalone_entities(&mut self, root: &SyntaxNode, stats: &mut ProcessStats) {
        for node in root.descendants() {
            if node.kind() == SyntaxKind::EntitySpan {
                let text = node.text().to_string();
                let range = node.text_range();
                
                let entity_id = self.make_entity_id(&text);
                
                // Ensure node exists in graph
                let concept = ConceptNode::new(&entity_id, &text, "Entity");
                let _idx = self.graph.ensure_node(concept);
                stats.nodes_created = self.graph.node_count();
                
                // Link if not already linked
                if !self.synapse.contains_range(range) {
                    self.synapse.link(range, entity_id);
                    stats.synapse_links += 1;
                }
            }
        }
    }
    
    /// Generate a stable entity ID from a label
    fn make_entity_id(&self, label: &str) -> String {
        // Simple normalization: lowercase, trim, replace spaces with underscores
        label.trim().to_lowercase().replace(' ', "_")
    }
    
    // -------------------------------------------------------------------------
    // Phase 4: Projection Integration
    // -------------------------------------------------------------------------
    
    /// Process a document with all projection types (Phase 4)
    ///
    /// This is the enhanced entry point that:
    /// 1. Builds the CST from text + spans
    /// 2. Extracts ALL projection types (Triple, Quad, Attribution, StateChange)
    /// 3. Updates the graph with new concepts and rich edge types
    /// 4. Populates the synapse bridge
    ///
    /// Returns the extracted projections and statistics.
    pub fn process_with_projections<S: SemanticSpan>(
        &mut self,
        text: &str,
        spans: &[S],
    ) -> (Vec<Projection>, ProcessStats) {
        // Clear previous synapse state
        self.synapse.clear();
        
        // 1. Build CST
        self.green = Some(zip_reality(text, spans));
        
        // 2. Extract all projections
        let root = self.syntax_root().expect("Just built the tree");
        let (projections, proj_stats) = project_all_with_stats(&root);
        
        // 3. Populate graph from projections
        let mut stats = ProcessStats {
            triples_extracted: proj_stats.triples,
            quads_extracted: proj_stats.quads,
            attributions_extracted: proj_stats.attributions,
            state_changes_extracted: proj_stats.state_changes,
            ..Default::default()
        };
        
        self.populate_from_projections(&projections, &mut stats);
        
        // 4. Link standalone entities
        self.link_standalone_entities(&root, &mut stats);
        
        self.last_stats = Some(stats.clone());
        
        (projections, stats)
    }
    
    /// Populate graph from projections
    ///
    /// Dispatches each projection type to its specific ingestion method.
    pub fn populate_from_projections(&mut self, projections: &[Projection], stats: &mut ProcessStats) {
        for proj in projections {
            match proj {
                Projection::Triple(t) => self.ingest_triple(t, stats),
                Projection::Quad(q) => self.ingest_quad(q, stats),
                Projection::Attribution(a) => self.ingest_attribution(a, stats),
                Projection::StateChange(s) => self.ingest_state_change(s, stats),
            }
        }
    }
    
    /// Ingest a QuadPlus projection (SPO + modifiers)
    fn ingest_quad(&mut self, quad: &QuadPlus, stats: &mut ProcessStats) {
        let subject_id = self.make_entity_id(&quad.subject);
        let object_id = self.make_entity_id(&quad.object);
        
        // Ensure nodes exist
        let subject_node = ConceptNode::new(&subject_id, &quad.subject, "Entity");
        let _subject_idx = self.graph.ensure_node(subject_node);
        
        let object_node = ConceptNode::new(&object_id, &quad.object, "Entity");
        let _object_idx = self.graph.ensure_node(object_node);
        
        stats.nodes_created = self.graph.node_count();
        
        // Create modified relation edge
        let edge = ConceptEdge::modified_relation(
            &quad.predicate.to_uppercase(),
            quad.manner.clone(),
            quad.location.clone(),
            quad.time.clone(),
        );
        
        if self.graph.add_edge(&subject_id, &object_id, edge).is_some() {
            stats.edges_created += 1;
        }
        
        // Link spans
        if let Some((start, end)) = quad.subject_span {
            self.synapse.link_offsets(start as u32, end as u32, subject_id.clone());
            stats.synapse_links += 1;
        }
        if let Some((start, end)) = quad.object_span {
            self.synapse.link_offsets(start as u32, end as u32, object_id);
            stats.synapse_links += 1;
        }
    }
    
    /// Ingest an Attribution projection (dialogue)
    fn ingest_attribution(&mut self, attr: &Attribution, stats: &mut ProcessStats) {
        let speaker_id = self.make_entity_id(&attr.speaker);
        let quote_id = format!("quote_{}", attr.quote_span.0);
        
        // Speaker node
        let speaker_node = ConceptNode::new(&speaker_id, &attr.speaker, "Character");
        let _speaker_idx = self.graph.ensure_node(speaker_node);
        
        // Quote node (special type)
        let quote_node = ConceptNode::new(&quote_id, &attr.quote, "Quote");
        let _quote_idx = self.graph.ensure_node(quote_node);
        
        stats.nodes_created = self.graph.node_count();
        
        // Create attribution edge
        let edge = ConceptEdge::attribution(&attr.verb);
        
        if self.graph.add_edge(&speaker_id, &quote_id, edge).is_some() {
            stats.edges_created += 1;
        }
        
        // Link speaker span
        if let Some((start, end)) = attr.speaker_span {
            self.synapse.link_offsets(start as u32, end as u32, speaker_id);
            stats.synapse_links += 1;
        }
    }
    
    /// Ingest a StateChange projection
    fn ingest_state_change(&mut self, change: &StateChange, stats: &mut ProcessStats) {
        let entity_id = self.make_entity_id(&change.entity);
        let state_id = format!("state_{}", change.to_state.to_lowercase());
        
        // Entity node
        let entity_node = ConceptNode::new(&entity_id, &change.entity, "Character");
        let _entity_idx = self.graph.ensure_node(entity_node);
        
        // State node
        let state_node = ConceptNode::new(&state_id, &change.to_state, "State");
        let _state_idx = self.graph.ensure_node(state_node);
        
        stats.nodes_created = self.graph.node_count();
        
        // Create state transition edge
        let edge = ConceptEdge::state_transition(&change.to_state, change.trigger.clone());
        
        if self.graph.add_edge(&entity_id, &state_id, edge).is_some() {
            stats.edges_created += 1;
        }
        
        // Link entity span
        if let Some((start, end)) = change.entity_span {
            self.synapse.link_offsets(start as u32, end as u32, entity_id);
            stats.synapse_links += 1;
        }
    }
    
    // -------------------------------------------------------------------------
    // Phase 3: NER Inference Integration
    // -------------------------------------------------------------------------
    
    /// Confidence thresholds for NER inference
    const AUTO_PROMOTE_THRESHOLD: f32 = 0.90;
    const SUGGEST_THRESHOLD: f32 = 0.60;
    
    /// Merge NER inference results into the graph
    ///
    /// Applies confidence tiers:
    /// - >= 0.90: Auto-promote to graph immediately
    /// - 0.60 - 0.89: Suggest (returned for UI to display)
    /// - < 0.60: Ignore
    ///
    /// Explicit entities already in the graph are never overwritten.
    pub fn merge_inference(
        &mut self,
        result: &crate::ai::AnalysisResult,
        world_id: &str,
    ) -> MergeResult {
        let mut merge_result = MergeResult::default();
        
        for entity in &result.entities {
            let entity_id = self.make_entity_id(&entity.text);
            
            // Check if explicit entity already exists
            if self.graph.node_by_id(&entity_id).is_some() {
                log::debug!(
                    "[RealityEngine] Skipping inferred entity '{}' - explicit entity exists",
                    entity.text
                );
                merge_result.skipped_explicit.push(entity.clone());
                continue;
            }
            
            // Apply confidence tiers
            if entity.confidence >= Self::AUTO_PROMOTE_THRESHOLD {
                // Auto-promote: Create node immediately
                let node = ConceptNode::new(
                    &entity_id,
                    &entity.text,
                    &entity.label,
                );
                self.graph.ensure_node(node);
                
                log::debug!(
                    "[RealityEngine] Auto-promoted inferred entity '{}' (confidence: {:.2})",
                    entity.text,
                    entity.confidence
                );
                merge_result.auto_promoted.push(entity.clone());
                
            } else if entity.confidence >= Self::SUGGEST_THRESHOLD {
                // Suggest: Return for UI to display
                log::debug!(
                    "[RealityEngine] Suggesting inferred entity '{}' (confidence: {:.2})",
                    entity.text,
                    entity.confidence
                );
                merge_result.suggested.push(entity.clone());
                
            } else {
                // Ignore: Confidence too low
                log::debug!(
                    "[RealityEngine] Ignoring low-confidence entity '{}' (confidence: {:.2})",
                    entity.text,
                    entity.confidence
                );
                merge_result.ignored.push(entity.clone());
            }
        }
        
        log::info!(
            "[RealityEngine] Merged inference for {}: {} auto, {} suggest, {} ignore, {} explicit-skipped",
            result.doc_id,
            merge_result.auto_promoted.len(),
            merge_result.suggested.len(),
            merge_result.ignored.len(),
            merge_result.skipped_explicit.len(),
        );
        
        merge_result
    }
    
    /// Get dynamic labels from current graph ontology
    ///
    /// Returns the top N entity kinds from the graph, useful for
    /// feeding to GLiNER as candidate labels.
    pub fn get_entity_kinds(&self, limit: usize) -> Vec<String> {
        use std::collections::HashMap;
        
        let mut kind_counts: HashMap<String, usize> = HashMap::new();
        
        for node in self.graph.nodes() {
            *kind_counts.entry(node.kind.clone()).or_default() += 1;
        }
        
        // Sort by count descending
        let mut kinds: Vec<_> = kind_counts.into_iter().collect();
        kinds.sort_by(|a, b| b.1.cmp(&a.1));
        
        kinds.into_iter()
            .take(limit)
            .map(|(kind, _)| kind)
            .collect()
    }
}

/// Result of merging NER inference into the graph
#[derive(Debug, Clone, Default)]
pub struct MergeResult {
    /// Entities auto-promoted to graph (confidence >= 0.90)
    pub auto_promoted: Vec<crate::ai::InferredEntity>,
    /// Entities suggested for user review (0.60 <= confidence < 0.90)
    pub suggested: Vec<crate::ai::InferredEntity>,
    /// Entities ignored due to low confidence (< 0.60)
    pub ignored: Vec<crate::ai::InferredEntity>,
    /// Entities skipped because explicit entity already exists
    pub skipped_explicit: Vec<crate::ai::InferredEntity>,
}

/// Current engine state statistics
#[derive(Debug, Clone)]
pub struct EngineStats {
    pub has_cst: bool,
    pub node_count: usize,
    pub edge_count: usize,
    pub synapse_links: usize,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    enum TestSpan {
        Entity { start: usize, end: usize, label: String },
        Relation { start: usize, end: usize },
    }
    
    impl SemanticSpan for TestSpan {
        fn start(&self) -> usize {
            match self {
                TestSpan::Entity { start, .. } => *start,
                TestSpan::Relation { start, .. } => *start,
            }
        }
        fn end(&self) -> usize {
            match self {
                TestSpan::Entity { end, .. } => *end,
                TestSpan::Relation { end, .. } => *end,
            }
        }
        fn syntax_kind(&self) -> SyntaxKind {
            match self {
                TestSpan::Entity { .. } => SyntaxKind::EntitySpan,
                TestSpan::Relation { .. } => SyntaxKind::RelationSpan,
            }
        }
    }
    
    fn entity(start: usize, end: usize, label: &str) -> TestSpan {
        TestSpan::Entity { start, end, label: label.to_string() }
    }
    
    fn relation(start: usize, end: usize) -> TestSpan {
        TestSpan::Relation { start, end }
    }
    
    // -------------------------------------------------------------------------
    // Basic Processing Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_engine_process_simple() {
        let mut engine = RealityEngine::new();
        
        // "Frodo owns Sting."
        //  ^^^^^      ^^^^^
        //  0..5       11..16
        let text = "Frodo owns Sting.";
        let spans = vec![
            entity(0, 5, "Frodo"),
            relation(6, 10),
            entity(11, 16, "Sting"),
        ];
        
        let triple_count = engine.process(text, &spans);
        
        assert_eq!(triple_count, 1, "Should extract 1 triple");
        
        let stats = engine.stats();
        assert!(stats.has_cst, "Should have CST");
        assert_eq!(stats.node_count, 2, "Should have 2 nodes (Frodo, Sting)");
        assert_eq!(stats.edge_count, 1, "Should have 1 edge (owns)");
    }
    
    #[test]
    fn test_engine_process_chain() {
        let mut engine = RealityEngine::new();
        
        // "A owns B. B owns C."
        let text = "A owns B. B owns C.";
        let spans = vec![
            entity(0, 1, "A"),
            relation(2, 6),
            entity(7, 8, "B"),
            entity(10, 11, "B"),
            relation(12, 16),
            entity(17, 18, "C"),
        ];
        
        let triple_count = engine.process(text, &spans);
        
        assert_eq!(triple_count, 2, "Should extract 2 triples");
        
        let stats = engine.stats();
        assert_eq!(stats.node_count, 3, "Should have 3 nodes (A, B, C)");
        assert_eq!(stats.edge_count, 2, "Should have 2 edges");
    }
    
    #[test]
    fn test_engine_synapse_populated() {
        let mut engine = RealityEngine::new();
        
        let text = "Frodo owns Sting.";
        let spans = vec![
            entity(0, 5, "Frodo"),
            relation(6, 10),
            entity(11, 16, "Sting"),
        ];
        
        engine.process(text, &spans);
        
        // Check synapse has links
        assert!(engine.synapse().link_count() >= 2, "Should have at least 2 synapse links");
        
        // Check we can look up by offset
        let result = engine.synapse().node_at(2); // Inside "Frodo"
        assert!(result.is_some(), "Should find entity at offset 2");
        assert_eq!(result.unwrap().0, "frodo");
        
        let result = engine.synapse().node_at(13); // Inside "Sting"
        assert!(result.is_some(), "Should find entity at offset 13");
        assert_eq!(result.unwrap().0, "sting");
    }
    
    #[test]
    fn test_engine_stats() {
        let mut engine = RealityEngine::new();
        
        // Before processing
        let stats = engine.stats();
        assert!(!stats.has_cst);
        assert_eq!(stats.node_count, 0);
        
        // Process
        let text = "Hello World.";
        let spans: Vec<TestSpan> = vec![];
        engine.process(text, &spans);
        
        // After processing (no entities, but has CST)
        let stats = engine.stats();
        assert!(stats.has_cst);
    }
    
    #[test]
    fn test_engine_reprocess() {
        let mut engine = RealityEngine::new();
        
        // First document
        let text1 = "A owns B.";
        let spans1 = vec![
            entity(0, 1, "A"),
            relation(2, 6),
            entity(7, 8, "B"),
        ];
        engine.process(text1, &spans1);
        
        assert_eq!(engine.graph().node_count(), 2);
        
        // Second document (graph state persists, synapse clears)
        let text2 = "C owns D.";
        let spans2 = vec![
            entity(0, 1, "C"),
            relation(2, 6),
            entity(7, 8, "D"),
        ];
        engine.process(text2, &spans2);
        
        // Graph should now have 4 nodes (A, B, C, D)
        assert_eq!(engine.graph().node_count(), 4);
        
        // Synapse should only have links for the current document
        let synapse = engine.synapse();
        assert!(synapse.node_at(0).is_some()); // "C" in current doc
        assert!(synapse.spans_of("a").is_empty()); // "A" from old doc has no spans
    }
    
    #[test]
    fn test_engine_clear() {
        let mut engine = RealityEngine::new();
        
        let text = "A owns B.";
        let spans = vec![
            entity(0, 1, "A"),
            relation(2, 6),
            entity(7, 8, "B"),
        ];
        engine.process(text, &spans);
        
        assert!(engine.stats().has_cst);
        assert!(engine.graph().node_count() > 0);
        
        engine.clear();
        
        assert!(!engine.stats().has_cst);
        assert_eq!(engine.graph().node_count(), 0);
        assert!(engine.synapse().is_empty());
    }
    
    // -------------------------------------------------------------------------
    // Graph Query Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_engine_graph_queries() {
        let mut engine = RealityEngine::new();
        
        let text = "Frodo owns Sting.";
        let spans = vec![
            entity(0, 5, "Frodo"),
            relation(6, 10),
            entity(11, 16, "Sting"),
        ];
        engine.process(text, &spans);
        
        // Query outgoing edges from Frodo
        let outgoing = engine.graph().outgoing_edges("frodo");
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].0.id, "sting");
        assert_eq!(outgoing[0].1.relation, "owns");
        
        // Query incoming edges to Sting
        let incoming = engine.graph().incoming_edges("sting");
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].0.id, "frodo");
    }
    
    // -------------------------------------------------------------------------
    // Phase 4: Projection Integration Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_engine_process_with_projections() {
        let mut engine = RealityEngine::new();
        
        let text = "Frodo owns Sting.";
        let spans = vec![
            entity(0, 5, "Frodo"),
            relation(6, 10),
            entity(11, 16, "Sting"),
        ];
        
        let (projections, stats) = engine.process_with_projections(text, &spans);
        
        // Should have triples
        assert!(stats.triples_extracted >= 1 || !projections.is_empty(),
            "Should extract at least one projection");
        
        // Graph should be populated
        assert!(engine.graph().node_count() >= 2, "Should have nodes");
    }
    
    #[test]
    fn test_engine_ingest_quad() {
        use super::super::projection::QuadPlus;
        
        let mut engine = RealityEngine::new();
        let mut stats = ProcessStats::default();
        
        let quad = QuadPlus {
            subject: "Gandalf".to_string(),
            predicate: "defeated".to_string(),
            object: "Sauron".to_string(),
            manner: Some("with magic".to_string()),
            location: Some("in Mordor".to_string()),
            time: None,
            subject_span: Some((0, 7)),
            object_span: Some((17, 23)),
        };
        
        engine.ingest_quad(&quad, &mut stats);
        
        // Should create nodes
        assert_eq!(engine.graph().node_count(), 2);
        
        // Should create edge with modified relation kind
        assert_eq!(engine.graph().edge_count(), 1);
        
        let edges: Vec<_> = engine.graph().edges().collect();
        match &edges[0].2.edge_kind {
            super::super::graph::EdgeKind::ModifiedRelation { manner, location, time } => {
                assert_eq!(manner.as_deref(), Some("with magic"));
                assert_eq!(location.as_deref(), Some("in Mordor"));
                assert!(time.is_none());
            }
            _ => panic!("Expected ModifiedRelation edge kind"),
        }
    }
    
    #[test]
    fn test_engine_ingest_attribution() {
        use super::super::projection::Attribution;
        
        let mut engine = RealityEngine::new();
        let mut stats = ProcessStats::default();
        
        let attr = Attribution {
            speaker: "Gandalf".to_string(),
            quote: "You shall not pass!".to_string(),
            quote_span: (0, 21),
            verb: "shouted".to_string(),
            speaker_span: Some((22, 29)),
        };
        
        engine.ingest_attribution(&attr, &mut stats);
        
        // Should create speaker and quote nodes
        assert_eq!(engine.graph().node_count(), 2);
        
        // Should create attribution edge
        let edges: Vec<_> = engine.graph().edges().collect();
        assert_eq!(edges.len(), 1);
        
        match &edges[0].2.edge_kind {
            super::super::graph::EdgeKind::Attribution { verb } => {
                assert_eq!(verb, "shouted");
            }
            _ => panic!("Expected Attribution edge kind"),
        }
    }
    
    #[test]
    fn test_engine_ingest_state_change() {
        use super::super::projection::StateChange;
        
        let mut engine = RealityEngine::new();
        let mut stats = ProcessStats::default();
        
        let change = StateChange {
            entity: "Frodo".to_string(),
            from_state: None,
            to_state: "invisible".to_string(),
            trigger: Some("after putting on the Ring".to_string()),
            entity_span: Some((0, 5)),
        };
        
        engine.ingest_state_change(&change, &mut stats);
        
        // Should create entity and state nodes
        assert_eq!(engine.graph().node_count(), 2);
        
        // Should create state transition edge
        let edges: Vec<_> = engine.graph().edges().collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].2.relation, "BECAME_INVISIBLE");
        
        match &edges[0].2.edge_kind {
            super::super::graph::EdgeKind::StateTransition { trigger } => {
                assert!(trigger.as_ref().unwrap().contains("Ring"));
            }
            _ => panic!("Expected StateTransition edge kind"),
        }
    }
    
    #[test]
    fn test_engine_populate_from_projections() {
        use super::super::projection::{Projection, Triple, QuadPlus};
        
        let mut engine = RealityEngine::new();
        let mut stats = ProcessStats::default();
        
        let projections = vec![
            Projection::Triple(Triple {
                source: "A".to_string(),
                relation: "owns".to_string(),
                target: "B".to_string(),
                source_span: None,
                target_span: None,
            }),
            Projection::Quad(QuadPlus {
                subject: "C".to_string(),
                predicate: "defeated".to_string(),
                object: "D".to_string(),
                manner: Some("with sword".to_string()),
                location: None,
                time: None,
                subject_span: None,
                object_span: None,
            }),
        ];
        
        engine.populate_from_projections(&projections, &mut stats);
        
        // Should have 4 nodes (A, B, C, D)
        assert_eq!(engine.graph().node_count(), 4);
        
        // Should have 2 edges
        assert_eq!(engine.graph().edge_count(), 2);
    }
    
    #[test]
    fn test_evolution_1_5_integration() {
        // Integration test for the full Evolution 1.5 pipeline
        let mut engine = RealityEngine::new();
        
        // Simple test with entity-relation-entity pattern
        let text = "Frodo destroyed Ring.";
        let spans = vec![
            entity(0, 5, "Frodo"),
            relation(6, 15),  // "destroyed"
            entity(16, 20, "Ring"),
        ];
        
        let (projections, stats) = engine.process_with_projections(text, &spans);
        
        // Should have extracted at least triples
        assert!(stats.triples_extracted >= 1, 
            "Should extract at least 1 triple. Stats: {:?}", stats);
        
        // Graph should be populated
        assert!(engine.graph().node_count() >= 2, 
            "Graph should have at least 2 nodes");
        assert!(engine.graph().edge_count() >= 1,
            "Graph should have at least 1 edge");
        
        // Verify CST was built
        assert!(engine.syntax_root().is_some(), "Should have syntax tree");
        
        // Verify synapse was populated
        assert!(engine.synapse().link_count() >= 2, 
            "Synapse should have at least 2 links");
    }
    
    // -------------------------------------------------------------------------
    // Phase 3: NER Integration Tests (TDD)
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_merge_inference_high_confidence_auto_promotes() {
        let mut engine = RealityEngine::new();
        
        let result = crate::ai::AnalysisResult {
            doc_id: "note1".to_string(),
            entities: vec![
                crate::ai::InferredEntity {
                    text: "Zorian".to_string(),
                    label: "CHARACTER".to_string(),
                    start: 0,
                    end: 6,
                    confidence: 0.95, // High confidence - should auto-promote
                },
            ],
            inference_time_ms: 50,
        };
        
        let merge_result = engine.merge_inference(&result, "world1");
        
        assert_eq!(merge_result.auto_promoted.len(), 1);
        assert_eq!(merge_result.suggested.len(), 0);
        assert_eq!(merge_result.ignored.len(), 0);
        
        // Should have created a node in the graph
        assert!(engine.graph().node_by_id("zorian").is_some());
    }
    
    #[test]
    fn test_merge_inference_medium_confidence_suggests() {
        let mut engine = RealityEngine::new();
        
        let result = crate::ai::AnalysisResult {
            doc_id: "note1".to_string(),
            entities: vec![
                crate::ai::InferredEntity {
                    text: "Cyoria".to_string(),
                    label: "LOCATION".to_string(),
                    start: 20,
                    end: 26,
                    confidence: 0.75, // Medium confidence - should suggest
                },
            ],
            inference_time_ms: 50,
        };
        
        let merge_result = engine.merge_inference(&result, "world1");
        
        assert_eq!(merge_result.auto_promoted.len(), 0);
        assert_eq!(merge_result.suggested.len(), 1);
        assert_eq!(merge_result.ignored.len(), 0);
        
        // Should NOT create a node in the graph
        assert!(engine.graph().node_by_id("cyoria").is_none());
    }
    
    #[test]
    fn test_merge_inference_low_confidence_ignores() {
        let mut engine = RealityEngine::new();
        
        let result = crate::ai::AnalysisResult {
            doc_id: "note1".to_string(),
            entities: vec![
                crate::ai::InferredEntity {
                    text: "something".to_string(),
                    label: "THING".to_string(),
                    start: 50,
                    end: 59,
                    confidence: 0.45, // Low confidence - should ignore
                },
            ],
            inference_time_ms: 50,
        };
        
        let merge_result = engine.merge_inference(&result, "world1");
        
        assert_eq!(merge_result.auto_promoted.len(), 0);
        assert_eq!(merge_result.suggested.len(), 0);
        assert_eq!(merge_result.ignored.len(), 1);
    }
    
    #[test]
    fn test_merge_inference_explicit_entity_override() {
        let mut engine = RealityEngine::new();
        
        // Pre-add an explicit entity "Zorian" to the graph
        let explicit_node = ConceptNode::new("zorian", "Zorian", "Character");
        engine.graph_mut().ensure_node(explicit_node);
        
        // Now try to infer the same entity via NER
        let result = crate::ai::AnalysisResult {
            doc_id: "note1".to_string(),
            entities: vec![
                crate::ai::InferredEntity {
                    text: "Zorian".to_string(),
                    label: "PERSON".to_string(), // Different type!
                    start: 0,
                    end: 6,
                    confidence: 0.98, // Very high confidence
                },
            ],
            inference_time_ms: 50,
        };
        
        let merge_result = engine.merge_inference(&result, "world1");
        
        // Should be skipped because explicit entity exists
        assert_eq!(merge_result.auto_promoted.len(), 0);
        assert_eq!(merge_result.suggested.len(), 0);
        assert_eq!(merge_result.skipped_explicit.len(), 1);
        
        // Original entity type should be preserved
        let node = engine.graph().node_by_id("zorian").unwrap();
        assert_eq!(node.kind, "Character"); // Not overwritten to "PERSON"
    }
    
    #[test]
    fn test_merge_inference_mixed_confidences() {
        let mut engine = RealityEngine::new();
        
        let result = crate::ai::AnalysisResult {
            doc_id: "note1".to_string(),
            entities: vec![
                crate::ai::InferredEntity {
                    text: "High".to_string(),
                    label: "TYPE".to_string(),
                    start: 0,
                    end: 4,
                    confidence: 0.92,
                },
                crate::ai::InferredEntity {
                    text: "Medium".to_string(),
                    label: "TYPE".to_string(),
                    start: 10,
                    end: 16,
                    confidence: 0.70,
                },
                crate::ai::InferredEntity {
                    text: "Low".to_string(),
                    label: "TYPE".to_string(),
                    start: 20,
                    end: 23,
                    confidence: 0.40,
                },
            ],
            inference_time_ms: 50,
        };
        
        let merge_result = engine.merge_inference(&result, "world1");
        
        assert_eq!(merge_result.auto_promoted.len(), 1, "Should auto-promote 1");
        assert_eq!(merge_result.suggested.len(), 1, "Should suggest 1");
        assert_eq!(merge_result.ignored.len(), 1, "Should ignore 1");
    }
    
    #[test]
    fn test_confidence_thresholds() {
        // Verify exact threshold boundaries
        let mut engine = RealityEngine::new();
        
        // Exactly 0.90 should auto-promote
        let result_90 = crate::ai::AnalysisResult {
            doc_id: "note1".to_string(),
            entities: vec![
                crate::ai::InferredEntity {
                    text: "Exact90".to_string(),
                    label: "TYPE".to_string(),
                    start: 0,
                    end: 7,
                    confidence: 0.90,
                },
            ],
            inference_time_ms: 50,
        };
        let r = engine.merge_inference(&result_90, "world1");
        assert_eq!(r.auto_promoted.len(), 1, "0.90 should auto-promote");
        
        // Exactly 0.60 should suggest
        let result_60 = crate::ai::AnalysisResult {
            doc_id: "note2".to_string(),
            entities: vec![
                crate::ai::InferredEntity {
                    text: "Exact60".to_string(),
                    label: "TYPE".to_string(),
                    start: 0,
                    end: 7,
                    confidence: 0.60,
                },
            ],
            inference_time_ms: 50,
        };
        let r = engine.merge_inference(&result_60, "world1");
        assert_eq!(r.suggested.len(), 1, "0.60 should suggest");
        
        // 0.59 should be ignored
        let result_59 = crate::ai::AnalysisResult {
            doc_id: "note3".to_string(),
            entities: vec![
                crate::ai::InferredEntity {
                    text: "Below60".to_string(),
                    label: "TYPE".to_string(),
                    start: 0,
                    end: 7,
                    confidence: 0.59,
                },
            ],
            inference_time_ms: 50,
        };
        let r = engine.merge_inference(&result_59, "world1");
        assert_eq!(r.ignored.len(), 1, "0.59 should be ignored");
    }
    
    #[test]
    fn test_get_entity_kinds() {
        let mut engine = RealityEngine::new();
        
        // Add some entities with different kinds
        engine.graph_mut().ensure_node(ConceptNode::new("zorian", "Zorian", "Character"));
        engine.graph_mut().ensure_node(ConceptNode::new("zach", "Zach", "Character"));
        engine.graph_mut().ensure_node(ConceptNode::new("kael", "Kael", "Character"));
        engine.graph_mut().ensure_node(ConceptNode::new("cyoria", "Cyoria", "Location"));
        engine.graph_mut().ensure_node(ConceptNode::new("sting", "Sting", "Item"));
        
        let kinds = engine.get_entity_kinds(10);
        
        // Should return kinds sorted by count
        assert_eq!(kinds.len(), 3);
        assert_eq!(kinds[0], "Character"); // 3 occurrences
        // Location and Item could be in any order (both have 1)
        assert!(kinds.contains(&"Location".to_string()));
        assert!(kinds.contains(&"Item".to_string()));
    }
    
    #[test]
    fn test_get_entity_kinds_limit() {
        let mut engine = RealityEngine::new();
        
        // Add entities with many different kinds
        engine.graph_mut().ensure_node(ConceptNode::new("a", "A", "Kind1"));
        engine.graph_mut().ensure_node(ConceptNode::new("b", "B", "Kind2"));
        engine.graph_mut().ensure_node(ConceptNode::new("c", "C", "Kind3"));
        engine.graph_mut().ensure_node(ConceptNode::new("d", "D", "Kind4"));
        engine.graph_mut().ensure_node(ConceptNode::new("e", "E", "Kind5"));
        
        let kinds = engine.get_entity_kinds(2);
        
        // Should only return top 2
        assert_eq!(kinds.len(), 2);
    }
}
