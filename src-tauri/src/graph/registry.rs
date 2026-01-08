//! GraphRegistry - Core CRUD operations
//!
//! Unified registry for nodes (entities) and edges (relationships).
//! Uses CozoDB with SQLite backend for persistence.

use cozo::{DbInstance, DataValue, NamedRows};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::schema;
use super::types::*;

// =============================================================================
// GraphRegistry
// =============================================================================

/// Unified graph registry for nodes and edges
pub struct GraphRegistry {
    pub(crate) db: DbInstance,
    initialized: bool,
}

impl GraphRegistry {
    /// Create a new GraphRegistry with SQLite backend
    pub fn new(path: &str) -> Result<Self, GraphError> {
        let db = DbInstance::new("sqlite", path, Default::default())
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        
        let mut registry = Self { db, initialized: false };
        registry.init()?;
        Ok(registry)
    }

    /// Create an in-memory GraphRegistry (for testing)
    pub fn in_memory() -> Result<Self, GraphError> {
        let db = DbInstance::new("mem", "", Default::default())
            .map_err(|e| GraphError::StorageError(e.to_string()))?;
        
        let mut registry = Self { db, initialized: false };
        registry.init()?;
        Ok(registry)
    }

    /// Initialize schema
    fn init(&mut self) -> Result<(), GraphError> {
        schema::init_schema(&self.db)
            .map_err(|e| GraphError::StorageError(e))?;
        self.initialized = true;
        Ok(())
    }

    /// Get a reference to the CozoDB instance
    pub fn db(&self) -> &DbInstance {
        &self.db
    }

