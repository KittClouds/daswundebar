/**
 * HighlighterBridge - Bridge to Rust SyntaxCortex and ImplicitCortex
 * 
 * Provides high-performance syntax highlighting via WASM:
 * - Pattern matching (wikilinks, entities, tags, etc.)
 * - Implicit entity detection (Aho-Corasick)
 * - Content-hash based caching
 */

import type { RegisteredEntity } from '@/lib/tauri/smart-graph-registry';
import { decorationCache } from './decoration-cache';

// ==================== TYPES ====================

// Re-export types from shared types file
export type { HighlightSpan, SpanMetadata } from './types';
import type { HighlightSpan } from './types';

// Extended result type with textLength for optimization
export interface HighlightResult {
    spans: HighlightSpan[];
    contentHash: string;
    textLength: number; // M2: For O(1) length pre-check before O(n) hash
    wasCached: boolean;
    scanTimeMs: number;
}

// ==================== BRIDGE CLASS ====================

class HighlighterBridge {
    private initialized = false;
    private tauriMode = false; // True when using Tauri backend instead of WASM
    private lastResultByNote = new Map<string, HighlightResult & { textLength: number }>();
    private lastHashByNote = new Map<string, string>();
    private lastLengthByNote = new Map<string, number>(); // M2: Fast length check
    private hydrationCallbacks: Array<() => void> = [];

    // Rust cortexes (imported dynamically) - LEGACY, unused in Tauri mode
    private DocumentCortex: any = null;
    private cortex: any = null;

    /**
     * Initialize the bridge by loading Tauri or fallback mode
     */
    async initialize(): Promise<boolean> {
        if (this.initialized) return true;

        try {
            // Check scanner mode
            const { getScannerMode, tauriOrchestrator } = await import('@/lib/tauri');
            const mode = getScannerMode();

            if (mode === 'tauri') {
                // Wait for orchestrator (handles connection + entity loading + hydration)
                await tauriOrchestrator.waitForReady();
                this.tauriMode = true;
                console.log('[HighlighterBridge] Tauri backend ready (via orchestrator)');
            } else {
                this.tauriMode = false;
                console.log('[HighlighterBridge] Running in fallback mode (no Tauri)');
            }

            this.initialized = true;
            return true;
        } catch (error) {
            console.error('[HighlighterBridge] Failed to initialize:', error);
            return false;
        }
    }

    /**
     * Check if bridge is ready
     * In Tauri mode, delegates to tauriScanner.isReady()
     * In fallback mode, returns false (WASM disabled)
     */
    isReady(): boolean {
        if (!this.initialized) return false;
        if (this.tauriMode) {
            // Dynamically check Tauri scanner readiness
            try {
                // Access the global tauriScanner - already imported during init
                const mod = require('@/lib/tauri');
                return mod.tauriScanner?.isReady() ?? false;
            } catch {
                return false;
            }
        }
        // Legacy WASM mode - check cortex
        return this.cortex !== null;
    }

    /**
     * Hydrate with entities for implicit matching
     */
    hydrateEntities(entities: Array<{ id: string; label: string; kind: string; aliases: string[] }>): void {
        if (!this.isReady()) {
            return;  // Silent return - will be called again when ready
        }

        try {
            const entityDefs = entities.map(e => ({
                id: e.id,
                label: e.label,
                kind: e.kind,
                aliases: e.aliases || [],
            }));

            this.cortex.hydrateEntities(entityDefs);
            console.log(`[HighlighterBridge] Hydrated with ${entities.length} entities`);

            // Clear all caches so fresh highlight happens
            this.clearAllCaches();

            // Invalidate persistent cache (entity change = all spans potentially different)
            decorationCache.invalidateAll();

            // Notify subscribers that entities were hydrated
            for (const callback of this.hydrationCallbacks) {
                try {
                    callback();
                } catch (err) {
                    console.warn('[HighlighterBridge] Hydration callback error:', err);
                }
            }
        } catch (error) {
            console.error('[HighlighterBridge] Failed to hydrate entities:', error);
        }
    }

    /**
     * Subscribe to hydration events (for triggering editor rescan)
     * Returns unsubscribe function
     */
    onHydration(callback: () => void): () => void {
        this.hydrationCallbacks.push(callback);
        return () => {
            const idx = this.hydrationCallbacks.indexOf(callback);
            if (idx >= 0) {
                this.hydrationCallbacks.splice(idx, 1);
            }
        };
    }

    /**
     * Compute content hash for change detection
     */
    private computeHash(text: string): string {
        // Simple djb2 hash
        let hash = 5381;
        for (let i = 0; i < text.length; i++) {
            hash = ((hash << 5) + hash) + text.charCodeAt(i);
            hash = hash >>> 0; // Convert to unsigned 32-bit
        }
        return hash.toString(16);
    }

