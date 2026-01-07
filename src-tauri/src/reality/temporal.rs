//! Temporal Causality Graph: Event chains with temporal ordering
//!
//! Evolution 3.0.4: Temporal Pathfinding
//!
//! Provides causal chain analysis with temporal consistency validation.
//! Events can TRIGGER, ENABLE, or PREVENT other events.

use std::collections::{HashMap, HashSet, VecDeque};
use super::graph::{ConceptGraph, ConceptNode, ConceptEdge, EdgeKind};

// =============================================================================
// Types
// =============================================================================

/// Type of causal relationship between events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CausalType {
    /// Event A directly causes Event B
    Triggers,
    /// Event A makes Event B possible (but doesn't directly cause it)
    Enables,
    /// Event A prevents Event B from happening
    Prevents,
}

impl CausalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CausalType::Triggers => "TRIGGERS",
            CausalType::Enables => "ENABLES",
            CausalType::Prevents => "PREVENTS",
        }
    }
}

/// An item in a causal chain
#[derive(Debug, Clone)]
pub struct CausalChainItem {
    pub event_id: String,
    pub event_name: String,
    pub timestamp: Option<i64>,
    pub depth: usize,
    pub causal_type: CausalType,
}

/// A temporal violation: causation flowing backwards in time
#[derive(Debug, Clone)]
pub struct TemporalViolation {
    pub cause_id: String,
    pub cause_name: String,
    pub cause_time: i64,
    pub effect_id: String,
    pub effect_name: String,
    pub effect_time: i64,
    pub delta_seconds: i64,
}

/// Statistics about the causality graph
#[derive(Debug, Clone, Default)]
pub struct CausalityStats {
    pub event_count: usize,
    pub causal_edge_count: usize,
    pub temporal_violations: usize,
    pub max_chain_depth: usize,
}

// =============================================================================
// CausalityGraph
// =============================================================================

/// A graph of events with causal relationships and temporal ordering
/// 
/// This extends ConceptGraph with:
/// - Temporal ordering (timestamps for events)
/// - Causal edge typing (TRIGGERS, ENABLES, PREVENTS)
/// - Temporal consistency validation
pub struct CausalityGraph {
    /// The underlying concept graph (events as nodes, causation as edges)
    graph: ConceptGraph,
    /// Temporal ordering: event_id → timestamp (epoch millis or fantasy time)
    temporal_order: HashMap<String, i64>,
}

impl Default for CausalityGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CausalityGraph {
    /// Create a new empty causality graph
    pub fn new() -> Self {
        Self {
            graph: ConceptGraph::new(),
            temporal_order: HashMap::new(),
        }
    }
    
    /// Create from a ConceptGraph with optional timestamps
    pub fn from_graph(graph: ConceptGraph, timestamps: HashMap<String, i64>) -> Self {
        Self {
            graph,
            temporal_order: timestamps,
        }
    }
    
    /// Add an event to the graph
    pub fn add_event(&mut self, id: &str, name: &str, timestamp: Option<i64>) {
        self.graph.ensure_node(ConceptNode::new(id, name, "EVENT"));
        if let Some(ts) = timestamp {
            self.temporal_order.insert(id.to_string(), ts);
        }
    }
    
    /// Add a causal link between events
    pub fn add_causal_link(
        &mut self,
        cause_id: &str,
        effect_id: &str,
        causal_type: CausalType,
        confidence: f64,
    ) {
        let edge = ConceptEdge::new(causal_type.as_str(), confidence);
        self.graph.add_edge(cause_id, effect_id, edge);
    }
    
    /// Get the timestamp for an event
    pub fn timestamp(&self, event_id: &str) -> Option<i64> {
        self.temporal_order.get(event_id).copied()
    }
    
    /// Get the underlying graph
    pub fn graph(&self) -> &ConceptGraph {
        &self.graph
    }
    
    /// Get mutable access to the underlying graph
    pub fn graph_mut(&mut self) -> &mut ConceptGraph {
        &mut self.graph
    }
    
    // -------------------------------------------------------------------------
    // Causal Chain Queries
    // -------------------------------------------------------------------------
    
    /// Find events that caused this event (upstream)
    pub fn causes_of(&self, event_id: &str, max_depth: usize) -> Vec<CausalChainItem> {
        self.traverse_causal_chain(event_id, max_depth, true)
    }
    
    /// Find events caused by this event (downstream)
    pub fn effects_of(&self, event_id: &str, max_depth: usize) -> Vec<CausalChainItem> {
        self.traverse_causal_chain(event_id, max_depth, false)
    }
    
