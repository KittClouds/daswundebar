//! Graph Backend Trait
//!
//! Unified interface for graph operations that can be implemented by:
//! - `CozoGraph` (persistent CozoDB-backed)
//! - `ConceptGraph` (in-memory petgraph-backed)
//!
//! This allows code to be agnostic about the underlying graph storage.

use serde::{Deserialize, Serialize};

// =============================================================================
// Common Types
// =============================================================================

/// A node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
}

/// An edge in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub weight: f64,
}

/// A ranked entity (PageRank result)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedEntity {
    pub id: String,
    pub label: String,
    pub score: f64,
    pub rank: usize,
}

/// A path between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPath {
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
    pub distance: f64,
}

/// A detected community
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphCommunity {
    pub id: usize,
    pub members: Vec<String>,
}

/// Graph statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub node_count: usize,
    pub edge_count: usize,
    pub density: f64,
}

// =============================================================================
// GraphBackend Trait
// =============================================================================

/// Unified graph backend trait
///
/// Implementations:
/// - `CozoGraph` (persistent, CozoDB-backed)
/// - `ConceptGraph` (in-memory, petgraph-backed)
pub trait GraphBackend {
    /// Error type for operations
    type Error: std::fmt::Debug;

    // -------------------------------------------------------------------------
    // Node Operations
    // -------------------------------------------------------------------------

    /// Add or update a node
    fn ensure_node(&self, node: &GraphNode) -> Result<(), Self::Error>;

    /// Get a node by ID
    fn get_node(&self, id: &str) -> Result<Option<GraphNode>, Self::Error>;

    /// Count nodes
    fn node_count(&self) -> Result<usize, Self::Error>;

    // -------------------------------------------------------------------------
    // Edge Operations
    // -------------------------------------------------------------------------

    /// Add or update an edge
    fn add_edge(&self, edge: &GraphEdge) -> Result<(), Self::Error>;

    /// Get outgoing edges from a node
    fn outgoing_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>, Self::Error>;

    /// Get incoming edges to a node
    fn incoming_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>, Self::Error>;

    /// Count edges
    fn edge_count(&self) -> Result<usize, Self::Error>;

    // -------------------------------------------------------------------------
    // Graph Algorithms
    // -------------------------------------------------------------------------

    /// Compute PageRank
    fn pagerank(&self) -> Result<Vec<RankedEntity>, Self::Error>;

    /// Find shortest path between two nodes
    fn shortest_path(&self, source: &str, target: &str) -> Result<Option<GraphPath>, Self::Error>;

    /// Detect communities
    fn detect_communities(&self) -> Result<Vec<GraphCommunity>, Self::Error>;

    /// Get N-hop neighborhood
    fn neighborhood(&self, node_id: &str, max_hops: usize) -> Result<Vec<(String, usize)>, Self::Error>;

    /// Get top N entities by PageRank
    fn top_entities(&self, n: usize) -> Result<Vec<RankedEntity>, Self::Error> {
        let mut ranked = self.pagerank()?;
        ranked.truncate(n);
        Ok(ranked)
    }

    /// Get graph statistics
    fn stats(&self) -> Result<GraphStatistics, Self::Error> {
        let node_count = self.node_count()?;
        let edge_count = self.edge_count()?;
        let max_edges = if node_count > 1 { node_count * (node_count - 1) } else { 1 };
        let density = edge_count as f64 / max_edges as f64;

        Ok(GraphStatistics {
            node_count,
            edge_count,
            density,
        })
    }
}

// =============================================================================
// CozoGraph Implementation
// =============================================================================

use super::cozo_graph;
use cozo::DbInstance;

/// Wrapper to implement GraphBackend for CozoGraph
pub struct CozoGraphBackend<'a> {
    inner: cozo_graph::CozoGraph<'a>,
}

impl<'a> CozoGraphBackend<'a> {
    pub fn new(db: &'a DbInstance) -> Self {
        Self {
            inner: cozo_graph::CozoGraph::new(db),
        }
    }
}

impl<'a> GraphBackend for CozoGraphBackend<'a> {
    type Error = String;

    fn ensure_node(&self, node: &GraphNode) -> Result<(), Self::Error> {
        let data = cozo_graph::NodeData {
            id: node.id.clone(),
            label: node.label.clone(),
            normalized: node.label.trim().to_lowercase(),
            kind: node.kind.clone(),
            subtype: None,
            source_note: String::new(),
            metadata: None,
        };
        self.inner.ensure_node(&data)
    }

