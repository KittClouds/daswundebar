//! RelationFilter - Post-processing filter for relationship tuning (Native Tauri)
//!
//! Applies confidence adjustment based on:
//! - Distance between entities
//! - Sentence boundary detection
//! - Configurable thresholds
//!
//! Note: With the new CST + Graph inference pipeline, UnifiedRelation already
//! has confidence scores. This filter is for additional post-processing when
//! needed, or for legacy ExtractedRelation compatibility.

use serde::{Deserialize, Serialize};

use super::relation::ExtractedRelation;

// =============================================================================
// Types
// =============================================================================

/// A relation with adjusted confidence after filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredRelation {
    /// The original extracted relation
    #[serde(flatten)]
    pub relation: ExtractedRelation,
    /// Adjusted confidence after all filters applied
    pub adjusted_confidence: f64,
    /// Original confidence before adjustment
    pub original_confidence: f64,
    /// Debug info: why was confidence adjusted?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustment_reason: Option<String>,
    /// Was this relation kept after filtering?
    pub kept: bool,
}

/// Configuration for the relation filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Minimum confidence threshold to keep a relation (0.0 - 1.0)
    pub min_confidence: f64,
    /// Enable distance-based confidence decay
    pub distance_decay_enabled: bool,
    /// Multiplier for same-sentence relations (e.g., 1.2 = 20% boost)
    pub same_sentence_bonus: f64,
    /// Multiplier for cross-sentence relations (e.g., 0.6 = 40% penalty)
    pub cross_sentence_penalty: f64,
    /// Maximum distance to consider (beyond this, apply max penalty)
    pub max_distance: usize,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.4,
            distance_decay_enabled: true,
            same_sentence_bonus: 1.15,
            cross_sentence_penalty: 0.7,
            max_distance: 500,
        }
    }
}

/// Statistics from filtering
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterStats {
    pub input_count: usize,
    pub output_count: usize,
    pub dropped_by_confidence: usize,
    pub dropped_by_distance: usize,
    pub boosted_same_sentence: usize,
    pub penalized_cross_sentence: usize,
}

// =============================================================================
// RelationFilter
// =============================================================================

/// Post-processing filter for relationship tuning
pub struct RelationFilter {
    config: FilterConfig,
}

