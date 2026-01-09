//! Note Tauri Commands
//!
//! Exposes note CRUD operations including rename

use crate::repos::NoteRepo;
use crate::surreal::types::{NoteInput, NoteResponse, NoteUpdate};
use crate::surreal::SurrealConnection;

/// Create a new note
#[tauri::command]
pub async fn surreal_create_note(
    world_id: String,
    title: String,
    folder_id: Option<String>,
    content: Option<String>,
    entity_kind: Option<String>,
    entity_subtype: Option<String>,
) -> Result<NoteResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let input = NoteInput {
        world_id,
        title,
        content,
        folder_id,
        entity_kind,
        entity_subtype,
        is_entity: None,
    };

    NoteRepo::create(&db, input)
        .await
        .map(|n| n.into())
        .map_err(|e| e.to_string())
}

/// Get a note by ID
#[tauri::command]
pub async fn surreal_get_note(world_id: String, id: String) -> Result<NoteResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NoteRepo::get(&db, &world_id, &id)
        .await
        .map(|n| n.into())
        .map_err(|e| e.to_string())
}

/// Rename a note (update title)
#[tauri::command]
pub async fn surreal_rename_note(world_id: String, id: String, new_title: String) -> Result<NoteResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NoteRepo::rename(&db, &world_id, &id, new_title)
        .await
        .map(|n| n.into())
        .map_err(|e| e.to_string())
}

/// Update note content
#[tauri::command]
pub async fn surreal_update_note_content(
    world_id: String,
    id: String,
    content: String,
) -> Result<NoteResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NoteRepo::update_content(&db, &world_id, &id, content)
        .await
        .map(|n| n.into())
        .map_err(|e| e.to_string())
}

/// Update note with partial data
#[tauri::command]
pub async fn surreal_update_note(
    world_id: String,
    id: String,
    title: Option<String>,
    content: Option<String>,
    is_pinned: Option<bool>,
    favorite: Option<bool>,
    entity_kind: Option<String>,
    is_entity: Option<bool>,
) -> Result<NoteResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let update = NoteUpdate {
        title,
        content,
        folder_id: None,
        entity_kind,
        entity_subtype: None,
        is_entity,
        is_pinned,
        favorite,
    };

    NoteRepo::update(&db, &world_id, &id, update)
        .await
        .map(|n| n.into())
        .map_err(|e| e.to_string())
}

/// Move a note to a different folder
#[tauri::command]
pub async fn surreal_move_note(
    world_id: String,
    id: String,
    folder_id: String,
) -> Result<NoteResponse, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NoteRepo::move_note(&db, &world_id, &id, folder_id)
        .await
        .map(|n| n.into())
        .map_err(|e| e.to_string())
}

/// Delete a note
#[tauri::command]
pub async fn surreal_delete_note(world_id: String, id: String) -> Result<bool, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NoteRepo::delete(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all notes in a folder
#[tauri::command]
pub async fn surreal_get_notes_by_folder(
    world_id: String,
    folder_id: String,
) -> Result<Vec<NoteResponse>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NoteRepo::get_by_folder(&db, &world_id, &folder_id)
        .await
        .map(|notes| notes.into_iter().map(|n| n.into()).collect())
        .map_err(|e| e.to_string())
}

/// Search notes by title
#[tauri::command]
pub async fn surreal_search_notes(world_id: String, query: String) -> Result<Vec<NoteResponse>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NoteRepo::search(&db, &world_id, &query)
        .await
        .map(|notes| notes.into_iter().map(|n| n.into()).collect())
        .map_err(|e| e.to_string())
}

/// List all notes in a world
#[tauri::command]
pub async fn surreal_list_notes(world_id: String) -> Result<Vec<NoteResponse>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    NoteRepo::list_all(&db, &world_id)
        .await
        .map(|notes| notes.into_iter().map(|n| n.into()).collect())
        .map_err(|e| e.to_string())
}
