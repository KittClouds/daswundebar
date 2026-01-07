/**
 * HighlighterFacade - Editor decoration span generation
 * 
 * Scans text and returns decoration spans for syntax highlighting in the editor.
 * This facade handles wikilinks, entities, tags, mentions, triples, etc.
 * All heavy lifting is in Rust WASM.
 * 
 * KEEP THIS FILE AS REFERENCE - maintains full functionality even if Rust breaks
 * 
 * ⚠️ WASM SERIALIZATION QUIRKS (CRITICAL):
 * ----------------------------------------
 * 1. HashMap → Map: serde_wasm_bindgen serializes Rust HashMap<String, String> as 
 *    JavaScript Map, NOT a plain object. Use getCaptureValue() from unified-scanner-utils.ts.
 * 
 * 2. std::time::Instant → PANIC: Not available in WASM. Use the `instant` crate with
 *    the "wasm-bindgen" feature, or avoid timing entirely.
 * 
 * 3. String Slicing → PANIC: Rust string slicing on non-char-boundary causes panic.
 *    Always use .get() or .find() for safe substring extraction.
 * 
 * 4. Panics → RuntimeError: Any Rust panic in WASM causes "RuntimeError: memory access 
 *    out of bounds". Wrap fallible code in std::panic::catch_unwind.
 * 
 * @see unified-scanner-utils.ts for defensive helper functions
 * @module scanner/highlighter-facade
 */

import type { UnifiedScanResult as TauriScanResult } from '@/lib/tauri';

// =============================================================================
// TYPES - Mirror the Rust types exactly
// =============================================================================

/** Kind of reference detected (matches Rust RefKind) */
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

/** Map Rust enum ordinal to string (Rust uses repr(u8)) */
const REF_KIND_MAP: Record<number, RefKind> = {
    0: 'Entity',
    1: 'Wikilink',
    2: 'Backlink',
    3: 'Tag',
    4: 'Mention',
    5: 'Triple',
    6: 'InlineRelation',
    7: 'Temporal',
    8: 'Implicit',
    9: 'Relation',
};

/** Styling hints for TS decoration (matches Rust StylingHint) */
export interface StylingHint {
    /** Semantic color key (e.g., "entity-character", "wikilink", "tag") */
    color_key: string;
    /** Confidence for implicit matches (1.0 for explicit) */
    confidence: number;
    /** Whether this should render as a widget */
    widget_mode: boolean;
    /** Whether the cursor is inside (TS determines from selection) */
    is_editing: boolean;
}

/** A decoration span for TS to render (matches Rust DecorationSpan) */
export interface DecorationSpan {
    kind: RefKind;
    start: number;
    end: number;
    /** Display text (for widgets) */
    label: string;
    /** Full matched text */
    raw_text: string;
    /** 
     * Captured groups from regex pattern
     * 
     * ⚠️ IMPORTANT: serde_wasm_bindgen serializes Rust HashMap<String, String> as JavaScript Map,
     * NOT as a plain object. Use getCaptureValue() from unified-scanner-utils.ts to access safely.
     * 
     * @see getCaptureValue for safe access that handles both Map and Object formats
     */
    captures: Map<string, string> | Record<string, string>;
    /** Styling hints (TS applies mode-aware CSS) */
    styling: StylingHint;
}

/** Full scan result (matches Rust UnifiedScanResult) */
export interface UnifiedScanResult {
    spans: DecorationSpan[];
    stats: UnifiedScanStats;
}

/** Statistics (matches Rust UnifiedScanStats) */
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
// HIGHLIGHTING MODE TYPES
// =============================================================================

export type HighlightMode = 'off' | 'clean' | 'vivid' | 'focus';

/** Entity kinds to highlight in Focus mode */
export interface FocusModeConfig {
    kinds: string[];
}

/** Mode-aware CSS class mapping */
export interface ModeStyles {
    /** Base class for the span */
    baseClass: string;
    /** Additional classes based on mode */
    modeClass: string;
    /** Whether to show as widget */
    showWidget: boolean;
    /** Whether to apply color */
    applyColor: boolean;
}

// =============================================================================
// FACADE CLASS
// =============================================================================

