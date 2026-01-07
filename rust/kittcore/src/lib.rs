//! KittCore: Document Scanner + ResoRank Search Engine + Embeddings
//!
//! A Rust/WASM implementation of the KittClouds document analysis pipeline.
//!
//! # Architecture
//!
//! ## Scanner Components (Entity System 5.0)
//! - `document.rs` - DocumentCortex: **Unified scanner** - single scan() for all extraction
//! - `relation.rs` - RelationCortex: Relationship extraction (260+ patterns, bidirectional)
//! - `implicit.rs` - ImplicitCortex: Entity name matching via Aho-Corasick
//! - `triple.rs` - TripleCortex: Triple [[A->B->C]] syntax extraction
//! - `change.rs` - ChangeDetector: Content-addressable skip detection
//! - `syntax.rs` - SyntaxCortex: Document syntax patterns (wikilinks, tags, entities)
//! - `temporal.rs` - TemporalCortex: Temporal expression detection (126+ patterns)
//! - `reflex.rs` - ReflexCortex: Entity name matching (legacy)
//!
//! ## Search Components (ResoRank)
//! - `config.rs` - Configuration types and defaults
//! - `types.rs` - Core data structures (TokenMetadata, DocumentMetadata, etc.)
//! - `scorer.rs` - Main ResoRankScorer implementation
//!
//! ## Embeddings (A/B Testing Pipeline)
//! - `embeddings/` - Rust-native ONNX inference for A/B testing against TypeScript
//! - Uses tract-onnx for pure Rust WASM-compatible inference
//! - Supports BGESmallENV15, ModernBERTBase, AllMiniLML6V2
//!
//! # Usage (WASM)
//! ```javascript,ignore
//! import init, { DocumentCortex, EmbedCortex } from 'kittcore';
//!
//! await init();
//!
//! // Create unified scanner
//! const cortex = new DocumentCortex();
//!
//! // Hydrate with entities for implicit matching
//! cortex.hydrate_entities([
//!   { id: 'e1', label: 'Frodo', kind: 'CHARACTER', aliases: ['Mr. Frodo'] }
//! ]);
//!
//! // Single scan call - extracts everything
//! const result = cortex.scan(
//!   "Frodo is brother of Sam. [[Frodo->OWNS->Ring]]",
//!   [{ label: 'Frodo', start: 0, end: 5 }, { label: 'Sam', start: 20, end: 23 }]
//! );
//!
//! // === Embeddings (A/B Testing) ===
//! const embedder = new EmbedCortex();
//! await embedder.loadModel(modelBytes, tokenizerJson);
//! const embedding = embedder.embedText("Hello world");
//! ```

// Scanner modules (Entity System 5.0)
pub mod scanner;
pub mod resorank;
pub mod reality;
pub mod embeddings;
pub mod rag;
pub mod hnsw;
#[cfg(feature = "sqlite_wasm")]
pub mod db;  // SQLite WASM (experimental)
pub mod graphdb;  // Graph database layer

// Public exports - Scanner (explicit to avoid incremental collision with resorank)
pub use scanner::{
    // Core types
    document::{DocumentCortex, ScanResult, ScanStats, ScanTimings, ScanError},
    // Cortex modules
    reflex::{ReflexCortex, EntityMatch, MatchType, ReflexStats},
    syntax::{SyntaxCortex, SyntaxMatch, SyntaxKind},
    temporal::{TemporalCortex, TemporalMention, TemporalScanResult, TemporalScanStats, TemporalKind},
    relation::{RelationCortex, ExtractedRelation, EntitySpan},
    implicit::{ImplicitCortex, ImplicitMention, EntityDefinition},
    triple::{TripleCortex, ExtractedTriple},
    change::ChangeDetector,
    conductor::ScanConductor,
    chunker::{Chunker, Chunk, ChunkKind, ChunkResult, ChunkStats},
    attacher::{Attacher, Dependency, DependencyGraph, DependencyKind},
    resolver::{Resolver, NarrativeContext, MentionChain, Gender},
    dialogue::{DialogueAttributor, QuotePosition},
    narrative::{NarrativeGraph, NarrativeResult, NarrativeStats},
    // Unified Scanner (NEW - Rust-first document processing)
    unified::{
        WasmUnifiedScanner, UnifiedScanner, UnifiedScanResult, UnifiedScanStats,
        DecorationSpan, StylingHint, RefKind,
    },
    // Constraints (NEW - Ref validation)
    constraints::{
        WasmConstraintEngine, ConstraintEngine, ConstraintResult,
    },
    // Projections (NEW - Views & projections)
    projections::{
        WasmProjector, Projector,
        TimelineEvent, CharacterSheet, RelationshipGraph, LinkGraph,
    },
};

// Public exports - ResoRank
pub use resorank::*;

// Public exports - Reality (Semantic Graph Engine)
pub use reality::{
    api::RealityCortex,
    engine::RealityEngine,
    graph::{ConceptGraph, ConceptNode, ConceptEdge},
    synapse::SynapseBridge,
};

// Public exports - Embeddings (A/B Testing Pipeline)
pub use embeddings::{
    EmbedCortex,
    EmbedConfig, OnnxModel, PoolingStrategy, SplittingStrategy,
    EmbedModel, EmbedResult,
    TextChunker, Chunk as EmbedChunk,
};

// Public exports - RAG Pipeline (Phase 1)
pub use rag::{
    RagPipeline,
    RagChunker, Chunk as RagChunk,
    VectorIndex, SearchResult as VectorSearchResult,
};

// Public exports - SQLite WASM (experimental)
#[cfg(feature = "sqlite_wasm")]
pub use db::WasmDatabase;

// WASM bindings - only when wasm feature is enabled
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator for smaller WASM bundle size.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// Initialize panic hook for better error messages in browser console
#[cfg_attr(feature = "wasm", wasm_bindgen(start))]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Greet function for testing WASM binding
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! KittCore is ready.", name)
}

/// Get version information
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn version() -> String {
    format!("kittcore v{}", env!("CARGO_PKG_VERSION"))
}

/// Get build timestamp for detecting stale WASM binaries
/// This changes every time the WASM is rebuilt
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn build_timestamp() -> String {
    // This is set at compile time
    option_env!("BUILD_TIMESTAMP")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .to_string()
}

/// Check if this build has catch_unwind protection
/// Used to verify the binary isn't stale
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub fn has_panic_protection() -> bool {
    true // This version has catch_unwind in scan()
}
