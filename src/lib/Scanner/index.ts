/**
 * Scanner Module - Unified API for Rust WASM Document Processing
 * 
 * Two main pipelines:
 *   - ExtractorFacade: Entity/relation extraction → persists to CozoDB
 *   - HighlighterFacade: Decoration spans → editor highlighting
 * 
 * Usage:
 *   import { extractorFacade, highlighterFacade } from '@/lib/scanner';
 *   await extractorFacade.initialize();
 *   extractorFacade.scan(noteId, text);  // extracts & persists
 *   highlighterFacade.scan(text);        // returns decoration spans
 */

// ===========================================================================
// PIPELINE CONFIG - Controls WASM vs Tauri mode
// ===========================================================================

export {
    ACTIVE_PIPELINE,
    PIPELINE_VERBOSE_LOGGING,
    RAG_ENABLED,
    CST_EXTRACTION_ENABLED,
    isWasmEnabled,
    isTauriPipelineEnabled,
    getResolvedPipeline,
    logPipelineStatus,
    type PipelineMode,
} from './pipeline-config';

// ===========================================================================
// EXTRACTION PIPELINE - Entity/relation extraction → CozoDB
// ===========================================================================

export { extractorFacade, scannerFacade, parseNoteConnectionsFromDocument } from './extractor-facade';

// Re-export types from bridge
export type {
    ScanResult,
    ExtractedRelation,
    ExtractedTriple,
    ImplicitMention,
    TemporalMention,
    EntityDefinition,
    EntitySpan,
} from './bridge';

// Low-level bridges (for advanced use / A/B testing)
export {
    conductorBridge,
    ConductorBridge,
} from './bridge';

// Persistence
export { persistTemporalMentions, clearTemporalMentions } from './temporal-persistence';

// Pattern Bridge (for loading relation patterns)
export {
    scannerPatternBridge,
    ScannerPatternBridge,
    loadRelationPatternsForScanner,
    refreshScannerPatterns,
} from './pattern-bridge';

// ===========================================================================
// HIGHLIGHTING PIPELINE - Decoration spans for editor
// ===========================================================================

export {
    HighlighterFacade,
    highlighterFacade,
    // Legacy aliases (deprecated)
    unifiedScannerFacade,
    toPatternRanges,
    type RefKind,
    type StylingHint,
    type DecorationSpan,
    type UnifiedScanResult,
    type UnifiedScanStats,
    type HighlightMode,
    type FocusModeConfig,
    type ModeStyles,
} from './highlighter-facade';

/** @deprecated Use HighlighterFacade instead */
export { HighlighterFacade as UnifiedScannerFacade } from './highlighter-facade';

// ===========================================================================
// OTHER FACADES
// ===========================================================================

// Constraints - Validation + uniqueness
export {
    ConstraintsFacade,
    constraintsFacade,
    type ConstraintResult,
    type RefInput,
    type RefPosition,
    type RefPayload,
} from './constraints-facade';

// Projections - Timelines, graphs, character sheets
export {
    ProjectionsFacade,
    projectionsFacade,
    type TimelineEvent,
    type CharacterSheet,
    type CharacterRelationship,
    type NoteAppearance,
    type CharacterStats,
    type RelationshipGraph,
    type LinkGraph,
    type ProjectionRef,
    type ProjectionPayload,
} from './projections-facade';

/**
 * Initialize highlighter ONLY - call at app startup
 * 
 * Ultra-fast: just loads WASM for instant decoration rendering.
 * Does NOT require entities, DB, or note context.
 */
export async function initializeHighlighter(): Promise<boolean> {
    try {
        const m = await import('./highlighter-facade');
        await m.highlighterFacade.initialize();
        return m.highlighterFacade.isReady();
    } catch (err) {
        console.error('[Scanner] Highlighter init failed:', err);
        return false;
    }
}

/**
 * Initialize auxiliary facades - call lazily after app ready
 */
export async function initializeAuxiliaryFacades(): Promise<{
    constraints: boolean;
    projections: boolean;
}> {
    const results = await Promise.allSettled([
        import('./constraints-facade').then(m => m.constraintsFacade.initialize()),
        import('./projections-facade').then(m => m.projectionsFacade.initialize()),
    ]);
    return {
        constraints: results[0].status === 'fulfilled',
        projections: results[1].status === 'fulfilled',
    };
}

/**
 * Initialize all Rust-first facades (backward compat)
 * 
 * @deprecated Use initializeHighlighter() at app startup instead
 */
export async function initializeRustFacades(): Promise<{
    highlighter: boolean;
    constraints: boolean;
    projections: boolean;
}> {
    const highlighter = await initializeHighlighter();
    const auxiliary = await initializeAuxiliaryFacades();
    return { highlighter, ...auxiliary };
}