impl Default for RelationFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationFilter {
    /// Create a new RelationFilter with default config
    pub fn new() -> Self {
        Self {
            config: FilterConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: FilterConfig) -> Self {
        Self { config }
    }

    /// Set minimum confidence threshold
    pub fn set_min_confidence(&mut self, threshold: f64) {
        self.config.min_confidence = threshold.clamp(0.0, 1.0);
    }

    /// Enable/disable distance decay
    pub fn set_distance_decay(&mut self, enabled: bool) {
        self.config.distance_decay_enabled = enabled;
    }

    /// Set same-sentence bonus multiplier
    pub fn set_same_sentence_bonus(&mut self, bonus: f64) {
        self.config.same_sentence_bonus = bonus.max(1.0);
    }

    /// Set cross-sentence penalty multiplier
    pub fn set_cross_sentence_penalty(&mut self, penalty: f64) {
        self.config.cross_sentence_penalty = penalty.clamp(0.0, 1.0);
    }

    /// Set maximum distance
    pub fn set_max_distance(&mut self, distance: usize) {
        self.config.max_distance = distance;
    }

    /// Get current config
    pub fn config(&self) -> &FilterConfig {
        &self.config
    }

    /// Apply all filters to raw relations
    pub fn filter(
        &self,
        text: &str,
        relations: Vec<ExtractedRelation>,
    ) -> (Vec<FilteredRelation>, FilterStats) {
        let input_count = relations.len();
        let mut output = Vec::with_capacity(relations.len());
        let mut stats = FilterStats {
            input_count,
            ..Default::default()
        };

        for relation in relations {
            let original_confidence = relation.confidence;
            let mut adjusted = original_confidence;
            let mut reasons = Vec::new();

            // Calculate distance between head and tail
            let distance = self.calculate_distance(&relation);

            // Apply distance decay
            if self.config.distance_decay_enabled {
                let factor = self.distance_factor(distance);
                if factor < 1.0 {
                    adjusted *= factor;
                    reasons.push(format!(
                        "distance_decay({} chars, factor={:.2})",
                        distance, factor
                    ));
                }
            }

            // Check if beyond max distance
            if distance > self.config.max_distance {
                stats.dropped_by_distance += 1;
                output.push(FilteredRelation {
                    relation,
                    adjusted_confidence: adjusted,
                    original_confidence,
                    adjustment_reason: Some(format!(
                        "distance {} > max {}",
                        distance, self.config.max_distance
                    )),
                    kept: false,
                });
                continue;
            }

            // Apply sentence boundary bonus/penalty
            let same_sent = self.same_sentence(text, relation.head_end, relation.tail_start);
            if same_sent {
                adjusted *= self.config.same_sentence_bonus;
                reasons.push(format!(
                    "same_sentence_bonus({:.2}x)",
                    self.config.same_sentence_bonus
                ));
                stats.boosted_same_sentence += 1;
            } else {
                adjusted *= self.config.cross_sentence_penalty;
                reasons.push(format!(
                    "cross_sentence_penalty({:.2}x)",
                    self.config.cross_sentence_penalty
                ));
                stats.penalized_cross_sentence += 1;
            }

            // Clamp to 1.0 max
            adjusted = adjusted.min(1.0);

            // Check minimum confidence threshold
            let kept = adjusted >= self.config.min_confidence;
            if !kept {
                stats.dropped_by_confidence += 1;
                reasons.push(format!(
                    "below_threshold({:.2} < {:.2})",
                    adjusted, self.config.min_confidence
                ));
            }

            output.push(FilteredRelation {
                relation,
                adjusted_confidence: adjusted,
                original_confidence,
                adjustment_reason: if reasons.is_empty() {
                    None
                } else {
                    Some(reasons.join(", "))
                },
                kept,
            });
        }

        stats.output_count = output.iter().filter(|r| r.kept).count();
        (output, stats)
    }

    /// Filter and return only kept relations
    pub fn filter_kept(&self, text: &str, relations: Vec<ExtractedRelation>) -> Vec<FilteredRelation> {
        let (all, _) = self.filter(text, relations);
        all.into_iter().filter(|r| r.kept).collect()
    }

    /// Calculate distance between head entity and tail entity
    fn calculate_distance(&self, relation: &ExtractedRelation) -> usize {
        if relation.tail_start > relation.head_end {
            relation.tail_start - relation.head_end
        } else if relation.head_start > relation.tail_end {
            relation.head_start - relation.tail_end
        } else {
            0 // Overlapping
        }
    }

    /// Confidence decay factor based on distance
    fn distance_factor(&self, distance: usize) -> f64 {
        match distance {
            0..=50 => 1.0,     // Very close: full confidence
            51..=100 => 0.95,  // Close: minimal penalty
            101..=200 => 0.85, // Normal: slight penalty
            201..=300 => 0.70, // Further: moderate penalty
            301..=400 => 0.55, // Far: significant penalty
            401..=500 => 0.40, // Very far: major penalty
            _ => 0.25,         // Beyond max: severe penalty
        }
    }

    /// Check if two positions are in the same sentence
    fn same_sentence(&self, text: &str, pos1: usize, pos2: usize) -> bool {
        let start = pos1.min(pos2);
        let end = pos1.max(pos2);

        // Bounds check
        if end > text.len() {
            return false;
        }

        // Check for sentence-ending punctuation between positions
        let segment = &text[start..end];
        !segment.contains('.') && !segment.contains('!') && !segment.contains('?')
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_relation(
        head: &str,
        tail: &str,
        head_end: usize,
        tail_start: usize,
        confidence: f64,
    ) -> ExtractedRelation {
        ExtractedRelation {
            head_entity: head.to_string(),
            head_start: 0,
            head_end,
            tail_entity: tail.to_string(),
            tail_start,
            tail_end: tail_start + 10,
            relation_type: "TEST".to_string(),
            pattern_matched: "test".to_string(),
            pattern_start: head_end + 5,
            pattern_end: tail_start.saturating_sub(5),
            confidence,
        }
    }

    #[test]
    fn test_filter_keeps_high_confidence() {
        let filter = RelationFilter::new();
        let relations = vec![make_relation("A", "B", 10, 30, 0.9)];

        let (result, stats) = filter.filter("A tested B in the same sentence", relations);

        assert_eq!(stats.input_count, 1);
        assert_eq!(stats.output_count, 1);
        assert!(result[0].kept);
    }

    #[test]
    fn test_filter_drops_low_confidence() {
        let filter = RelationFilter::new();
        let relations = vec![
            make_relation("A", "B", 10, 500, 0.3), // Far apart, low base confidence
        ];

        let (result, stats) = filter.filter("A very long text. B appears here.", relations);

        assert!(!result[0].kept);
        assert!(stats.dropped_by_confidence > 0 || stats.dropped_by_distance > 0);
    }

    #[test]
    fn test_same_sentence_bonus() {
        let filter = RelationFilter::new();
        let relations = vec![make_relation("A", "B", 1, 8, 0.5)];

        let text = "A loves B very much";
        let (result, stats) = filter.filter(text, relations);

        // Same sentence: 0.5 * 1.15 = 0.575 > 0.5
        assert!(
            result[0].adjusted_confidence > 0.5,
            "Expected > 0.5, got {}",
            result[0].adjusted_confidence
        );
        assert_eq!(stats.boosted_same_sentence, 1);
    }

    #[test]
    fn test_cross_sentence_penalty() {
        let filter = RelationFilter::new();
        let relations = vec![make_relation("A", "B", 5, 50, 0.8)];

        let text = "A is here. Then something happened. B is there.";
        let (result, stats) = filter.filter(text, relations);

        assert!(result[0].adjusted_confidence < 0.8);
        assert_eq!(stats.penalized_cross_sentence, 1);
    }

    #[test]
    fn test_distance_decay() {
        let filter = RelationFilter::new();

        let close = make_relation("A", "B", 10, 30, 0.8);
        let far = make_relation("C", "D", 10, 350, 0.8);

        let text = "A is B and C is D";
        let (result, _) = filter.filter(text, vec![close, far]);

        assert!(result[0].adjusted_confidence > result[1].adjusted_confidence);
    }

    #[test]
    fn test_configurable_threshold() {
        let mut filter = RelationFilter::new();
        filter.set_min_confidence(0.7);

        let relations = vec![make_relation("A", "B", 10, 30, 0.6)];

        let (result, _) = filter.filter("A loves B", relations);

        // Even with bonus, 0.6 * 1.15 = 0.69 < 0.7
        assert!(!result[0].kept);
    }
}
