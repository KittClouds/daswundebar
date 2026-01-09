//! Content Commands - Tauri commands for notes, folders, etc.
//!
//! These replace the SurrealDB commands with CozoDB-backed implementations.

use super::content_types::*;
use super::content_repos::{NoteRepo, FolderRepo, NetworkRepo, EntityRepo, RelationshipRepo, CalEventRepo, PeriodRepo, FieldBindingRepo};
use super::commands::GRAPH_REGISTRY;

// =============================================================================
// NOTE COMMANDS
// =============================================================================

/// Create a new note
#[tauri::command]
pub async fn cozo_create_note(
    world_id: String,
    title: String,
    content: Option<String>,
    folder_id: Option<String>,
    entity_kind: Option<String>,
    entity_subtype: Option<String>,
    is_entity: Option<bool>,
) -> Result<Note, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let input = NoteInput {
        world_id,
        title,
        content,
        folder_id,
        entity_kind,
        entity_subtype,
        is_entity,
    };
    
    NoteRepo::create(registry.db(), input)
}

/// Get a note by ID
#[tauri::command]
pub async fn cozo_get_note(world_id: String, id: String) -> Result<Option<Note>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    NoteRepo::get(registry.db(), &world_id, &id)
}

/// List all notes in a world
#[tauri::command]
pub async fn cozo_list_notes(world_id: String) -> Result<Vec<Note>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    NoteRepo::list_all(registry.db(), &world_id)
}

/// Update a note
#[tauri::command]
pub async fn cozo_update_note(
    world_id: String,
    id: String,
    title: Option<String>,
    content: Option<String>,
    folder_id: Option<String>,
    entity_kind: Option<String>,
    entity_subtype: Option<String>,
    is_entity: Option<bool>,
    is_pinned: Option<bool>,
    favorite: Option<bool>,
) -> Result<Note, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    // Check if content is changing BEFORE moving into struct
    let content_changed = content.is_some();
    
    let update = NoteUpdate {
        title,
        content,
        folder_id,
        entity_kind,
        entity_subtype,
        is_entity,
        is_pinned,
        favorite,
    };
    
    let note = NoteRepo::update(registry.db(), &world_id, &id, update)?;
    
    // Queue for background scan if content changed
    if content_changed {
        crate::SCAN_QUEUE.push(id.clone());
        log::debug!("[ContentCommands] Queued note {} for background scan", id);
    }
    
    Ok(note)
}

/// Delete a note
#[tauri::command]
pub async fn cozo_delete_note(world_id: String, id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    NoteRepo::delete(registry.db(), &world_id, &id)
}

// =============================================================================
// FOLDER COMMANDS
// =============================================================================

/// Create a new folder
#[tauri::command]
pub async fn cozo_create_folder(
    world_id: String,
    name: String,
    parent_id: Option<String>,
    entity_kind: Option<String>,
    entity_subtype: Option<String>,
    color: Option<String>,
    is_typed_root: Option<bool>,
) -> Result<Folder, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let input = FolderInput {
        world_id,
        name,
        parent_id,
        entity_kind,
        entity_subtype,
        color,
        is_typed_root,
    };
    
    FolderRepo::create(registry.db(), input)
}

/// Get a folder by ID
#[tauri::command]
pub async fn cozo_get_folder(world_id: String, id: String) -> Result<Option<Folder>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    FolderRepo::get(registry.db(), &world_id, &id)
}

/// List all folders in a world
#[tauri::command]
pub async fn cozo_list_folders(world_id: String) -> Result<Vec<Folder>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    FolderRepo::list_all(registry.db(), &world_id)
}

/// Get folder tree
#[tauri::command]
pub async fn cozo_get_folder_tree(world_id: String) -> Result<Vec<FolderTreeNode>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    FolderRepo::get_tree(registry.db(), &world_id)
}

/// Update a folder (rename, move, etc.)
#[tauri::command]
pub async fn cozo_update_folder(
    world_id: String,
    id: String,
    name: Option<String>,
    parent_id: Option<String>,
    entity_kind: Option<String>,
    entity_subtype: Option<String>,
    color: Option<String>,
    collapsed: Option<bool>,
) -> Result<Folder, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let update = FolderUpdate {
        name,
        parent_id,
        entity_kind,
        entity_subtype,
        color,
        collapsed,
    };
    
    FolderRepo::update(registry.db(), &world_id, &id, update)
}

/// Delete a folder
#[tauri::command]
pub async fn cozo_delete_folder(world_id: String, id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    FolderRepo::delete(registry.db(), &world_id, &id)
}

// =============================================================================
// NETWORK COMMANDS
// =============================================================================

/// Create a new network
#[tauri::command]
pub async fn cozo_create_network(
    world_id: String,
    name: String,
    schema_id: String,
    root_folder_id: Option<String>,
    root_entity_id: Option<String>,
    namespace: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Network, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let input = NetworkInput {
        world_id,
        name,
        schema_id,
        root_folder_id,
        root_entity_id,
        namespace,
        description,
        tags,
    };
    
    NetworkRepo::create(registry.db(), input)
}

