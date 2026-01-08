//! Incremental Scanner: Chunk-based delta scanning (Native Tauri)
//!
//! Enables sub-20ms updates by only reprocessing changed regions.
//!
//! # Architecture
//! - **Paragraph-based chunking**: Split by `\n\n` with sentence fallback
//! - **LCS-based diffing**: Align chunks via longest-increasing-subsequence
//! - **O(log n) shifting**: Binary-search prefix sums for coordinate adjustment
//! - **Partial rescan**: Only run extractors on dirty regions + context padding
//!
//! # Improvements over kittcore version
//! - LCS alignment prevents "insert at top → everything dirty" cascade
//! - ShiftIndex for O(log n) shift queries instead of O(n)
//! - ahash for faster hashing (if available)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;

// =============================================================================
// Constants
// =============================================================================

/// Maximum chunk size before triggering sentence-level fallback
const MAX_CHUNK_SIZE: usize = 2000;

/// Dirty ratio threshold for incremental scanning (30%)
const DIRTY_RATIO_THRESHOLD: f64 = 0.30;

/// Maximum dirty chunks before falling back to full rescan
const MAX_DIRTY_CHUNKS: usize = 10;

/// Context padding: how many paragraphs to expand around dirty region
const CONTEXT_PADDING_PARAGRAPHS: usize = 1;

// =============================================================================
// Core Types
// =============================================================================

/// A chunk of text with position and hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Absolute start position in the source text
    pub start: usize,
    /// Absolute end position in the source text
    pub end: usize,
    /// Content hash
    pub hash: u64,
}

impl Chunk {
    /// Create a new chunk from text slice with position
    pub fn new(text: &str, start: usize) -> Self {
        Self {
            start,
            end: start + text.len(),
            hash: compute_hash(text),
        }
    }

    /// Length of this chunk
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if chunk is empty
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// A single change record for shift calculation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Change {
    /// Original end position (for shift calculation)
    pub old_end: usize,
    /// Net shift caused by this change
    pub shift: i64,
}

/// Pre-computed index for O(log n) shift queries
#[derive(Debug, Clone, Default)]
pub struct ShiftIndex {
    /// Sorted old_end positions
    ends: Vec<usize>,
    /// Cumulative shift up to ends[i]
    prefix_shift: Vec<i64>,
}

impl ShiftIndex {
    /// Build from changes (must be sorted by old_end)
    pub fn from_changes(changes: &[Change]) -> Self {
        if changes.is_empty() {
            return Self::default();
        }

        let mut sorted: Vec<_> = changes.to_vec();
        sorted.sort_by_key(|c| c.old_end);

        let mut ends = Vec::with_capacity(sorted.len());
        let mut prefix_shift = Vec::with_capacity(sorted.len());
        let mut cumulative = 0i64;

        for change in sorted {
            cumulative += change.shift;
            ends.push(change.old_end);
            prefix_shift.push(cumulative);
        }

        Self { ends, prefix_shift }
    }

    /// O(log n) shift lookup
    pub fn shift_at(&self, pos: usize) -> i64 {
        if self.ends.is_empty() {
            return 0;
        }
        // Find rightmost change where old_end <= pos
        let idx = self.ends.partition_point(|&e| e <= pos);
        if idx == 0 { 0 } else { self.prefix_shift[idx - 1] }
    }
}

/// Delta result from comparing old chunks to new text
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Delta {
    /// Ranges in the NEW text that need rescanning (sorted, non-overlapping)
    pub dirty_ranges: Vec<Range<usize>>,
    /// Individual change records
    pub changes: Vec<Change>,
    /// Total number of chunks in the new text
    pub total_chunks: usize,
    /// Number of dirty (unmatched) chunks
    pub dirty_chunks: usize,
}

impl Delta {
    /// Check if incremental scanning should be used
    pub fn should_use_incremental(&self) -> bool {
        if self.total_chunks == 0 {
            return false;
        }
        let dirty_ratio = self.dirty_chunks as f64 / self.total_chunks as f64;
        dirty_ratio < DIRTY_RATIO_THRESHOLD && self.dirty_chunks < MAX_DIRTY_CHUNKS
    }