    /// Traverse causal chain in a direction
    fn traverse_causal_chain(
        &self,
        start_id: &str,
        max_depth: usize,
        upstream: bool,
    ) -> Vec<CausalChainItem> {
        let mut results: Vec<CausalChainItem> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        
        queue.push_back((start_id.to_string(), 0));
        visited.insert(start_id.to_string());
        
        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            
            // Get connected events (incoming = causes, outgoing = effects)
            let connected = if upstream {
                self.graph.incoming_edges(&current_id)
            } else {
                self.graph.outgoing_edges(&current_id)
            };
            
            for (neighbor, edge) in connected {
                if visited.contains(&neighbor.id) {
                    continue;
                }
                
                visited.insert(neighbor.id.clone());
                
                let causal_type = match edge.relation.as_str() {
                    "TRIGGERS" => CausalType::Triggers,
                    "ENABLES" => CausalType::Enables,
                    "PREVENTS" => CausalType::Prevents,
                    _ => CausalType::Triggers, // Default
                };
                
                results.push(CausalChainItem {
                    event_id: neighbor.id.clone(),
                    event_name: neighbor.label.clone(),
                    timestamp: self.temporal_order.get(&neighbor.id).copied(),
                    depth: depth + 1,
                    causal_type,
                });
                
                queue.push_back((neighbor.id.clone(), depth + 1));
            }
        }
        
        // Sort by depth, then by timestamp if available
        results.sort_by(|a, b| {
            match a.depth.cmp(&b.depth) {
                std::cmp::Ordering::Equal => {
                    match (a.timestamp, b.timestamp) {
                        (Some(ta), Some(tb)) => ta.cmp(&tb),
                        _ => std::cmp::Ordering::Equal,
                    }
                }
                ord => ord,
            }
        });
        
