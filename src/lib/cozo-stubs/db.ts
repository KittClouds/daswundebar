/**
 * CozoDB Stub - DEPRECATED
 * 
 * The browser CozoDB has been replaced by Rust CozoDB via Tauri.
 * This stub exists only to prevent import errors during migration.
 * 
 * All consumers should migrate to:
 * - lib/tauri/smart-graph-registry.ts for entity operations
 * - lib/tauri/time-registry.ts for temporal operations
 * - lib/tauri/graph-registry.ts for graph queries
 */

// Stub cozoDb - throws helpful errors if accidentally called
export const cozoDb = {
    isReady: () => false,
    runQuery: (_query: string, _params?: Record<string, unknown>) => {
        console.warn('[cozoDb STUB] Browser CozoDB is deprecated. Use Tauri graph-registry instead.');
        return { ok: false, rows: [], headers: [], message: 'DEPRECATED: Use Tauri' };
    },
    init: async () => {
        console.warn('[cozoDb STUB] Browser CozoDB init is a no-op. Tauri initializes automatically.');
    },
    close: () => {
        console.warn('[cozoDb STUB] Browser CozoDB close is a no-op.');
    },
};

export default cozoDb;
