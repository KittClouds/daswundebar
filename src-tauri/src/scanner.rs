//! UnifiedScanner - Native Tauri Implementation
//! 
//! This is a direct port of the kittcore UnifiedScanner without WASM dependencies.
//! The core algorithm is identical - regex-based pattern matching with overlap resolution.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use regex::Regex;

// =============================================================================
// CONTRACT TYPES
// =============================================================================

/// Kind of reference detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RefKind {
    Entity = 0,
    Wikilink = 1,
    Backlink = 2,
    Tag = 3,
    Mention = 4,
    Triple = 5,
    InlineRelation = 6,
    Temporal = 7,
    Implicit = 8,
    Relation = 9,
}

/// Styling hints for TS decoration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StylingHint {
    pub color_key: String,
    pub confidence: f64,
    pub widget_mode: bool,
    pub is_editing: bool,
}

/// A decoration span for TS to render
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecorationSpan {
    pub kind: RefKind,
    pub start: usize,
    pub end: usize,
    pub label: String,
    pub raw_text: String,
    pub captures: HashMap<String, String>,
    pub styling: StylingHint,
}

/// Full scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedScanResult {
    pub spans: Vec<DecorationSpan>,
    pub stats: UnifiedScanStats,
}

/// Statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnifiedScanStats {
    pub entity_count: usize,
    pub wikilink_count: usize,
    pub backlink_count: usize,
    pub tag_count: usize,
    pub mention_count: usize,
    pub triple_count: usize,
    pub temporal_count: usize,
    pub implicit_count: usize,
    pub relation_count: usize,
    pub total_spans: usize,
    pub scan_time_us: u64,
}

// =============================================================================
// IMPLEMENTATION
// =============================================================================

/// Pattern definition with compiled regex and metadata
struct PatternDef {
    regex: Regex,
    kind: RefKind,
    priority: u8,
    capture_names: Vec<(&'static str, usize)>,
    styling: StylingHint,
}

/// Internal struct for sorting before overlap resolution
struct RawMatch {
    kind: RefKind,
    priority: u8,
    start: usize,
    end: usize,
    label: String,
    raw_text: String,
    captures: HashMap<String, String>,
    styling: StylingHint,
}

/// Unified document scanner - combines all pattern detection
pub struct UnifiedScanner {
    patterns: Vec<PatternDef>,
}

impl UnifiedScanner {
    pub fn new() -> Self {
        let patterns = Self::build_patterns();
        Self { patterns }
    }
    
