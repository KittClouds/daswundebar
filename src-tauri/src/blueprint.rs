//! Blueprint Hub - CozoDB Schema and Commands
//!
//! Stores blueprint definitions for the Blueprint Hub feature.
//! Blueprints define entity types, relationship types, and extraction profiles.

use cozo::DbInstance;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// =============================================================================
// Types
// =============================================================================

#[taurpc::ipc_type]
pub struct BlueprintMeta {
    pub blueprint_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub is_system: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[taurpc::ipc_type]
pub struct BlueprintVersion {
    pub version_id: String,
    pub blueprint_id: String,
    pub version_number: i32,
    pub status: String, // draft | published | archived | deprecated
    pub change_summary: Option<String>,
    pub published_at: Option<i64>,
    pub created_at: i64,
}

#[taurpc::ipc_type]
pub struct EntityTypeDef {
    pub entity_type_id: String,
    pub version_id: String,
    pub entity_kind: String,
    pub entity_subtype: Option<String>,
    pub display_name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub is_abstract: bool,
    pub parent_type_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldDef {
    pub field_id: String,
    pub entity_type_id: String,
    pub field_name: String,
    pub display_label: String,
    pub data_type: String,
    pub is_required: bool,
    pub is_array: bool,
    pub default_value: Option<String>,
    pub validation_rules: Option<serde_json::Value>,
    pub ui_hints: Option<serde_json::Value>,
    pub display_order: i32,
    pub group_name: Option<String>,
    pub description: Option<String>,
    pub created_at: i64,
}

#[taurpc::ipc_type]
pub struct RelationshipTypeDef {
    pub relationship_type_id: String,
    pub version_id: String,
    pub relationship_name: String,
    pub display_label: String,
    pub source_entity_kind: String,
    pub target_entity_kind: String,
    pub direction: String,
    pub cardinality: String,
    pub is_symmetric: bool,
    pub inverse_label: Option<String>,
    pub description: Option<String>,
    pub verb_patterns: Option<Vec<String>>,
    pub confidence: Option<f64>,
    pub pattern_category: Option<String>,
    pub created_at: i64,
}

// =============================================================================
// Schema Creation
// =============================================================================

/// Initialize all blueprint-related CozoDB relations
pub fn init_blueprint_schema(db: &DbInstance) -> Result<(), String> {
    // BlueprintMeta table
    let meta_query = r#"
        :create blueprint_meta {
            blueprint_id: String =>
            name: String,
            description: String default '',
            category: String default '',
            author: String default '',
            tags: String default '[]',
            is_system: Bool default false,
            created_at: Int,
            updated_at: Int
        }
    "#;

    // BlueprintVersion table
    let version_query = r#"
        :create blueprint_version {
            version_id: String =>
            blueprint_id: String,
            version_number: Int,
            status: String,
            change_summary: String default '',
            published_at: Int default 0,
            created_at: Int
        }
    "#;

    // EntityTypeDef table
    let entity_type_query = r#"
        :create blueprint_entity_type {
            entity_type_id: String =>
            version_id: String,
            entity_kind: String,
            entity_subtype: String default '',
            display_name: String,
            description: String default '',
            icon: String default '',
            color: String default '',
            is_abstract: Bool default false,
            parent_type_id: String default '',
            created_at: Int
        }
    "#;

    // FieldDef table
    let field_query = r#"
        :create blueprint_field {
            field_id: String =>
            entity_type_id: String,
            field_name: String,
            display_label: String,
            data_type: String,
            is_required: Bool default false,
            is_array: Bool default false,
            default_value: String default '',
            validation_rules: String default '{}',
            ui_hints: String default '{}',
            display_order: Int default 0,
            group_name: String default '',
            description: String default '',
            created_at: Int
        }
    "#;

    // RelationshipTypeDef table
    let rel_type_query = r#"
        :create blueprint_relationship_type {
            relationship_type_id: String =>
            version_id: String,
            relationship_name: String,
            display_label: String,
            source_entity_kind: String,
            target_entity_kind: String,
            direction: String default 'directed',
            cardinality: String default 'many_to_many',
            is_symmetric: Bool default false,
            inverse_label: String default '',
            description: String default '',
            verb_patterns: String default '[]',
            confidence: Float default 1.0,
            pattern_category: String default '',
            created_at: Int
        }
    "#;

    // Helper to check if relation exists
    let relation_exists = |db: &DbInstance, name: &str| -> bool {
        let query = "::relations".to_string();
        match db.run_script(&query, BTreeMap::new(), cozo::ScriptMutability::Immutable) {
            Ok(result) => {
                for row in result.rows {
                    if let Some(cozo::DataValue::Str(rel_name)) = row.first() {
                        if rel_name.as_str() == name {
                            return true;
                        }
                    }
                }
                false
            }
            Err(_) => false,
        }
    };

    // Execute each only if not exists
    let schemas = [
        ("blueprint_meta", meta_query),
        ("blueprint_version", version_query),
        ("blueprint_entity_type", entity_type_query),
        ("blueprint_field", field_query),
        ("blueprint_relationship_type", rel_type_query),
    ];

    for (name, query) in schemas {
        if !relation_exists(db, name) {
            db.run_script(query, BTreeMap::new(), cozo::ScriptMutability::Mutable)
                .map_err(|e| format!("Failed to create {}: {}", name, e))?;
            log::info!("[Blueprint] Created relation: {}", name);
        }
    }

    log::info!("[Blueprint] Schema initialized");
    Ok(())
}

// =============================================================================
// Blueprint Meta CRUD
// =============================================================================

pub fn create_blueprint_meta(db: &DbInstance, meta: &BlueprintMeta) -> Result<(), String> {
    let tags_json = serde_json::to_string(&meta.tags).unwrap_or_else(|_| "[]".to_string());
    
    let query = r#"
        ?[blueprint_id, name, description, category, author, tags, is_system, created_at, updated_at] <- [[
            $blueprint_id, $name, $description, $category, $author, $tags, $is_system, $created_at, $updated_at
        ]]
        :put blueprint_meta {
            blueprint_id => name, description, category, author, tags, is_system, created_at, updated_at
        }
    "#;

    let mut params = BTreeMap::new();
    params.insert("blueprint_id".to_string(), cozo::DataValue::Str(meta.blueprint_id.clone().into()));
    params.insert("name".to_string(), cozo::DataValue::Str(meta.name.clone().into()));
    params.insert("description".to_string(), cozo::DataValue::Str(meta.description.clone().unwrap_or_default().into()));
    params.insert("category".to_string(), cozo::DataValue::Str(meta.category.clone().unwrap_or_default().into()));
    params.insert("author".to_string(), cozo::DataValue::Str(meta.author.clone().unwrap_or_default().into()));
    params.insert("tags".to_string(), cozo::DataValue::Str(tags_json.into()));
    params.insert("is_system".to_string(), cozo::DataValue::Bool(meta.is_system));
    params.insert("created_at".to_string(), cozo::DataValue::Num(cozo::Num::Int(meta.created_at)));
    params.insert("updated_at".to_string(), cozo::DataValue::Num(cozo::Num::Int(meta.updated_at)));

    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to create blueprint: {}", e))?;

    Ok(())
}

pub fn get_blueprint_meta(db: &DbInstance, blueprint_id: &str) -> Result<Option<BlueprintMeta>, String> {
    let query = r#"
        ?[blueprint_id, name, description, category, author, tags, is_system, created_at, updated_at] := 
            *blueprint_meta{blueprint_id, name, description, category, author, tags, is_system, created_at, updated_at},
            blueprint_id == $id
    "#;

    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(blueprint_id.into()));

