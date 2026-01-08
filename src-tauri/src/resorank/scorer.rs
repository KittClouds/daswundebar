//! Main ResoRankScorer implementation
//!
//! The core scoring engine implementing BM25F with proximity and BMX extensions.
//! Ported from kittcore WASM - WASM bindings removed.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::config::{CorpusStatistics, ResoRankConfig, F32, U32, Usize};
use super::entropy::{calculate_query_entropy_stats, EntropyCache, QueryEntropyStats};
use super::math::{
    adaptive_segment_count, calculate_adaptive_alpha, calculate_beta, calculate_idf,
    format_binary, normalize_score, normalized_term_frequency_bmx, saturate_bmx,
};
use super::proximity::{
    detect_phrase_match, global_proximity_multiplier, idf_weighted_proximity_multiplier,
    pairwise_proximity_bonus, per_term_proximity_multiplier, ProximityResult, ProximityStrategy,
};
use super::types::{
    DocumentAccumulator, DocumentMetadata, FieldContribution, ScoreExplanation, SearchResult,
    TermBreakdown, TermWithIdf, TokenMetadata,
};

// =============================================================================
// Main Scorer
// =============================================================================

/// ResoRank Scorer - BM25F with Proximity and BMX extensions
#[derive(Serialize, Deserialize)]
pub struct ResoRankScorer {
    config: ResoRankConfig,
    corpus_stats: CorpusStatistics,
    proximity_strategy: ProximityStrategy,

    // Indexes
    document_index: HashMap<String, DocumentMetadata>,
    token_index: HashMap<String, HashMap<String, TokenMetadata>>,

    // Caches
    idf_cache: HashMap<Usize, F32>,
    entropy_cache: EntropyCache,

    // Pre-computed BMX parameters
    cached_alpha: Option<F32>,
    cached_beta: Option<F32>,
    cached_gamma: Option<F32>,
}

