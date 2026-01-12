//! Graph Registry Module
//!
//! Unified graph storage for nodes (entities) and edges (relationships).
//! Uses CozoDB with SQLite backend for persistence.
//!
//! # Architecture
//! - `schema.rs` - CozoDB relation definitions (knowledge graph + content)
//! - `types.rs` - Rust types for nodes/edges
//! - `content_types.rs` - Content types (notes, folders, networks, etc.)
//! - `content_repos.rs` - Content CRUD repos (SurrealDB replacement)
//! - `content_commands.rs` - Tauri commands for content (notes, folders)
//! - `registry.rs` - Core GraphRegistry struct
//! - `queries.rs` - Datalog query builders (neighbors, paths, subgraphs)
//! - `algorithms.rs` - Graph algorithms (PageRank, communities)
//! - `bridge.rs` - Scanner bridge (hydration, ingest)
//! - `commands.rs` - Tauri commands

pub mod schema;
mod types;
mod registry;
mod queries;
mod algorithms;
mod bridge;
pub mod cozo_graph;
pub mod commands;
pub mod content_types;
pub mod content_repos;
pub mod content_commands;
pub mod graph_backend;
pub mod projection;

pub use schema::*;
pub use types::*;
pub use registry::*;
pub use queries::*;
pub use algorithms::*;
pub use bridge::*;
pub use cozo_graph::*;
pub use content_types::*;
pub use content_repos::*;
pub use content_commands::*;
pub use graph_backend::*;
pub use projection::*;

#[cfg(test)]
mod tests;