    let result = db.run_script(query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get blueprint: {}", e))?;

    if result.rows.is_empty() {
        return Ok(None);
    }

    let row = &result.rows[0];
    let empty_to_none = |s: &str| if s.is_empty() { None } else { Some(s.to_string()) };
    
    Ok(Some(BlueprintMeta {
        blueprint_id: row[0].get_str().unwrap_or_default().to_string(),
        name: row[1].get_str().unwrap_or_default().to_string(),
        description: row[2].get_str().and_then(|s| empty_to_none(s)),
        category: row[3].get_str().and_then(|s| empty_to_none(s)),
        author: row[4].get_str().and_then(|s| empty_to_none(s)),
        tags: serde_json::from_str(row[5].get_str().unwrap_or("[]")).unwrap_or_default(),
        is_system: row[6].get_bool().unwrap_or(false),
        created_at: row[7].get_int().unwrap_or(0),
        updated_at: row[8].get_int().unwrap_or(0),
    }))
}

pub fn list_blueprint_metas(db: &DbInstance) -> Result<Vec<BlueprintMeta>, String> {
    let query = r#"
        ?[blueprint_id, name, description, category, author, tags, is_system, created_at, updated_at] := 
            *blueprint_meta{blueprint_id, name, description, category, author, tags, is_system, created_at, updated_at}
    "#;

    let result = db.run_script(query, BTreeMap::new(), cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to list blueprints: {}", e))?;

    let empty_to_none = |s: &str| if s.is_empty() { None } else { Some(s.to_string()) };
    let mut metas = Vec::new();
    for row in &result.rows {
        metas.push(BlueprintMeta {
            blueprint_id: row[0].get_str().unwrap_or_default().to_string(),
            name: row[1].get_str().unwrap_or_default().to_string(),
            description: row[2].get_str().and_then(|s| empty_to_none(s)),
            category: row[3].get_str().and_then(|s| empty_to_none(s)),
            author: row[4].get_str().and_then(|s| empty_to_none(s)),
            tags: serde_json::from_str(row[5].get_str().unwrap_or("[]")).unwrap_or_default(),
            is_system: row[6].get_bool().unwrap_or(false),
            created_at: row[7].get_int().unwrap_or(0),
            updated_at: row[8].get_int().unwrap_or(0),
        });
    }

    Ok(metas)
}

pub fn delete_blueprint_meta(db: &DbInstance, blueprint_id: &str) -> Result<(), String> {
    let query = r#"
        ?[blueprint_id] := blueprint_id = $id
        :rm blueprint_meta { blueprint_id }
    "#;

    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(blueprint_id.into()));

    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to delete blueprint: {}", e))?;

    Ok(())
}

// =============================================================================
// Version CRUD
// =============================================================================

pub fn create_version(db: &DbInstance, version: &BlueprintVersion) -> Result<(), String> {
    let query = r#"
        ?[version_id, blueprint_id, version_number, status, change_summary, published_at, created_at] <- [[
            $version_id, $blueprint_id, $version_number, $status, $change_summary, $published_at, $created_at
        ]]
        :put blueprint_version {
            version_id => blueprint_id, version_number, status, change_summary, published_at, created_at
        }
    "#;

    let mut params = BTreeMap::new();
    params.insert("version_id".to_string(), cozo::DataValue::Str(version.version_id.clone().into()));
    params.insert("blueprint_id".to_string(), cozo::DataValue::Str(version.blueprint_id.clone().into()));
    params.insert("version_number".to_string(), cozo::DataValue::Num(cozo::Num::Int(version.version_number as i64)));
    params.insert("status".to_string(), cozo::DataValue::Str(version.status.clone().into()));
    params.insert("change_summary".to_string(), cozo::DataValue::Str(version.change_summary.clone().unwrap_or_default().into()));
    params.insert("published_at".to_string(), cozo::DataValue::Num(cozo::Num::Int(version.published_at.unwrap_or(0))));
    params.insert("created_at".to_string(), cozo::DataValue::Num(cozo::Num::Int(version.created_at)));

    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to create version: {}", e))?;
    Ok(())
}

pub fn get_versions_by_blueprint(db: &DbInstance, blueprint_id: &str) -> Result<Vec<BlueprintVersion>, String> {
    let query = r#"
        ?[version_id, blueprint_id, version_number, status, change_summary, published_at, created_at] := 
            *blueprint_version{version_id, blueprint_id, version_number, status, change_summary, published_at, created_at},
            blueprint_id == $id
    "#;

    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(blueprint_id.into()));

