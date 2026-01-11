//! Graph Schema - CozoDB Relation Definitions
//!
//! Creates the stored relations for nodes, edges, aliases, and vectors.

use cozo::DbInstance;

// =============================================================================
// Schema Queries
// =============================================================================

/// Create the nodes relation (entities)
pub const CREATE_NODES: &str = r#"
:create nodes {
    id: String =>
    label: String,
    normalized: String,
    kind: String,
    subtype: String default '',
    source_note: String,
    created_at: Float,
    created_by: String,
    mention_count: Int default 0,
    metadata: String default ''
}
"#;

/// Create the node_aliases relation
pub const CREATE_ALIASES: &str = r#"
:create node_aliases {
    node_id: String,
    alias: String
=>
    normalized: String
}
"#;

/// Create the edges relation (relationships)
pub const CREATE_EDGES: &str = r#"
:create edges {
    id: String =>
    source_id: String,
    target_id: String,
    edge_type: String,
    inverse_type: String default '',
    bidirectional: Bool default false,
    weight: Float default 1.0,
    confidence: Float default 1.0,
    created_at: Float,
    created_by: String,
    source_note: String default '',
    metadata: String default ''
}
"#;

/// Create edge type definitions (schema)
pub const CREATE_EDGE_TYPES: &str = r#"
:create edge_types {
    type_name: String =>
    inverse_name: String default '',
    source_kinds: String,
    target_kinds: String,
    bidirectional: Bool default false,
    color: String default '',
    description: String default ''
}
"#;

/// Create node vectors relation for HNSW
pub const CREATE_VECTORS: &str = r#"
:create node_vectors {
    node_id: String =>
    model: String,
    vector: <F32; 384>,
    created_at: Float
}
"#;

/// Create HNSW index on vectors
pub const CREATE_HNSW_INDEX: &str = r#"
::hnsw create node_vectors:semantic_idx {
    dim: 384,
    m: 32,
    dtype: F32,
    fields: [vector],
    distance: Cosine,
    ef_construction: 200
}
"#;

// =============================================================================
// Content Schema (SurrealDB Replacement)
// =============================================================================

/// Notes relation
pub const CREATE_NOTES: &str = r#"
:create notes {
    id: String =>
    world_id: String,
    title: String,
    content: String,
    folder_id: String default '',
    entity_kind: String default '',
    entity_subtype: String default '',
    is_entity: Bool default false,
    is_pinned: Bool default false,
    favorite: Bool default false,
    created_at: Float,
    updated_at: Float
}
"#;

/// Folders relation
pub const CREATE_FOLDERS: &str = r#"
:create folders {
    id: String =>
    world_id: String,
    name: String,
    parent_id: String default '',
    entity_kind: String default '',
    entity_subtype: String default '',
    color: String default '',
    is_typed_root: Bool default false,
    network_id: String default '',
    collapsed: Bool default false,
    fantasy_year: Int default 0,
    fantasy_month: Int default 0,
    fantasy_day: Int default 0,
    created_at: Float,
    updated_at: Float
}
"#;

/// Networks relation
pub const CREATE_NETWORKS: &str = r#"
:create networks {
    id: String =>
    world_id: String,
    name: String,
    schema_id: String,
    root_folder_id: String default '',
    root_entity_id: String default '',
    namespace: String default '',
    description: String default '',
    tags: String default '[]',
    member_count: Int default 0,
    relationship_count: Int default 0,
    max_depth: Int default 0,
    created_at: Float,
    updated_at: Float
}
"#;

/// Domain entities relation (separate from knowledge graph nodes)
pub const CREATE_ENTITIES: &str = r#"
:create entities {
    id: String =>
    world_id: String,
    label: String,
    entity_kind: String,
    entity_subtype: String default '',
    note_id: String default '',
    folder_id: String default '',
    is_active: Bool default true,
    aliases: String default '[]',
    attributes: String default '{}',
    created_at: Float,
    updated_at: Float
}
"#;

/// Network membership edge
pub const CREATE_IN_NETWORK: &str = r#"
:create in_network {
    id: String =>
    world_id: String,
    network_id: String,
    entity_id: String,
    role: String default 'MEMBER',
    depth_level: Int default 0,
    group_id: String default '',
    joined_at: Float
}
"#;

