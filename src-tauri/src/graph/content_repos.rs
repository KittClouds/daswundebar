//! Content Repositories - CozoDB CRUD operations for notes, folders, etc.
//!
//! Replaces SurrealDB repos with clean CozoDB Datalog queries.

use std::collections::BTreeMap;
use cozo::{DbInstance, DataValue, ScriptMutability};
use uuid::Uuid;
use chrono::Utc;

use super::content_types::*;

// =============================================================================
// HELPERS
// =============================================================================

fn now() -> f64 {
    Utc::now().timestamp_millis() as f64 / 1000.0
}

fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

fn opt_string(s: &Option<String>) -> String {
    s.clone().unwrap_or_default()
}

fn extract_string(v: &DataValue) -> String {
    match v {
        DataValue::Str(s) => s.to_string(),
        _ => String::new(),
    }
}

fn extract_float(v: &DataValue) -> f64 {
    match v {
        DataValue::Num(cozo::Num::Float(f)) => *f,
        DataValue::Num(cozo::Num::Int(i)) => *i as f64,
        _ => 0.0,
    }
}

fn extract_bool(v: &DataValue) -> bool {
    match v {
        DataValue::Bool(b) => *b,
        _ => false,
    }
}

fn extract_int(v: &DataValue) -> i32 {
    match v {
        DataValue::Num(cozo::Num::Int(i)) => *i as i32,
        DataValue::Num(cozo::Num::Float(f)) => *f as i32,
        _ => 0,
    }
}


// =============================================================================
// NOTE REPO
// =============================================================================

pub struct NoteRepo;

