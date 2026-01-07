//! ChangeDetector: Content-Addressable Change Detection (Native Tauri)
//!
//! Uses content hashing to detect changes and skip redundant scans.
//! DefaultHasher for speed, with skip rate tracking.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// =============================================================================
// Types
// =============================================================================

/// Result of change detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeResult {
    /// True if content has changed since last check
    pub has_changed: bool,
    /// Current content hash
    pub content_hash: u64,
    /// Previous content hash (if any)
    pub previous_hash: Option<u64>,
}

// =============================================================================
// ChangeDetector
// =============================================================================

/// Content-addressable change detector
pub struct ChangeDetector {
    /// Hash of previous content
    last_hash: Option<u64>,
    /// Number of checks performed
    check_count: u64,
    /// Number of skipped (unchanged) checks
    skip_count: u64,
}

impl Default for ChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeDetector {
    pub fn new() -> Self {
        Self {
            last_hash: None,
            check_count: 0,
            skip_count: 0,
        }
    }

    /// Get skip rate as percentage
    pub fn skip_rate(&self) -> f64 {
        if self.check_count == 0 {
            return 0.0;
        }
        (self.skip_count as f64 / self.check_count as f64) * 100.0
    }

    /// Get total number of checks
    pub fn check_count(&self) -> u64 {
        self.check_count
    }

    /// Get number of skipped checks
    pub fn skip_count(&self) -> u64 {
        self.skip_count
    }

    /// Reset the detector state
    pub fn reset(&mut self) {
        self.last_hash = None;
        self.check_count = 0;
        self.skip_count = 0;
    }

    /// Check if content has changed
    /// Returns true if content is different from last check
    pub fn has_changed(&mut self, text: &str) -> bool {
        self.check_count += 1;

        let current_hash = Self::compute_hash(text);

        let changed = match self.last_hash {
            None => true, // First check always counts as changed
            Some(prev) => prev != current_hash,
        };

        if !changed {
            self.skip_count += 1;
        }

        self.last_hash = Some(current_hash);
        changed
    }

    /// Check and return detailed result
    pub fn check(&mut self, text: &str) -> ChangeResult {
        self.check_count += 1;

        let current_hash = Self::compute_hash(text);
        let previous_hash = self.last_hash;

        let has_changed = match previous_hash {
            None => true,
            Some(prev) => prev != current_hash,
        };

        if !has_changed {
            self.skip_count += 1;
        }

        self.last_hash = Some(current_hash);

        ChangeResult {
            has_changed,
            content_hash: current_hash,
            previous_hash,
        }
    }

    /// Compute hash of content
    fn compute_hash(text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// Get the last computed hash
    pub fn last_hash(&self) -> Option<u64> {
        self.last_hash
    }

    /// Force set the last hash (for testing or external sync)
    #[allow(dead_code)]
    pub fn set_last_hash(&mut self, hash: u64) {
        self.last_hash = Some(hash);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_check_returns_changed() {
        let mut detector = ChangeDetector::new();
        assert!(detector.has_changed("Hello world"));
    }

    #[test]
    fn test_same_content_unchanged() {
        let mut detector = ChangeDetector::new();

        detector.has_changed("Hello world");
        assert!(!detector.has_changed("Hello world"));
    }

    #[test]
    fn test_different_content_changed() {
        let mut detector = ChangeDetector::new();

        detector.has_changed("Hello world");
        assert!(detector.has_changed("Hello universe"));
    }

    #[test]
    fn test_skip_count() {
        let mut detector = ChangeDetector::new();

        detector.has_changed("Hello"); // First: changed
        detector.has_changed("Hello"); // Same: skipped
        detector.has_changed("Hello"); // Same: skipped

        assert_eq!(detector.check_count(), 3);
        assert_eq!(detector.skip_count(), 2);
    }

    #[test]
    fn test_skip_rate() {
        let mut detector = ChangeDetector::new();

        detector.has_changed("A"); // Changed
        detector.has_changed("A"); // Skipped
        detector.has_changed("A"); // Skipped
        detector.has_changed("A"); // Skipped

        // 3 skips out of 4 checks = 75%
        assert!((detector.skip_rate() - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_text() {
        let mut detector = ChangeDetector::new();

        detector.has_changed("");
        assert!(!detector.has_changed(""));
        assert!(detector.has_changed("not empty"));
    }

    #[test]
    fn test_reset() {
        let mut detector = ChangeDetector::new();

        detector.has_changed("Hello");
        detector.has_changed("Hello");
        assert_eq!(detector.check_count(), 2);

        detector.reset();
        assert_eq!(detector.check_count(), 0);
        assert_eq!(detector.skip_count(), 0);
        assert!(detector.last_hash().is_none());

        // After reset, first check is changed again
        assert!(detector.has_changed("Hello"));
    }

    #[test]
    fn test_check_result() {
        let mut detector = ChangeDetector::new();

        let result1 = detector.check("Hello");
        assert!(result1.has_changed);
        assert!(result1.previous_hash.is_none());

        let result2 = detector.check("Hello");
        assert!(!result2.has_changed);
        assert!(result2.previous_hash.is_some());
        assert_eq!(result2.content_hash, result1.content_hash);
    }

    #[test]
    fn test_hash_deterministic() {
        let mut detector = ChangeDetector::new();

        let result1 = detector.check("The quick brown fox");
        detector.reset();
        let result2 = detector.check("The quick brown fox");

        assert_eq!(result1.content_hash, result2.content_hash);
    }

    #[test]
    fn test_whitespace_matters() {
        let mut detector = ChangeDetector::new();

        detector.has_changed("Hello world");
        assert!(detector.has_changed("Hello  world")); // Extra space = change
        assert!(detector.has_changed("Hello world ")); // Trailing space = change
    }
}
