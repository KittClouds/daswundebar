//! Cross-Document Entity Linking Commands
//!
//! Exposes functionality for finding duplicate entities, merging them, and
//! managing the entity linking pipeline.

use tauri::{AppHandle, State, command};
use crate::graph::GraphRegistry;
use crate::rag::embeddings::EmbeddingService;
use crate::crossdoc::types::{LinkingConfig, EntityCluster, LinkingStats};
use crate::crossdoc::{linker, hybrid, context_embed};
use crate::crossdoc::string_sim;

/// Find potential duplicate entities
///
/// Uses the configured strategy (String or Hybrid) to discover clusters.
#[command]
pub async fn crossdoc_find_duplicates(
    registry: State<'_, GraphRegistry>,
    embedder: State<'_, EmbeddingService>,
    config: LinkingConfig,
) -> Result<(Vec<EntityCluster>, LinkingStats), String> {
    log::info!("[CrossDoc] Starting duplicate detection with config: {:?}", config);
    
    // If we only want string similarity, use the simpler linker
    if config.semantic_threshold >= 1.0 {
        // High threshold means we effectively disable semantic
        let (clusters, stats) = linker::cluster_by_string_similarity(&registry, &config);
        return Ok((clusters, stats));
    }
    
    // Use hybrid approach
    match hybrid::discover_clusters(&registry, &embedder, &config).await {
        Ok(clusters) => {
            let stats = LinkingStats {
                entities_processed: 0, // TODO: Count total entities
                clusters_found: clusters.len(),
                merges_suggested: clusters.iter().map(|c| c.members.len() - 1).sum(),
                avg_confidence: if clusters.is_empty() { 0.0 } else {
                    clusters.iter().map(|c| c.confidence).sum::<f32>() / clusters.len() as f32
                },
                processing_time_ms: 0,
                exact_matches: 0,
                fuzzy_matches: 0,
            };
            Ok((clusters, stats))
        },
        Err(e) => Err(format!("Hybrid linker error: {}", e)),
    }
}

/// Find similar entities to a given text query
/// 
/// Returns a list of similar entities with their scores.
#[command]
pub async fn crossdoc_find_similar(
    registry: State<'_, GraphRegistry>,
    query: String,
    limit: usize,
) -> Result<Vec<(crate::graph::Node, f32)>, String> {
    // 1. String similarity search
    let string_hits = linker::find_similar_by_name(&registry, &query, 0.6);
    
    // 2. Semantic search (Phase 2)
    // We assume context_embed has a query function exposed
    // This requires embedding the query text on the fly
    let semantic_hits = context_embed::find_similar_by_embedding(&registry, &query, limit, 0.7)
        .unwrap_or_default();
        
    // Merge hits
    // Map of NodeID -> (Node, max_score)
    use std::collections::HashMap;
    let mut hits: HashMap<String, (crate::graph::Node, f32)> = HashMap::new();
    
    for (node, score) in string_hits {
        hits.insert(node.id.clone(), (node, score));
    }
    
    for (node_id, score) in semantic_hits {
        if let Some((_, existing_score)) = hits.get(&node_id) {
            if score > *existing_score {
                if let Ok(Some(node)) = registry.get_node(&node_id) {
                    hits.insert(node_id, (node, score));
                }
            }
        } else {
            if let Ok(Some(node)) = registry.get_node(&node_id) {
                hits.insert(node_id, (node, score));
            }
        }
    }
    
    // Sort by score
    let mut results: Vec<_> = hits.into_values().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    
    Ok(results)
}

/// Calculate corpus statistics
#[command]
pub fn crossdoc_stats(
    registry: State<'_, GraphRegistry>,
) -> Result<hybrid::CrossDocStats, String> {
    hybrid::calculate_stats(&registry)
}

/// Generate and store embeddings for all entities
///
/// This is a maintenance task that should be run periodically.
#[command]
pub async fn crossdoc_embed_all_missing(
    registry: State<'_, GraphRegistry>,
    _embedder: State<'_, EmbeddingService>, // Ensure embedder is initialized
) -> Result<(usize, usize), String> {
    context_embed::embed_all_missing(&registry)
}