    fn build_patterns() -> Vec<PatternDef> {
        vec![
            // Priority 110: Inline Relationship
            PatternDef {
                regex: Regex::new(r"\[([A-Z_]+)(?::([A-Z_]+))?\|([^\]\->]+)->([A-Z_]+)->([^\]]+)\]").unwrap(),
                kind: RefKind::Triple,
                priority: 110,
                capture_names: vec![
                    ("subjectKind", 1),
                    ("subjectSubtype", 2),
                    ("subjectLabel", 3),
                    ("predicate", 4),
                    ("objectLabel", 5),
                ],
                styling: StylingHint {
                    color_key: "triple".to_string(),
                    confidence: 1.0,
                    widget_mode: true,
                    is_editing: false,
                },
            },
            
            // Priority 105: Full Triple (arrow syntax)
            PatternDef {
                regex: Regex::new(r"\[([A-Z_]+)(?::([A-Z_]+))?\|([^\]]+)\]\s*->([A-Z_]+)->\s*\[([A-Z_]+)(?::([A-Z_]+))?\|([^\]]+)\]").unwrap(),
                kind: RefKind::Triple,
                priority: 105,
                capture_names: vec![
                    ("subjectKind", 1),
                    ("subjectSubtype", 2),
                    ("subjectLabel", 3),
                    ("predicate", 4),
                    ("objectKind", 5),
                    ("objectSubtype", 6),
                    ("objectLabel", 7),
                ],
                styling: StylingHint {
                    color_key: "triple".to_string(),
                    confidence: 1.0,
                    widget_mode: true,
                    is_editing: false,
                },
            },
            
            // Priority 103: Parenthesized Triple
            PatternDef {
                regex: Regex::new(r"\[([A-Z_]+)(?::([A-Z_]+))?\|([^\]]+)\]\s*\(([A-Z_]+)\)\s*\[([A-Z_]+)(?::([A-Z_]+))?\|([^\]]+)\]").unwrap(),
                kind: RefKind::Triple,
                priority: 103,
                capture_names: vec![
                    ("subjectKind", 1),
                    ("subjectSubtype", 2),
                    ("subjectLabel", 3),
                    ("predicate", 4),
                    ("objectKind", 5),
                    ("objectSubtype", 6),
                    ("objectLabel", 7),
                ],
                styling: StylingHint {
                    color_key: "triple".to_string(),
                    confidence: 1.0,
                    widget_mode: true,
                    is_editing: false,
                },
            },
            
            // Priority 100: Entity
            PatternDef {
                regex: Regex::new(r"\[([A-Z_]+)(?::([A-Z_]+))?\|([^\]\|]+)(?:\|(\{[^}]+\}))?\]").unwrap(),
                kind: RefKind::Entity,
                priority: 100,
                capture_names: vec![
                    ("entityKind", 1),
                    ("subtype", 2),
                    ("label", 3),
                    ("attributes", 4),
                ],
                styling: StylingHint {
                    color_key: "entity".to_string(),
                    confidence: 1.0,
                    widget_mode: true,
                    is_editing: false,
                },
            },
            
            // Priority 90: Wikilink
            PatternDef {
                regex: Regex::new(r"\[\[([^\]\|]+)(?:\|([^\]]+))?\]\]").unwrap(),
                kind: RefKind::Wikilink,
                priority: 90,
                capture_names: vec![
                    ("target", 1),
                    ("displayText", 2),
                ],
                styling: StylingHint {
                    color_key: "wikilink".to_string(),
                    confidence: 1.0,
                    widget_mode: true,
                    is_editing: false,
                },
            },
            
            // Priority 85: Backlink
            PatternDef {
                regex: Regex::new(r"<<([^>\|]+)(?:\|([^>]+))?>>").unwrap(),
                kind: RefKind::Backlink,
                priority: 85,
                capture_names: vec![
                    ("target", 1),
                    ("displayText", 2),
                ],
                styling: StylingHint {
                    color_key: "backlink".to_string(),
                    confidence: 1.0,
                    widget_mode: true,
                    is_editing: false,
                },
            },
            
            // Priority 70: Tag
            PatternDef {
                regex: Regex::new(r"(?:^|[^&])#([\w\-]+)").unwrap(),
                kind: RefKind::Tag,
                priority: 70,
                capture_names: vec![
                    ("tagName", 1),
                ],
                styling: StylingHint {
                    color_key: "tag".to_string(),
                    confidence: 1.0,
                    widget_mode: false,
                    is_editing: false,
                },
            },
            
            // Priority 70: Mention
            PatternDef {
                regex: Regex::new(r"@([\w\-]+)").unwrap(),
                kind: RefKind::Mention,
                priority: 70,
                capture_names: vec![
                    ("username", 1),
                ],
                styling: StylingHint {
                    color_key: "mention".to_string(),
                    confidence: 1.0,
                    widget_mode: false,
                    is_editing: false,
                },
            },
        ]
    }
    
