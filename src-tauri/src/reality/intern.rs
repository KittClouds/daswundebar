//! Global String Interner
//!
//! This module provides a thread-safe, global string interner using `lasso`.
//! It allows deduplicating strings across the reality engine to save memory
//! and speed up comparisons.

use lasso::{Spur, ThreadedRodeo};
use std::sync::OnceLock;

/// Global interner instance
static INTERNER: OnceLock<ThreadedRodeo> = OnceLock::new();

/// Get the global string interner, initializing it if necessary
pub fn interner() -> &'static ThreadedRodeo {
    INTERNER.get_or_init(ThreadedRodeo::default)
}

/// Intern a string, getting back a unique styling (Spur)
///
/// If the string was already interned, returns the existing Spur.
/// This operation is thread-safe.
pub fn intern(s: &str) -> Spur {
    interner().get_or_intern(s)
}

/// Resolve a Spur back to the original string
///
/// Returns a static reference because the interner keeps strings alive forever.
pub fn resolve(spur: &Spur) -> &'static str {
    interner().resolve(spur)
}

/// Check if a string has been interned
pub fn contains(s: &str) -> bool {
    interner().contains(s)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_roundtrip() {
        let input = "hello world";
        let spur = intern(input);
        let output = resolve(&spur);
        
        assert_eq!(input, output, "Resolved string should match original");
    }

    #[test]
    fn test_intern_dedup() {
        let s1 = "deduplicate me";
        let s2 = "deduplicate me"; // Same content
        
        let spur1 = intern(s1);
        let spur2 = intern(s2);
        
        assert_eq!(spur1, spur2, "Identical strings should produce identical Spurs");
        
        // Ensure memory address of resolved string is the same (pointing to same backing storage)
        let resolved_addr1 = resolve(&spur1).as_ptr();
        let resolved_addr2 = resolve(&spur2).as_ptr();
        assert_eq!(resolved_addr1, resolved_addr2, "Resolved strings should point to same memory");
    }

    #[test]
    fn test_intern_different() {
        let s1 = "apple";
        let s2 = "orange";
        
        let spur1 = intern(s1);
        let spur2 = intern(s2);
        
        assert_ne!(spur1, spur2, "Different strings should produce different Spurs");
    }
    
    #[test]
    fn test_contains() {
        let s = "unique_string_test_contains";
        assert!(!contains(s), "Should not contain string before interning");
        
        intern(s);
        
        assert!(contains(s), "Should contain string after interning");
    }
}
