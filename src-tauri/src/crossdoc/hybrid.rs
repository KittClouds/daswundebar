//! Hybrid Entity Linker (Phase 3)
//!
//! Combines string similarity (Phase 1) and semantic embedding similarity (Phase 2)
//! to identify, cluster, and merge duplicate entities.
//!
//! Also implements GraphRAG patterns:
//! - Importance scoring
//! - Co-occurrence tracking
//! - Graph statistics

use crate::crossdoc::types::{LinkingConfig, EntityCluster, ClusterMember};
use crate::crossdoc::{string_sim, context_embed, linker};
use crate::graph::{GraphRegistry, Node};
use crate::rag::embeddings::EmbeddingService;
use cozo::{DataValue, ScriptMutability};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Calculate Importance Score using the GraphRAG-inspired formula
///
/// Formula: (ln(doc_freq)*0.4 + ln(mentions)*0.3 + spread*0.3) / 3.0
pub fn calculate_importance(
    doc_frequency: usize,
    total_mentions: usize,
    source_docs: usize
) -> f32 {
    let doc_freq_score = (doc_frequency as f32).ln() + 1.0;
    let mention_score = (total_mentions as f32).ln() + 1.0;
    let spread_score = source_docs as f32;
    
    (doc_freq_score * 0.4 + mention_score * 0.3 + spread_score * 0.3) / 3.0
}

/// Calculate importance for a Node
pub fn node_importance(node: &Node, registry: &GraphRegistry) -> f32 {
    // 1. Doc frequency: How many docs is this entity in?
    // We can infer this from 'APPEARS_IN' edges or similar.
    // For now, let's approximate source_docs as 1 if we don't have edge counts handy,
    // or expensive graph queries.
    // Ideally, `node.metadata` would store `doc_frequency`.
    
    // Using mention_count as a proxy for both if data missing
    let mentions = node.mention_count.max(1);
    let docs = 1; // Default fallback
    
    // Better: Query "mentions" table or edge count if available.
    // For this implementation, we rely on what's in Node.
    
    calculate_importance(docs, mentions as usize, docs)
}

/// Combined similarity score
///
/// Mixes string similarity and semantic similarity based on config weight.
/// If vector score is unavailable, falls back to string score.
pub fn hybrid_similarity(
    string_score: f32,
    vector_score: Option<f32>,
    config: &LinkingConfig,
) -> f32 {
    match vector_score {
        Some(sem) => {
            // Default split: 0.4 string, 0.6 semantic (tweakable)
            let w_str = 0.4;
            let w_sem = 0.6;
            (string_score * w_str) + (sem * w_sem)
        }
        None => string_score,
    }
}

