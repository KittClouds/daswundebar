//! Period Tauri Commands
//!
//! Exposes period CRUD operations to the frontend.

use crate::repos::PeriodRepo;
use crate::surreal::SurrealConnection;
use crate::surreal::types::{Period, PeriodInput, PeriodUpdate};

/// Create a new period
#[tauri::command]
pub async fn surreal_create_period(
    world_id: String,
    calendar_id: String,
    name: String,
    description: Option<String>,
    start_year: i32,
    start_month: Option<i32>,
    end_year: Option<i32>,
    end_month: Option<i32>,
    parent_period_id: Option<String>,
    period_type: Option<String>,
    color: String,
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
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
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
    
    PeriodRepo::create(&db, input)
        .await
        .map_err(|e| e.to_string())
}

/// Get period by ID
#[tauri::command]
pub async fn surreal_get_period(
    world_id: String,
    id: String,
) -> Result<Option<Period>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    PeriodRepo::get(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Get root periods (no parent)
#[tauri::command]
pub async fn surreal_get_root_periods(
    world_id: String,
    calendar_id: String,
) -> Result<Vec<Period>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    PeriodRepo::get_roots(&db, &world_id, &calendar_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get children of a period
#[tauri::command]
pub async fn surreal_get_period_children(
    world_id: String,
    parent_id: String,
) -> Result<Vec<Period>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    PeriodRepo::get_children(&db, &world_id, &parent_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get periods containing a year
#[tauri::command]
pub async fn surreal_get_periods_by_year(
    world_id: String,
    calendar_id: String,
    year: i32,
) -> Result<Vec<Period>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    PeriodRepo::get_by_year(&db, &world_id, &calendar_id, year)
        .await
        .map_err(|e| e.to_string())
}

/// Update period
#[tauri::command]
pub async fn surreal_update_period(
    world_id: String,
    id: String,
    name: Option<String>,
    description: Option<String>,
    start_year: Option<i32>,
    start_month: Option<i32>,
    end_year: Option<i32>,
    end_month: Option<i32>,
    period_type: Option<String>,
    color: Option<String>,
    icon: Option<String>,
    abbreviation: Option<String>,
    direction: Option<String>,
    arc_type: Option<String>,
    dominant_theme: Option<String>,
    summary: Option<String>,
    detailed_notes: Option<String>,
    show_on_timeline: Option<bool>,
    timeline_color: Option<String>,
    timeline_icon: Option<String>,
) -> Result<Period, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let update = PeriodUpdate {
        name,
        description,
        start_year,
        start_month,
        end_year,
        end_month,
        period_type,
        color,
        icon,
        abbreviation,
        direction,
        arc_type,
        dominant_theme,
        summary,
        detailed_notes,
        show_on_timeline,
        timeline_color,
        timeline_icon,
    };
    
    PeriodRepo::update(&db, &world_id, &id, update)
        .await
        .map_err(|e| e.to_string())
}

/// Delete period
#[tauri::command]
pub async fn surreal_delete_period(
    world_id: String,
    id: String,
) -> Result<(), String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    PeriodRepo::delete(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// List all periods
#[tauri::command]
pub async fn surreal_list_periods(
    world_id: String,
    calendar_id: String,
) -> Result<Vec<Period>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    PeriodRepo::list(&db, &world_id, &calendar_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get periods by type
#[tauri::command]
pub async fn surreal_get_periods_by_type(
    world_id: String,
    calendar_id: String,
    period_type: String,
) -> Result<Vec<Period>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    PeriodRepo::get_by_type(&db, &world_id, &calendar_id, &period_type)
        .await
        .map_err(|e| e.to_string())
}
