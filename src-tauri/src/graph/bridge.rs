//! Scanner Bridge - Rust-to-Rust entity/result coordination
//!
//! Breaks the circular hydration loop by:
//! 1. Keeping entity data in Rust (GraphRegistry) 
//! 2. Detecting when entity set actually changes (hash-based)
//! 3. Only re-hydrating scanner when necessary
//! 4. Ingesting scan results directly into GraphRegistry

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::registry::GraphRegistry;
use super::types::*;

// =============================================================================
// Types
// =============================================================================

/// Entity definition for scanner hydration
#[derive(Debug, Clone)]
pub struct ScannerEntity {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub aliases: Vec<String>,
}

/// Extracted relation from scan
#[derive(Debug, Clone)]
pub struct ScannedRelation {
    pub head_label: String,
    pub tail_label: String,
    pub relation_type: String,
    pub confidence: f64,
    pub source: RelationSource,
}

/// Source of the extracted relation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationSource {
    Explicit,      // [X] (REL) [Y] syntax
    CstInference,   // CST-based pattern matching
    GraphInference, // Graph-based inference
}

/// Extracted entity mention from scan
#[derive(Debug, Clone)]
pub struct ScannedMention {
    pub entity_label: String,
    pub entity_kind: String,
    pub source_note: String,
}

/// Result of ingesting scan results
#[derive(Debug, Clone, Default)]
pub struct IngestResult {
    pub entities_created: usize,
    pub entities_updated: usize,
    pub edges_created: usize,
    pub edges_updated: usize,
}

/// State tracking for smart hydration
#[derive(Debug, Default)]
pub struct HydrationState {
    /// Hash of entity set for change detection
    entity_set_hash: u64,
    /// Number of entities at last hydration
    entity_count: usize,
}

// =============================================================================
// Scanner Bridge
// =============================================================================

/// Bridge between GraphRegistry and Scanner
/// 
/// Responsibilities:
/// - Convert GraphRegistry nodes to ScannerEntity format
/// - Detect when entity set changes (avoid unnecessary re-hydration)
/// - Ingest scan results back into GraphRegistry
pub struct ScannerBridge {
    state: HydrationState,
}

impl ScannerBridge {
    pub fn new() -> Self {
        Self {
            state: HydrationState::default(),
        }
    }

    /// Get entities for scanner hydration, but only if set has changed
    /// 
    /// Returns Some(entities) if hydration is needed, None if unchanged
    pub fn get_entities_if_changed(&mut self, registry: &GraphRegistry) -> Result<Option<Vec<ScannerEntity>>, GraphError> {
        let nodes = registry.get_nodes(NodeFilter::default())?;
        
        // Calculate hash of current entity set
        let mut hasher = DefaultHasher::new();
        for node in &nodes {
            node.id.hash(&mut hasher);
            node.label.hash(&mut hasher);
            node.kind.as_str().hash(&mut hasher);
            for alias in &node.aliases {
                alias.hash(&mut hasher);
            }
        }
        let current_hash = hasher.finish();
        
        // Check if changed
        if current_hash == self.state.entity_set_hash && nodes.len() == self.state.entity_count {
            return Ok(None); // No change, skip hydration
        }
        
        // Update state
        self.state.entity_set_hash = current_hash;
        self.state.entity_count = nodes.len();
        
        // Convert to ScannerEntity format
        let entities: Vec<ScannerEntity> = nodes.into_iter()
            .map(|n| ScannerEntity {
                id: n.id,
                label: n.label,
                kind: n.kind.as_str().to_string(),
                aliases: n.aliases,
            })
            .collect();
        
        Ok(Some(entities))
    }

    /// Force get all entities (bypass change detection)
    pub fn get_all_entities(&self, registry: &GraphRegistry) -> Result<Vec<ScannerEntity>, GraphError> {
        let nodes = registry.get_nodes(NodeFilter::default())?;
        
        Ok(nodes.into_iter()
            .map(|n| ScannerEntity {
                id: n.id,
                label: n.label,
                kind: n.kind.as_str().to_string(),
                aliases: n.aliases,
            })
            .collect())
    }

