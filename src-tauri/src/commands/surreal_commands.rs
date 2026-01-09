//! SurrealDB Initialization Commands

use crate::surreal::SurrealConnection;

/// Initialize SurrealDB connection
#[tauri::command]
pub async fn init_surreal_db() -> Result<String, String> {
    SurrealConnection::init()
        .await
        .map_err(|e| e.to_string())?;
    
    Ok("SurrealDB initialized".to_string())
}

/// Check if SurrealDB is ready
#[tauri::command]
pub fn is_surreal_ready() -> bool {
    SurrealConnection::is_initialized()
}

/// Shutdown SurrealDB connection
#[tauri::command]
pub async fn shutdown_surreal_db() -> Result<(), String> {
    SurrealConnection::shutdown()
        .await
        .map_err(|e| e.to_string())
}
