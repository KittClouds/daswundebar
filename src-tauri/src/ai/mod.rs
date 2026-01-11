//! AI Subsystem - NER and Inference Services
//!
//! Phase 1: GLiNER-based Named Entity Recognition
//!
//! # Architecture
//!
//! ```text
//! Frontend ──► NerService ──► Worker Thread ──► GLiNER (gline-rs)
//!                  │                                    │
//!                  └────────────────────────────────────┘
//!                              Async Results
//! ```
//!
//! The NER service runs on a dedicated OS thread (not tokio) to:
//! 1. Keep the model loaded in memory (~100MB)
//! 2. Avoid blocking the async runtime
//! 3. Process requests via mpsc channels

pub mod model_manager;
pub mod ner_service;
pub mod suggestion_layer;
pub mod ner_postprocess;
pub mod commands;

pub use model_manager::{ModelManager, ModelStatus, DownloadProgress};
pub use ner_service::{NerService, NerServiceConfig, AnalysisRequest, AnalysisResult, InferredEntity};
pub use suggestion_layer::{SuggestionLayer, Suggestion, SuggestionStatus, AcceptResult, map_label_to_kind};