/// Relationship edge between entities
pub const CREATE_RELATES_TO: &str = r#"
:create relates_to {
    id: String =>
    world_id: String,
    source_id: String,
    target_id: String,
    network_id: String default '',
    relationship_code: String,
    inverse_id: String default '',
    strength: Float default 1.0,
    start_date: Float default 0,
    end_date: Float default 0,
    notes: String default '',
    attributes: String default '{}',
    created_at: Float
}
"#;

/// Calendar events relation
pub const CREATE_CAL_EVENTS: &str = r#"
:create cal_events {
    id: String =>
    world_id: String,
    calendar_id: String,
    title: String,
    description: String default '',
    date_year: Int,
    date_month: Int,
    date_day: Int,
    date_hour: Int default -1,
    date_minute: Int default -1,
    era_id: String default '',
    end_year: Int default 0,
    end_month: Int default 0,
    end_day: Int default 0,
    is_all_day: Bool default true,
    recurrence: String default '{}',
    parent_event_id: String default '',
    importance: String default 'minor',
    category: String default 'event',
    tags: String default '[]',
    color: String default '',
    icon: String default '',
    entity_id: String default '',
    entity_kind: String default '',
    source_note_id: String default '',
    created_at: Float,
    updated_at: Float
}
"#;

/// Periods (eras, ages, epochs)
pub const CREATE_PERIODS: &str = r#"
:create periods {
    id: String =>
    world_id: String,
    calendar_id: String,
    name: String,
    description: String default '',
    start_year: Int,
    start_month: Int default 1,
    end_year: Int default 0,
    end_month: Int default 0,
    parent_period_id: String default '',
    period_type: String default 'era',
    color: String,
    icon: String default '',
    abbreviation: String default '',
    direction: String default 'ascending',
    triggered_by: String default '',
    ends_when: String default '',
    major_events: String default '[]',
    arc_type: String default '',
    dominant_theme: String default '',
    protagonist_id: String default '',
    antagonist_id: String default '',
    summary: String default '',
    detailed_notes: String default '',
    show_on_timeline: Bool default true,
    timeline_color: String default '',
    timeline_icon: String default '',
    created_at: Float,
    updated_at: Float
}
"#;

/// Entity-event participation link
pub const CREATE_OCCURS_ON: &str = r#"
:create occurs_on {
    id: String =>
    world_id: String,
    entity_id: String,
    event_id: String,
    role: String default 'participant',
    significance: String default 'minor',
    notes: String default '',
    created_at: Float
}
"#;

/// Field bindings for inheritance/aggregation
pub const CREATE_FIELD_BINDINGS: &str = r#"
:create field_bindings {
    id: String =>
    world_id: String,
    source_entity_id: String,
    source_field_name: String,
    target_entity_id: String,
    target_field_name: String,
    binding_type: String default 'inherit',
    transform: String default '{}',
    aggregation_fn: String default '',
    aggregation_filter: String default '{}',
    allow_override: Bool default true,
    is_active: Bool default true,
    created_at: Float,
    updated_at: Float
}
"#;

/// Decoration span cache for highlighting
pub const CREATE_DECORATION_SPANS: &str = r#"
:create decoration_spans {
    note_id: String =>
    content_hash: String,
    spans_json: String,
    created_at: Float
}
"#;

/// Inferred entities from NER (GLiNER)
/// Status: 'pending' | 'accepted' | 'rejected'
pub const CREATE_INFERRED_ENTITIES: &str = r#"
:create inferred_entities {
    id: String =>
    world_id: String,
    source_note_id: String,
    text: String,
    label: String,
    entity_type: String,
    byte_start: Int,
    byte_end: Int,
    confidence: Float,
    status: String default 'pending',
    promoted_entity_id: String default '',
    inferred_at: Float,
    reviewed_at: Float default 0
}
"#;

// =============================================================================
// Schema Management
// =============================================================================