/**
 * HighlighterFacade - Generates decoration spans for editor highlighting
 * 
 * Usage:
 * ```typescript
 * const facade = new HighlighterFacade();
 * await facade.initialize();
 * 
 * const result = facade.scan("Visit [[Rivendell]] and meet [CHARACTER|Frodo]");
 * const decorations = facade.getDecorations(result, 'vivid');
 * ```
 */
export class HighlighterFacade {
    private scanner: WasmUnifiedScanner | null = null;
    private initialized = false;
    private initPromise: Promise<void> | null = null;

    // Cache for performance
    private lastText: string = '';
    private lastResult: UnifiedScanResult | null = null;

    /**
     * Initialize the WASM scanner
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;
        if (this.initPromise) return this.initPromise;

        this.initPromise = this._doInit();
        return this.initPromise;
    }

    private async _doInit(): Promise<void> {
        try {
            // Use Tauri/fallback adapter instead of WASM
            const { getScannerMode, tauriScanner } = await import('@/lib/tauri');
            const mode = getScannerMode();

            if (mode === 'tauri') {
                await tauriScanner.initialize();
                console.log('[Highlighter] Ready via Tauri IPC');
            } else {
                console.log('[Highlighter] Running in fallback mode');
            }

            this.initialized = true;
        } catch (error) {
            console.error('[Highlighter] Failed to initialize:', error);
            throw error;
        }
    }

    /**
     * Check if the facade is ready
     */
    isReady(): boolean {
        return this.initialized && this.scanner !== null;
    }

    /**
     * Scan text and return decoration spans
     * 
     * @param text - Plain text to scan
     * @returns UnifiedScanResult with all spans and statistics
     */
    scan(text: string): UnifiedScanResult {
        if (!this.isReady()) {
            console.warn('[Highlighter] Not initialized, returning empty result');
            return { spans: [], stats: this.emptyStats() };
        }

        // Check cache
        if (text === this.lastText && this.lastResult) {
            return this.lastResult;
        }

        try {
            const rawResult = this.scanner!.scan(text);
            const result = this.parseResult(rawResult);

            // Update cache
            this.lastText = text;
            this.lastResult = result;

            return result;
        } catch (error) {
            console.error('[Highlighter] Scan failed:', error);
            return { spans: [], stats: this.emptyStats() };
        }
    }

    /**
     * Get mode-aware decorations for ProseMirror
     * 
     * This is the main API for KittHighlighter integration.
     * 
     * @param result - Scan result from scan()
     * @param mode - Highlighting mode
     * @param focusConfig - Optional focus mode configuration
     * @param cursorPosition - Current cursor position (for is_editing)
     * @returns Array of decoration specs ready for ProseMirror
     */
    getDecorations(
        result: UnifiedScanResult,
        mode: HighlightMode,
        focusConfig?: FocusModeConfig,
        cursorPosition?: number
    ): Array<{
        from: number;
        to: number;
        spec: ModeStyles;
        span: DecorationSpan;
    }> {
        if (mode === 'off') return [];

        return result.spans
            .filter(span => this.shouldShowSpan(span, mode, focusConfig))
            .map(span => {
                const isEditing = cursorPosition !== undefined &&
                    cursorPosition >= span.start &&
                    cursorPosition <= span.end;

                return {
                    from: span.start,
                    to: span.end,
                    spec: this.getStylesForMode(span, mode, isEditing),
                    span,
                };
            });
    }

    /**
     * Determine if a span should be shown in the current mode
     */
    private shouldShowSpan(
        span: DecorationSpan,
        mode: HighlightMode,
        focusConfig?: FocusModeConfig
    ): boolean {
        if (mode === 'off') return false;
        if (mode === 'vivid' || mode === 'clean') return true;

        // Focus mode - only show matching kinds
        if (mode === 'focus' && focusConfig) {
            if (span.kind === 'Entity') {
                const entityKind = span.captures['entityKind']?.toLowerCase();
                return focusConfig.kinds.some(k => k.toLowerCase() === entityKind);
            }
            // Show all non-entity spans in focus mode
            return true;
        }

        return true;
    }

