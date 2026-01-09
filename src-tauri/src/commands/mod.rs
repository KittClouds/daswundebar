//! Tauri Commands Module
//!
//! Exposes SurrealDB operations to the frontend

// Phase 1
pub mod folder_commands;
pub mod note_commands;
pub mod surreal_commands;

// Phase 2
pub mod network_commands;
pub mod entity_commands;
pub mod relationship_commands;

// Phase 3
pub mod calendar_commands;
pub mod period_commands;

// Phase 4
pub mod binding_commands;

pub use folder_commands::*;
pub use note_commands::*;
pub use surreal_commands::*;
pub use network_commands::*;
pub use entity_commands::*;
pub use relationship_commands::*;
pub use calendar_commands::*;
pub use period_commands::*;
pub use binding_commands::*;

