//! Blueprint API - Entity types, fields, and relationship definitions

use crate::blueprint::commands as cmd;

// ============================================================================
// API Trait
// ============================================================================

#[taurpc::procedures(path = "blueprint", export_to = "../src/bindings.ts")]
pub trait BlueprintApi {
    // Blueprint CRUD
    async fn init() -> Result<(), String>;
    async fn create(params: String) -> Result<String, String>;
    async fn get(blueprint_id: String) -> Result<Option<String>, String>;
    async fn list() -> Result<String, String>;
    async fn update(blueprint_id: String, params: String) -> Result<String, String>;
    async fn delete(blueprint_id: String) -> Result<(), String>;
    
    // Version management
    async fn version_create(params: String) -> Result<String, String>;
    async fn version_list(blueprint_id: String) -> Result<String, String>;
    async fn version_delete(version_id: String) -> Result<(), String>;
    
    // Entity Types
    async fn entity_type_create(params: String) -> Result<String, String>;
    async fn entity_type_list(version_id: String) -> Result<String, String>;
    async fn entity_type_delete(entity_type_id: String) -> Result<(), String>;
    
    // Fields
    async fn field_create(params: String) -> Result<String, String>;
    async fn field_list(entity_type_id: String) -> Result<String, String>;
    async fn field_delete(field_id: String) -> Result<(), String>;
    
    // Relationship Types
    async fn relationship_type_create(params: String) -> Result<String, String>;
    async fn relationship_type_list(version_id: String) -> Result<String, String>;
    async fn relationship_type_delete(relationship_type_id: String) -> Result<(), String>;
}

// ============================================================================
// Implementation
// ============================================================================

#[derive(Clone, Default)]
pub struct BlueprintApiImpl;

#[taurpc::resolvers]
impl BlueprintApi for BlueprintApiImpl {
    async fn init(self) -> Result<(), String> {
        cmd::blueprint_init()
    }
    
    async fn create(self, params: String) -> Result<String, String> {
        let input: cmd::CreateBlueprintInput = serde_json::from_str(&params)
            .map_err(|e| format!("Failed to parse params: {}", e))?;
        let result = cmd::blueprint_create(input)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get(self, blueprint_id: String) -> Result<Option<String>, String> {
        let result = cmd::blueprint_get(blueprint_id)?;
        match result {
            Some(bp) => Ok(Some(serde_json::to_string(&bp).map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }
    
    async fn list(self) -> Result<String, String> {
        let result = cmd::blueprint_list()?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn update(self, blueprint_id: String, params: String) -> Result<String, String> {
        let updates: cmd::CreateBlueprintInput = serde_json::from_str(&params)
            .map_err(|e| format!("Failed to parse params: {}", e))?;
        let result = cmd::blueprint_update(blueprint_id, updates)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete(self, blueprint_id: String) -> Result<(), String> {
        cmd::blueprint_delete(blueprint_id)
    }
    
    async fn version_create(self, params: String) -> Result<String, String> {
        let input: cmd::CreateVersionInput = serde_json::from_str(&params)
            .map_err(|e| format!("Failed to parse params: {}", e))?;
        let result = cmd::blueprint_version_create(input)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn version_list(self, blueprint_id: String) -> Result<String, String> {
        let result = cmd::blueprint_version_list(blueprint_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn version_delete(self, version_id: String) -> Result<(), String> {
        cmd::blueprint_version_delete(version_id)
    }
    
    async fn entity_type_create(self, params: String) -> Result<String, String> {
        let input: cmd::CreateEntityTypeInput = serde_json::from_str(&params)
            .map_err(|e| format!("Failed to parse params: {}", e))?;
        let result = cmd::blueprint_entity_type_create(input)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn entity_type_list(self, version_id: String) -> Result<String, String> {
        let result = cmd::blueprint_entity_type_list(version_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn entity_type_delete(self, entity_type_id: String) -> Result<(), String> {
        cmd::blueprint_entity_type_delete(entity_type_id)
    }
    
    async fn field_create(self, params: String) -> Result<String, String> {
        let input: cmd::CreateFieldInput = serde_json::from_str(&params)
            .map_err(|e| format!("Failed to parse params: {}", e))?;
        let result = cmd::blueprint_field_create(input)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn field_list(self, entity_type_id: String) -> Result<String, String> {
        let result = cmd::blueprint_field_list(entity_type_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn field_delete(self, field_id: String) -> Result<(), String> {
        cmd::blueprint_field_delete(field_id)
    }
    
    async fn relationship_type_create(self, params: String) -> Result<String, String> {
        let input: cmd::CreateRelationshipTypeInput = serde_json::from_str(&params)
            .map_err(|e| format!("Failed to parse params: {}", e))?;
        let result = cmd::blueprint_relationship_type_create(input)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn relationship_type_list(self, version_id: String) -> Result<String, String> {
        let result = cmd::blueprint_relationship_type_list(version_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn relationship_type_delete(self, relationship_type_id: String) -> Result<(), String> {
        cmd::blueprint_relationship_type_delete(relationship_type_id)
    }
}
