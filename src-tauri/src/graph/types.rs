//! Graph Registry Types
//!
//! Core types for nodes (entities) and edges (relationships).

use serde::{Deserialize, Serialize};

// =============================================================================
// Node Types
// =============================================================================

/// Node kinds (formerly entity kinds)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeKind {
    Character,
    Location,
    Item,
    Event,
    Concept,
    Organization,
    TimeUnit,
    Custom,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Character => "CHARACTER",
            Self::Location => "LOCATION",
            Self::Item => "ITEM",
            Self::Event => "EVENT",
            Self::Concept => "CONCEPT",
            Self::Organization => "ORGANIZATION",
            Self::TimeUnit => "TIME_UNIT",
            Self::Custom => "CUSTOM",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "CHARACTER" => Self::Character,
            "LOCATION" => Self::Location,
            "ITEM" => Self::Item,
            "EVENT" => Self::Event,
            "CONCEPT" => Self::Concept,
            "ORGANIZATION" => Self::Organization,
            "TIME_UNIT" => Self::TimeUnit,
            _ => Self::Custom,
        }
    }
}

/// A node in the graph (formerly entity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub normalized: String,
    pub kind: NodeKind,
    pub subtype: Option<String>,
    pub source_note: String,
    pub created_at: f64,
    pub created_by: CreatedBy,
    pub mention_count: i64,
    pub aliases: Vec<String>,
    pub metadata: Option<serde_json::Value>,
}

/// How a node was created
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CreatedBy {
    User,
    Extraction,
    Auto,
}

impl CreatedBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Extraction => "extraction",
            Self::Auto => "auto",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" => Self::User,
            "extraction" => Self::Extraction,
            _ => Self::Auto,
        }
    }
}

/// Input for registering a new node
#[derive(Debug, Clone)]
pub struct NodeInput {
    pub label: String,
    pub kind: NodeKind,
    pub source_note: String,
    pub subtype: Option<String>,
    pub aliases: Vec<String>,
    pub created_by: CreatedBy,
    pub metadata: Option<serde_json::Value>,
}

/// Filters for querying nodes
#[derive(Debug, Clone, Default)]
pub struct NodeFilter {
    pub kind: Option<NodeKind>,
    pub subtype: Option<String>,
    pub min_mentions: Option<i64>,
    pub label_contains: Option<String>,
}

/// Updates to apply to a node
#[derive(Debug, Clone, Default)]
pub struct NodeUpdate {
    pub label: Option<String>,
    pub kind: Option<NodeKind>,
    pub subtype: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// =============================================================================
// Edge Types
// =============================================================================

/// An edge in the graph (formerly relationship)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub inverse_type: Option<String>,
    pub bidirectional: bool,
    pub weight: f64,
    pub confidence: f64,
    pub created_at: f64,
    pub created_by: CreatedBy,
    pub source_note: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Input for creating a new edge
#[derive(Debug, Clone)]
pub struct EdgeInput {
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub inverse_type: Option<String>,
    pub bidirectional: bool,
    pub weight: f64,
    pub confidence: f64,
    pub created_by: CreatedBy,
    pub source_note: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Updates to apply to an edge
#[derive(Debug, Clone, Default)]
pub struct EdgeUpdate {
    pub weight: Option<f64>,
    pub confidence: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

/// Direction for edge queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Outgoing,
    Incoming,
    Both,
}

// =============================================================================
// Edge Type Definitions (Schema)
// =============================================================================

/// Definition of an edge type (schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTypeDef {
    pub type_name: String,
    pub inverse_name: Option<String>,
    pub source_kinds: Vec<NodeKind>,
    pub target_kinds: Vec<NodeKind>,
    pub bidirectional: bool,
    pub color: Option<String>,
    pub description: Option<String>,
}

// =============================================================================
// Results & Errors
// =============================================================================

/// Result of registering a node
#[derive(Debug, Clone)]
pub struct NodeRegistrationResult {
    pub node: Node,
    pub is_new: bool,
    pub was_merged: bool,
}

/// Result of batch operations
#[derive(Debug, Clone, Default)]
pub struct BatchResult {
    pub nodes_created: usize,
    pub nodes_updated: usize,
    pub edges_created: usize,
    pub edges_updated: usize,
}

/// Graph registry errors
#[derive(Debug, Clone)]
pub enum GraphError {
    NotInitialized,
    NodeNotFound(String),
    EdgeNotFound(String),
    DuplicateNode(String),
    InvalidEdge(String),
    QueryError(String),
    StorageError(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "GraphRegistry not initialized"),
            Self::NodeNotFound(id) => write!(f, "Node not found: {}", id),
            Self::EdgeNotFound(id) => write!(f, "Edge not found: {}", id),
            Self::DuplicateNode(label) => write!(f, "Duplicate node: {}", label),
            Self::InvalidEdge(msg) => write!(f, "Invalid edge: {}", msg),
            Self::QueryError(msg) => write!(f, "Query error: {}", msg),
            Self::StorageError(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

impl std::error::Error for GraphError {}
