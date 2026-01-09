//! CozoDB-Native Graph Operations
//!
//! Phase 2A: Migration from petgraph to CozoDB native graph algorithms.
//!
//! This module provides graph operations that execute directly against CozoDB's
//! persistent relations (`nodes`, `edges`) using Datalog queries and fixed
//! algorithms (PageRank, ShortestPath, CommunityDetection).
//!
//! ## Design Philosophy
//!
//! Instead of duplicating graph state in petgraph (in-memory) and CozoDB (persistent),
//! this module treats CozoDB as the SINGLE SOURCE OF TRUTH for graph data.
//! All graph queries are executed as Datalog, leveraging CozoDB's native algorithms.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cozo::DbInstance;
//! use cozo_graph::{CozoGraph, NodeData, EdgeData};
//!
//! let db = DbInstance::new("mem", "", Default::default())?;
//! let graph = CozoGraph::new(&db);
//!
//! // Upsert node
//! graph.ensure_node(NodeData { id: "frodo", label: "Frodo", kind: "CHARACTER", ... })?;
//!
//! // Add edge
//! graph.add_edge(EdgeData { source: "frodo", target: "ring", edge_type: "CARRIES", ... })?;
//!
//! // PageRank
//! let ranked = graph.pagerank()?;
//! ```

use cozo::{DbInstance, DataValue, ScriptMutability};
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};

// =============================================================================
// Types
// =============================================================================

/// Node data for upsert operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    pub id: String,
    pub label: String,
    pub normalized: String,
    pub kind: String,
    pub subtype: Option<String>,
    pub source_note: String,
    pub metadata: Option<String>,
}

impl NodeData {
    /// Create a new node with minimal required fields
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: impl Into<String>) -> Self {
        let label_str = label.into();
        let normalized = label_str.trim().to_lowercase();
        Self {
            id: id.into(),
            label: label_str,
            normalized,
            kind: kind.into(),
            subtype: None,
            source_note: String::new(),
            metadata: None,
        }
    }
}

/// Edge data for upsert operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub weight: f64,
    pub confidence: f64,
    pub source_note: Option<String>,
    pub metadata: Option<String>,
}

impl EdgeData {
    /// Create a new edge with minimal required fields
    pub fn new(source: impl Into<String>, target: impl Into<String>, edge_type: impl Into<String>) -> Self {
        let src = source.into();
        let tgt = target.into();
        let et = edge_type.into();
        let id = format!("{}:{}:{}", src, et, tgt);
        Self {
            id,
            source_id: src,
            target_id: tgt,
            edge_type: et,
            weight: 1.0,
            confidence: 1.0,
            source_note: None,
            metadata: None,
        }
    }

    /// Builder: set weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// A ranked entity result from PageRank or centrality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub score: f64,
    pub rank: usize,
}

/// A path between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePath {
    pub nodes: Vec<String>,
    pub distance: f64,
    pub narrative: String,
}

/// A detected community
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedCommunity {
    pub id: usize,
    pub members: Vec<String>,
    pub member_count: usize,
}

/// Graph statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub density: f64,
    pub connected_components: usize,
}

// =============================================================================
// CozoGraph: Main API
// =============================================================================

/// CozoDB-native graph operations
///
/// All operations execute directly against CozoDB relations.
/// No in-memory petgraph duplication.
pub struct CozoGraph<'a> {
    db: &'a DbInstance,
    /// Optional world_id filter for multi-tenant scenarios
    world_id: Option<String>,
}

impl<'a> CozoGraph<'a> {
    /// Create a new CozoGraph handle
    pub fn new(db: &'a DbInstance) -> Self {
        Self { db, world_id: None }
    }

    /// Create with a specific world_id filter
    pub fn with_world(db: &'a DbInstance, world_id: impl Into<String>) -> Self {
        Self {
            db,
            world_id: Some(world_id.into()),
        }
    }

    // -------------------------------------------------------------------------
    // Node Operations
    // -------------------------------------------------------------------------

    /// Upsert a node (insert or update)
    pub fn ensure_node(&self, node: &NodeData) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64() * 1000.0;

