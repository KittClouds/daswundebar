/**
 * Scanner Adapter - Auto-selects Tauri or TypeScript fallback
 * 
 * This adapter provides a unified interface that:
 * - Uses Tauri IPC when running in Tauri desktop app
 * - Falls back to TypeScript implementations in browser
 * 
 * This is the single source of truth for scanner functionality.
 * 
 * @module lib/Scanner/adapter
 */

import { isTauri, tauriScanner, unifiedScan as tauriUnifiedScan, getScannerMode } from '@/lib/tauri';
import type {
    EntityDefinition,
    EntitySpan,
    ScanResult,
    UnifiedScanResult,
    DecorationSpan,
    UnifiedScanStats,
    RefKind,
    ScannerMode,
} from '@/lib/tauri';

// Re-export types for convenience
export type {
    EntityDefinition,
    EntitySpan,
    ScanResult,
    UnifiedScanResult,
    DecorationSpan,
    UnifiedScanStats,
    RefKind,
    ScannerMode,
};

// Re-export getScannerMode for convenience
export { getScannerMode };

// =============================================================================
// UNIFIED SCANNER
// =============================================================================

/**
 * Empty stats for fallback mode
 */
function emptyStats(): UnifiedScanStats {
    return {
        entity_count: 0,
        wikilink_count: 0,
        backlink_count: 0,
        tag_count: 0,
        mention_count: 0,
        triple_count: 0,
        temporal_count: 0,
        implicit_count: 0,
        relation_count: 0,
        total_spans: 0,
        scan_time_us: 0,
    };
}

/**
 * Empty result for fallback mode
 */
function emptyResult(): UnifiedScanResult {
    return { spans: [], stats: emptyStats() };
}

/**
 * Unified scan - uses Tauri when available, falls back to empty result
 */
export async function unifiedScan(text: string, entities: EntitySpan[] = []): Promise<UnifiedScanResult> {
    if (isTauri()) {
        try {
            return await tauriUnifiedScan(text, entities);
        } catch (error) {
            console.error('[ScannerAdapter] Tauri scan failed:', error);
            return emptyResult();
        }
    }

    // Fallback: return empty result
    // In future, could implement pure-TS scanning here
    console.log('[ScannerAdapter] Running in fallback mode (no Tauri)');
    return emptyResult();
}

// =============================================================================
// CONDUCTOR BRIDGE REPLACEMENT
// =============================================================================

/** Bridge configuration */
export interface ScannerBridgeConfig {
    debounceMs: number;
    logPerformance: boolean;
    slowScanThresholdMs: number;
}

const DEFAULT_CONFIG: ScannerBridgeConfig = {
    debounceMs: 300,
    logPerformance: true,
    slowScanThresholdMs: 50,
};

/**
 * ScannerBridge - Unified bridge that works with both Tauri and fallback
 * 
 * This is a drop-in replacement for ConductorBridge.
 */
export class ScannerBridge {
    private ready = false;
    private readyCallbacks: Array<() => void> = [];
    private debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();
    private resultHandlers: Array<(noteId: string, result: ScanResult) => void> = [];
    private config: ScannerBridgeConfig;
    private mode: ScannerMode;

    constructor(config: Partial<ScannerBridgeConfig> = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
        this.mode = getScannerMode();
    }

    /**
     * Initialize the scanner bridge
     */
    async initialize(): Promise<void> {
        this.mode = getScannerMode();

        if (this.mode === 'tauri') {
            await tauriScanner.initialize();
        }

        console.log(`[ScannerBridge] Initialized in ${this.mode} mode`);
    }

    /**
     * Check if ready
     */
    isReady(): boolean {
        if (this.mode === 'tauri') {
            return tauriScanner.isReady();
        }
        return this.ready;
    }

    /**
     * Get current state name
     */
    getStateName(): string {
        if (this.mode === 'tauri') {
            return tauriScanner.getStateName();
        }
        return this.ready ? 'ready' : 'not-ready';
    }