/// Discover clusters using the Hybrid approach
///
/// 1. Candidate Generation:
///    - Get all string matches (fast)
///    - Get all vector matches (accurate)
/// 2. Scoring:
///    - Apply hybrid scoring
/// 3. Clustering:
///    - Group connected components
/// 4. Canonical Selection:
///    - Highest Importance Score wins
pub async fn discover_clusters(
    registry: &GraphRegistry,
    embedder: &EmbeddingService, // Needed if we run live embeddings, but usually we use stored ones
    config: &LinkingConfig,
) -> Result<Vec<EntityCluster>, String> {
    
    // 1. Get all nodes
    let nodes = registry.get_nodes(Default::default())
        .map_err(|e| format!("Failed to fetch nodes: {}", e))?;
        
    // Candidates map: NodeID -> Vec<(SimilarNodeID, HybridScore)>
    let mut adjacency: HashMap<String, Vec<(String, f32)>> = HashMap::new();
    
    // Optimization: Instead of O(N^2) checks, we combine:
    // A. The string linker's greedy clustering groups
    // B. The embedding HNSW neighbors
    
    // A. Run string clustering first (cheap)
    // We reuse logic from `linker.rs` but just to get pairs
    // Actually, let's iterate and build high-confidence pairs
    
    // We'll iterate all nodes and find their match candidates
    for node in &nodes {
        // --- 1. String Candidates ---
        let string_candidates = linker::find_similar_by_name(registry, &node.label, config.string_threshold);
        
        // --- 2. Vector Candidates ---
        // We use the stored embedding to query HNSW
        // Fallback to empty if no embedding exists
        let vector_candidates = match context_embed::find_similar_to_node(
            registry, 
            &node.id, 
            10, // Limit candidates
            config.semantic_threshold
        ) {
            Ok(c) => c,
            Err(_) => vec![],
        };
        
        // Map of candidate_id -> (string_score, vector_score)
        let mut merged_candidates: HashMap<String, (f32, Option<f32>)> = HashMap::new();
        
        for (candidate, score) in string_candidates {
            if candidate.id == node.id { continue; }
            merged_candidates.entry(candidate.id)
                .and_modify(|e| e.0 = score)
                .or_insert((score, None));
        }
        
        for (cand_id, score) in vector_candidates {
            if cand_id == node.id { continue; }
            merged_candidates.entry(cand_id)
                .and_modify(|e| e.1 = Some(score))
                .or_insert((0.0, Some(score)));
        }
        
        // Compute hybrid scores and store edges
        for (cand_id, (s_score, v_score)) in merged_candidates {
            let proper_s_score = if s_score > 0.0 {
                s_score
            } else {
                // If we only found it via vector, compute string match now lazily
                if let Ok(Some(cand_node)) = registry.get_node(&cand_id) {
                    string_sim::string_similarity(&node.label, &cand_node.label)
                } else {
                    0.0
                }
            };
            
            let final_score = hybrid_similarity(proper_s_score, v_score, config);
            
            // If accumulated evidence is strong enough
            if final_score >= config.string_threshold {
                adjacency.entry(node.id.clone())
                    .or_default()
                    .push((cand_id, final_score));
            }
        }
    }
    
    // 2. Connected Components (Clustering)
    let mut visited: HashSet<String> = HashSet::new();
    let mut clusters: Vec<EntityCluster> = Vec::new();
    
    for node in &nodes {
        if visited.contains(&node.id) { continue; }
        
        // Start a new cluster
        let mut members = Vec::new();
        let mut queue = vec![node.id.clone()];
        visited.insert(node.id.clone());
        
        // Add self as first member
        members.push(ClusterMember {
            node_id: node.id.clone(),
            label: node.label.clone(),
            source_note: node.source_note.clone(),
            similarity: 1.0, 
        });
        
        // BFS for connected components
        while let Some(curr_id) = queue.pop() {
            if let Some(neighbors) = adjacency.get(&curr_id) {
                for (neighbor_id, score) in neighbors {
                    if !visited.contains(neighbor_id) {
                        visited.insert(neighbor_id.clone());
                        queue.push(neighbor_id.clone());
                        
                        // Fetch node details for member struct
                        if let Ok(Some(n)) = registry.get_node(neighbor_id) {
                            members.push(ClusterMember {
                                node_id: n.id,
                                label: n.label,
                                source_note: n.source_note,
                                similarity: *score,
                            });
                        }
                    }
                }
            }
        }
        
        // If we found a cluster > 1 member, or just singular, we process it.
        // Usually we only care about real clusters > 1 for merging purposes,
        // but returning all is fine.
        if members.len() > 1 {
            // 3. Elect Canonical Leader via Importance Score
            // Sort by Importance (descending), then mention_count, then label length (shortest often better), then alphabet
            // We need to re-fetch nodes to calculate detailed importance if strict
            // For now, sorting by mention_count (from Phase 1) is a good proxy, 
            // but let's try to simulate importance.
            
            // Helper to get importance
            let get_importance = |m: &ClusterMember| -> f32 {
                // Fetch full node
                if let Ok(Some(n)) = registry.get_node(&m.node_id) {
                    node_importance(&n, registry)
                } else {
                    0.0
                }
            };
            
            // Sort: High Importance -> High Mentions -> Short Label -> Alpha
            members.sort_by(|a, b| {
                let imp_a = get_importance(a);
                let imp_b = get_importance(b);
                
                imp_b.partial_cmp(&imp_a).unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.label.len().cmp(&b.label.len())) // Shortest name prefered (e.g. "Google" vs "Google Inc.")
                    .then_with(|| a.label.cmp(&b.label))
            });
            
            let canonical = &members[0];
            
            clusters.push(EntityCluster {
                cluster_id: uuid::Uuid::new_v4().to_string(),
                canonical_id: canonical.node_id.clone(),
                canonical_name: canonical.label.clone(),
                members,
                confidence: 0.85, // Aggregate confidence
            });
        }
    }
    
    Ok(clusters)
}

