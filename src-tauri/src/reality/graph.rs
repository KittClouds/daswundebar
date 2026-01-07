//! In-memory semantic graph engine using petgraph
//!
//! This module provides a pure Rust graph structure for representing
//! semantic relationships between concepts. No serialization, no WASM —
//! just fast in-memory graph operations.

// Use petgraph from rustworkx-core to ensure version compatibility
use rustworkx_core::petgraph::graph::{DiGraph, NodeIndex, EdgeIndex};
use rustworkx_core::petgraph::visit::EdgeRef;
use rustworkx_core::petgraph::Direction;
use std::collections::HashMap;


// =============================================================================
// Types
// =============================================================================

/// A concept node in the semantic graph
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConceptNode {
    /// Stable identifier (e.g., entity ID from scanner)
    pub id: String,
    /// Display name
    pub label: String,
    /// Type/category: "Person", "Place", "Concept", "Item", etc.
    pub kind: String,
}

impl ConceptNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: kind.into(),
        }
    }
}

/// An edge/relationship between concepts
#[derive(Debug, Clone)]
pub struct ConceptEdge {
    /// Relation type: "owns", "created", "located_in", etc.
    pub relation: String,
    /// Confidence or strength (0.0 to 1.0)
    pub weight: f64,
    /// Edge kind for Phase 4 richer projections
    pub edge_kind: EdgeKind,
    // --- Provenance fields ---
    /// Source document ID (for multi-document graphs)
    pub source_doc: Option<String>,
    /// Source text span (start, end) in bytes
    pub source_span: Option<(u32, u32)>,
    /// Creation timestamp (epoch millis)
    pub created_at: Option<u64>,
}

/// Edge kinds for richer semantic relationships
///
/// Phase 4 of Evolution 1.5: Support for different relationship types
/// from the projection system.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    /// Standard relation (existing): "DEFEATED", "OWNS", "LOVES"
    Relation,
    
    /// Dialogue attribution: Speaker → Quote
    /// Used when someone says/shouts/whispers something
    Attribution {
        /// The dialogue verb: "said", "shouted", "whispered"
        verb: String,
    },
    
    /// Entity state transition: Entity → State
    /// "Frodo became invisible"
    StateTransition {
        /// What triggered the state change
        trigger: Option<String>,
    },
    
    /// Modified relation (from QuadPlus): SPO + modifiers
    /// "Gandalf defeated Sauron with magic in Mordor during the battle"
    ModifiedRelation {
        /// HOW: "with magic", "by force"
        manner: Option<String>,
        /// WHERE: "in Mordor", "at the bridge"
        location: Option<String>,
        /// WHEN: "during the battle", "after midnight"
        time: Option<String>,
    },
}

impl Default for EdgeKind {
    fn default() -> Self {
        EdgeKind::Relation
    }
}

impl ConceptEdge {
    pub fn new(relation: impl Into<String>, weight: f64) -> Self {
        Self {
            relation: relation.into(),
            weight,
            edge_kind: EdgeKind::Relation,
            source_doc: None,
            source_span: None,
            created_at: None,
        }
    }
    
    /// Create an edge with default weight (1.0)
    pub fn unweighted(relation: impl Into<String>) -> Self {
        Self::new(relation, 1.0)
    }

    /// Builder: set source document
    pub fn with_doc(mut self, doc_id: impl Into<String>) -> Self {
        self.source_doc = Some(doc_id.into());
        self
    }

    /// Builder: set source text span
    pub fn with_span(mut self, start: u32, end: u32) -> Self {
        self.source_span = Some((start, end));
        self
    }

    /// Builder: set creation timestamp
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.created_at = Some(ts);
        self
    }
    
    /// Builder: set edge kind
    pub fn with_kind(mut self, kind: EdgeKind) -> Self {
        self.edge_kind = kind;
        self
    }
    
    /// Create an attribution edge (Speaker → Quote)
    pub fn attribution(verb: impl Into<String>) -> Self {
        Self {
            relation: "SAID".to_string(),
            weight: 1.0,
            edge_kind: EdgeKind::Attribution { verb: verb.into() },
            source_doc: None,
            source_span: None,
            created_at: None,
        }
    }
    
    /// Create a state transition edge (Entity → State)
    pub fn state_transition(to_state: impl Into<String>, trigger: Option<String>) -> Self {
        Self {
            relation: format!("BECAME_{}", to_state.into().to_uppercase()),
            weight: 1.0,
            edge_kind: EdgeKind::StateTransition { trigger },
            source_doc: None,
            source_span: None,
            created_at: None,
        }
    }
    
    /// Create a modified relation edge (QuadPlus)
    pub fn modified_relation(
        relation: impl Into<String>,
        manner: Option<String>,
        location: Option<String>,
        time: Option<String>,
    ) -> Self {
        Self {
            relation: relation.into(),
            weight: 1.0,
            edge_kind: EdgeKind::ModifiedRelation { manner, location, time },
            source_doc: None,
            source_span: None,
            created_at: None,
        }
    }
}


