//! Context Embedding for Entity Linking (Phase 2)
//!
//! Generates rich context strings from CST projections and graph neighborhood,
//! embeds them using EmbeddingService, and stores in `node_vectors` for HNSW search.

use crate::graph::GraphRegistry;
use crate::graph::{Node, Direction};
use crate::rag::embeddings;
use crate::reality::Projection;
use cozo::{DataValue, ScriptMutability};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Build a rich context string for an entity based on its graph neighborhood
/// and CST projections.
///
/// Format:
/// "Entity: [Name]. Type: [Kind]. Also known as: [aliases]. Related to: [neighbors]."
pub fn build_cst_context(
    node: &Node,
    _projections: &[Projection], // Future: Use SVO triples/quads for richer context
    neighbors: Option<&[String]>,
) -> String {
    let mut parts = Vec::new();
    
    parts.push(format!("Entity: {}", node.label));
    parts.push(format!("Type: {}", node.kind.as_str()));
    
    if let Some(subtype) = &node.subtype {
        if !subtype.is_empty() {
            parts.push(format!("Subtype: {}", subtype));
        }
    }
    
    if !node.aliases.is_empty() {
        parts.push(format!("Also known as: {}", node.aliases.join(", ")));
    }
    
    if let Some(neighbor_labels) = neighbors {
        if !neighbor_labels.is_empty() {
            parts.push(format!("Related to: {}", neighbor_labels.join(", ")));
        }
    }
    
    parts.join(". ")
}

/// Build context string including 1-hop neighbors from the graph
pub fn build_context_with_neighbors(
    registry: &GraphRegistry,
    node: &Node,
    projections: &[Projection],
) -> String {
    // Get 1-hop neighbors
    let neighbor_labels: Vec<String> = match registry.get_edges(&node.id, Direction::Both) {
        Ok(edges) => {
            edges.iter()
                .filter_map(|edge| {
                    let target_id = if edge.source_id == node.id {
                        &edge.target_id
                    } else {
                        &edge.source_id
                    };
                    registry.get_node(target_id).ok().flatten().map(|n| n.label)
                })
                .take(10) // Limit to 10 neighbors to avoid embedding bloat
                .collect()
        }
        Err(_) => vec![],
    };
    
    build_cst_context(node, projections, Some(&neighbor_labels))
}

/// Embed a node's context and store it in the `node_vectors` relation
pub fn embed_and_store(
    registry: &GraphRegistry,
    node: &Node,
    context: &str,
) -> Result<(), String> {
    // 1. Generate embedding using global embedder
    let texts = vec![context.to_string()];
    let embeddings = embeddings::embed_texts(&texts)?;
    
    let vector = embeddings.first()
        .ok_or("No embedding generated")?;
    
    // 2. Store in CozoDB
    let query = r#"
        ?[node_id, model, vector, created_at] <- [[$node_id, $model, $vector, $created_at]]
        :put node_vectors {node_id => model, vector, created_at}
    "#;
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    
    // Build vector as DataValue::Vec of floats  
    let vector_data: Vec<DataValue> = vector.iter()
        .map(|&x| DataValue::from(x as f64))
        .collect();
        
    let mut params = BTreeMap::new();
    params.insert("node_id".to_string(), DataValue::Str(node.id.clone().into()));
    params.insert("model".to_string(), DataValue::Str("bge-small-en-v1.5".into()));
    params.insert("vector".to_string(), DataValue::List(vector_data));
    params.insert("created_at".to_string(), DataValue::from(now));
    
    registry.db().run_script(query, params, ScriptMutability::Mutable)
        .map_err(|e| format!("CozoDB error storing vector: {}", e))?;
        
    log::debug!("[ContextEmbedder] Stored embedding for node {} ({})", node.label, node.id);
    
    Ok(())
}

/// Check if a node already has an embedding stored
pub fn has_embedding(registry: &GraphRegistry, node_id: &str) -> bool {
    let query = r#"
        ?[node_id] := *node_vectors{node_id}, node_id = $node_id
    "#;
    
    let mut params = BTreeMap::new();
    params.insert("node_id".to_string(), DataValue::Str(node_id.into()));
    
    match registry.db().run_script(query, params, ScriptMutability::Immutable) {
        Ok(result) => !result.rows.is_empty(),
        Err(_) => false,
    }
}

// Helper to extract f32 from DataValue
fn extract_f32_from_datavalue(v: &DataValue) -> Option<f32> {
    match v {
        DataValue::Num(cozo::Num::Float(f)) => Some(*f as f32),
        DataValue::Num(cozo::Num::Int(i)) => Some(*i as f32),
        _ => None,
    }
}

