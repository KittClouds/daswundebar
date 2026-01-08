//! DocumentCortex: Unified Document Scanner (Native Tauri)
//!
//! Single scan() call for all extraction types:
//! - Triple extraction (via TripleCortex)
//! - Implicit entity mentions (via ImplicitCortex)
//! - Temporal expression detection (via TemporalCortex)
//! - Relationship extraction (via RelationEngine)
//! - Structured relations (via StructuredRelationExtractor)
//!
//! Supports incremental scanning for sub-20ms updates.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use super::chunker::Chunker;
use super::implicit::{EntityDefinition, ImplicitCortex, ImplicitMention};
use super::incremental::{self, Delta, ExtractedItems, IncrementalState, IncrementalStats};
use super::relation::{EntitySpan, RelationEngine, UnifiedRelation};
use super::structured_relation::{StructuredRelation, StructuredRelationExtractor};
use super::temporal::{TemporalCortex, TemporalMention};
use super::triple::{ExtractedTriple, TripleCortex};

// =============================================================================
// Types
// =============================================================================

/// Timing statistics for each scan phase
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanTimings {
    pub total_us: u64,
    pub relation_us: u64,
    pub temporal_us: u64,
    pub implicit_us: u64,
    pub structured_us: u64,
    pub triple_us: u64,
    pub unified_us: u64,
}

/// Aggregate statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanStats {
    pub timings: ScanTimings,
    pub content_hash: String,
    pub was_skipped: bool,
    pub was_incremental: bool,
    pub entities_found: usize,
    pub temporal_found: usize,
    pub implicit_found: usize,
    pub triples_found: usize,
    pub structured_found: usize,
    pub unified_found: usize,
}

/// Unified scan result
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanResult {
    pub implicit: Vec<ImplicitMention>,
    pub triples: Vec<ExtractedTriple>,
    pub structured: Vec<StructuredRelation>,
    pub temporal: Vec<TemporalMention>,
    pub unified_relations: Vec<UnifiedRelation>,
    pub stats: ScanStats,
}

/// Change detection result
#[derive(Debug)]
struct ChangeResult {
    has_changed: bool,
    content_hash: u64,
}

/// Simple change detector
struct ChangeDetector {
    last_hash: Option<u64>,
    total_checks: u64,
    skipped_checks: u64,
}

impl ChangeDetector {
    fn new() -> Self {
        Self {
            last_hash: None,
            total_checks: 0,
            skipped_checks: 0,
        }
    }

    fn check(&mut self, text: &str) -> ChangeResult {
        self.total_checks += 1;
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        let has_changed = self.last_hash != Some(hash);
        if !has_changed {
            self.skipped_checks += 1;
        }
        self.last_hash = Some(hash);

        ChangeResult {
            has_changed,
            content_hash: hash,
        }
    }

    fn skip_rate(&self) -> f64 {
        if self.total_checks == 0 {
            0.0
        } else {
            (self.skipped_checks as f64 / self.total_checks as f64) * 100.0
        }
    }

    fn reset(&mut self) {
        self.last_hash = None;
    }
}

// =============================================================================
// DocumentCortex
// =============================================================================

/// Unified document scanner
pub struct DocumentCortex {
    // Shared chunker (expensive to create, reuse across scans)
    chunker: Chunker,
    
    // Core extractors
    relation_engine: RelationEngine,
    implicit_cortex: ImplicitCortex,
    triple_cortex: TripleCortex,
    temporal_cortex: TemporalCortex,
    structured_extractor: StructuredRelationExtractor,

    // State
    change_detector: ChangeDetector,
    last_result: Option<ScanResult>,
    incremental_state: Option<IncrementalState>,
    incremental_stats: IncrementalStats,
}

impl Default for DocumentCortex {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentCortex {
    /// Create a new DocumentCortex
    pub fn new() -> Self {
        Self {
            chunker: Chunker::new(),  // Single chunker instance (400+ word lexicon)
            relation_engine: RelationEngine::new(),
            implicit_cortex: ImplicitCortex::new(),
            triple_cortex: TripleCortex::new(),
            temporal_cortex: TemporalCortex::new(),
            structured_extractor: StructuredRelationExtractor::new(),
            change_detector: ChangeDetector::new(),
            last_result: None,
            incremental_state: None,
            incremental_stats: IncrementalStats::default(),
        }
    }