    /**
     * Highlight text using Rust scanner
     * Returns cached result if content unchanged
     */
    highlight(text: string, noteId: string): HighlightResult {
        const start = performance.now();

        // M2: Fast O(1) length pre-check before O(n) hash
        const lastLength = this.lastLengthByNote.get(noteId);
        const lengthChanged = lastLength !== undefined && lastLength !== text.length;

        // Only compute hash if length matches (potential cache hit)
        if (!lengthChanged) {
            const hash = this.computeHash(text);
            const lastHash = this.lastHashByNote.get(noteId);

            if (lastHash === hash) {
                const cached = this.lastResultByNote.get(noteId);
                if (cached) {
                    return { ...cached, wasCached: true };
                }
            }
        }

        // Compute hash for storage (needed even if length changed)
        const hash = this.computeHash(text);

        // Not cached - perform full scan
        if (!this.isReady()) {
            return {
                spans: [],
                contentHash: hash,
                textLength: text.length,
                wasCached: false,
                scanTimeMs: performance.now() - start,
            };
        }

        try {
            // Call Rust DocumentCortex.scan() - returns full ScanResult
            const scanResult = this.cortex.scan(text, []);

            if (!scanResult) {
                return {
                    spans: [],
                    contentHash: hash,
                    textLength: text.length,
                    wasCached: false,
                    scanTimeMs: performance.now() - start,
                };
            }

            // Convert Rust results to HighlightSpans
            const spans: HighlightSpan[] = [];

            // Add implicit mentions
            if (scanResult.implicit && Array.isArray(scanResult.implicit)) {
                for (const m of scanResult.implicit) {
                    spans.push({
                        kind: 'implicit',
                        start: m.start,
                        end: m.end,
                        content: m.matched_text,
                        label: m.entity_label,
                        target: m.entity_id,
                        confidence: m.confidence ?? (m.is_alias_match ? 0.9 : 1.0),
                        metadata: {
                            entityKind: m.entity_kind,
                            entityId: m.entity_id,
                            isAlias: m.is_alias_match,
                        },
                    });
                }
            }

            // Add temporal mentions
            if (scanResult.temporal && Array.isArray(scanResult.temporal)) {
                for (const t of scanResult.temporal) {
                    spans.push({
                        kind: 'temporal',
                        start: t.start,
                        end: t.end,
                        content: t.text,
                        label: t.text,
                        target: t.kind,
                        confidence: t.confidence ?? 0.9,
                        metadata: {
                            captures: t.metadata,
                        },
                    });
                }
            }

            // Sort by position
            spans.sort((a, b) => a.start - b.start);

            const result: HighlightResult = {
                spans,
                contentHash: hash,
                textLength: text.length,
                wasCached: false,
                scanTimeMs: performance.now() - start,
            };

            // Cache result + length (in-memory)
            this.lastHashByNote.set(noteId, hash);
            this.lastLengthByNote.set(noteId, text.length);
            this.lastResultByNote.set(noteId, result);

            // Persist to SQLite (fire and forget - non-blocking)
            decorationCache.set(noteId, hash, spans).catch(() => { });

            return result;
        } catch (error) {
            console.error('[HighlighterBridge] Highlight failed:', error);
            return {
                spans: [],
                contentHash: hash,
                textLength: text.length,
                wasCached: false,
                scanTimeMs: performance.now() - start,
            };
        }
    }

    /**
     * Pre-load cached decorations from SQLite into memory
     * Call this when a note is about to be rendered for instant cache hits
     */
    async preloadCache(noteId: string, text: string): Promise<boolean> {
        const hash = this.computeHash(text);

        // Check if already in memory
        if (this.lastHashByNote.get(noteId) === hash) {
            return true; // Already cached
        }

        // Try to load from SQLite
        const spans = await decorationCache.get(noteId, hash);
        if (spans) {
            // Populate in-memory cache
            const result: HighlightResult = {
                spans,
                contentHash: hash,
                textLength: text.length,
                wasCached: true,
                scanTimeMs: 0,
            };
            this.lastHashByNote.set(noteId, hash);
            this.lastLengthByNote.set(noteId, text.length);
            this.lastResultByNote.set(noteId, result);
            return true;
        }

        return false;
    }

    /**
     * Clear cache for a specific note
     */
    invalidateCache(noteId: string): void {
        this.lastHashByNote.delete(noteId);
        this.lastResultByNote.delete(noteId);
        this.lastLengthByNote.delete(noteId);
        // Also invalidate persistent cache
        decorationCache.invalidate(noteId).catch(() => { });
    }

    /**
     * Clear all caches
     */
    clearAllCaches(): void {
        this.lastHashByNote.clear();
        this.lastResultByNote.clear();
        this.lastLengthByNote.clear();
    }
}

// ==================== SINGLETON ====================

export const highlighterBridge = new HighlighterBridge();
