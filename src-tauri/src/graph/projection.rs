//! Graph Projection Layer
//!
//! Provides a high-performance in-memory graph projection using `petgraph`.
//! This acts as a read-heavy optimization layer sitting on top of the
//! authoritative CozoDB storage.
//!
//! The projection is "hydrated" from CozoDB and used for expensive algorithms
//! like PageRank, Centrality, and complex Pathfinding.

use std::collections::HashMap;
use petgraph::graph::{NodeIndex, EdgeIndex};
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::{EdgeRef, IntoNodeReferences};
use petgraph::algo;
use cozo::DbInstance;
use serde::{Deserialize, Serialize};

use super::graph_backend::{GraphBackend, GraphNode, GraphEdge, RankedEntity, GraphCommunity, GraphPath, GraphStatistics};
use super::content_repos::{NoteRepo, EntityRepo}; // Or generic query helpers

/// A high-performance in-memory projection of the graph.
/// Uses stable indices to allow for easier synchronization (future proofing).
pub struct GraphProjection {
    /// The underlying graph structure
    graph: StableDiGraph<GraphNode, GraphEdge>,
    
    /// Fast lookup: External ID (UUID) -> Internal NodeIndex
    id_to_index: HashMap<String, NodeIndex>,
    
    /// Reverse lookup: Internal NodeIndex -> External ID
    index_to_id: HashMap<NodeIndex, String>,
}

impl Default for GraphProjection {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphProjection {
    /// Create a new empty projection
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::new(),
            id_to_index: HashMap::new(),
            index_to_id: HashMap::new(),
        }
    }

    /// Hydrate the projection from CozoDB
    /// 
    /// This wipes the current state and rebuilds from the database.
    /// In a large production app, incremental hydration would be preferred,
    /// but full rebuild is fine for typical graph sizes (<100k nodes).
    pub fn hydrate(&mut self, db: &DbInstance) -> Result<(), String> {
        self.clear();

        // 1. Fetch all nodes from 'nodes' relation
        // Schema: id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata
        let node_query = r#"
            ?[id, label, kind] := *nodes[id, label, _norm, kind, _sub, _src, _at, _by, _mc, _meta]
        "#;
        
        let result = db.run_script(node_query, Default::default(), cozo::ScriptMutability::Immutable)
            .map_err(|e| format!("Node hydration failed: {}", e))?;

        for row in result.rows {
            if row.len() < 3 { continue; }
            let id = row[0].get_str().unwrap_or_default().to_string();
            let label = row[1].get_str().unwrap_or_default().to_string();
            let kind = row[2].get_str().unwrap_or_default().to_string();

            self.add_node(GraphNode { id, label, kind });
        }

        // 2. Fetch all edges
        // Schema: id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata
        let edge_query = r#"
            ?[source, target, type, weight] := *edges[_id, source, target, type, _inv, _bi, weight, _conf, _at, _by, _src, _meta]
        "#;
        
        let edge_result = db.run_script(edge_query, Default::default(), cozo::ScriptMutability::Immutable)
             .map_err(|e| format!("Edge hydration failed: {}", e))?;

        for row in edge_result.rows {
            if row.len() < 4 { continue; }
            let source_id = row[0].get_str().unwrap_or_default();
            let target_id = row[1].get_str().unwrap_or_default();
            let type_ = row[2].get_str().unwrap_or_default().to_string();
            let weight = match &row[3] {
                cozo::DataValue::Num(cozo::Num::Float(f)) => *f,
                cozo::DataValue::Num(cozo::Num::Int(i)) => *i as f64,
                _ => 1.0,
            };

            // Only add edge if both nodes exist (graph integrity)
            if self.id_to_index.contains_key(source_id) && self.id_to_index.contains_key(target_id) {
                let edge = GraphEdge {
                    source_id: source_id.to_string(),
                    target_id: target_id.to_string(),
                    edge_type: type_,
                    weight,
                };
                self.add_edge_internal(edge);
            }
        }

        Ok(())
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.graph.clear();
        self.id_to_index.clear();
        self.index_to_id.clear();
    }

    /// Add a node to the graph
    fn add_node(&mut self, node: GraphNode) {
        if self.id_to_index.contains_key(&node.id) {
            return; // Already exists
        }
        
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.id_to_index.insert(id.clone(), idx);
        self.index_to_id.insert(idx, id);
    }

    /// Add an edge (internal helper)
    fn add_edge_internal(&mut self, edge: GraphEdge) {
        if let (Some(&src), Some(&tgt)) = (self.id_to_index.get(&edge.source_id), self.id_to_index.get(&edge.target_id)) {
            self.graph.add_edge(src, tgt, edge);
        }
    }

    /// Identify weakly connected components (ignoring edge direction)
    pub fn connected_components(&self) -> Result<Vec<Vec<String>>, String> {
        let mut visited = std::collections::HashSet::new();
        let mut components = Vec::new();
        
        for start_node in self.graph.node_indices() {
             if visited.contains(&start_node) { continue; }
             
             let mut component = Vec::new();
             let mut queue = std::collections::VecDeque::new();
             queue.push_back(start_node);
             visited.insert(start_node);
             
             while let Some(idx) = queue.pop_front() {
                 if let Some(id) = self.index_to_id.get(&idx) {
                     component.push(id.clone());
                 }
                 
                 // Walk neighbors in BOTH directions manually
                 for neighbor in self.graph.neighbors(idx) {
                     if !visited.contains(&neighbor) {
                         visited.insert(neighbor);
                         queue.push_back(neighbor);
                     }
                 }
                 for neighbor in self.graph.neighbors_directed(idx, petgraph::Direction::Incoming) {
                     if !visited.contains(&neighbor) {
                         visited.insert(neighbor);
                         queue.push_back(neighbor);
                     }
                 }
             }
             components.push(component);
        }
        
        components.sort_by(|a, b| b.len().cmp(&a.len()));
        Ok(components)
    }
}

