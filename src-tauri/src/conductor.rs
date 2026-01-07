//! ScanConductor: Unified coordinator for document scanning (Native Tauri)
//!
//! # Design Principles
//! 1. State machine: Uninitialized → Initialized → Ready
//! 2. One scan result serves both highlighting AND relationship extraction
//! 3. Zero-cost wrapper - just state gating around DocumentCortex
//!
//! # Usage
//! ```ignore
//! let mut conductor = ScanConductor::new();
//! conductor.init();
//! conductor.hydrate_entities(entities)?;
//! let result = conductor.scan(text, &[]); // Returns Some(ScanResult)
//! ```

use super::document::{DocumentCortex, ScanResult};
use super::implicit::EntityDefinition;
use super::incremental::IncrementalStats;
use super::relation::EntitySpan;

// =============================================================================
// State Machine
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Fresh instance, nothing initialized
    Uninitialized,
    /// Cortexes ready, but no entities hydrated
    Initialized,
    /// Fully ready - cortexes initialized AND entities hydrated
    Ready,
}

// =============================================================================
// ScanConductor
// =============================================================================

/// Single coordinator for all document scanning operations.
///
/// Ensures proper initialization and hydration ordering.
/// Eliminates race conditions between scanner and highlighter.
pub struct ScanConductor {
    cortex: DocumentCortex,
    state: State,
}

impl Default for ScanConductor {
    fn default() -> Self {
        Self::new()
    }
}

impl ScanConductor {
    /// Create a new uninitialized conductor
    pub fn new() -> Self {
        Self {
            cortex: DocumentCortex::default(),
            state: State::Uninitialized,
        }
    }

    /// Initialize internal cortexes. Idempotent - safe to call multiple times.
    pub fn init(&mut self) {
        if self.state == State::Uninitialized {
            self.cortex = DocumentCortex::new();
            self.state = State::Initialized;
        }
    }

    /// Hydrate entities for implicit matching.
    /// Auto-initializes if needed. Marks conductor as Ready.
    /// Also resets the change detector so subsequent scans re-process with new entities.
    pub fn hydrate_entities(&mut self, entities: Vec<EntityDefinition>) -> Result<(), String> {
        // Auto-init if caller forgot
        if self.state == State::Uninitialized {
            self.init();
        }
        self.cortex.hydrate_entities(entities)?;
        // CRITICAL: Reset change detector so next scan re-processes with new entities
        self.cortex.reset();
        self.state = State::Ready;
        Ok(())
    }

    /// Check if conductor is fully ready for scanning
    pub fn is_ready(&self) -> bool {
        self.state == State::Ready
    }

    /// Current state name (for debugging)
    pub fn state_name(&self) -> &'static str {
        match self.state {
            State::Uninitialized => "uninitialized",
            State::Initialized => "initialized",
            State::Ready => "ready",
        }
    }

    /// Unified scan. Returns None if not ready.
    pub fn scan(&mut self, text: &str, external_spans: &[EntitySpan]) -> Option<ScanResult> {
        if self.state != State::Ready {
            return None;
        }
        Some(self.cortex.scan(text, external_spans))
    }

    /// Force scan even if not fully ready (for testing/debugging)
    pub fn scan_force(&mut self, text: &str, external_spans: &[EntitySpan]) -> ScanResult {
        if self.state == State::Uninitialized {
            self.init();
        }
        self.cortex.scan(text, external_spans)
    }

    /// Get hydrated entity count (for debugging)
    pub fn entity_count(&self) -> usize {
        self.cortex.implicit_pattern_count()
    }

    /// Get incremental stats
    pub fn incremental_stats(&self) -> &IncrementalStats {
        self.cortex.incremental_stats()
    }

    /// Reset conductor to initialized state (clears entities, keeps cortex)
    pub fn reset(&mut self) {
        self.cortex.reset();
        if self.state == State::Ready {
            self.state = State::Initialized;
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(id: &str, label: &str, kind: &str) -> EntityDefinition {
        EntityDefinition {
            id: id.to_string(),
            label: label.to_string(),
            kind: kind.to_string(),
            aliases: vec![],
        }
    }

    #[test]
    fn test_conductor_rejects_scan_before_init() {
        let mut conductor = ScanConductor::new();
        let result = conductor.scan("test text", &[]);
        assert!(result.is_none(), "Should return None before init");
    }

    #[test]
    fn test_conductor_rejects_scan_after_init_but_before_hydrate() {
        let mut conductor = ScanConductor::new();
        conductor.init();
        let result = conductor.scan("test text", &[]);
        assert!(result.is_none(), "Should return None before hydration");
    }

    #[test]
    fn test_conductor_allows_scan_after_hydration() {
        let mut conductor = ScanConductor::new();
        conductor.init();
        conductor.hydrate_entities(vec![]).unwrap();

        let result = conductor.scan("test text", &[]);
        assert!(result.is_some(), "Should return Some after hydration");
    }

    #[test]
    fn test_conductor_auto_inits_on_hydrate() {
        let mut conductor = ScanConductor::new();
        // Skip init(), go straight to hydrate
        conductor.hydrate_entities(vec![]).unwrap();

        assert!(
            conductor.is_ready(),
            "Should be ready after hydrate (auto-init)"
        );
        let result = conductor.scan("test text", &[]);
        assert!(result.is_some(), "Should scan after auto-init");
    }

    #[test]
    fn test_conductor_finds_entities_after_hydration() {
        let mut conductor = ScanConductor::new();
        conductor
            .hydrate_entities(vec![make_entity("1", "Luffy", "CHARACTER")])
            .unwrap();

        let result = conductor.scan("Luffy is fighting", &[]).unwrap();
        assert_eq!(result.implicit.len(), 1, "Should find 1 implicit entity");
        assert_eq!(result.implicit[0].entity_label, "Luffy");
    }

    #[test]
    fn test_conductor_state_progression() {
        let mut conductor = ScanConductor::new();

        assert_eq!(conductor.state_name(), "uninitialized");
        assert!(!conductor.is_ready());

        conductor.init();
        assert_eq!(conductor.state_name(), "initialized");
        assert!(!conductor.is_ready());

        conductor.hydrate_entities(vec![]).unwrap();
        assert_eq!(conductor.state_name(), "ready");
        assert!(conductor.is_ready());
    }

    #[test]
    fn test_conductor_force_scan_works_before_ready() {
        let mut conductor = ScanConductor::new();
        let result = conductor.scan_force("test text", &[]);

        assert!(result.stats.timings.total_us >= 0);
    }

    #[test]
    fn test_conductor_reset_clears_ready_state() {
        let mut conductor = ScanConductor::new();
        conductor
            .hydrate_entities(vec![make_entity("1", "Luffy", "CHARACTER")])
            .unwrap();

        assert!(conductor.is_ready());

        conductor.reset();

        assert_eq!(conductor.state_name(), "initialized");
        assert!(!conductor.is_ready());
    }
}
