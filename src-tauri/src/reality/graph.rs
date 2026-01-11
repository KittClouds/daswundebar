//! In-memory semantic graph engine (petgraph-free)
//!
//! This module provides a pure Rust graph structure for representing
//! semantic relationships between concepts. Uses HashMap-based storage
//! instead of petgraph for simplicity and to remove external dependencies.
//!
//! **V2: Removed petgraph dependency. Uses simple HashMap-based adjacency lists.**

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
// NodeIndex (simple replacement for petgraph::NodeIndex)
// =============================================================================

/// Simple node index (usize wrapper for compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeIndex(pub usize);

impl NodeIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }
    
    pub fn index(&self) -> usize {
        self.0
    }
}

/// Simple edge index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeIndex(pub usize);

// =============================================================================
// Internal Edge Storage
// =============================================================================

/// Internal edge representation
#[derive(Debug, Clone)]
struct StoredEdge {
    source_idx: NodeIndex,
    target_idx: NodeIndex,
    edge: ConceptEdge,
}

// =============================================================================
// ConceptGraph
// =============================================================================

/// The semantic graph — pure in-memory, no external dependencies
/// 
/// Uses HashMap-based adjacency lists instead of petgraph.
pub struct ConceptGraph {
    /// Node storage: index → node
    nodes: Vec<ConceptNode>,
    /// Fast lookup: node ID → index
    id_to_index: HashMap<String, NodeIndex>,
    /// Edge storage
    edges: Vec<StoredEdge>,
    /// Outgoing edges: source_idx → [edge indices]
    outgoing: HashMap<NodeIndex, Vec<usize>>,
    /// Incoming edges: target_idx → [edge indices]
    incoming: HashMap<NodeIndex, Vec<usize>>,
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
            nodes: Vec::new(),
            id_to_index: HashMap::new(),
            edges: Vec::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
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
        let idx = NodeIndex(self.nodes.len());
        self.nodes.push(node);
        self.id_to_index.insert(id, idx);
        idx
    }
    
    /// Add an edge between two nodes (by ID)
    /// 
    /// Returns the EdgeIndex if both nodes exist, None otherwise.
    pub fn add_edge(&mut self, source_id: &str, target_id: &str, edge: ConceptEdge) -> Option<EdgeIndex> {
        let source_idx = *self.id_to_index.get(source_id)?;
        let target_idx = *self.id_to_index.get(target_id)?;
        
        let edge_idx = self.edges.len();
        self.edges.push(StoredEdge {
            source_idx,
            target_idx,
            edge,
        });
        
        self.outgoing.entry(source_idx).or_default().push(edge_idx);
        self.incoming.entry(target_idx).or_default().push(edge_idx);
        
        Some(EdgeIndex(edge_idx))
    }
    
    /// Add an edge, creating nodes if they don't exist
    pub fn add_edge_with_nodes(
        &mut self,
        source: ConceptNode,
        target: ConceptNode,
        edge: ConceptEdge,
    ) -> EdgeIndex {
        let source_idx = self.ensure_node(source);
        let target_idx = self.ensure_node(target);
        
        let edge_idx = self.edges.len();
        self.edges.push(StoredEdge {
            source_idx,
            target_idx,
            edge,
        });
        
        self.outgoing.entry(source_idx).or_default().push(edge_idx);
        self.incoming.entry(target_idx).or_default().push(edge_idx);
        
        EdgeIndex(edge_idx)
    }
    
    /// Find a node by ID
    pub fn get_node(&self, id: &str) -> Option<&ConceptNode> {
        let idx = self.id_to_index.get(id)?;
        self.nodes.get(idx.0)
    }
    
    /// Alias for get_node - find a node by ID
    pub fn node_by_id(&self, id: &str) -> Option<&ConceptNode> {
        self.get_node(id)
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
        
        let Some(edge_indices) = self.outgoing.get(&idx) else {
            return vec![];
        };
        
        edge_indices
            .iter()
            .filter_map(|&edge_idx| {
                let stored = self.edges.get(edge_idx)?;
                let target = self.nodes.get(stored.target_idx.0)?;
                Some((target, &stored.edge))
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
        
        let Some(edge_indices) = self.incoming.get(&idx) else {
            return vec![];
        };
        
        edge_indices
            .iter()
            .filter_map(|&edge_idx| {
                let stored = self.edges.get(edge_idx)?;
                let source = self.nodes.get(stored.source_idx.0)?;
                Some((source, &stored.edge))
            })
            .collect()
    }
    
    /// Get all neighbors of a node (both directions)
    pub fn neighbors(&self, id: &str) -> Vec<&ConceptNode> {
        let mut result = Vec::new();
        
        // Outgoing
        for (node, _) in self.outgoing_edges(id) {
            if !result.iter().any(|n: &&ConceptNode| n.id == node.id) {
                result.push(node);
            }
        }
        
        // Incoming
        for (node, _) in self.incoming_edges(id) {
            if !result.iter().any(|n: &&ConceptNode| n.id == node.id) {
                result.push(node);
            }
        }
        
        result
    }
    
    /// Get neighbors (undirected) for an index
    pub fn neighbors_undirected(&self, idx: NodeIndex) -> Vec<NodeIndex> {
        let mut result = Vec::new();
        
        if let Some(out_edges) = self.outgoing.get(&idx) {
            for &edge_idx in out_edges {
                if let Some(stored) = self.edges.get(edge_idx) {
                    if !result.contains(&stored.target_idx) {
                        result.push(stored.target_idx);
                    }
                }
            }
        }
        
        if let Some(in_edges) = self.incoming.get(&idx) {
            for &edge_idx in in_edges {
                if let Some(stored) = self.edges.get(edge_idx) {
                    if !result.contains(&stored.source_idx) {
                        result.push(stored.source_idx);
                    }
                }
            }
        }
        
        result
    }
    
    /// Count of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    /// Count of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    
    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    
    /// Clear all nodes and edges
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.id_to_index.clear();
        self.edges.clear();
        self.outgoing.clear();
        self.incoming.clear();
    }
    
    /// Iterate over all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &ConceptNode> {
        self.nodes.iter()
    }
    
    /// Iterate over all edges with their source and target
    pub fn edges(&self) -> impl Iterator<Item = (&ConceptNode, &ConceptNode, &ConceptEdge)> {
        self.edges.iter().filter_map(move |stored| {
            let source = self.nodes.get(stored.source_idx.0)?;
            let target = self.nodes.get(stored.target_idx.0)?;
            Some((source, target, &stored.edge))
        })
    }
    
    /// Get node weight by index
    pub fn node_weight(&self, idx: NodeIndex) -> Option<&ConceptNode> {
        self.nodes.get(idx.0)
    }

    // =========================================================================
    // Subgraph Extraction
    // =========================================================================

    /// Extract a subgraph centered on a node, up to `depth` hops away
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
            if let Some(node) = self.nodes.get(current_idx.0) {
                result.ensure_node(node.clone());
            }

            // If we haven't reached max depth, explore neighbors
            if current_depth < depth {
                for neighbor_idx in self.neighbors_undirected(current_idx) {
                    if !visited.contains(&neighbor_idx) {
                        visited.insert(neighbor_idx);
                        queue.push_back((neighbor_idx, current_depth + 1));
                    }
                }
            }
        }

        // Add edges between visited nodes
        for stored in &self.edges {
            if visited.contains(&stored.source_idx) && visited.contains(&stored.target_idx) {
                if let (Some(source), Some(target)) = (
                    self.nodes.get(stored.source_idx.0),
                    self.nodes.get(stored.target_idx.0),
                ) {
                    result.add_edge(&source.id, &target.id, stored.edge.clone());
                }
            }
        }

        result
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut graph = ConceptGraph::new();
        let node = ConceptNode::new("frodo", "Frodo", "Character");
        let idx = graph.ensure_node(node);
        
        assert_eq!(graph.node_count(), 1);
        assert_eq!(idx.0, 0);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = ConceptGraph::new();
        graph.ensure_node(ConceptNode::new("frodo", "Frodo", "Character"));
        graph.ensure_node(ConceptNode::new("ring", "Ring", "Item"));
        
        let edge = graph.add_edge("frodo", "ring", ConceptEdge::unweighted("owns"));
        
        assert!(edge.is_some());
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_outgoing_edges() {
        let mut graph = ConceptGraph::new();
        graph.ensure_node(ConceptNode::new("frodo", "Frodo", "Character"));
        graph.ensure_node(ConceptNode::new("ring", "Ring", "Item"));
        graph.add_edge("frodo", "ring", ConceptEdge::unweighted("owns"));
        
        let edges = graph.outgoing_edges("frodo");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0.id, "ring");
    }

    #[test]
    fn test_neighbors() {
        let mut graph = ConceptGraph::new();
        graph.ensure_node(ConceptNode::new("a", "A", "T"));
        graph.ensure_node(ConceptNode::new("b", "B", "T"));
        graph.ensure_node(ConceptNode::new("c", "C", "T"));
        graph.add_edge("a", "b", ConceptEdge::unweighted("rel"));
        graph.add_edge("c", "a", ConceptEdge::unweighted("rel"));
        
        let neighbors = graph.neighbors("a");
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut graph = ConceptGraph::new();
        graph.ensure_node(ConceptNode::new("a", "A", "T"));
        graph.ensure_node(ConceptNode::new("b", "B", "T"));
        graph.add_edge("a", "b", ConceptEdge::unweighted("rel"));
        
        graph.clear();
        
        assert!(graph.is_empty());
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_subgraph() {
        let mut graph = ConceptGraph::new();
        graph.ensure_node(ConceptNode::new("a", "A", "T"));
        graph.ensure_node(ConceptNode::new("b", "B", "T"));
        graph.ensure_node(ConceptNode::new("c", "C", "T"));
        graph.ensure_node(ConceptNode::new("d", "D", "T"));
        graph.add_edge("a", "b", ConceptEdge::unweighted("rel"));
        graph.add_edge("b", "c", ConceptEdge::unweighted("rel"));
        graph.add_edge("c", "d", ConceptEdge::unweighted("rel"));
        
        let sub = graph.subgraph("a", 1);
        assert_eq!(sub.node_count(), 2); // a and b
        
        let sub2 = graph.subgraph("a", 2);
        assert_eq!(sub2.node_count(), 3); // a, b, c
    }
}
