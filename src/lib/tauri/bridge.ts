/**
 * Tauri Bridge - Native IPC replacement for WASM Scanner
 * 
 * This module provides the same API as the WASM scanner but uses Tauri's
 * native IPC (`invoke()`) for communication with the Rust backend.
 * 
 * Benefits over WASM:
 * - Multi-threaded Rust (no single-threaded WASM constraint)
 * - Native file system access
 * - True async operations
 * - Better memory management
 * 
 * @module lib/tauri/bridge
 */

import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Types (same as WASM bridge)
// =============================================================================

/** Entity definition for hydration */
export interface EntityDefinition {
    id: string;
    label: string;
    kind: string;
    aliases: string[];
}

/** Entity span for relationship extraction */
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
    relation_us: number;
    temporal_us: number;
    implicit_us: number;
    structured_us: number;
    triple_us: number;
    unified_us: number;
}

/** Aggregate scan statistics */
export interface ScanStats {
    timings: ScanTimings;
    content_hash: string;
    was_skipped: boolean;
    was_incremental: boolean;
    entities_found: number;
    temporal_found: number;
    implicit_found: number;
    triples_found: number;
    structured_found: number;
    unified_found: number;
}

/** Extracted relation */
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

/** Extracted triple */
export interface ExtractedTriple {
    source: string;
    predicate: string;
    target: string;
    start: number;
    end: number;
    raw_text: string;
}

/** Implicit entity mention */
export interface ImplicitMention {
    entity_id: string;
    entity_label: string;
    entity_kind: string;
    matched_text: string;
    start: number;
    end: number;
    is_alias_match: boolean;
}

/** Unified scan result */
export interface ScanResult {
    relations: ExtractedRelation[];
    implicit: ImplicitMention[];
    triples: ExtractedTriple[];
    stats: ScanStats;
    errors: Array<{ phase: string; message: string }>;
    temporal?: TemporalMention[];
}

/** Temporal mention */
export interface TemporalMention {
    kind: string;
    text: string;
    start: number;
    end: number;
    confidence: number;
    metadata?: TemporalMetadata;
}

export interface TemporalMetadata {
    weekday_index?: number;
    month_index?: number;
    narrative_number?: number;
    direction?: string;
    era_year?: number;
    era_name?: string;
}

/** Calendar dictionary for hydration */
export interface CalendarDictionary {
    months: string[];
    weekdays: string[];
    eras: string[];
}

// =============================================================================
// Decoration Types (for highlighter)
// =============================================================================

export type RefKind =
    | 'Entity'
    | 'Wikilink'
    | 'Backlink'
    | 'Tag'
    | 'Mention'
    | 'Triple'
    | 'InlineRelation'
    | 'Temporal'
    | 'Implicit'
    | 'Relation';

export interface StylingHint {
    color_key: string;
    confidence: number;
    widget_mode: boolean;
    is_editing: boolean;
}

export interface DecorationSpan {
    kind: RefKind;
    start: number;
    end: number;
    label: string;
    raw_text: string;
    captures: Map<string, string> | Record<string, string>;
    styling: StylingHint;
}

export interface UnifiedScanResult {
    spans: DecorationSpan[];
    stats: UnifiedScanStats;
}

export interface UnifiedScanStats {
    entity_count: number;
    wikilink_count: number;
    backlink_count: number;
    tag_count: number;
    mention_count: number;
    triple_count: number;
    temporal_count: number;
    implicit_count: number;
    relation_count: number;
    total_spans: number;
    scan_time_us: number;
}

// =============================================================================
// Tauri Command Wrappers
// =============================================================================

/**
 * Check if running inside Tauri
 * 
 * In Tauri v2, the __TAURI__ global may not be exposed immediately.
 * We also check __TAURI_INTERNALS__ which is the internal IPC mechanism.
 */
export function isTauri(): boolean {
    if (typeof window === 'undefined') return false;

    // Tauri v1 style
    if ('__TAURI__' in window) return true;

    // Tauri v2 internals (IPC mechanism)
    if ('__TAURI_INTERNALS__' in window) return true;

    return false;
}

/** Scanner mode type */
export type ScannerMode = 'tauri' | 'fallback';

/**
 * Get current scanner mode
 */
export function getScannerMode(): ScannerMode {
    return isTauri() ? 'tauri' : 'fallback';
}

/**
 * Greet (test IPC connectivity)
 */
export async function greet(name: string): Promise<string> {
    return invoke<string>('greet', { name });
}