    let result = db.run_script(query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get versions: {}", e))?;

    let empty_to_none = |s: &str| if s.is_empty() { None } else { Some(s.to_string()) };
    let mut versions = Vec::new();
    for row in &result.rows {
        versions.push(BlueprintVersion {
            version_id: row[0].get_str().unwrap_or_default().to_string(),
            blueprint_id: row[1].get_str().unwrap_or_default().to_string(),
            version_number: row[2].get_int().unwrap_or(0) as i32,
            status: row[3].get_str().unwrap_or_default().to_string(),
            change_summary: row[4].get_str().and_then(|s| empty_to_none(s)),
            published_at: { let v = row[5].get_int().unwrap_or(0); if v == 0 { None } else { Some(v) } },
            created_at: row[6].get_int().unwrap_or(0),
        });
    }
    Ok(versions)
}

pub fn delete_version(db: &DbInstance, version_id: &str) -> Result<(), String> {
    let query = r#"
        ?[version_id] := version_id = $id
        :rm blueprint_version { version_id }
    "#;
    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(version_id.into()));
    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to delete version: {}", e))?;
    Ok(())
}

// =============================================================================
// EntityType CRUD
// =============================================================================

pub fn create_entity_type(db: &DbInstance, et: &EntityTypeDef) -> Result<(), String> {
    let query = r#"
        ?[entity_type_id, version_id, entity_kind, entity_subtype, display_name, description, icon, color, is_abstract, parent_type_id, created_at] <- [[
            $entity_type_id, $version_id, $entity_kind, $entity_subtype, $display_name, $description, $icon, $color, $is_abstract, $parent_type_id, $created_at
        ]]
        :put blueprint_entity_type {
            entity_type_id => version_id, entity_kind, entity_subtype, display_name, description, icon, color, is_abstract, parent_type_id, created_at
        }
    "#;

    let mut params = BTreeMap::new();
    params.insert("entity_type_id".to_string(), cozo::DataValue::Str(et.entity_type_id.clone().into()));
    params.insert("version_id".to_string(), cozo::DataValue::Str(et.version_id.clone().into()));
    params.insert("entity_kind".to_string(), cozo::DataValue::Str(et.entity_kind.clone().into()));
    params.insert("entity_subtype".to_string(), cozo::DataValue::Str(et.entity_subtype.clone().unwrap_or_default().into()));
    params.insert("display_name".to_string(), cozo::DataValue::Str(et.display_name.clone().into()));
    params.insert("description".to_string(), cozo::DataValue::Str(et.description.clone().unwrap_or_default().into()));
    params.insert("icon".to_string(), cozo::DataValue::Str(et.icon.clone().unwrap_or_default().into()));
    params.insert("color".to_string(), cozo::DataValue::Str(et.color.clone().unwrap_or_default().into()));
    params.insert("is_abstract".to_string(), cozo::DataValue::Bool(et.is_abstract));
    params.insert("parent_type_id".to_string(), cozo::DataValue::Str(et.parent_type_id.clone().unwrap_or_default().into()));
    params.insert("created_at".to_string(), cozo::DataValue::Num(cozo::Num::Int(et.created_at)));

    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to create entity type: {}", e))?;
    Ok(())
}

pub fn get_entity_types_by_version(db: &DbInstance, version_id: &str) -> Result<Vec<EntityTypeDef>, String> {
    let query = r#"
        ?[entity_type_id, version_id, entity_kind, entity_subtype, display_name, description, icon, color, is_abstract, parent_type_id, created_at] := 
            *blueprint_entity_type{entity_type_id, version_id, entity_kind, entity_subtype, display_name, description, icon, color, is_abstract, parent_type_id, created_at},
            version_id == $id
    "#;

    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(version_id.into()));

