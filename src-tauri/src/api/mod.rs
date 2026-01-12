//! TauRPC API Module - Type-safe IPC Layer
//!
//! All domain APIs for typed communication between frontend and backend.
//!
//! # Architecture
//!
//! ```text
//! Router
//! ├── core.*         (version, scan, relations)
//! ├── graph.*        (entities, edges, hydration)
//! ├── ner.*          (model, analysis, suggestions)
//! ├── fst_ner.*      (hot-path entity recognition)
//! ├── rag.*          (embeddings, search)
//! ├── blueprint.*    (types, fields, relationships)
//! └── time_registry.* (change history)
//! └── content.*      (notes, folders, etc.)
//! ```

// Domain modules
pub mod core;
pub mod graph;
pub mod ner;
pub mod fst_ner;
pub mod rag;
pub mod blueprint;
pub mod time_registry;
pub mod content;

use taurpc::Router;

// Import traits for into_handler() method
use core::CoreApi;
use graph::GraphApi;
use ner::NerApi;
use fst_ner::FstNerApi;
use rag::RagApi;
use blueprint::BlueprintApi;
use time_registry::TimeRegistryApi;
use content::ContentApi;

/// Create the unified TauRPC router with all API handlers
pub fn create_router() -> Router<tauri::Wry> {
    Router::new()
        .merge(core::CoreApiImpl::default().into_handler())
        .merge(graph::GraphApiImpl::default().into_handler())
        .merge(ner::NerApiImpl::default().into_handler())
        .merge(fst_ner::FstNerApiImpl::default().into_handler())
        .merge(rag::RagApiImpl::default().into_handler())
        .merge(blueprint::BlueprintApiImpl::default().into_handler())
        .merge(time_registry::TimeRegistryApiImpl::default().into_handler())
        .merge(content::ContentApiImpl::default().into_handler())
}

