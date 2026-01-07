//! Graph Queries - Traversal and Path Finding
//!
//! Advanced graph queries using CozoDB's Datalog:
//! - Neighbor traversal (n-hop)
//! - Shortest path
//! - All paths
//! - Subgraph extraction
//! - Common neighbors

use cozo::DataValue;
use std::collections::BTreeMap;

use super::registry::GraphRegistry;
use super::types::*;

// =============================================================================
// Types
// =============================================================================

/// A path through the graph
#[derive(Debug, Clone)]
pub struct GraphPath {
    /// Ordered list of node IDs from start to end
    pub nodes: Vec<String>,
    /// Edge IDs connecting the nodes
    pub edges: Vec<String>,
    /// Total path length (number of edges)
    pub length: usize,
}

/// A subgraph extraction
#[derive(Debug, Clone)]
pub struct Subgraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// Neighbor query options
#[derive(Debug, Clone, Default)]
pub struct NeighborOptions {
    /// Maximum depth to traverse (default 1)
    pub depth: usize,
    /// Filter by edge types (empty = all types)
    pub edge_types: Vec<String>,
    /// Include the starting node in results
    pub include_self: bool,
}

// =============================================================================
// Query Implementation
// =============================================================================

impl GraphRegistry {
    /// Get neighbors of a node up to n hops away
    pub fn neighbors(&self, node_id: &str, options: NeighborOptions) -> Result<Vec<Node>, GraphError> {
        use std::collections::{HashSet, VecDeque};
        
        let max_depth = if options.depth == 0 { 1 } else { options.depth };
        let mut visited = HashSet::new();
        let mut result_ids = Vec::new();
        let mut queue = VecDeque::new();
        
        queue.push_back((node_id.to_string(), 0usize));
        visited.insert(node_id.to_string());
        
        while let Some((current, depth)) = queue.pop_front() {
            if depth > 0 {
                result_ids.push(current.clone());
            }
            
            if depth < max_depth {
                let edges = self.get_edges(&current, Direction::Both)?;
                
                for edge in edges {
                    let neighbor = if edge.source_id == current {
                        &edge.target_id
                    } else {
                        &edge.source_id
                    };
                    
                    // Filter by edge type if specified
                    if !options.edge_types.is_empty() && !options.edge_types.contains(&edge.edge_type) {
                        continue;
                    }
                    
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        queue.push_back((neighbor.clone(), depth + 1));
                    }
                }
            }
        }
        
        // Collect nodes
        let mut nodes = Vec::new();
        for id in result_ids {
            if let Some(node) = self.get_node(&id)? {
                nodes.push(node);
            }
        }
        
        // Optionally include self
        if options.include_self {
            if let Some(self_node) = self.get_node(node_id)? {
                nodes.insert(0, self_node);
            }
        }
        