    let result = db.run_script(query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get entity types: {}", e))?;

    let empty_to_none = |s: &str| if s.is_empty() { None } else { Some(s.to_string()) };
    let mut types = Vec::new();
    for row in &result.rows {
        types.push(EntityTypeDef {
            entity_type_id: row[0].get_str().unwrap_or_default().to_string(),
            version_id: row[1].get_str().unwrap_or_default().to_string(),
            entity_kind: row[2].get_str().unwrap_or_default().to_string(),
            entity_subtype: row[3].get_str().and_then(|s| empty_to_none(s)),
            display_name: row[4].get_str().unwrap_or_default().to_string(),
            description: row[5].get_str().and_then(|s| empty_to_none(s)),
            icon: row[6].get_str().and_then(|s| empty_to_none(s)),
            color: row[7].get_str().and_then(|s| empty_to_none(s)),
            is_abstract: row[8].get_bool().unwrap_or(false),
            parent_type_id: row[9].get_str().and_then(|s| empty_to_none(s)),
            created_at: row[10].get_int().unwrap_or(0),
        });
    }
    Ok(types)
}

pub fn delete_entity_type(db: &DbInstance, entity_type_id: &str) -> Result<(), String> {
    let query = r#"
        ?[entity_type_id] := entity_type_id = $id
        :rm blueprint_entity_type { entity_type_id }
    "#;
    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(entity_type_id.into()));
    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to delete entity type: {}", e))?;
    Ok(())
}

// =============================================================================
// Field CRUD
// =============================================================================

pub fn create_field(db: &DbInstance, field: &FieldDef) -> Result<(), String> {
    let validation_json = field.validation_rules.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string());
    let hints_json = field.ui_hints.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string());

    let query = r#"
        ?[field_id, entity_type_id, field_name, display_label, data_type, is_required, is_array, default_value, validation_rules, ui_hints, display_order, group_name, description, created_at] <- [[
            $field_id, $entity_type_id, $field_name, $display_label, $data_type, $is_required, $is_array, $default_value, $validation_rules, $ui_hints, $display_order, $group_name, $description, $created_at
        ]]
        :put blueprint_field {
            field_id => entity_type_id, field_name, display_label, data_type, is_required, is_array, default_value, validation_rules, ui_hints, display_order, group_name, description, created_at
        }
    "#;

    let mut params = BTreeMap::new();
    params.insert("field_id".to_string(), cozo::DataValue::Str(field.field_id.clone().into()));
    params.insert("entity_type_id".to_string(), cozo::DataValue::Str(field.entity_type_id.clone().into()));
    params.insert("field_name".to_string(), cozo::DataValue::Str(field.field_name.clone().into()));
    params.insert("display_label".to_string(), cozo::DataValue::Str(field.display_label.clone().into()));
    params.insert("data_type".to_string(), cozo::DataValue::Str(field.data_type.clone().into()));
    params.insert("is_required".to_string(), cozo::DataValue::Bool(field.is_required));
    params.insert("is_array".to_string(), cozo::DataValue::Bool(field.is_array));
    params.insert("default_value".to_string(), cozo::DataValue::Str(field.default_value.clone().unwrap_or_default().into()));
    params.insert("validation_rules".to_string(), cozo::DataValue::Str(validation_json.into()));
    params.insert("ui_hints".to_string(), cozo::DataValue::Str(hints_json.into()));
    params.insert("display_order".to_string(), cozo::DataValue::Num(cozo::Num::Int(field.display_order as i64)));
    params.insert("group_name".to_string(), cozo::DataValue::Str(field.group_name.clone().unwrap_or_default().into()));
    params.insert("description".to_string(), cozo::DataValue::Str(field.description.clone().unwrap_or_default().into()));
    params.insert("created_at".to_string(), cozo::DataValue::Num(cozo::Num::Int(field.created_at)));

    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to create field: {}", e))?;
    Ok(())
}