impl NoteRepo {
    /// Create a new note
    pub fn create(db: &DbInstance, input: NoteInput) -> Result<Note, String> {
        let id = generate_id();
        let now = now();
        
        let query = r#"
            ?[id, world_id, title, content, folder_id, entity_kind, entity_subtype,
              is_entity, is_pinned, favorite, created_at, updated_at] <- [[
                $id, $world_id, $title, $content, $folder_id, $entity_kind, $entity_subtype,
                $is_entity, false, false, $now, $now
            ]]
            :put notes {
                id, world_id, title, content, folder_id, entity_kind, entity_subtype,
                is_entity, is_pinned, favorite, created_at, updated_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("world_id".to_string(), DataValue::Str(input.world_id.clone().into()));
        params.insert("title".to_string(), DataValue::Str(input.title.clone().into()));
        params.insert("content".to_string(), DataValue::Str(input.content.unwrap_or_default().into()));
        params.insert("folder_id".to_string(), DataValue::Str(opt_string(&input.folder_id).into()));
        params.insert("entity_kind".to_string(), DataValue::Str(opt_string(&input.entity_kind).into()));
        params.insert("entity_subtype".to_string(), DataValue::Str(opt_string(&input.entity_subtype).into()));
        params.insert("is_entity".to_string(), DataValue::Bool(input.is_entity.unwrap_or(false)));
        params.insert("now".to_string(), DataValue::from(now));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to create note: {}", e))?;
        
        // Return the created note
        Self::get(db, &input.world_id, &id)?
            .ok_or_else(|| "Note created but not found".to_string())
    }
    
    /// Get a note by ID
    pub fn get(db: &DbInstance, world_id: &str, id: &str) -> Result<Option<Note>, String> {
        let query = r#"
            ?[id, world_id, title, content, folder_id, entity_kind, entity_subtype,
              is_entity, is_pinned, favorite, created_at, updated_at] :=
                *notes[id, world_id, title, content, folder_id, entity_kind, entity_subtype,
                       is_entity, is_pinned, favorite, created_at, updated_at],
                id = $id,
                world_id = $world_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get note: {}", e))?;
        
        if let Some(row) = result.rows.first() {
            Ok(Some(Self::row_to_note(row)?))
        } else {
            Ok(None)
        }
    }
    
    /// List all notes in a world
    pub fn list_all(db: &DbInstance, world_id: &str) -> Result<Vec<Note>, String> {
        let query = r#"
            ?[id, world_id, title, content, folder_id, entity_kind, entity_subtype,
              is_entity, is_pinned, favorite, created_at, updated_at] :=
                *notes[id, world_id, title, content, folder_id, entity_kind, entity_subtype,
                       is_entity, is_pinned, favorite, created_at, updated_at],
                world_id = $world_id
            :order -updated_at
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list notes: {}", e))?;
        
        let mut notes = Vec::new();
        for row in &result.rows {
            notes.push(Self::row_to_note(row)?);
        }
        Ok(notes)
    }
    
    /// Update a note
    pub fn update(db: &DbInstance, world_id: &str, id: &str, update: NoteUpdate) -> Result<Note, String> {
        // Get existing note first
        let existing = Self::get(db, world_id, id)?
            .ok_or_else(|| format!("Note not found: {}", id))?;
        
        let query = r#"
            ?[id, world_id, title, content, folder_id, entity_kind, entity_subtype,
              is_entity, is_pinned, favorite, created_at, updated_at] <- [[
                $id, $world_id, $title, $content, $folder_id, $entity_kind, $entity_subtype,
                $is_entity, $is_pinned, $favorite, $created_at, $updated_at
            ]]
            :put notes {
                id, world_id, title, content, folder_id, entity_kind, entity_subtype,
                is_entity, is_pinned, favorite, created_at, updated_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        params.insert("title".to_string(), DataValue::Str(update.title.unwrap_or(existing.title).into()));
        params.insert("content".to_string(), DataValue::Str(update.content.unwrap_or(existing.content).into()));
        params.insert("folder_id".to_string(), DataValue::Str(update.folder_id.or(existing.folder_id).unwrap_or_default().into()));
        params.insert("entity_kind".to_string(), DataValue::Str(update.entity_kind.or(existing.entity_kind).unwrap_or_default().into()));
        params.insert("entity_subtype".to_string(), DataValue::Str(update.entity_subtype.or(existing.entity_subtype).unwrap_or_default().into()));
        params.insert("is_entity".to_string(), DataValue::Bool(update.is_entity.unwrap_or(existing.is_entity)));
        params.insert("is_pinned".to_string(), DataValue::Bool(update.is_pinned.unwrap_or(existing.is_pinned)));
        params.insert("favorite".to_string(), DataValue::Bool(update.favorite.unwrap_or(existing.favorite)));
        params.insert("created_at".to_string(), DataValue::from(existing.created_at));
        params.insert("updated_at".to_string(), DataValue::from(now()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to update note: {}", e))?;
        
        Self::get(db, world_id, id)?
            .ok_or_else(|| "Note updated but not found".to_string())
    }
    
    /// Delete a note
    pub fn delete(db: &DbInstance, world_id: &str, id: &str) -> Result<bool, String> {
        let query = r#"
            ?[id] <- [[$id]]
            :rm notes { id }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to delete note: {}", e))?;
        
        // Verify deletion
        Ok(Self::get(db, world_id, id)?.is_none())
    }
    
    /// Convert a row to a Note
    fn row_to_note(row: &[DataValue]) -> Result<Note, String> {
        if row.len() < 12 {
            return Err("Invalid row length for Note".to_string());
        }
        
        Ok(Note {
            id: extract_string(&row[0]),
            world_id: extract_string(&row[1]),
            title: extract_string(&row[2]),
            content: extract_string(&row[3]),
            folder_id: {
                let s = extract_string(&row[4]);
                if s.is_empty() { None } else { Some(s) }
            },
            entity_kind: {
                let s = extract_string(&row[5]);
                if s.is_empty() { None } else { Some(s) }
            },
            entity_subtype: {
                let s = extract_string(&row[6]);
                if s.is_empty() { None } else { Some(s) }
            },
            is_entity: extract_bool(&row[7]),
            is_pinned: extract_bool(&row[8]),
            favorite: extract_bool(&row[9]),
            created_at: extract_float(&row[10]),
            updated_at: extract_float(&row[11]),
        })
    }
}

// =============================================================================
// FOLDER REPO
// =============================================================================

pub struct FolderRepo;

impl FolderRepo {
    /// Create a new folder
    pub fn create(db: &DbInstance, input: FolderInput) -> Result<Folder, String> {
        let id = generate_id();
        let now = now();
        
        let query = r#"
            ?[id, world_id, name, parent_id, entity_kind, entity_subtype, color,
              is_typed_root, network_id, collapsed, fantasy_year, fantasy_month, fantasy_day,
              created_at, updated_at] <- [[
                $id, $world_id, $name, $parent_id, $entity_kind, $entity_subtype, $color,
                $is_typed_root, '', false, 0, 0, 0, $now, $now
            ]]
            :put folders {
                id, world_id, name, parent_id, entity_kind, entity_subtype, color,
                is_typed_root, network_id, collapsed, fantasy_year, fantasy_month, fantasy_day,
                created_at, updated_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("world_id".to_string(), DataValue::Str(input.world_id.clone().into()));
        params.insert("name".to_string(), DataValue::Str(input.name.clone().into()));
        params.insert("parent_id".to_string(), DataValue::Str(opt_string(&input.parent_id).into()));
        params.insert("entity_kind".to_string(), DataValue::Str(opt_string(&input.entity_kind).into()));
        params.insert("entity_subtype".to_string(), DataValue::Str(opt_string(&input.entity_subtype).into()));
        params.insert("color".to_string(), DataValue::Str(opt_string(&input.color).into()));
        params.insert("is_typed_root".to_string(), DataValue::Bool(input.is_typed_root.unwrap_or(false)));
        params.insert("now".to_string(), DataValue::from(now));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to create folder: {}", e))?;
        
        Self::get(db, &input.world_id, &id)?
            .ok_or_else(|| "Folder created but not found".to_string())
    }
    
    /// Get a folder by ID
    pub fn get(db: &DbInstance, world_id: &str, id: &str) -> Result<Option<Folder>, String> {
        let query = r#"
            ?[id, world_id, name, parent_id, entity_kind, entity_subtype, color,
              is_typed_root, network_id, collapsed, fantasy_year, fantasy_month, fantasy_day,
              created_at, updated_at] :=
                *folders[id, world_id, name, parent_id, entity_kind, entity_subtype, color,
                         is_typed_root, network_id, collapsed, fantasy_year, fantasy_month, fantasy_day,
                         created_at, updated_at],
                id = $id,
                world_id = $world_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get folder: {}", e))?;
        
        if let Some(row) = result.rows.first() {
            Ok(Some(Self::row_to_folder(row)?))
        } else {
            Ok(None)
        }
    }
    
    /// List all folders in a world
    pub fn list_all(db: &DbInstance, world_id: &str) -> Result<Vec<Folder>, String> {
        let query = r#"
            ?[id, world_id, name, parent_id, entity_kind, entity_subtype, color,
              is_typed_root, network_id, collapsed, fantasy_year, fantasy_month, fantasy_day,
              created_at, updated_at] :=
                *folders[id, world_id, name, parent_id, entity_kind, entity_subtype, color,
                         is_typed_root, network_id, collapsed, fantasy_year, fantasy_month, fantasy_day,
                         created_at, updated_at],
                world_id = $world_id
            :order name
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list folders: {}", e))?;
        
        let mut folders = Vec::new();
        for row in &result.rows {
            folders.push(Self::row_to_folder(row)?);
        }
        Ok(folders)
    }
    
    /// Update a folder
    pub fn update(db: &DbInstance, world_id: &str, id: &str, update: FolderUpdate) -> Result<Folder, String> {
        // Get existing folder first
        let existing = Self::get(db, world_id, id)?
            .ok_or_else(|| format!("Folder not found: {}", id))?;
        
        let query = r#"
            ?[id, world_id, name, parent_id, entity_kind, entity_subtype, color,
              is_typed_root, network_id, collapsed, fantasy_year, fantasy_month, fantasy_day,
              created_at, updated_at] <- [[
                $id, $world_id, $name, $parent_id, $entity_kind, $entity_subtype, $color,
                $is_typed_root, $network_id, $collapsed, $fantasy_year, $fantasy_month, $fantasy_day,
                $created_at, $updated_at
            ]]
            :put folders {
                id, world_id, name, parent_id, entity_kind, entity_subtype, color,
                is_typed_root, network_id, collapsed, fantasy_year, fantasy_month, fantasy_day,
                created_at, updated_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        params.insert("name".to_string(), DataValue::Str(update.name.unwrap_or(existing.name).into()));
        params.insert("parent_id".to_string(), DataValue::Str(update.parent_id.or(existing.parent_id).unwrap_or_default().into()));
        params.insert("entity_kind".to_string(), DataValue::Str(update.entity_kind.or(existing.entity_kind).unwrap_or_default().into()));
        params.insert("entity_subtype".to_string(), DataValue::Str(update.entity_subtype.or(existing.entity_subtype).unwrap_or_default().into()));
        params.insert("color".to_string(), DataValue::Str(update.color.or(existing.color).unwrap_or_default().into()));
        params.insert("is_typed_root".to_string(), DataValue::Bool(existing.is_typed_root));
        params.insert("network_id".to_string(), DataValue::Str(existing.network_id.unwrap_or_default().into()));
        params.insert("collapsed".to_string(), DataValue::Bool(update.collapsed.unwrap_or(existing.collapsed)));
        params.insert("fantasy_year".to_string(), DataValue::from(existing.fantasy_year.unwrap_or(0) as i64));
        params.insert("fantasy_month".to_string(), DataValue::from(existing.fantasy_month.unwrap_or(0) as i64));
        params.insert("fantasy_day".to_string(), DataValue::from(existing.fantasy_day.unwrap_or(0) as i64));
        params.insert("created_at".to_string(), DataValue::from(existing.created_at));
        params.insert("updated_at".to_string(), DataValue::from(now()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to update folder: {}", e))?;
        
        Self::get(db, world_id, id)?
            .ok_or_else(|| "Folder updated but not found".to_string())
    }
    
    /// Get folder tree (recursive)
    pub fn get_tree(db: &DbInstance, world_id: &str) -> Result<Vec<FolderTreeNode>, String> {
        use std::collections::HashMap;
        
        let folders = Self::list_all(db, world_id)?;
        let notes = NoteRepo::list_all(db, world_id)?;
        
        // Index folders by ID
        let mut folder_map: HashMap<String, Folder> = HashMap::new();
        let mut children_map: HashMap<Option<String>, Vec<String>> = HashMap::new();
        
        for folder in folders {
            let folder_id = folder.id.clone();
            let parent_key = folder.parent_id.clone();
            
            children_map.entry(parent_key).or_default().push(folder_id.clone());
            folder_map.insert(folder_id, folder);
        }
        
        // Index notes by folder_id
        let mut notes_by_folder: HashMap<String, Vec<NoteSummary>> = HashMap::new();
        for note in notes {
            if let Some(folder_id) = &note.folder_id {
                notes_by_folder.entry(folder_id.clone()).or_default().push(NoteSummary {
                    id: note.id,
                    title: note.title,
                    is_pinned: note.is_pinned,
                    favorite: note.favorite,
                    entity_kind: note.entity_kind,
                    updated_at: note.updated_at,
                });
            }
        }
        
        // Build tree recursively
        fn build_node(
            folder_id: &str,
            folder_map: &HashMap<String, Folder>,
            children_map: &HashMap<Option<String>, Vec<String>>,
            notes_by_folder: &HashMap<String, Vec<NoteSummary>>,
        ) -> Option<FolderTreeNode> {
            let folder = folder_map.get(folder_id)?.clone();
            
            let child_ids = children_map.get(&Some(folder_id.to_string())).cloned().unwrap_or_default();
            let children: Vec<FolderTreeNode> = child_ids
                .iter()
                .filter_map(|id| build_node(id, folder_map, children_map, notes_by_folder))
                .collect();
            
            let notes = notes_by_folder.get(folder_id).cloned().unwrap_or_default();
            
            Some(FolderTreeNode { folder, children, notes })
        }
        
        // Find roots (folders without parent)
        let root_ids = children_map.get(&None).cloned().unwrap_or_default();
        
        Ok(root_ids
            .iter()
            .filter_map(|id| build_node(id, &folder_map, &children_map, &notes_by_folder))
            .collect())
    }
    
    /// Delete a folder
    pub fn delete(db: &DbInstance, world_id: &str, id: &str) -> Result<bool, String> {
        let query = r#"
            ?[id] <- [[$id]]
            :rm folders { id }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to delete folder: {}", e))?;
        
        Ok(Self::get(db, world_id, id)?.is_none())
    }
    
    fn row_to_folder(row: &[DataValue]) -> Result<Folder, String> {
        if row.len() < 15 {
            return Err("Invalid row length for Folder".to_string());
        }
        
        Ok(Folder {
            id: extract_string(&row[0]),
            world_id: extract_string(&row[1]),
            name: extract_string(&row[2]),
            parent_id: {
                let s = extract_string(&row[3]);
                if s.is_empty() { None } else { Some(s) }
            },
            entity_kind: {
                let s = extract_string(&row[4]);
                if s.is_empty() { None } else { Some(s) }
            },
            entity_subtype: {
                let s = extract_string(&row[5]);
                if s.is_empty() { None } else { Some(s) }
            },
            color: {
                let s = extract_string(&row[6]);
                if s.is_empty() { None } else { Some(s) }
            },
            is_typed_root: extract_bool(&row[7]),
            network_id: {
                let s = extract_string(&row[8]);
                if s.is_empty() { None } else { Some(s) }
            },
            collapsed: extract_bool(&row[9]),
            fantasy_year: {
                let v = extract_int(&row[10]);
                if v == 0 { None } else { Some(v) }
            },
            fantasy_month: {
                let v = extract_int(&row[11]);
                if v == 0 { None } else { Some(v) }
            },
            fantasy_day: {
                let v = extract_int(&row[12]);
                if v == 0 { None } else { Some(v) }
            },
            created_at: extract_float(&row[13]),
            updated_at: extract_float(&row[14]),
        })
    }
}

// =============================================================================
// NETWORK REPO
// =============================================================================

pub struct NetworkRepo;

impl NetworkRepo {
    /// Create a new network
    pub fn create(db: &DbInstance, input: NetworkInput) -> Result<Network, String> {
        let id = generate_id();
        let now = now();
        
        let tags_json = serde_json::to_string(&input.tags.unwrap_or_default())
            .unwrap_or_else(|_| "[]".to_string());
        
        let query = r#"
            ?[id, world_id, name, schema_id, root_folder_id, root_entity_id, namespace,
              description, tags, member_count, relationship_count, max_depth, created_at, updated_at] <- [[
                $id, $world_id, $name, $schema_id, $root_folder_id, $root_entity_id, $namespace,
                $description, $tags, 0, 0, 0, $now, $now
            ]]
            :put networks {
                id, world_id, name, schema_id, root_folder_id, root_entity_id, namespace,
                description, tags, member_count, relationship_count, max_depth, created_at, updated_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("world_id".to_string(), DataValue::Str(input.world_id.clone().into()));
        params.insert("name".to_string(), DataValue::Str(input.name.into()));
        params.insert("schema_id".to_string(), DataValue::Str(input.schema_id.into()));
        params.insert("root_folder_id".to_string(), DataValue::Str(opt_string(&input.root_folder_id).into()));
        params.insert("root_entity_id".to_string(), DataValue::Str(opt_string(&input.root_entity_id).into()));
        params.insert("namespace".to_string(), DataValue::Str(input.namespace.unwrap_or_default().into()));
        params.insert("description".to_string(), DataValue::Str(opt_string(&input.description).into()));
        params.insert("tags".to_string(), DataValue::Str(tags_json.into()));
        params.insert("now".to_string(), DataValue::from(now));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to create network: {}", e))?;
        
        Self::get(db, &input.world_id, &id)?
            .ok_or_else(|| "Network created but not found".to_string())
    }
    
    /// Get a network by ID
    pub fn get(db: &DbInstance, world_id: &str, id: &str) -> Result<Option<Network>, String> {
        let query = r#"
            ?[id, world_id, name, schema_id, root_folder_id, root_entity_id, namespace,
              description, tags, member_count, relationship_count, max_depth, created_at, updated_at] :=
                *networks[id, world_id, name, schema_id, root_folder_id, root_entity_id, namespace,
                          description, tags, member_count, relationship_count, max_depth, created_at, updated_at],
                id = $id,
                world_id = $world_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get network: {}", e))?;
        
        if let Some(row) = result.rows.first() {
            Ok(Some(Self::row_to_network(row)?))
        } else {
            Ok(None)
        }
    }
    
    /// List all networks in a world
    pub fn list_all(db: &DbInstance, world_id: &str) -> Result<Vec<Network>, String> {
        let query = r#"
            ?[id, world_id, name, schema_id, root_folder_id, root_entity_id, namespace,
              description, tags, member_count, relationship_count, max_depth, created_at, updated_at] :=
                *networks[id, world_id, name, schema_id, root_folder_id, root_entity_id, namespace,
                          description, tags, member_count, relationship_count, max_depth, created_at, updated_at],
                world_id = $world_id
            :order name
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list networks: {}", e))?;
        
        let mut networks = Vec::new();
        for row in &result.rows {
            networks.push(Self::row_to_network(row)?);
        }
        Ok(networks)
    }
    
    /// Delete a network
    pub fn delete(db: &DbInstance, _world_id: &str, id: &str) -> Result<bool, String> {
        let query = r#"
            ?[id] <- [[$id]]
            :rm networks { id }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to delete network: {}", e))?;
        
        Ok(true)
    }
    
    fn row_to_network(row: &[DataValue]) -> Result<Network, String> {
        if row.len() < 14 {
            return Err("Invalid row length for Network".to_string());
        }
        
        let tags_str = extract_string(&row[8]);
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        
        Ok(Network {
            id: extract_string(&row[0]),
            world_id: extract_string(&row[1]),
            name: extract_string(&row[2]),
            schema_id: extract_string(&row[3]),
            root_folder_id: {
                let s = extract_string(&row[4]);
                if s.is_empty() { None } else { Some(s) }
            },
            root_entity_id: {
                let s = extract_string(&row[5]);
                if s.is_empty() { None } else { Some(s) }
            },
            namespace: extract_string(&row[6]),
            description: {
                let s = extract_string(&row[7]);
                if s.is_empty() { None } else { Some(s) }
            },
            tags,
            member_count: extract_int(&row[9]),
            relationship_count: extract_int(&row[10]),
            max_depth: extract_int(&row[11]),
            created_at: extract_float(&row[12]),
            updated_at: extract_float(&row[13]),
        })
    }
}

// =============================================================================
// ENTITY REPO (Domain entities, not knowledge graph nodes)
// =============================================================================

pub struct EntityRepo;

impl EntityRepo {
    /// Create a new entity
    pub fn create(db: &DbInstance, input: EntityInput) -> Result<Entity, String> {
        let id = generate_id();
        let now = now();
        
        let aliases_json = serde_json::to_string(&input.aliases.unwrap_or_default())
            .unwrap_or_else(|_| "[]".to_string());
        let attrs_json = serde_json::to_string(&input.attributes.unwrap_or(serde_json::json!({})))
            .unwrap_or_else(|_| "{}".to_string());
        
        let query = r#"
            ?[id, world_id, label, entity_kind, entity_subtype, note_id, folder_id,
              is_active, aliases, attributes, created_at, updated_at] <- [[
                $id, $world_id, $label, $entity_kind, $entity_subtype, $note_id, $folder_id,
                true, $aliases, $attributes, $now, $now
            ]]
            :put entities {
                id, world_id, label, entity_kind, entity_subtype, note_id, folder_id,
                is_active, aliases, attributes, created_at, updated_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("world_id".to_string(), DataValue::Str(input.world_id.clone().into()));
        params.insert("label".to_string(), DataValue::Str(input.label.into()));
        params.insert("entity_kind".to_string(), DataValue::Str(input.entity_kind.into()));
        params.insert("entity_subtype".to_string(), DataValue::Str(opt_string(&input.entity_subtype).into()));
        params.insert("note_id".to_string(), DataValue::Str(opt_string(&input.note_id).into()));
        params.insert("folder_id".to_string(), DataValue::Str(opt_string(&input.folder_id).into()));
        params.insert("aliases".to_string(), DataValue::Str(aliases_json.into()));
        params.insert("attributes".to_string(), DataValue::Str(attrs_json.into()));
        params.insert("now".to_string(), DataValue::from(now));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to create entity: {}", e))?;
        
        Self::get(db, &input.world_id, &id)?
            .ok_or_else(|| "Entity created but not found".to_string())
    }
    
    /// Get an entity by ID
    pub fn get(db: &DbInstance, world_id: &str, id: &str) -> Result<Option<Entity>, String> {
        let query = r#"
            ?[id, world_id, label, entity_kind, entity_subtype, note_id, folder_id,
              is_active, aliases, attributes, created_at, updated_at] :=
                *entities[id, world_id, label, entity_kind, entity_subtype, note_id, folder_id,
                          is_active, aliases, attributes, created_at, updated_at],
                id = $id,
                world_id = $world_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get entity: {}", e))?;
        
        if let Some(row) = result.rows.first() {
            Ok(Some(Self::row_to_entity(row)?))
        } else {
            Ok(None)
        }
    }
    
    /// List entities by kind
    pub fn list_by_kind(db: &DbInstance, world_id: &str, kind: &str) -> Result<Vec<Entity>, String> {
        let query = r#"
            ?[id, world_id, label, entity_kind, entity_subtype, note_id, folder_id,
              is_active, aliases, attributes, created_at, updated_at] :=
                *entities[id, world_id, label, entity_kind, entity_subtype, note_id, folder_id,
                          is_active, aliases, attributes, created_at, updated_at],
                world_id = $world_id,
                entity_kind = $kind
            :order label
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        params.insert("kind".to_string(), DataValue::Str(kind.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list entities: {}", e))?;
        
        let mut entities = Vec::new();
        for row in &result.rows {
            entities.push(Self::row_to_entity(row)?);
        }
        Ok(entities)
    }
    
    /// List all entities
    pub fn list_all(db: &DbInstance, world_id: &str) -> Result<Vec<Entity>, String> {
        let query = r#"
            ?[id, world_id, label, entity_kind, entity_subtype, note_id, folder_id,
              is_active, aliases, attributes, created_at, updated_at] :=
                *entities[id, world_id, label, entity_kind, entity_subtype, note_id, folder_id,
                          is_active, aliases, attributes, created_at, updated_at],
                world_id = $world_id
            :order label
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list entities: {}", e))?;
        
        let mut entities = Vec::new();
        for row in &result.rows {
            entities.push(Self::row_to_entity(row)?);
        }
        Ok(entities)
    }
    
    /// Delete an entity
    pub fn delete(db: &DbInstance, _world_id: &str, id: &str) -> Result<bool, String> {
        let query = r#"
            ?[id] <- [[$id]]
            :rm entities { id }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to delete entity: {}", e))?;
        
        Ok(true)
    }
    
    fn row_to_entity(row: &[DataValue]) -> Result<Entity, String> {
        if row.len() < 12 {
            return Err("Invalid row length for Entity".to_string());
        }
        
        let aliases_str = extract_string(&row[8]);
        let aliases: Vec<String> = serde_json::from_str(&aliases_str).unwrap_or_default();
        
        let attrs_str = extract_string(&row[9]);
        let attributes: serde_json::Value = serde_json::from_str(&attrs_str)
            .unwrap_or(serde_json::json!({}));
        
        Ok(Entity {
            id: extract_string(&row[0]),
            world_id: extract_string(&row[1]),
            label: extract_string(&row[2]),
            entity_kind: extract_string(&row[3]),
            entity_subtype: {
                let s = extract_string(&row[4]);
                if s.is_empty() { None } else { Some(s) }
            },
            note_id: {
                let s = extract_string(&row[5]);
                if s.is_empty() { None } else { Some(s) }
            },
            folder_id: {
                let s = extract_string(&row[6]);
                if s.is_empty() { None } else { Some(s) }
            },
            is_active: extract_bool(&row[7]),
            aliases,
            attributes,
            created_at: extract_float(&row[10]),
            updated_at: extract_float(&row[11]),
        })
    }
}

// =============================================================================
// RELATIONSHIP REPO
// =============================================================================

pub struct RelationshipRepo;

impl RelationshipRepo {
    /// Create a relationship
    pub fn create(db: &DbInstance, input: CreateRelationshipInput) -> Result<Relationship, String> {
        let id = generate_id();
        let now = now();
        
        let attrs_json = serde_json::to_string(&input.attributes.unwrap_or(serde_json::json!({})))
            .unwrap_or_else(|_| "{}".to_string());
        
        let query = r#"
            ?[id, world_id, source_id, target_id, network_id, relationship_code, inverse_id,
              strength, start_date, end_date, notes, attributes, created_at] <- [[
                $id, $world_id, $source_id, $target_id, $network_id, $relationship_code, '',
                $strength, $start_date, $end_date, $notes, $attributes, $now
            ]]
            :put relates_to {
                id, world_id, source_id, target_id, network_id, relationship_code, inverse_id,
                strength, start_date, end_date, notes, attributes, created_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("world_id".to_string(), DataValue::Str(input.world_id.clone().into()));
        params.insert("source_id".to_string(), DataValue::Str(input.source_id.into()));
        params.insert("target_id".to_string(), DataValue::Str(input.target_id.into()));
        params.insert("network_id".to_string(), DataValue::Str(opt_string(&input.network_id).into()));
        params.insert("relationship_code".to_string(), DataValue::Str(input.relationship_code.into()));
        params.insert("strength".to_string(), DataValue::from(input.strength.unwrap_or(1.0) as f64));
        params.insert("start_date".to_string(), DataValue::from(input.start_date.unwrap_or(0.0)));
        params.insert("end_date".to_string(), DataValue::from(input.end_date.unwrap_or(0.0)));
        params.insert("notes".to_string(), DataValue::Str(opt_string(&input.notes).into()));
        params.insert("attributes".to_string(), DataValue::Str(attrs_json.into()));
        params.insert("now".to_string(), DataValue::from(now));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to create relationship: {}", e))?;
        
        Self::get(db, &input.world_id, &id)?
            .ok_or_else(|| "Relationship created but not found".to_string())
    }
    
    /// Get a relationship by ID
    pub fn get(db: &DbInstance, world_id: &str, id: &str) -> Result<Option<Relationship>, String> {
        let query = r#"
            ?[id, world_id, source_id, target_id, network_id, relationship_code, inverse_id,
              strength, start_date, end_date, notes, attributes, created_at] :=
                *relates_to[id, world_id, source_id, target_id, network_id, relationship_code, inverse_id,
                            strength, start_date, end_date, notes, attributes, created_at],
                id = $id,
                world_id = $world_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get relationship: {}", e))?;
        
        if let Some(row) = result.rows.first() {
            Ok(Some(Self::row_to_relationship(row)?))
        } else {
            Ok(None)
        }
    }
    
    /// Get relationships for an entity
    pub fn get_for_entity(db: &DbInstance, world_id: &str, entity_id: &str) -> Result<Vec<Relationship>, String> {
        let query = r#"
            ?[id, world_id, source_id, target_id, network_id, relationship_code, inverse_id,
              strength, start_date, end_date, notes, attributes, created_at] :=
                *relates_to[id, world_id, source_id, target_id, network_id, relationship_code, inverse_id,
                            strength, start_date, end_date, notes, attributes, created_at],
                world_id = $world_id,
                or(source_id = $entity_id, target_id = $entity_id)
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        params.insert("entity_id".to_string(), DataValue::Str(entity_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get relationships: {}", e))?;
        
        let mut rels = Vec::new();
        for row in &result.rows {
            rels.push(Self::row_to_relationship(row)?);
        }
        Ok(rels)
    }
    
    /// Delete a relationship
    pub fn delete(db: &DbInstance, _world_id: &str, id: &str) -> Result<bool, String> {
        let query = r#"
            ?[id] <- [[$id]]
            :rm relates_to { id }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to delete relationship: {}", e))?;
        
        Ok(true)
    }
    
    fn row_to_relationship(row: &[DataValue]) -> Result<Relationship, String> {
        if row.len() < 13 {
            return Err("Invalid row length for Relationship".to_string());
        }
        
        let attrs_str = extract_string(&row[11]);
        let attributes: serde_json::Value = serde_json::from_str(&attrs_str)
            .unwrap_or(serde_json::json!({}));
        
        Ok(Relationship {
            id: extract_string(&row[0]),
            world_id: extract_string(&row[1]),
            source_id: extract_string(&row[2]),
            target_id: extract_string(&row[3]),
            network_id: {
                let s = extract_string(&row[4]);
                if s.is_empty() { None } else { Some(s) }
            },
            relationship_code: extract_string(&row[5]),
            inverse_id: {
                let s = extract_string(&row[6]);
                if s.is_empty() { None } else { Some(s) }
            },
            strength: extract_float(&row[7]) as f32,
            start_date: {
                let v = extract_float(&row[8]);
                if v == 0.0 { None } else { Some(v) }
            },
            end_date: {
                let v = extract_float(&row[9]);
                if v == 0.0 { None } else { Some(v) }
            },
            notes: {
                let s = extract_string(&row[10]);
                if s.is_empty() { None } else { Some(s) }
            },
            attributes,
            created_at: extract_float(&row[12]),
        })
    }
}

// =============================================================================
// CAL EVENT REPO
// =============================================================================

pub struct CalEventRepo;

impl CalEventRepo {
    /// Create a calendar event
    pub fn create(db: &DbInstance, input: CalEventInput) -> Result<CalEvent, String> {
        let id = generate_id();
        let now = now();
        
        let recurrence_json = serde_json::to_string(&input.recurrence)
            .unwrap_or_else(|_| "null".to_string());
        let tags_json = serde_json::to_string(&input.tags.unwrap_or_default())
            .unwrap_or_else(|_| "[]".to_string());
        
        let query = r#"
            ?[id, world_id, calendar_id, title, description, date_year, date_month, date_day,
              date_hour, date_minute, era_id, end_year, end_month, end_day, is_all_day,
              recurrence, parent_event_id, importance, category, tags, color, icon,
              entity_id, entity_kind, source_note_id, created_at, updated_at] <- [[
                $id, $world_id, $calendar_id, $title, $description, $date_year, $date_month, $date_day,
                $date_hour, $date_minute, $era_id, $end_year, $end_month, $end_day, $is_all_day,
                $recurrence, '', $importance, $category, $tags, $color, $icon,
                $entity_id, $entity_kind, $source_note_id, $now, $now
            ]]
            :put cal_events {
                id, world_id, calendar_id, title, description, date_year, date_month, date_day,
                date_hour, date_minute, era_id, end_year, end_month, end_day, is_all_day,
                recurrence, parent_event_id, importance, category, tags, color, icon,
                entity_id, entity_kind, source_note_id, created_at, updated_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("world_id".to_string(), DataValue::Str(input.world_id.clone().into()));
        params.insert("calendar_id".to_string(), DataValue::Str(input.calendar_id.into()));
        params.insert("title".to_string(), DataValue::Str(input.title.into()));
        params.insert("description".to_string(), DataValue::Str(opt_string(&input.description).into()));
        params.insert("date_year".to_string(), DataValue::from(input.date_year as i64));
        params.insert("date_month".to_string(), DataValue::from(input.date_month as i64));
        params.insert("date_day".to_string(), DataValue::from(input.date_day as i64));
        params.insert("date_hour".to_string(), DataValue::from(input.date_hour.unwrap_or(0) as i64));
        params.insert("date_minute".to_string(), DataValue::from(input.date_minute.unwrap_or(0) as i64));
        params.insert("era_id".to_string(), DataValue::Str(opt_string(&input.era_id).into()));
        params.insert("end_year".to_string(), DataValue::from(input.end_year.unwrap_or(0) as i64));
        params.insert("end_month".to_string(), DataValue::from(input.end_month.unwrap_or(0) as i64));
        params.insert("end_day".to_string(), DataValue::from(input.end_day.unwrap_or(0) as i64));
        params.insert("is_all_day".to_string(), DataValue::Bool(input.is_all_day.unwrap_or(true)));
        params.insert("recurrence".to_string(), DataValue::Str(recurrence_json.into()));
        params.insert("importance".to_string(), DataValue::Str(input.importance.unwrap_or_else(|| "normal".to_string()).into()));
        params.insert("category".to_string(), DataValue::Str(input.category.unwrap_or_else(|| "event".to_string()).into()));
        params.insert("tags".to_string(), DataValue::Str(tags_json.into()));
        params.insert("color".to_string(), DataValue::Str(opt_string(&input.color).into()));
        params.insert("icon".to_string(), DataValue::Str(opt_string(&input.icon).into()));
        params.insert("entity_id".to_string(), DataValue::Str(opt_string(&input.entity_id).into()));
        params.insert("entity_kind".to_string(), DataValue::Str(opt_string(&input.entity_kind).into()));
        params.insert("source_note_id".to_string(), DataValue::Str(opt_string(&input.source_note_id).into()));
        params.insert("now".to_string(), DataValue::from(now));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to create cal_event: {}", e))?;
        
        Self::get(db, &input.world_id, &id)?
            .ok_or_else(|| "CalEvent created but not found".to_string())
    }
    
    /// Get a calendar event by ID
    pub fn get(db: &DbInstance, world_id: &str, id: &str) -> Result<Option<CalEvent>, String> {
        let query = r#"
            ?[id, world_id, calendar_id, title, description, date_year, date_month, date_day,
              date_hour, date_minute, era_id, end_year, end_month, end_day, is_all_day,
              recurrence, parent_event_id, importance, category, tags, color, icon,
              entity_id, entity_kind, source_note_id, created_at, updated_at] :=
                *cal_events[id, world_id, calendar_id, title, description, date_year, date_month, date_day,
                            date_hour, date_minute, era_id, end_year, end_month, end_day, is_all_day,
                            recurrence, parent_event_id, importance, category, tags, color, icon,
                            entity_id, entity_kind, source_note_id, created_at, updated_at],
                id = $id,
                world_id = $world_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get cal_event: {}", e))?;
        
        if let Some(row) = result.rows.first() {
            Ok(Some(Self::row_to_event(row)?))
        } else {
            Ok(None)
        }
    }
    
    /// List all events in a world
    pub fn list_all(db: &DbInstance, world_id: &str) -> Result<Vec<CalEvent>, String> {
        let query = r#"
            ?[id, world_id, calendar_id, title, description, date_year, date_month, date_day,
              date_hour, date_minute, era_id, end_year, end_month, end_day, is_all_day,
              recurrence, parent_event_id, importance, category, tags, color, icon,
              entity_id, entity_kind, source_note_id, created_at, updated_at] :=
                *cal_events[id, world_id, calendar_id, title, description, date_year, date_month, date_day,
                            date_hour, date_minute, era_id, end_year, end_month, end_day, is_all_day,
                            recurrence, parent_event_id, importance, category, tags, color, icon,
                            entity_id, entity_kind, source_note_id, created_at, updated_at],
                world_id = $world_id
            :order date_year, date_month, date_day
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list cal_events: {}", e))?;
        
        let mut events = Vec::new();
        for row in &result.rows {
            events.push(Self::row_to_event(row)?);
        }
        Ok(events)
    }
    
    /// List events by month
    pub fn list_by_month(db: &DbInstance, world_id: &str, year: i32, month: i32) -> Result<Vec<CalEvent>, String> {
        let query = r#"
            ?[id, world_id, calendar_id, title, description, date_year, date_month, date_day,
              date_hour, date_minute, era_id, end_year, end_month, end_day, is_all_day,
              recurrence, parent_event_id, importance, category, tags, color, icon,
              entity_id, entity_kind, source_note_id, created_at, updated_at] :=
                *cal_events[id, world_id, calendar_id, title, description, date_year, date_month, date_day,
                            date_hour, date_minute, era_id, end_year, end_month, end_day, is_all_day,
                            recurrence, parent_event_id, importance, category, tags, color, icon,
                            entity_id, entity_kind, source_note_id, created_at, updated_at],
                world_id = $world_id,
                date_year = $year,
                date_month = $month
            :order date_day
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        params.insert("year".to_string(), DataValue::from(year as i64));
        params.insert("month".to_string(), DataValue::from(month as i64));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list cal_events by month: {}", e))?;
        
        let mut events = Vec::new();
        for row in &result.rows {
            events.push(Self::row_to_event(row)?);
        }
        Ok(events)
    }
    
    /// Delete a calendar event
    pub fn delete(db: &DbInstance, _world_id: &str, id: &str) -> Result<bool, String> {
        let query = r#"
            ?[id] <- [[$id]]
            :rm cal_events { id }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to delete cal_event: {}", e))?;
        
        Ok(true)
    }
    
    fn row_to_event(row: &[DataValue]) -> Result<CalEvent, String> {
        if row.len() < 27 {
            return Err("Invalid row length for CalEvent".to_string());
        }
        
        let recurrence_str = extract_string(&row[15]);
        let recurrence: Option<serde_json::Value> = if recurrence_str.is_empty() || recurrence_str == "null" {
            None
        } else {
            serde_json::from_str(&recurrence_str).ok()
        };
        
        let tags_str = extract_string(&row[19]);
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        
        Ok(CalEvent {
            id: extract_string(&row[0]),
            world_id: extract_string(&row[1]),
            calendar_id: extract_string(&row[2]),
            title: extract_string(&row[3]),
            description: {
                let s = extract_string(&row[4]);
                if s.is_empty() { None } else { Some(s) }
            },
            date_year: extract_int(&row[5]),
            date_month: extract_int(&row[6]),
            date_day: extract_int(&row[7]),
            date_hour: {
                let v = extract_int(&row[8]);
                if v == 0 { None } else { Some(v) }
            },
            date_minute: {
                let v = extract_int(&row[9]);
                if v == 0 { None } else { Some(v) }
            },
            era_id: {
                let s = extract_string(&row[10]);
                if s.is_empty() { None } else { Some(s) }
            },
            end_year: {
                let v = extract_int(&row[11]);
                if v == 0 { None } else { Some(v) }
            },
            end_month: {
                let v = extract_int(&row[12]);
                if v == 0 { None } else { Some(v) }
            },
            end_day: {
                let v = extract_int(&row[13]);
                if v == 0 { None } else { Some(v) }
            },
            is_all_day: extract_bool(&row[14]),
            recurrence,
            parent_event_id: {
                let s = extract_string(&row[16]);
                if s.is_empty() { None } else { Some(s) }
            },
            importance: extract_string(&row[17]),
            category: extract_string(&row[18]),
            tags,
            color: {
                let s = extract_string(&row[20]);
                if s.is_empty() { None } else { Some(s) }
            },
            icon: {
                let s = extract_string(&row[21]);
                if s.is_empty() { None } else { Some(s) }
            },
            entity_id: {
                let s = extract_string(&row[22]);
                if s.is_empty() { None } else { Some(s) }
            },
            entity_kind: {
                let s = extract_string(&row[23]);
                if s.is_empty() { None } else { Some(s) }
            },
            source_note_id: {
                let s = extract_string(&row[24]);
                if s.is_empty() { None } else { Some(s) }
            },
            created_at: extract_float(&row[25]),
            updated_at: extract_float(&row[26]),
        })
    }
}

// =============================================================================
// PERIOD REPO
// =============================================================================

pub struct PeriodRepo;

impl PeriodRepo {
    /// Create a period
    pub fn create(db: &DbInstance, input: PeriodInput) -> Result<Period, String> {
        let id = generate_id();
        let now = now();
        
        let major_events_json = "[]".to_string();
        
        let query = r#"
            ?[id, world_id, calendar_id, name, description, start_year, start_month,
              end_year, end_month, parent_period_id, period_type, color, icon, abbreviation,
              direction, triggered_by, ends_when, major_events, arc_type, dominant_theme,
              protagonist_id, antagonist_id, summary, detailed_notes, show_on_timeline,
              timeline_color, timeline_icon, created_at, updated_at] <- [[
                $id, $world_id, $calendar_id, $name, $description, $start_year, $start_month,
                $end_year, $end_month, $parent_period_id, $period_type, $color, $icon, $abbreviation,
                $direction, $triggered_by, $ends_when, $major_events, $arc_type, $dominant_theme,
                $protagonist_id, $antagonist_id, $summary, $detailed_notes, $show_on_timeline,
                $timeline_color, $timeline_icon, $now, $now
            ]]
            :put periods {
                id, world_id, calendar_id, name, description, start_year, start_month,
                end_year, end_month, parent_period_id, period_type, color, icon, abbreviation,
                direction, triggered_by, ends_when, major_events, arc_type, dominant_theme,
                protagonist_id, antagonist_id, summary, detailed_notes, show_on_timeline,
                timeline_color, timeline_icon, created_at, updated_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("world_id".to_string(), DataValue::Str(input.world_id.clone().into()));
        params.insert("calendar_id".to_string(), DataValue::Str(input.calendar_id.into()));
        params.insert("name".to_string(), DataValue::Str(input.name.into()));
        params.insert("description".to_string(), DataValue::Str(opt_string(&input.description).into()));
        params.insert("start_year".to_string(), DataValue::from(input.start_year as i64));
        params.insert("start_month".to_string(), DataValue::from(input.start_month.unwrap_or(1) as i64));
        params.insert("end_year".to_string(), DataValue::from(input.end_year.unwrap_or(0) as i64));
        params.insert("end_month".to_string(), DataValue::from(input.end_month.unwrap_or(0) as i64));
        params.insert("parent_period_id".to_string(), DataValue::Str(opt_string(&input.parent_period_id).into()));
        params.insert("period_type".to_string(), DataValue::Str(input.period_type.unwrap_or_else(|| "era".to_string()).into()));
        params.insert("color".to_string(), DataValue::Str(input.color.into()));
        params.insert("icon".to_string(), DataValue::Str(opt_string(&input.icon).into()));
        params.insert("abbreviation".to_string(), DataValue::Str(opt_string(&input.abbreviation).into()));
        params.insert("direction".to_string(), DataValue::Str(input.direction.unwrap_or_else(|| "forward".to_string()).into()));
        params.insert("triggered_by".to_string(), DataValue::Str(opt_string(&input.triggered_by).into()));
        params.insert("ends_when".to_string(), DataValue::Str(opt_string(&input.ends_when).into()));
        params.insert("major_events".to_string(), DataValue::Str(major_events_json.into()));
        params.insert("arc_type".to_string(), DataValue::Str(opt_string(&input.arc_type).into()));
        params.insert("dominant_theme".to_string(), DataValue::Str(opt_string(&input.dominant_theme).into()));
        params.insert("protagonist_id".to_string(), DataValue::Str(opt_string(&input.protagonist_id).into()));
        params.insert("antagonist_id".to_string(), DataValue::Str(opt_string(&input.antagonist_id).into()));
        params.insert("summary".to_string(), DataValue::Str(opt_string(&input.summary).into()));
        params.insert("detailed_notes".to_string(), DataValue::Str(opt_string(&input.detailed_notes).into()));
        params.insert("show_on_timeline".to_string(), DataValue::Bool(input.show_on_timeline.unwrap_or(true)));
        params.insert("timeline_color".to_string(), DataValue::Str(opt_string(&input.timeline_color).into()));
        params.insert("timeline_icon".to_string(), DataValue::Str(opt_string(&input.timeline_icon).into()));
        params.insert("now".to_string(), DataValue::from(now));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to create period: {}", e))?;
        
        Self::get(db, &input.world_id, &id)?
            .ok_or_else(|| "Period created but not found".to_string())
    }
    
    /// Get a period by ID
    pub fn get(db: &DbInstance, world_id: &str, id: &str) -> Result<Option<Period>, String> {
        let query = r#"
            ?[id, world_id, calendar_id, name, description, start_year, start_month,
              end_year, end_month, parent_period_id, period_type, color, icon, abbreviation,
              direction, triggered_by, ends_when, major_events, arc_type, dominant_theme,
              protagonist_id, antagonist_id, summary, detailed_notes, show_on_timeline,
              timeline_color, timeline_icon, created_at, updated_at] :=
                *periods[id, world_id, calendar_id, name, description, start_year, start_month,
                         end_year, end_month, parent_period_id, period_type, color, icon, abbreviation,
                         direction, triggered_by, ends_when, major_events, arc_type, dominant_theme,
                         protagonist_id, antagonist_id, summary, detailed_notes, show_on_timeline,
                         timeline_color, timeline_icon, created_at, updated_at],
                id = $id,
                world_id = $world_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get period: {}", e))?;
        
        if let Some(row) = result.rows.first() {
            Ok(Some(Self::row_to_period(row)?))
        } else {
            Ok(None)
        }
    }
    
    /// List all periods
    pub fn list_all(db: &DbInstance, world_id: &str) -> Result<Vec<Period>, String> {
        let query = r#"
            ?[id, world_id, calendar_id, name, description, start_year, start_month,
              end_year, end_month, parent_period_id, period_type, color, icon, abbreviation,
              direction, triggered_by, ends_when, major_events, arc_type, dominant_theme,
              protagonist_id, antagonist_id, summary, detailed_notes, show_on_timeline,
              timeline_color, timeline_icon, created_at, updated_at] :=
                *periods[id, world_id, calendar_id, name, description, start_year, start_month,
                         end_year, end_month, parent_period_id, period_type, color, icon, abbreviation,
                         direction, triggered_by, ends_when, major_events, arc_type, dominant_theme,
                         protagonist_id, antagonist_id, summary, detailed_notes, show_on_timeline,
                         timeline_color, timeline_icon, created_at, updated_at],
                world_id = $world_id
            :order start_year
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list periods: {}", e))?;
        
        let mut periods = Vec::new();
        for row in &result.rows {
            periods.push(Self::row_to_period(row)?);
        }
        Ok(periods)
    }
    
    /// Get children of a period
    pub fn get_children(db: &DbInstance, world_id: &str, parent_id: &str) -> Result<Vec<Period>, String> {
        let query = r#"
            ?[id, world_id, calendar_id, name, description, start_year, start_month,
              end_year, end_month, parent_period_id, period_type, color, icon, abbreviation,
              direction, triggered_by, ends_when, major_events, arc_type, dominant_theme,
              protagonist_id, antagonist_id, summary, detailed_notes, show_on_timeline,
              timeline_color, timeline_icon, created_at, updated_at] :=
                *periods[id, world_id, calendar_id, name, description, start_year, start_month,
                         end_year, end_month, parent_period_id, period_type, color, icon, abbreviation,
                         direction, triggered_by, ends_when, major_events, arc_type, dominant_theme,
                         protagonist_id, antagonist_id, summary, detailed_notes, show_on_timeline,
                         timeline_color, timeline_icon, created_at, updated_at],
                world_id = $world_id,
                parent_period_id = $parent_id
            :order start_year
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        params.insert("parent_id".to_string(), DataValue::Str(parent_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get period children: {}", e))?;
        
        let mut periods = Vec::new();
        for row in &result.rows {
            periods.push(Self::row_to_period(row)?);
        }
        Ok(periods)
    }
    
    /// Delete a period
    pub fn delete(db: &DbInstance, _world_id: &str, id: &str) -> Result<bool, String> {
        let query = r#"
            ?[id] <- [[$id]]
            :rm periods { id }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to delete period: {}", e))?;
        
        Ok(true)
    }
    
    fn row_to_period(row: &[DataValue]) -> Result<Period, String> {
        if row.len() < 29 {
            return Err("Invalid row length for Period".to_string());
        }
        
        let major_events_str = extract_string(&row[17]);
        let major_events: Vec<String> = serde_json::from_str(&major_events_str).unwrap_or_default();
        
        Ok(Period {
            id: extract_string(&row[0]),
            world_id: extract_string(&row[1]),
            calendar_id: extract_string(&row[2]),
            name: extract_string(&row[3]),
            description: {
                let s = extract_string(&row[4]);
                if s.is_empty() { None } else { Some(s) }
            },
            start_year: extract_int(&row[5]),
            start_month: {
                let v = extract_int(&row[6]);
                if v == 0 { None } else { Some(v) }
            },
            end_year: {
                let v = extract_int(&row[7]);
                if v == 0 { None } else { Some(v) }
            },
            end_month: {
                let v = extract_int(&row[8]);
                if v == 0 { None } else { Some(v) }
            },
            parent_period_id: {
                let s = extract_string(&row[9]);
                if s.is_empty() { None } else { Some(s) }
            },
            period_type: extract_string(&row[10]),
            color: extract_string(&row[11]),
            icon: {
                let s = extract_string(&row[12]);
                if s.is_empty() { None } else { Some(s) }
            },
            abbreviation: {
                let s = extract_string(&row[13]);
                if s.is_empty() { None } else { Some(s) }
            },
            direction: extract_string(&row[14]),
            triggered_by: {
                let s = extract_string(&row[15]);
                if s.is_empty() { None } else { Some(s) }
            },
            ends_when: {
                let s = extract_string(&row[16]);
                if s.is_empty() { None } else { Some(s) }
            },
            major_events,
            arc_type: {
                let s = extract_string(&row[18]);
                if s.is_empty() { None } else { Some(s) }
            },
            dominant_theme: {
                let s = extract_string(&row[19]);
                if s.is_empty() { None } else { Some(s) }
            },
            protagonist_id: {
                let s = extract_string(&row[20]);
                if s.is_empty() { None } else { Some(s) }
            },
            antagonist_id: {
                let s = extract_string(&row[21]);
                if s.is_empty() { None } else { Some(s) }
            },
            summary: {
                let s = extract_string(&row[22]);
                if s.is_empty() { None } else { Some(s) }
            },
            detailed_notes: {
                let s = extract_string(&row[23]);
                if s.is_empty() { None } else { Some(s) }
            },
            show_on_timeline: extract_bool(&row[24]),
            timeline_color: {
                let s = extract_string(&row[25]);
                if s.is_empty() { None } else { Some(s) }
            },
            timeline_icon: {
                let s = extract_string(&row[26]);
                if s.is_empty() { None } else { Some(s) }
            },
            created_at: extract_float(&row[27]),
            updated_at: extract_float(&row[28]),
        })
    }
}

// =============================================================================
// FIELD BINDING REPO
// =============================================================================

pub struct FieldBindingRepo;

impl FieldBindingRepo {
    /// Create a field binding
    pub fn create(db: &DbInstance, input: FieldBindingInput) -> Result<FieldBinding, String> {
        let id = generate_id();
        let now = now();
        
        let transform_json = serde_json::to_string(&input.transform)
            .unwrap_or_else(|_| "null".to_string());
        let agg_filter_json = serde_json::to_string(&input.aggregation_filter)
            .unwrap_or_else(|_| "null".to_string());
        let binding_type_str = match input.binding_type {
            BindingType::Mirror => "mirror",
            BindingType::Inherit => "inherit",
            BindingType::Aggregate => "aggregate",
        };
        let agg_fn_str = input.aggregation_fn.as_ref().map(|f| match f {
            AggregationFunction::Sum => "sum",
            AggregationFunction::Avg => "avg",
            AggregationFunction::Min => "min",
            AggregationFunction::Max => "max",
            AggregationFunction::Count => "count",
            AggregationFunction::Concat => "concat",
            AggregationFunction::First => "first",
            AggregationFunction::Last => "last",
        }).unwrap_or("");
        
        let query = r#"
            ?[id, world_id, source_entity_id, source_field_name, target_entity_id, target_field_name,
              binding_type, transform, aggregation_fn, aggregation_filter, allow_override, is_active,
              created_at, updated_at] <- [[
                $id, $world_id, $source_entity_id, $source_field_name, $target_entity_id, $target_field_name,
                $binding_type, $transform, $aggregation_fn, $aggregation_filter, $allow_override, true,
                $now, $now
            ]]
            :put field_bindings {
                id, world_id, source_entity_id, source_field_name, target_entity_id, target_field_name,
                binding_type, transform, aggregation_fn, aggregation_filter, allow_override, is_active,
                created_at, updated_at
            }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("world_id".to_string(), DataValue::Str(input.world_id.clone().into()));
        params.insert("source_entity_id".to_string(), DataValue::Str(input.source_entity_id.into()));
        params.insert("source_field_name".to_string(), DataValue::Str(input.source_field_name.into()));
        params.insert("target_entity_id".to_string(), DataValue::Str(input.target_entity_id.into()));
        params.insert("target_field_name".to_string(), DataValue::Str(input.target_field_name.into()));
        params.insert("binding_type".to_string(), DataValue::Str(binding_type_str.into()));
        params.insert("transform".to_string(), DataValue::Str(transform_json.into()));
        params.insert("aggregation_fn".to_string(), DataValue::Str(agg_fn_str.into()));
        params.insert("aggregation_filter".to_string(), DataValue::Str(agg_filter_json.into()));
        params.insert("allow_override".to_string(), DataValue::Bool(input.allow_override.unwrap_or(false)));
        params.insert("now".to_string(), DataValue::from(now));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to create field_binding: {}", e))?;
        
        Self::get(db, &input.world_id, &id)?
            .ok_or_else(|| "FieldBinding created but not found".to_string())
    }
    
    /// Get a binding by ID
    pub fn get(db: &DbInstance, world_id: &str, id: &str) -> Result<Option<FieldBinding>, String> {
        let query = r#"
            ?[id, world_id, source_entity_id, source_field_name, target_entity_id, target_field_name,
              binding_type, transform, aggregation_fn, aggregation_filter, allow_override, is_active,
              created_at, updated_at] :=
                *field_bindings[id, world_id, source_entity_id, source_field_name, target_entity_id, target_field_name,
                                binding_type, transform, aggregation_fn, aggregation_filter, allow_override, is_active,
                                created_at, updated_at],
                id = $id,
                world_id = $world_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to get field_binding: {}", e))?;
        
        if let Some(row) = result.rows.first() {
            Ok(Some(Self::row_to_binding(row)?))
        } else {
            Ok(None)
        }
    }
    
    /// List all bindings
    pub fn list_all(db: &DbInstance, world_id: &str) -> Result<Vec<FieldBinding>, String> {
        let query = r#"
            ?[id, world_id, source_entity_id, source_field_name, target_entity_id, target_field_name,
              binding_type, transform, aggregation_fn, aggregation_filter, allow_override, is_active,
              created_at, updated_at] :=
                *field_bindings[id, world_id, source_entity_id, source_field_name, target_entity_id, target_field_name,
                                binding_type, transform, aggregation_fn, aggregation_filter, allow_override, is_active,
                                created_at, updated_at],
                world_id = $world_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list field_bindings: {}", e))?;
        
        let mut bindings = Vec::new();
        for row in &result.rows {
            bindings.push(Self::row_to_binding(row)?);
        }
        Ok(bindings)
    }
    
    /// List bindings for an entity
    pub fn list_by_entity(db: &DbInstance, world_id: &str, entity_id: &str) -> Result<Vec<FieldBinding>, String> {
        let query = r#"
            ?[id, world_id, source_entity_id, source_field_name, target_entity_id, target_field_name,
              binding_type, transform, aggregation_fn, aggregation_filter, allow_override, is_active,
              created_at, updated_at] :=
                *field_bindings[id, world_id, source_entity_id, source_field_name, target_entity_id, target_field_name,
                                binding_type, transform, aggregation_fn, aggregation_filter, allow_override, is_active,
                                created_at, updated_at],
                world_id = $world_id,
                or(source_entity_id = $entity_id, target_entity_id = $entity_id)
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("world_id".to_string(), DataValue::Str(world_id.into()));
        params.insert("entity_id".to_string(), DataValue::Str(entity_id.into()));
        
        let result = db.run_script(query, params, ScriptMutability::Immutable)
            .map_err(|e| format!("Failed to list field_bindings by entity: {}", e))?;
        
        let mut bindings = Vec::new();
        for row in &result.rows {
            bindings.push(Self::row_to_binding(row)?);
        }
        Ok(bindings)
    }
    
    /// Delete a binding
    pub fn delete(db: &DbInstance, _world_id: &str, id: &str) -> Result<bool, String> {
        let query = r#"
            ?[id] <- [[$id]]
            :rm field_bindings { id }
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        db.run_script(query, params, ScriptMutability::Mutable)
            .map_err(|e| format!("Failed to delete field_binding: {}", e))?;
        
        Ok(true)
    }
    
    fn row_to_binding(row: &[DataValue]) -> Result<FieldBinding, String> {
        if row.len() < 14 {
            return Err("Invalid row length for FieldBinding".to_string());
        }
        
        let binding_type_str = extract_string(&row[6]);
        let binding_type = match binding_type_str.as_str() {
            "mirror" => BindingType::Mirror,
            "aggregate" => BindingType::Aggregate,
            _ => BindingType::Inherit,
        };
        
        let transform_str = extract_string(&row[7]);
        let transform: Option<serde_json::Value> = if transform_str.is_empty() || transform_str == "null" {
            None
        } else {
            serde_json::from_str(&transform_str).ok()
        };
        
        let agg_fn_str = extract_string(&row[8]);
        let aggregation_fn = match agg_fn_str.as_str() {
            "sum" => Some(AggregationFunction::Sum),
            "avg" => Some(AggregationFunction::Avg),
            "min" => Some(AggregationFunction::Min),
            "max" => Some(AggregationFunction::Max),
            "count" => Some(AggregationFunction::Count),
            "concat" => Some(AggregationFunction::Concat),
            "first" => Some(AggregationFunction::First),
            "last" => Some(AggregationFunction::Last),
            _ => None,
        };
        
        let agg_filter_str = extract_string(&row[9]);
        let aggregation_filter: Option<serde_json::Value> = if agg_filter_str.is_empty() || agg_filter_str == "null" {
            None
        } else {
            serde_json::from_str(&agg_filter_str).ok()
        };
        
        Ok(FieldBinding {
            id: extract_string(&row[0]),
            world_id: extract_string(&row[1]),
            source_entity_id: extract_string(&row[2]),
            source_field_name: extract_string(&row[3]),
            target_entity_id: extract_string(&row[4]),
            target_field_name: extract_string(&row[5]),
            binding_type,
            transform,
            aggregation_fn,
            aggregation_filter,
            allow_override: extract_bool(&row[10]),
            is_active: extract_bool(&row[11]),
            created_at: extract_float(&row[12]),
            updated_at: extract_float(&row[13]),
        })
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::schema::{init_schema, relation_exists};
    
    fn setup_test_db() -> DbInstance {
        let db = DbInstance::new("mem", "", Default::default()).unwrap();
        init_schema(&db).unwrap();
        db
    }
    
    #[test]
    fn test_note_schema_exists() {
        let db = setup_test_db();
        assert!(relation_exists(&db, "notes"));
    }
    
    #[test]
    fn test_folder_schema_exists() {
        let db = setup_test_db();
        assert!(relation_exists(&db, "folders"));
    }
    
    #[test]
    fn test_create_note() {
        let db = setup_test_db();
        
        let input = NoteInput {
            world_id: "test-world".to_string(),
            title: "Test Note".to_string(),
            content: Some("Hello world".to_string()),
            ..Default::default()
        };
        
        let note = NoteRepo::create(&db, input).unwrap();
        
        assert!(!note.id.is_empty());
        assert_eq!(note.title, "Test Note");
        assert_eq!(note.content, "Hello world");
        assert_eq!(note.world_id, "test-world");
    }
    
    #[test]
    fn test_get_note() {
        let db = setup_test_db();
        
        let input = NoteInput {
            world_id: "test-world".to_string(),
            title: "Findable Note".to_string(),
            ..Default::default()
        };
        
        let created = NoteRepo::create(&db, input).unwrap();
        let found = NoteRepo::get(&db, "test-world", &created.id).unwrap();
        
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Findable Note");
    }
    
    #[test]
    fn test_list_notes() {
        let db = setup_test_db();
        
        for i in 0..3 {
            let input = NoteInput {
                world_id: "test-world".to_string(),
                title: format!("Note {}", i),
                ..Default::default()
            };
            NoteRepo::create(&db, input).unwrap();
        }
        
        let notes = NoteRepo::list_all(&db, "test-world").unwrap();
        assert_eq!(notes.len(), 3);
    }
    
    #[test]
    fn test_update_note() {
        let db = setup_test_db();
        
        let input = NoteInput {
            world_id: "test-world".to_string(),
            title: "Original Title".to_string(),
            ..Default::default()
        };
        
        let created = NoteRepo::create(&db, input).unwrap();
        
        let update = NoteUpdate {
            title: Some("Updated Title".to_string()),
            content: Some("Updated content".to_string()),
            ..Default::default()
        };
        
        let updated = NoteRepo::update(&db, "test-world", &created.id, update).unwrap();
        
        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.content, "Updated content");
    }
    
    #[test]
    fn test_delete_note() {
        let db = setup_test_db();
        
        let input = NoteInput {
            world_id: "test-world".to_string(),
            title: "Deletable".to_string(),
            ..Default::default()
        };
        
        let created = NoteRepo::create(&db, input).unwrap();
        let deleted = NoteRepo::delete(&db, "test-world", &created.id).unwrap();
        
        assert!(deleted);
        assert!(NoteRepo::get(&db, "test-world", &created.id).unwrap().is_none());
    }
    
    #[test]
    fn test_create_folder() {
        let db = setup_test_db();
        
        let input = FolderInput {
            world_id: "test-world".to_string(),
            name: "Test Folder".to_string(),
            ..Default::default()
        };
        
        let folder = FolderRepo::create(&db, input).unwrap();
        
        assert!(!folder.id.is_empty());
        assert_eq!(folder.name, "Test Folder");
    }
    
    #[test]
    fn test_folder_tree() {
        let db = setup_test_db();
        
        // Create parent
        let parent = FolderRepo::create(&db, FolderInput {
            world_id: "test-world".to_string(),
            name: "Parent".to_string(),
            ..Default::default()
        }).unwrap();
        
        // Create child
        FolderRepo::create(&db, FolderInput {
            world_id: "test-world".to_string(),
            name: "Child".to_string(),
            parent_id: Some(parent.id.clone()),
            ..Default::default()
        }).unwrap();
        
        // Create note in parent
        NoteRepo::create(&db, NoteInput {
            world_id: "test-world".to_string(),
            title: "Note in Parent".to_string(),
            folder_id: Some(parent.id.clone()),
            ..Default::default()
        }).unwrap();
        
        let tree = FolderRepo::get_tree(&db, "test-world").unwrap();
        
        assert_eq!(tree.len(), 1); // One root
        assert_eq!(tree[0].folder.name, "Parent");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].folder.name, "Child");
        assert_eq!(tree[0].notes.len(), 1);
    }
}