impl ResoRankScorer {
    /// Create a new scorer with the given configuration
    pub fn new(
        config: ResoRankConfig,
        corpus_stats: CorpusStatistics,
        proximity_strategy: ProximityStrategy,
    ) -> Self {
        let (cached_alpha, cached_beta, cached_gamma) =
            if config.use_adaptive_alpha || config.enable_bmx_entropy || config.enable_bmx_similarity
            {
                let alpha = calculate_adaptive_alpha(corpus_stats.average_document_length);
                let beta = calculate_beta(corpus_stats.total_documents);
                let gamma = config.entropy_denom_weight.unwrap_or(alpha / 2.0);
                (Some(alpha), Some(beta), Some(gamma))
            } else {
                (None, None, None)
            };

        Self {
            config,
            corpus_stats,
            proximity_strategy,
            document_index: HashMap::new(),
            token_index: HashMap::new(),
            idf_cache: HashMap::new(),
            entropy_cache: EntropyCache::new(1000),
            cached_alpha,
            cached_beta,
            cached_gamma,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(corpus_stats: CorpusStatistics) -> Self {
        Self::new(
            ResoRankConfig::default(),
            corpus_stats,
            ProximityStrategy::IdfWeighted,
        )
    }

    // =========================================================================
    // Indexing
    // =========================================================================

    /// Index a document for later scoring
    pub fn index_document(
        &mut self,
        doc_id: &str,
        doc_meta: DocumentMetadata,
        tokens: HashMap<String, TokenMetadata>,
        use_adaptive_segments: bool,
    ) {
        let effective_max_segments = if use_adaptive_segments {
            adaptive_segment_count(doc_meta.total_token_count, 50)
        } else {
            self.config.max_segments
        };

        self.document_index.insert(doc_id.to_string(), doc_meta);

        for (term, mut meta) in tokens {
            if use_adaptive_segments {
                meta.segment_mask =
                    self.remap_segment_mask(meta.segment_mask, self.config.max_segments, effective_max_segments);
            }

            self.token_index
                .entry(term)
                .or_insert_with(HashMap::new)
                .insert(doc_id.to_string(), meta);
        }
    }

    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: &str) -> bool {
        let existed = self.document_index.remove(doc_id).is_some();

        for term_docs in self.token_index.values_mut() {
            term_docs.remove(doc_id);
        }

        self.token_index.retain(|_, docs| !docs.is_empty());

        existed
    }

    /// Clear all indexes and caches
    pub fn clear(&mut self) {
        self.document_index.clear();
        self.token_index.clear();
        self.idf_cache.clear();
        self.entropy_cache.clear();
    }

    /// Remap segment mask from one granularity to another
    fn remap_segment_mask(&self, mask: U32, from_segments: U32, to_segments: U32) -> U32 {
        if from_segments == to_segments || from_segments == 0 {
            return mask;
        }

        let mut new_mask = 0u32;
        for i in 0..from_segments {
            if mask & (1 << i) != 0 {
                let mapped_bit = ((i as f32 / from_segments as f32) * to_segments as f32) as u32;
                if mapped_bit < 32 {
                    new_mask |= 1 << mapped_bit;
                }
            }
        }
        new_mask
    }

    // =========================================================================
    // IDF Cache
    // =========================================================================

    fn get_or_calculate_idf(&mut self, corpus_doc_freq: Usize) -> F32 {
        if let Some(&idf) = self.idf_cache.get(&corpus_doc_freq) {
            return idf;
        }

        let idf = calculate_idf(self.corpus_stats.total_documents as f32, corpus_doc_freq);
        self.idf_cache.insert(corpus_doc_freq, idf);
        idf
    }

    /// Pre-compute IDF values for all indexed terms
    pub fn warm_idf_cache(&mut self) {
        let mut unique_frequencies = std::collections::HashSet::new();

        for term_docs in self.token_index.values() {
            for meta in term_docs.values() {
                unique_frequencies.insert(meta.corpus_doc_frequency);
            }
        }

        for freq in unique_frequencies {
            self.get_or_calculate_idf(freq);
        }
    }

    // =========================================================================
    // Scoring
    // =========================================================================

    /// Score a document against a query
    pub fn score(&mut self, query: &[String], doc_id: &str) -> F32 {
        if query.len() == 1 {
            return self.score_single_term(&query[0], doc_id);
        }

        let explanation = self.explain_score(query, doc_id);
        explanation.total_score
    }

    /// Fast path for single-term scoring
    fn score_single_term(&mut self, term: &str, doc_id: &str) -> F32 {
        let token_meta = match self.get_token_metadata(term, doc_id) {
            Some(meta) => meta.clone(),
            None => return 0.0,
        };

        let idf = self.get_or_calculate_idf(token_meta.corpus_doc_frequency);

        let (avg_entropy, gamma) = if self.config.enable_bmx_entropy {
            (1.0, self.cached_gamma.unwrap_or(0.0))
        } else {
            (0.0, 0.0)
        };

        let mut aggregated_s = 0.0;

        for (&field_id, field_data) in &token_meta.field_occurrences {
            let params = match self.config.field_params.get(&field_id) {
                Some(p) => p,
                None => continue,
            };

            let avg_len = self
                .corpus_stats
                .average_field_lengths
                .get(&field_id)
                .copied()
                .unwrap_or(1.0);

            let normalized_tf = normalized_term_frequency_bmx(
                field_data.tf,
                field_data.field_length,
                avg_len,
                params.b,
                avg_entropy,
                gamma,
            );

            aggregated_s += params.weight * normalized_tf;
        }

        let saturation_param = if self.config.use_adaptive_alpha {
            self.cached_alpha.unwrap_or(self.config.k1)
        } else {
            self.config.k1
        };

        let mut score = idf * saturate_bmx(aggregated_s, saturation_param);

        if self.config.enable_bmx_similarity {
            if let Some(beta) = self.cached_beta {
                score += beta * 1.0 * 1.0;
            }
        }

        score
    }

    /// Score with full explanation
    pub fn explain_score(&mut self, query: &[String], doc_id: &str) -> ScoreExplanation {
        let doc_meta = match self.document_index.get(doc_id) {
            Some(meta) => meta.clone(),
            None => return ScoreExplanation::empty(self.proximity_strategy.as_str()),
        };

        let entropy_stats =
            if self.config.enable_bmx_entropy || self.config.enable_bmx_similarity {
                Some(calculate_query_entropy_stats(
                    query,
                    &mut self.entropy_cache,
                    &self.token_index,
                ))
            } else {
                None
            };

        let mut accumulator = DocumentAccumulator::new(doc_meta.total_token_count);
        let mut term_breakdown = Vec::new();
        let mut doc_term_masks = HashMap::new();

        for (i, term) in query.iter().enumerate() {
            let (term_score, breakdown) =
                self.score_term_bm25f(term, doc_id, &mut accumulator, entropy_stats.as_ref());

            if let Some(bd) = breakdown {
                if let Ok(mask) = u32::from_str_radix(&bd.segment_mask, 2) {
                    doc_term_masks.insert(term.clone(), mask);
                }
                term_breakdown.push(bd);
            }

            if self.proximity_strategy == ProximityStrategy::PerTerm && term_score > 0.0 {
                if let Some(token_meta) = self.get_token_metadata(term, doc_id) {
                    let other_masks: Vec<_> = accumulator.term_masks[..i].to_vec();
                    let proximity = per_term_proximity_multiplier(
                        token_meta.segment_mask,
                        &other_masks,
                        self.config.proximity_alpha,
                        self.config.max_segments,
                    );
                    accumulator.bm25_score += term_score * proximity;
                }
            } else {
                accumulator.bm25_score += term_score;
            }
        }

        let prox_result = self.calculate_proximity_multiplier(&accumulator);
        let mut final_score = accumulator.bm25_score * prox_result.multiplier;

        let mut phrase_boost = 1.0;
        if self.config.enable_phrase_boost && query.len() >= 2 {
            if detect_phrase_match(query, &doc_term_masks) {
                phrase_boost = self.config.phrase_boost_multiplier;
                final_score *= phrase_boost;
            }
        }

        let (bmx_similarity_boost, bmx_similarity) =
            if self.config.enable_bmx_similarity && entropy_stats.is_some() {
                if let Some(beta) = self.cached_beta {
                    let sim = self.calculate_query_doc_similarity(query, doc_id);
                    let boost = beta * sim * entropy_stats.as_ref().unwrap().sum_normalized_entropies;
                    final_score += boost;
                    (Some(boost), Some(sim))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

        ScoreExplanation {
            total_score: final_score,
            bm25_component: accumulator.bm25_score,
            proximity_multiplier: prox_result.multiplier,
            idf_proximity_boost: prox_result.idf_boost,
            length_decay: prox_result.decay,
            phrase_boost,
            overlap_count: prox_result.overlap_count,
            term_breakdown,
            strategy: self.proximity_strategy.as_str().to_string(),
            bmx_entropy_similarity_boost: bmx_similarity_boost,
            bmx_similarity,
            bmx_avg_entropy: entropy_stats.as_ref().map(|s| s.avg_entropy),
            bmx_alpha: if self.config.use_adaptive_alpha {
                self.cached_alpha
            } else {
                None
            },
            bmx_beta: if self.config.enable_bmx_similarity {
                self.cached_beta
            } else {
                None
            },
            normalized_score: None,
        }
    }

    /// Score a single term with BM25F
    fn score_term_bm25f(
        &mut self,
        term: &str,
        doc_id: &str,
        accumulator: &mut DocumentAccumulator,
        entropy_stats: Option<&QueryEntropyStats>,
    ) -> (F32, Option<TermBreakdown>) {
        let token_meta = match self.get_token_metadata(term, doc_id) {
            Some(meta) => meta.clone(),
            None => return (0.0, None),
        };

        accumulator.term_masks.push(token_meta.segment_mask);
        let idf = self.get_or_calculate_idf(token_meta.corpus_doc_frequency);
        accumulator.term_idfs.push(idf);

        let avg_entropy = entropy_stats.map(|s| s.avg_entropy).unwrap_or(0.0);
        let gamma = if self.config.enable_bmx_entropy {
            self.cached_gamma.unwrap_or(0.0)
        } else {
            0.0
        };

        let mut aggregated_s = 0.0;
        let mut field_contributions = Vec::new();

        for (&field_id, field_data) in &token_meta.field_occurrences {
            let params = match self.config.field_params.get(&field_id) {
                Some(p) => p,
                None => continue,
            };

            let avg_len = self
                .corpus_stats
                .average_field_lengths
                .get(&field_id)
                .copied()
                .unwrap_or(1.0);

            let normalized_tf = normalized_term_frequency_bmx(
                field_data.tf,
                field_data.field_length,
                avg_len,
                params.b,
                avg_entropy,
                gamma,
            );

            let weighted_contribution = params.weight * normalized_tf;
            aggregated_s += weighted_contribution;

            field_contributions.push(FieldContribution {
                field_id,
                tf: field_data.tf,
                field_length: field_data.field_length,
                normalized_tf,
                weighted_contribution,
            });

            accumulator
                .field_masks
                .entry(field_id)
                .or_insert_with(Vec::new)
                .push(token_meta.segment_mask);
        }

        let saturation_param = if self.config.use_adaptive_alpha {
            self.cached_alpha.unwrap_or(self.config.k1)
        } else {
            self.config.k1
        };

        let saturated_score = idf * saturate_bmx(aggregated_s, saturation_param);

        let breakdown = TermBreakdown {
            term: term.to_string(),
            idf,
            aggregated_s,
            saturated_score,
            segment_mask: format_binary(token_meta.segment_mask, self.config.max_segments),
            field_contributions,
            entropy: entropy_stats.and_then(|s| s.normalized_entropies.get(term).copied()),
            raw_entropy: Some(
                self.entropy_cache
                    .get_cached(term)
                    .unwrap_or(0.0),
            ),
        };

        (saturated_score, Some(breakdown))
    }

    /// Calculate proximity multiplier based on strategy
    fn calculate_proximity_multiplier(&self, accumulator: &DocumentAccumulator) -> ProximityResult {
        match self.proximity_strategy {
            ProximityStrategy::Global => global_proximity_multiplier(
                &accumulator.term_masks,
                self.config.proximity_alpha,
                self.config.max_segments,
                accumulator.document_length,
                self.corpus_stats.average_document_length,
                self.config.proximity_decay_lambda,
            ),

            ProximityStrategy::IdfWeighted => {
                let term_data: Vec<_> = accumulator
                    .term_masks
                    .iter()
                    .zip(&accumulator.term_idfs)
                    .map(|(&mask, &idf)| TermWithIdf { mask, idf })
                    .collect();

                idf_weighted_proximity_multiplier(
                    &term_data,
                    self.config.proximity_alpha,
                    self.config.max_segments,
                    accumulator.document_length,
                    self.corpus_stats.average_document_length,
                    self.config.proximity_decay_lambda,
                    self.config.idf_proximity_scale,
                )
            }

            ProximityStrategy::Pairwise => {
                let bonus = pairwise_proximity_bonus(
                    &accumulator.term_masks,
                    self.config.proximity_alpha,
                    self.config.max_segments,
                );
                ProximityResult {
                    multiplier: 1.0 + bonus,
                    overlap_count: 0,
                    decay: 1.0,
                    idf_boost: 1.0,
                }
            }

            ProximityStrategy::PerTerm => ProximityResult::default(),
        }
    }

    fn get_token_metadata(&self, term: &str, doc_id: &str) -> Option<&TokenMetadata> {
        self.token_index.get(term)?.get(doc_id)
    }

    fn calculate_query_doc_similarity(&self, query: &[String], doc_id: &str) -> F32 {
        if query.is_empty() {
            return 0.0;
        }

        let common_terms = query
            .iter()
            .filter(|term| {
                self.token_index
                    .get(*term)
                    .map(|docs| docs.contains_key(doc_id))
                    .unwrap_or(false)
            })
            .count();

        common_terms as f32 / query.len() as f32
    }

    // =========================================================================
    // Search
    // =========================================================================

    /// Search for documents matching the query
    pub fn search(&mut self, query: &[String], limit: usize) -> Vec<SearchResult> {
        let mut candidates = std::collections::HashSet::new();
        for term in query {
            if let Some(term_docs) = self.token_index.get(term) {
                for doc_id in term_docs.keys() {
                    candidates.insert(doc_id.clone());
                }
            }
        }

        let mut scores: Vec<_> = candidates
            .into_iter()
            .filter_map(|doc_id| {
                let score = self.score(query, &doc_id);
                if score > 0.0 {
                    Some(SearchResult {
                        doc_id,
                        score,
                        normalized_score: None,
                    })
                } else {
                    None
                }
            })
            .collect();

        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(limit);
        scores
    }

    /// Search with score normalization
    pub fn search_normalized(&mut self, query: &[String], limit: usize) -> Vec<SearchResult> {
        let mut results = self.search(query, limit);

        for result in &mut results {
            result.normalized_score = Some(normalize_score(
                result.score,
                query.len(),
                self.corpus_stats.total_documents,
            ));
        }

        results
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Get index statistics
    pub fn stats(&self) -> ScorerStats {
        ScorerStats {
            document_count: self.document_index.len(),
            term_count: self.token_index.len(),
            idf_cache_size: self.idf_cache.len(),
            entropy_cache_size: self.entropy_cache.stats().size,
        }
    }
}

/// Scorer statistics
#[derive(Debug, Clone)]
pub struct ScorerStats {
    pub document_count: usize,
    pub term_count: usize,
    pub idf_cache_size: usize,
    pub entropy_cache_size: usize,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resorank::types::FieldOccurrence;

    fn create_test_scorer() -> ResoRankScorer {
        let mut corp_stats = CorpusStatistics::default();
        corp_stats.total_documents = 100;
        corp_stats.average_document_length = 500.0;
        corp_stats.average_field_lengths.insert(0, 10.0);
        corp_stats.average_field_lengths.insert(1, 490.0);

        ResoRankScorer::with_defaults(corp_stats)
    }

    fn create_test_document() -> (DocumentMetadata, HashMap<String, TokenMetadata>) {
        let mut doc_meta = DocumentMetadata::new();
        doc_meta.set_field_length(0, 5);
        doc_meta.set_field_length(1, 100);

        let mut tokens = HashMap::new();

        let mut hello_meta = TokenMetadata::new(10);
        hello_meta.field_occurrences.insert(
            0,
            FieldOccurrence {
                tf: 1,
                field_length: 5,
            },
        );
        hello_meta.field_occurrences.insert(
            1,
            FieldOccurrence {
                tf: 3,
                field_length: 100,
            },
        );
        hello_meta.segment_mask = 0b0011;
        tokens.insert("hello".to_string(), hello_meta);

        let mut world_meta = TokenMetadata::new(5);
        world_meta.field_occurrences.insert(
            1,
            FieldOccurrence {
                tf: 2,
                field_length: 100,
            },
        );
        world_meta.segment_mask = 0b0110;
        tokens.insert("world".to_string(), world_meta);

        (doc_meta, tokens)
    }

    #[test]
    fn test_index_and_score() {
        let mut scorer = create_test_scorer();
        let (doc_meta, tokens) = create_test_document();

        scorer.index_document("doc1", doc_meta, tokens, false);

        let score = scorer.score(&["hello".to_string()], "doc1");
        assert!(score > 0.0);
    }

    #[test]
    fn test_multi_term_query() {
        let mut scorer = create_test_scorer();
        let (doc_meta, tokens) = create_test_document();

        scorer.index_document("doc1", doc_meta, tokens, false);

        let single_score = scorer.score(&["hello".to_string()], "doc1");
        let multi_score = scorer.score(&["hello".to_string(), "world".to_string()], "doc1");

        assert!(multi_score > single_score);
    }

    #[test]
    fn test_search() {
        let mut scorer = create_test_scorer();
        let (doc_meta, tokens) = create_test_document();

        scorer.index_document("doc1", doc_meta, tokens, false);

        let results = scorer.search(&["hello".to_string()], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_id, "doc1");
    }

    #[test]
    fn test_remove_document() {
        let mut scorer = create_test_scorer();
        let (doc_meta, tokens) = create_test_document();

        scorer.index_document("doc1", doc_meta, tokens, false);
        assert!(scorer.remove_document("doc1"));

        let results = scorer.search(&["hello".to_string()], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_explain_score() {
        let mut scorer = create_test_scorer();
        let (doc_meta, tokens) = create_test_document();

        scorer.index_document("doc1", doc_meta, tokens, false);

        let explanation = scorer.explain_score(&["hello".to_string(), "world".to_string()], "doc1");

        assert!(explanation.total_score > 0.0);
        assert_eq!(explanation.term_breakdown.len(), 2);
        assert!(explanation.proximity_multiplier >= 1.0);
    }

    #[test]
    fn test_scorer_stats() {
        let mut scorer = create_test_scorer();
        let (doc_meta, tokens) = create_test_document();

        scorer.index_document("doc1", doc_meta, tokens, false);

        let stats = scorer.stats();
        assert_eq!(stats.document_count, 1);
        assert_eq!(stats.term_count, 2);
    }
}
