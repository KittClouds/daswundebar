//! Content API - Notes, folders, networks, entities, relationships, calendar, periods
//!
//! Uses JSON string passthrough for complex params, awaits async commands.

use crate::graph::content_commands as cmd;

// ============================================================================
// API Trait  
// ============================================================================

#[taurpc::procedures(path = "content", export_to = "../src/bindings.ts")]
pub trait ContentApi {
    // Notes - using JSON for complex params
    async fn create_note(params: String) -> Result<String, String>;
    async fn get_note(world_id: String, id: String) -> Result<Option<String>, String>;
    async fn list_notes(world_id: String) -> Result<String, String>;
    async fn update_note(params: String) -> Result<String, String>;
    async fn delete_note(world_id: String, id: String) -> Result<bool, String>;
    
    // Folders
    async fn create_folder(params: String) -> Result<String, String>;
    async fn get_folder(world_id: String, id: String) -> Result<Option<String>, String>;
    async fn list_folders(world_id: String) -> Result<String, String>;
    async fn get_folder_tree(world_id: String) -> Result<String, String>;
    async fn update_folder(params: String) -> Result<String, String>;
    async fn delete_folder(world_id: String, id: String) -> Result<bool, String>;
    
    // Networks
    async fn create_network(params: String) -> Result<String, String>;
    async fn get_network(world_id: String, id: String) -> Result<Option<String>, String>;
    async fn list_networks(world_id: String) -> Result<String, String>;
    async fn delete_network(world_id: String, id: String) -> Result<bool, String>;
    
    // Entities
    async fn create_entity(params: String) -> Result<String, String>;
    async fn get_entity(world_id: String, id: String) -> Result<Option<String>, String>;
    async fn list_entities_by_kind(world_id: String, kind: String) -> Result<String, String>;
    async fn list_entities(world_id: String) -> Result<String, String>;
    async fn delete_entity(world_id: String, id: String) -> Result<bool, String>;
    
    // Relationships
    async fn create_relationship(params: String) -> Result<String, String>;
    async fn get_relationship(world_id: String, id: String) -> Result<Option<String>, String>;
    async fn get_entity_relationships(world_id: String, entity_id: String) -> Result<String, String>;
    async fn delete_relationship(world_id: String, id: String) -> Result<bool, String>;
    
    // Calendar Events
    async fn create_cal_event(params: String) -> Result<String, String>;
    async fn get_cal_event(world_id: String, id: String) -> Result<Option<String>, String>;
    async fn list_cal_events(world_id: String) -> Result<String, String>;
    async fn list_cal_events_by_month(world_id: String, year: i32, month: i32) -> Result<String, String>;
    async fn delete_cal_event(world_id: String, id: String) -> Result<bool, String>;
    
    // Periods
    async fn create_period(params: String) -> Result<String, String>;
    async fn get_period(world_id: String, id: String) -> Result<Option<String>, String>;
    async fn list_periods(world_id: String) -> Result<String, String>;
    async fn get_period_children(world_id: String, parent_id: String) -> Result<String, String>;
    async fn delete_period(world_id: String, id: String) -> Result<bool, String>;
    
    // Field Bindings
    async fn create_binding(params: String) -> Result<String, String>;
    async fn get_binding(world_id: String, id: String) -> Result<Option<String>, String>;
    async fn list_bindings(world_id: String) -> Result<String, String>;
    async fn list_bindings_by_entity(world_id: String, entity_id: String) -> Result<String, String>;
    async fn delete_binding(world_id: String, id: String) -> Result<bool, String>;
}

// ============================================================================
// Implementation - Delegates to existing content_commands (all async)
// ============================================================================

#[derive(Clone, Default)]
pub struct ContentApiImpl;

