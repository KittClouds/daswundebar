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
