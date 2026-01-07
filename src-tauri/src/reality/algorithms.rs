//! Graph Algorithms: Community Detection, Path Analysis, PageRank
//!
//! Evolution 3.0: Graph Algorithm Layer
//!
//! Provides in-memory, real-time graph analysis using petgraph algorithms.
//! Complements CozoDB's persistent graph layer with fast, iterative analysis.

use std::collections::{HashMap, HashSet, VecDeque};
use super::graph::{ConceptGraph, ConceptNode, ConceptEdge};
use rustworkx_core::petgraph::graph::NodeIndex;
use rustworkx_core::petgraph::Direction;
use rustworkx_core::petgraph::visit::EdgeRef;

// =============================================================================
// Types
// =============================================================================

/// A detected community/cluster
#[derive(Debug, Clone)]
pub struct Community {
    pub id: usize,
    pub members: Vec<String>,
    pub label: String,
    pub cohesion: f64,
}

/// An entity that bridges multiple communities
#[derive(Debug, Clone)]
pub struct BridgeEntity {
    pub entity_id: String,
    pub entity_name: String,
    pub communities: Vec<usize>,
    pub bridge_score: f64,
}

/// A path between two entities
#[derive(Debug, Clone)]
pub struct Path {
    pub entities: Vec<String>,
    pub edges: Vec<String>,
    pub cost: f64,
    pub narrative: String,
}

/// An entity with importance ranking
#[derive(Debug, Clone)]
pub struct RankedEntity {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub score: f64,
    pub rank: usize,
}

/// Importance comparison report
#[derive(Debug, Clone)]
pub struct ImportanceReport {
    pub by_pagerank: Vec<RankedEntity>,
    pub by_degree: Vec<RankedEntity>,
    pub outliers: Vec<OutlierEntity>,
}

/// Entity that ranks differently by PageRank vs Degree
#[derive(Debug, Clone)]
pub struct OutlierEntity {
    pub id: String,
    pub name: String,
    pub pagerank_rank: usize,
    pub degree_rank: usize,
    pub rank_delta: i32,
}

// =============================================================================
// Graph Algorithm Extensions
// =============================================================================

impl ConceptGraph {
    // -------------------------------------------------------------------------
    // Community Detection (Label Propagation)
    // -------------------------------------------------------------------------
    
