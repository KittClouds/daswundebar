// Reality Engine - Core semantic graph system
//
// Ported directly from kittcore/reality - zero rewrites needed!

pub mod syntax;
pub mod ast;
pub mod parser;
pub mod projection;
pub mod graph;
pub mod synapse;
pub mod engine;
pub mod entity_layer;
pub mod metadata_layer;
pub mod unification;
pub mod global;
pub mod algorithms;
pub mod temporal;
pub mod intern;

// Re-export key types for convenience
pub use syntax::{SyntaxKind, RealityLanguage};
pub use graph::{ConceptGraph, ConceptNode, ConceptEdge, EdgeKind};
pub use synapse::SynapseBridge;
pub use engine::RealityEngine;
pub use projection::{Triple, QuadPlus, Attribution, StateChange, Projection};
pub use global::GlobalGraph;
pub use unification::EntityUnifier;

