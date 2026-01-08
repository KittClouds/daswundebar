//! Core data structures for ResoRank
//!
//! Ported from kittcore WASM to Tauri native.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::config::{FieldId, F32, U32, Usize};

// =============================================================================
// Token Metadata
// =============================================================================

/// Field occurrence data within a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldOccurrence {
    /// Term frequency in this field
    pub tf: U32,
    /// Length of this field (in tokens)
    pub field_length: U32,
}

/// Metadata for a token in a specific document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    /// Occurrences by field (field_id -> occurrence data)
    pub field_occurrences: HashMap<FieldId, FieldOccurrence>,
    /// Bitmask indicating which segments contain this token
    pub segment_mask: U32,
    /// Number of documents in corpus containing this term
    pub corpus_doc_frequency: Usize,
}

impl TokenMetadata {
    pub fn new(corpus_doc_frequency: Usize) -> Self {
        Self {
            field_occurrences: HashMap::new(),
            segment_mask: 0,
            corpus_doc_frequency,
        }
    }

    /// Add an occurrence in a specific field
    pub fn add_field_occurrence(&mut self, field_id: FieldId, tf: U32, field_length: U32) {
        self.field_occurrences.insert(field_id, FieldOccurrence { tf, field_length });
    }

    /// Set segment mask (bitmap of segments where this token appears)
    pub fn set_segment_mask(&mut self, mask: U32) {
        self.segment_mask = mask;
    }
}

// =============================================================================
// Document Metadata
// =============================================================================

/// Metadata for an indexed document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Field lengths (field_id -> length in tokens)
    pub field_lengths: HashMap<FieldId, U32>,
    /// Total token count across all fields
    pub total_token_count: U32,
}

impl DocumentMetadata {
    pub fn new() -> Self {
        Self {
            field_lengths: HashMap::new(),
            total_token_count: 0,
        }
    }

    /// Set the length of a specific field
    pub fn set_field_length(&mut self, field_id: FieldId, length: U32) {
        self.field_lengths.insert(field_id, length);
        self.recalculate_total();
    }

    fn recalculate_total(&mut self) {
        self.total_token_count = self.field_lengths.values().sum();
    }
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Internal Accumulators
// =============================================================================

/// Accumulator for scoring a document
#[derive(Debug, Clone)]
pub struct DocumentAccumulator {
    /// Accumulated BM25 score
    pub bm25_score: F32,
    /// Segment masks for each query term
    pub term_masks: Vec<U32>,
    /// IDF values for each query term
    pub term_idfs: Vec<F32>,
    /// Field-specific masks (field_id -> list of masks per term)
    pub field_masks: HashMap<FieldId, Vec<U32>>,
    /// Total document length
    pub document_length: U32,
}

impl DocumentAccumulator {
    pub fn new(document_length: U32) -> Self {
        Self {
            bm25_score: 0.0,
            term_masks: Vec::new(),
            term_idfs: Vec::new(),
            field_masks: HashMap::new(),
            document_length,
        }
    }
}

// =============================================================================
// Term with IDF
// =============================================================================

/// Term data with IDF for proximity calculations
#[derive(Debug, Clone, Copy)]
pub struct TermWithIdf {
    pub mask: U32,
    pub idf: F32,
}

// =============================================================================
// Score Explanation Types
// =============================================================================

/// Breakdown of a single term's contribution to the score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermBreakdown {
    pub term: String,
    pub idf: F32,
    pub aggregated_s: F32,
    pub saturated_score: F32,
    pub segment_mask: String,
    pub field_contributions: Vec<FieldContribution>,
    pub entropy: Option<F32>,
    pub raw_entropy: Option<F32>,
}

/// Contribution from a single field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldContribution {
    pub field_id: FieldId,
    pub tf: U32,
    pub field_length: U32,
    pub normalized_tf: F32,
    pub weighted_contribution: F32,
}

/// Full explanation of a document's score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreExplanation {
    pub total_score: F32,
    pub bm25_component: F32,
    pub proximity_multiplier: F32,
    pub idf_proximity_boost: F32,
    pub length_decay: F32,
    pub phrase_boost: F32,
    pub overlap_count: U32,
    pub term_breakdown: Vec<TermBreakdown>,
    pub strategy: String,
    pub bmx_entropy_similarity_boost: Option<F32>,
    pub bmx_similarity: Option<F32>,
    pub bmx_avg_entropy: Option<F32>,
    pub bmx_alpha: Option<F32>,
    pub bmx_beta: Option<F32>,
    pub normalized_score: Option<F32>,
}

impl ScoreExplanation {
    pub fn empty(strategy: &str) -> Self {
        Self {
            total_score: 0.0,
            bm25_component: 0.0,
            proximity_multiplier: 1.0,
            idf_proximity_boost: 1.0,
            length_decay: 1.0,
            phrase_boost: 1.0,
            overlap_count: 0,
            term_breakdown: Vec::new(),
            strategy: strategy.to_string(),
            bmx_entropy_similarity_boost: None,
            bmx_similarity: None,
            bmx_avg_entropy: None,
            bmx_alpha: None,
            bmx_beta: None,
            normalized_score: None,
        }
    }
}

// =============================================================================
// Search Results
// =============================================================================

/// Search result with document ID and score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub doc_id: String,
    pub score: F32,
    pub normalized_score: Option<F32>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_metadata() {
        let mut meta = TokenMetadata::new(10);
        meta.add_field_occurrence(0, 3, 100);
        meta.set_segment_mask(0b1010);
        
        assert_eq!(meta.corpus_doc_frequency, 10);
        assert_eq!(meta.segment_mask, 0b1010);
        assert!(meta.field_occurrences.contains_key(&0));
    }

    #[test]
    fn test_document_metadata() {
        let mut doc = DocumentMetadata::new();
        doc.set_field_length(0, 10);
        doc.set_field_length(1, 500);
        
        assert_eq!(doc.total_token_count, 510);
    }

    #[test]
    fn test_document_accumulator() {
        let acc = DocumentAccumulator::new(100);
        assert_eq!(acc.bm25_score, 0.0);
        assert_eq!(acc.document_length, 100);
    }

    #[test]
    fn test_score_explanation_empty() {
        let exp = ScoreExplanation::empty("IdfWeighted");
        assert_eq!(exp.total_score, 0.0);
        assert_eq!(exp.strategy, "IdfWeighted");
    }
}