    /// Get pattern count for implicit entity matching
    pub fn implicit_pattern_count(&self) -> usize {
        self.implicit_cortex.pattern_count()
    }

    /// Get skip rate from change detector
    pub fn skip_rate(&self) -> f64 {
        self.change_detector.skip_rate()
    }

    /// Get incremental stats
    pub fn incremental_stats(&self) -> &IncrementalStats {
        &self.incremental_stats
    }

    /// Reset change detector, cached result, and incremental state
    pub fn reset(&mut self) {
        self.change_detector.reset();
        self.last_result = None;
        self.incremental_state = None;
    }

    /// Hydrate implicit entity matcher with entities
    pub fn hydrate_entities(&mut self, entities: Vec<EntityDefinition>) -> Result<(), String> {
        self.implicit_cortex.hydrate(entities);
        self.implicit_cortex.build()
    }

    /// Unified scan - one call extracts everything
    pub fn scan(&mut self, text: &str, external_spans: &[EntitySpan]) -> ScanResult {
        // FAST PATH: Empty or whitespace-only text
        if text.trim().is_empty() {
            return ScanResult::default();
        }
        
        let overall_start = Instant::now();

        // Check for changes
        let change_result = self.change_detector.check(text);

        // If unchanged and we have a cached result, return it
        if !change_result.has_changed {
            if let Some(ref cached) = self.last_result {
                let mut result = cached.clone();
                result.stats.was_skipped = true;
                result.stats.content_hash = format!("{:x}", change_result.content_hash);
                result.stats.timings.total_us = overall_start.elapsed().as_micros() as u64;
                return result;
            }
        }

        // Try incremental path if we have previous state
        if let Some(ref state) = self.incremental_state {
            let delta = incremental::compute_delta(&state.chunks, text);
            if delta.should_use_incremental() {
                let dirty_chunks = delta.dirty_chunks;
                let total_chunks = delta.total_chunks;

                let result =
                    self.scan_incremental(text, external_spans, delta, change_result.content_hash);
                self.incremental_stats.incremental_count += 1;
                let count = self.incremental_stats.incremental_count as f64;
                self.incremental_stats.avg_dirty_ratio = (self.incremental_stats.avg_dirty_ratio
                    * (count - 1.0)
                    + dirty_chunks as f64 / total_chunks as f64)
                    / count;
                return result;
            }
        }

        // Full rescan
        self.incremental_stats.full_rescan_count += 1;
        self.scan_full(text, external_spans, change_result.content_hash)
    }

