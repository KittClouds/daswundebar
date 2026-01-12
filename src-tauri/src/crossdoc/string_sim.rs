//! String similarity algorithms for entity linking
//! 
//! Wraps `strsim` crate and adds normalization logic.

use strsim;

/// Normalize a string for comparison
/// - Lowercases
/// - Trims whitespace
/// - Removes punctuation (optional, keeping simple for now)
pub fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Compute Levenshtein similarity (normalized 0.0 to 1.0)
pub fn levenshtein_similarity(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 1.0;
    }
    
    // strsim::levenshtein returns distance (usize)
    let distance = strsim::normalized_levenshtein(a, b);
    distance as f32
}

/// Compute Jaro-Winkler similarity (0.0 to 1.0)
pub fn jaro_winkler(a: &str, b: &str) -> f32 {
    strsim::jaro_winkler(a, b) as f32
}

/// Compute similarity using the best available algorithm
/// Returns the MAXIMUM similarity score from available methods
pub fn string_similarity(a: &str, b: &str) -> f32 {
    let norm_a = normalize(a);
    let norm_b = normalize(b);
    
    if norm_a == norm_b {
        return 1.0;
    }
    
    let jw_score = jaro_winkler(&norm_a, &norm_b);
    let lev_score = levenshtein_similarity(&norm_a, &norm_b);
    
    // Return the max score
    jw_score.max(lev_score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("  Gandalf  "), "gandalf");
        assert_eq!(normalize("Aragorn"), "aragorn");
    }

    #[test]
    fn test_exact_match() {
        assert_eq!(string_similarity("Gandalf", "Gandalf"), 1.0);
        assert_eq!(string_similarity("Gandalf", "gandalf"), 1.0);
    }

    #[test]
    fn test_similar_entities() {
        // High similarity expected
        let score = string_similarity("Gandalf", "Gandolf");
        assert!(score > 0.85, "Gandalf != Gandolf score: {}", score);
        
        // Low similarity expected
        let score = string_similarity("Gandalf", "Sauron");
        assert!(score < 0.5, "Gandalf == Sauron score: {}", score);
    }
    
    #[test]
    fn test_levenshtein() {
        // "kitten" vs "sitting" -> 3 edits / 7 max len -> ~0.57 similarity, normalized_levenshtein returns 0.5714
        let score = levenshtein_similarity("kitten", "sitting");
        assert!(score > 0.5);
        assert!(score < 0.6);
    }
}