/// Get a network by ID
#[tauri::command]
pub async fn cozo_get_network(world_id: String, id: String) -> Result<Option<Network>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    NetworkRepo::get(registry.db(), &world_id, &id)
}

/// List all networks
#[tauri::command]
pub async fn cozo_list_networks(world_id: String) -> Result<Vec<Network>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    NetworkRepo::list_all(registry.db(), &world_id)
}

/// Delete a network
#[tauri::command]
pub async fn cozo_delete_network(world_id: String, id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    NetworkRepo::delete(registry.db(), &world_id, &id)
}

// =============================================================================
// ENTITY COMMANDS (Domain entities)
// =============================================================================

/// Create a new entity
#[tauri::command]
pub async fn cozo_create_entity(
    world_id: String,
    label: String,
    entity_kind: String,
    entity_subtype: Option<String>,
    note_id: Option<String>,
    folder_id: Option<String>,
    aliases: Option<Vec<String>>,
    attributes: Option<serde_json::Value>,
) -> Result<Entity, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let input = EntityInput {
        world_id,
        label,
        entity_kind,
        entity_subtype,
        note_id,
        folder_id,
        aliases,
        attributes,
    };
    
    EntityRepo::create(registry.db(), input)
}

/// Get an entity by ID
#[tauri::command]
pub async fn cozo_get_entity(world_id: String, id: String) -> Result<Option<Entity>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    EntityRepo::get(registry.db(), &world_id, &id)
}

/// List entities by kind
#[tauri::command]
pub async fn cozo_list_entities_by_kind(world_id: String, kind: String) -> Result<Vec<Entity>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    EntityRepo::list_by_kind(registry.db(), &world_id, &kind)
}

/// List all entities
#[tauri::command]
pub async fn cozo_list_entities(world_id: String) -> Result<Vec<Entity>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    EntityRepo::list_all(registry.db(), &world_id)
}

/// Delete an entity
#[tauri::command]
pub async fn cozo_delete_entity(world_id: String, id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    EntityRepo::delete(registry.db(), &world_id, &id)
}

// =============================================================================
// RELATIONSHIP COMMANDS
// =============================================================================

/// Create a relationship
#[tauri::command]
pub async fn cozo_create_relationship(
    world_id: String,
    source_id: String,
    target_id: String,
    relationship_code: String,
    network_id: Option<String>,
    strength: Option<f32>,
    start_date: Option<f64>,
    end_date: Option<f64>,
    notes: Option<String>,
    attributes: Option<serde_json::Value>,
) -> Result<Relationship, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let input = CreateRelationshipInput {
        world_id,
        source_id,
        target_id,
        network_id,
        relationship_code,
        strength,
        start_date,
        end_date,
        notes,
        attributes,
        create_inverse: None,
    };
    
    RelationshipRepo::create(registry.db(), input)
}

/// Get a relationship by ID
#[tauri::command]
pub async fn cozo_get_relationship(world_id: String, id: String) -> Result<Option<Relationship>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    RelationshipRepo::get(registry.db(), &world_id, &id)
}

/// Get relationships for an entity
#[tauri::command]
pub async fn cozo_get_entity_relationships(world_id: String, entity_id: String) -> Result<Vec<Relationship>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    RelationshipRepo::get_for_entity(registry.db(), &world_id, &entity_id)
}

/// Delete a relationship
#[tauri::command]
pub async fn cozo_delete_relationship(world_id: String, id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    RelationshipRepo::delete(registry.db(), &world_id, &id)
}

// =============================================================================
// CALENDAR EVENT COMMANDS
// =============================================================================

/// Create a calendar event
#[tauri::command]
pub async fn cozo_create_cal_event(
    world_id: String,
    calendar_id: String,
    title: String,
    date_year: i32,
    date_month: i32,
    date_day: i32,
    description: Option<String>,
    date_hour: Option<i32>,
    date_minute: Option<i32>,
    era_id: Option<String>,
    end_year: Option<i32>,
    end_month: Option<i32>,
    end_day: Option<i32>,
    is_all_day: Option<bool>,
    importance: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    color: Option<String>,
    icon: Option<String>,
    entity_id: Option<String>,
    entity_kind: Option<String>,
    source_note_id: Option<String>,
) -> Result<CalEvent, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let input = CalEventInput {
        world_id,
        calendar_id,
        title,
        description,
        date_year,
        date_month,
        date_day,
        date_hour,
        date_minute,
        era_id,
        end_year,
        end_month,
        end_day,
        is_all_day,
        recurrence: None,
        importance,
        category,
        tags,
        color,
        icon,
        entity_id,
        entity_kind,
        source_note_id,
    };
    
    CalEventRepo::create(registry.db(), input)
}

/// Get a calendar event by ID
#[tauri::command]
pub async fn cozo_get_cal_event(world_id: String, id: String) -> Result<Option<CalEvent>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    CalEventRepo::get(registry.db(), &world_id, &id)
}

/// List all calendar events
#[tauri::command]
pub async fn cozo_list_cal_events(world_id: String) -> Result<Vec<CalEvent>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    CalEventRepo::list_all(registry.db(), &world_id)
}

