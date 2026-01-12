use serde::{Deserialize, Serialize};

/// Configuration for entity linking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkingConfig {
    /// Minimum similarity score to be considered a match (0.0 to 1.0)
    /// Default: 0.85
    pub string_threshold: f32,
    
    /// Minimum semantic similarity matching threshold
    /// Default: 0.75
    pub semantic_threshold: f32,
    
    /// Whether to ignore case when comparing strings
    /// Default: true
    pub case_insensitive: bool,
}

impl Default for LinkingConfig {
    fn default() -> Self {
        Self {
            string_threshold: 0.85,
            semantic_threshold: 0.75,
            case_insensitive: true,
        }
    }
}

/// A member of an entity cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    pub node_id: String,
    pub label: String,
    pub source_note: String,
    /// Similarity score to the canonical entity (0.0 to 1.0)
    pub similarity: f32,
}

/// A cluster of similar entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCluster {
    pub cluster_id: String,
    pub canonical_id: String,
    pub canonical_name: String,
    pub members: Vec<ClusterMember>,
    pub confidence: f32,
}

/// Statistics for a linking run
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LinkingStats {
    pub entities_processed: usize,
    pub clusters_found: usize,
    pub merges_suggested: usize,
    pub avg_confidence: f32,
    pub processing_time_ms: u64,
    // Legacy fields - kept for compatibility but mapped
    pub exact_matches: usize,
    pub fuzzy_matches: usize,
}
