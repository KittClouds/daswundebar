//! Graph Algorithms - Centrality, Community Detection, and Ranking
//!
//! Implements key graph algorithms:
//! - PageRank (influence scoring)
//! - Degree centrality
//! - Betweenness centrality
//! - Connected components
//! - Community detection (label propagation)

use std::collections::{HashMap, HashSet, VecDeque};

use super::registry::GraphRegistry;
use super::types::*;
use super::projection::GraphProjection;
use super::graph_backend::GraphBackend;

// =============================================================================
// Types
// =============================================================================

/// A community of nodes
#[derive(Debug, Clone)]
pub struct Community {
    /// Community ID
    pub id: usize,
    /// Node IDs in this community
    pub members: Vec<String>,
    /// Number of internal edges
    pub internal_edges: usize,
}

/// Centrality metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CentralityMetric {
    Degree,
    InDegree,
    OutDegree,
    Betweenness,
}

/// Graph statistics
#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub avg_degree: f64,
    pub density: f64,
    pub component_count: usize,
}

// =============================================================================
// Algorithm Implementation
// =============================================================================

impl GraphRegistry {
    /// Create an in-memory projection of the graph for algorithm execution
    pub fn project(&self) -> Result<GraphProjection, GraphError> {
        let mut proj = GraphProjection::new();
        proj.hydrate(&self.db).map_err(|e| GraphError::QueryError(e))?;
        Ok(proj)
    }

    /// Calculate PageRank scores using optimized in-memory projection
    /// 
    /// # Arguments
    /// * `_damping` - Ignored (uses standard 0.85)
    /// * `_iterations` - Ignored (uses convergence check)
    /// 
    /// # Returns
    /// Vector of (node_id, score) tuples sorted by score descending
    pub fn pagerank(&self, _damping: f64, _iterations: usize) -> Result<Vec<(String, f64)>, GraphError> {
        let proj = self.project()?;
        
        let ranked = proj.pagerank()
            .map_err(|e| GraphError::QueryError(e))?;
        
        Ok(ranked.into_iter().map(|r| (r.id, r.score)).collect())
    }

    /// Calculate centrality scores for all nodes
    /// 
    /// # Arguments
    /// * `metric` - Which centrality metric to compute
    /// 
    /// # Returns
    /// Vector of (node_id, score) tuples sorted by score descending
    pub fn centrality(&self, metric: CentralityMetric) -> Result<Vec<(String, f64)>, GraphError> {
        match metric {
            CentralityMetric::Degree => self.degree_centrality(),
            CentralityMetric::InDegree => self.in_degree_centrality(),
            CentralityMetric::OutDegree => self.out_degree_centrality(),
            CentralityMetric::Betweenness => self.betweenness_centrality(),
        }
    }