/**
 * Get version info
 */
export async function version(): Promise<string> {
    return invoke<string>('version');
}

/**
 * Unified document scan
 */
export async function unifiedScan(
    text: string,
    entities: EntitySpan[]
): Promise<UnifiedScanResult> {
    const resultJson = await invoke<string>('unified_scan', {
        text,
        entitiesJson: JSON.stringify(entities),
    });
    return JSON.parse(resultJson);
}

/**
 * Hydrate entities for implicit matching
 */
export async function hydrateEntities(entities: EntityDefinition[]): Promise<string> {
    return invoke<string>('hydrate_entities', {
        entitiesJson: JSON.stringify(entities),
    });
}

/**
 * Extract triples from text
 */
export async function extractTriples(text: string): Promise<ExtractedTriple[]> {
    const resultJson = await invoke<string>('extract_triples', { text });
    return JSON.parse(resultJson);
}

/**
 * Scan for temporal expressions
 */
export async function scanTemporal(text: string): Promise<TemporalMention[]> {
    const resultJson = await invoke<string>('scan_temporal', { text });
    return JSON.parse(resultJson);
}

/**
 * Scan document syntax (wikilinks, tags, entities)
 */
export async function scanSyntax(text: string): Promise<unknown[]> {
    const resultJson = await invoke<string>('scan_syntax', { text });
    return JSON.parse(resultJson);
}

/**
 * Index document for ResoRank search
 */
export async function resorankIndex(docId: string, content: string): Promise<string> {
    return invoke<string>('resorank_index', { docId, content });
}

/**
 * Search documents using ResoRank
 */
export async function resorankSearch(query: string, limit: number): Promise<unknown[]> {
    const resultJson = await invoke<string>('resorank_search', { query, limit });
    return JSON.parse(resultJson);
}

// =============================================================================
// ScanConductor API (New Unified Scanner)
// =============================================================================

/** Full document scan result from ScanConductor */
export interface ConductorScanResult {
    implicit: ImplicitMention[];
    triples: ExtractedTriple[];
    structured: StructuredRelation[];
    temporal: TemporalMention[];
    unified_relations: UnifiedRelation[];
    stats: ConductorStats;
}

/** Structured relation from SVO extraction */
export interface StructuredRelation {
    subject: string;
    subject_span: { start: number; end: number };
    predicate: string;
    predicate_span: { start: number; end: number };
    object?: string;
    object_span?: { start: number; end: number };
    relation_type: string;
    confidence: number;
    modifiers: RelationModifier[];
}

export interface RelationModifier {
    kind: string;
    text: string;
    span: { start: number; end: number };
}

/** Unified relation from CST + Graph inference */
export interface UnifiedRelation {
    head: string;
    tail: string;
    relation_type: string;
    confidence: number;
    source: string;
    span?: [number, number];
    head_kind?: string;
    tail_kind?: string;
}

/** Stats from ScanConductor */
export interface ConductorStats {
    timings: {
        total_us: number;
        relation_us: number;
        temporal_us: number;
        implicit_us: number;
        structured_us: number;
        triple_us: number;
        unified_us: number;
    };
    content_hash: string;
    was_skipped: boolean;
    was_incremental: boolean;
    entities_found: number;
    temporal_found: number;
    implicit_found: number;
    triples_found: number;
    structured_found: number;
    unified_found: number;
}

/** Conductor status */
export interface ConductorStatus {
    state: 'uninitialized' | 'initialized' | 'ready';
    entity_count: number;
    incremental_scans: number;
    full_rescans: number;
    avg_dirty_ratio: number;
}

/**
 * Hydrate ScanConductor with entity definitions
 */
export async function conductorHydrate(entities: EntityDefinition[]): Promise<{ hydrated: number }> {
    const resultJson = await invoke<string>('conductor_hydrate', {
        entitiesJson: JSON.stringify(entities),
    });
    return JSON.parse(resultJson);
}

/**
 * Full document scan using ScanConductor
 * Requires conductor to be hydrated first
 */
export async function conductorScan(
    text: string,
    entitySpans: EntitySpan[] = []
): Promise<ConductorScanResult> {
    const resultJson = await invoke<string>('conductor_scan', {
        text,
        entitiesJson: JSON.stringify(entitySpans),
    });
    return JSON.parse(resultJson);
}

/**
 * Force scan even if conductor not ready (for debugging)
 */