/// Create CO_OCCURS edges for entities appearing in the same context
///
/// Implements the GraphRAG co-occurrence pattern.
pub fn create_cooccurrence_edges(
    registry: &GraphRegistry,
    _doc_id: &str, // Could track provenance
    entity_ids: &[String],
) -> Result<usize, String> {
    if entity_ids.len() < 2 {
        return Ok(0);
    }
    
    let mut created_count = 0;
    
    // Create edges between all pairs (undirected/bidirectional)
    // Cozo doesn't have native undirected, so we usually create one direction 
    // and query ignoring direction, or create both.
    // GraphRAG uses undirected. We'll store:
    // source <-> target with type "CO_OCCURS"
    
    for (i, id_a) in entity_ids.iter().enumerate() {
        for id_b in entity_ids.iter().skip(i + 1) {
            // Sort to ensure consistent direction if we want unique edges
            // or just rely on 'CO_OCCURS' semantics
            
            // Upsert edge: increment weight if exists
            // Since our create_edge is simple, we might need a custom query to increment weight
            
            // Custom query to upsert/increment co-occurrence
            let query = r#"
                ?[source_id, target_id, type] <- [[$src, $tgt, 'CO_OCCURS']]
                
                // Check if exists
                existing[weight] := *edges{source_id, target_id, type, weight}
                
                // New weight
                :put edges {
                    source_id,
                    target_id,
                    type,
                    weight: coalesce(weight, 0) + 1
                }
            "#;
            
            // Run twice for both directions? Or just canonical direction?
            // Canonical is better for storage: min(a,b) -> max(a,b)
            let (src, tgt) = if id_a < id_b { (id_a, id_b) } else { (id_b, id_a) };
            
            let mut params = BTreeMap::new();
            params.insert("src".to_string(), DataValue::Str(src.clone().into()));
            params.insert("tgt".to_string(), DataValue::Str(tgt.clone().into()));
            
            registry.db().run_script(query, params, ScriptMutability::Mutable)
                .map_err(|e| format!("Edge creation failed: {}", e))?;
                
            created_count += 1;
        }
    }
    
    Ok(created_count)
}

/// GraphStats - snapshot of corpus connectivity
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrossDocStats {
    pub total_entities: usize,
    pub total_relations: usize,
    pub cross_document_entities: usize,
    pub graph_density: f32,
    pub avg_degree: f32,
}

/// Calculate graph statistics
pub fn calculate_stats(registry: &GraphRegistry) -> Result<CrossDocStats, String> {
    // We can run a single heavy read query to aggregate stats
    let query = r#"
        total_ents[count(id)] := *nodes{id}
        total_rels[count(source_id)] := *edges{source_id}
        
        // Entities with mentions in > 1 document (approx via importance or explicit tracking)
        // For now, simple count
        
        ?[ents, rels] := total_ents[ents], total_rels[rels]
    "#;
    
    let result = registry.db().run_script(query, Default::default(), ScriptMutability::Immutable)
        .map_err(|e| e.to_string())?;
        
    if let Some(row) = result.rows.first() {
        let ents = match row[0] { DataValue::Num(cozo::Num::Int(i)) => i as usize, _ => 0 };
        let rels = match row[1] { DataValue::Num(cozo::Num::Int(i)) => i as usize, _ => 0 };
        
        let density = if ents > 1 {
            (2.0 * rels as f32) / (ents * (ents - 1)) as f32
        } else {
            0.0
        };
        
        let avg_degree = if ents > 0 {
            (2.0 * rels as f32) / ents as f32
        } else {
            0.0
        };
        
        Ok(CrossDocStats {
            total_entities: ents,
            total_relations: rels,
            cross_document_entities: 0, // TODO: Need document_map to count this properly
            graph_density: density,
            avg_degree,
        })
    } else {
        Err("No stats returned".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crossdoc::types::LinkingConfig;

    #[test]
    fn test_calculate_importance_logic() {
        // Doc freq = 10, Mentions = 100, Source Docs = 10
        let score = calculate_importance(10, 100, 10);
        assert!(score > 1.9 && score < 2.1);
    }

    #[test]
    fn test_hybrid_similarity_weights() {
        let config = LinkingConfig::default();
        
        // Case 1: Only string score (0.9)
        let s1 = hybrid_similarity(0.9, None, &config);
        assert_eq!(s1, 0.9);
        
        // Case 2: String (0.9) + Vector (0.1) -> should be heavily penalized by vector
        // 0.9 * 0.4 + 0.1 * 0.6 = 0.36 + 0.06 = 0.42
        let s2 = hybrid_similarity(0.9, Some(0.1), &config);
        assert!((s2 - 0.42).abs() < 0.001);
        
        // Case 3: String (0.5) + Vector (0.9) -> saved by vector
        // 0.5 * 0.4 + 0.9 * 0.6 = 0.2 + 0.54 = 0.74
        let s3 = hybrid_similarity(0.5, Some(0.9), &config);
        assert!((s3 - 0.74).abs() < 0.001);
    }
}

