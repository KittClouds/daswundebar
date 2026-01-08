//! Entropy calculations for BMX
//!
//! Implements term entropy computation and LRU caching for the
//! BMX (BM25 with Entropy) extension.

use std::collections::HashMap;

use super::config::F32;
use super::math::sigmoid;
use super::types::TokenMetadata;
use serde::{Deserialize, Serialize};

// =============================================================================
// Entropy Cache
// =============================================================================

/// LRU cache for computed entropy values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyCache {
    cache: HashMap<String, F32>,
    access_order: Vec<String>,
    max_size: usize,
}

impl EntropyCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size),
            access_order: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Get entropy value (compute if missing)
    pub fn get<K: std::hash::Hash + Eq + Clone>(
        &mut self,
        term: &str,
        token_index: &HashMap<String, HashMap<K, TokenMetadata>>,
    ) -> F32 {
        if let Some(&entropy) = self.cache.get(term) {
            self.mark_accessed(term);
            return entropy;
        }

        let entropy = self.compute_entropy(term, token_index);
        self.set(term.to_string(), entropy);
        entropy
    }

    /// Check if term is in cache
    pub fn has(&self, term: &str) -> bool {
        self.cache.contains_key(term)
    }

    /// Get cached value without computing
    pub fn get_cached(&self, term: &str) -> Option<F32> {
        self.cache.get(term).copied()
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> EntropyCacheStats {
        EntropyCacheStats {
            size: self.cache.len(),
            memory_mb: (self.cache.len() * 48) as f32 / (1024.0 * 1024.0),
        }
    }

    /// Compute entropy for a single term (BMX Equation 5)
    fn compute_entropy<K: std::hash::Hash + Eq>(
        &self,
        term: &str,
        token_index: &HashMap<String, HashMap<K, TokenMetadata>>,
    ) -> F32 {
        let term_docs = match token_index.get(term) {
            Some(docs) => docs,
            None => return 0.0,
        };

        let mut raw_entropy = 0.0;

        for metadata in term_docs.values() {
            let mut total_tf: u32 = 0;
            for occurrence in metadata.field_occurrences.values() {
                total_tf += occurrence.tf;
            }

            let capped_tf = total_tf.min(10) as f32;
            let pj = sigmoid(capped_tf);

            if pj > 1e-6 && pj < 0.999999 {
                raw_entropy -= pj * pj.ln();
            }
        }

        raw_entropy
    }

    /// Set value with LRU eviction
    fn set(&mut self, term: String, entropy: F32) {
        if self.cache.len() >= self.max_size {
            if let Some(evict_key) = self.access_order.first().cloned() {
                self.cache.remove(&evict_key);
                self.access_order.remove(0);
            }
        }

        self.cache.insert(term.clone(), entropy);
        self.access_order.push(term);
    }

    /// Mark term as recently accessed
    fn mark_accessed(&mut self, term: &str) {
        if let Some(idx) = self.access_order.iter().position(|t| t == term) {
            let t = self.access_order.remove(idx);
            self.access_order.push(t);
        }
    }
}

impl Default for EntropyCache {
    fn default() -> Self {
        Self::new(1000)
    }
}

// =============================================================================
// Entropy Cache Stats
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyCacheStats {
    pub size: usize,
    pub memory_mb: f32,
}

// =============================================================================
// Query Entropy Calculation
// =============================================================================

/// Calculate query-level entropy statistics (BMX Equations 5-6)
pub fn calculate_query_entropy_stats<K: std::hash::Hash + Eq + Clone>(
    query: &[String],
    cache: &mut EntropyCache,
    token_index: &HashMap<String, HashMap<K, TokenMetadata>>,
) -> QueryEntropyStats {
    let mut normalized_entropies = HashMap::new();
    let mut max_raw_entropy: F32 = 0.0;

    for term in query {
        let raw_entropy = cache.get(term, token_index);
        max_raw_entropy = max_raw_entropy.max(raw_entropy);
    }

    let normalization_factor = max_raw_entropy.max(1e-9);

    let mut sum_normalized: F32 = 0.0;
    for term in query {
        let raw_entropy = cache.get(term, token_index);
        let normalized = raw_entropy / normalization_factor;
        normalized_entropies.insert(term.clone(), normalized);
        sum_normalized += normalized;
    }

    let avg_entropy = if !query.is_empty() {
        sum_normalized / query.len() as f32
    } else {
        0.0
    };

    QueryEntropyStats {
        normalized_entropies,
        avg_entropy,
        sum_normalized_entropies: sum_normalized,
        max_raw_entropy,
    }
}

/// Query-level entropy statistics
#[derive(Debug, Clone)]
pub struct QueryEntropyStats {
    pub normalized_entropies: HashMap<String, F32>,
    pub avg_entropy: F32,
    pub sum_normalized_entropies: F32,
    pub max_raw_entropy: F32,
}

impl Default for QueryEntropyStats {
    fn default() -> Self {
        Self {
            normalized_entropies: HashMap::new(),
            avg_entropy: 0.0,
            sum_normalized_entropies: 0.0,
            max_raw_entropy: 0.0,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resorank::types::FieldOccurrence;

    fn create_test_token_index() -> HashMap<String, HashMap<String, TokenMetadata>> {
        let mut index: HashMap<String, HashMap<String, TokenMetadata>> = HashMap::new();

        let mut rare_docs = HashMap::new();
        let mut rare_meta = TokenMetadata::new(1);
        rare_meta.field_occurrences.insert(
            0,
            FieldOccurrence {
                tf: 1,
                field_length: 100,
            },
        );
        rare_docs.insert("doc1".to_string(), rare_meta);
        index.insert("rare".to_string(), rare_docs);

        let mut common_docs = HashMap::new();
        for i in 1..=3 {
            let mut common_meta = TokenMetadata::new(3);
            common_meta.field_occurrences.insert(
                0,
                FieldOccurrence {
                    tf: 2,
                    field_length: 100,
                },
            );
            common_docs.insert(format!("doc{}", i), common_meta);
        }
        index.insert("common".to_string(), common_docs);

        index
    }

    #[test]
    fn test_entropy_cache_basic() {
        let mut cache = EntropyCache::new(100);
        let index = create_test_token_index();

        let e1 = cache.get("rare", &index);
        assert!(e1 >= 0.0);

        let e2 = cache.get("rare", &index);
        assert_eq!(e1, e2);

        assert!(cache.has("rare"));
        assert!(!cache.has("nonexistent"));
    }

    #[test]
    fn test_entropy_cache_lru_eviction() {
        let mut cache = EntropyCache::new(2);
        let index = create_test_token_index();

        cache.get("rare", &index);
        cache.get("common", &index);
        
        assert!(cache.has("rare"));
        assert!(cache.has("common"));

        cache.set("new_term".to_string(), 1.0);
        
        assert!(!cache.has("rare"));
        assert!(cache.has("common"));
        assert!(cache.has("new_term"));
    }

    #[test]
    fn test_query_entropy_stats() {
        let mut cache = EntropyCache::new(100);
        let index = create_test_token_index();

        let query = vec!["rare".to_string(), "common".to_string()];
        let stats = calculate_query_entropy_stats(&query, &mut cache, &index);

        assert!(stats.avg_entropy >= 0.0);
        assert!(stats.avg_entropy <= 1.0);
        assert_eq!(stats.normalized_entropies.len(), 2);
    }
}