#[taurpc::resolvers]
impl ContentApi for ContentApiImpl {
    // Notes
    async fn create_note(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { world_id: String, title: String, content: Option<String>, folder_id: Option<String>, entity_kind: Option<String>, entity_subtype: Option<String>, is_entity: Option<bool> }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        let result = cmd::cozo_create_note(p.world_id, p.title, p.content, p.folder_id, p.entity_kind, p.entity_subtype, p.is_entity).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_note(self, world_id: String, id: String) -> Result<Option<String>, String> {
        let result = cmd::cozo_get_note(world_id, id).await?;
        match result {
            Some(n) => Ok(Some(serde_json::to_string(&n).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn list_notes(self, world_id: String) -> Result<String, String> {
        let result = cmd::cozo_list_notes(world_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn update_note(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { world_id: String, id: String, title: Option<String>, content: Option<String>, folder_id: Option<String>, entity_kind: Option<String>, entity_subtype: Option<String>, is_entity: Option<bool>, is_pinned: Option<bool>, favorite: Option<bool> }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        let result = cmd::cozo_update_note(p.world_id, p.id, p.title, p.content, p.folder_id, p.entity_kind, p.entity_subtype, p.is_entity, p.is_pinned, p.favorite).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_note(self, world_id: String, id: String) -> Result<bool, String> {
        cmd::cozo_delete_note(world_id, id).await
    }
    
    // Folders
    async fn create_folder(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { world_id: String, name: String, parent_id: Option<String>, entity_kind: Option<String>, entity_subtype: Option<String>, color: Option<String>, is_typed_root: Option<bool> }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        let result = cmd::cozo_create_folder(p.world_id, p.name, p.parent_id, p.entity_kind, p.entity_subtype, p.color, p.is_typed_root).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_folder(self, world_id: String, id: String) -> Result<Option<String>, String> {
        let result = cmd::cozo_get_folder(world_id, id).await?;
        match result {
            Some(f) => Ok(Some(serde_json::to_string(&f).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn list_folders(self, world_id: String) -> Result<String, String> {
        let result = cmd::cozo_list_folders(world_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_folder_tree(self, world_id: String) -> Result<String, String> {
        let result = cmd::cozo_get_folder_tree(world_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn update_folder(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { world_id: String, id: String, name: Option<String>, parent_id: Option<String>, entity_kind: Option<String>, entity_subtype: Option<String>, color: Option<String>, collapsed: Option<bool> }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        let result = cmd::cozo_update_folder(p.world_id, p.id, p.name, p.parent_id, p.entity_kind, p.entity_subtype, p.color, p.collapsed).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_folder(self, world_id: String, id: String) -> Result<bool, String> {
        cmd::cozo_delete_folder(world_id, id).await
    }
    
    // Networks
    async fn create_network(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { world_id: String, name: String, schema_id: String, root_folder_id: Option<String>, root_entity_id: Option<String>, namespace: Option<String>, description: Option<String>, tags: Option<Vec<String>> }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        let result = cmd::cozo_create_network(p.world_id, p.name, p.schema_id, p.root_folder_id, p.root_entity_id, p.namespace, p.description, p.tags).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_network(self, world_id: String, id: String) -> Result<Option<String>, String> {
        let result = cmd::cozo_get_network(world_id, id).await?;
        match result {
            Some(n) => Ok(Some(serde_json::to_string(&n).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn list_networks(self, world_id: String) -> Result<String, String> {
        let result = cmd::cozo_list_networks(world_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_network(self, world_id: String, id: String) -> Result<bool, String> {
        cmd::cozo_delete_network(world_id, id).await
    }
    
    // Entities
    async fn create_entity(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { world_id: String, label: String, entity_kind: String, entity_subtype: Option<String>, note_id: Option<String>, folder_id: Option<String>, aliases: Option<Vec<String>>, attributes: Option<serde_json::Value> }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        let result = cmd::cozo_create_entity(p.world_id, p.label, p.entity_kind, p.entity_subtype, p.note_id, p.folder_id, p.aliases, p.attributes).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_entity(self, world_id: String, id: String) -> Result<Option<String>, String> {
        let result = cmd::cozo_get_entity(world_id, id).await?;
        match result {
            Some(e) => Ok(Some(serde_json::to_string(&e).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn list_entities_by_kind(self, world_id: String, kind: String) -> Result<String, String> {
        let result = cmd::cozo_list_entities_by_kind(world_id, kind).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn list_entities(self, world_id: String) -> Result<String, String> {
        let result = cmd::cozo_list_entities(world_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_entity(self, world_id: String, id: String) -> Result<bool, String> {
        cmd::cozo_delete_entity(world_id, id).await
    }
    
    // Relationships
    async fn create_relationship(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { world_id: String, source_id: String, target_id: String, relationship_code: String, network_id: Option<String>, strength: Option<f32>, start_date: Option<f64>, end_date: Option<f64>, notes: Option<String>, attributes: Option<serde_json::Value> }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        let result = cmd::cozo_create_relationship(p.world_id, p.source_id, p.target_id, p.relationship_code, p.network_id, p.strength, p.start_date, p.end_date, p.notes, p.attributes).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_relationship(self, world_id: String, id: String) -> Result<Option<String>, String> {
        let result = cmd::cozo_get_relationship(world_id, id).await?;
        match result {
            Some(r) => Ok(Some(serde_json::to_string(&r).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn get_entity_relationships(self, world_id: String, entity_id: String) -> Result<String, String> {
        let result = cmd::cozo_get_entity_relationships(world_id, entity_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_relationship(self, world_id: String, id: String) -> Result<bool, String> {
        cmd::cozo_delete_relationship(world_id, id).await
    }
    
    // Calendar Events
    async fn create_cal_event(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { 
            world_id: String, calendar_id: String, title: String, date_year: i32, date_month: i32, date_day: i32,
            description: Option<String>, date_hour: Option<i32>, date_minute: Option<i32>,
            end_year: Option<i32>, end_month: Option<i32>, end_day: Option<i32>, end_hour: Option<i32>, end_minute: Option<i32>,
            all_day: Option<bool>, recurrence: Option<String>, category: Option<String>, tags: Option<Vec<String>>,
            color: Option<String>, icon: Option<String>, entity_id: Option<String>, entity_kind: Option<String>, source_note_id: Option<String>
        }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        // Note: cozo_create_cal_event has different params - using None for fields that changed
        let result = cmd::cozo_create_cal_event(
            p.world_id, p.calendar_id, p.title, p.date_year, p.date_month, p.date_day,
            p.description, p.date_hour, p.date_minute, None, // era_id
            p.end_year, p.end_month, p.end_day, p.all_day, p.recurrence,
            p.category, p.tags, p.color, p.icon, p.entity_id, p.entity_kind, p.source_note_id
        ).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_cal_event(self, world_id: String, id: String) -> Result<Option<String>, String> {
        let result = cmd::cozo_get_cal_event(world_id, id).await?;
        match result {
            Some(e) => Ok(Some(serde_json::to_string(&e).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn list_cal_events(self, world_id: String) -> Result<String, String> {
        let result = cmd::cozo_list_cal_events(world_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn list_cal_events_by_month(self, world_id: String, year: i32, month: i32) -> Result<String, String> {
        let result = cmd::cozo_list_cal_events_by_month(world_id, year, month).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_cal_event(self, world_id: String, id: String) -> Result<bool, String> {
        cmd::cozo_delete_cal_event(world_id, id).await
    }
    
    // Periods
    async fn create_period(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { 
            world_id: String, calendar_id: String, name: String, start_year: i32, color: String,
            description: Option<String>, start_month: Option<i32>, end_year: Option<i32>, end_month: Option<i32>,
            parent_id: Option<String>, period_type: Option<String>, icon: Option<String>, abbreviation: Option<String>,
            direction: Option<String>, triggered_by: Option<String>, ends_when: Option<String>, arc_type: Option<String>,
            dominant_theme: Option<String>, protagonist_id: Option<String>, antagonist_id: Option<String>,
            summary: Option<String>, detailed_notes: Option<String>,
            show_on_timeline: Option<bool>, timeline_color: Option<String>, timeline_icon: Option<String>
        }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        let result = cmd::cozo_create_period(
            p.world_id, p.calendar_id, p.name, p.start_year, p.color, p.description, p.start_month, p.end_year, p.end_month,
            p.parent_id, p.period_type, p.icon, p.abbreviation, p.direction, p.triggered_by, p.ends_when, p.arc_type,
            p.dominant_theme, p.protagonist_id, p.antagonist_id, p.summary, p.detailed_notes,
            p.show_on_timeline, p.timeline_color, p.timeline_icon
        ).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_period(self, world_id: String, id: String) -> Result<Option<String>, String> {
        let result = cmd::cozo_get_period(world_id, id).await?;
        match result {
            Some(p) => Ok(Some(serde_json::to_string(&p).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn list_periods(self, world_id: String) -> Result<String, String> {
        let result = cmd::cozo_list_periods(world_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_period_children(self, world_id: String, parent_id: String) -> Result<String, String> {
        let result = cmd::cozo_get_period_children(world_id, parent_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_period(self, world_id: String, id: String) -> Result<bool, String> {
        cmd::cozo_delete_period(world_id, id).await
    }
    
    // Field Bindings
    async fn create_binding(self, params: String) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct P { world_id: String, source_entity_id: String, source_field_name: String, target_entity_id: String, target_field_name: String, binding_type: String, transform: Option<serde_json::Value>, aggregation_fn: Option<String>, aggregation_filter: Option<serde_json::Value>, allow_override: Option<bool> }
        let p: P = serde_json::from_str(&params).map_err(|e| e.to_string())?;
        let result = cmd::cozo_create_binding(p.world_id, p.source_entity_id, p.source_field_name, p.target_entity_id, p.target_field_name, p.binding_type, p.transform, p.aggregation_fn, p.aggregation_filter, p.allow_override).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_binding(self, world_id: String, id: String) -> Result<Option<String>, String> {
        let result = cmd::cozo_get_binding(world_id, id).await?;
        match result {
            Some(b) => Ok(Some(serde_json::to_string(&b).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn list_bindings(self, world_id: String) -> Result<String, String> {
        let result = cmd::cozo_list_bindings(world_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn list_bindings_by_entity(self, world_id: String, entity_id: String) -> Result<String, String> {
        let result = cmd::cozo_list_bindings_by_entity(world_id, entity_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_binding(self, world_id: String, id: String) -> Result<bool, String> {
        cmd::cozo_delete_binding(world_id, id).await
    }
}
