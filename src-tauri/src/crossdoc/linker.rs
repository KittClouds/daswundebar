//! Greedy Entity Clustering Logic
//!
//! Implements a greedy clustering algorithm to group similar entities.
//! Based on graphrag-rs patterns.

use crate::graph::GraphRegistry;
use crate::graph::Node;
use super::types::{EntityCluster, ClusterMember, LinkingConfig, LinkingStats};
use super::string_sim;

/// Find all nodes similar to a given label in the registry
pub fn find_similar_by_name(
    registry: &GraphRegistry,
    label: &str,
    threshold: f32,
) -> Vec<(Node, f32)> {
    let mut results = Vec::new();
    
    // This is expensive: O(N) scan. 
    // In production, we should filter candidates by some index (e.g., length, first char)
    // or use the implicit cortex (FST) for faster lookup if possible.
    // For now, we rely on GraphRegistry::get_nodes() filtering or just fetch all?
    // GraphRegistry doesn't expose a cheap filtered iterator yet, so we get all nodes.
    // Optimization: Filter by `kind` if provided (not yet in signature).
    
    // Using unwrap for now, handled error would be better
    let all_nodes = match registry.get_nodes(Default::default()) {
        Ok(nodes) => nodes,
        Err(_) => return Vec::new(),
    };

    for node in all_nodes {
        let score = string_sim::string_similarity(label, &node.label);
        if score >= threshold {
            results.push((node, score));
        }
    }
    
    // Sort by score descending
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    results
}

/// Cluster all entities in the registry based on string similarity
pub fn cluster_by_string_similarity(
    registry: &GraphRegistry,
    config: &LinkingConfig,
) -> (Vec<EntityCluster>, LinkingStats) {
    let mut clusters = Vec::new();
    let mut stats = LinkingStats::default();

    // 1. Fetch all nodes
    // Using unwrap is risky, catch error properly in real impl
    let mut nodes = match registry.get_nodes(Default::default()) {
        Ok(n) => n,
        Err(e) => {
            log::error!("Failed to fetch nodes for clustering: {}", e);
            return (vec![], stats);
        }
    };
    
    stats.entities_processed = nodes.len();
    
    // 2. Greedy clustering
    // Sort nodes by mention_count descending (canonical entities usually appear more)
    // Tie-breaker: alphabetical by label for deterministic ordering
    nodes.sort_by(|a, b| {
        b.mention_count.cmp(&a.mention_count)
            .then_with(|| a.label.cmp(&b.label))
    });
    
    let mut processed_indices = vec![false; nodes.len()];
    let mut cluster_counter = 0;
    
    for i in 0..nodes.len() {
        if processed_indices[i] {
            continue;
        }
        
        processed_indices[i] = true;
        let seed = &nodes[i];
        
        let mut members = Vec::new();
        
        // Add seed as first member
        members.push(ClusterMember {
            node_id: seed.id.clone(),
            label: seed.label.clone(),
            source_note: seed.source_note.clone(),
            similarity: 1.0,
        });
        
        // Find matches in remaining unprocessed nodes
        for j in (i + 1)..nodes.len() {
            if processed_indices[j] {
                continue;
            }
            
            let candidate = &nodes[j];
            
            // Only cluster same kinds? (Optional, graphrag-rs does check types)
            // if seed.kind != candidate.kind { continue; }
            
            let score = string_sim::string_similarity(&seed.label, &candidate.label);
            
            if score >= config.string_threshold {
                processed_indices[j] = true;
                members.push(ClusterMember {
                    node_id: candidate.id.clone(),
                    label: candidate.label.clone(),
                    source_note: candidate.source_note.clone(),
                    similarity: score,
                });
                
                if score == 1.0 {
                    stats.exact_matches += 1;
                } else {
                    stats.fuzzy_matches += 1;
                }
            }
        }
        
        // Create cluster if we have members (even singletons form a cluster effectively)
        if !members.is_empty() {
             cluster_counter += 1;
             stats.clusters_found += 1;
             
             clusters.push(EntityCluster {
                 cluster_id: format!("cluster_{}", cluster_counter),
                 canonical_id: seed.id.clone(),
                 canonical_name: seed.label.clone(),
                 members,
                 confidence: 1.0, // Base confidence
             });
        }
    }
    
    // Populate derived stats
    stats.merges_suggested = clusters.iter().map(|c| if c.members.len() > 1 { c.members.len() - 1 } else { 0 }).sum();
    stats.avg_confidence = if !clusters.is_empty() {
        clusters.iter().map(|c| c.confidence).sum::<f32>() / clusters.len() as f32
    } else {
        0.0
    };
    
    (clusters, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GraphRegistry;
    use crate::graph::{NodeInput, NodeKind, CreatedBy};

    fn create_test_registry() -> GraphRegistry {
        let registry = GraphRegistry::in_memory().unwrap();
        registry
    }

    #[test]
    fn test_clustering() {
        let registry = create_test_registry();
        
        // Add test entities
        let inputs = vec![
            ("Gandalf", "note1"),
            ("Gandolf", "note2"), // Typo
            ("Aragorn", "note3"),
            ("Strider", "note3"), // Different name entirely
            ("Frodo Baggins", "note4"),
        ];
        
        for (label, note) in inputs {
            registry.register_node(NodeInput {
                label: label.to_string(),
                kind: NodeKind::Character,
                source_note: note.to_string(),
                subtype: None,
                aliases: vec![],
                created_by: CreatedBy::User,
                metadata: None,
            }).unwrap();
        }
        
        let config = LinkingConfig::default();
        let (clusters, stats) = cluster_by_string_similarity(&registry, &config);
        
        assert_eq!(stats.entities_processed, 5);
        
        // Expecting:
        // 1. Gandalf + Gandolf (Cluster)
        // 2. Aragorn (Cluster)
        // 3. Strider (Cluster)
        // 4. Frodo Baggins (Cluster)
        // Total 4 clusters
        assert_eq!(clusters.len(), 4);
        
        // Find Gandalf cluster
        let gandalf_cluster = clusters.iter().find(|c| c.canonical_name == "Gandalf").unwrap();
        assert_eq!(gandalf_cluster.members.len(), 2); // Gandalf + Gandolf
    }
}
