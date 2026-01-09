//! Folder Tauri Commands
//!
//! Exposes folder CRUD operations including rename

use crate::repos::FolderRepo;
use crate::surreal::types::{FolderInput, FolderResponse, FolderTreeNodeResponse};
use crate::surreal::SurrealConnection;

/// Create a new folder
#[tauri::command]
pub async fn surreal_create_folder(
    world_id: String,
    name: String,
    parent_id: Option<String>,
    entity_kind: Option<String>,
    entity_subtype: Option<String>,
    color: Option<String>,
) -> Result<FolderResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let input = FolderInput {
        world_id,
        name,
        parent_id,
        entity_kind,
        entity_subtype,
        color,
        is_typed_root: None,
    };

    FolderRepo::create(&db, input)
        .await
        .map(|f| f.into())
        .map_err(|e| e.to_string())
}

/// Get a folder by ID
#[tauri::command]
pub async fn surreal_get_folder(world_id: String, id: String) -> Result<FolderResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    FolderRepo::get(&db, &world_id, &id)
        .await
        .map(|f| f.into())
        .map_err(|e| e.to_string())
}

/// Rename a folder
#[tauri::command]
pub async fn surreal_rename_folder(world_id: String, id: String, new_name: String) -> Result<FolderResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    FolderRepo::rename(&db, &world_id, &id, new_name)
        .await
        .map(|f| f.into())
        .map_err(|e| e.to_string())
}

/// Move a folder to a new parent
#[tauri::command]
pub async fn surreal_move_folder(
    world_id: String,
    id: String,
    new_parent_id: Option<String>,
) -> Result<FolderResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    FolderRepo::move_folder(&db, &world_id, &id, new_parent_id)
        .await
        .map(|f| f.into())
        .map_err(|e| e.to_string())
}

/// Delete a folder
#[tauri::command]
pub async fn surreal_delete_folder(
    world_id: String,
    id: String,
    recursive: Option<bool>,
) -> Result<u32, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    FolderRepo::delete(&db, &world_id, &id, recursive.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

/// Get the folder tree
#[tauri::command]
pub async fn surreal_get_folder_tree(
    world_id: String,
    root_id: Option<String>,
) -> Result<Vec<FolderTreeNodeResponse>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    FolderRepo::get_tree(&db, &world_id, root_id)
        .await
        .map(|nodes| nodes.into_iter().map(|n| n.into()).collect())
        .map_err(|e| e.to_string())
}

/// Get root folders (no parent)
#[tauri::command]
pub async fn surreal_get_root_folders(world_id: String) -> Result<Vec<FolderResponse>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    FolderRepo::get_roots(&db, &world_id)
        .await
        .map(|folders| folders.into_iter().map(|f| f.into()).collect())
        .map_err(|e| e.to_string())
}

/// Get children of a folder
#[tauri::command]
pub async fn surreal_get_folder_children(
    world_id: String,
    parent_id: String,
) -> Result<Vec<FolderResponse>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    FolderRepo::get_children(&db, &world_id, &parent_id)
        .await
        .map(|folders| folders.into_iter().map(|f| f.into()).collect())
        .map_err(|e| e.to_string())
}