    pub fn scan(&self, text: &str) -> UnifiedScanResult {
        let start_time = std::time::Instant::now();
        
        // Collect all matches from all patterns
        let mut all_matches: Vec<RawMatch> = Vec::new();
        
        for pattern in &self.patterns {
            for cap in pattern.regex.captures_iter(text) {
                let full_match = cap.get(0).unwrap();
                
                // For tags, adjust the start if there's a preceding char
                let (actual_start, actual_end) = if pattern.kind == RefKind::Tag {
                    let raw_start = full_match.start();
                    let raw_text = full_match.as_str();
                    if let Some(offset) = raw_text.find('#') {
                        (raw_start + offset, full_match.end())
                    } else {
                        (raw_start, full_match.end())
                    }
                } else {
                    (full_match.start(), full_match.end())
                };
                
                // Validate bounds
                if actual_start > text.len() || actual_end > text.len() || actual_start > actual_end {
                    continue;
                }
                
                // Safe string slicing
                let raw_text = match text.get(actual_start..actual_end) {
                    Some(slice) => slice.to_string(),
                    None => continue,
                };
                
                // Extract captures
                let mut captures = HashMap::new();
                for (name, idx) in &pattern.capture_names {
                    if let Some(m) = cap.get(*idx) {
                        captures.insert(name.to_string(), m.as_str().trim().to_string());
                    }
                }
                
                // Determine label based on kind
                let label = match pattern.kind {
                    RefKind::Wikilink | RefKind::Backlink => {
                        captures.get("displayText")
                            .or_else(|| captures.get("target"))
                            .cloned()
                            .unwrap_or_default()
                    }
                    RefKind::Entity => {
                        captures.get("label").cloned().unwrap_or_default()
                    }
                    RefKind::Tag => {
                        captures.get("tagName").cloned().unwrap_or_default()
                    }
                    RefKind::Mention => {
                        captures.get("username").cloned().unwrap_or_default()
                    }
                    RefKind::Triple => {
                        let subj = captures.get("subjectLabel").cloned().unwrap_or_default();
                        let pred = captures.get("predicate").cloned().unwrap_or_default();
                        let obj = captures.get("objectLabel").cloned().unwrap_or_default();
                        format!("{} →{}→ {}", subj, pred, obj)
                    }
                    _ => String::new(),
                };
                
                // Build styling with entity kind awareness
                let mut styling = pattern.styling.clone();
                if pattern.kind == RefKind::Entity {
                    if let Some(kind) = captures.get("entityKind") {
                        styling.color_key = format!("entity-{}", kind.to_lowercase());
                    }
                }
                
                all_matches.push(RawMatch {
                    kind: pattern.kind,
                    priority: pattern.priority,
                    start: actual_start,
                    end: actual_end,
                    label,
                    raw_text,
                    captures,
                    styling,
                });
            }
        }
        
        // Sort by priority (desc), then position (asc), then length (desc)
        all_matches.sort_by(|a, b| {
            if a.priority != b.priority {
                return b.priority.cmp(&a.priority);
            }
            if a.start != b.start {
                return a.start.cmp(&b.start);
            }
            b.end.cmp(&a.end)
        });
        
        // Resolve overlaps: higher priority wins
        let mut covered = vec![false; text.len() + 1];
        let mut result_spans: Vec<DecorationSpan> = Vec::new();
        
        for m in all_matches {
            if m.start >= text.len() || m.end > text.len() {
                continue;
            }
            
            // Check if any position is already covered
            let mut overlaps = false;
            for i in m.start..m.end {
                if i < covered.len() && covered[i] {
                    overlaps = true;
                    break;
                }
            }
            
            if !overlaps {
                // Mark as covered
                for i in m.start..m.end {
                    if i < covered.len() {
                        covered[i] = true;
                    }
                }
                
                result_spans.push(DecorationSpan {
                    kind: m.kind,
                    start: m.start,
                    end: m.end,
                    label: m.label,
                    raw_text: m.raw_text,
                    captures: m.captures,
                    styling: m.styling,
                });
            }
        }
        
        // Sort by position for output
        result_spans.sort_by_key(|s| s.start);
        
        // Build stats
        let mut stats = UnifiedScanStats::default();
        for span in &result_spans {
            match span.kind {
                RefKind::Entity => stats.entity_count += 1,
                RefKind::Wikilink => stats.wikilink_count += 1,
                RefKind::Backlink => stats.backlink_count += 1,
                RefKind::Tag => stats.tag_count += 1,
                RefKind::Mention => stats.mention_count += 1,
                RefKind::Triple => stats.triple_count += 1,
                RefKind::Temporal => stats.temporal_count += 1,
                RefKind::Implicit => stats.implicit_count += 1,
                RefKind::Relation | RefKind::InlineRelation => stats.relation_count += 1,
            }
        }
        stats.total_spans = result_spans.len();
        stats.scan_time_us = start_time.elapsed().as_micros() as u64;
        
        UnifiedScanResult {
            spans: result_spans,
            stats,
        }
    }
}

impl Default for UnifiedScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_entity() {
        let scanner = UnifiedScanner::new();
        let result = scanner.scan("[CHARACTER|Jon Snow]");
        
        assert_eq!(result.spans.len(), 1);
        assert_eq!(result.spans[0].kind, RefKind::Entity);
        assert_eq!(result.spans[0].label, "Jon Snow");
    }

    #[test]
    fn test_wikilink() {
        let scanner = UnifiedScanner::new();
        let result = scanner.scan("Visit [[Winterfell]] today");
        
        assert_eq!(result.spans.len(), 1);
        assert_eq!(result.spans[0].kind, RefKind::Wikilink);
        assert_eq!(result.spans[0].label, "Winterfell");
    }

    #[test]
    fn test_triple() {
        let scanner = UnifiedScanner::new();
        let result = scanner.scan("[CHARACTER|Jon] (LOVES) [CHARACTER|Ygritte]");
        
        assert_eq!(result.spans.len(), 1);
        assert_eq!(result.spans[0].kind, RefKind::Triple);
    }

    #[test]
    fn test_overlap_resolution() {
        let scanner = UnifiedScanner::new();
        // Triple syntax should win over individual entities
        let result = scanner.scan("[CHARACTER|Jon] ->KNOWS-> [CHARACTER|Sam]");
        
        assert_eq!(result.spans.len(), 1);
        assert_eq!(result.spans[0].kind, RefKind::Triple);
    }
}