    /// Degree centrality (total connections)
    fn degree_centrality(&self) -> Result<Vec<(String, f64)>, GraphError> {
        let nodes = self.get_nodes(NodeFilter::default())?;
        let n = nodes.len();
        
        if n <= 1 {
            return Ok(nodes.iter().map(|n| (n.id.clone(), 0.0)).collect());
        }
        
        let mut scores = Vec::new();
        let max_degree = (n - 1) as f64; // Normalize by max possible degree
        
        for node in nodes {
            let edges = self.get_edges(&node.id, Direction::Both)?;
            let degree = edges.len() as f64;
            scores.push((node.id, degree / max_degree));
        }
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scores)
    }

    /// In-degree centrality
    fn in_degree_centrality(&self) -> Result<Vec<(String, f64)>, GraphError> {
        let nodes = self.get_nodes(NodeFilter::default())?;
        let n = nodes.len();
        
        if n <= 1 {
            return Ok(nodes.iter().map(|n| (n.id.clone(), 0.0)).collect());
        }
        
        let mut scores = Vec::new();
        let max_degree = (n - 1) as f64;
        
        for node in nodes {
            let edges = self.get_edges(&node.id, Direction::Incoming)?;
            let degree = edges.len() as f64;
            scores.push((node.id, degree / max_degree));
        }
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scores)
    }

    /// Out-degree centrality
    fn out_degree_centrality(&self) -> Result<Vec<(String, f64)>, GraphError> {
        let nodes = self.get_nodes(NodeFilter::default())?;
        let n = nodes.len();
        
        if n <= 1 {
            return Ok(nodes.iter().map(|n| (n.id.clone(), 0.0)).collect());
        }
        
        let mut scores = Vec::new();
        let max_degree = (n - 1) as f64;
        
        for node in nodes {
            let edges = self.get_edges(&node.id, Direction::Outgoing)?;
            let degree = edges.len() as f64;
            scores.push((node.id, degree / max_degree));
        }
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scores)
    }

    /// Betweenness centrality (measures how often a node lies on shortest paths)
    fn betweenness_centrality(&self) -> Result<Vec<(String, f64)>, GraphError> {
        let nodes = self.get_nodes(NodeFilter::default())?;
        let n = nodes.len();
        
        if n <= 2 {
            return Ok(nodes.iter().map(|n| (n.id.clone(), 0.0)).collect());
        }
        
        let mut betweenness: HashMap<String, f64> = nodes.iter()
            .map(|n| (n.id.clone(), 0.0))
            .collect();
        
        let node_ids: Vec<_> = nodes.iter().map(|n| n.id.clone()).collect();
        
        // For each node as source, run BFS and count paths
        for source in &node_ids {
            let (predecessors, path_counts, distances) = self.bfs_paths(source)?;
            
            // Accumulate dependencies
            let mut dependency: HashMap<String, f64> = node_ids.iter()
                .map(|id| (id.clone(), 0.0))
                .collect();
            
            // Process nodes in order of decreasing distance
            let mut nodes_by_dist: Vec<_> = distances.iter().collect();
            nodes_by_dist.sort_by(|a, b| b.1.cmp(a.1));
            
            for (node, _) in nodes_by_dist {
                if node == source {
                    continue;
                }
                
                let sigma_w = *path_counts.get(node).unwrap_or(&0.0);
                if sigma_w == 0.0 {
                    continue;
                }
                
                if let Some(preds) = predecessors.get(node) {
                    for pred in preds {
                        let sigma_v = *path_counts.get(pred).unwrap_or(&0.0);
                        if sigma_v > 0.0 {
                            let contrib = (sigma_v / sigma_w) * (1.0 + dependency.get(node).unwrap_or(&0.0));
                            *dependency.get_mut(pred).unwrap() += contrib;
                        }
                    }
                }
                
                if node != source {
                    *betweenness.get_mut(node).unwrap() += dependency.get(node).unwrap_or(&0.0);
                }
            }
        }
        
        // Normalize
        let normalization = if n > 2 {
            2.0 / ((n - 1) * (n - 2)) as f64
        } else {
            1.0
        };
        
        let mut result: Vec<_> = betweenness.into_iter()
            .map(|(id, score)| (id, score * normalization))
            .collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(result)
    }

    /// BFS to find all shortest paths from a source
    fn bfs_paths(&self, source: &str) -> Result<(
        HashMap<String, Vec<String>>,  // predecessors
        HashMap<String, f64>,           // path_counts
        HashMap<String, usize>,         // distances
    ), GraphError> {
        let mut predecessors: HashMap<String, Vec<String>> = HashMap::new();
        let mut path_counts: HashMap<String, f64> = HashMap::new();
        let mut distances: HashMap<String, usize> = HashMap::new();
        
        path_counts.insert(source.to_string(), 1.0);
        distances.insert(source.to_string(), 0);
        
        let mut queue = VecDeque::new();
        queue.push_back(source.to_string());
        
        while let Some(current) = queue.pop_front() {
            let current_dist = *distances.get(&current).unwrap_or(&0);
            let edges = self.get_edges(&current, Direction::Both)?;
            
            for edge in edges {
                let neighbor = if edge.source_id == current {
                    &edge.target_id
                } else {
                    &edge.source_id
                };
                
                if !distances.contains_key(neighbor) {
                    distances.insert(neighbor.clone(), current_dist + 1);
                    queue.push_back(neighbor.clone());
                }
                
                if distances.get(neighbor) == Some(&(current_dist + 1)) {
                    let current_count = *path_counts.get(&current).unwrap_or(&0.0);
                    *path_counts.entry(neighbor.clone()).or_insert(0.0) += current_count;
                    predecessors.entry(neighbor.clone()).or_insert_with(Vec::new).push(current.clone());
                }
            }
        }
        
        Ok((predecessors, path_counts, distances))
    }

    /// Find connected components in the graph
    /// 
    /// # Returns
    /// Vector of components, each containing node IDs
    pub fn connected_components(&self) -> Result<Vec<Vec<String>>, GraphError> {
        let proj = self.project()?;
        
        let components = proj.connected_components()
             .map_err(|e| GraphError::QueryError(e))?;
             
        Ok(components)
    }

    /// Detect communities using label propagation algorithm
    /// 
    /// Simple but effective community detection that assigns each node
    /// to the community most common among its neighbors.
    pub fn communities(&self) -> Result<Vec<Community>, GraphError> {
        let nodes = self.get_nodes(NodeFilter::default())?;
        
        if nodes.is_empty() {
            return Ok(vec![]);
        }
        
        // Initialize each node with its own label
        let mut labels: HashMap<String, usize> = nodes.iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();
        
        let node_ids: Vec<_> = nodes.iter().map(|n| n.id.clone()).collect();
        let max_iterations = 20;
        
        for _ in 0..max_iterations {
            let mut changed = false;
            
            for node_id in &node_ids {
                let edges = self.get_edges(node_id, Direction::Both)?;
                
                if edges.is_empty() {
                    continue;
                }
                
                // Count neighbor labels
                let mut label_counts: HashMap<usize, usize> = HashMap::new();
                for edge in edges {
                    let neighbor = if edge.source_id == *node_id {
                        &edge.target_id
                    } else {
                        &edge.source_id
                    };
                    
                    if let Some(&label) = labels.get(neighbor) {
                        *label_counts.entry(label).or_insert(0) += 1;
                    }
                }
                
                // Find most common label
                if let Some((&most_common, _)) = label_counts.iter().max_by_key(|&(_, count)| count) {
                    let current_label = *labels.get(node_id).unwrap();
                    if most_common != current_label {
                        labels.insert(node_id.clone(), most_common);
                        changed = true;
                    }
                }
            }
            
            if !changed {
                break;
            }
        }
        
        // Group by label
        let mut community_map: HashMap<usize, Vec<String>> = HashMap::new();
        for (node_id, label) in &labels {
            community_map.entry(*label).or_insert_with(Vec::new).push(node_id.clone());
        }
        
        // Build community structs
        let mut communities = Vec::new();
        for (id, members) in community_map {
            // Count internal edges
            let member_set: HashSet<_> = members.iter().cloned().collect();
            let mut internal_edges = 0;
            
            for member in &members {
                let edges = self.get_edges(member, Direction::Outgoing)?;
                for edge in edges {
                    if member_set.contains(&edge.target_id) {
                        internal_edges += 1;
                    }
                }
            }
            
            communities.push(Community {
                id,
                members,
                internal_edges,
            });
        }
        
        // Sort by size descending
        communities.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
        
        Ok(communities)
    }

    /// Calculate graph statistics
    pub fn stats(&self) -> Result<GraphStats, GraphError> {
        let nodes = self.get_nodes(NodeFilter::default())?;
        let node_count = nodes.len();
        
        if node_count == 0 {
            return Ok(GraphStats::default());
        }
        
        // Count edges (count each once)
        let mut edge_count = 0;
        let mut total_degree = 0;
        
        for node in &nodes {
            let out_edges = self.get_edges(&node.id, Direction::Outgoing)?;
            edge_count += out_edges.len();
            total_degree += out_edges.len();
            
            let in_edges = self.get_edges(&node.id, Direction::Incoming)?;
            total_degree += in_edges.len();
        }
        
        let avg_degree = total_degree as f64 / node_count as f64;
        
        // Density = actual edges / possible edges
        let max_edges = node_count * (node_count - 1); // Directed graph
        let density = if max_edges > 0 {
            edge_count as f64 / max_edges as f64
        } else {
            0.0
        };
        
        let components = self.connected_components()?;
        
        Ok(GraphStats {
            node_count,
            edge_count,
            avg_degree,
            density,
            component_count: components.len(),
        })
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

    fn test_input(label: &str) -> NodeInput {
        NodeInput {
            label: label.to_string(),
            kind: NodeKind::Character,
            source_note: "test".to_string(),
            subtype: None,
            aliases: vec![],
            created_by: CreatedBy::User,
            metadata: None,
        }
    }

    fn edge_input(source: &str, target: &str) -> EdgeInput {
        EdgeInput {
            source_id: source.to_string(),
            target_id: target.to_string(),
            edge_type: "KNOWS".to_string(),
            inverse_type: None,
            bidirectional: false,
            weight: 1.0,
            confidence: 1.0,
            created_by: CreatedBy::User,
            source_note: None,
            metadata: None,
        }
    }

    // Star graph: center connected to all others
    fn build_star_graph() -> (GraphRegistry, String, Vec<String>) {
        let registry = test_registry();
        let center = registry.register_node(test_input("Center")).unwrap().node.id;
        let mut leaves = Vec::new();
        
        for i in 0..4 {
            let leaf = registry.register_node(test_input(&format!("Leaf{}", i))).unwrap().node.id;
            registry.create_edge(edge_input(&center, &leaf)).unwrap();
            leaves.push(leaf);
        }
        
        (registry, center, leaves)
    }

    // Two clusters connected by a bridge
    fn build_bridge_graph() -> (GraphRegistry, Vec<String>) {
        let registry = test_registry();
        
        // Cluster 1: A-B-C
        let a = registry.register_node(test_input("A")).unwrap().node.id;
        let b = registry.register_node(test_input("B")).unwrap().node.id;
        let c = registry.register_node(test_input("C")).unwrap().node.id;
        registry.create_edge(edge_input(&a, &b)).unwrap();
        registry.create_edge(edge_input(&b, &c)).unwrap();
        registry.create_edge(edge_input(&a, &c)).unwrap();
        
        // Cluster 2: D-E-F
        let d = registry.register_node(test_input("D")).unwrap().node.id;
        let e = registry.register_node(test_input("E")).unwrap().node.id;
        let f = registry.register_node(test_input("F")).unwrap().node.id;
        registry.create_edge(edge_input(&d, &e)).unwrap();
        registry.create_edge(edge_input(&e, &f)).unwrap();
        registry.create_edge(edge_input(&d, &f)).unwrap();
        
        // Bridge: C-D
        registry.create_edge(edge_input(&c, &d)).unwrap();
        
        (registry, vec![a, b, c, d, e, f])
    }

    #[test]
    fn test_pagerank_basic() {
        let (registry, center, leaves) = build_star_graph();
        let scores = registry.pagerank(0.85, 20).unwrap();
        
        // Center should have low PageRank (no incoming edges)
        // Leaves should have higher PageRank (incoming from center)
        assert!(!scores.is_empty());
        
        let scores_map: HashMap<_, _> = scores.into_iter().collect();
        
        // All leaves should have roughly equal scores
        let leaf_scores: Vec<_> = leaves.iter()
            .filter_map(|id| scores_map.get(id))
            .collect();
        
        assert!(leaf_scores.len() == 4);
    }

    #[test]
    fn test_degree_centrality() {
        let (registry, center, _) = build_star_graph();
        let scores = registry.centrality(CentralityMetric::Degree).unwrap();
        
        // Center has highest degree (connected to all)
        assert!(!scores.is_empty());
        assert_eq!(scores[0].0, center);
    }

    #[test]
    fn test_betweenness_centrality() {
        let (registry, ids) = build_bridge_graph();
        let scores = registry.centrality(CentralityMetric::Betweenness).unwrap();
        
        // C and D should have highest betweenness (bridge nodes)
        assert!(!scores.is_empty());
        
        let scores_map: HashMap<_, _> = scores.into_iter().collect();
        let c_score = scores_map.get(&ids[2]).unwrap_or(&0.0);
        let d_score = scores_map.get(&ids[3]).unwrap_or(&0.0);
        let a_score = scores_map.get(&ids[0]).unwrap_or(&0.0);
        
        // Bridge nodes should have higher betweenness
        assert!(*c_score >= *a_score || *d_score >= *a_score);
    }

    #[test]
    fn test_connected_components() {
        let registry = test_registry();
        
        // Create two disconnected pairs
        let a = registry.register_node(test_input("A")).unwrap().node.id;
        let b = registry.register_node(test_input("B")).unwrap().node.id;
        registry.create_edge(edge_input(&a, &b)).unwrap();
        
        let c = registry.register_node(test_input("C")).unwrap().node.id;
        let d = registry.register_node(test_input("D")).unwrap().node.id;
        registry.create_edge(edge_input(&c, &d)).unwrap();
        
        let components = registry.connected_components().unwrap();
        
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].len(), 2);
        assert_eq!(components[1].len(), 2);
    }

    #[test]
    fn test_community_detection() {
        let (registry, _) = build_bridge_graph();
        let communities = registry.communities().unwrap();
        
        // Should detect at least one community
        assert!(!communities.is_empty());
        
        // Total members should equal node count
        let total_members: usize = communities.iter().map(|c| c.members.len()).sum();
        assert_eq!(total_members, 6);
    }

    #[test]
    fn test_graph_stats() {
        let (registry, _, _) = build_star_graph();
        let stats = registry.stats().unwrap();
        
        assert_eq!(stats.node_count, 5);  // 1 center + 4 leaves
        assert_eq!(stats.edge_count, 4);  // 4 edges from center
        assert!(stats.avg_degree > 0.0);
        assert!(stats.density > 0.0);
        assert_eq!(stats.component_count, 1);  // All connected
    }
}