    /// Build a ShiftIndex for O(log n) shift queries
    pub fn build_shift_index(&self) -> ShiftIndex {
        ShiftIndex::from_changes(&self.changes)
    }

    /// O(log n) check if a range overlaps any dirty range
    /// (dirty_ranges must be sorted and non-overlapping)
    pub fn overlaps_dirty(&self, start: usize, end: usize) -> bool {
        // Binary search for first range that could overlap
        let idx = self.dirty_ranges.partition_point(|r| r.end <= start);
        if idx < self.dirty_ranges.len() {
            let range = &self.dirty_ranges[idx];
            return start < range.end && end > range.start;
        }
        false
    }
}

/// Cached extracted items from a previous scan
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractedItems {
    pub relations: Vec<super::relation::UnifiedRelation>,
    pub implicit: Vec<super::implicit::ImplicitMention>,
    pub triples: Vec<super::triple::ExtractedTriple>,
    pub temporal: Vec<super::temporal::TemporalMention>,
    pub structured: Vec<super::structured_relation::StructuredRelation>,
}

/// Full incremental state persisted between scans
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncrementalState {
    /// Chunked representation of the last scanned text
    pub chunks: Vec<Chunk>,
    /// Cached extracted items
    pub extracted_items: ExtractedItems,
}

/// Statistics for incremental scanning
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncrementalStats {
    pub incremental_count: u64,
    pub full_rescan_count: u64,
    pub avg_dirty_ratio: f64,
}

// =============================================================================
// Trait for shiftable items
// =============================================================================

/// Trait for items that have start/end positions
pub trait HasSpan {
    fn start(&self) -> usize;
    fn end(&self) -> usize;
    fn set_start(&mut self, start: usize);
    fn set_end(&mut self, end: usize);
}

// Implement HasSpan for all extraction types
impl HasSpan for super::relation::UnifiedRelation {
    fn start(&self) -> usize {
        self.span.map(|(s, _)| s).unwrap_or(0)
    }
    fn end(&self) -> usize {
        self.span.map(|(_, e)| e).unwrap_or(0)
    }
    fn set_start(&mut self, start: usize) {
        if let Some((_, end)) = self.span {
            self.span = Some((start, end));
        }
    }
    fn set_end(&mut self, end: usize) {
        if let Some((start, _)) = self.span {
            self.span = Some((start, end));
        }
    }
}

impl HasSpan for super::implicit::ImplicitMention {
    fn start(&self) -> usize {
        self.start
    }
    fn end(&self) -> usize {
        self.end
    }
    fn set_start(&mut self, start: usize) {
        self.start = start;
    }
    fn set_end(&mut self, end: usize) {
        self.end = end;
    }
}

impl HasSpan for super::triple::ExtractedTriple {
    fn start(&self) -> usize {
        self.start
    }
    fn end(&self) -> usize {
        self.end
    }
    fn set_start(&mut self, start: usize) {
        self.start = start;
    }
    fn set_end(&mut self, end: usize) {
        self.end = end;
    }
}

impl HasSpan for super::temporal::TemporalMention {
    fn start(&self) -> usize {
        self.start
    }
    fn end(&self) -> usize {
        self.end
    }
    fn set_start(&mut self, start: usize) {
        self.start = start;
    }
    fn set_end(&mut self, end: usize) {
        self.end = end;
    }
}

impl HasSpan for super::structured_relation::StructuredRelation {
    fn start(&self) -> usize {
        self.subject_span.start
    }
    fn end(&self) -> usize {
        self.object_span.as_ref().map(|s| s.end)
            .unwrap_or(self.predicate_span.end)
    }
    fn set_start(&mut self, start: usize) {
        // Shift subject_span
        let delta = start as isize - self.subject_span.start as isize;
        self.subject_span.start = start;
        self.subject_span.end = (self.subject_span.end as isize + delta).max(0) as usize;
        // Shift predicate_span
        self.predicate_span.start = (self.predicate_span.start as isize + delta).max(0) as usize;
        self.predicate_span.end = (self.predicate_span.end as isize + delta).max(0) as usize;
        // Shift object_span if present
        if let Some(ref mut obj) = self.object_span {
            obj.start = (obj.start as isize + delta).max(0) as usize;
            obj.end = (obj.end as isize + delta).max(0) as usize;
        }
    }
    fn set_end(&mut self, _end: usize) {
        // End is derived from spans, shifting is done via set_start
    }
}

