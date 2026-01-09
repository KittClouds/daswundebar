//! Calendar Tauri Commands
//!
//! Exposes calendar event CRUD operations to the frontend.

use crate::repos::CalendarRepo;
use crate::surreal::SurrealConnection;
use crate::surreal::types::{
    CalEvent, CalEventInput, CalEventUpdate, DateRange, OccursOn, LinkEntityToEventInput,
};

/// Create a new calendar event
#[tauri::command]
pub async fn surreal_create_cal_event(
    world_id: String,
    calendar_id: String,
    title: String,
    description: Option<String>,
    date_year: i32,
    date_month: i32,
    date_day: i32,
    date_hour: Option<i32>,
    date_minute: Option<i32>,
    era_id: Option<String>,
    end_year: Option<i32>,
    end_month: Option<i32>,
    end_day: Option<i32>,
    is_all_day: Option<bool>,
    recurrence: Option<serde_json::Value>,
    importance: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    color: Option<String>,
    icon: Option<String>,
    entity_id: Option<String>,
    entity_kind: Option<String>,
    source_note_id: Option<String>,
) -> Result<CalEvent, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
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
        recurrence,
        importance,
        category,
        tags,
        color,
        icon,
        entity_id,
        entity_kind,
        source_note_id,
    };
    
    CalendarRepo::create(&db, input)
        .await
        .map_err(|e| e.to_string())
}

/// Get calendar event by ID
#[tauri::command]
pub async fn surreal_get_cal_event(
    world_id: String,
    id: String,
) -> Result<Option<CalEvent>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    CalendarRepo::get(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Get events in a date range
#[tauri::command]
pub async fn surreal_get_cal_events_range(
    world_id: String,
    calendar_id: String,
    start_year: i32,
    start_month: Option<i32>,
    start_day: Option<i32>,
    end_year: i32,
    end_month: Option<i32>,
    end_day: Option<i32>,
) -> Result<Vec<CalEvent>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let range = DateRange {
        start_year,
        start_month,
        start_day,
        end_year,
        end_month,
        end_day,
    };
    
    CalendarRepo::get_range(&db, &world_id, &calendar_id, range)
        .await
        .map_err(|e| e.to_string())
}

/// Get events for a specific month
#[tauri::command]
pub async fn surreal_get_cal_events_by_month(
    world_id: String,
    calendar_id: String,
    year: i32,
    month: i32,
) -> Result<Vec<CalEvent>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    CalendarRepo::get_by_month(&db, &world_id, &calendar_id, year, month)
        .await
        .map_err(|e| e.to_string())
}

/// Update calendar event
#[tauri::command]
pub async fn surreal_update_cal_event(
    world_id: String,
    id: String,
    title: Option<String>,
    description: Option<String>,
    date_year: Option<i32>,
    date_month: Option<i32>,
    date_day: Option<i32>,
    date_hour: Option<i32>,
    date_minute: Option<i32>,
    era_id: Option<String>,
    end_year: Option<i32>,
    end_month: Option<i32>,
    end_day: Option<i32>,
    is_all_day: Option<bool>,
    recurrence: Option<serde_json::Value>,
    importance: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    color: Option<String>,
    icon: Option<String>,
) -> Result<CalEvent, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let update = CalEventUpdate {
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
        recurrence,
        importance,
        category,
        tags,
        color,
        icon,
    };
    
    CalendarRepo::update(&db, &world_id, &id, update)
        .await
        .map_err(|e| e.to_string())
}

/// Delete calendar event
#[tauri::command]
pub async fn surreal_delete_cal_event(
    world_id: String,
    id: String,
) -> Result<(), String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    CalendarRepo::delete(&db, &world_id, &id)
        .await
        .map_err(|e| e.to_string())
}

/// List all events for a calendar
#[tauri::command]
pub async fn surreal_list_cal_events(
    world_id: String,
    calendar_id: String,
) -> Result<Vec<CalEvent>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    CalendarRepo::list(&db, &world_id, &calendar_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get events by entity
#[tauri::command]
pub async fn surreal_get_cal_events_by_entity(
    world_id: String,
    entity_id: String,
) -> Result<Vec<CalEvent>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    CalendarRepo::get_by_entity(&db, &world_id, &entity_id)
        .await
        .map_err(|e| e.to_string())
}

/// Link entity to event
#[tauri::command]
pub async fn surreal_link_entity_to_event(
    world_id: String,
    entity_id: String,
    event_id: String,
    role: Option<String>,
    significance: Option<String>,
    notes: Option<String>,
) -> Result<OccursOn, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    let input = LinkEntityToEventInput {
        world_id,
        entity_id,
        event_id,
        role,
        significance,
        notes,
    };
    
    CalendarRepo::link_entity(&db, input)
        .await
        .map_err(|e| e.to_string())
}

/// Unlink entity from event
#[tauri::command]
pub async fn surreal_unlink_entity_from_event(
    world_id: String,
    entity_id: String,
    event_id: String,
) -> Result<(), String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    CalendarRepo::unlink_entity(&db, &world_id, &entity_id, &event_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get event participants
#[tauri::command]
pub async fn surreal_get_event_participants(
    world_id: String,
    event_id: String,
) -> Result<Vec<OccursOn>, String> {
    let db = SurrealConnection::get().map_err(|e| e.to_string())?;
    
    CalendarRepo::get_event_participants(&db, &world_id, &event_id)
        .await
        .map_err(|e| e.to_string())
}
