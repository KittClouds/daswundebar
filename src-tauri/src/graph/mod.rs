//! Graph Registry Module
//!
//! Unified graph storage for nodes (entities) and edges (relationships).
//! Uses CozoDB with SQLite backend for persistence.
//!
//! # Architecture
//! - `schema.rs` - CozoDB relation definitions
//! - `types.rs` - Rust types for nodes/edges
//! - `registry.rs` - Core GraphRegistry struct
//! - `queries.rs` - Datalog query builders (neighbors, paths, subgraphs)
//! - `algorithms.rs` - Graph algorithms (PageRank, communities)
//! - `bridge.rs` - Scanner bridge (hydration, ingest)
//! - `commands.rs` - Tauri commands

mod schema;
mod types;
mod registry;
mod queries;
mod algorithms;
mod bridge;
pub mod commands;

pub use schema::*;
pub use types::*;
pub use registry::*;
pub use queries::*;
pub use algorithms::*;
pub use bridge::*;

#[cfg(test)]
mod tests;