    /// Incremental scan - only rescan dirty regions
    fn scan_incremental(
        &mut self,
        text: &str,
        _external_spans: &[EntitySpan],
        delta: Delta,
        content_hash: u64,
    ) -> ScanResult {
        let overall_start = Instant::now();
        let mut result = ScanResult::default();
        result.stats.content_hash = format!("{:x}", content_hash);
        result.stats.was_incremental = true;

        // Start with cached items
        let state = self.incremental_state.take().unwrap_or_default();

        // Clone and shift preserved items
        let mut unified_relations = state.extracted_items.relations.clone();
        let mut implicit = state.extracted_items.implicit.clone();
        let mut triples = state.extracted_items.triples.clone();
        let mut temporal = state.extracted_items.temporal.clone();
        let mut structured = state.extracted_items.structured.clone();

        incremental::shift_items(&mut unified_relations, &delta);
        incremental::shift_items(&mut implicit, &delta);
        incremental::shift_items(&mut triples, &delta);
        incremental::shift_items(&mut temporal, &delta);
        incremental::shift_items(&mut structured, &delta);

        // Extract from dirty regions only
        for (range, dirty_text) in incremental::extract_dirty_text(text, &delta) {
            let offset = range.start;

            // Triple extraction on dirty region
            let triple_start = Instant::now();
            for mut triple in self.triple_cortex.extract(dirty_text) {
                triple.start += offset;
                triple.end += offset;
                triples.push(triple);
            }
            result.stats.timings.triple_us += triple_start.elapsed().as_micros() as u64;

            // Implicit mentions from dirty region
            let implicit_start = Instant::now();
            for mut mention in self.implicit_cortex.find_mentions(dirty_text) {
                mention.start += offset;
                mention.end += offset;
                implicit.push(mention);
            }
            result.stats.timings.implicit_us += implicit_start.elapsed().as_micros() as u64;

            // Build spans from implicit mentions for relation extraction
            let all_spans: Vec<EntitySpan> = implicit
                .iter()
                .filter(|m| m.start >= offset && m.end <= offset + dirty_text.len())
                .map(|mention| EntitySpan {
                    label: mention.entity_label.clone(),
                    entity_id: Some(mention.entity_id.clone()),
                    start: mention.start,
                    end: mention.end,
                    kind: Some(mention.entity_kind.clone()),
                })
                .collect();

            // Relation extraction on dirty region
            let relation_start = Instant::now();
            let adjusted_spans: Vec<EntitySpan> = all_spans
                .iter()
                .map(|s| EntitySpan {
                    label: s.label.clone(),
                    entity_id: s.entity_id.clone(),
                    start: s.start.saturating_sub(offset),
                    end: s.end.saturating_sub(offset),
                    kind: s.kind.clone(),
                })
                .collect();

            let (unified_rels, _) = self
                .relation_engine
                .extract(dirty_text, &adjusted_spans, &[]);
            for rel in unified_rels {
                let adjusted_span = rel.span.map(|(s, e)| (s + offset, e + offset));
                unified_relations.push(UnifiedRelation {
                    span: adjusted_span,
                    ..rel
                });
            }
            result.stats.timings.unified_us += relation_start.elapsed().as_micros() as u64;

            // Temporal from dirty region
            let temporal_start = Instant::now();
            let temporal_result = self.temporal_cortex.scan(dirty_text);
            for mut mention in temporal_result.mentions {
                mention.start += offset;
                mention.end += offset;
                temporal.push(mention);
            }
            result.stats.timings.temporal_us += temporal_start.elapsed().as_micros() as u64;
            
            // Structured relations from dirty region
            let structured_start = Instant::now();
            for mut sr in self.structured_extractor.extract_structured(dirty_text, &adjusted_spans) {
                sr.subject_span.start += offset;
                sr.subject_span.end += offset;
                sr.predicate_span.start += offset;
                sr.predicate_span.end += offset;
                if let Some(ref mut obj) = sr.object_span {
                    obj.start += offset;
                    obj.end += offset;
                }
                structured.push(sr);
            }
            result.stats.timings.structured_us += structured_start.elapsed().as_micros() as u64;
        }

        // Deduplicate
        unified_relations.sort_by_key(|r| r.span.unwrap_or((0, 0)));
        unified_relations
            .dedup_by(|a, b| a.head == b.head && a.tail == b.tail && a.relation_type == b.relation_type);

        implicit.sort_by_key(|m| m.start);
        implicit.dedup_by(|a, b| a.start == b.start && a.entity_id == b.entity_id);

        triples.sort_by_key(|t| t.start);
        triples.dedup_by(|a, b| a.start == b.start && a.source == b.source);

        temporal.sort_by_key(|t| t.start);
        temporal.dedup_by(|a, b| a.start == b.start && a.text == b.text);
        
        structured.sort_by_key(|s| s.subject_span.start);
        structured.dedup_by(|a, b| a.subject == b.subject && a.predicate == b.predicate && a.object == b.object);

        // Populate result
        result.stats.unified_found = unified_relations.len();
        result.stats.implicit_found = implicit.len();
        result.stats.triples_found = triples.len();
        result.stats.temporal_found = temporal.len();
        result.stats.structured_found = structured.len();
        result.unified_relations = unified_relations.clone();
        result.implicit = implicit.clone();
        result.triples = triples.clone();
        result.temporal = temporal.clone();
        result.structured = structured.clone();

        // Finalize
        result.stats.timings.total_us = overall_start.elapsed().as_micros() as u64;
        result.stats.was_skipped = false;

        // Update state
        self.incremental_state = Some(IncrementalState {
            chunks: incremental::chunk_text(text),
            extracted_items: ExtractedItems {
                relations: unified_relations,
                implicit,
                triples,
                temporal,
                structured,
            },
        });
        self.last_result = Some(result.clone());

        result
    }

