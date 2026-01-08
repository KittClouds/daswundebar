//! ResoRank - Resonance-Based Hybrid Scoring System
//!
//! A high-performance BM25F implementation with:
//! - Multi-field scoring with configurable weights
//! - 4 proximity strategies (Global, PerTerm, Pairwise, IdfWeighted)
//! - BMX entropy-weighted extensions
//! - LRU caching for IDF and entropy values
//!
//! Ported from kittcore WASM to Tauri native.

#![allow(dead_code)] // Will be used when Tauri commands are wired

mod config;
mod entropy;
mod math;
mod proximity;
mod scorer;
mod types;

// Public API
pub use config::{ResoRankConfig, CorpusStatistics, FieldParams, CorpusSize};
pub use proximity::ProximityStrategy;
pub use scorer::{ResoRankScorer, ScorerStats};
pub use types::{
    DocumentMetadata, TokenMetadata, FieldOccurrence,
    SearchResult, ScoreExplanation, TermBreakdown, FieldContribution,
};