// Implement the unified GraphBackend trait
impl GraphBackend for GraphProjection {
    type Error = String;

    fn ensure_node(&self, _node: &GraphNode) -> Result<(), Self::Error> {
        // Projection is READ-ONLY for "write" ops in this context.
        // You should write to Cozo, then hydrate.
        // Or, we could implement write-through, but let's encourage hydration pattern.
        Err("Cannot write directly to GraphProjection. Write to DB and hydrate.".to_string())
    }

    fn get_node(&self, id: &str) -> Result<Option<GraphNode>, Self::Error> {
        if let Some(&idx) = self.id_to_index.get(id) {
            Ok(Some(self.graph[idx].clone()))
        } else {
            Ok(None)
        }
    }

    fn node_count(&self) -> Result<usize, Self::Error> {
        Ok(self.graph.node_count())
    }

    fn add_edge(&self, _edge: &GraphEdge) -> Result<(), Self::Error> {
         Err("Cannot write directly to GraphProjection. Write to DB and hydrate.".to_string())
    }

    fn outgoing_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>, Self::Error> {
        let Some(&idx) = self.id_to_index.get(node_id) else {
            return Ok(vec![]);
        };

        Ok(self.graph.edges(idx)
            .map(|e| e.weight().clone()) // GraphEdge is the weight type
            .collect())
    }

    fn incoming_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>, Self::Error> {
         let Some(&idx) = self.id_to_index.get(node_id) else {
            return Ok(vec![]);
        };

        // petgraph's edges_directed(Incoming)
        Ok(self.graph.edges_directed(idx, petgraph::Direction::Incoming)
            .map(|e| e.weight().clone())
            .collect())
    }

    fn edge_count(&self) -> Result<usize, Self::Error> {
        Ok(self.graph.edge_count())
    }

    // --- ALGORITHMS ---

