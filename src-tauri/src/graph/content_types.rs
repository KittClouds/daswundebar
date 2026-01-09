//! Content Types - Clean Rust types for CozoDB storage (SurrealDB replacement)
//!
//! All IDs are plain String UUIDs, no RecordId nonsense.

use serde::{Deserialize, Serialize};

// =============================================================================
// NOTE
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub world_id: String,
    pub title: String,
    pub content: String,
    pub folder_id: Option<String>,
    pub entity_kind: Option<String>,
    pub entity_subtype: Option<String>,
    pub is_entity: bool,
    pub is_pinned: bool,
    pub favorite: bool,
    pub created_at: f64,
    pub updated_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NoteInput {
    pub world_id: String,
    pub title: String,
    pub content: Option<String>,
    pub folder_id: Option<String>,
    pub entity_kind: Option<String>,
    pub entity_subtype: Option<String>,
    pub is_entity: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NoteUpdate {
    pub title: Option<String>,
    pub content: Option<String>,
    pub folder_id: Option<String>,
    pub entity_kind: Option<String>,
    pub entity_subtype: Option<String>,
    pub is_entity: Option<bool>,
    pub is_pinned: Option<bool>,
    pub favorite: Option<bool>,
}

// =============================================================================
// FOLDER
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub entity_kind: Option<String>,
    pub entity_subtype: Option<String>,
    pub color: Option<String>,
    pub is_typed_root: bool,
    pub network_id: Option<String>,
    pub collapsed: bool,
    pub fantasy_year: Option<i32>,
    pub fantasy_month: Option<i32>,
    pub fantasy_day: Option<i32>,
    pub created_at: f64,
    pub updated_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FolderInput {
    pub world_id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub entity_kind: Option<String>,
    pub entity_subtype: Option<String>,
    pub color: Option<String>,
    pub is_typed_root: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FolderUpdate {
    pub name: Option<String>,
    pub parent_id: Option<String>,
    pub entity_kind: Option<String>,
    pub entity_subtype: Option<String>,
    pub color: Option<String>,
    pub collapsed: Option<bool>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderTreeNode {
    pub folder: Folder,
    pub children: Vec<FolderTreeNode>,
    pub notes: Vec<NoteSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub is_pinned: bool,
    pub favorite: bool,
    pub entity_kind: Option<String>,
    pub updated_at: f64,
}

// =============================================================================
// NETWORK
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub schema_id: String,
    pub root_folder_id: Option<String>,
    pub root_entity_id: Option<String>,
    pub namespace: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub member_count: i32,
    pub relationship_count: i32,
    pub max_depth: i32,
    pub created_at: f64,
    pub updated_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInput {
    pub world_id: String,
    pub name: String,
    pub schema_id: String,
    pub root_folder_id: Option<String>,
    pub root_entity_id: Option<String>,
    pub namespace: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

// =============================================================================
// ENTITY (Domain entity, separate from knowledge graph nodes)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub world_id: String,
    pub label: String,
    pub entity_kind: String,
    pub entity_subtype: Option<String>,
    pub note_id: Option<String>,
    pub folder_id: Option<String>,
    pub is_active: bool,
    pub aliases: Vec<String>,
    pub attributes: serde_json::Value,
    pub created_at: f64,
    pub updated_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityInput {
    pub world_id: String,
    pub label: String,
    pub entity_kind: String,
    pub entity_subtype: Option<String>,
    pub note_id: Option<String>,
    pub folder_id: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub attributes: Option<serde_json::Value>,
}

// =============================================================================
// RELATIONSHIPS
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub world_id: String,
    pub source_id: String,
    pub target_id: String,
    pub network_id: Option<String>,
    pub relationship_code: String,
    pub inverse_id: Option<String>,
    pub strength: f32,
    pub start_date: Option<f64>,
    pub end_date: Option<f64>,
    pub notes: Option<String>,
    pub attributes: serde_json::Value,
    pub created_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateRelationshipInput {
    pub world_id: String,
    pub source_id: String,
    pub target_id: String,
    pub network_id: Option<String>,
    pub relationship_code: String,
    pub strength: Option<f32>,
    pub start_date: Option<f64>,
    pub end_date: Option<f64>,
    pub notes: Option<String>,
    pub attributes: Option<serde_json::Value>,
    pub create_inverse: Option<bool>,
}

// =============================================================================
// CALENDAR EVENTS
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalEvent {
    pub id: String,
    pub world_id: String,
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub date_year: i32,
    pub date_month: i32,
    pub date_day: i32,
    pub date_hour: Option<i32>,
    pub date_minute: Option<i32>,
    pub era_id: Option<String>,
    pub end_year: Option<i32>,
    pub end_month: Option<i32>,
    pub end_day: Option<i32>,
    pub is_all_day: bool,
    pub recurrence: Option<serde_json::Value>,
    pub parent_event_id: Option<String>,
    pub importance: String,
    pub category: String,
    pub tags: Vec<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub entity_id: Option<String>,
    pub entity_kind: Option<String>,
    pub source_note_id: Option<String>,
    pub created_at: f64,
    pub updated_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalEventInput {
    pub world_id: String,
    pub calendar_id: String,
    pub title: String,
    pub description: Option<String>,
    pub date_year: i32,
    pub date_month: i32,
    pub date_day: i32,
    pub date_hour: Option<i32>,
    pub date_minute: Option<i32>,
    pub era_id: Option<String>,
    pub end_year: Option<i32>,
    pub end_month: Option<i32>,
    pub end_day: Option<i32>,
    pub is_all_day: Option<bool>,
    pub recurrence: Option<serde_json::Value>,
    pub importance: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub entity_id: Option<String>,
    pub entity_kind: Option<String>,
    pub source_note_id: Option<String>,
}

// =============================================================================
// PERIODS
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Period {
    pub id: String,
    pub world_id: String,
    pub calendar_id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_year: i32,
    pub start_month: Option<i32>,
    pub end_year: Option<i32>,
    pub end_month: Option<i32>,
    pub parent_period_id: Option<String>,
    pub period_type: String,
    pub color: String,
    pub icon: Option<String>,
    pub abbreviation: Option<String>,
    pub direction: String,
    pub triggered_by: Option<String>,
    pub ends_when: Option<String>,
    pub major_events: Vec<String>,
    pub arc_type: Option<String>,
    pub dominant_theme: Option<String>,
    pub protagonist_id: Option<String>,
    pub antagonist_id: Option<String>,
    pub summary: Option<String>,
    pub detailed_notes: Option<String>,
    pub show_on_timeline: bool,
    pub timeline_color: Option<String>,
    pub timeline_icon: Option<String>,
    pub created_at: f64,
    pub updated_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeriodInput {
    pub world_id: String,
    pub calendar_id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_year: i32,
    pub start_month: Option<i32>,
    pub end_year: Option<i32>,
    pub end_month: Option<i32>,
    pub parent_period_id: Option<String>,
    pub period_type: Option<String>,
    pub color: String,
    pub icon: Option<String>,
    pub abbreviation: Option<String>,
    pub direction: Option<String>,
    pub triggered_by: Option<String>,
    pub ends_when: Option<String>,
    pub arc_type: Option<String>,
    pub dominant_theme: Option<String>,
    pub protagonist_id: Option<String>,
    pub antagonist_id: Option<String>,
    pub summary: Option<String>,
    pub detailed_notes: Option<String>,
    pub show_on_timeline: Option<bool>,
    pub timeline_color: Option<String>,
    pub timeline_icon: Option<String>,
}

// =============================================================================
// FIELD BINDINGS
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BindingType {
    Mirror,
    Inherit,
    Aggregate,
}

impl Default for BindingType {
    fn default() -> Self {
        Self::Inherit
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AggregationFunction {
    Sum,
    Avg,
    Min,
    Max,
    Count,
    Concat,
    First,
    Last,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldBinding {
    pub id: String,
    pub world_id: String,
    pub source_entity_id: String,
    pub source_field_name: String,
    pub target_entity_id: String,
    pub target_field_name: String,
    pub binding_type: BindingType,
    pub transform: Option<serde_json::Value>,
    pub aggregation_fn: Option<AggregationFunction>,
    pub aggregation_filter: Option<serde_json::Value>,
    pub allow_override: bool,
    pub is_active: bool,
    pub created_at: f64,
    pub updated_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldBindingInput {
    pub world_id: String,
    pub source_entity_id: String,
    pub source_field_name: String,
    pub target_entity_id: String,
    pub target_field_name: String,
    pub binding_type: BindingType,
    pub transform: Option<serde_json::Value>,
    pub aggregation_fn: Option<AggregationFunction>,
    pub aggregation_filter: Option<serde_json::Value>,
    pub allow_override: Option<bool>,
}