        let query = r#"
            ?[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata] <- [[
                $id, $label, $normalized, $kind, $subtype, $source_note, $created_at, "system", 0, $metadata
            ]]
            :put nodes {id => label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata}
        "#;

        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(node.id.clone().into()));
        params.insert("label".to_string(), DataValue::Str(node.label.clone().into()));
        params.insert("normalized".to_string(), DataValue::Str(node.normalized.clone().into()));
        params.insert("kind".to_string(), DataValue::Str(node.kind.clone().into()));
        params.insert("subtype".to_string(), DataValue::Str(node.subtype.clone().unwrap_or_default().into()));
        params.insert("source_note".to_string(), DataValue::Str(node.source_note.clone().into()));
        params.insert("created_at".to_string(), DataValue::from(now));
        params.insert("metadata".to_string(), DataValue::Str(node.metadata.clone().unwrap_or_default().into()));

        self.db
            .run_script(query, params.into(), ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to upsert node: {}", e))?;

        Ok(())
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Result<Option<NodeData>, String> {
        let query = r#"
            ?[id, label, normalized, kind, subtype, source_note, metadata] := 
                *nodes[id, label, normalized, kind, subtype, source_note, _, _, _, metadata],
                id = $id
        "#;

        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));

        let result = self.db
            .run_script(query, params.into(), ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get node: {}", e))?;

        if result.rows.is_empty() {
            return Ok(None);
        }

        let row = &result.rows[0];
        Ok(Some(NodeData {
            id: extract_string(&row[0])?,
            label: extract_string(&row[1])?,
            normalized: extract_string(&row[2])?,
            kind: extract_string(&row[3])?,
            subtype: Some(extract_string(&row[4])?),
            source_note: extract_string(&row[5])?,
            metadata: Some(extract_string(&row[6])?),
        }))
    }

    /// Count total nodes
    pub fn node_count(&self) -> Result<usize, String> {
        let query = "?[count(id)] := *nodes[id, _, _, _, _, _, _, _, _, _]";

        let result = self.db
            .run_script(query, Default::default(), ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to count nodes: {}", e))?;

        if result.rows.is_empty() {
            return Ok(0);
        }

        extract_int(&result.rows[0][0]).map(|n| n as usize)
    }

    // -------------------------------------------------------------------------
    // Edge Operations
    // -------------------------------------------------------------------------

    /// Upsert an edge
    pub fn add_edge(&self, edge: &EdgeData) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64() * 1000.0;

        let query = r#"
            ?[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] <- [[
                $id, $source_id, $target_id, $edge_type, "", false, $weight, $confidence, $created_at, "system", $source_note, $metadata
            ]]
            :put edges {id => source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata}
        "#;

        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(edge.id.clone().into()));
        params.insert("source_id".to_string(), DataValue::Str(edge.source_id.clone().into()));
        params.insert("target_id".to_string(), DataValue::Str(edge.target_id.clone().into()));
        params.insert("edge_type".to_string(), DataValue::Str(edge.edge_type.clone().into()));
        params.insert("weight".to_string(), DataValue::from(edge.weight));
        params.insert("confidence".to_string(), DataValue::from(edge.confidence));
        params.insert("created_at".to_string(), DataValue::from(now));
        params.insert("source_note".to_string(), DataValue::Str(edge.source_note.clone().unwrap_or_default().into()));
        params.insert("metadata".to_string(), DataValue::Str(edge.metadata.clone().unwrap_or_default().into()));

        self.db
            .run_script(query, params.into(), ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to upsert edge: {}", e))?;

        Ok(())
    }

    /// Get outgoing edges from a node
    pub fn outgoing_edges(&self, node_id: &str) -> Result<Vec<(String, String, f64)>, String> {
        let query = r#"
            ?[target_id, edge_type, weight] := 
                *edges[_, source, target_id, edge_type, _, _, weight, _, _, _, _, _],
                source = $source_id
        "#;

        let mut params = BTreeMap::new();
        params.insert("source_id".to_string(), DataValue::Str(node_id.into()));

        let result = self.db
            .run_script(query, params.into(), ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get outgoing edges: {}", e))?;

        let mut edges = Vec::new();
        for row in result.rows {
            edges.push((
                extract_string(&row[0])?,
                extract_string(&row[1])?,
                extract_float(&row[2])?,
            ));
        }

        Ok(edges)
    }

    /// Get incoming edges to a node
    pub fn incoming_edges(&self, node_id: &str) -> Result<Vec<(String, String, f64)>, String> {
        let query = r#"
            ?[source_id, edge_type, weight] := 
                *edges[_, source_id, target, edge_type, _, _, weight, _, _, _, _, _],
                target = $target_id
        "#;

        let mut params = BTreeMap::new();
        params.insert("target_id".to_string(), DataValue::Str(node_id.into()));

        let result = self.db
            .run_script(query, params.into(), ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get incoming edges: {}", e))?;

        let mut edges = Vec::new();
        for row in result.rows {
            edges.push((
                extract_string(&row[0])?,
                extract_string(&row[1])?,
                extract_float(&row[2])?,
            ));
        }

        Ok(edges)
    }

    /// Count total edges
    pub fn edge_count(&self) -> Result<usize, String> {
        let query = "?[count(id)] := *edges[id, _, _, _, _, _, _, _, _, _, _, _]";

        let result = self.db
            .run_script(query, Default::default(), ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to count edges: {}", e))?;

        if result.rows.is_empty() {
            return Ok(0);
        }

        extract_int(&result.rows[0][0]).map(|n| n as usize)
    }

    // -------------------------------------------------------------------------
    // Graph Algorithms (CozoDB Native)
    // -------------------------------------------------------------------------

    /// Compute PageRank using CozoDB's native algorithm
    ///
    /// Returns nodes ranked by their PageRank score.
    pub fn pagerank(&self) -> Result<Vec<RankedNode>, String> {
        // Build edge relation for PageRank input
        // PageRank expects: edge[from, to] or edge[from, to, weight]
        let query = r#"
            edge[source_id, target_id] := *edges[_, source_id, target_id, _, _, _, _, _, _, _, _, _]
            ?[node, score] <~ PageRank(edge[])
            :order -score
        "#;

        let result = self.db
            .run_script(query, Default::default(), ScriptMutability::Immutable)
            .map_err(|e| format!("PageRank failed: {}", e))?;

        let mut ranked = Vec::new();
        for (i, row) in result.rows.iter().enumerate() {
            let node_id = extract_string(&row[0])?;
            let score = extract_float(&row[1])?;

            // Get node details
            let node_data = self.get_node(&node_id)?;
            let (label, kind) = node_data
                .map(|n| (n.label, n.kind))
                .unwrap_or_else(|| (node_id.clone(), "Unknown".to_string()));

            ranked.push(RankedNode {
                id: node_id,
                label,
                kind,
                score,
                rank: i + 1,
            });
        }

        Ok(ranked)
    }

    /// Find shortest path between two nodes using CozoDB's Dijkstra algorithm
    pub fn shortest_path(&self, source: &str, target: &str) -> Result<Option<NodePath>, String> {
        let query = r#"
            edge[source_id, target_id, weight] := *edges[_, source_id, target_id, _, _, _, weight, _, _, _, _, _]
            starting[] <- [[$source]]
            goal[] <- [[$target]]
            ?[starting, goal, distance, path] <~ ShortestPathDijkstra(edge[], starting[], goal[])
        "#;

        let mut params = BTreeMap::new();
        params.insert("source".to_string(), DataValue::Str(source.into()));
        params.insert("target".to_string(), DataValue::Str(target.into()));

        let result = self.db
            .run_script(query, params.into(), ScriptMutability::Immutable)
            .map_err(|e| format!("ShortestPath failed: {}", e))?;

        if result.rows.is_empty() {
            return Ok(None);
        }

        let row = &result.rows[0];
        let distance = extract_float(&row[2])?;
        let path_list = extract_list(&row[3])?;

        // Build narrative from path
        let narrative = path_list.join(" → ");

        Ok(Some(NodePath {
            nodes: path_list,
            distance,
            narrative,
        }))
    }

    /// Detect communities using CozoDB's Louvain algorithm
    pub fn detect_communities(&self) -> Result<Vec<DetectedCommunity>, String> {
        let query = r#"
            edge[source_id, target_id] := *edges[_, source_id, target_id, _, _, _, _, _, _, _, _, _]
            ?[cluster, node] <~ CommunityDetectionLouvain(edge[])
        "#;

        let result = self.db
            .run_script(query, Default::default(), ScriptMutability::Immutable)
            .map_err(|e| format!("Community detection failed: {}", e))?;

        // Group by cluster
        let mut cluster_members: std::collections::HashMap<usize, Vec<String>> = std::collections::HashMap::new();
        for row in result.rows {
            // cluster can be an int or a list of ints for hierarchical clustering
            let cluster_id = match &row[0] {
                DataValue::Num(n) => {
                    match n {
                        cozo::Num::Int(i) => *i as usize,
                        cozo::Num::Float(f) => *f as usize,
                    }
                }
                DataValue::List(l) => {
                    // Take the first element of the list
                    l.first().and_then(|v| match v {
                        DataValue::Num(n) => match n {
                            cozo::Num::Int(i) => Some(*i as usize),
                            cozo::Num::Float(f) => Some(*f as usize),
                        },
                        _ => None
                    }).unwrap_or(0)
                }
                _ => 0,
            };
            let node_id = extract_string(&row[1])?;
            cluster_members.entry(cluster_id).or_default().push(node_id);
        }

        let mut communities: Vec<DetectedCommunity> = cluster_members
            .into_iter()
            .map(|(id, members)| {
                let member_count = members.len();
                DetectedCommunity { id, members, member_count }
            })
            .collect();

        // Sort by size descending
        communities.sort_by(|a, b| b.member_count.cmp(&a.member_count));

        Ok(communities)
    }

    /// Get graph statistics
    pub fn stats(&self) -> Result<GraphStats, String> {
        let node_count = self.node_count()?;
        let edge_count = self.edge_count()?;

        let max_edges = if node_count > 1 {
            node_count * (node_count - 1)
        } else {
            1
        };
        let density = edge_count as f64 / max_edges as f64;

        Ok(GraphStats {
            node_count,
            edge_count,
            density,
            connected_components: 0, // TODO: implement with CozoDB connectivity check
        })
    }

    /// Get N-hop neighborhood of a node
    /// 
    /// Uses iterative edge traversal to find all nodes within `max_hops` of the given node.
    pub fn neighborhood(&self, node_id: &str, max_hops: usize) -> Result<Vec<(String, usize)>, String> {
        // Collect neighbors iteratively using Rust
        // Start with direct neighbors (hop 1), then expand
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut current_frontier: Vec<String> = vec![node_id.to_string()];
        let mut neighbors: Vec<(String, usize)> = Vec::new();
        
        visited.insert(node_id.to_string());
        
        for hop in 1..=max_hops {
            let mut next_frontier: Vec<String> = Vec::new();
            
            for current_node in &current_frontier {
                // Get outgoing edges
                for (target, _, _) in self.outgoing_edges(current_node)? {
                    if !visited.contains(&target) {
                        visited.insert(target.clone());
                        neighbors.push((target.clone(), hop));
                        next_frontier.push(target);
                    }
                }
                
                // Get incoming edges (for undirected traversal)
                for (source, _, _) in self.incoming_edges(current_node)? {
                    if !visited.contains(&source) {
                        visited.insert(source.clone());
                        neighbors.push((source.clone(), hop));
                        next_frontier.push(source);
                    }
                }
            }
            
            if next_frontier.is_empty() {
                break; // No more nodes to explore
            }
            current_frontier = next_frontier;
        }
        
        // Sort by distance
        neighbors.sort_by_key(|(_, d)| *d);
        Ok(neighbors)
    }

    /// Get top N entities by PageRank
    pub fn top_entities(&self, n: usize) -> Result<Vec<RankedNode>, String> {
        let mut ranked = self.pagerank()?;
        ranked.truncate(n);
        Ok(ranked)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn extract_string(val: &DataValue) -> Result<String, String> {
    match val {
        DataValue::Str(s) => Ok(s.to_string()),
        DataValue::Null => Ok(String::new()),
        _ => Err(format!("Expected string, got {:?}", val)),
    }
}

fn extract_float(val: &DataValue) -> Result<f64, String> {
    match val {
        DataValue::Num(n) => {
            match n {
                cozo::Num::Int(i) => Ok(*i as f64),
                cozo::Num::Float(f) => Ok(*f),
            }
        }
        _ => Err(format!("Expected float, got {:?}", val)),
    }
}

fn extract_int(val: &DataValue) -> Result<i64, String> {
    match val {
        DataValue::Num(n) => {
            match n {
                cozo::Num::Int(i) => Ok(*i),
                cozo::Num::Float(f) => Ok(*f as i64),
            }
        }
        _ => Err(format!("Expected int, got {:?}", val)),
    }
}

fn extract_list(val: &DataValue) -> Result<Vec<String>, String> {
    match val {
        DataValue::List(l) => {
            l.iter()
                .map(|v| extract_string(v))
                .collect()
        }
        _ => Err(format!("Expected list, got {:?}", val)),
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
    fn test_ensure_node() {
        let db = create_test_db();
        let graph = CozoGraph::new(&db);

        let node = NodeData::new("frodo", "Frodo Baggins", "CHARACTER");
        graph.ensure_node(&node).unwrap();

        let retrieved = graph.get_node("frodo").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().label, "Frodo Baggins");
    }

    #[test]
    fn test_node_count() {
        let db = create_test_db();
        let graph = CozoGraph::new(&db);

        assert_eq!(graph.node_count().unwrap(), 0);

        graph.ensure_node(&NodeData::new("a", "A", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("b", "B", "TYPE")).unwrap();

        assert_eq!(graph.node_count().unwrap(), 2);
    }

    #[test]
    fn test_add_edge() {
        let db = create_test_db();
        let graph = CozoGraph::new(&db);

        graph.ensure_node(&NodeData::new("frodo", "Frodo", "CHARACTER")).unwrap();
        graph.ensure_node(&NodeData::new("ring", "The Ring", "ITEM")).unwrap();

        let edge = EdgeData::new("frodo", "ring", "CARRIES");
        graph.add_edge(&edge).unwrap();

        let outgoing = graph.outgoing_edges("frodo").unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].0, "ring");
        assert_eq!(outgoing[0].1, "CARRIES");
    }

    #[test]
    fn test_pagerank() {
        let db = create_test_db();
        let graph = CozoGraph::new(&db);

        // Build simple graph: A -> B -> C
        graph.ensure_node(&NodeData::new("a", "A", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("b", "B", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("c", "C", "TYPE")).unwrap();

        graph.add_edge(&EdgeData::new("a", "b", "LINKS")).unwrap();
        graph.add_edge(&EdgeData::new("b", "c", "LINKS")).unwrap();

        let ranked = graph.pagerank().unwrap();
        assert_eq!(ranked.len(), 3);

        // C should rank highest (most incoming through B)
        // Note: exact ranking depends on algorithm parameters
        println!("PageRank: {:?}", ranked);
    }

    #[test]
    fn test_shortest_path() {
        let db = create_test_db();
        let graph = CozoGraph::new(&db);

        graph.ensure_node(&NodeData::new("a", "A", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("b", "B", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("c", "C", "TYPE")).unwrap();

        graph.add_edge(&EdgeData::new("a", "b", "LINKS").with_weight(1.0)).unwrap();
        graph.add_edge(&EdgeData::new("b", "c", "LINKS").with_weight(1.0)).unwrap();

        let path = graph.shortest_path("a", "c").unwrap();
        assert!(path.is_some());

        let path = path.unwrap();
        assert_eq!(path.nodes.len(), 3);
        assert_eq!(path.distance, 2.0);
    }

    #[test]
    fn test_community_detection() {
        let db = create_test_db();
        let graph = CozoGraph::new(&db);

        // Create two clusters
        graph.ensure_node(&NodeData::new("a1", "A1", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("a2", "A2", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("b1", "B1", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("b2", "B2", "TYPE")).unwrap();

        // Dense cluster A
        graph.add_edge(&EdgeData::new("a1", "a2", "LINKS")).unwrap();
        graph.add_edge(&EdgeData::new("a2", "a1", "LINKS")).unwrap();

        // Dense cluster B
        graph.add_edge(&EdgeData::new("b1", "b2", "LINKS")).unwrap();
        graph.add_edge(&EdgeData::new("b2", "b1", "LINKS")).unwrap();

        // Weak link between clusters
        graph.add_edge(&EdgeData::new("a1", "b1", "LINKS")).unwrap();

        let communities = graph.detect_communities().unwrap();
        println!("Communities: {:?}", communities);
        assert!(!communities.is_empty());
    }

    #[test]
    fn test_neighborhood() {
        let db = create_test_db();
        let graph = CozoGraph::new(&db);

        graph.ensure_node(&NodeData::new("a", "A", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("b", "B", "TYPE")).unwrap();
        graph.ensure_node(&NodeData::new("c", "C", "TYPE")).unwrap();

        graph.add_edge(&EdgeData::new("a", "b", "LINKS")).unwrap();
        graph.add_edge(&EdgeData::new("b", "c", "LINKS")).unwrap();

        let neighbors_1hop = graph.neighborhood("a", 1).unwrap();
        assert_eq!(neighbors_1hop.len(), 1);
        assert_eq!(neighbors_1hop[0].0, "b");

        let neighbors_2hop = graph.neighborhood("a", 2).unwrap();
        assert_eq!(neighbors_2hop.len(), 2); // b and c
    }
}