/// Check if a relation exists
pub fn relation_exists(db: &DbInstance, name: &str) -> bool {
    let query = "::relations".to_string();
    match db.run_script(&query, Default::default(), cozo::ScriptMutability::Immutable) {
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
}

/// Initialize the graph schema (idempotent)
pub fn init_schema(db: &DbInstance) -> Result<(), String> {
    let relations = [
        ("nodes", CREATE_NODES),
        ("node_aliases", CREATE_ALIASES),
        ("edges", CREATE_EDGES),
        ("edge_types", CREATE_EDGE_TYPES),
        ("node_vectors", CREATE_VECTORS),
    ];

    for (name, query) in relations {
        if !relation_exists(db, name) {
            db.run_script(query, Default::default(), cozo::ScriptMutability::Mutable)
                .map_err(|e| format!("Failed to create {}: {}", name, e))?;
            log::info!("[GraphSchema] Created relation: {}", name);
        }
    }

    // Create HNSW index if vectors relation was just created
    if relation_exists(db, "node_vectors") {
        // Check if index exists by trying to use it
        let index_check = r#"?[exists] := exists = false"#;
        let _ = db.run_script(index_check, Default::default(), cozo::ScriptMutability::Immutable);
        
        // Try to create index (will fail silently if exists)
        let _ = db.run_script(CREATE_HNSW_INDEX, Default::default(), cozo::ScriptMutability::Mutable);
    }

    // Also initialize content schema
    init_content_schema(db)?;

    Ok(())
}

/// Initialize content schema (notes, folders, networks, etc.) - SurrealDB replacement
pub fn init_content_schema(db: &DbInstance) -> Result<(), String> {
    let content_relations = [
        ("notes", CREATE_NOTES),
        ("folders", CREATE_FOLDERS),
        ("networks", CREATE_NETWORKS),
        ("entities", CREATE_ENTITIES),
        ("in_network", CREATE_IN_NETWORK),
        ("relates_to", CREATE_RELATES_TO),
        ("cal_events", CREATE_CAL_EVENTS),
        ("periods", CREATE_PERIODS),
        ("occurs_on", CREATE_OCCURS_ON),
        ("field_bindings", CREATE_FIELD_BINDINGS),
        ("decoration_spans", CREATE_DECORATION_SPANS),
        ("inferred_entities", CREATE_INFERRED_ENTITIES),
    ];

    for (name, query) in content_relations {
        if !relation_exists(db, name) {
            db.run_script(query, Default::default(), cozo::ScriptMutability::Mutable)
                .map_err(|e| format!("Failed to create content relation {}: {}", name, e))?;
            log::info!("[ContentSchema] Created relation: {}", name);
        }
    }

    Ok(())
}

/// Drop all graph relations (for testing)
#[cfg(test)]
pub fn drop_schema(db: &DbInstance) -> Result<(), String> {
    // Drop HNSW index first (if it exists)
    let _ = db.run_script(
        "::hnsw drop node_vectors:semantic_idx",
        Default::default(),
        cozo::ScriptMutability::Mutable
    );
    
    let relations = ["node_vectors", "edge_types", "edges", "node_aliases", "nodes"];
    
    for name in relations {
        if relation_exists(db, name) {
            let query = format!("::remove {}", name);
            db.run_script(&query, Default::default(), cozo::ScriptMutability::Mutable)
                .map_err(|e| format!("Failed to drop {}: {}", name, e))?;
        }
    }
    
    Ok(())
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
    fn test_schema_creates_nodes_relation() {
        let db = create_test_db();
        init_schema(&db).unwrap();
        assert!(relation_exists(&db, "nodes"));
    }

    #[test]
    fn test_schema_creates_edges_relation() {
        let db = create_test_db();
        init_schema(&db).unwrap();
        assert!(relation_exists(&db, "edges"));
    }

    #[test]
    fn test_schema_creates_aliases_relation() {
        let db = create_test_db();
        init_schema(&db).unwrap();
        assert!(relation_exists(&db, "node_aliases"));
    }

    #[test]
    fn test_schema_creates_edge_types_relation() {
        let db = create_test_db();
        init_schema(&db).unwrap();
        assert!(relation_exists(&db, "edge_types"));
    }

    #[test]
    fn test_schema_creates_vectors_relation() {
        let db = create_test_db();
        init_schema(&db).unwrap();
        assert!(relation_exists(&db, "node_vectors"));
    }

    #[test]
    fn test_schema_idempotent() {
        let db = create_test_db();
        init_schema(&db).unwrap();
        // Second call should not error
        init_schema(&db).unwrap();
        assert!(relation_exists(&db, "nodes"));
    }

    #[test]
    fn test_drop_schema() {
        let db = create_test_db();
        init_schema(&db).unwrap();
        drop_schema(&db).unwrap();
        assert!(!relation_exists(&db, "nodes"));
    }
}