    /// Ingest entity mentions from a scan
    /// 
    /// For each mention, registers the entity or increments mention count
    pub fn ingest_mentions(
        &mut self,
        registry: &GraphRegistry,
        mentions: Vec<ScannedMention>,
    ) -> Result<IngestResult, GraphError> {
        let mut result = IngestResult::default();
        
        for mention in mentions {
            let input = NodeInput {
                label: mention.entity_label,
                kind: NodeKind::from_str(&mention.entity_kind),
                source_note: mention.source_note,
                subtype: None,
                aliases: vec![],
                created_by: CreatedBy::Extraction,
                metadata: None,
            };
            
            let reg_result = registry.register_node(input)?;
            
            if reg_result.is_new {
                result.entities_created += 1;
            } else {
                result.entities_updated += 1;
            }
        }
        
        // Invalidate cache since we may have added entities
        if result.entities_created > 0 {
            self.state.entity_set_hash = 0; // Force next check to re-hydrate
        }
        
        Ok(result)
    }

    /// Ingest relations from a scan
    /// 
    /// For each relation, looks up entities by label and creates edge
    pub fn ingest_relations(
        &self,
        registry: &GraphRegistry,
        relations: Vec<ScannedRelation>,
        source_note: &str,
    ) -> Result<IngestResult, GraphError> {
        let mut result = IngestResult::default();
        
        for rel in relations {
            // Look up head and tail entities
            let head = match registry.find_node(&rel.head_label)? {
                Some(n) => n,
                None => continue, // Skip if entity not found
            };
            
            let tail = match registry.find_node(&rel.tail_label)? {
                Some(n) => n,
                None => continue, // Skip if entity not found
            };
            
            // Create edge
            let edge_input = EdgeInput {
                source_id: head.id,
                target_id: tail.id,
                edge_type: rel.relation_type,
                inverse_type: None,
                bidirectional: false,
                weight: 1.0,
                confidence: rel.confidence,
                created_by: match rel.source {
                    RelationSource::Explicit => CreatedBy::User,
                    _ => CreatedBy::Extraction,
                },
                source_note: Some(source_note.to_string()),
                metadata: None,
            };
            
            // Check if edge already exists (by type between same nodes)
            let existing_edges = registry.get_edges(&edge_input.source_id, Direction::Outgoing)?;
            let exists = existing_edges.iter().any(|e| 
                e.target_id == edge_input.target_id && e.edge_type == edge_input.edge_type
            );
            
            if !exists {
                registry.create_edge(edge_input)?;
                result.edges_created += 1;
            } else {
                result.edges_updated += 1; // Could update confidence here
            }
        }
        
        Ok(result)
    }

    /// Full ingest: mentions + relations
    pub fn ingest_scan_result(
        &mut self,
        registry: &GraphRegistry,
        mentions: Vec<ScannedMention>,
        relations: Vec<ScannedRelation>,
        source_note: &str,
    ) -> Result<IngestResult, GraphError> {
        // First ingest mentions (may create new entities)
        let mut result = self.ingest_mentions(registry, mentions)?;
        
        // Then ingest relations (uses entities)
        let rel_result = self.ingest_relations(registry, relations, source_note)?;
        
        result.edges_created += rel_result.edges_created;
        result.edges_updated += rel_result.edges_updated;
        
        Ok(result)
    }

    /// Check if hydration was already done and is still valid
    pub fn is_hydrated(&self) -> bool {
        self.state.entity_count > 0
    }

    /// Reset state (force next hydration)
    pub fn invalidate(&mut self) {
        self.state = HydrationState::default();
    }
}