pub fn get_fields_by_entity_type(db: &DbInstance, entity_type_id: &str) -> Result<Vec<FieldDef>, String> {
    let query = r#"
        ?[field_id, entity_type_id, field_name, display_label, data_type, is_required, is_array, default_value, validation_rules, ui_hints, display_order, group_name, description, created_at] := 
            *blueprint_field{field_id, entity_type_id, field_name, display_label, data_type, is_required, is_array, default_value, validation_rules, ui_hints, display_order, group_name, description, created_at},
            entity_type_id == $id
    "#;

    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(entity_type_id.into()));

    let result = db.run_script(query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get fields: {}", e))?;

    let empty_to_none = |s: &str| if s.is_empty() { None } else { Some(s.to_string()) };
    let mut fields = Vec::new();
    for row in &result.rows {
        fields.push(FieldDef {
            field_id: row[0].get_str().unwrap_or_default().to_string(),
            entity_type_id: row[1].get_str().unwrap_or_default().to_string(),
            field_name: row[2].get_str().unwrap_or_default().to_string(),
            display_label: row[3].get_str().unwrap_or_default().to_string(),
            data_type: row[4].get_str().unwrap_or_default().to_string(),
            is_required: row[5].get_bool().unwrap_or(false),
            is_array: row[6].get_bool().unwrap_or(false),
            default_value: row[7].get_str().and_then(|s| empty_to_none(s)),
            validation_rules: row[8].get_str().and_then(|s| serde_json::from_str(s).ok()),
            ui_hints: row[9].get_str().and_then(|s| serde_json::from_str(s).ok()),
            display_order: row[10].get_int().unwrap_or(0) as i32,
            group_name: row[11].get_str().and_then(|s| empty_to_none(s)),
            description: row[12].get_str().and_then(|s| empty_to_none(s)),
            created_at: row[13].get_int().unwrap_or(0),
        });
    }
    Ok(fields)
}

pub fn delete_field(db: &DbInstance, field_id: &str) -> Result<(), String> {
    let query = r#"
        ?[field_id] := field_id = $id
        :rm blueprint_field { field_id }
    "#;
    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(field_id.into()));
    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to delete field: {}", e))?;
    Ok(())
}

// =============================================================================
// RelationshipType CRUD
// =============================================================================

pub fn create_relationship_type(db: &DbInstance, rt: &RelationshipTypeDef) -> Result<(), String> {
    let patterns_json = serde_json::to_string(&rt.verb_patterns.clone().unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());

    let query = r#"
        ?[relationship_type_id, version_id, relationship_name, display_label, source_entity_kind, target_entity_kind, direction, cardinality, is_symmetric, inverse_label, description, verb_patterns, confidence, pattern_category, created_at] <- [[
            $relationship_type_id, $version_id, $relationship_name, $display_label, $source_entity_kind, $target_entity_kind, $direction, $cardinality, $is_symmetric, $inverse_label, $description, $verb_patterns, $confidence, $pattern_category, $created_at
        ]]
        :put blueprint_relationship_type {
            relationship_type_id => version_id, relationship_name, display_label, source_entity_kind, target_entity_kind, direction, cardinality, is_symmetric, inverse_label, description, verb_patterns, confidence, pattern_category, created_at
        }
    "#;

    let mut params = BTreeMap::new();
    params.insert("relationship_type_id".to_string(), cozo::DataValue::Str(rt.relationship_type_id.clone().into()));
    params.insert("version_id".to_string(), cozo::DataValue::Str(rt.version_id.clone().into()));
    params.insert("relationship_name".to_string(), cozo::DataValue::Str(rt.relationship_name.clone().into()));
    params.insert("display_label".to_string(), cozo::DataValue::Str(rt.display_label.clone().into()));
    params.insert("source_entity_kind".to_string(), cozo::DataValue::Str(rt.source_entity_kind.clone().into()));
    params.insert("target_entity_kind".to_string(), cozo::DataValue::Str(rt.target_entity_kind.clone().into()));
    params.insert("direction".to_string(), cozo::DataValue::Str(rt.direction.clone().into()));
    params.insert("cardinality".to_string(), cozo::DataValue::Str(rt.cardinality.clone().into()));
    params.insert("is_symmetric".to_string(), cozo::DataValue::Bool(rt.is_symmetric));
    params.insert("inverse_label".to_string(), cozo::DataValue::Str(rt.inverse_label.clone().unwrap_or_default().into()));
    params.insert("description".to_string(), cozo::DataValue::Str(rt.description.clone().unwrap_or_default().into()));
    params.insert("verb_patterns".to_string(), cozo::DataValue::Str(patterns_json.into()));
    params.insert("confidence".to_string(), cozo::DataValue::from(rt.confidence.unwrap_or(1.0)));
    params.insert("pattern_category".to_string(), cozo::DataValue::Str(rt.pattern_category.clone().unwrap_or_default().into()));
    params.insert("created_at".to_string(), cozo::DataValue::Num(cozo::Num::Int(rt.created_at)));

    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to create relationship type: {}", e))?;
    Ok(())
}

