//! Entity Tauri Commands
//!
//! Exposes entity CRUD operations to the frontend.

use crate::repos::EntityRepo;
use crate::surreal::SurrealConnection;
use crate::surreal::types::{Entity, EntityInput, EntityUpdate};

/// Create a new entity
#[tauri::command]
pub async fn surreal_create_entity(
    world_id: String,
    label: String,
    entity_kind: String,
    entity_subtype: Option<String>,
    note_id: Option<String>,
    folder_id: Option<String>,
    aliases: Option<Vec<String>>,
    attributes: Option<serde_json::Value>,
) -> Result<Entity, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
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
    
    EntityRepo::create(&db, input)
        .await
        .map_err(|e| e.to_string())
}

/// Get entity by ID
#[tauri::command]
pub async fn surreal_get_entity(
    world_id: String,
    id: String,
) -> Result<Option<Entity>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    EntityRepo::get(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Get entity by note ID
#[tauri::command]
pub async fn surreal_get_entity_by_note(
    world_id: String,
    note_id: String,
) -> Result<Option<Entity>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    EntityRepo::get_by_note(&db, &world_id, &note_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get entity by label
#[tauri::command]
pub async fn surreal_get_entity_by_label(
    world_id: String,
    label: String,
) -> Result<Option<Entity>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    EntityRepo::get_by_label(&db, &world_id, &label)
        .await
        .map_err(|e| e.to_string())
}

/// Update entity
#[tauri::command]
pub async fn surreal_update_entity(
    world_id: String,
    id: String,
    label: Option<String>,
    entity_kind: Option<String>,
    entity_subtype: Option<String>,
    is_active: Option<bool>,
    aliases: Option<Vec<String>>,
    attributes: Option<serde_json::Value>,
) -> Result<Entity, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let update = EntityUpdate {
        label,
        entity_kind,
        entity_subtype,
        is_active,
        aliases,
        attributes,
    };
    
    EntityRepo::update(&db, &world_id, &id, update)
        .await
        .map_err(|e| e.to_string())
}

/// Delete entity
#[tauri::command]
pub async fn surreal_delete_entity(
    world_id: String,
    id: String,
) -> Result<(), String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    EntityRepo::delete(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// List entities by kind
#[tauri::command]
pub async fn surreal_list_entities_by_kind(
    world_id: String,
    entity_kind: String,
) -> Result<Vec<Entity>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    EntityRepo::list_by_kind(&db, &world_id, &entity_kind)
        .await
        .map_err(|e| e.to_string())
}

/// List all entities
#[tauri::command]
pub async fn surreal_list_entities(
    world_id: String,
) -> Result<Vec<Entity>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    EntityRepo::list(&db, &world_id)
        .await
        .map_err(|e| e.to_string())
}

/// Search entities
#[tauri::command]
pub async fn surreal_search_entities(
    world_id: String,
    query: String,
) -> Result<Vec<Entity>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    EntityRepo::search(&db, &world_id, &query)
        .await
        .map_err(|e| e.to_string())
}

/// Link entity to note
#[tauri::command]
pub async fn surreal_link_entity_to_note(
    world_id: String,
    entity_id: String,
    note_id: String,
) -> Result<Entity, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    EntityRepo::link_to_note(&db, &world_id, &entity_id, &note_id)
        .await
        .map_err(|e| e.to_string())
}

/// Link entity to folder
#[tauri::command]
pub async fn surreal_link_entity_to_folder(
    world_id: String,
    entity_id: String,
    folder_id: String,
) -> Result<Entity, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    EntityRepo::link_to_folder(&db, &world_id, &entity_id, &folder_id)
        .await
        .map_err(|e| e.to_string())
}
