//! Network Tauri Commands
//!
//! Exposes network CRUD operations to the frontend.

use crate::repos::NetworkRepo;
use crate::surreal::SurrealConnection;
use crate::surreal::types::{
    Network, NetworkInput, NetworkUpdate, NetworkMemberSummary, AddMemberInput,
};

/// Create a new network
#[tauri::command]
pub async fn surreal_create_network(
    world_id: String,
    name: String,
    schema_id: String,
    root_folder_id: Option<String>,
    root_entity_id: Option<String>,
    namespace: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Network, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
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
    
    NetworkRepo::create(&db, input)
        .await
        .map_err(|e| e.to_string())
}

/// Get a network by ID
#[tauri::command]
pub async fn surreal_get_network(
    world_id: String,
    id: String,
) -> Result<Option<Network>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NetworkRepo::get(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Get network by folder ID
#[tauri::command]
pub async fn surreal_get_network_by_folder(
    world_id: String,
    folder_id: String,
) -> Result<Option<Network>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NetworkRepo::get_by_folder(&db, &world_id, &folder_id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a network
#[tauri::command]
pub async fn surreal_update_network(
    world_id: String,
    id: String,
    name: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Network, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let update = NetworkUpdate {
        name,
        description,
        tags,
        member_count: None,
        relationship_count: None,
        max_depth: None,
    };
    
    NetworkRepo::update(&db, &world_id, &id, update)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a network
#[tauri::command]
pub async fn surreal_delete_network(
    world_id: String,
    id: String,
) -> Result<(), String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NetworkRepo::delete(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// List all networks
#[tauri::command]
pub async fn surreal_list_networks(
    world_id: String,
) -> Result<Vec<Network>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NetworkRepo::list(&db, &world_id)
        .await
        .map_err(|e| e.to_string())
}

/// Add a member to a network
#[tauri::command]
pub async fn surreal_add_network_member(
    world_id: String,
    network_id: String,
    entity_id: String,
    role: Option<String>,
    depth_level: Option<i32>,
    group_id: Option<String>,
) -> Result<(), String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let input = AddMemberInput {
        world_id,
        network_id,
        entity_id,
        role,
        depth_level,
        group_id,
    };
    
    NetworkRepo::add_member(&db, input)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Remove a member from a network
#[tauri::command]
pub async fn surreal_remove_network_member(
    world_id: String,
    network_id: String,
    entity_id: String,
) -> Result<(), String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NetworkRepo::remove_member(&db, &world_id, &network_id, &entity_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all members of a network
#[tauri::command]
pub async fn surreal_get_network_members(
    world_id: String,
    network_id: String,
) -> Result<Vec<NetworkMemberSummary>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NetworkRepo::get_members(&db, &world_id, &network_id)
        .await
        .map_err(|e| e.to_string())
}
