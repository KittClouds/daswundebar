/**
 * RustScanner - TypeScript bridge to Rust Scanner
 * 
 * This module now uses Tauri IPC instead of WASM for native desktop builds.
 * All heavy lifting happens in the Rust backend via Tauri commands.
 * 
 * @module scanner/bridge
 */

// =============================================================================
// RE-EXPORTS FROM ADAPTER
// =============================================================================

// Re-export the unified adapter which handles Tauri/fallback automatically
export {
    ScannerBridge as ConductorBridge,
    scannerBridge as conductorBridge,
    ScannerBridge,
    scannerBridge,
} from './adapter';

export type {
    ScannerBridgeConfig as RustScannerConfig,
    ScannerBridgeConfig,
} from './adapter';

// =============================================================================
// Types (for backwards compatibility)
// =============================================================================

/** Entity definition for hydration (matches Rust EntityDefinition) */
export interface EntityDefinition {
    id: string;
    label: string;
    kind: string;
    aliases: string[];
}

/** Entity span for relationship extraction (matches Rust EntitySpan) */
export interface EntitySpan {
    label: string;
    start: number;
    end: number;
    kind?: string;
    entity_id?: string;
}

/** Timing statistics from scan */
export interface ScanTimings {
    total_us: number;
    syntax_us: number;
    relation_us: number;
    temporal_us: number;
    implicit_us: number;
    triple_us: number;
}

/** Aggregate scan statistics */
export interface ScanStats {
    timings: ScanTimings;
    content_hash: number;
    was_skipped: boolean;
    entities_found: number;
    relations_found: number;
    temporal_found: number;
    implicit_found: number;
    triples_found: number;
}

/** Extracted relation from Rust */
export interface ExtractedRelation {
    head_entity: string;
    head_start: number;
    head_end: number;
    tail_entity: string;
    tail_start: number;
    tail_end: number;
    relation_type: string;
    pattern_matched: string;
    pattern_start: number;
    pattern_end: number;
    confidence: number;
}

/** Extracted triple from Rust */
export interface ExtractedTriple {
    source: string;
    predicate: string;
    target: string;
    start: number;
    end: number;
    raw_text: string;
}

/** Implicit entity mention from Rust */
export interface ImplicitMention {
    entity_id: string;
    entity_label: string;
    entity_kind: string;
    matched_text: string;
    start: number;
    end: number;
    is_alias_match: boolean;
}

/** Unified scan result from Rust */
export interface ScanResult {
    relations: ExtractedRelation[];
    implicit: ImplicitMention[];
    triples: ExtractedTriple[];
    stats: ScanStats;
    errors: Array<{ phase: string; message: string }>;
    temporal?: TemporalMention[];
}

/** Temporal metadata (matches Rust TemporalMetadata) */
export interface TemporalMetadata {
    weekday_index?: number;
    month_index?: number;
    narrative_number?: number;
    direction?: string;
    era_year?: number;
    era_name?: string;
}

/** Temporal mention (matches Rust TemporalMention) */
export interface TemporalMention {
    kind: string;
    text: string;
    start: number;
    end: number;
    confidence: number;
    metadata?: TemporalMetadata;
}

/** Calendar dictionary for hydration */
export interface CalendarDictionary {
    months: string[];
    weekdays: string[];
    eras: string[];
}