// =============================================================================
// Core Functions
// =============================================================================

/// Compute hash of text (using standard hasher for now)
fn compute_hash(text: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Split text by sentences (fallback for giant paragraphs)
fn split_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;
    
    for (i, c) in text.char_indices() {
        if matches!(c, '.' | '!' | '?') {
            let rest = &text[i + c.len_utf8()..];
            if rest.starts_with(char::is_whitespace) || rest.is_empty() {
                let sentence = &text[start..i + c.len_utf8()];
                if !sentence.trim().is_empty() {
                    sentences.push(sentence);
                }
                start = i + c.len_utf8();
                while start < text.len() && text[start..].starts_with(char::is_whitespace) {
                    start += text[start..].chars().next().map_or(1, |c| c.len_utf8());
                }
            }
        }
    }
    
    if start < text.len() && !text[start..].trim().is_empty() {
        sentences.push(&text[start..]);
    }
    
    if sentences.is_empty() && !text.trim().is_empty() {
        sentences.push(text);
    }
    
    sentences
}

/// Split text into chunks (paragraph-based with sentence fallback)
pub fn chunk_text(text: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut pos = 0;
    
    for part in text.split("\n\n") {
        if part.is_empty() {
            pos += 2;
            continue;
        }
        
        if part.len() <= MAX_CHUNK_SIZE {
            chunks.push(Chunk::new(part, pos));
        } else {
            // Giant blob: split by sentence
            let mut sent_pos = pos;
            for sentence in split_sentences(part) {
                chunks.push(Chunk::new(sentence, sent_pos));
                sent_pos += sentence.len();
                // Skip whitespace between sentences
                while sent_pos < pos + part.len() {
                    let remaining = &part[sent_pos - pos..];
                    if remaining.starts_with(char::is_whitespace) {
                        sent_pos += remaining.chars().next().map_or(1, |c| c.len_utf8());
                    } else {
                        break;
                    }
                }
            }
        }
        
        pos += part.len() + 2;
    }
    
    chunks
}

