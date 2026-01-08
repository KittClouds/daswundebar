// src-tauri/src/rag/mod.rs
//! RAG Pipeline Module for Tauri Native
//!
//! Unified RAG (Retrieval-Augmented Generation) pipeline using:
//! - CozoDB HNSW for vector similarity search
//! - embed-anything for native embedding generation
//! - RAPTOR for hierarchical retrieval

pub mod schema;
pub mod embeddings;
pub mod chunker;
pub mod commands;

// Re-exports
pub use commands::*;
pub use embeddings::EmbeddingService;
pub use chunker::RagChunker;