    /**
     * Get CSS class mapping for a span in a given mode
     */
    private getStylesForMode(
        span: DecorationSpan,
        mode: HighlightMode,
        isEditing: boolean
    ): ModeStyles {
        const baseClass = `highlight-${span.kind.toLowerCase()}`;

        switch (mode) {
            case 'clean':
                return {
                    baseClass,
                    modeClass: isEditing ? 'highlight-editing' : 'highlight-clean',
                    showWidget: span.styling.widget_mode && !isEditing,
                    applyColor: isEditing, // Only color when editing
                };

            case 'vivid':
                return {
                    baseClass,
                    modeClass: isEditing ? 'highlight-vivid-editing' : `highlight-vivid ${span.styling.color_key}`,
                    showWidget: span.styling.widget_mode && !isEditing, // Expand on click
                    applyColor: true,
                };

            case 'focus':
                return {
                    baseClass,
                    modeClass: 'highlight-focus',
                    showWidget: span.styling.widget_mode,
                    applyColor: true,
                };

            default:
                return {
                    baseClass,
                    modeClass: '',
                    showWidget: false,
                    applyColor: false,
                };
        }
    }

    /**
     * Parse raw WASM result into typed structure
     */
    private parseResult(raw: unknown): UnifiedScanResult {
        const data = raw as {
            spans: Array<{
                kind: number | RefKind;
                start: number;
                end: number;
                label: string;
                raw_text: string;
                captures: Record<string, string>;
                styling: StylingHint;
            }>;
            stats: UnifiedScanStats;
        };

        return {
            spans: data.spans.map(span => ({
                ...span,
                kind: typeof span.kind === 'number'
                    ? REF_KIND_MAP[span.kind] || 'Entity'
                    : span.kind,
            })),
            stats: data.stats,
        };
    }

    /**
     * Return empty stats structure
     */
    private emptyStats(): UnifiedScanStats {
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
     * Clear the cache (call when entities change)
     */
    invalidateCache(): void {
        this.lastText = '';
        this.lastResult = null;
    }

    /**
     * Get count of spans (cheap operation for validation)
     */
    countSpans(text: string): number {
        if (!this.isReady()) return 0;
        try {
            return this.scanner!.countSpans(text);
        } catch {
            return 0;
        }
    }
}

// =============================================================================
// SINGLETON
// =============================================================================

/** Singleton instance for app-wide use */
export const highlighterFacade = new HighlighterFacade();
/** @deprecated Use highlighterFacade instead */
export const unifiedScannerFacade = highlighterFacade;

// =============================================================================
// LEGACY COMPATIBILITY HELPERS
// =============================================================================

/**
 * Convert UnifiedScanResult to legacy PatternRange format
 * 
 * Use this to integrate with existing code that expects the old format.
 */
export function toPatternRanges(result: UnifiedScanResult): Array<{
    start: number;
    end: number;
    kind: string;
    label: string;
    fullMatch: string;
    color?: string;
    backgroundColor?: string;
    extraClasses?: string;
}> {
    return result.spans.map(span => ({
        start: span.start,
        end: span.end,
        kind: span.kind.toLowerCase(),
        label: span.label,
        fullMatch: span.raw_text,
        color: getColorForKind(span),
        backgroundColor: getBackgroundForKind(span),
        extraClasses: span.styling.widget_mode ? 'widget-enabled' : '',
    }));
}

/**
 * Get color for a span kind
 */
function getColorForKind(span: DecorationSpan): string {
    const colorMap: Record<string, string> = {
        'entity-character': 'var(--entity-character)',
        'entity-location': 'var(--entity-location)',
        'entity-organization': 'var(--entity-organization)',
        'entity-item': 'var(--entity-item)',
        'entity-event': 'var(--entity-event)',
        'entity-concept': 'var(--entity-concept)',
        'wikilink': 'hsl(var(--primary))',
        'backlink': 'hsl(var(--primary))',
        'tag': '#3b82f6',
        'mention': '#8b5cf6',
        'triple': 'var(--entity-relationship)',
    };
    return colorMap[span.styling.color_key] || 'inherit';
}

/**
 * Get background color for a span kind
 */
function getBackgroundForKind(span: DecorationSpan): string {
    const bgMap: Record<string, string> = {
        'wikilink': 'hsl(var(--primary) / 0.15)',
        'backlink': 'hsl(var(--primary) / 0.20)',
        'tag': '#3b82f620',
        'mention': '#8b5cf620',
    };
    return bgMap[span.styling.color_key] || 'transparent';
}