/// List calendar events by month
#[tauri::command]
pub async fn cozo_list_cal_events_by_month(world_id: String, year: i32, month: i32) -> Result<Vec<CalEvent>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    CalEventRepo::list_by_month(registry.db(), &world_id, year, month)
}

/// Delete a calendar event
#[tauri::command]
pub async fn cozo_delete_cal_event(world_id: String, id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    CalEventRepo::delete(registry.db(), &world_id, &id)
}

// =============================================================================
// PERIOD COMMANDS
// =============================================================================

/// Create a period
#[tauri::command]
pub async fn cozo_create_period(
    world_id: String,
    calendar_id: String,
    name: String,
    start_year: i32,
    color: String,
    description: Option<String>,
    start_month: Option<i32>,
    end_year: Option<i32>,
    end_month: Option<i32>,
    parent_period_id: Option<String>,
    period_type: Option<String>,
    icon: Option<String>,
    abbreviation: Option<String>,
    direction: Option<String>,
    triggered_by: Option<String>,
    ends_when: Option<String>,
    arc_type: Option<String>,
    dominant_theme: Option<String>,
    protagonist_id: Option<String>,
    antagonist_id: Option<String>,
    summary: Option<String>,
    detailed_notes: Option<String>,
    show_on_timeline: Option<bool>,
    timeline_color: Option<String>,
    timeline_icon: Option<String>,
) -> Result<Period, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let input = PeriodInput {
        world_id,
        calendar_id,
        name,
        description,
        start_year,
        start_month,
        end_year,
        end_month,
        parent_period_id,
        period_type,
        color,
        icon,
        abbreviation,
        direction,
        triggered_by,
        ends_when,
        arc_type,
        dominant_theme,
        protagonist_id,
        antagonist_id,
        summary,
        detailed_notes,
        show_on_timeline,
        timeline_color,
        timeline_icon,
    };
    
    PeriodRepo::create(registry.db(), input)
}

/// Get a period by ID
#[tauri::command]
pub async fn cozo_get_period(world_id: String, id: String) -> Result<Option<Period>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    PeriodRepo::get(registry.db(), &world_id, &id)
}

/// List all periods
#[tauri::command]
pub async fn cozo_list_periods(world_id: String) -> Result<Vec<Period>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    PeriodRepo::list_all(registry.db(), &world_id)
}

/// Get children of a period
#[tauri::command]
pub async fn cozo_get_period_children(world_id: String, parent_id: String) -> Result<Vec<Period>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    PeriodRepo::get_children(registry.db(), &world_id, &parent_id)
}

/// Delete a period
#[tauri::command]
pub async fn cozo_delete_period(world_id: String, id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    PeriodRepo::delete(registry.db(), &world_id, &id)
}

// =============================================================================
// FIELD BINDING COMMANDS
// =============================================================================

/// Create a field binding
#[tauri::command]
pub async fn cozo_create_binding(
    world_id: String,
    source_entity_id: String,
    source_field_name: String,
    target_entity_id: String,
    target_field_name: String,
    binding_type: String,
    transform: Option<serde_json::Value>,
    aggregation_fn: Option<String>,
    aggregation_filter: Option<serde_json::Value>,
    allow_override: Option<bool>,
) -> Result<FieldBinding, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    let bt = match binding_type.as_str() {
        "mirror" => BindingType::Mirror,
        "aggregate" => BindingType::Aggregate,
        _ => BindingType::Inherit,
    };
    
    let agg = aggregation_fn.as_ref().and_then(|s| match s.as_str() {
        "sum" => Some(AggregationFunction::Sum),
        "avg" => Some(AggregationFunction::Avg),
        "min" => Some(AggregationFunction::Min),
        "max" => Some(AggregationFunction::Max),
        "count" => Some(AggregationFunction::Count),
        "concat" => Some(AggregationFunction::Concat),
        "first" => Some(AggregationFunction::First),
        "last" => Some(AggregationFunction::Last),
        _ => None,
    });
    
    let input = FieldBindingInput {
        world_id,
        source_entity_id,
        source_field_name,
        target_entity_id,
        target_field_name,
        binding_type: bt,
        transform,
        aggregation_fn: agg,
        aggregation_filter,
        allow_override,
    };
    
    FieldBindingRepo::create(registry.db(), input)
}

/// Get a binding by ID
#[tauri::command]
pub async fn cozo_get_binding(world_id: String, id: String) -> Result<Option<FieldBinding>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    FieldBindingRepo::get(registry.db(), &world_id, &id)
}

/// List all bindings
#[tauri::command]
pub async fn cozo_list_bindings(world_id: String) -> Result<Vec<FieldBinding>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    FieldBindingRepo::list_all(registry.db(), &world_id)
}

/// List bindings for an entity
#[tauri::command]
pub async fn cozo_list_bindings_by_entity(world_id: String, entity_id: String) -> Result<Vec<FieldBinding>, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    FieldBindingRepo::list_by_entity(registry.db(), &world_id, &entity_id)
}

/// Delete a binding
#[tauri::command]
pub async fn cozo_delete_binding(world_id: String, id: String) -> Result<bool, String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    FieldBindingRepo::delete(registry.db(), &world_id, &id)
}