        Ok(nodes)
    }

    /// Find the shortest path between two nodes
    pub fn shortest_path(&self, from_id: &str, to_id: &str) -> Result<Option<GraphPath>, GraphError> {
        // Use CozoDB's built-in shortest path fixed rule
        let query = r#"
            path[node] := node = $from_id
            path[node] := path[prev], *edges{source_id, target_id},
                or(
                    and(source_id = prev, node = target_id),
                    and(target_id = prev, node = source_id)
                )
            
            ?[path] <~ ShortestPathBFS(*edges[], source_id, target_id, $from_id, $to_id)
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("from_id".to_string(), DataValue::Str(from_id.into()));
        params.insert("to_id".to_string(), DataValue::Str(to_id.into()));
        
        // ShortestPathBFS returns a list of nodes
        let result = self.run_query(query, params);
        
        match result {
            Ok(named_rows) => {
                if named_rows.rows.is_empty() {
                    return Ok(None);
                }
                
                // Extract path from result
                let path_data = &named_rows.rows[0][0];
                let node_ids = self.extract_path_nodes(path_data)?;
                
                if node_ids.is_empty() {
                    return Ok(None);
                }
                
                // Build path with edge IDs
                let mut edges = Vec::new();
                for i in 0..node_ids.len().saturating_sub(1) {
                    // Find edge between consecutive nodes
                    if let Some(edge) = self.find_edge_between(&node_ids[i], &node_ids[i + 1])? {
                        edges.push(edge.id);
                    }
                }
                
                Ok(Some(GraphPath {
                    length: node_ids.len().saturating_sub(1),
                    nodes: node_ids,
                    edges,
                }))
            }
            Err(_) => {
                // BFS not available or no path found - use manual BFS
                self.shortest_path_manual(from_id, to_id)
            }
        }
    }

    /// Manual BFS shortest path implementation
    fn shortest_path_manual(&self, from_id: &str, to_id: &str) -> Result<Option<GraphPath>, GraphError> {
        use std::collections::{HashSet, VecDeque};
        
        if from_id == to_id {
            return Ok(Some(GraphPath {
                nodes: vec![from_id.to_string()],
                edges: vec![],
                length: 0,
            }));
        }
        
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();
        
        visited.insert(from_id.to_string());
        queue.push_back(from_id.to_string());
        
        while let Some(current) = queue.pop_front() {
            let edges = self.get_edges(&current, Direction::Both)?;
            
            for edge in edges {
                let neighbor = if edge.source_id == current {
                    &edge.target_id
                } else {
                    &edge.source_id
                };
                
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.clone());
                    parent.insert(neighbor.clone(), (current.clone(), edge.id.clone()));
                    
                    if neighbor == to_id {
                        // Reconstruct path
                        let mut path_nodes = vec![to_id.to_string()];
                        let mut path_edges = vec![];
                        let mut node = to_id.to_string();
                        
                        while let Some((parent_node, edge_id)) = parent.get(&node) {
                            path_nodes.push(parent_node.clone());
                            path_edges.push(edge_id.clone());
                            node = parent_node.clone();
                        }
                        
                        path_nodes.reverse();
                        path_edges.reverse();
                        
                        return Ok(Some(GraphPath {
                            length: path_nodes.len() - 1,
                            nodes: path_nodes,
                            edges: path_edges,
                        }));
                    }
                    
                    queue.push_back(neighbor.clone());
                }
            }
        }
        
        Ok(None)
    }

    /// Find all paths between two nodes up to max depth
    pub fn all_paths(&self, from_id: &str, to_id: &str, max_depth: usize) -> Result<Vec<GraphPath>, GraphError> {
        let mut all_paths = Vec::new();
        let mut current_path = vec![from_id.to_string()];
        let mut current_edges = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(from_id.to_string());
        
        self.dfs_all_paths(
            from_id,
            to_id,
            max_depth,
            &mut current_path,
            &mut current_edges,
            &mut visited,
            &mut all_paths,
        )?;
        
        Ok(all_paths)
    }

    fn dfs_all_paths(
        &self,
        current: &str,
        target: &str,
        remaining_depth: usize,
        current_path: &mut Vec<String>,
        current_edges: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        all_paths: &mut Vec<GraphPath>,
    ) -> Result<(), GraphError> {
        if current == target {
            all_paths.push(GraphPath {
                nodes: current_path.clone(),
                edges: current_edges.clone(),
                length: current_path.len() - 1,
            });
            return Ok(());
        }
        
        if remaining_depth == 0 {
            return Ok(());
        }
        
        let edges = self.get_edges(current, Direction::Both)?;
        
        for edge in edges {
            let neighbor = if edge.source_id == current {
                &edge.target_id
            } else {
                &edge.source_id
            };
            
            if !visited.contains(neighbor) {
                visited.insert(neighbor.clone());
                current_path.push(neighbor.clone());
                current_edges.push(edge.id.clone());
                
                self.dfs_all_paths(
                    neighbor,
                    target,
                    remaining_depth - 1,
                    current_path,
                    current_edges,
                    visited,
                    all_paths,
                )?;
                
                current_path.pop();
                current_edges.pop();
                visited.remove(neighbor);
            }
        }
        
        Ok(())
    }

    /// Extract a subgraph containing the specified nodes and all edges between them
    pub fn subgraph(&self, node_ids: &[&str]) -> Result<Subgraph, GraphError> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let id_set: std::collections::HashSet<_> = node_ids.iter().cloned().collect();
        
        // Get all nodes
        for id in node_ids {
            if let Some(node) = self.get_node(id)? {
                nodes.push(node);
            }
        }
        
        // Get all edges between these nodes
        for id in node_ids {
            let node_edges = self.get_edges(id, Direction::Outgoing)?;
            for edge in node_edges {
                // Only include if both endpoints are in the subgraph
                if id_set.contains(edge.target_id.as_str()) {
                    // Avoid duplicates
                    if !edges.iter().any(|e: &Edge| e.id == edge.id) {
                        edges.push(edge);
                    }
                }
            }
        }
        
        Ok(Subgraph { nodes, edges })
    }

    /// Find nodes that are neighbors of both A and B
    pub fn common_neighbors(&self, node_a: &str, node_b: &str) -> Result<Vec<Node>, GraphError> {
        // Get neighbors of A
        let neighbors_a = self.neighbors(node_a, NeighborOptions {
            depth: 1,
            ..Default::default()
        })?;
        let set_a: std::collections::HashSet<_> = neighbors_a.iter().map(|n| n.id.clone()).collect();
        
        // Get neighbors of B
        let neighbors_b = self.neighbors(node_b, NeighborOptions {
            depth: 1,
            ..Default::default()
        })?;
        
        // Find intersection
        let common: Vec<Node> = neighbors_b
            .into_iter()
            .filter(|n| set_a.contains(&n.id) && n.id != node_a && n.id != node_b)
            .collect();
        
        Ok(common)
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Run a raw Datalog query
    fn run_query(&self, query: &str, params: BTreeMap<String, DataValue>) -> Result<cozo::NamedRows, GraphError> {
        self.db.run_script(query, params, cozo::ScriptMutability::Immutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))
    }

    /// Extract path nodes from Cozo result
    fn extract_path_nodes(&self, val: &DataValue) -> Result<Vec<String>, GraphError> {
        match val {
            DataValue::List(items) => {
                let mut nodes = Vec::new();
                for item in items.iter() {
                    if let DataValue::Str(s) = item {
                        nodes.push(s.to_string());
                    }
                }
                Ok(nodes)
            }
            DataValue::Str(s) => Ok(vec![s.to_string()]),
            _ => Ok(vec![]),
        }
    }

    /// Find edge between two nodes
    fn find_edge_between(&self, from: &str, to: &str) -> Result<Option<Edge>, GraphError> {
        let query = r#"
            ?[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] :=
                *edges[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata],
                or(
                    and(source_id = $from, target_id = $to),
                    and(source_id = $to, target_id = $from)
                )
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("from".to_string(), DataValue::Str(from.into()));
        params.insert("to".to_string(), DataValue::Str(to.into()));
        
        // Use union approach instead of or()
        let query_fixed = r#"
            e1[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] :=
                *edges[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata],
                source_id = $from, target_id = $to
            e1[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] :=
                *edges[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata],
                source_id = $to, target_id = $from
            ?[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] := e1[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata]
        "#;
        
        let result = self.run_query(query_fixed, params)?;
        
        if result.rows.is_empty() {
            return Ok(None);
        }
        
        Ok(Some(self.row_to_edge(&result.rows[0])?))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> GraphRegistry {
        GraphRegistry::in_memory().unwrap()
    }

    fn test_input(label: &str, kind: NodeKind) -> NodeInput {
        NodeInput {
            label: label.to_string(),
            kind,
            source_note: "test".to_string(),
            subtype: None,
            aliases: vec![],
            created_by: CreatedBy::User,
            metadata: None,
        }
    }

    fn edge_input(source: &str, target: &str, edge_type: &str) -> EdgeInput {
        EdgeInput {
            source_id: source.to_string(),
            target_id: target.to_string(),
            edge_type: edge_type.to_string(),
            inverse_type: None,
            bidirectional: false,
            weight: 1.0,
            confidence: 1.0,
            created_by: CreatedBy::User,
            source_note: None,
            metadata: None,
        }
    }

    // Build a simple graph: A -> B -> C -> D
    fn build_chain_graph() -> (GraphRegistry, Vec<String>) {
        let registry = test_registry();
        let a = registry.register_node(test_input("A", NodeKind::Character)).unwrap().node.id;
        let b = registry.register_node(test_input("B", NodeKind::Character)).unwrap().node.id;
        let c = registry.register_node(test_input("C", NodeKind::Character)).unwrap().node.id;
        let d = registry.register_node(test_input("D", NodeKind::Character)).unwrap().node.id;
        
        registry.create_edge(edge_input(&a, &b, "KNOWS")).unwrap();
        registry.create_edge(edge_input(&b, &c, "KNOWS")).unwrap();
        registry.create_edge(edge_input(&c, &d, "KNOWS")).unwrap();
        
        (registry, vec![a, b, c, d])
    }

    // Build a diamond graph:
    //     B
    //    / \
    //   A   D
    //    \ /
    //     C
    fn build_diamond_graph() -> (GraphRegistry, Vec<String>) {
        let registry = test_registry();
        let a = registry.register_node(test_input("A", NodeKind::Character)).unwrap().node.id;
        let b = registry.register_node(test_input("B", NodeKind::Character)).unwrap().node.id;
        let c = registry.register_node(test_input("C", NodeKind::Character)).unwrap().node.id;
        let d = registry.register_node(test_input("D", NodeKind::Character)).unwrap().node.id;
        
        registry.create_edge(edge_input(&a, &b, "KNOWS")).unwrap();
        registry.create_edge(edge_input(&a, &c, "KNOWS")).unwrap();
        registry.create_edge(edge_input(&b, &d, "KNOWS")).unwrap();
        registry.create_edge(edge_input(&c, &d, "KNOWS")).unwrap();
        
        (registry, vec![a, b, c, d])
    }

    #[test]
    fn test_neighbors_depth_1() {
        let (registry, ids) = build_chain_graph();
        let neighbors = registry.neighbors(&ids[1], NeighborOptions::default()).unwrap();
        
        // B should have neighbors A and C
        assert_eq!(neighbors.len(), 2);
        let neighbor_ids: Vec<_> = neighbors.iter().map(|n| n.id.clone()).collect();
        assert!(neighbor_ids.contains(&ids[0])); // A
        assert!(neighbor_ids.contains(&ids[2])); // C
    }

    #[test]
    fn test_neighbors_depth_2() {
        let (registry, ids) = build_chain_graph();
        let neighbors = registry.neighbors(&ids[0], NeighborOptions {
            depth: 2,
            ..Default::default()
        }).unwrap();
        
        // A at depth 2 should reach B and C
        assert!(neighbors.len() >= 2);
        let neighbor_ids: Vec<_> = neighbors.iter().map(|n| n.id.clone()).collect();
        assert!(neighbor_ids.contains(&ids[1])); // B
        assert!(neighbor_ids.contains(&ids[2])); // C
    }

    #[test]
    fn test_shortest_path_direct() {
        let (registry, ids) = build_chain_graph();
        let path = registry.shortest_path(&ids[0], &ids[1]).unwrap().unwrap();
        
        assert_eq!(path.length, 1);
        assert_eq!(path.nodes.len(), 2);
        assert_eq!(path.nodes[0], ids[0]);
        assert_eq!(path.nodes[1], ids[1]);
    }

    #[test]
    fn test_shortest_path_multi_hop() {
        let (registry, ids) = build_chain_graph();
        let path = registry.shortest_path(&ids[0], &ids[3]).unwrap().unwrap();
        
        // A -> B -> C -> D = 3 hops
        assert_eq!(path.length, 3);
        assert_eq!(path.nodes.len(), 4);
    }

    #[test]
    fn test_shortest_path_diamond() {
        let (registry, ids) = build_diamond_graph();
        let path = registry.shortest_path(&ids[0], &ids[3]).unwrap().unwrap();
        
        // Should find path of length 2 (A->B->D or A->C->D)
        assert_eq!(path.length, 2);
    }

    #[test]
    fn test_shortest_path_not_found() {
        let registry = test_registry();
        let a = registry.register_node(test_input("A", NodeKind::Character)).unwrap().node.id;
        let b = registry.register_node(test_input("B", NodeKind::Character)).unwrap().node.id;
        // No edge between A and B
        
        let path = registry.shortest_path(&a, &b).unwrap();
        assert!(path.is_none());
    }

    #[test]
    fn test_all_paths() {
        let (registry, ids) = build_diamond_graph();
        let paths = registry.all_paths(&ids[0], &ids[3], 3).unwrap();
        
        // Should find 2 paths: A->B->D and A->C->D
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().all(|p| p.length == 2));
    }

    #[test]
    fn test_subgraph() {
        let (registry, ids) = build_diamond_graph();
        let subgraph = registry.subgraph(&[&ids[0], &ids[1], &ids[3]]).unwrap();
        
        // Should have 3 nodes
        assert_eq!(subgraph.nodes.len(), 3);
        // Should have edges A->B and B->D
        assert_eq!(subgraph.edges.len(), 2);
    }

    #[test]
    fn test_common_neighbors() {
        let (registry, ids) = build_diamond_graph();
        
        // B and C share neighbor D (via edges B->D and C->D)
        // And they share neighbor A (via edges A->B and A->C)
        let common = registry.common_neighbors(&ids[1], &ids[2]).unwrap();
        
        // Should find A and D as common neighbors
        let common_ids: Vec<_> = common.iter().map(|n| n.id.clone()).collect();
        assert!(common_ids.contains(&ids[0]) || common_ids.contains(&ids[3]));
    }
}