pub fn get_relationship_types_by_version(db: &DbInstance, version_id: &str) -> Result<Vec<RelationshipTypeDef>, String> {
    let query = r#"
        ?[relationship_type_id, version_id, relationship_name, display_label, source_entity_kind, target_entity_kind, direction, cardinality, is_symmetric, inverse_label, description, verb_patterns, confidence, pattern_category, created_at] := 
            *blueprint_relationship_type{relationship_type_id, version_id, relationship_name, display_label, source_entity_kind, target_entity_kind, direction, cardinality, is_symmetric, inverse_label, description, verb_patterns, confidence, pattern_category, created_at},
            version_id == $id
    "#;

    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(version_id.into()));

    let result = db.run_script(query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get relationship types: {}", e))?;

    let empty_to_none = |s: &str| if s.is_empty() { None } else { Some(s.to_string()) };
    let mut types = Vec::new();
    for row in &result.rows {
        types.push(RelationshipTypeDef {
            relationship_type_id: row[0].get_str().unwrap_or_default().to_string(),
            version_id: row[1].get_str().unwrap_or_default().to_string(),
            relationship_name: row[2].get_str().unwrap_or_default().to_string(),
            display_label: row[3].get_str().unwrap_or_default().to_string(),
            source_entity_kind: row[4].get_str().unwrap_or_default().to_string(),
            target_entity_kind: row[5].get_str().unwrap_or_default().to_string(),
            direction: row[6].get_str().unwrap_or_default().to_string(),
            cardinality: row[7].get_str().unwrap_or_default().to_string(),
            is_symmetric: row[8].get_bool().unwrap_or(false),
            inverse_label: row[9].get_str().and_then(|s| empty_to_none(s)),
            description: row[10].get_str().and_then(|s| empty_to_none(s)),
            verb_patterns: row[11].get_str().and_then(|s| serde_json::from_str(s).ok()),
            confidence: { let v = row[12].get_float(); if v.map(|f| f == 0.0).unwrap_or(true) { None } else { v } },
            pattern_category: row[13].get_str().and_then(|s| empty_to_none(s)),
            created_at: row[14].get_int().unwrap_or(0),
        });
    }
    Ok(types)
}

