//! Relationship Tauri Commands
//!
//! Exposes relationship CRUD operations to the frontend.
//! Supports auto-inverse creation.

use chrono::{DateTime, Utc};
use crate::repos::RelationshipRepo;
use crate::surreal::SurrealConnection;
use crate::surreal::types::{RelatesTo, CreateRelationshipInput, RelationshipSummary};

/// Create a relationship between two entities
/// Auto-creates inverse relationship by default
#[tauri::command]
pub async fn surreal_create_relationship(
    world_id: String,
    source_id: String,
    target_id: String,
    relationship_code: String,
    network_id: Option<String>,
    strength: Option<f32>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    notes: Option<String>,
    attributes: Option<serde_json::Value>,
    create_inverse: Option<bool>,
) -> Result<RelatesTo, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
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
        create_inverse,
    };
    
    RelationshipRepo::create(&db, input)
        .await
        .map_err(|e| e.to_string())
}

/// Get relationship by ID
#[tauri::command]
pub async fn surreal_get_relationship(
    world_id: String,
    id: String,
) -> Result<Option<RelatesTo>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    RelationshipRepo::get(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete relationship (and its inverse)
#[tauri::command]
pub async fn surreal_delete_relationship(
    world_id: String,
    id: String,
) -> Result<(), String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    RelationshipRepo::delete(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all relationships for an entity
#[tauri::command]
pub async fn surreal_get_entity_relationships(
    world_id: String,
    entity_id: String,
) -> Result<Vec<RelatesTo>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    RelationshipRepo::get_by_entity(&db, &world_id, &entity_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all relationships in a network
#[tauri::command]
pub async fn surreal_get_network_relationships(
    world_id: String,
    network_id: String,
) -> Result<Vec<RelatesTo>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    RelationshipRepo::get_by_network(&db, &world_id, &network_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get relationships by code
#[tauri::command]
pub async fn surreal_get_relationships_by_code(
    world_id: String,
    relationship_code: String,
) -> Result<Vec<RelatesTo>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    RelationshipRepo::get_by_code(&db, &world_id, &relationship_code)
        .await
        .map_err(|e| e.to_string())
}

/// Get relationship summaries for a network (with entity labels)
#[tauri::command]
pub async fn surreal_get_network_relationship_summaries(
    world_id: String,
    network_id: String,
) -> Result<Vec<RelationshipSummary>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    RelationshipRepo::get_summaries_by_network(&db, &world_id, &network_id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete all relationships between two entities
#[tauri::command]
pub async fn surreal_delete_relationships_between(
    world_id: String,
    source_id: String,
    target_id: String,
    relationship_code: Option<String>,
) -> Result<i32, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    RelationshipRepo::delete_between(
        &db,
        &world_id, 
        &source_id, 
        &target_id,
        relationship_code.as_deref()
    )
        .await
        .map_err(|e| e.to_string())
}