export async function conductorScanForce(
    text: string,
    entitySpans: EntitySpan[] = []
): Promise<ConductorScanResult> {
    const resultJson = await invoke<string>('conductor_scan_force', {
        text,
        entitiesJson: JSON.stringify(entitySpans),
    });
    return JSON.parse(resultJson);
}

/**
 * Get conductor status
 */
export async function conductorStatus(): Promise<ConductorStatus> {
    const resultJson = await invoke<string>('conductor_status');
    return JSON.parse(resultJson);
}

/**
 * Reset conductor state
 */
export async function conductorReset(): Promise<{ reset: boolean }> {
    const resultJson = await invoke<string>('conductor_reset');
    return JSON.parse(resultJson);
}

// =============================================================================
// TauriScanner - Drop-in replacement for ConductorBridge
// =============================================================================

export interface TauriScannerConfig {
    debounceMs: number;
    logPerformance: boolean;
    slowScanThresholdMs: number;
}

const DEFAULT_CONFIG: TauriScannerConfig = {
    debounceMs: 300,
    logPerformance: true,
    slowScanThresholdMs: 50,
};

/**
 * TauriScanner - Native replacement for ConductorBridge
 * 
 * Provides the same API as ConductorBridge but uses Tauri IPC instead of WASM.
 */
export class TauriScanner {
    private ready = false;
    private readyCallbacks: Array<() => void> = [];
    private debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();
    private resultHandlers: Array<(noteId: string, result: ScanResult) => void> = [];
    private config: TauriScannerConfig;

    constructor(config: Partial<TauriScannerConfig> = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
    }

    /**
     * Initialize the Tauri bridge
     */
    async initialize(): Promise<void> {
        if (!isTauri()) {
            console.warn('[TauriScanner] Not running in Tauri, falling back to stub mode');
            return;
        }

        try {
            const versionStr = await version();
            console.log(`[TauriScanner] Connected to backend: ${versionStr}`);
        } catch (error) {
            console.error('[TauriScanner] Failed to connect to backend:', error);
            throw error;
        }
    }

    /**
     * Check if fully ready
     */
    isReady(): boolean {
        return this.ready;
    }

    /**
     * Get current state name
     */
    getStateName(): string {
        return this.ready ? 'ready' : 'not-ready';
    }

    /**
     * Subscribe to ready event
     */
    onReady(callback: () => void): () => void {
        if (this.ready) {
            callback();
        } else {
            this.readyCallbacks.push(callback);
        }
        return () => {
            const idx = this.readyCallbacks.indexOf(callback);
            if (idx >= 0) this.readyCallbacks.splice(idx, 1);
        };
    }

    /**
     * Hydrate entities (both legacy and ScanConductor)
     */
    async hydrateEntities(entities: EntityDefinition[]): Promise<void> {
        if (!isTauri()) {
            console.warn('[TauriScanner] Not in Tauri, skipping hydration');
            this.ready = true;
            this.fireReadyCallbacks();
            return;
        }

        try {
            // Hydrate both the legacy ImplicitCortex and the new ScanConductor
            const [legacyResult, conductorResult] = await Promise.all([
                hydrateEntities(entities),
                conductorHydrate(entities),
            ]);

            console.log(`[TauriScanner] Legacy: ${legacyResult}, Conductor: hydrated ${conductorResult.hydrated} entities`);

            if (!this.ready) {
                this.ready = true;
                this.fireReadyCallbacks();
            }
        } catch (error) {
            console.error('[TauriScanner] Hydration failed:', error);
            throw error;
        }
    }

    private fireReadyCallbacks(): void {
        for (const cb of this.readyCallbacks) {
            try { cb(); } catch (e) { console.warn('[TauriScanner] Ready callback error:', e); }
        }
        this.readyCallbacks = [];
    }

    /**
     * Hydrate calendar patterns
     */
    async hydrateCalendar(_dictionary: CalendarDictionary): Promise<void> {
        // TODO: Implement when Rust backend supports calendar hydration
        console.log('[TauriScanner] Calendar hydration not yet implemented');
    }

    /**
     * Register result handler
     */
    onResult(handler: (noteId: string, result: ScanResult) => void): () => void {
        this.resultHandlers.push(handler);
        return () => {
            const idx = this.resultHandlers.indexOf(handler);
            if (idx >= 0) this.resultHandlers.splice(idx, 1);
        };
    }