        results
    }
    
    // -------------------------------------------------------------------------
    // Temporal Validation
    // -------------------------------------------------------------------------
    
    /// Validate temporal consistency: causes should precede effects
    /// 
    /// Returns violations where effect timestamp < cause timestamp
    pub fn validate_temporal_order(&self) -> Vec<TemporalViolation> {
        let mut violations: Vec<TemporalViolation> = Vec::new();
        
        for (source, target, edge) in self.graph.edges() {
            // Skip non-causal edges
            if !matches!(edge.relation.as_str(), "TRIGGERS" | "ENABLES") {
                continue;
            }
            
            let Some(cause_time) = self.temporal_order.get(&source.id) else {
                continue;
            };
            
            let Some(effect_time) = self.temporal_order.get(&target.id) else {
                continue;
            };
            
            // Violation: effect happens before cause
            if effect_time < cause_time {
                violations.push(TemporalViolation {
                    cause_id: source.id.clone(),
                    cause_name: source.label.clone(),
                    cause_time: *cause_time,
                    effect_id: target.id.clone(),
                    effect_name: target.label.clone(),
                    effect_time: *effect_time,
                    delta_seconds: cause_time - effect_time,
                });
            }
        }
        
        violations
    }
    
    /// Check if the causal graph is acyclic
    /// 
    /// A causal graph with cycles is problematic (event can't cause itself)
    pub fn is_acyclic(&self) -> bool {
        // Use DFS to detect cycles
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        
        for node in self.graph.nodes() {
            if self.has_cycle_dfs(&node.id, &mut visited, &mut rec_stack) {
                return false;
            }
        }
        
        true
    }
    
    fn has_cycle_dfs(
        &self,
        node_id: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        if rec_stack.contains(node_id) {
            return true; // Cycle detected
        }
        
        if visited.contains(node_id) {
            return false; // Already processed, no cycle from this node
        }
        
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());
        
        for (neighbor, _) in self.graph.outgoing_edges(node_id) {
            if self.has_cycle_dfs(&neighbor.id, visited, rec_stack) {
                return true;
            }
        }
        
        rec_stack.remove(node_id);
        false
    }
    
    /// Get statistics about the causality graph
    pub fn stats(&self) -> CausalityStats {
        let mut max_depth = 0;
        
        // Find max chain depth from any root event
        for node in self.graph.nodes() {
            if self.graph.incoming_edges(&node.id).is_empty() {
                // This is a root event (no causes)
                let effects = self.effects_of(&node.id, 100);
                if let Some(last) = effects.last() {
                    max_depth = max_depth.max(last.depth);
                }
            }
        }
        
        CausalityStats {
            event_count: self.graph.node_count(),
            causal_edge_count: self.graph.edge_count(),
            temporal_violations: self.validate_temporal_order().len(),
            max_chain_depth: max_depth,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    fn build_wano_arc() -> CausalityGraph {
        let mut graph = CausalityGraph::new();
        
        // Events with approximate timestamps (arc order)
        graph.add_event("alliance", "Alliance Forms", Some(1000));
        graph.add_event("udon", "Udon Prison Breakout", Some(2000));
        graph.add_event("raid", "Raid on Onigashima", Some(3000));
        graph.add_event("rooftop", "Rooftop Battle", Some(4000));
        graph.add_event("gear5", "Luffy Awakens Gear Fifth", Some(5000));
        graph.add_event("kaido_defeat", "Kaidou Defeated", Some(6000));
        graph.add_event("liberation", "Wano Liberation", Some(7000));
        
        // Causal links
        graph.add_causal_link("alliance", "raid", CausalType::Enables, 0.9);
        graph.add_causal_link("udon", "raid", CausalType::Enables, 0.8);
        graph.add_causal_link("raid", "rooftop", CausalType::Triggers, 1.0);
        graph.add_causal_link("rooftop", "gear5", CausalType::Triggers, 0.95);
        graph.add_causal_link("gear5", "kaido_defeat", CausalType::Enables, 1.0);
        graph.add_causal_link("kaido_defeat", "liberation", CausalType::Triggers, 1.0);
        
        graph
    }
    
    #[test]
    fn test_causality_graph_new() {
        let graph = CausalityGraph::new();
        assert_eq!(graph.graph().node_count(), 0);
    }
    
    #[test]
    fn test_add_event() {
        let mut graph = CausalityGraph::new();
        graph.add_event("e1", "Event One", Some(1000));
        
        assert_eq!(graph.graph().node_count(), 1);
        assert_eq!(graph.timestamp("e1"), Some(1000));
    }
    
    #[test]
    fn test_causes_of() {
        let graph = build_wano_arc();
        
        // What caused Kaidou's defeat?
        let causes = graph.causes_of("kaido_defeat", 5);
        
        assert!(!causes.is_empty());
        
        // Gear5 directly caused it
        assert!(causes.iter().any(|c| c.event_id == "gear5" && c.depth == 1));
        
        // Rooftop battle is 2 hops away
        assert!(causes.iter().any(|c| c.event_id == "rooftop" && c.depth == 2));
    }
    
    #[test]
    fn test_effects_of() {
        let graph = build_wano_arc();
        
        // What did the alliance enable?
        let effects = graph.effects_of("alliance", 5);
        
        assert!(!effects.is_empty());
        
        // Raid was directly enabled
        assert!(effects.iter().any(|e| e.event_id == "raid" && e.depth == 1));
        
        // Liberation is downstream
        assert!(effects.iter().any(|e| e.event_id == "liberation"));
    }
    
    #[test]
    fn test_temporal_validation_valid() {
        let graph = build_wano_arc();
        let violations = graph.validate_temporal_order();
        
        assert!(violations.is_empty(), "Well-ordered graph should have no violations");
    }
    
    #[test]
    fn test_temporal_validation_invalid() {
        let mut graph = CausalityGraph::new();
        
        // Effect happens BEFORE cause (invalid)
        graph.add_event("cause", "The Cause", Some(2000));
        graph.add_event("effect", "The Effect", Some(1000)); // Before!
        graph.add_causal_link("cause", "effect", CausalType::Triggers, 1.0);
        
        let violations = graph.validate_temporal_order();
        
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].cause_id, "cause");
        assert_eq!(violations[0].effect_id, "effect");
    }
    
    #[test]
    fn test_is_acyclic() {
        let graph = build_wano_arc();
        assert!(graph.is_acyclic(), "Linear causality should be acyclic");
    }
    
    #[test]
    fn test_cyclic_detection() {
        let mut graph = CausalityGraph::new();
        
        // Create a cycle: A → B → C → A
        graph.add_event("a", "Event A", None);
        graph.add_event("b", "Event B", None);
        graph.add_event("c", "Event C", None);
        
        graph.add_causal_link("a", "b", CausalType::Triggers, 1.0);
        graph.add_causal_link("b", "c", CausalType::Triggers, 1.0);
        graph.add_causal_link("c", "a", CausalType::Triggers, 1.0); // Cycle!
        
        assert!(!graph.is_acyclic(), "Should detect cycle");
    }
    
    #[test]
    fn test_stats() {
        let graph = build_wano_arc();
        let stats = graph.stats();
        
        assert_eq!(stats.event_count, 7);
        assert_eq!(stats.causal_edge_count, 6);
        assert_eq!(stats.temporal_violations, 0);
        assert!(stats.max_chain_depth >= 4, "Chain should be at least Alliance → Raid → Rooftop → Gear5 → Kaidou");
    }
}
