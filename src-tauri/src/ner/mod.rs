//! FST-NER Engine - Hot-Path Named Entity Recognition
//!
//! A compiled finite-state transducer cascade for keystroke-level NER.
//!
//! # Architecture
//!
//! ```text
//! Text → Tokenizer → Gazetteer → Rules → Resolver → Spans
//! ```
//!
//! # Performance Target
//!
//! - < 2ms for 10KB text
//! - < 0.5ms per keystroke (visible paragraph)

pub mod tokenizer;
pub mod orthographic;
pub mod gazetteer;
pub mod rules;
pub mod pipeline;
pub mod commands;

pub use tokenizer::{Token, TokenKind, Tokenizer};
pub use orthographic::{OrthographicShape, OrthographicClassifier};
pub use gazetteer::{Gazetteer, GazetteerMatch, MatchPriority};
pub use rules::{Rule, RuleEngine, RuleMatch};
pub use pipeline::{Pipeline, PipelineResult, EntityMatch, MatchSource};
pub use commands::{fst_scan, fst_hydrate, fst_clear, fst_add_entity, FstScanResult, FstMatch, EntityDef};
