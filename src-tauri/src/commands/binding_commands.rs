//! Binding Tauri Commands
//!
//! Tauri commands for field binding CRUD operations (Phase 4)

use crate::repos::BindingRepo;
use crate::surreal::types::{
    AggregationFilter, AggregationFunction, BindingTransform, BindingType, FieldBinding,
    FieldBindingInput, FieldBindingUpdate,
};
use crate::surreal::SurrealConnection;

// =============================================================================
// CREATE
// =============================================================================

#[tauri::command]
pub async fn surreal_create_binding(
    world_id: String,
    source_entity_id: String,
    source_field_name: String,
    target_entity_id: String,
    target_field_name: String,
    binding_type: String,
    transform: Option<BindingTransform>,
    aggregation_fn: Option<String>,
    aggregation_filter: Option<AggregationFilter>,
    allow_override: Option<bool>,
) -> Result<FieldBinding, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;

    let binding_type = match binding_type.as_str() {
        "mirror" => BindingType::Mirror,
        "inherit" => BindingType::Inherit,
        "aggregate" => BindingType::Aggregate,
        _ => return Err(format!("Invalid binding type: {}", binding_type)),
    };

    let aggregation_fn = aggregation_fn.and_then(|s| match s.as_str() {
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
        binding_type,
        transform,
        aggregation_fn,
        aggregation_filter,
        allow_override,
    };

    BindingRepo::create(&db, input)
        .await
        .map_err(|e| e.to_string())
}

// =============================================================================
// READ
// =============================================================================

#[tauri::command]
pub async fn surreal_get_binding(
    world_id: String,
    id: String,
) -> Result<FieldBinding, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    BindingRepo::get(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surreal_list_bindings(
    world_id: String,
) -> Result<Vec<FieldBinding>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    BindingRepo::list(&db, &world_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surreal_get_bindings_by_source(
    world_id: String,
    entity_id: String,
    field_name: String,
) -> Result<Vec<FieldBinding>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    BindingRepo::get_by_source(&db, &world_id, &entity_id, &field_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surreal_get_bindings_by_target(
    world_id: String,
    entity_id: String,
    field_name: String,
) -> Result<Vec<FieldBinding>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    BindingRepo::get_by_target(&db, &world_id, &entity_id, &field_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surreal_get_bindings_by_entity(
    world_id: String,
    entity_id: String,
) -> Result<Vec<FieldBinding>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    BindingRepo::get_by_entity(&db, &world_id, &entity_id)
        .await
        .map_err(|e| e.to_string())
}

// =============================================================================
// UPDATE
// =============================================================================

#[tauri::command]
pub async fn surreal_update_binding(
    world_id: String,
    id: String,
    transform: Option<BindingTransform>,
    aggregation_fn: Option<String>,
    aggregation_filter: Option<AggregationFilter>,
    allow_override: Option<bool>,
    is_active: Option<bool>,
) -> Result<FieldBinding, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;

    let aggregation_fn = aggregation_fn.and_then(|s| match s.as_str() {
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

    let update = FieldBindingUpdate {
        transform,
        aggregation_fn,
        aggregation_filter,
        allow_override,
        is_active,
    };

    BindingRepo::update(&db, &world_id, &id, update)
        .await
        .map_err(|e| e.to_string())
}

// =============================================================================
// DELETE
// =============================================================================

#[tauri::command]
pub async fn surreal_delete_binding(
    world_id: String,
    id: String,
) -> Result<bool, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    BindingRepo::delete(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surreal_delete_bindings_by_entity(
    world_id: String,
    entity_id: String,
) -> Result<usize, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    BindingRepo::delete_by_entity(&db, &world_id, &entity_id)
        .await
        .map_err(|e| e.to_string())
}
