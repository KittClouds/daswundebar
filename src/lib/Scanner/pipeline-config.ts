/**
 * Pipeline Configuration
 * 
 * Controls which pipeline (WASM vs Tauri) is active for A/B testing.
 * 
 * To switch pipelines, change the PIPELINE constant:
 *   - 'tauri': Use Tauri native scanner (Rust IPC)
 *   - 'wasm': Use WASM kittcore (legacy)
 *   - 'auto': Auto-detect (Tauri if available, else WASM)
 */

export type PipelineMode = 'tauri' | 'wasm' | 'auto';

// ============================================================================
// TOGGLE THIS TO SWITCH PIPELINES
// ============================================================================

/**
 * Active pipeline for scanning/highlighting
 * 
 * Set to 'tauri' to test native Rust pipeline
 * Set to 'wasm' to test legacy WASM pipeline
 * Set to 'auto' for automatic detection
 */
export const ACTIVE_PIPELINE: PipelineMode = 'tauri';

// ============================================================================
// Feature Flags
// ============================================================================

/** Enable verbose logging for pipeline operations */
export const PIPELINE_VERBOSE_LOGGING = true;

/** Enable RAG pipeline (native embeddings) */
export const RAG_ENABLED = true;

/** Enable CST relation extraction */
export const CST_EXTRACTION_ENABLED = true;

// ============================================================================
// Helpers
// ============================================================================

/**
 * Check if WASM pipeline is enabled
 */
export function isWasmEnabled(): boolean {
    return ACTIVE_PIPELINE === 'wasm';
}

/**
 * Check if Tauri pipeline is enabled
 */
export function isTauriPipelineEnabled(): boolean {
    return ACTIVE_PIPELINE === 'tauri';
}

/**
 * Get resolved pipeline mode
 * 
 * If 'tauri' is configured but Tauri isn't present, falls back to 'wasm'
 */
export function getResolvedPipeline(): 'tauri' | 'wasm' {
    // Use the centralized check from bridge.ts (handles both v1 and v2)
    const hasTauri = typeof window !== 'undefined' && (
        '__TAURI__' in window || '__TAURI_INTERNALS__' in window
    );

    if (ACTIVE_PIPELINE === 'auto') {
        return hasTauri ? 'tauri' : 'wasm';
    }

    if (ACTIVE_PIPELINE === 'tauri' && !hasTauri) {
        console.warn('[Pipeline] Config is "tauri" but Tauri not detected - falling back to WASM');
        return 'wasm';
    }

    return ACTIVE_PIPELINE;
}

/**
 * Log pipeline status at startup
 */
export function logPipelineStatus(): void {
    const resolved = getResolvedPipeline();
    const hasTauri = typeof window !== 'undefined' && (
        '__TAURI__' in window || '__TAURI_INTERNALS__' in window
    );

    console.log(`%c[Pipeline] Active: ${resolved.toUpperCase()}`, 'color: #0ea5e9; font-weight: bold');
    console.log(`  - Config: ${ACTIVE_PIPELINE}`);
    console.log(`  - Tauri detected: ${hasTauri}`);
    console.log(`  - RAG: ${RAG_ENABLED ? 'enabled' : 'disabled'}`);
    console.log(`  - CST: ${CST_EXTRACTION_ENABLED ? 'enabled' : 'disabled'}`);
    console.log(`  - Verbose: ${PIPELINE_VERBOSE_LOGGING ? 'yes' : 'no'}`);
}
