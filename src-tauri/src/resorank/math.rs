//! Math utilities for ResoRank
//!
//! Includes IDF, TF normalization, saturation, and bit operations.

use super::config::{F32, U32, Usize};

// =============================================================================
// IDF Calculation
// =============================================================================

/// Calculate Inverse Document Frequency (IDF)
///
/// Uses the standard BM25 IDF formula:
/// IDF = ln(1 + (N - df + 0.5) / (df + 0.5))
#[inline]
pub fn calculate_idf(total_documents: F32, doc_frequency: Usize) -> F32 {
    if doc_frequency == 0 {
        return 0.0;
    }

    let df = doc_frequency as f32;
    let ratio = (total_documents - df + 0.5) / (df + 0.5);
    
    (1.0 + ratio.max(0.0)).ln()
}

// =============================================================================
// Term Frequency Normalization
// =============================================================================

/// Normalized term frequency using BM25F length normalization
///
/// Formula: tf / (1 - b + b * (fieldLength / avgFieldLength))
#[inline]
pub fn normalized_term_frequency(
    tf: U32,
    field_length: U32,
    average_field_length: F32,
    b: F32,
) -> F32 {
    if average_field_length <= 0.0 || tf == 0 {
        return 0.0;
    }

    let denominator = 1.0 - b + b * (field_length as f32 / average_field_length);
    
    if denominator > 0.0 {
        tf as f32 / denominator
    } else {
        0.0
    }
}

/// BMX-enhanced normalized term frequency
///
/// Adds entropy adjustment to the denominator:
/// tf / (1 - b + b * (fieldLength / avgFieldLength) + gamma * E)
#[inline]
pub fn normalized_term_frequency_bmx(
    tf: U32,
    field_length: U32,
    average_field_length: F32,
    b: F32,
    avg_entropy: F32,
    gamma: F32,
) -> F32 {
    if average_field_length <= 0.0 || tf == 0 {
        return 0.0;
    }

    let length_norm = 1.0 - b + b * (field_length as f32 / average_field_length);
    let denominator = length_norm + gamma * avg_entropy;

    if denominator > 0.0 {
        tf as f32 / denominator
    } else {
        0.0
    }
}

// =============================================================================
// Saturation Functions
// =============================================================================

/// Term saturation function (BM25)
///
/// Formula: ((k1 + 1) * aggregatedScore) / (k1 + aggregatedScore)
#[inline]
pub fn saturate(aggregated_score: F32, k1: F32) -> F32 {
    saturate_bmx(aggregated_score, k1)
}

/// BMX-compatible saturation function
#[inline]
pub fn saturate_bmx(aggregated_score: F32, k1_or_alpha: F32) -> F32 {
    if !aggregated_score.is_finite() || aggregated_score <= 0.0 {
        return 0.0;
    }

    if k1_or_alpha <= 0.0 {
        return aggregated_score;
    }

    ((k1_or_alpha + 1.0) * aggregated_score) / (k1_or_alpha + aggregated_score)
}

// =============================================================================
// Bit Operations
// =============================================================================

/// Population count (number of set bits in a u32)
#[inline]
pub fn pop_count(mut n: U32) -> U32 {
    n = n - ((n >> 1) & 0x55555555);
    n = (n & 0x33333333) + ((n >> 2) & 0x33333333);
    (((n + (n >> 4)) & 0x0F0F0F0F).wrapping_mul(0x01010101)) >> 24
}

/// Format a number as binary string with fixed width
#[inline]
pub fn format_binary(n: U32, bits: U32) -> String {
    format!("{:0width$b}", n, width = bits as usize)
}

// =============================================================================
// Segment Calculation
// =============================================================================

/// Calculate adaptive segment count based on document length
#[inline]
pub fn adaptive_segment_count(doc_length: U32, tokens_per_segment: U32) -> U32 {
    let raw = (doc_length as f32 / tokens_per_segment as f32).ceil() as u32;
    raw.clamp(8, 32)
}

// =============================================================================
// BMX Parameter Calculations
// =============================================================================

/// Sigmoid function for entropy calculation
#[inline]
pub fn sigmoid(x: F32) -> F32 {
    1.0 / (1.0 + (-x).exp())
}

/// Calculate adaptive alpha parameter (BMX Equation 3)
#[inline]
pub fn calculate_adaptive_alpha(average_document_length: F32) -> F32 {
    (average_document_length / 100.0).clamp(0.5, 1.5)
}

/// Calculate beta parameter for similarity boost
#[inline]
pub fn calculate_beta(total_documents: Usize) -> F32 {
    1.0 / (1.0 + total_documents as f32).ln()
}

/// Calculate normalized score
#[inline]
pub fn normalize_score(raw_score: F32, query_length: usize, total_documents: Usize) -> F32 {
    let max_idf_approx = (1.0 + (total_documents as f32 - 0.5) / 1.5).ln();
    let score_max = query_length as f32 * (max_idf_approx + 1.0);
    
    if score_max > 0.0 {
        raw_score / score_max
    } else {
        0.0
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_idf() {
        let idf_rare = calculate_idf(100.0, 1);
        assert!(idf_rare > 3.0);

        let idf_common = calculate_idf(100.0, 50);
        assert!(idf_common < 1.0);
        assert!(idf_common >= 0.0);

        let idf_zero = calculate_idf(100.0, 0);
        assert_eq!(idf_zero, 0.0);
    }

    #[test]
    fn test_normalized_term_frequency() {
        let ntf = normalized_term_frequency(3, 100, 100.0, 0.75);
        assert!((ntf - 3.0).abs() < 0.001);

        let ntf_long = normalized_term_frequency(3, 200, 100.0, 0.75);
        assert!(ntf_long < 3.0);

        let ntf_short = normalized_term_frequency(3, 50, 100.0, 0.75);
        assert!(ntf_short > 3.0);
    }

    #[test]
    fn test_saturate() {
        let sat1 = saturate(1.0, 1.2);
        let sat2 = saturate(2.0, 1.2);
        let sat10 = saturate(10.0, 1.2);

        assert!(sat2 < 2.0 * sat1);
        assert!(sat10 < 10.0 * sat1);

        let sat100 = saturate(100.0, 1.2);
        assert!(sat100 < 2.2);
    }

    #[test]
    fn test_pop_count() {
        assert_eq!(pop_count(0b0000), 0);
        assert_eq!(pop_count(0b0001), 1);
        assert_eq!(pop_count(0b1111), 4);
        assert_eq!(pop_count(0b10101010), 4);
        assert_eq!(pop_count(0xFFFFFFFF), 32);
    }

    #[test]
    fn test_adaptive_segment_count() {
        assert_eq!(adaptive_segment_count(100, 50), 8);
        assert_eq!(adaptive_segment_count(1000, 50), 20);
        assert_eq!(adaptive_segment_count(5000, 50), 32);
    }

    #[test]
    fn test_sigmoid() {
        assert!((sigmoid(0.0) - 0.5).abs() < 0.001);
        assert!(sigmoid(10.0) > 0.999);
        assert!(sigmoid(-10.0) < 0.001);
    }

    #[test]
    fn test_adaptive_alpha() {
        assert_eq!(calculate_adaptive_alpha(50.0), 0.5);
        assert_eq!(calculate_adaptive_alpha(100.0), 1.0);
        assert_eq!(calculate_adaptive_alpha(200.0), 1.5);
    }
}
