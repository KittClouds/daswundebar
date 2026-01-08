/**
 * Tauri ResoRank Bridge
 * 
 * TypeScript wrapper for the native Rust ResoRank scorer.
 * Provides BM25F search with proximity scoring via Tauri commands.
 */

import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Types
// =============================================================================

/** Search result from ResoRank */
export interface ResoRankSearchResult {
    doc_id: string;
    score: number;
    normalized_score?: number;
}

/** Index statistics */
export interface ResoRankStats {
    documentCount: number;
    termCount: number;
    idfCacheSize: number;
    entropyCacheSize: number;
}

// =============================================================================
// Tauri Commands
// =============================================================================

/**
 * Search the ResoRank index
 */
export async function resorankSearch(query: string, limit: number = 20): Promise<ResoRankSearchResult[]> {
    return invoke<ResoRankSearchResult[]>('resorank_search', { query, limit });
}

/**
 * Index a document
 */
export async function resorankIndex(docId: string, title: string, content: string): Promise<boolean> {
    return invoke<boolean>('resorank_index', { docId, title, content });
}

/**
 * Clear the entire index
 */
export async function resorankClear(): Promise<void> {
    return invoke<void>('resorank_clear');
}

/**
 * Get index statistics
 */
export async function resorankStats(): Promise<ResoRankStats> {
    return invoke<ResoRankStats>('resorank_stats');
}

// =============================================================================
// ResoRank Facade for React Hooks
// =============================================================================

/**
 * ResoRank Tauri Facade
 * 
 * Matches the API expected by useResoRankSearch hook.
 * Uses native Rust BM25F scoring instead of TypeScript.
 */
class TauriResoRankFacade {
    private initialized = false;
    private indexedDocs = new Set<string>();

    /**
     * Initialize the facade
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;

        // Clear any stale index data
        await resorankClear();
        this.indexedDocs.clear();
        this.initialized = true;

        console.log('[TauriResoRank] Initialized');
    }

    /**
     * Index a note for search
     */
    async indexNote(id: string, title: string, content: string): Promise<void> {
        await resorankIndex(id, title, content);
        this.indexedDocs.add(id);
    }

    /**
     * Bulk index multiple notes
     */
    async indexNotes(notes: Array<{ id: string; title: string; content: string }>): Promise<void> {
        // Clear and re-index
        await resorankClear();
        this.indexedDocs.clear();

        // Index all notes
        for (const note of notes) {
            await resorankIndex(note.id, note.title, note.content);
            this.indexedDocs.add(note.id);
        }

        console.log(`[TauriResoRank] Indexed ${notes.length} notes`);
    }

    /**
     * Search for notes matching query
     */
    async search(query: string, limit: number = 20): Promise<ResoRankSearchResult[]> {
        if (!query.trim()) return [];
        return resorankSearch(query, limit);
    }

    /**
     * Clear all indexed data
     */
    async clear(): Promise<void> {
        await resorankClear();
        this.indexedDocs.clear();
    }

    /**
     * Check if initialized
     */
    isReady(): boolean {
        return this.initialized;
    }

    /**
     * Get statistics
     */
    async getStats(): Promise<ResoRankStats> {
        return resorankStats();
    }
}

// Singleton instance
export const tauriResorank = new TauriResoRankFacade();