/// Find nodes with similar semantic context using HNSW index
/// Returns vec of (node_id, similarity_score)
pub fn find_similar_by_embedding(
    registry: &GraphRegistry,
    query_text: &str,
    limit: usize,
    threshold: f32,
) -> Result<Vec<(String, f32)>, String> {
    // 1. Embed query using global embedder
    let texts = vec![query_text.to_string()];
    let embeddings = embeddings::embed_texts(&texts)?;
    
    let vector = embeddings.first()
        .ok_or("No embedding generated")?;
    
    // 2. Query CozoDB HNSW index
    // Cozo HNSW search syntax: ~relation:index_name{ ... | query_vector: $vec, k: $k }
    let query = r#"
        ?[node_id, distance] := ~node_vectors:semantic_idx{node_id | query: $query_vector, k: $k, ef: 50}
    "#;
    
    let vector_data: Vec<DataValue> = vector.iter()
        .map(|&x| DataValue::from(x as f64))
        .collect();
    
    let mut params = BTreeMap::new();
    params.insert("query_vector".to_string(), DataValue::List(vector_data));
    params.insert("k".to_string(), DataValue::from(limit as i64));
    
    let result = registry.db().run_script(query, params, ScriptMutability::Immutable)
        .map_err(|e| format!("CozoDB HNSW error: {}", e))?;
        
    let mut matches = Vec::new();
    for row in &result.rows {
        let node_id = match &row[0] {
            DataValue::Str(s) => s.to_string(),
            _ => continue,
        };
        let distance = extract_f32_from_datavalue(&row[1]).unwrap_or(0.0);
        
        // Cosine distance: 0 = identical, 2 = opposite
        // Convert to similarity: 1 - distance
        let similarity = 1.0 - distance;
        
        if similarity >= threshold {
            matches.push((node_id, similarity));
        }
    }
    
    // Sort by similarity descending
    matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    Ok(matches)
}

/// Find similar nodes to an existing node by its stored embedding
pub fn find_similar_to_node(
    registry: &GraphRegistry,
    node_id: &str,
    limit: usize,
    threshold: f32,
) -> Result<Vec<(String, f32)>, String> {
    // 1. Get the node to build a query from its label
    let node = registry.get_node(node_id)
        .map_err(|e| format!("Failed to get node: {}", e))?
        .ok_or_else(|| format!("Node not found: {}", node_id))?;
    
    // Use the node's label as query text (simpler approach)
    let query_text = build_cst_context(&node, &[], None);
    
    // 2. Find similar nodes using the context
    let mut results = find_similar_by_embedding(registry, &query_text, limit + 1, threshold)?;
    
    // Remove self from results
    results.retain(|(id, _)| id != node_id);
    
    // Limit to requested amount
    results.truncate(limit);
    
    Ok(results)
}

/// Embed all nodes that don't have embeddings yet
pub fn embed_all_missing(registry: &GraphRegistry) -> Result<(usize, usize), String> {
    if !embeddings::is_embedder_ready() {
        return Err("Embedder not initialized. Call init_embedder first.".to_string());
    }
    
    // Get all nodes
    let nodes = registry.get_nodes(Default::default())
        .map_err(|e| format!("Failed to get nodes: {}", e))?;
    
    let mut embedded = 0;
    let mut skipped = 0;
    
    for node in &nodes {
        if has_embedding(registry, &node.id) {
            skipped += 1;
            continue;
        }
        
        let context = build_context_with_neighbors(registry, node, &[]);
        match embed_and_store(registry, node, &context) {
            Ok(_) => embedded += 1,
            Err(e) => {
                log::warn!("[ContextEmbedder] Failed to embed {}: {}", node.label, e);
            }
        }
    }
    
    log::info!("[ContextEmbedder] Embedded {} nodes, skipped {} (already had embeddings)", embedded, skipped);
    
    Ok((embedded, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{NodeKind, CreatedBy};

    #[test]
    fn test_build_cst_context_basic() {
        let node = Node {
            id: "test123".to_string(),
            label: "Gandalf".to_string(),
            normalized: "gandalf".to_string(),
            kind: NodeKind::Character,
            subtype: Some("Wizard".to_string()),
            source_note: "note1".to_string(),
            created_at: 0.0,
            created_by: CreatedBy::User,
            mention_count: 5,
            aliases: vec!["Mithrandir".to_string(), "Grey Pilgrim".to_string()],
            metadata: None,
        };
        
        let context = build_cst_context(&node, &[], None);
        
        assert!(context.contains("Entity: Gandalf"));
        assert!(context.contains("Type: CHARACTER"));
        assert!(context.contains("Subtype: Wizard"));
        assert!(context.contains("Also known as: Mithrandir, Grey Pilgrim"));
    }
    
    #[test]
    fn test_build_cst_context_with_neighbors() {
        let node = Node {
            id: "test123".to_string(),
            label: "Frodo".to_string(),
            normalized: "frodo".to_string(),
            kind: NodeKind::Character,
            subtype: None,
            source_note: "note1".to_string(),
            created_at: 0.0,
            created_by: CreatedBy::User,
            mention_count: 10,
            aliases: vec![],
            metadata: None,
        };
        
        let neighbors = vec!["Sam".to_string(), "Gandalf".to_string(), "The Ring".to_string()];
        let context = build_cst_context(&node, &[], Some(&neighbors));
        
        assert!(context.contains("Entity: Frodo"));
        assert!(context.contains("Related to: Sam, Gandalf, The Ring"));
    }
}