/// Compute LCS-based delta between old chunks and new text
/// 
/// Unlike the "lockstep walk" approach, this properly aligns chunks
/// so that insertions don't cascade into "everything dirty".
pub fn compute_delta(old_chunks: &[Chunk], new_text: &str) -> Delta {
    let new_chunks = chunk_text(new_text);
    let mut delta = Delta {
        total_chunks: new_chunks.len(),
        ..Default::default()
    };
    
    if old_chunks.is_empty() {
        if !new_text.is_empty() {
            delta.dirty_ranges.push(0..new_text.len());
            delta.dirty_chunks = new_chunks.len();
        }
        return delta;
    }

    if new_chunks.is_empty() {
        return delta;
    }

    // Build hash→positions index for old chunks
    // Use VecDeque so we can pop_front for INJECTIVE matching
    use std::collections::VecDeque;
    let mut old_positions: HashMap<(u64, usize), VecDeque<usize>> = HashMap::new();
    for (i, c) in old_chunks.iter().enumerate() {
        old_positions.entry((c.hash, c.len())).or_default().push_back(i);
    }

    // For each new chunk, take a UNIQUE old index if available (injective)
    let matches: Vec<Option<usize>> = new_chunks.iter()
        .map(|nc| {
            old_positions
                .get_mut(&(nc.hash, nc.len()))
                .and_then(|q| q.pop_front())  // Consume the match!
        })
        .collect();

    // Compute longest increasing subsequence of matched indices
    // This gives us the maximal set of chunks that preserved order
    let lis_indices = longest_increasing_subsequence(&matches);
    let lis_set: std::collections::HashSet<usize> = lis_indices.into_iter().collect();

    // Mark chunks NOT in LIS as dirty
    let mut dirty_new_indices = Vec::new();
    for (new_idx, _) in new_chunks.iter().enumerate() {
        if !lis_set.contains(&new_idx) {
            dirty_new_indices.push(new_idx);
        }
    }

    delta.dirty_chunks = dirty_new_indices.len();

    // Build dirty ranges with context padding
    for &new_idx in &dirty_new_indices {
        let new_chunk = &new_chunks[new_idx];
        
        let padded_start = if new_idx >= CONTEXT_PADDING_PARAGRAPHS {
            new_chunks.get(new_idx - CONTEXT_PADDING_PARAGRAPHS)
                .map(|c| c.start)
                .unwrap_or(new_chunk.start)
        } else {
            0
        };
        
        let padded_end = new_chunks.get(new_idx + CONTEXT_PADDING_PARAGRAPHS)
            .map(|c| c.end)
            .unwrap_or(new_chunk.end)
            .min(new_text.len());

        // Merge with last if overlapping
        if let Some(last) = delta.dirty_ranges.last_mut() {
            if padded_start <= last.end {
                last.end = last.end.max(padded_end);
                continue;
            }
        }
        delta.dirty_ranges.push(padded_start..padded_end);
    }

    // Compute changes for shift calculation
    // For each dirty new chunk, record its size difference vs corresponding old chunk
    for &new_idx in &dirty_new_indices {
        let new_chunk = &new_chunks[new_idx];
        
        // Find corresponding old chunk (by index, clamped)
        if let Some(old_chunk) = old_chunks.get(new_idx) {
            delta.changes.push(Change {
                old_end: old_chunk.end,
                shift: new_chunk.len() as i64 - old_chunk.len() as i64,
            });
        } else {
            // Appended chunk
            let old_end = old_chunks.last().map(|c| c.end).unwrap_or(0);
            delta.changes.push(Change {
                old_end,
                shift: new_chunk.len() as i64,
            });
        }
    }

    delta
}

/// Compute longest increasing subsequence of matched indices
fn longest_increasing_subsequence(matches: &[Option<usize>]) -> Vec<usize> {
    if matches.is_empty() {
        return Vec::new();
    }

    // Filter to only Some values with their new_idx
    let valid: Vec<(usize, usize)> = matches.iter()
        .enumerate()
        .filter_map(|(new_idx, opt)| opt.map(|old_idx| (new_idx, old_idx)))
        .collect();

    if valid.is_empty() {
        return Vec::new();
    }

    // Standard LIS algorithm on old_idx values
    let n = valid.len();
    let mut dp = vec![1usize; n];
    let mut prev = vec![None; n];

    for i in 1..n {
        for j in 0..i {
            if valid[j].1 < valid[i].1 && dp[j] + 1 > dp[i] {
                dp[i] = dp[j] + 1;
                prev[i] = Some(j);
            }
        }
    }

    // Find the end of longest sequence
    let (max_idx, _) = dp.iter().enumerate().max_by_key(|(_, &v)| v).unwrap();

    // Reconstruct path
    let mut path = Vec::new();
    let mut idx = Some(max_idx);
    while let Some(i) = idx {
        path.push(valid[i].0); // new_idx
        idx = prev[i];
    }
    path.reverse();
    path
}

/// Shift items based on delta changes (O(n log m) version)
pub fn shift_items<T: HasSpan>(items: &mut Vec<T>, delta: &Delta) {
    let shift_index = delta.build_shift_index();
    
    items.retain_mut(|item| {
        // O(log n) check if overlaps dirty
        if delta.overlaps_dirty(item.start(), item.end()) {
            return false;
        }
        
        // O(log m) shift lookup
        let shift = shift_index.shift_at(item.start());
        if shift != 0 {
            let new_start = (item.start() as i64 + shift).max(0) as usize;
            let new_end = (item.end() as i64 + shift).max(0) as usize;
            item.set_start(new_start);
            item.set_end(new_end);
        }
        
        true
    });
}