    fn pagerank(&self) -> Result<Vec<RankedEntity>, Self::Error> {
        // Petgraph's pagerank is often simple or missing in some versions/features.
        // We implement a simple iterative PageRank here if needed, 
        // OR use `petgraph::algo::page_rank` if available (it often isn't exposed perfectly).
        // Let's implement a standard power iteration for robustness.
        
        let damping_factor = 0.85;
        let tolerance = 0.0001;
        let max_iter = 100;
        let n = self.graph.node_count();
        
        if n == 0 { return Ok(vec![]); }

        let mut scores: HashMap<NodeIndex, f64> = self.graph.node_indices()
            .map(|idx| (idx, 1.0 / n as f64))
            .collect();
            
        let mut new_scores = scores.clone();

        for _ in 0..max_iter {
            let mut diff = 0.0;
            
            for node_idx in self.graph.node_indices() {
                let mut incoming_sum = 0.0;
                
                // For each incoming node
                for edge in self.graph.edges_directed(node_idx, petgraph::Direction::Incoming) {
                    let source_idx = edge.source();
                    let out_degree = self.graph.edges(source_idx).count();
                    
                    if out_degree > 0 {
                        if let Some(score) = scores.get(&source_idx) {
                             incoming_sum += score / out_degree as f64;
                        }
                    }
                }
                
                let rank = (1.0 - damping_factor) / n as f64 + damping_factor * incoming_sum;
                new_scores.insert(node_idx, rank);
                
                if let Some(old_score) = scores.get(&node_idx) {
                    diff += (rank - old_score).abs();
                }
            }
            
            scores = new_scores.clone();
            if diff < tolerance { break; }
        }

        // Convert to RankedEntity
        let mut ranked: Vec<RankedEntity> = scores.into_iter()
            .map(|(idx, score)| {
                let node = &self.graph[idx];
                RankedEntity {
                    id: node.id.clone(),
                    label: node.label.clone(),
                    score,
                    rank: 0, // Fill after sort
                }
            })
            .collect();

        // Sort descending
        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Assign ranks
        for (i, entity) in ranked.iter_mut().enumerate() {
            entity.rank = i + 1;
        }

        Ok(ranked)
    }

    fn shortest_path(&self, source: &str, target: &str) -> Result<Option<GraphPath>, Self::Error> {
        let Some(&src_idx) = self.id_to_index.get(source) else { return Ok(None); };
        let Some(&tgt_idx) = self.id_to_index.get(target) else { return Ok(None); };

        // Uniform cost (BFS) or Dijkstra? Edge weights present?
        // Let's use Dijkstra since we have weights.
        let res = algo::dijkstra(&self.graph, src_idx, Some(tgt_idx), |e| e.weight().weight);
        
        if let Some(&dist) = res.get(&tgt_idx) {
            // Reconstruct path (dijkstra just returns costs map)
            // Petgraph `astar` might be easier for single path, but let's assume existence implies path.
            // Actually, we need the path. `astar` returns path.
            
            let path_res = algo::astar(
                &self.graph, 
                src_idx, 
                |finish| finish == tgt_idx, 
                |e| e.weight().weight, 
                |_| 0.0 // No heuristic
            );

            if let Some((cost, path_indices)) = path_res {
                let nodes: Vec<String> = path_indices.into_iter()
                    .map(|idx| self.index_to_id.get(&idx).cloned().unwrap_or_default())
                    .collect();
                
                return Ok(Some(GraphPath {
                    nodes,
                    edges: vec![], // Populating edges requires iterating path pairs
                    distance: cost,
                }));
            }
        }
        
        Ok(None)
    }


    fn detect_communities(&self) -> Result<Vec<GraphCommunity>, Self::Error> {
        // Use SCC for community detection for now, or implement LPA
        let sccs = algo::kosaraju_scc(&self.graph);
        
        let communities = sccs.into_iter().enumerate().map(|(id, indices)| {
            let members = indices.into_iter()
                .filter_map(|idx| self.index_to_id.get(&idx).cloned())
                .collect();
                
            GraphCommunity {
                id,
                members
            }
        }).collect();
        
        Ok(communities)
    }

    fn neighborhood(&self, node_id: &str, max_hops: usize) -> Result<Vec<(String, usize)>, Self::Error> {
        let Some(&start_idx) = self.id_to_index.get(node_id) else {
            return Ok(vec![]);
        };

        // Simple BFS up to max_hops
        let mut visited = HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        
        visited.insert(start_idx, 0);
        queue.push_back((start_idx, 0));
        
        let mut result = Vec::new();

        while let Some((curr, depth)) = queue.pop_front() {
            if depth >= max_hops { continue; }
            
            for neighbor in self.graph.neighbors(curr) {
                if !visited.contains_key(&neighbor) {
                    let new_depth = depth + 1;
                    visited.insert(neighbor, new_depth);
                    queue.push_back((neighbor, new_depth));
                    
                    if let Some(id) = self.index_to_id.get(&neighbor) {
                        result.push((id.clone(), new_depth));
                    }
                }
            }
        }
        
        Ok(result)
    }
}
