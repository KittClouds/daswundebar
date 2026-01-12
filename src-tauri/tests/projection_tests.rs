#[cfg(test)]
mod tests {
    use app_lib::graph::projection::GraphProjection;
    use app_lib::graph::graph_backend::{GraphBackend, GraphNode, GraphEdge};
    use app_lib::graph::schema::init_schema;
    use cozo::DbInstance;

    fn create_test_db() -> DbInstance {
        let db = DbInstance::new("mem", "", Default::default()).unwrap();
        init_schema(&db).unwrap();
        db
    }

    #[test]
    fn test_projection_lifecycle() {
        let mut proj = GraphProjection::new();
        assert_eq!(proj.node_count().unwrap(), 0);

        // Verify hydration from empty DB
        let db = create_test_db();
        proj.hydrate(&db).expect("Hydration failed"); 
        assert_eq!(proj.node_count().unwrap(), 0);
    }

    #[test]
    fn test_hydration_with_data() {
        let db = create_test_db();
        // Insert nodes
        let node_script = r#"
            ?[id, label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata] <- [
                ['n1', 'Node 1', 'node 1', 'Concept', '', '', 0.0, 'test', 0, ''],
                ['n2', 'Node 2', 'node 2', 'Concept', '', '', 0.0, 'test', 0, '']
            ]
            :put nodes {id => label, normalized, kind, subtype, source_note, created_at, created_by, mention_count, metadata}
        "#;
        if let Err(e) = db.run_script(node_script, Default::default(), cozo::ScriptMutability::Mutable) {
            panic!("Failed to insert nodes: {}", e);
        }

        // Insert edges
        let edge_script = r#"
            ?[id, source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata] <- [
                ['e1', 'n1', 'n2', 'bonds', '', false, 1.0, 1.0, 0.0, 'test', '', '']
            ]
            :put edges {id => source_id, target_id, edge_type, inverse_type, bidirectional, weight, confidence, created_at, created_by, source_note, metadata}
        "#;
        if let Err(e) = db.run_script(edge_script, Default::default(), cozo::ScriptMutability::Mutable) {
            panic!("Failed to insert edge: {}", e);
        }

        let mut proj = GraphProjection::new();
        proj.hydrate(&db).expect("Hydration failed");

        assert_eq!(proj.node_count().unwrap(), 2);
        assert_eq!(proj.edge_count().unwrap(), 1);
        
        let path = proj.shortest_path("n1", "n2").unwrap();
        assert!(path.is_some());
        assert_eq!(path.unwrap().distance, 1.0);
    }
}