    /// Get current timestamp
    fn now() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }

    /// Generate a unique ID
    fn generate_id() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        Self::now().to_bits().hash(&mut hasher);
        std::process::id().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Normalize a label for matching
    fn normalize(label: &str) -> String {
        label.trim().to_lowercase()
    }

    // =========================================================================
    // Node CRUD
    // =========================================================================

    /// Register a node (create or update)
    pub fn register_node(&self, input: NodeInput) -> Result<NodeRegistrationResult, GraphError> {
        let normalized = Self::normalize(&input.label);
        
        // Check if exists
        if let Some(existing) = self.find_node(&input.label)? {
            // Update mention count
            let query = r#"
                ?[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata] :=
                    *nodes[id, label, normalized, kind, subtype, source_note, created_at, created_by, old_count, metadata],
                    id = $id,
                    mention_count = old_count + 1
                :put nodes {id => label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata}
            "#;
            
            let mut params = BTreeMap::new();
            params.insert("id".to_string(), DataValue::Str(existing.id.clone().into()));
            
            self.db.run_script(query, params, cozo::ScriptMutability::Mutable)
                .map_err(|e| GraphError::QueryError(e.to_string()))?;
            
            let updated = self.get_node(&existing.id)?.unwrap();
            return Ok(NodeRegistrationResult {
                node: updated,
                is_new: false,
                was_merged: false,
            });
        }

        // Create new node
        let id = Self::generate_id();
        let now = Self::now();
        
        let query = r#"
            ?[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata] <- [[
                $id, $label, $normalized, $kind, $subtype, $source_note, $created_at, $created_by, 1, $metadata
            ]]
            :put nodes {id => label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata}
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("label".to_string(), DataValue::Str(input.label.clone().into()));
        params.insert("normalized".to_string(), DataValue::Str(normalized.into()));
        params.insert("kind".to_string(), DataValue::Str(input.kind.as_str().into()));
        params.insert("subtype".to_string(), DataValue::Str(input.subtype.unwrap_or_default().into()));
        params.insert("source_note".to_string(), DataValue::Str(input.source_note.into()));
        params.insert("created_at".to_string(), DataValue::from(now));
        params.insert("created_by".to_string(), DataValue::Str(input.created_by.as_str().into()));
        params.insert("metadata".to_string(), DataValue::Str(
            input.metadata.map(|v| v.to_string()).unwrap_or_default().into()
        ));
        
        self.db.run_script(query, params, cozo::ScriptMutability::Mutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        // Add aliases
        for alias in input.aliases {
            self.add_alias(&id, &alias)?;
        }
        
        let node = self.get_node(&id)?.unwrap();
        Ok(NodeRegistrationResult {
            node,
            is_new: true,
            was_merged: false,
        })
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Result<Option<Node>, GraphError> {
        let query = r#"
            ?[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata] :=
                *nodes[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata],
                id = $id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        let result = self.db.run_script(query, params, cozo::ScriptMutability::Immutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        if result.rows.is_empty() {
            return Ok(None);
        }
        
        let row = &result.rows[0];
        let aliases = self.get_aliases(id)?;
        
        Ok(Some(self.row_to_node(row, aliases)?))
    }

    /// Find a node by label (case-insensitive)
    pub fn find_node(&self, label: &str) -> Result<Option<Node>, GraphError> {
        let normalized = Self::normalize(label);
        
        let query = r#"
            ?[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata] :=
                *nodes[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata],
                normalized = $normalized
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("normalized".to_string(), DataValue::Str(normalized.into()));
        
        let result = self.db.run_script(query, params, cozo::ScriptMutability::Immutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        if result.rows.is_empty() {
            // Try aliases
            return self.find_node_by_alias(label);
        }
        
        let row = &result.rows[0];
        let id = self.extract_string(&row[0])?;
        let aliases = self.get_aliases(&id)?;
        
        Ok(Some(self.row_to_node(row, aliases)?))
    }

    /// Find node by alias
    fn find_node_by_alias(&self, alias: &str) -> Result<Option<Node>, GraphError> {
        let normalized = Self::normalize(alias);
        
        let query = r#"
            ?[node_id] :=
                *node_aliases[node_id, _, normalized],
                normalized = $normalized
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("normalized".to_string(), DataValue::Str(normalized.into()));
        
        let result = self.db.run_script(query, params, cozo::ScriptMutability::Immutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        if result.rows.is_empty() {
            return Ok(None);
        }
        
        let node_id = self.extract_string(&result.rows[0][0])?;
        self.get_node(&node_id)
    }

    /// Get all nodes with optional filters
    pub fn get_nodes(&self, filter: NodeFilter) -> Result<Vec<Node>, GraphError> {
        let mut conditions = vec!["*nodes[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata]".to_string()];
        let mut params = BTreeMap::new();
        
        if let Some(kind) = filter.kind {
            conditions.push("kind = $kind".to_string());
            params.insert("kind".to_string(), DataValue::Str(kind.as_str().into()));
        }
        
        if let Some(subtype) = filter.subtype {
            conditions.push("subtype = $subtype".to_string());
            params.insert("subtype".to_string(), DataValue::Str(subtype.into()));
        }
        
        if let Some(min) = filter.min_mentions {
            conditions.push("mention_count >= $min_mentions".to_string());
            params.insert("min_mentions".to_string(), DataValue::from(min));
        }
        
        let query = format!(
            "?[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata] := {}",
            conditions.join(", ")
        );
        
        let result = self.db.run_script(&query, params, cozo::ScriptMutability::Immutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        let mut nodes = Vec::new();
        for row in &result.rows {
            let id = self.extract_string(&row[0])?;
            let aliases = self.get_aliases(&id)?;
            nodes.push(self.row_to_node(row, aliases)?);
        }
        
        Ok(nodes)
    }

    /// Delete a node (cascades to edges and aliases)
    pub fn delete_node(&self, id: &str) -> Result<bool, GraphError> {
        // Check if exists
        if self.get_node(id)?.is_none() {
            return Ok(false);
        }
        
        // Delete aliases
        let alias_query = r#"
            ?[node_id, alias] := *node_aliases[node_id, alias, _], node_id = $id
            :rm node_aliases {node_id, alias}
        "#;
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        let _ = self.db.run_script(alias_query, params.clone(), cozo::ScriptMutability::Mutable);
        
        // Delete edges where node is source
        let edge_query_src = r#"
            ?[id] := *edges{id, source_id}, source_id = $node_id
            :rm edges {id}
        "#;
        params.insert("node_id".to_string(), DataValue::Str(id.into()));
        let _ = self.db.run_script(edge_query_src, params.clone(), cozo::ScriptMutability::Mutable);
        
        // Delete edges where node is target
        let edge_query_tgt = r#"
            ?[id] := *edges{id, target_id}, target_id = $node_id
            :rm edges {id}
        "#;
        let _ = self.db.run_script(edge_query_tgt, params.clone(), cozo::ScriptMutability::Mutable);
        
        // Delete node
        let node_query = r#"
            ?[id] := id = $id
            :rm nodes {id}
        "#;
        self.db.run_script(node_query, params, cozo::ScriptMutability::Mutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        Ok(true)
    }

    /// Clear all nodes and edges from the graph
    /// Returns the number of nodes deleted
    pub fn clear_all(&self) -> Result<usize, GraphError> {
        // Count nodes first
        let count_query = "?[count(id)] := *nodes{id}";
        let count_result = self.db.run_script(count_query, Default::default(), cozo::ScriptMutability::Immutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        let count = count_result.rows.first()
            .and_then(|r| r.first())
            .map(|v| match v {
                DataValue::Num(n) => match n {
                    cozo::Num::Int(i) => *i as usize,
                    cozo::Num::Float(f) => *f as usize,
                },
                _ => 0,
            })
            .unwrap_or(0);
        
        // Delete all aliases
        let alias_query = r#"
            ?[node_id, alias] := *node_aliases{node_id, alias}
            :rm node_aliases {node_id, alias}
        "#;
        let _ = self.db.run_script(alias_query, Default::default(), cozo::ScriptMutability::Mutable);
        
        // Delete all edges
        let edge_query = r#"
            ?[id] := *edges{id}
            :rm edges {id}
        "#;
        let _ = self.db.run_script(edge_query, Default::default(), cozo::ScriptMutability::Mutable);
        
        // Delete all nodes
        let node_query = r#"
            ?[id] := *nodes{id}
            :rm nodes {id}
        "#;
        self.db.run_script(node_query, Default::default(), cozo::ScriptMutability::Mutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        log::info!("[GraphRegistry] Cleared {} nodes from graph", count);
        Ok(count)
    }

    // =========================================================================
    // Alias Operations
    // =========================================================================

    /// Add an alias to a node
    pub fn add_alias(&self, node_id: &str, alias: &str) -> Result<bool, GraphError> {
        let normalized = Self::normalize(alias);
        
        let query = r#"
            ?[node_id, alias, normalized] <- [[$node_id, $alias, $normalized]]
            :put node_aliases {node_id, alias => normalized}
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("node_id".to_string(), DataValue::Str(node_id.into()));
        params.insert("alias".to_string(), DataValue::Str(alias.into()));
        params.insert("normalized".to_string(), DataValue::Str(normalized.into()));
        
        self.db.run_script(query, params, cozo::ScriptMutability::Mutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        Ok(true)
    }

    /// Get aliases for a node
    pub fn get_aliases(&self, node_id: &str) -> Result<Vec<String>, GraphError> {
        let query = r#"
            ?[alias] := *node_aliases[node_id, alias, _], node_id = $node_id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("node_id".to_string(), DataValue::Str(node_id.into()));
        
        let result = self.db.run_script(query, params, cozo::ScriptMutability::Immutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        let mut aliases = Vec::new();
        for row in &result.rows {
            aliases.push(self.extract_string(&row[0])?);
        }
        
        Ok(aliases)
    }

    // =========================================================================
    // Edge CRUD
    // =========================================================================

    /// Create an edge between two nodes
    pub fn create_edge(&self, input: EdgeInput) -> Result<Edge, GraphError> {
        // Validate source and target exist
        if self.get_node(&input.source_id)?.is_none() {
            return Err(GraphError::NodeNotFound(input.source_id));
        }
        if self.get_node(&input.target_id)?.is_none() {
            return Err(GraphError::NodeNotFound(input.target_id));
        }
        
        let id = Self::generate_id();
        let now = Self::now();
        
        let query = r#"
            ?[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] <- [[
                $id, $source_id, $target_id, $edge_type, $inverse_type, $bidirectional, $weight, $confidence, $created_at, $created_by, $source_note, $metadata
            ]]
            :put edges {id => source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata}
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.clone().into()));
        params.insert("source_id".to_string(), DataValue::Str(input.source_id.into()));
        params.insert("target_id".to_string(), DataValue::Str(input.target_id.into()));
        params.insert("edge_type".to_string(), DataValue::Str(input.edge_type.into()));
        params.insert("inverse_type".to_string(), DataValue::Str(input.inverse_type.unwrap_or_default().into()));
        params.insert("bidirectional".to_string(), DataValue::Bool(input.bidirectional));
        params.insert("weight".to_string(), DataValue::from(input.weight));
        params.insert("confidence".to_string(), DataValue::from(input.confidence));
        params.insert("created_at".to_string(), DataValue::from(now));
        params.insert("created_by".to_string(), DataValue::Str(input.created_by.as_str().into()));
        params.insert("source_note".to_string(), DataValue::Str(input.source_note.unwrap_or_default().into()));
        params.insert("metadata".to_string(), DataValue::Str(
            input.metadata.map(|v| v.to_string()).unwrap_or_default().into()
        ));
        
        self.db.run_script(query, params, cozo::ScriptMutability::Mutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        self.get_edge(&id)?.ok_or_else(|| GraphError::EdgeNotFound(id))
    }

    /// Get an edge by ID
    pub fn get_edge(&self, id: &str) -> Result<Option<Edge>, GraphError> {
        let query = r#"
            ?[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] :=
                *edges[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata],
                id = $id
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        let result = self.db.run_script(query, params, cozo::ScriptMutability::Immutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        if result.rows.is_empty() {
            return Ok(None);
        }
        
        Ok(Some(self.row_to_edge(&result.rows[0])?))
    }

    /// Get edges connected to a node
    pub fn get_edges(&self, node_id: &str, direction: Direction) -> Result<Vec<Edge>, GraphError> {
        let query = match direction {
            Direction::Outgoing => r#"
                ?[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] :=
                    *edges[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata],
                    source_id = $node_id
            "#,
            Direction::Incoming => r#"
                ?[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] :=
                    *edges[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata],
                    target_id = $node_id
            "#,
            Direction::Both => r#"
                out[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] :=
                    *edges[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata],
                    source_id = $node_id
                out[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] :=
                    *edges[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata],
                    target_id = $node_id
                ?[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] := out[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata]
            "#,
        };
        
        let mut params = BTreeMap::new();
        params.insert("node_id".to_string(), DataValue::Str(node_id.into()));
        
        let result = self.db.run_script(query, params, cozo::ScriptMutability::Immutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        let mut edges = Vec::new();
        for row in &result.rows {
            edges.push(self.row_to_edge(row)?);
        }
        
        Ok(edges)
    }

    /// Delete an edge
    pub fn delete_edge(&self, id: &str) -> Result<bool, GraphError> {
        if self.get_edge(id)?.is_none() {
            return Ok(false);
        }
        
        let query = r#"
            ?[id] := id = $id
            :rm edges {id}
        "#;
        
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), DataValue::Str(id.into()));
        
        self.db.run_script(query, params, cozo::ScriptMutability::Mutable)
            .map_err(|e| GraphError::QueryError(e.to_string()))?;
        
        Ok(true)
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    pub(crate) fn extract_string(&self, val: &DataValue) -> Result<String, GraphError> {
        match val {
            DataValue::Str(s) => Ok(s.to_string()),
            _ => Err(GraphError::QueryError("Expected string".into())),
        }
    }

    fn extract_f64(&self, val: &DataValue) -> Result<f64, GraphError> {
        match val {
            DataValue::Num(n) => {
                // Num can be Int or Float - convert to f64
                match n {
                    cozo::Num::Int(i) => Ok(*i as f64),
                    cozo::Num::Float(f) => Ok(*f),
                }
            }
            _ => Err(GraphError::QueryError("Expected number".into())),
        }
    }

    fn extract_i64(&self, val: &DataValue) -> Result<i64, GraphError> {
        match val {
            DataValue::Num(n) => {
                match n {
                    cozo::Num::Int(i) => Ok(*i),
                    cozo::Num::Float(f) => Ok(*f as i64),
                }
            }
            _ => Err(GraphError::QueryError("Expected integer".into())),
        }
    }

    fn extract_bool(&self, val: &DataValue) -> Result<bool, GraphError> {
        match val {
            DataValue::Bool(b) => Ok(*b),
            _ => Err(GraphError::QueryError("Expected bool".into())),
        }
    }

    fn row_to_node(&self, row: &[DataValue], aliases: Vec<String>) -> Result<Node, GraphError> {
        Ok(Node {
            id: self.extract_string(&row[0])?,
            label: self.extract_string(&row[1])?,
            normalized: self.extract_string(&row[2])?,
            kind: NodeKind::from_str(&self.extract_string(&row[3])?),
            subtype: {
                let s = self.extract_string(&row[4])?;
                if s.is_empty() { None } else { Some(s) }
            },
            source_note: self.extract_string(&row[5])?,
            created_at: self.extract_f64(&row[6])?,
            created_by: CreatedBy::from_str(&self.extract_string(&row[7])?),
            mention_count: self.extract_i64(&row[8])?,
            aliases,
            metadata: {
                let s = self.extract_string(&row[9])?;
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            },
        })
    }

    pub(crate) fn row_to_edge(&self, row: &[DataValue]) -> Result<Edge, GraphError> {
        Ok(Edge {
            id: self.extract_string(&row[0])?,
            source_id: self.extract_string(&row[1])?,
            target_id: self.extract_string(&row[2])?,
            edge_type: self.extract_string(&row[3])?,
            inverse_type: {
                let s = self.extract_string(&row[4])?;
                if s.is_empty() { None } else { Some(s) }
            },
            bidirectional: self.extract_bool(&row[5])?,
            weight: self.extract_f64(&row[6])?,
            confidence: self.extract_f64(&row[7])?,
            created_at: self.extract_f64(&row[8])?,
            created_by: CreatedBy::from_str(&self.extract_string(&row[9])?),
            source_note: {
                let s = self.extract_string(&row[10])?;
                if s.is_empty() { None } else { Some(s) }
            },
            metadata: {
                let s = self.extract_string(&row[11])?;
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            },
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> GraphRegistry {
        GraphRegistry::in_memory().unwrap()
    }

    fn test_input(label: &str, kind: NodeKind) -> NodeInput {
        NodeInput {
            label: label.to_string(),
            kind,
            source_note: "test_note".to_string(),
            subtype: None,
            aliases: vec![],
            created_by: CreatedBy::User,
            metadata: None,
        }
    }

    // Node CRUD tests
    #[test]
    fn test_register_node_creates_new() {
        let registry = test_registry();
        let result = registry.register_node(test_input("Gandalf", NodeKind::Character)).unwrap();
        
        assert!(result.is_new);
        assert_eq!(result.node.label, "Gandalf");
        assert_eq!(result.node.kind, NodeKind::Character);
    }

    #[test]
    fn test_register_node_returns_existing() {
        let registry = test_registry();
        let r1 = registry.register_node(test_input("Gandalf", NodeKind::Character)).unwrap();
        let r2 = registry.register_node(test_input("Gandalf", NodeKind::Character)).unwrap();
        
        assert!(r1.is_new);
        assert!(!r2.is_new);
        assert_eq!(r1.node.id, r2.node.id);
        assert_eq!(r2.node.mention_count, 2);
    }

    #[test]
    fn test_get_node_by_id() {
        let registry = test_registry();
        let result = registry.register_node(test_input("Frodo", NodeKind::Character)).unwrap();
        
        let node = registry.get_node(&result.node.id).unwrap().unwrap();
        assert_eq!(node.label, "Frodo");
    }

    #[test]
    fn test_find_node_by_label() {
        let registry = test_registry();
        registry.register_node(test_input("Mordor", NodeKind::Location)).unwrap();
        
        let node = registry.find_node("mordor").unwrap().unwrap();  // case insensitive
        assert_eq!(node.label, "Mordor");
    }

    #[test]
    fn test_register_node_with_aliases() {
        let registry = test_registry();
        let input = NodeInput {
            label: "Aragorn".to_string(),
            kind: NodeKind::Character,
            source_note: "test".to_string(),
            subtype: None,
            aliases: vec!["Strider".to_string(), "Elessar".to_string()],
            created_by: CreatedBy::User,
            metadata: None,
        };
        
        let result = registry.register_node(input).unwrap();
        assert_eq!(result.node.aliases.len(), 2);
        
        // Should find by alias
        let found = registry.find_node("strider").unwrap().unwrap();
        assert_eq!(found.id, result.node.id);
    }

    #[test]
    fn test_get_nodes_by_kind() {
        let registry = test_registry();
        registry.register_node(test_input("Gandalf", NodeKind::Character)).unwrap();
        registry.register_node(test_input("Saruman", NodeKind::Character)).unwrap();
        registry.register_node(test_input("Mordor", NodeKind::Location)).unwrap();
        
        let characters = registry.get_nodes(NodeFilter {
            kind: Some(NodeKind::Character),
            ..Default::default()
        }).unwrap();
        
        assert_eq!(characters.len(), 2);
    }

    #[test]
    fn test_delete_node_cascades_edges() {
        let registry = test_registry();
        let n1 = registry.register_node(test_input("Frodo", NodeKind::Character)).unwrap();
        let n2 = registry.register_node(test_input("Ring", NodeKind::Item)).unwrap();
        
        registry.create_edge(EdgeInput {
            source_id: n1.node.id.clone(),
            target_id: n2.node.id.clone(),
            edge_type: "OWNS".to_string(),
            inverse_type: Some("OWNED_BY".to_string()),
            bidirectional: false,
            weight: 1.0,
            confidence: 1.0,
            created_by: CreatedBy::User,
            source_note: None,
            metadata: None,
        }).unwrap();
        
        // Delete Frodo
        registry.delete_node(&n1.node.id).unwrap();
        
        // Edge should be gone
        let edges = registry.get_edges(&n2.node.id, Direction::Both).unwrap();
        assert!(edges.is_empty());
    }

    // Edge CRUD tests
    #[test]
    fn test_create_edge() {
        let registry = test_registry();
        let n1 = registry.register_node(test_input("Gandalf", NodeKind::Character)).unwrap();
        let n2 = registry.register_node(test_input("Saruman", NodeKind::Character)).unwrap();
        
        let edge = registry.create_edge(EdgeInput {
            source_id: n1.node.id,
            target_id: n2.node.id,
            edge_type: "KNOWS".to_string(),
            inverse_type: None,
            bidirectional: true,
            weight: 1.0,
            confidence: 0.9,
            created_by: CreatedBy::User,
            source_note: None,
            metadata: None,
        }).unwrap();
        
        assert_eq!(edge.edge_type, "KNOWS");
        assert!(edge.bidirectional);
    }

    #[test]
    fn test_get_edges_from_node() {
        let registry = test_registry();
        let n1 = registry.register_node(test_input("Frodo", NodeKind::Character)).unwrap();
        let n2 = registry.register_node(test_input("Sam", NodeKind::Character)).unwrap();
        let n3 = registry.register_node(test_input("Ring", NodeKind::Item)).unwrap();
        
        registry.create_edge(EdgeInput {
            source_id: n1.node.id.clone(),
            target_id: n2.node.id.clone(),
            edge_type: "FRIEND_OF".to_string(),
            inverse_type: None,
            bidirectional: true,
            weight: 1.0,
            confidence: 1.0,
            created_by: CreatedBy::User,
            source_note: None,
            metadata: None,
        }).unwrap();
        
        registry.create_edge(EdgeInput {
            source_id: n1.node.id.clone(),
            target_id: n3.node.id.clone(),
            edge_type: "OWNS".to_string(),
            inverse_type: None,
            bidirectional: false,
            weight: 1.0,
            confidence: 1.0,
            created_by: CreatedBy::User,
            source_note: None,
            metadata: None,
        }).unwrap();
        
        let outgoing = registry.get_edges(&n1.node.id, Direction::Outgoing).unwrap();
        assert_eq!(outgoing.len(), 2);
    }

    #[test]
    fn test_delete_edge() {
        let registry = test_registry();
        let n1 = registry.register_node(test_input("A", NodeKind::Character)).unwrap();
        let n2 = registry.register_node(test_input("B", NodeKind::Character)).unwrap();
        
        let edge = registry.create_edge(EdgeInput {
            source_id: n1.node.id,
            target_id: n2.node.id,
            edge_type: "TEST".to_string(),
            inverse_type: None,
            bidirectional: false,
            weight: 1.0,
            confidence: 1.0,
            created_by: CreatedBy::User,
            source_note: None,
            metadata: None,
        }).unwrap();
        
        assert!(registry.delete_edge(&edge.id).unwrap());
        assert!(registry.get_edge(&edge.id).unwrap().is_none());
    }
}