impl Default for ScannerBridge {
    fn default() -> Self {
        Self::new()
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

    fn test_node(label: &str) -> NodeInput {
        NodeInput {
            label: label.to_string(),
            kind: NodeKind::Character,
            source_note: "test".to_string(),
            subtype: None,
            aliases: vec![],
            created_by: CreatedBy::User,
            metadata: None,
        }
    }

    #[test]
    fn test_get_entities_detects_changes() {
        let registry = test_registry();
        let mut bridge = ScannerBridge::new();
        
        // First call should return entities
        registry.register_node(test_node("Frodo")).unwrap();
        let result1 = bridge.get_entities_if_changed(&registry).unwrap();
        assert!(result1.is_some());
        assert_eq!(result1.unwrap().len(), 1);
        
        // Second call with no changes should return None
        let result2 = bridge.get_entities_if_changed(&registry).unwrap();
        assert!(result2.is_none());
        
        // Add entity, should detect change
        registry.register_node(test_node("Sam")).unwrap();
        let result3 = bridge.get_entities_if_changed(&registry).unwrap();
        assert!(result3.is_some());
        assert_eq!(result3.unwrap().len(), 2);
    }

    #[test]
    fn test_ingest_mentions() {
        let registry = test_registry();
        let mut bridge = ScannerBridge::new();
        
        let mentions = vec![
            ScannedMention {
                entity_label: "Gandalf".to_string(),
                entity_kind: "CHARACTER".to_string(),
                source_note: "note1".to_string(),
            },
            ScannedMention {
                entity_label: "Mordor".to_string(),
                entity_kind: "LOCATION".to_string(),
                source_note: "note1".to_string(),
            },
        ];
        
        let result = bridge.ingest_mentions(&registry, mentions).unwrap();
        
        assert_eq!(result.entities_created, 2);
        assert_eq!(result.entities_updated, 0);
        
        // Verify entities exist
        assert!(registry.find_node("Gandalf").unwrap().is_some());
        assert!(registry.find_node("Mordor").unwrap().is_some());
    }

    #[test]
    fn test_ingest_relations() {
        let registry = test_registry();
        let bridge = ScannerBridge::new();
        
        // First create entities
        registry.register_node(test_node("Frodo")).unwrap();
        registry.register_node(test_node("Ring")).unwrap();
        
        let relations = vec![
            ScannedRelation {
                head_label: "Frodo".to_string(),
                tail_label: "Ring".to_string(),
                relation_type: "OWNS".to_string(),
                confidence: 0.9,
                source: RelationSource::CstInference,
            },
        ];
        
        let result = bridge.ingest_relations(&registry, relations, "note1").unwrap();
        
        assert_eq!(result.edges_created, 1);
        
        // Verify edge exists
        let frodo = registry.find_node("Frodo").unwrap().unwrap();
        let edges = registry.get_edges(&frodo.id, Direction::Outgoing).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, "OWNS");
    }

    #[test]
    fn test_ingest_skips_unknown_entities() {
        let registry = test_registry();
        let bridge = ScannerBridge::new();
        
        // Don't create entities first
        let relations = vec![
            ScannedRelation {
                head_label: "Unknown1".to_string(),
                tail_label: "Unknown2".to_string(),
                relation_type: "KNOWS".to_string(),
                confidence: 0.5,
                source: RelationSource::GraphInference,
            },
        ];
        
        let result = bridge.ingest_relations(&registry, relations, "note1").unwrap();
        
        // Should skip, not error
        assert_eq!(result.edges_created, 0);
    }

    #[test]
    fn test_full_ingest() {
        let registry = test_registry();
        let mut bridge = ScannerBridge::new();
        
        let mentions = vec![
            ScannedMention {
                entity_label: "Aragorn".to_string(),
                entity_kind: "CHARACTER".to_string(),
                source_note: "note1".to_string(),
            },
            ScannedMention {
                entity_label: "Andúril".to_string(),
                entity_kind: "ITEM".to_string(),
                source_note: "note1".to_string(),
            },
        ];
        
        let relations = vec![
            ScannedRelation {
                head_label: "Aragorn".to_string(),
                tail_label: "Andúril".to_string(),
                relation_type: "WIELDS".to_string(),
                confidence: 1.0,
                source: RelationSource::Explicit,
            },
        ];
        
        let result = bridge.ingest_scan_result(&registry, mentions, relations, "note1").unwrap();
        
        assert_eq!(result.entities_created, 2);
        assert_eq!(result.edges_created, 1);
    }
}