/// Extract text for dirty ranges
pub fn extract_dirty_text<'a>(text: &'a str, delta: &Delta) -> Vec<(Range<usize>, &'a str)> {
    delta.dirty_ranges
        .iter()
        .filter_map(|range| {
            let start = range.start.min(text.len());
            let end = range.end.min(text.len());
            if start < end {
                Some((start..end, &text[start..end]))
            } else {
                None
            }
        })
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_single_paragraph() {
        let text = "Hello world";
        let chunks = chunk_text(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 11);
    }
    
    #[test]
    fn test_chunk_text_multiple_paragraphs() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let chunks = chunk_text(text);
        assert_eq!(chunks.len(), 2);
    }
    
    #[test]
    fn test_compute_delta_no_change() {
        let text = "Hello world";
        let chunks = chunk_text(text);
        let delta = compute_delta(&chunks, text);
        
        assert!(delta.dirty_ranges.is_empty());
        assert_eq!(delta.dirty_chunks, 0);
    }
    
    #[test]
    fn test_compute_delta_append() {
        let old_text = "P1.\n\nP2.\n\nP3.\n\nP4.";
        let old_chunks = chunk_text(old_text);
        
        let new_text = "P1.\n\nP2.\n\nP3.\n\nP4. More.";
        let delta = compute_delta(&old_chunks, new_text);
        
        assert_eq!(delta.dirty_chunks, 1);
        assert!(delta.should_use_incremental());
    }
    
    #[test]
    fn test_compute_delta_insert_at_top() {
        // This is the KEY test - insert at top should NOT make everything dirty
        let old_text = "AAA\n\nBBB\n\nCCC\n\nDDD";
        let old_chunks = chunk_text(old_text);
        assert_eq!(old_chunks.len(), 4);
        
        // Insert new paragraph at the beginning
        let new_text = "NEW\n\nAAA\n\nBBB\n\nCCC\n\nDDD";
        let delta = compute_delta(&old_chunks, new_text);
        
        // With LCS alignment, only the NEW paragraph should be dirty
        // The original 4 paragraphs should be matched
        assert_eq!(delta.dirty_chunks, 1, 
            "Insert at top should only dirty 1 chunk, not cascade");
    }
    
    #[test]
    fn test_shift_index_binary_search() {
        let changes = vec![
            Change { old_end: 10, shift: 5 },
            Change { old_end: 20, shift: -3 },
        ];
        let index = ShiftIndex::from_changes(&changes);
        
        assert_eq!(index.shift_at(5), 0);   // Before first change
        assert_eq!(index.shift_at(10), 5);  // At first change
        assert_eq!(index.shift_at(15), 5);  // Between changes
        assert_eq!(index.shift_at(20), 2);  // At second: 5 + (-3) = 2
        assert_eq!(index.shift_at(25), 2);  // After all
    }
    
    #[test]
    fn test_overlaps_dirty_binary_search() {
        let delta = Delta {
            dirty_ranges: vec![10..20, 30..40],
            ..Default::default()
        };
        
        assert!(!delta.overlaps_dirty(0, 5));    // Before first
        assert!(delta.overlaps_dirty(15, 25));   // Overlaps first
        assert!(!delta.overlaps_dirty(22, 28));  // Gap between
        assert!(delta.overlaps_dirty(35, 45));   // Overlaps second
    }
    
    #[test]
    fn test_lis_empty() {
        let matches: Vec<Option<usize>> = vec![];
        assert!(longest_increasing_subsequence(&matches).is_empty());
    }
    
    #[test]
    fn test_lis_all_none() {
        let matches = vec![None, None, None];
        assert!(longest_increasing_subsequence(&matches).is_empty());
    }
    
    #[test]
    fn test_lis_simple() {
        let matches = vec![Some(0), Some(1), Some(2)];
        let lis = longest_increasing_subsequence(&matches);
        assert_eq!(lis, vec![0, 1, 2]);
    }
    
    #[test]
    fn test_lis_with_gaps() {
        // new_idx: 0    1    2    3    4
        // old_idx: 1    0    2    None 3
        // LIS should be indices 0, 2, 4 (old values 1, 2, 3)
        let matches = vec![Some(1), Some(0), Some(2), None, Some(3)];
        let lis = longest_increasing_subsequence(&matches);
        assert!(lis.len() >= 3);
    }
}