pub fn delete_relationship_type(db: &DbInstance, relationship_type_id: &str) -> Result<(), String> {
    let query = r#"
        ?[relationship_type_id] := relationship_type_id = $id
        :rm blueprint_relationship_type { relationship_type_id }
    "#;
    let mut params = BTreeMap::new();
    params.insert("id".to_string(), cozo::DataValue::Str(relationship_type_id.into()));
    db.run_script(query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to delete relationship type: {}", e))?;
    Ok(())
}

// =============================================================================
// Tauri Commands
// =============================================================================

pub mod commands {
    use super::*;
    use crate::graph::commands::GRAPH_REGISTRY;

    /// Create blueprint input
    #[taurpc::ipc_type]
    pub struct CreateBlueprintInput {
        pub name: String,
        pub description: Option<String>,
        pub category: Option<String>,
        pub author: Option<String>,
        pub tags: Option<Vec<String>>,
        pub is_system: Option<bool>,
    }

    /// Initialize blueprint schema (called during app startup)
    #[tauri::command]
    pub fn blueprint_init() -> Result<(), String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        init_blueprint_schema(registry.db())
    }

    /// Create a new blueprint
    #[tauri::command]
    pub fn blueprint_create(input: CreateBlueprintInput) -> Result<BlueprintMeta, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();
        let id = format!("bp-{}", uuid::Uuid::new_v4());

        let meta = BlueprintMeta {
            blueprint_id: id,
            name: input.name,
            description: input.description,
            category: input.category,
            author: input.author,
            tags: input.tags.unwrap_or_default(),
            is_system: input.is_system.unwrap_or(false),
            created_at: now,
            updated_at: now,
        };

        create_blueprint_meta(registry.db(), &meta)?;
        log::info!("[Blueprint] Created: {}", meta.blueprint_id);
        Ok(meta)
    }

    /// Get a blueprint by ID
    #[tauri::command]
    pub fn blueprint_get(blueprint_id: String) -> Result<Option<BlueprintMeta>, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        get_blueprint_meta(registry.db(), &blueprint_id)
    }

    /// List all blueprints
    #[tauri::command]
    pub fn blueprint_list() -> Result<Vec<BlueprintMeta>, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        list_blueprint_metas(registry.db())
    }

    /// Update a blueprint
    #[tauri::command]
    pub fn blueprint_update(
        blueprint_id: String,
        updates: CreateBlueprintInput,
    ) -> Result<BlueprintMeta, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();

        // Get existing
        let existing = get_blueprint_meta(registry.db(), &blueprint_id)?
            .ok_or_else(|| format!("Blueprint not found: {}", blueprint_id))?;

        // Merge updates
        let updated = BlueprintMeta {
            blueprint_id: existing.blueprint_id,
            name: updates.name,
            description: updates.description.or(existing.description),
            category: updates.category.or(existing.category),
            author: updates.author.or(existing.author),
            tags: updates.tags.unwrap_or(existing.tags),
            is_system: updates.is_system.unwrap_or(existing.is_system),
            created_at: existing.created_at,
            updated_at: now,
        };

        create_blueprint_meta(registry.db(), &updated)?; // put = upsert
        log::info!("[Blueprint] Updated: {}", blueprint_id);
        Ok(updated)
    }

    /// Delete a blueprint
    #[tauri::command]
    pub fn blueprint_delete(blueprint_id: String) -> Result<(), String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        delete_blueprint_meta(registry.db(), &blueprint_id)?;
        log::info!("[Blueprint] Deleted: {}", blueprint_id);
        Ok(())
    }

    // =========================================================================
    // Version Commands
    // =========================================================================

    #[taurpc::ipc_type]
    pub struct CreateVersionInput {
        pub blueprint_id: String,
        pub version_number: i32,
        pub status: String,
        pub change_summary: Option<String>,
    }

    #[tauri::command]
    pub fn blueprint_version_create(input: CreateVersionInput) -> Result<BlueprintVersion, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();
        let id = format!("ver-{}", uuid::Uuid::new_v4());

        let version = BlueprintVersion {
            version_id: id,
            blueprint_id: input.blueprint_id,
            version_number: input.version_number,
            status: input.status,
            change_summary: input.change_summary,
            published_at: None,
            created_at: now,
        };

        create_version(registry.db(), &version)?;
        Ok(version)
    }

    #[tauri::command]
    pub fn blueprint_version_list(blueprint_id: String) -> Result<Vec<BlueprintVersion>, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        get_versions_by_blueprint(registry.db(), &blueprint_id)
    }

    #[tauri::command]
    pub fn blueprint_version_delete(version_id: String) -> Result<(), String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        delete_version(registry.db(), &version_id)
    }

    // =========================================================================
    // EntityType Commands
    // =========================================================================

    #[taurpc::ipc_type]
    pub struct CreateEntityTypeInput {
        pub version_id: String,
        pub entity_kind: String,
        pub entity_subtype: Option<String>,
        pub display_name: String,
        pub description: Option<String>,
        pub icon: Option<String>,
        pub color: Option<String>,
        pub is_abstract: Option<bool>,
        pub parent_type_id: Option<String>,
    }

    #[tauri::command]
    pub fn blueprint_entity_type_create(input: CreateEntityTypeInput) -> Result<EntityTypeDef, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();
        let id = format!("et-{}", uuid::Uuid::new_v4());

        let entity_type = EntityTypeDef {
            entity_type_id: id,
            version_id: input.version_id,
            entity_kind: input.entity_kind,
            entity_subtype: input.entity_subtype,
            display_name: input.display_name,
            description: input.description,
            icon: input.icon,
            color: input.color,
            is_abstract: input.is_abstract.unwrap_or(false),
            parent_type_id: input.parent_type_id,
            created_at: now,
        };

        create_entity_type(registry.db(), &entity_type)?;
        Ok(entity_type)
    }

    #[tauri::command]
    pub fn blueprint_entity_type_list(version_id: String) -> Result<Vec<EntityTypeDef>, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        get_entity_types_by_version(registry.db(), &version_id)
    }

    #[tauri::command]
    pub fn blueprint_entity_type_delete(entity_type_id: String) -> Result<(), String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        delete_entity_type(registry.db(), &entity_type_id)
    }

    // =========================================================================
    // Field Commands
    // =========================================================================

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct CreateFieldInput {
        pub entity_type_id: String,
        pub field_name: String,
        pub display_label: String,
        pub data_type: String,
        pub is_required: Option<bool>,
        pub is_array: Option<bool>,
        pub default_value: Option<String>,
        pub validation_rules: Option<serde_json::Value>,
        pub ui_hints: Option<serde_json::Value>,
        pub display_order: Option<i32>,
        pub group_name: Option<String>,
        pub description: Option<String>,
    }

    #[tauri::command]
    pub fn blueprint_field_create(input: CreateFieldInput) -> Result<FieldDef, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();
        let id = format!("fld-{}", uuid::Uuid::new_v4());

        let field = FieldDef {
            field_id: id,
            entity_type_id: input.entity_type_id,
            field_name: input.field_name,
            display_label: input.display_label,
            data_type: input.data_type,
            is_required: input.is_required.unwrap_or(false),
            is_array: input.is_array.unwrap_or(false),
            default_value: input.default_value,
            validation_rules: input.validation_rules,
            ui_hints: input.ui_hints,
            display_order: input.display_order.unwrap_or(0),
            group_name: input.group_name,
            description: input.description,
            created_at: now,
        };

        create_field(registry.db(), &field)?;
        Ok(field)
    }

    #[tauri::command]
    pub fn blueprint_field_list(entity_type_id: String) -> Result<Vec<FieldDef>, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        get_fields_by_entity_type(registry.db(), &entity_type_id)
    }

    #[tauri::command]
    pub fn blueprint_field_delete(field_id: String) -> Result<(), String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        delete_field(registry.db(), &field_id)
    }

    // =========================================================================
    // RelationshipType Commands
    // =========================================================================

    #[taurpc::ipc_type]
    pub struct CreateRelationshipTypeInput {
        pub version_id: String,
        pub relationship_name: String,
        pub display_label: String,
        pub source_entity_kind: String,
        pub target_entity_kind: String,
        pub direction: Option<String>,
        pub cardinality: Option<String>,
        pub is_symmetric: Option<bool>,
        pub inverse_label: Option<String>,
        pub description: Option<String>,
        pub verb_patterns: Option<Vec<String>>,
        pub confidence: Option<f64>,
        pub pattern_category: Option<String>,
    }

    #[tauri::command]
    pub fn blueprint_relationship_type_create(input: CreateRelationshipTypeInput) -> Result<RelationshipTypeDef, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();
        let id = format!("rt-{}", uuid::Uuid::new_v4());

        let rel_type = RelationshipTypeDef {
            relationship_type_id: id,
            version_id: input.version_id,
            relationship_name: input.relationship_name,
            display_label: input.display_label,
            source_entity_kind: input.source_entity_kind,
            target_entity_kind: input.target_entity_kind,
            direction: input.direction.unwrap_or_else(|| "directed".to_string()),
            cardinality: input.cardinality.unwrap_or_else(|| "many_to_many".to_string()),
            is_symmetric: input.is_symmetric.unwrap_or(false),
            inverse_label: input.inverse_label,
            description: input.description,
            verb_patterns: input.verb_patterns,
            confidence: input.confidence,
            pattern_category: input.pattern_category,
            created_at: now,
        };

        create_relationship_type(registry.db(), &rel_type)?;
        Ok(rel_type)
    }

    #[tauri::command]
    pub fn blueprint_relationship_type_list(version_id: String) -> Result<Vec<RelationshipTypeDef>, String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        get_relationship_types_by_version(registry.db(), &version_id)
    }

    #[tauri::command]
    pub fn blueprint_relationship_type_delete(relationship_type_id: String) -> Result<(), String> {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        delete_relationship_type(registry.db(), &relationship_type_id)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> DbInstance {
        DbInstance::new("mem", "", Default::default()).unwrap()
    }

    #[test]
    fn test_init_schema() {
        let db = create_test_db();
        let result = init_blueprint_schema(&db);
        assert!(result.is_ok());
        
        // Running twice should not fail (idempotent)
        let result2 = init_blueprint_schema(&db);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_blueprint_crud() {
        let db = create_test_db();
        init_blueprint_schema(&db).unwrap();

        let meta = BlueprintMeta {
            blueprint_id: "test-bp-1".to_string(),
            name: "Test Blueprint".to_string(),
            description: Some("A test blueprint".to_string()),
            category: Some("narrative".to_string()),
            author: Some("Test Author".to_string()),
            tags: vec!["test".to_string(), "demo".to_string()],
            is_system: false,
            created_at: 1704067200,
            updated_at: 1704067200,
        };

        // Create
        create_blueprint_meta(&db, &meta).unwrap();

        // Read
        let fetched = get_blueprint_meta(&db, "test-bp-1").unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.name, "Test Blueprint");
        assert_eq!(fetched.tags.len(), 2);

        // List
        let all = list_blueprint_metas(&db).unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        delete_blueprint_meta(&db, "test-bp-1").unwrap();
        let after_delete = get_blueprint_meta(&db, "test-bp-1").unwrap();
        assert!(after_delete.is_none());
    }
}