// =============================================================================
// ConceptGraph
// =============================================================================

/// The semantic graph — pure in-memory, no serialization
/// 
/// Uses a directed graph (DiGraph) where:
/// - Nodes are ConceptNode (entities/concepts)
/// - Edges are ConceptEdge (relationships with direction)
pub struct ConceptGraph {
    /// The underlying petgraph structure
    graph: DiGraph<ConceptNode, ConceptEdge>,
    /// Fast lookup: node ID → petgraph NodeIndex
    id_to_index: HashMap<String, NodeIndex>,
}

impl Default for ConceptGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ConceptGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            id_to_index: HashMap::new(),
        }
    }
    
    /// Add a node or get existing node's index
    /// 
    /// If a node with the same ID exists, returns its index.
    /// Otherwise, adds the node and returns the new index.
    pub fn ensure_node(&mut self, node: ConceptNode) -> NodeIndex {
        if let Some(&idx) = self.id_to_index.get(&node.id) {
            return idx;
        }
        
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.id_to_index.insert(id, idx);
        idx
    }
    
    /// Add an edge between two nodes (by ID)
    /// 
    /// Returns the EdgeIndex if both nodes exist, None otherwise.
    pub fn add_edge(&mut self, source_id: &str, target_id: &str, edge: ConceptEdge) -> Option<EdgeIndex> {
        let source_idx = self.id_to_index.get(source_id)?;
        let target_idx = self.id_to_index.get(target_id)?;
        
        Some(self.graph.add_edge(*source_idx, *target_idx, edge))
    }
    
    /// Add an edge, creating nodes if they don't exist
    /// 
    /// This is a convenience method that ensures both nodes exist before adding the edge.
    pub fn add_edge_with_nodes(
        &mut self,
        source: ConceptNode,
        target: ConceptNode,
        edge: ConceptEdge,
    ) -> EdgeIndex {
        let source_idx = self.ensure_node(source);
        let target_idx = self.ensure_node(target);
        self.graph.add_edge(source_idx, target_idx, edge)
    }
    
    /// Find a node by ID
    pub fn get_node(&self, id: &str) -> Option<&ConceptNode> {
        let idx = self.id_to_index.get(id)?;
        self.graph.node_weight(*idx)
    }
    
    /// Get the NodeIndex for a given ID
    pub fn get_index(&self, id: &str) -> Option<NodeIndex> {
        self.id_to_index.get(id).copied()
    }
    
    /// Get all outgoing edges from a node
    /// 
    /// Returns Vec of (target_node, edge)
    pub fn outgoing_edges(&self, id: &str) -> Vec<(&ConceptNode, &ConceptEdge)> {
        let Some(&idx) = self.id_to_index.get(id) else {
            return vec![];
        };
        
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .filter_map(|edge_ref| {
                let target_node = self.graph.node_weight(edge_ref.target())?;
                Some((target_node, edge_ref.weight()))
            })
            .collect()
    }
    
    /// Get all incoming edges to a node
    /// 
    /// Returns Vec of (source_node, edge)
    pub fn incoming_edges(&self, id: &str) -> Vec<(&ConceptNode, &ConceptEdge)> {
        let Some(&idx) = self.id_to_index.get(id) else {
            return vec![];
        };
        
        self.graph
            .edges_directed(idx, Direction::Incoming)
            .filter_map(|edge_ref| {
                let source_node = self.graph.node_weight(edge_ref.source())?;
                Some((source_node, edge_ref.weight()))
            })
            .collect()
    }
    
    /// Get all neighbors of a node (both directions)
    pub fn neighbors(&self, id: &str) -> Vec<&ConceptNode> {
        let Some(&idx) = self.id_to_index.get(id) else {
            return vec![];
        };
        
        self.graph
            .neighbors_undirected(idx)
            .filter_map(|neighbor_idx| self.graph.node_weight(neighbor_idx))
            .collect()
    }
    
    /// Count of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }
    
    /// Count of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
    
    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }
    
    /// Clear all nodes and edges
    pub fn clear(&mut self) {
        self.graph.clear();
        self.id_to_index.clear();
    }
    
    /// Iterate over all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &ConceptNode> {
        self.graph.node_weights()
    }
    
    /// Iterate over all edges with their source and target
    pub fn edges(&self) -> impl Iterator<Item = (&ConceptNode, &ConceptNode, &ConceptEdge)> {
        self.graph.edge_indices().filter_map(|edge_idx| {
            let (source_idx, target_idx) = self.graph.edge_endpoints(edge_idx)?;
            let source = self.graph.node_weight(source_idx)?;
            let target = self.graph.node_weight(target_idx)?;
            let edge = self.graph.edge_weight(edge_idx)?;
            Some((source, target, edge))
        })
    }

    /// Get raw petgraph reference for advanced algorithms
    pub fn raw_graph(&self) -> &DiGraph<ConceptNode, ConceptEdge> {
        &self.graph
    }
    
    /// Alias for raw_graph() - used by algorithms module
    pub fn graph(&self) -> &DiGraph<ConceptNode, ConceptEdge> {
        &self.graph
    }


    // =========================================================================
    // Subgraph Extraction
    // =========================================================================

    /// Extract a subgraph centered on a node, up to `depth` hops away
    /// 
    /// Uses BFS to find all nodes within `depth` edges of the center.
    /// Returns a new ConceptGraph containing only those nodes and their connecting edges.
    pub fn subgraph(&self, center_id: &str, depth: usize) -> ConceptGraph {
        use std::collections::{HashSet, VecDeque};

        let mut result = ConceptGraph::new();
        
        let Some(&center_idx) = self.id_to_index.get(center_id) else {
            return result;
        };

        // BFS to find nodes within depth
        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
        
        queue.push_back((center_idx, 0));
        visited.insert(center_idx);

        while let Some((current_idx, current_depth)) = queue.pop_front() {
            // Add node to result
            if let Some(node) = self.graph.node_weight(current_idx) {
                result.ensure_node(node.clone());
            }

            // If we haven't reached max depth, explore neighbors
            if current_depth < depth {
                for neighbor_idx in self.graph.neighbors_undirected(current_idx) {
                    if !visited.contains(&neighbor_idx) {
                        visited.insert(neighbor_idx);
                        queue.push_back((neighbor_idx, current_depth + 1));
                    }
                }
            }
        }

        // Add edges between visited nodes
        for edge_idx in self.graph.edge_indices() {
            if let Some((source_idx, target_idx)) = self.graph.edge_endpoints(edge_idx) {
                if visited.contains(&source_idx) && visited.contains(&target_idx) {
                    if let (Some(source), Some(target), Some(edge)) = (
                        self.graph.node_weight(source_idx),
                        self.graph.node_weight(target_idx),
                        self.graph.edge_weight(edge_idx),
                    ) {
                        result.add_edge(&source.id, &target.id, edge.clone());
                    }
                }
            }
        }

        result
    }

    // =========================================================================
    // Centrality & Connectivity (via rustworkx-core)
    // =========================================================================

    /// Get degree centrality for all nodes
    /// 
    /// Degree = (in_degree + out_degree) / (2 * (n - 1))
    /// Higher values = more connected nodes
    pub fn centrality_degree(&self) -> Vec<(String, f64)> {
        let n = self.graph.node_count();
        if n <= 1 {
            return self.nodes().map(|node| (node.id.clone(), 0.0)).collect();
        }

        let normalizer = 2.0 * (n - 1) as f64;
        
        self.graph.node_indices().filter_map(|idx| {
            let node = self.graph.node_weight(idx)?;
            let in_deg = self.graph.edges_directed(idx, Direction::Incoming).count();
            let out_deg = self.graph.edges_directed(idx, Direction::Outgoing).count();
            let centrality = (in_deg + out_deg) as f64 / normalizer;
            Some((node.id.clone(), centrality))
        }).collect()
    }

    /// Find isolated nodes (no connections)
    /// 
    /// These are "orphan" entities that appear but have no relationships.
    pub fn orphan_nodes(&self) -> Vec<&ConceptNode> {
        use rustworkx_core::connectivity::isolates;
        
        // isolates returns Vec<NodeIndex>
        isolates(&self.graph)
            .into_iter()
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect()
    }

    /// Count connected components (narrative threads)
    /// 
    /// Returns the number of disconnected subgraphs.
    /// A value > 1 means the narrative is fragmented.
    /// 
    /// Note: Uses undirected graph interpretation for connectivity analysis.
    pub fn connected_component_count(&self) -> usize {
        use rustworkx_core::connectivity::number_connected_components;
        use rustworkx_core::petgraph::graph::UnGraph;
        
        if self.graph.node_count() == 0 {
            return 0;
        }
        
        // Convert to undirected for connectivity analysis
        let mut undirected: UnGraph<(), ()> = UnGraph::new_undirected();
        
        let node_map: HashMap<NodeIndex, _> = self.graph.node_indices()
            .map(|idx| (idx, undirected.add_node(())))
            .collect();
        
        for edge_ref in self.graph.edge_references() {
            if let (Some(&src), Some(&tgt)) = (
                node_map.get(&edge_ref.source()),
                node_map.get(&edge_ref.target())
            ) {
                if !undirected.contains_edge(src, tgt) {
                    undirected.add_edge(src, tgt, ());
                }
            }
        }
        
        number_connected_components(&undirected)
    }


    /// Find articulation points (critical nodes)
    /// 
    /// These are nodes that, if removed, would fragment the graph.
    /// In narrative terms: "keystone" characters/concepts.
    /// 
    /// Note: articulation_points works on undirected graphs, so we treat
    /// edges as bidirectional for this analysis.
    pub fn critical_nodes(&self) -> Vec<&ConceptNode> {
        use rustworkx_core::connectivity::articulation_points;
        use rustworkx_core::petgraph::graph::UnGraph;
        
        // Convert directed graph to undirected for articulation point analysis
        let mut undirected: UnGraph<(), ()> = UnGraph::new_undirected();
        
        // Add all nodes
        let node_map: HashMap<NodeIndex, _> = self.graph.node_indices()
            .map(|idx| (idx, undirected.add_node(())))
            .collect();
        
        // Add all edges (both directions become one undirected edge)
        for edge_ref in self.graph.edge_references() {
            if let (Some(&src), Some(&tgt)) = (
                node_map.get(&edge_ref.source()),
                node_map.get(&edge_ref.target())
            ) {
                // Check if edge already exists (avoid duplicates)
                if !undirected.contains_edge(src, tgt) {
                    undirected.add_edge(src, tgt, ());
                }
            }
        }
        
        // articulation_points returns HashSet<NodeIndex>
        let ap_set = articulation_points(&undirected, None);
        
        // Map back to original graph's nodes
        let reverse_map: HashMap<_, _> = node_map.iter()
            .map(|(orig, und)| (*und, *orig))
            .collect();
        
        ap_set.into_iter()
            .filter_map(|und_idx| {
                let orig_idx = reverse_map.get(&und_idx)?;
                self.graph.node_weight(*orig_idx)
            })
            .collect()
    }


    /// Compute narrative health score (0-100)
    /// 
    /// Based on:
    /// - Orphan penalty: -5 per orphan node
    /// - Fragmentation penalty: -20 per extra component beyond 1
    /// - Critical node bonus: +5 per articulation point (shows structure)
    pub fn narrative_health_score(&self) -> u32 {
        let n = self.node_count();
        if n == 0 {
            return 100;
        }

        let orphans = self.orphan_nodes().len();
        let components = self.connected_component_count();
        let critical = self.critical_nodes().len();

        let mut score: i32 = 100;
        
        // Orphan penalty (capped at 50)
        score -= (orphans as i32 * 5).min(50);
        
        // Fragmentation penalty
        if components > 1 {
            score -= ((components - 1) as i32 * 20).min(40);
        }
        
        // Critical node bonus (shows narrative structure, up to +20)
        score += (critical as i32 * 5).min(20);

        score.clamp(0, 100) as u32
    }
}


// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    fn make_node(id: &str, label: &str, kind: &str) -> ConceptNode {
        ConceptNode::new(id, label, kind)
    }
    
    // -------------------------------------------------------------------------
    // Node Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_graph_add_node() {
        let mut graph = ConceptGraph::new();
        
        let node = make_node("frodo", "Frodo Baggins", "Person");
        let idx1 = graph.ensure_node(node.clone());
        
        assert_eq!(graph.node_count(), 1);
        
        // Adding same ID should return existing index
        let idx2 = graph.ensure_node(make_node("frodo", "Frodo", "Character"));
        assert_eq!(idx1, idx2, "Same ID should return same index");
        assert_eq!(graph.node_count(), 1, "Should not create duplicate");
    }
    
    #[test]
    fn test_graph_no_duplicate_nodes() {
        let mut graph = ConceptGraph::new();
        
        // Add multiple nodes with same ID
        for i in 0..5 {
            graph.ensure_node(make_node("only-one", &format!("Label {}", i), "Test"));
        }
        
        assert_eq!(graph.node_count(), 1, "Should only have one node despite 5 ensure_node calls");
        
        // Original label should be preserved
        let node = graph.get_node("only-one").unwrap();
        assert_eq!(node.label, "Label 0", "First label should be preserved");
    }
    
    #[test]
    fn test_graph_get_node() {
        let mut graph = ConceptGraph::new();
        
        graph.ensure_node(make_node("gandalf", "Gandalf the Grey", "Wizard"));
        
        let found = graph.get_node("gandalf");
        assert!(found.is_some());
        assert_eq!(found.unwrap().label, "Gandalf the Grey");
        
        let not_found = graph.get_node("saruman");
        assert!(not_found.is_none());
    }
    
    // -------------------------------------------------------------------------
    // Edge Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_graph_add_edge() {
        let mut graph = ConceptGraph::new();
        
        graph.ensure_node(make_node("frodo", "Frodo", "Person"));
        graph.ensure_node(make_node("sting", "Sting", "Item"));
        
        let edge_idx = graph.add_edge("frodo", "sting", ConceptEdge::unweighted("owns"));
        
        assert!(edge_idx.is_some(), "Edge should be created");
        assert_eq!(graph.edge_count(), 1);
    }
    
    #[test]
    fn test_graph_add_edge_missing_nodes() {
        let mut graph = ConceptGraph::new();
        
        // Try to add edge without nodes
        let result = graph.add_edge("a", "b", ConceptEdge::unweighted("test"));
        assert!(result.is_none(), "Should fail when nodes don't exist");
        
        // Add one node, still should fail
        graph.ensure_node(make_node("a", "A", "Test"));
        let result = graph.add_edge("a", "b", ConceptEdge::unweighted("test"));
        assert!(result.is_none(), "Should fail when target doesn't exist");
    }
    
    #[test]
    fn test_graph_add_edge_with_nodes() {
        let mut graph = ConceptGraph::new();
        
        graph.add_edge_with_nodes(
            make_node("sam", "Samwise", "Person"),
            make_node("frodo", "Frodo", "Person"),
            ConceptEdge::new("serves", 0.95),
        );
        
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }
    
    // -------------------------------------------------------------------------
    // Query Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_graph_query_outgoing() {
        let mut graph = ConceptGraph::new();
        
        // Frodo owns multiple items
        graph.ensure_node(make_node("frodo", "Frodo", "Person"));
        graph.ensure_node(make_node("sting", "Sting", "Item"));
        graph.ensure_node(make_node("ring", "The One Ring", "Item"));
        graph.ensure_node(make_node("mithril", "Mithril Coat", "Item"));
        
        graph.add_edge("frodo", "sting", ConceptEdge::unweighted("owns"));
        graph.add_edge("frodo", "ring", ConceptEdge::unweighted("carries"));
        graph.add_edge("frodo", "mithril", ConceptEdge::unweighted("wears"));
        
        let outgoing = graph.outgoing_edges("frodo");
        
        assert_eq!(outgoing.len(), 3, "Frodo should have 3 outgoing edges");
        
        // Check that we can find all items
        let target_ids: Vec<&str> = outgoing.iter().map(|(node, _)| node.id.as_str()).collect();
        assert!(target_ids.contains(&"sting"));
        assert!(target_ids.contains(&"ring"));
        assert!(target_ids.contains(&"mithril"));
    }
    
    #[test]
    fn test_graph_query_incoming() {
        let mut graph = ConceptGraph::new();
        
        // Multiple people own the ring at different times
        graph.ensure_node(make_node("ring", "The One Ring", "Item"));
        graph.ensure_node(make_node("sauron", "Sauron", "Villain"));
        graph.ensure_node(make_node("isildur", "Isildur", "Person"));
        graph.ensure_node(make_node("gollum", "Gollum", "Creature"));
        graph.ensure_node(make_node("bilbo", "Bilbo", "Person"));
        graph.ensure_node(make_node("frodo", "Frodo", "Person"));
        
        graph.add_edge("sauron", "ring", ConceptEdge::unweighted("created"));
        graph.add_edge("isildur", "ring", ConceptEdge::unweighted("took"));
        graph.add_edge("gollum", "ring", ConceptEdge::unweighted("found"));
        graph.add_edge("bilbo", "ring", ConceptEdge::unweighted("won"));
        graph.add_edge("frodo", "ring", ConceptEdge::unweighted("inherited"));
        
        let incoming = graph.incoming_edges("ring");
        
        assert_eq!(incoming.len(), 5, "Ring should have 5 incoming edges");
    }
    
    #[test]
    fn test_graph_neighbors() {
        let mut graph = ConceptGraph::new();
        
        graph.ensure_node(make_node("frodo", "Frodo", "Person"));
        graph.ensure_node(make_node("sam", "Sam", "Person"));
        graph.ensure_node(make_node("mordor", "Mordor", "Place"));
        graph.ensure_node(make_node("gandalf", "Gandalf", "Wizard"));
        
        // Frodo has outgoing edges to Sam and Mordor
        graph.add_edge("frodo", "sam", ConceptEdge::unweighted("friend_of"));
        graph.add_edge("frodo", "mordor", ConceptEdge::unweighted("traveled_to"));
        // Gandalf has an edge TO Frodo (incoming for Frodo)
        graph.add_edge("gandalf", "frodo", ConceptEdge::unweighted("guided"));
        
        let neighbors = graph.neighbors("frodo");
        
        // Note: neighbors_undirected returns each neighbor once per edge
        // So Frodo should have at least 3 neighbors: Sam, Mordor, Gandalf
        assert!(neighbors.len() >= 3, "Frodo should have at least 3 neighbors, got {}", neighbors.len());
    }
    
    // -------------------------------------------------------------------------
    // Iteration Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_graph_iterate_nodes() {
        let mut graph = ConceptGraph::new();
        
        graph.ensure_node(make_node("a", "A", "Test"));
        graph.ensure_node(make_node("b", "B", "Test"));
        graph.ensure_node(make_node("c", "C", "Test"));
        
        let node_ids: Vec<&str> = graph.nodes().map(|n| n.id.as_str()).collect();
        
        assert_eq!(node_ids.len(), 3);
        assert!(node_ids.contains(&"a"));
        assert!(node_ids.contains(&"b"));
        assert!(node_ids.contains(&"c"));
    }
    
    #[test]
    fn test_graph_iterate_edges() {
        let mut graph = ConceptGraph::new();
        
        graph.add_edge_with_nodes(
            make_node("a", "A", "Test"),
            make_node("b", "B", "Test"),
            ConceptEdge::unweighted("connects"),
        );
        graph.add_edge_with_nodes(
            make_node("b", "B", "Test"),
            make_node("c", "C", "Test"),
            ConceptEdge::unweighted("leads_to"),
        );
        
        let edges: Vec<_> = graph.edges().collect();
        
        assert_eq!(edges.len(), 2);
    }
    
    // -------------------------------------------------------------------------
    // Utility Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_graph_clear() {
        let mut graph = ConceptGraph::new();
        
        graph.ensure_node(make_node("a", "A", "Test"));
        graph.ensure_node(make_node("b", "B", "Test"));
        graph.add_edge("a", "b", ConceptEdge::unweighted("test"));
        
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        
        graph.clear();
        
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.is_empty());
    }
    
    #[test]
    fn test_graph_edge_weights() {
        let mut graph = ConceptGraph::new();
        
        graph.ensure_node(make_node("a", "A", "Test"));
        graph.ensure_node(make_node("b", "B", "Test"));
        
        graph.add_edge("a", "b", ConceptEdge::new("high_confidence", 0.95));
        
        let edges = graph.outgoing_edges("a");
        assert_eq!(edges.len(), 1);
        
        let (_, edge) = &edges[0];
        assert_eq!(edge.relation, "high_confidence");
        assert!((edge.weight - 0.95).abs() < f64::EPSILON);
    }

    // -------------------------------------------------------------------------
    // Edge Provenance Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_edge_provenance_builder() {
        let edge = ConceptEdge::unweighted("owns")
            .with_doc("chapter1.md")
            .with_span(100, 150)
            .with_timestamp(1704067200000);

        assert_eq!(edge.relation, "owns");
        assert_eq!(edge.source_doc, Some("chapter1.md".to_string()));
        assert_eq!(edge.source_span, Some((100, 150)));
        assert_eq!(edge.created_at, Some(1704067200000));
    }

    // -------------------------------------------------------------------------
    // Subgraph Extraction Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_subgraph_depth_0() {
        let mut graph = ConceptGraph::new();
        
        graph.ensure_node(make_node("a", "A", "Test"));
        graph.ensure_node(make_node("b", "B", "Test"));
        graph.add_edge("a", "b", ConceptEdge::unweighted("connects"));

        // Depth 0 = only the center node
        let sub = graph.subgraph("a", 0);
        assert_eq!(sub.node_count(), 1);
        assert!(sub.get_node("a").is_some());
        assert!(sub.get_node("b").is_none());
    }

    #[test]
    fn test_subgraph_depth_1() {
        let mut graph = ConceptGraph::new();
        
        graph.ensure_node(make_node("center", "Center", "Test"));
        graph.ensure_node(make_node("n1", "N1", "Test"));
        graph.ensure_node(make_node("n2", "N2", "Test"));
        graph.ensure_node(make_node("far", "Far", "Test"));
        
        graph.add_edge("center", "n1", ConceptEdge::unweighted("to"));
        graph.add_edge("center", "n2", ConceptEdge::unweighted("to"));
        graph.add_edge("n1", "far", ConceptEdge::unweighted("to"));

        // Depth 1 = center + immediate neighbors
        let sub = graph.subgraph("center", 1);
        assert_eq!(sub.node_count(), 3); // center, n1, n2
        assert!(sub.get_node("center").is_some());
        assert!(sub.get_node("n1").is_some());
        assert!(sub.get_node("n2").is_some());
        assert!(sub.get_node("far").is_none());
    }

    #[test]
    fn test_subgraph_includes_edges() {
        let mut graph = ConceptGraph::new();
        
        graph.add_edge_with_nodes(
            make_node("a", "A", "Test"),
            make_node("b", "B", "Test"),
            ConceptEdge::unweighted("connected"),
        );

        let sub = graph.subgraph("a", 1);
        assert_eq!(sub.node_count(), 2);
        assert_eq!(sub.edge_count(), 1);
    }

    // -------------------------------------------------------------------------
    // Centrality & Connectivity Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_degree_centrality() {
        let mut graph = ConceptGraph::new();
        
        // Hub pattern: center connects to all others
        graph.ensure_node(make_node("hub", "Hub", "Test"));
        graph.ensure_node(make_node("a", "A", "Test"));
        graph.ensure_node(make_node("b", "B", "Test"));
        graph.ensure_node(make_node("c", "C", "Test"));
        
        graph.add_edge("hub", "a", ConceptEdge::unweighted("to"));
        graph.add_edge("hub", "b", ConceptEdge::unweighted("to"));
        graph.add_edge("hub", "c", ConceptEdge::unweighted("to"));

        let centrality = graph.centrality_degree();
        
        // Hub should have highest centrality
        let hub_centrality = centrality.iter().find(|(id, _)| id == "hub").unwrap().1;
        let a_centrality = centrality.iter().find(|(id, _)| id == "a").unwrap().1;
        
        assert!(hub_centrality > a_centrality, "Hub should be more central");
    }

    #[test]
    fn test_orphan_nodes() {
        let mut graph = ConceptGraph::new();
        
        // Connected pair
        graph.add_edge_with_nodes(
            make_node("a", "A", "Test"),
            make_node("b", "B", "Test"),
            ConceptEdge::unweighted("connected"),
        );
        
        // Orphan
        graph.ensure_node(make_node("orphan", "Orphan", "Test"));

        let orphans = graph.orphan_nodes();
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].id, "orphan");
    }

    #[test]
    fn test_connected_components() {
        let mut graph = ConceptGraph::new();
        
        // Component 1: a-b
        graph.add_edge_with_nodes(
            make_node("a", "A", "Test"),
            make_node("b", "B", "Test"),
            ConceptEdge::unweighted("connected"),
        );
        
        // Component 2: c-d
        graph.add_edge_with_nodes(
            make_node("c", "C", "Test"),
            make_node("d", "D", "Test"),
            ConceptEdge::unweighted("connected"),
        );

        let count = graph.connected_component_count();
        assert_eq!(count, 2, "Should have 2 connected components");
    }

    #[test]
    fn test_critical_nodes() {
        let mut graph = ConceptGraph::new();
        
        // Linear chain: a -> b -> c
        // b is the critical node (bridge)
        graph.ensure_node(make_node("a", "A", "Test"));
        graph.ensure_node(make_node("b", "B", "Test"));
        graph.ensure_node(make_node("c", "C", "Test"));
        
        graph.add_edge("a", "b", ConceptEdge::unweighted("to"));
        graph.add_edge("b", "c", ConceptEdge::unweighted("to"));

        let critical = graph.critical_nodes();
        
        // In a linear chain a-b-c, b is an articulation point
        let critical_ids: Vec<&str> = critical.iter().map(|n| n.id.as_str()).collect();
        assert!(critical_ids.contains(&"b"), "b should be critical node");
    }

    #[test]
    fn test_narrative_health_score() {
        // Healthy graph: single component, no orphans
        let mut healthy = ConceptGraph::new();
        healthy.add_edge_with_nodes(
            make_node("a", "A", "Test"),
            make_node("b", "B", "Test"),
            ConceptEdge::unweighted("connected"),
        );
        healthy.add_edge_with_nodes(
            make_node("b", "B", "Test"),
            make_node("c", "C", "Test"),
            ConceptEdge::unweighted("connected"),
        );

        let score = healthy.narrative_health_score();
        assert!(score >= 80, "Healthy graph should score >= 80, got {}", score);

        // Fragmented graph: multiple components
        let mut fragmented = ConceptGraph::new();
        fragmented.add_edge_with_nodes(
            make_node("a", "A", "Test"),
            make_node("b", "B", "Test"),
            ConceptEdge::unweighted("connected"),
        );
        fragmented.ensure_node(make_node("orphan1", "O1", "Test"));
        fragmented.ensure_node(make_node("orphan2", "O2", "Test"));

        let frag_score = fragmented.narrative_health_score();
        assert!(frag_score < score, "Fragmented graph should score lower");
    }

    // -------------------------------------------------------------------------
    // Phase 4: EdgeKind Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_edge_kind_default() {
        let edge = ConceptEdge::unweighted("owns");
        assert!(matches!(edge.edge_kind, EdgeKind::Relation));
    }

    #[test]
    fn test_edge_kind_attribution() {
        let edge = ConceptEdge::attribution("shouted");
        
        assert_eq!(edge.relation, "SAID");
        assert!(matches!(
            edge.edge_kind, 
            EdgeKind::Attribution { verb } if verb == "shouted"
        ));
    }

    #[test]
    fn test_edge_kind_state_transition() {
        let edge = ConceptEdge::state_transition("invisible", Some("after putting on the Ring".to_string()));
        
        assert_eq!(edge.relation, "BECAME_INVISIBLE");
        assert!(matches!(
            edge.edge_kind,
            EdgeKind::StateTransition { trigger: Some(t) } if t.contains("Ring")
        ));
    }

    #[test]
    fn test_edge_kind_state_transition_no_trigger() {
        let edge = ConceptEdge::state_transition("angry", None);
        
        assert_eq!(edge.relation, "BECAME_ANGRY");
        assert!(matches!(
            edge.edge_kind,
            EdgeKind::StateTransition { trigger: None }
        ));
    }

    #[test]
    fn test_edge_kind_modified_relation() {
        let edge = ConceptEdge::modified_relation(
            "DEFEATED",
            Some("with magic".to_string()),
            Some("in Mordor".to_string()),
            Some("during the battle".to_string()),
        );
        
        assert_eq!(edge.relation, "DEFEATED");
        match edge.edge_kind {
            EdgeKind::ModifiedRelation { manner, location, time } => {
                assert_eq!(manner.as_deref(), Some("with magic"));
                assert_eq!(location.as_deref(), Some("in Mordor"));
                assert_eq!(time.as_deref(), Some("during the battle"));
            }
            _ => panic!("Expected ModifiedRelation edge kind"),
        }
    }

    #[test]
    fn test_edge_kind_modified_relation_partial() {
        let edge = ConceptEdge::modified_relation(
            "ATTACKED",
            None,
            Some("at the bridge".to_string()),
            None,
        );
        
        match edge.edge_kind {
            EdgeKind::ModifiedRelation { manner, location, time } => {
                assert!(manner.is_none());
                assert_eq!(location.as_deref(), Some("at the bridge"));
                assert!(time.is_none());
            }
            _ => panic!("Expected ModifiedRelation edge kind"),
        }
    }

    #[test]
    fn test_edge_with_kind_builder() {
        let edge = ConceptEdge::unweighted("custom")
            .with_kind(EdgeKind::Attribution { verb: "whispered".to_string() });
        
        assert!(matches!(
            edge.edge_kind,
            EdgeKind::Attribution { verb } if verb == "whispered"
        ));
    }

    #[test]
    fn test_graph_with_edge_kinds() {
        let mut graph = ConceptGraph::new();
        
        // Add relation edge
        graph.add_edge_with_nodes(
            make_node("frodo", "Frodo", "Person"),
            make_node("ring", "Ring", "Item"),
            ConceptEdge::unweighted("owns"),
        );
        
        // Add attribution edge
        graph.add_edge_with_nodes(
            make_node("gandalf", "Gandalf", "Wizard"),
            make_node("quote_1", "You shall not pass!", "Quote"),
            ConceptEdge::attribution("shouted"),
        );
        
        // Add state transition edge
        graph.add_edge_with_nodes(
            make_node("frodo", "Frodo", "Person"),
            make_node("invisible", "invisible", "State"),
            ConceptEdge::state_transition("invisible", Some("after putting on Ring".to_string())),
        );
        
        assert_eq!(graph.node_count(), 5);
        assert_eq!(graph.edge_count(), 3);
        
        // Verify edge kinds are preserved
        let edges: Vec<_> = graph.edges().collect();
        
        let has_relation = edges.iter().any(|(_, _, e)| matches!(e.edge_kind, EdgeKind::Relation));
        let has_attribution = edges.iter().any(|(_, _, e)| matches!(e.edge_kind, EdgeKind::Attribution { .. }));
        let has_state = edges.iter().any(|(_, _, e)| matches!(e.edge_kind, EdgeKind::StateTransition { .. }));
        
        assert!(has_relation, "Should have Relation edge");
        assert!(has_attribution, "Should have Attribution edge");
        assert!(has_state, "Should have StateTransition edge");
    }
}