    /// Full scan - extract everything from scratch
    fn scan_full(
        &mut self,
        text: &str,
        external_spans: &[EntitySpan],
        content_hash: u64,
    ) -> ScanResult {
        let overall_start = Instant::now();
        let mut result = ScanResult::default();
        result.stats.content_hash = format!("{:x}", content_hash);

        // CRITICAL: Chunk text ONCE and reuse (was chunking 2x before, 100-200ms wasted)
        let chunk_result = self.chunker.chunk(text);

        // Phase 1: Triple extraction
        let triple_start = Instant::now();
        result.triples = self.triple_cortex.extract(text);
        result.stats.timings.triple_us = triple_start.elapsed().as_micros() as u64;
        result.stats.triples_found = result.triples.len();

        // Phase 2: Implicit entity mentions
        let implicit_start = Instant::now();
        result.implicit = self.implicit_cortex.find_mentions(text);
        result.stats.timings.implicit_us = implicit_start.elapsed().as_micros() as u64;
        result.stats.implicit_found = result.implicit.len();

        // Phase 3: Build entity spans from implicit mentions
        let mut all_spans: Vec<EntitySpan> = result
            .implicit
            .iter()
            .map(|mention| EntitySpan {
                label: mention.entity_label.clone(),
                entity_id: Some(mention.entity_id.clone()),
                start: mention.start,
                end: mention.end,
                kind: Some(mention.entity_kind.clone()),
            })
            .collect();

        // Merge with external spans
        for ext in external_spans {
            let overlaps = all_spans.iter().any(|s| {
                (ext.start >= s.start && ext.start < s.end)
                    || (ext.end > s.start && ext.end <= s.end)
                    || (ext.start <= s.start && ext.end >= s.end)
            });
            if !overlaps {
                all_spans.push(ext.clone());
            }
        }

        // Phase 4: Temporal extraction
        let temporal_start = Instant::now();
        let temporal_result = self.temporal_cortex.scan(text);
        result.temporal = temporal_result.mentions;
        result.stats.timings.temporal_us = temporal_start.elapsed().as_micros() as u64;
        result.stats.temporal_found = result.temporal.len();

        // Phase 5: Structured relation extraction (uses pre-computed chunks!)
        let structured_start = Instant::now();
        result.structured = self.structured_extractor.extract_from_chunks(text, &all_spans, &chunk_result);
        result.stats.timings.structured_us = structured_start.elapsed().as_micros() as u64;
        result.stats.structured_found = result.structured.len();

        // Convert structured relations to triples
        for sr in &result.structured {
            if let Some(ref object) = sr.object {
                result.triples.push(ExtractedTriple {
                    source: sr.subject.clone(),
                    predicate: sr.relation_type.clone(),
                    target: object.clone(),
                    start: sr.subject_span.start,
                    end: sr
                        .object_span
                        .as_ref()
                        .map(|s| s.end)
                        .unwrap_or(sr.predicate_span.end),
                    raw_text: format!("{} {} {}", sr.subject, sr.predicate, object),
                    source_kind: None,
                    target_kind: None,
                });
            }
        }
        result.stats.triples_found = result.triples.len();

        // Phase 6: Unified relation extraction (CST + Graph inference) - uses pre-computed chunks!
        let unified_start = Instant::now();
        let existing_edges: Vec<(String, String, String)> = result
            .triples
            .iter()
            .map(|t| (t.source.clone(), t.target.clone(), t.predicate.clone()))
            .collect();

        let (unified_relations, _) =
            self.relation_engine.extract_with_chunks(text, &all_spans, &existing_edges, &chunk_result);
        result.unified_relations = unified_relations;
        result.stats.timings.unified_us = unified_start.elapsed().as_micros() as u64;
        result.stats.unified_found = result.unified_relations.len();

        // Finalize
        result.stats.timings.total_us = overall_start.elapsed().as_micros() as u64;
        result.stats.was_skipped = false;

        // Update incremental state
        self.incremental_state = Some(IncrementalState {
            chunks: incremental::chunk_text(text),
            extracted_items: ExtractedItems {
                relations: result.unified_relations.clone(),
                implicit: result.implicit.clone(),
                triples: result.triples.clone(),
                temporal: result.temporal.clone(),
                structured: result.structured.clone(),
            },
        });

        self.last_result = Some(result.clone());

        result
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn entity_def(id: &str, label: &str, kind: &str) -> EntityDefinition {
        EntityDefinition {
            id: id.to_string(),
            label: label.to_string(),
            kind: kind.to_string(),
            aliases: vec![],
        }
    }

    #[test]
    fn test_basic_scan_returns_result() {
        let mut cortex = DocumentCortex::new();
        let result = cortex.scan("Hello world", &[]);

        assert!(!result.stats.was_skipped);
        assert!(result.stats.timings.total_us > 0);
    }

    #[test]
    fn test_scan_extracts_triples() {
        let mut cortex = DocumentCortex::new();

        let text = "The relationship: [[Frodo->OWNS->Ring]]";
        let result = cortex.scan(text, &[]);

        assert_eq!(result.triples.len(), 1);
        assert_eq!(result.triples[0].source, "Frodo");
        assert_eq!(result.triples[0].predicate, "OWNS");
        assert_eq!(result.triples[0].target, "Ring");
    }

    #[test]
    fn test_scan_extracts_implicit_mentions() {
        let mut cortex = DocumentCortex::new();

        cortex
            .hydrate_entities(vec![
                entity_def("char_001", "Gandalf", "CHARACTER"),
                entity_def("char_002", "Frodo", "CHARACTER"),
            ])
            .unwrap();

        let text = "Gandalf gave the ring to Frodo";
        let result = cortex.scan(text, &[]);

        assert_eq!(result.implicit.len(), 2);
    }

    #[test]
    fn test_scan_skips_unchanged() {
        let mut cortex = DocumentCortex::new();

        let text = "Hello world";
        let result1 = cortex.scan(text, &[]);
        assert!(!result1.stats.was_skipped);

        let result2 = cortex.scan(text, &[]);
        assert!(result2.stats.was_skipped);
    }

    #[test]
    fn test_scan_detects_changes() {
        let mut cortex = DocumentCortex::new();

        cortex.scan("Hello world", &[]);
        let result2 = cortex.scan("Hello universe", &[]);

        assert!(!result2.stats.was_skipped);
    }

    #[test]
    fn test_reset_clears_cache() {
        let mut cortex = DocumentCortex::new();

        cortex.scan("Hello", &[]);
        cortex.reset();

        let result = cortex.scan("Hello", &[]);
        assert!(!result.stats.was_skipped);
    }
    
    // =========================================================================
    // Incremental Scan Integration Tests
    // =========================================================================
    
    #[test]
    fn test_incremental_scan_triggers_on_small_edit() {
        let mut cortex = DocumentCortex::new();
        
        // First: Create a multi-paragraph document (needs enough chunks for LCS to work)
        let original = "Paragraph one with stuff.\n\nParagraph two with content.\n\nParagraph three here.\n\nParagraph four at end.";
        let result1 = cortex.scan(original, &[]);
        assert!(!result1.stats.was_incremental, "First scan should be full");
        
        // Then: Edit only the last paragraph (should trigger incremental)
        let edited = "Paragraph one with stuff.\n\nParagraph two with content.\n\nParagraph three here.\n\nParagraph four at end. ADDED";
        let result2 = cortex.scan(edited, &[]);
        
        // This should be incremental because only 1 chunk changed out of 4
        assert!(result2.stats.was_incremental, "Small edit should trigger incremental scan");
    }
    
    #[test]
    fn test_incremental_falls_back_on_large_edit() {
        let mut cortex = DocumentCortex::new();
        
        let original = "AAA\n\nBBB\n\nCCC\n\nDDD";
        cortex.scan(original, &[]);
        
        // Complete rewrite should NOT use incremental (dirty ratio too high)
        let rewritten = "XXX\n\nYYY\n\nZZZ\n\nWWW";
        let result = cortex.scan(rewritten, &[]);
        
        assert!(!result.stats.was_incremental, "Full rewrite should not use incremental");
    }
    
    #[test]
    fn test_incremental_stats_accumulate() {
        let mut cortex = DocumentCortex::new();
        
        let base = "Para 1.\n\nPara 2.\n\nPara 3.\n\nPara 4.";
        cortex.scan(base, &[]);
        
        // Small edit 1
        cortex.scan("Para 1.\n\nPara 2.\n\nPara 3.\n\nPara 4. X", &[]);
        // Small edit 2  
        cortex.scan("Para 1.\n\nPara 2.\n\nPara 3.\n\nPara 4. XY", &[]);
        
        let stats = cortex.incremental_stats();
        assert!(stats.incremental_count >= 1, "Should have incremental scans");
        assert!(stats.avg_dirty_ratio > 0.0, "Should track dirty ratio");
    }
}