    /**
     * Scan (debounced) - Uses conductor pipeline
     */
    scan(noteId: string, text: string, _entitySpans: EntitySpan[] = []): void {
        if (!this.ready) {
            console.log('[TauriScanner] Scan skipped - not ready');
            return;
        }

        const existing = this.debounceTimers.get(noteId);
        if (existing) clearTimeout(existing);

        const timer = setTimeout(() => {
            // Use unified conductor pipeline (fires onResult handlers)
            this.conductorScanImmediate(noteId, text, []);
            this.debounceTimers.delete(noteId);
        }, this.config.debounceMs);

        this.debounceTimers.set(noteId, timer);
    }

    /**
     * Immediate scan (bypasses debounce)
     */
    async scanImmediate(noteId: string, text: string, entitySpans: EntitySpan[] = []): Promise<ScanResult | null> {
        return this.executeScan(noteId, text, entitySpans);
    }

    private async executeScan(noteId: string, text: string, entitySpans: EntitySpan[]): Promise<ScanResult | null> {
        if (!isTauri()) {
            // Stub mode - return empty result
            return {
                relations: [],
                implicit: [],
                triples: [],
                stats: {
                    timings: { total_us: 0, syntax_us: 0, relation_us: 0, temporal_us: 0, implicit_us: 0, triple_us: 0 },
                    content_hash: 0,
                    was_skipped: true,
                    entities_found: 0,
                    relations_found: 0,
                    temporal_found: 0,
                    implicit_found: 0,
                    triples_found: 0,
                },
                errors: [],
            };
        }

        try {
            const startTime = performance.now();

            // Call Tauri backend
            const resultJson = await invoke<string>('unified_scan', {
                text,
                entitiesJson: JSON.stringify(entitySpans),
            });

            const result = JSON.parse(resultJson) as ScanResult;
            const endTime = performance.now();

            if (this.config.logPerformance) {
                const totalMs = endTime - startTime;
                if (totalMs > this.config.slowScanThresholdMs) {
                    console.warn(`[TauriScanner] Slow scan: ${totalMs.toFixed(1)}ms`);
                }
            }

            for (const handler of this.resultHandlers) {
                try { handler(noteId, result); }
                catch (e) { console.error('[TauriScanner] Handler error:', e); }
            }

            return result;
        } catch (error) {
            console.error('[TauriScanner] Scan error:', error);
            return null;
        }
    }

    /**
     * Full scan using ScanConductor (with incremental support)
     * This is the recommended scan method for new code
     */
    async conductorScanImmediate(
        noteId: string,
        text: string,
        entitySpans: EntitySpan[] = []
    ): Promise<ConductorScanResult | null> {
        if (!isTauri()) {
            return null;
        }

        try {
            const startTime = performance.now();
            const result = await conductorScan(text, entitySpans);
            const endTime = performance.now();

            if (this.config.logPerformance) {
                const totalMs = endTime - startTime;
                const mode = result.stats.was_incremental ? 'INCREMENTAL' : result.stats.was_skipped ? 'CACHED' : 'FULL';
                if (totalMs > this.config.slowScanThresholdMs) {
                    console.warn(`[TauriScanner] ${mode} scan: ${totalMs.toFixed(1)}ms`);
                } else {
                    console.debug(`[TauriScanner] ${mode} scan: ${totalMs.toFixed(1)}ms`);
                }
            }

            // Fire result handlers so persistence layer gets called
            // Convert ConductorScanResult to ScanResult-like shape for handlers
            for (const handler of this.resultHandlers) {
                try {
                    handler(noteId, result as any);
                } catch (e) {
                    console.error('[TauriScanner] Handler error:', e);
                }
            }

            return result;
        } catch (error) {
            console.error('[TauriScanner] Conductor scan error:', error);
            return null;
        }
    }

    /**
     * Get conductor status
     */
    async getConductorStatus(): Promise<ConductorStatus | null> {
        if (!isTauri()) return null;
        try {
            return await conductorStatus();
        } catch (error) {
            console.error('[TauriScanner] Status error:', error);
            return null;
        }
    }

    /**
     * Shutdown
     */
    shutdown(): void {
        for (const timer of this.debounceTimers.values()) clearTimeout(timer);
        this.debounceTimers.clear();
        this.resultHandlers = [];
        this.ready = false;
    }
}

// =============================================================================
// Singleton
// =============================================================================

/** Singleton TauriScanner instance */
export const tauriScanner = new TauriScanner();

/** Alias for backwards compatibility with ConductorBridge */
export const conductorBridge = tauriScanner;
