//! Entity Unification: Alias resolution for entity merging
//!
//! Phase 3.5.3 of Evolution 1.5: Cross-Document Analysis
//!
//! Manages canonical entity IDs and alias mappings.
//! Examples:
//! - "strider" → "aragorn"
//! - "samwise" → "sam"

use std::collections::{HashMap, HashSet};

// =============================================================================
// EntityUnifier
// =============================================================================

/// Entity unifier with alias resolution
/// 
/// Manages canonical entity IDs and alias mappings.
/// All lookups are case-insensitive.
#[derive(Debug, Clone, Default)]
pub struct EntityUnifier {
    /// Alias → Canonical ID (both normalized to lowercase)
    aliases: HashMap<String, String>,
}

impl EntityUnifier {
    /// Create a new empty unifier
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add an alias mapping
    /// 
    /// Both alias and canonical are normalized to lowercase.
    pub fn add_alias(&mut self, alias: &str, canonical: &str) {
        let alias_normalized = alias.trim().to_lowercase();
        let canonical_normalized = canonical.trim().to_lowercase();
        
        // Don't add self-referential aliases
        if alias_normalized != canonical_normalized {
            self.aliases.insert(alias_normalized, canonical_normalized);
        }
    }
    
    /// Resolve an entity ID through alias chain
    /// 
    /// Returns canonical ID (or normalized input if no alias).
    /// Prevents infinite loops from circular references.
    pub fn resolve(&self, entity_id: &str) -> String {
        let normalized = entity_id.trim().to_lowercase();
        
        // Follow alias chain with cycle detection
        let mut current = normalized.clone();
        let mut seen = HashSet::new();
        seen.insert(current.clone());
        
        while let Some(canonical) = self.aliases.get(&current) {
            if seen.contains(canonical) {
                // Cycle detected, return current
                break;
            }
            seen.insert(canonical.clone());
            current = canonical.clone();
        }
        
        current
    }
    
    /// Bulk add aliases from a list of (alias, canonical) pairs
    pub fn add_aliases(&mut self, mappings: &[(&str, &str)]) {
        for (alias, canonical) in mappings {
            self.add_alias(alias, canonical);
        }
    }
    
    /// Get all known aliases that point to a canonical ID
    pub fn aliases_of(&self, canonical: &str) -> Vec<String> {
        let canonical_lower = canonical.trim().to_lowercase();
        self.aliases
            .iter()
            .filter(|(_, v)| **v == canonical_lower)
            .map(|(k, _)| k.clone())
            .collect()
    }
    
    /// Check if an entity has any known aliases
    pub fn has_aliases(&self, entity_id: &str) -> bool {
        let normalized = entity_id.trim().to_lowercase();
        
        // Either it IS an alias, or something points to it
        self.aliases.contains_key(&normalized) || 
        self.aliases.values().any(|v| v == &normalized)
    }
    
    /// Count of alias mappings
    pub fn alias_count(&self) -> usize {
        self.aliases.len()
    }
    
    /// Check if unifier is empty
    pub fn is_empty(&self) -> bool {
        self.aliases.is_empty()
    }
    
    /// Clear all aliases
    pub fn clear(&mut self) {
        self.aliases.clear();
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unifier_new_empty() {
        let unifier = EntityUnifier::new();
        assert!(unifier.is_empty());
        assert_eq!(unifier.alias_count(), 0);
    }
    
    #[test]
    fn test_unifier_resolve_alias() {
        let mut unifier = EntityUnifier::new();
        unifier.add_alias("strider", "aragorn");
        
        assert_eq!(unifier.resolve("strider"), "aragorn");
        assert_eq!(unifier.resolve("aragorn"), "aragorn"); // canonical returns itself
    }
    
    #[test]
    fn test_unifier_case_insensitive() {
        let mut unifier = EntityUnifier::new();
        unifier.add_alias("Strider", "Aragorn");
        
        assert_eq!(unifier.resolve("STRIDER"), "aragorn");
        assert_eq!(unifier.resolve("strider"), "aragorn");
        assert_eq!(unifier.resolve("StRiDeR"), "aragorn");
    }
    
    #[test]
    fn test_unifier_chain_resolution() {
        let mut unifier = EntityUnifier::new();
        unifier.add_alias("ranger", "strider");
        unifier.add_alias("strider", "aragorn");
        
        // Should resolve full chain
        assert_eq!(unifier.resolve("ranger"), "aragorn");
        assert_eq!(unifier.resolve("strider"), "aragorn");
    }
    
    #[test]
    fn test_unifier_no_infinite_loop() {
        let mut unifier = EntityUnifier::new();
        unifier.add_alias("a", "b");
        unifier.add_alias("b", "c");
        unifier.add_alias("c", "a"); // Cycle!
        
        // Should not hang - returns some node in the cycle
        let result = unifier.resolve("a");
        // Could be any of a, b, c depending on where cycle breaks
        assert!(result == "a" || result == "b" || result == "c");
    }
    
    #[test]
    fn test_unifier_aliases_of() {
        let mut unifier = EntityUnifier::new();
        unifier.add_alias("strider", "aragorn");
        unifier.add_alias("ranger", "aragorn");
        unifier.add_alias("estel", "aragorn");
        
        let aliases = unifier.aliases_of("aragorn");
        assert_eq!(aliases.len(), 3);
        assert!(aliases.contains(&"strider".to_string()));
        assert!(aliases.contains(&"ranger".to_string()));
        assert!(aliases.contains(&"estel".to_string()));
    }
    
    #[test]
    fn test_unifier_has_aliases() {
        let mut unifier = EntityUnifier::new();
        unifier.add_alias("strider", "aragorn");
        
        assert!(unifier.has_aliases("strider")); // Is an alias
        assert!(unifier.has_aliases("aragorn")); // Something points to it
        assert!(!unifier.has_aliases("frodo"));  // No relationship
    }
    
    #[test]
    fn test_unifier_bulk_add() {
        let mut unifier = EntityUnifier::new();
        unifier.add_aliases(&[
            ("strider", "aragorn"),
            ("ranger", "aragorn"),
            ("samwise", "sam"),
        ]);
        
        assert_eq!(unifier.alias_count(), 3);
        assert_eq!(unifier.resolve("strider"), "aragorn");
        assert_eq!(unifier.resolve("samwise"), "sam");
    }
    
    #[test]
    fn test_unifier_unknown_returns_normalized() {
        let unifier = EntityUnifier::new();
        
        // Unknown entity just gets normalized
        assert_eq!(unifier.resolve("Frodo Baggins"), "frodo baggins");
        assert_eq!(unifier.resolve("  GANDALF  "), "gandalf");
    }
    
    #[test]
    fn test_unifier_clear() {
        let mut unifier = EntityUnifier::new();
        unifier.add_alias("strider", "aragorn");
        
        assert!(!unifier.is_empty());
        
        unifier.clear();
        
        assert!(unifier.is_empty());
        assert_eq!(unifier.resolve("strider"), "strider");
    }
    
    #[test]
    fn test_unifier_self_reference_ignored() {
        let mut unifier = EntityUnifier::new();
        unifier.add_alias("aragorn", "aragorn"); // Self-reference
        
        // Should not create a mapping
        assert!(unifier.is_empty());
    }
}
