//! Configuration types and defaults for ResoRank
//!
//! Ported from kittcore WASM to Tauri native.
//! WASM bindings removed - pure Rust.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type aliases for clarity
pub type F32 = f32;
pub type U32 = u32;
pub type Usize = usize;
pub type FieldId = u32;

// =============================================================================
// Field Parameters
// =============================================================================

/// Parameters for individual fields in BM25F scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldParams {
    /// Field weight (default: 1.0 for content, 2.0 for title)
    pub weight: F32,
    /// Length normalization parameter b (default: 0.75)
    pub b: F32,
}

impl FieldParams {
    pub fn new(weight: F32, b: F32) -> Self {
        Self { weight, b }
    }
}

impl Default for FieldParams {
    fn default() -> Self {
        Self {
            weight: 1.0,
            b: 0.75,
        }
    }
}

// =============================================================================
// Main Configuration
// =============================================================================

/// ResoRank configuration with BM25F + Proximity + BMX parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResoRankConfig {
    /// BM25 k1 parameter (term saturation). Default: 1.2
    pub k1: F32,
    /// Strength of proximity boosting. Default: 0.5
    pub proximity_alpha: F32,
    /// Maximum number of segments for proximity masks (max 32). Default: 16
    pub max_segments: U32,
    /// Decay factor for document length in proximity calc. Default: 0.5
    pub proximity_decay_lambda: F32,
    /// Configuration for individual fields (id -> params)
    pub field_params: HashMap<FieldId, FieldParams>,
    /// IDF scaling factor for proximity weighting. Default: 5.0
    pub idf_proximity_scale: F32,
    /// Enable exact phrase detection boost. Default: true
    pub enable_phrase_boost: bool,
    /// Multiplier for phrase matches. Default: 1.5
    pub phrase_boost_multiplier: F32,

    // ===== BMX PARAMETERS =====
    /// Enable BMX entropy weighting in denominator. Default: false
    pub enable_bmx_entropy: bool,
    /// Enable BMX entropy-weighted similarity boost. Default: false
    pub enable_bmx_similarity: bool,
    /// Use adaptive alpha parameter instead of k1. Default: false
    pub use_adaptive_alpha: bool,
    /// Weight for entropy in denominator (gamma). If None, auto-calculated as alpha/2.
    pub entropy_denom_weight: Option<F32>,
}

impl Default for ResoRankConfig {
    fn default() -> Self {
        let mut field_params = HashMap::new();
        field_params.insert(0, FieldParams { weight: 2.0, b: 0.75 }); // Title
        field_params.insert(1, FieldParams { weight: 1.0, b: 0.75 }); // Content

        Self {
            k1: 1.2,
            proximity_alpha: 0.5,
            max_segments: 16,
            proximity_decay_lambda: 0.5,
            field_params,
            idf_proximity_scale: 5.0,
            enable_phrase_boost: true,
            phrase_boost_multiplier: 1.5,
            enable_bmx_entropy: false,
            enable_bmx_similarity: false,
            use_adaptive_alpha: false,
            entropy_denom_weight: None,
        }
    }
}

impl ResoRankConfig {
    /// Production-optimized configuration
    pub fn production() -> Self {
        Self::default()
    }

    /// Full BMX integration preset
    pub fn bmx() -> Self {
        Self {
            enable_bmx_entropy: true,
            enable_bmx_similarity: true,
            use_adaptive_alpha: true,
            entropy_denom_weight: None,
            ..Self::default()
        }
    }

    /// BMX with entropy only (conservative adoption)
    pub fn bmx_entropy_only() -> Self {
        Self {
            enable_bmx_entropy: true,
            use_adaptive_alpha: true,
            enable_bmx_similarity: false,
            ..Self::default()
        }
    }

    /// Latency-optimized configuration (minimal features)
    pub fn latency() -> Self {
        Self {
            enable_phrase_boost: false,
            enable_bmx_entropy: false,
            enable_bmx_similarity: false,
            use_adaptive_alpha: false,
            ..Self::default()
        }
    }
}

// =============================================================================
// Corpus Statistics
// =============================================================================

/// Corpus-level statistics for BM25F scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusStatistics {
    /// Total number of documents in corpus
    pub total_documents: Usize,
    /// Average field lengths (field_id -> avg_length)
    pub average_field_lengths: HashMap<FieldId, F32>,
    /// Average document length across all fields
    pub average_document_length: F32,
}

impl Default for CorpusStatistics {
    fn default() -> Self {
        let mut average_field_lengths = HashMap::new();
        average_field_lengths.insert(0, 10.0);  // Title
        average_field_lengths.insert(1, 500.0); // Content

        Self {
            total_documents: 0,
            average_field_lengths,
            average_document_length: 510.0,
        }
    }
}

// =============================================================================
// Corpus Size Thresholds
// =============================================================================

/// Corpus size classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusSize {
    Tiny,   // <= 100 docs
    Small,  // <= 1,000 docs
    Medium, // <= 10,000 docs
    Large,  // <= 100,000 docs
    XLarge, // > 100,000 docs
}

impl CorpusSize {
    /// Classify corpus by document count
    pub fn from_count(count: usize) -> Self {
        match count {
            0..=100 => CorpusSize::Tiny,
            101..=1_000 => CorpusSize::Small,
            1_001..=10_000 => CorpusSize::Medium,
            10_001..=100_000 => CorpusSize::Large,
            _ => CorpusSize::XLarge,
        }
    }

    /// Estimated max QPS for this corpus size
    pub fn max_qps(&self) -> u32 {
        match self {
            CorpusSize::Tiny => 178_000,
            CorpusSize::Small => 8_000,
            CorpusSize::Medium => 450,
            CorpusSize::Large => 45,
            CorpusSize::XLarge => 10,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ResoRankConfig::default();
        assert_eq!(config.k1, 1.2);
        assert_eq!(config.max_segments, 16);
        assert!(config.field_params.contains_key(&0));
        assert!(config.field_params.contains_key(&1));
    }

    #[test]
    fn test_field_params() {
        let params = FieldParams::new(2.5, 0.8);
        assert_eq!(params.weight, 2.5);
        assert_eq!(params.b, 0.8);
    }

    #[test]
    fn test_corpus_size_classification() {
        assert_eq!(CorpusSize::from_count(50), CorpusSize::Tiny);
        assert_eq!(CorpusSize::from_count(500), CorpusSize::Small);
        assert_eq!(CorpusSize::from_count(5_000), CorpusSize::Medium);
    }

    #[test]
    fn test_corpus_stats_default() {
        let stats = CorpusStatistics::default();
        assert_eq!(stats.total_documents, 0);
        assert!(stats.average_field_lengths.contains_key(&0));
    }
}