    /**
     * Subscribe to ready event
     */
    onReady(callback: () => void): () => void {
        if (this.mode === 'tauri') {
            return tauriScanner.onReady(callback);
        }

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
     * Hydrate entities
     */
    async hydrateEntities(entities: EntityDefinition[]): Promise<void> {
        if (this.mode === 'tauri') {
            await tauriScanner.hydrateEntities(entities);
            return;
        }

        // Fallback mode - just mark as ready
        console.log(`[ScannerBridge] Fallback mode: ${entities.length} entities (no processing)`);
        this.ready = true;
        for (const cb of this.readyCallbacks) {
            try { cb(); } catch (e) { console.warn('[ScannerBridge] Ready callback error:', e); }
        }
        this.readyCallbacks = [];
    }

    /**
     * Register result handler
     */
    onResult(handler: (noteId: string, result: ScanResult) => void): () => void {
        if (this.mode === 'tauri') {
            return tauriScanner.onResult(handler);
        }

        this.resultHandlers.push(handler);
        return () => {
            const idx = this.resultHandlers.indexOf(handler);
            if (idx >= 0) this.resultHandlers.splice(idx, 1);
        };
    }

    /**
     * Scan (debounced)
     */
    scan(noteId: string, text: string, entitySpans: EntitySpan[] = []): void {
        if (this.mode === 'tauri') {
            tauriScanner.scan(noteId, text, entitySpans);
            return;
        }

        // Fallback: debounce then execute
        const existing = this.debounceTimers.get(noteId);
        if (existing) clearTimeout(existing);

        const timer = setTimeout(() => {
            this.executeFallbackScan(noteId, text);
            this.debounceTimers.delete(noteId);
        }, this.config.debounceMs);

        this.debounceTimers.set(noteId, timer);
    }

    /**
     * Immediate scan - Uses conductor pipeline for full document scanning
     */
    async scanImmediate(noteId: string, text: string, _entitySpans: EntitySpan[] = []): Promise<ScanResult | null> {
        if (this.mode === 'tauri') {
            // Use the unified conductor pipeline
            const result = await tauriScanner.conductorScanImmediate(noteId, text, []);
            // Type coercion: ConductorScanResult is a superset of ScanResult
            return result as unknown as ScanResult;
        }

        return this.executeFallbackScan(noteId, text);
    }

    private executeFallbackScan(noteId: string, _text: string): ScanResult | null {
        const result: ScanResult = {
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

        for (const handler of this.resultHandlers) {
            try { handler(noteId, result); }
            catch (e) { console.error('[ScannerBridge] Handler error:', e); }
        }

        return result;
    }

    /**
     * Shutdown
     */
    shutdown(): void {
        if (this.mode === 'tauri') {
            tauriScanner.shutdown();
            return;
        }

        for (const timer of this.debounceTimers.values()) clearTimeout(timer);
        this.debounceTimers.clear();
        this.resultHandlers = [];
        this.ready = false;
    }
}

// =============================================================================
// SINGLETONS
// =============================================================================

/** Singleton bridge instance */
export const scannerBridge = new ScannerBridge();

/** Alias for ConductorBridge compatibility */
export const conductorBridge = scannerBridge;

// =============================================================================
// HIGHLIGHTER ADAPTER
// =============================================================================

/**
 * Highlighter that works with Tauri or fallback
 */
export class HighlighterAdapter {
    private initialized = false;
    private mode: ScannerMode;
    private lastText = '';
    private lastResult: UnifiedScanResult | null = null;

    constructor() {
        this.mode = getScannerMode();
    }

    async initialize(): Promise<void> {
        this.mode = getScannerMode();

        if (this.mode === 'tauri') {
            await tauriScanner.initialize();
        }

        this.initialized = true;
        console.log(`[HighlighterAdapter] Initialized in ${this.mode} mode`);
    }

    isReady(): boolean {
        return this.initialized;
    }

    scan(text: string): UnifiedScanResult {
        // Check cache
        if (text === this.lastText && this.lastResult) {
            return this.lastResult;
        }

        // In Tauri mode, we need async call - for sync API, return cached or empty
        if (this.mode === 'tauri') {
            // Trigger async scan for next call
            this.asyncScan(text);
            return this.lastResult || emptyResult();
        }

        // Fallback: return empty
        return emptyResult();
    }

    private async asyncScan(text: string): Promise<void> {
        try {
            const result = await unifiedScan(text, []);
            this.lastText = text;
            this.lastResult = result;
        } catch (error) {
            console.error('[HighlighterAdapter] Async scan failed:', error);
        }
    }

    invalidateCache(): void {
        this.lastText = '';
        this.lastResult = null;
    }
}

/** Singleton highlighter */
export const highlighterAdapter = new HighlighterAdapter();

/** Alias for highlighterFacade compatibility */
export const highlighterFacade = highlighterAdapter;
