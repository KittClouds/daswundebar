//! Cross-Document Entity Linking
//! 
//! This module implements deterministic string-based entity linking
//! and semantic entity clustering using CST context embeddings.
//!
//! Submodules:
//! - `string_sim`: String similarity algorithms (Levenshtein, Jaro-Winkler)
//! - `linker`: Greedy clustering logic
//! - `types`: Shared types for linking and clusters

pub mod types;
pub mod string_sim;
pub mod linker;
pub mod context_embed;
pub mod hybrid;
// Phase 3:
pub mod commands;