    fn get_node(&self, id: &str) -> Result<Option<GraphNode>, Self::Error> {
        self.inner.get_node(id).map(|opt| {
            opt.map(|n| GraphNode {
                id: n.id,
                label: n.label,
                kind: n.kind,
            })
        })
    }

    fn node_count(&self) -> Result<usize, Self::Error> {
        self.inner.node_count()
    }

    fn add_edge(&self, edge: &GraphEdge) -> Result<(), Self::Error> {
        let data = cozo_graph::EdgeData {
            id: format!("{}:{}:{}", edge.source_id, edge.edge_type, edge.target_id),
            source_id: edge.source_id.clone(),
            target_id: edge.target_id.clone(),
            edge_type: edge.edge_type.clone(),
            weight: edge.weight,
            confidence: 1.0,
            source_note: None,
            metadata: None,
        };
        self.inner.add_edge(&data)
    }

    fn outgoing_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>, Self::Error> {
        self.inner.outgoing_edges(node_id).map(|edges| {
            edges
                .into_iter()
                .map(|(target, edge_type, weight)| GraphEdge {
                    source_id: node_id.to_string(),
                    target_id: target,
                    edge_type,
                    weight,
                })
                .collect()
        })
    }

    fn incoming_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>, Self::Error> {
        self.inner.incoming_edges(node_id).map(|edges| {
            edges
                .into_iter()
                .map(|(source, edge_type, weight)| GraphEdge {
                    source_id: source,
                    target_id: node_id.to_string(),
                    edge_type,
                    weight,
                })
                .collect()
        })
    }

    fn edge_count(&self) -> Result<usize, Self::Error> {
        self.inner.edge_count()
    }

    fn pagerank(&self) -> Result<Vec<RankedEntity>, Self::Error> {
        self.inner.pagerank().map(|ranked| {
            ranked
                .into_iter()
                .map(|r| RankedEntity {
                    id: r.id,
                    label: r.label,
                    score: r.score,
                    rank: r.rank,
                })
                .collect()
        })
    }

    fn shortest_path(&self, source: &str, target: &str) -> Result<Option<GraphPath>, Self::Error> {
        self.inner.shortest_path(source, target).map(|opt| {
            opt.map(|p| GraphPath {
                nodes: p.nodes,
                edges: vec![], // CozoGraph doesn't return edge labels in path
                distance: p.distance,
            })
        })
    }

    fn detect_communities(&self) -> Result<Vec<GraphCommunity>, Self::Error> {
        self.inner.detect_communities().map(|communities| {
            communities
                .into_iter()
                .map(|c| GraphCommunity {
                    id: c.id,
                    members: c.members,
                })
                .collect()
        })
    }

    fn neighborhood(&self, node_id: &str, max_hops: usize) -> Result<Vec<(String, usize)>, Self::Error> {
        self.inner.neighborhood(node_id, max_hops)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::schema::init_schema;

    fn create_test_db() -> DbInstance {
        let db = DbInstance::new("mem", "", Default::default()).unwrap();
        init_schema(&db).unwrap();
        db
    }

    #[test]
    fn test_cozo_backend_basic() {
        let db = create_test_db();
        let backend = CozoGraphBackend::new(&db);

        // Add nodes
        backend.ensure_node(&GraphNode {
            id: "a".to_string(),
            label: "Node A".to_string(),
            kind: "TYPE".to_string(),
        }).unwrap();

        backend.ensure_node(&GraphNode {
            id: "b".to_string(),
            label: "Node B".to_string(),
            kind: "TYPE".to_string(),
        }).unwrap();

        assert_eq!(backend.node_count().unwrap(), 2);

        // Add edge
        backend.add_edge(&GraphEdge {
            source_id: "a".to_string(),
            target_id: "b".to_string(),
            edge_type: "LINKS".to_string(),
            weight: 1.0,
        }).unwrap();

        assert_eq!(backend.edge_count().unwrap(), 1);
    }

    #[test]
    fn test_backend_stats() {
        let db = create_test_db();
        let backend = CozoGraphBackend::new(&db);

        backend.ensure_node(&GraphNode {
            id: "x".to_string(),
            label: "X".to_string(),
            kind: "T".to_string(),
        }).unwrap();

        let stats = backend.stats().unwrap();
        assert_eq!(stats.node_count, 1);
        assert_eq!(stats.edge_count, 0);
    }
}