    /// Detect communities using Label Propagation algorithm
    /// 
    /// This is a fast O(V+E) per iteration algorithm that's simpler than Louvain
    /// but gives similar results for narrative clustering.
    pub fn detect_communities(&self, max_iterations: usize) -> Vec<Community> {
        if self.is_empty() {
            return vec![];
        }
        
        let node_ids: Vec<String> = self.nodes().map(|n| n.id.clone()).collect();
        if node_ids.is_empty() {
            return vec![];
        }
        
        // Initialize: each node gets its own label
        let mut labels: HashMap<String, usize> = node_ids.iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), i))
            .collect();
        
        // Iterate until convergence or max iterations
        for _ in 0..max_iterations {
            let mut changed = false;
            
            for node_id in &node_ids {
                // Get neighbor labels with weights
                let mut label_weights: HashMap<usize, f64> = HashMap::new();
                
                for (neighbor, edge) in self.outgoing_edges(node_id) {
                    if let Some(&neighbor_label) = labels.get(&neighbor.id) {
                        *label_weights.entry(neighbor_label).or_default() += edge.weight;
                    }
                }
                
                for (neighbor, edge) in self.incoming_edges(node_id) {
                    if let Some(&neighbor_label) = labels.get(&neighbor.id) {
                        *label_weights.entry(neighbor_label).or_default() += edge.weight;
                    }
                }
                
                // Assign label with highest weight
                if let Some((&best_label, _)) = label_weights.iter()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                {
                    if labels.get(node_id) != Some(&best_label) {
                        labels.insert(node_id.clone(), best_label);
                        changed = true;
                    }
                }
            }
            
            if !changed {
                break;
            }
        }
        
        // Group nodes by label
        let mut label_groups: HashMap<usize, Vec<String>> = HashMap::new();
        for (node_id, label) in &labels {
            label_groups.entry(*label).or_default().push(node_id.clone());
        }
        
        // Convert to communities
        let mut communities: Vec<Community> = Vec::new();
        for (label, members) in label_groups {
            if members.is_empty() {
                continue;
            }
            
            // Calculate cohesion (internal edge density)
            let internal_edges = self.count_internal_edges(&members);
            let max_possible = members.len() * (members.len().saturating_sub(1));
            let cohesion = if max_possible > 0 {
                internal_edges as f64 / max_possible as f64
            } else {
                0.0
            };
            
            // Generate label from top members
            let top_names: Vec<String> = members.iter()
                .take(3)
                .filter_map(|id| self.get_node(id).map(|n| n.label.clone()))
                .collect();
            
            let community_label = if top_names.is_empty() {
                format!("Community {}", label)
            } else if top_names.len() == 1 {
                top_names[0].clone()
            } else {
                format!("{}'s Circle", top_names[0])
            };
            
            communities.push(Community {
                id: label,
                members,
                label: community_label,
                cohesion,
            });
        }
        
        // Sort by size descending
        communities.sort_by(|a, b| b.members.len().cmp(&a.members.len()));
        communities
    }
    
    /// Get the community containing a specific entity
    pub fn community_of<'a>(&self, entity_id: &str, communities: &'a [Community]) -> Option<&'a Community> {
        communities.iter().find(|c| c.members.contains(&entity_id.to_string()))
    }

    
    /// Find bridge entities (connected to multiple communities)
    pub fn find_bridges(&self, communities: &[Community]) -> Vec<BridgeEntity> {
        // Build entity → community mapping
        let mut entity_community: HashMap<String, usize> = HashMap::new();
        for community in communities {
            for member in &community.members {
                entity_community.insert(member.clone(), community.id);
            }
        }

        
        let mut bridges: Vec<BridgeEntity> = Vec::new();
        
        for node in self.nodes() {
            let my_community = entity_community.get(node.id.as_str());
            let mut connected_communities: HashSet<usize> = HashSet::new();
            
            // Check outgoing edges
            for (neighbor, _) in self.outgoing_edges(&node.id) {
                if let Some(&comm) = entity_community.get(neighbor.id.as_str()) {
                    if my_community != Some(&comm) {
                        connected_communities.insert(comm);
                    }
                }
            }
            
            // Check incoming edges
            for (neighbor, _) in self.incoming_edges(&node.id) {
                if let Some(&comm) = entity_community.get(neighbor.id.as_str()) {
                    if my_community != Some(&comm) {
                        connected_communities.insert(comm);
                    }
                }
            }
            
            if !connected_communities.is_empty() {
                let bridge_score = connected_communities.len() as f64;
                bridges.push(BridgeEntity {
                    entity_id: node.id.clone(),
                    entity_name: node.label.clone(),
                    communities: connected_communities.into_iter().collect(),
                    bridge_score,
                });
            }
        }
        
        // Sort by bridge score descending
        bridges.sort_by(|a, b| b.bridge_score.partial_cmp(&a.bridge_score).unwrap_or(std::cmp::Ordering::Equal));
        bridges
    }
    
    /// Count edges between members of a group
    fn count_internal_edges(&self, members: &[String]) -> usize {
        let member_set: HashSet<&String> = members.iter().collect();
        let mut count = 0;
        
        for member_id in members {
            for (neighbor, _) in self.outgoing_edges(member_id) {
                if member_set.contains(&neighbor.id) {
                    count += 1;
                }
            }
        }
        
        count
    }
    
    // -------------------------------------------------------------------------
    // Path Analysis
    // -------------------------------------------------------------------------
    
    /// Find shortest path between two entities using BFS (unweighted)
    pub fn shortest_path(&self, source: &str, target: &str) -> Option<Path> {
        let source_idx = self.get_index(source)?;
        let target_idx = self.get_index(target)?;
        
        if source_idx == target_idx {
            let node = self.get_node(source)?;
            return Some(Path {
                entities: vec![node.id.clone()],
                edges: vec![],
                cost: 0.0,
                narrative: node.label.clone(),
            });
        }
        
        // BFS
        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut parent: HashMap<NodeIndex, (NodeIndex, String)> = HashMap::new();
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();
        
        queue.push_back(source_idx);
        visited.insert(source_idx);
        
        while let Some(current) = queue.pop_front() {
            // Check all neighbors (undirected)
            for edge_ref in self.graph().edges_directed(current, Direction::Outgoing) {
                let neighbor = edge_ref.target();
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, (current, edge_ref.weight().relation.clone()));
                    
                    if neighbor == target_idx {
                        return Some(self.reconstruct_path(source_idx, target_idx, &parent));
                    }
                    
                    queue.push_back(neighbor);
                }
            }
            
            for edge_ref in self.graph().edges_directed(current, Direction::Incoming) {
                let neighbor = edge_ref.source();
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, (current, edge_ref.weight().relation.clone()));
                    
                    if neighbor == target_idx {
                        return Some(self.reconstruct_path(source_idx, target_idx, &parent));
                    }
                    
                    queue.push_back(neighbor);
                }
            }
        }
        
        None // No path found
    }
    
    /// Get entities within N hops of an entity
    pub fn neighborhood(&self, entity_id: &str, max_hops: usize) -> Vec<(String, usize)> {
        let Some(start_idx) = self.get_index(entity_id) else {
            return vec![];
        };
        
        let mut visited: HashMap<NodeIndex, usize> = HashMap::new();
        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
        
        queue.push_back((start_idx, 0));
        visited.insert(start_idx, 0);
        
        while let Some((current, distance)) = queue.pop_front() {
            if distance >= max_hops {
                continue;
            }
            
            // Explore neighbors (undirected)
            for edge_ref in self.graph().edges_directed(current, Direction::Outgoing) {
                let neighbor = edge_ref.target();
                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, distance + 1);
                    queue.push_back((neighbor, distance + 1));
                }
            }
            
            for edge_ref in self.graph().edges_directed(current, Direction::Incoming) {
                let neighbor = edge_ref.source();
                if !visited.contains_key(&neighbor) {
                    visited.insert(neighbor, distance + 1);
                    queue.push_back((neighbor, distance + 1));
                }
            }
        }
        
        // Convert to result
        let mut result: Vec<(String, usize)> = visited.iter()
            .filter(|(&idx, _)| idx != start_idx)
            .filter_map(|(&idx, &dist)| {
                self.graph().node_weight(idx).map(|n| (n.id.clone(), dist))
            })
            .collect();
        
        result.sort_by_key(|(_, dist)| *dist);
        result
    }
    
    fn reconstruct_path(
        &self, 
        source: NodeIndex, 
        target: NodeIndex, 
        parent: &HashMap<NodeIndex, (NodeIndex, String)>
    ) -> Path {
        let mut entities: Vec<String> = Vec::new();
        let mut edges: Vec<String> = Vec::new();
        let mut current = target;
        
        while let Some((prev, edge_label)) = parent.get(&current) {
            if let Some(node) = self.graph().node_weight(current) {
                entities.push(node.id.clone());
            }
            edges.push(edge_label.clone());
            current = *prev;
        }
        
        // Add source
        if let Some(node) = self.graph().node_weight(source) {
            entities.push(node.id.clone());
        }
        
        entities.reverse();
        edges.reverse();
        
        // Generate narrative
        let edges_len = edges.len();
        let narrative = self.generate_path_narrative(&entities, &edges);
        
        Path {
            entities,
            edges,
            cost: edges_len as f64,
            narrative,
        }
    }
    
    fn generate_path_narrative(&self, entity_ids: &[String], edges: &[String]) -> String {
        if entity_ids.is_empty() {
            return "No connection".to_string();
        }
        
        let names: Vec<String> = entity_ids.iter()
            .filter_map(|id| self.get_node(id).map(|n| n.label.clone()))
            .collect();
        
        if names.len() == 1 {
            return names[0].clone();
        }
        
        // Build: "Frodo → CARRIES → Ring → CREATED_BY → Sauron"
        let mut parts: Vec<String> = Vec::new();
        for (i, name) in names.iter().enumerate() {
            parts.push(name.clone());
            if i < edges.len() {
                parts.push(format!("[{}]", edges[i].to_uppercase()));
            }
        }
        
        parts.join(" → ")
    }
    
    // -------------------------------------------------------------------------
    // PageRank
    // -------------------------------------------------------------------------
    
    /// Compute PageRank scores using power iteration
    pub fn pagerank(&self, damping: f64, iterations: usize) -> Vec<RankedEntity> {
        if self.is_empty() {
            return vec![];
        }
        
        let n = self.node_count();
        let initial_score = 1.0 / n as f64;
        
        // Initialize scores
        let node_ids: Vec<String> = self.nodes().map(|n| n.id.clone()).collect();
        let mut scores: HashMap<String, f64> = node_ids.iter()
            .map(|id| (id.clone(), initial_score))
            .collect();
        
        // Precompute out-degrees
        let out_degrees: HashMap<&str, usize> = node_ids.iter()
            .map(|id| (id.as_str(), self.outgoing_edges(id).len()))
            .collect();
        
        // Power iteration
        for _ in 0..iterations {
            let mut new_scores: HashMap<String, f64> = HashMap::new();
            
            for node_id in &node_ids {
                let mut incoming_score = 0.0;
                
                // Sum contributions from incoming neighbors
                for (neighbor, _) in self.incoming_edges(node_id) {
                    let neighbor_out_degree = out_degrees.get(neighbor.id.as_str()).copied().unwrap_or(1);
                    if neighbor_out_degree > 0 {
                        let neighbor_score = scores.get(&neighbor.id).copied().unwrap_or(initial_score);
                        incoming_score += neighbor_score / neighbor_out_degree as f64;
                    }
                }
                
                // PageRank formula
                let new_score = (1.0 - damping) / n as f64 + damping * incoming_score;
                new_scores.insert(node_id.clone(), new_score);
            }
            
            scores = new_scores;
        }
        
        // Convert to ranked entities
        let mut ranked: Vec<RankedEntity> = scores.iter()
            .filter_map(|(id, &score)| {
                self.get_node(id).map(|node| RankedEntity {
                    id: id.clone(),
                    name: node.label.clone(),
                    kind: node.kind.clone(),
                    score,
                    rank: 0,
                })
            })
            .collect();
        
        // Sort by score descending and assign ranks
        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        for (i, entity) in ranked.iter_mut().enumerate() {
            entity.rank = i + 1;
        }
        
        ranked
    }
    
    /// Get top N most important entities by PageRank
    pub fn top_entities(&self, n: usize) -> Vec<RankedEntity> {
        self.pagerank(0.85, 20).into_iter().take(n).collect()
    }
    
    /// Compare PageRank vs Degree centrality
    pub fn importance_analysis(&self) -> ImportanceReport {
        let by_pagerank = self.pagerank(0.85, 20);
        
        // Calculate degree centrality
        let mut by_degree: Vec<RankedEntity> = self.nodes()
            .map(|node| {
                let degree = self.outgoing_edges(&node.id).len() + self.incoming_edges(&node.id).len();
                RankedEntity {
                    id: node.id.clone(),
                    name: node.label.clone(),
                    kind: node.kind.clone(),
                    score: degree as f64,
                    rank: 0,
                }
            })
            .collect();
        
        by_degree.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        for (i, entity) in by_degree.iter_mut().enumerate() {
            entity.rank = i + 1;
        }
        
        // Find outliers (entities that rank very differently)
        let pagerank_ranks: HashMap<&str, usize> = by_pagerank.iter()
            .map(|e| (e.id.as_str(), e.rank))
            .collect();
        
        let outliers: Vec<OutlierEntity> = by_degree.iter()
            .filter_map(|e| {
                let pr_rank = pagerank_ranks.get(e.id.as_str()).copied().unwrap_or(e.rank);
                let delta = (e.rank as i32 - pr_rank as i32).abs();
                if delta >= 3 {
                    Some(OutlierEntity {
                        id: e.id.clone(),
                        name: e.name.clone(),
                        pagerank_rank: pr_rank,
                        degree_rank: e.rank,
                        rank_delta: delta,
                    })
                } else {
                    None
                }
            })
            .collect();
        
        ImportanceReport {
            by_pagerank,
            by_degree,
            outliers,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    fn build_fellowship_graph() -> ConceptGraph {
        let mut graph = ConceptGraph::new();
        
        // Add characters
        graph.ensure_node(ConceptNode::new("frodo", "Frodo", "CHARACTER"));
        graph.ensure_node(ConceptNode::new("sam", "Sam", "CHARACTER"));
        graph.ensure_node(ConceptNode::new("gandalf", "Gandalf", "CHARACTER"));
        graph.ensure_node(ConceptNode::new("aragorn", "Aragorn", "CHARACTER"));
        graph.ensure_node(ConceptNode::new("legolas", "Legolas", "CHARACTER"));
        graph.ensure_node(ConceptNode::new("gimli", "Gimli", "CHARACTER"));
        graph.ensure_node(ConceptNode::new("boromir", "Boromir", "CHARACTER"));
        graph.ensure_node(ConceptNode::new("ring", "The One Ring", "ITEM"));
        graph.ensure_node(ConceptNode::new("sauron", "Sauron", "CHARACTER"));
        
        // Add relationships
        graph.add_edge("frodo", "sam", ConceptEdge::new("FRIENDS_WITH", 0.9));
        graph.add_edge("frodo", "gandalf", ConceptEdge::new("GUIDED_BY", 0.8));
        graph.add_edge("frodo", "ring", ConceptEdge::new("CARRIES", 1.0));
        graph.add_edge("gandalf", "aragorn", ConceptEdge::new("ALLIES_WITH", 0.8));
        graph.add_edge("aragorn", "legolas", ConceptEdge::new("ALLIES_WITH", 0.7));
        graph.add_edge("aragorn", "gimli", ConceptEdge::new("ALLIES_WITH", 0.7));
        graph.add_edge("boromir", "ring", ConceptEdge::new("TEMPTED_BY", 0.6));
        graph.add_edge("ring", "sauron", ConceptEdge::new("CREATED_BY", 1.0));
        
        graph
    }
    
    // -------------------------------------------------------------------------
    // Community Detection Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_community_detection_basic() {
        let graph = build_fellowship_graph();
        let communities = graph.detect_communities(10);
        
        assert!(!communities.is_empty(), "Should detect at least one community");
        
        // All entities should be in some community
        let total_members: usize = communities.iter().map(|c| c.members.len()).sum();
        assert_eq!(total_members, graph.node_count());
    }
    
    #[test]
    fn test_community_of() {
        let graph = build_fellowship_graph();
        let communities = graph.detect_communities(10);
        
        let frodo_community = graph.community_of("frodo", &communities);
        assert!(frodo_community.is_some());
    }
    
    #[test]
    fn test_find_bridges() {
        let graph = build_fellowship_graph();
        let communities = graph.detect_communities(10);
        let bridges = graph.find_bridges(&communities);
        
        // In a small connected graph, bridges might exist
        // depending on community structure
        println!("Bridges found: {:?}", bridges);
    }
    
    // -------------------------------------------------------------------------
    // Path Analysis Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_shortest_path_direct() {
        let graph = build_fellowship_graph();
        
        let path = graph.shortest_path("frodo", "sam");
        assert!(path.is_some());
        
        let path = path.unwrap();
        assert_eq!(path.entities.len(), 2);
        assert!(path.narrative.contains("Frodo"));
        assert!(path.narrative.contains("Sam"));
    }
    
    #[test]
    fn test_shortest_path_multi_hop() {
        let graph = build_fellowship_graph();
        
        // Frodo → Ring → Sauron
        let path = graph.shortest_path("frodo", "sauron");
        assert!(path.is_some());
        
        let path = path.unwrap();
        assert!(path.entities.len() >= 2, "Should have multi-hop path");
        println!("Path narrative: {}", path.narrative);
    }
    
    #[test]
    fn test_shortest_path_no_connection() {
        let mut graph = ConceptGraph::new();
        graph.ensure_node(ConceptNode::new("a", "A", "TYPE"));
        graph.ensure_node(ConceptNode::new("b", "B", "TYPE"));
        // No edge between them
        
        let path = graph.shortest_path("a", "b");
        assert!(path.is_none());
    }
    
    #[test]
    fn test_neighborhood() {
        let graph = build_fellowship_graph();
        
        let neighbors = graph.neighborhood("frodo", 1);
        assert!(!neighbors.is_empty());
        
        // Sam should be 1 hop away
        assert!(neighbors.iter().any(|(id, dist)| id == "sam" && *dist == 1));
    }
    
    #[test]
    fn test_neighborhood_multi_hop() {
        let graph = build_fellowship_graph();
        
        let neighbors = graph.neighborhood("frodo", 3);
        
        // Sauron should be reachable within 3 hops
        assert!(neighbors.iter().any(|(id, _)| id == "sauron"));
    }
    
    // -------------------------------------------------------------------------
    // PageRank Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_pagerank_basic() {
        let graph = build_fellowship_graph();
        let ranked = graph.pagerank(0.85, 20);
        
        assert!(!ranked.is_empty());
        assert_eq!(ranked.len(), graph.node_count());
        
        // First entity should have rank 1
        assert_eq!(ranked[0].rank, 1);
        
        // All scores should be positive and non-zero
        assert!(ranked.iter().all(|e| e.score > 0.0), "All PageRank scores should be positive");
    }

    
    #[test]
    fn test_top_entities() {
        let graph = build_fellowship_graph();
        let top = graph.top_entities(3);
        
        assert_eq!(top.len(), 3);
        println!("Top 3 entities: {:?}", top.iter().map(|e| &e.name).collect::<Vec<_>>());
    }
    
    #[test]
    fn test_importance_analysis() {
        let graph = build_fellowship_graph();
        let report = graph.importance_analysis();
        
        assert_eq!(report.by_pagerank.len(), graph.node_count());
        assert_eq!(report.by_degree.len(), graph.node_count());
    }
    
    // -------------------------------------------------------------------------
    // Empty Graph Tests
    // -------------------------------------------------------------------------
    
    #[test]
    fn test_algorithms_on_empty_graph() {
        let graph = ConceptGraph::new();
        
        let communities = graph.detect_communities(10);
        assert!(communities.is_empty());
        
        let path = graph.shortest_path("a", "b");
        assert!(path.is_none());
        
        let ranked = graph.pagerank(0.85, 20);
        assert!(ranked.is_empty());
    }
}
