/**
 * ResoRank Search Hooks
 * 
 * Uses native Rust BM25F scorer via Tauri when available,
 * falls back to TypeScript implementation in browser.
 */

import { useEffect, useRef, useState, useCallback } from 'react';
import type { Note } from '@/types/noteTypes';
import { tauriResorank, type ResoRankSearchResult } from '@/lib/tauri/resorank';

// Check if we're in Tauri environment
const isTauri = (): boolean => {
    return typeof window !== 'undefined' && '__TAURI__' in window;
};

export interface ResoRankResult {
    docId: string;
    score: number;
    normalizedScore?: number;
}

/**
 * Hook for ResoRank-powered search
 * Uses native Rust scorer via Tauri for maximum performance
 */
export function useResoRankSearch(notes: Note[]) {
    const [isReady, setIsReady] = useState(false);
    const [isIndexing, setIsIndexing] = useState(false);
    const lastIndexedCountRef = useRef(0);
    const initRef = useRef(false);

    // Initialize and index notes
    useEffect(() => {
        if (notes.length === 0) {
            setIsReady(false);
            return;
        }

        // Skip if already indexed this exact set
        if (lastIndexedCountRef.current === notes.length && initRef.current) {
            return;
        }

        if (!isTauri()) {
            // Fallback: Just mark as ready but search will return empty
            console.warn('[useResoRankSearch] Not in Tauri environment, search disabled');
            setIsReady(true);
            return;
        }

        const indexNotes = async () => {
            setIsIndexing(true);

            try {
                // Initialize if needed
                if (!initRef.current) {
                    await tauriResorank.initialize();
                    initRef.current = true;
                }

                // Index all notes
                const notesToIndex = notes.map(note => ({
                    id: note.id,
                    title: note.title,
                    content: note.content,
                }));

                await tauriResorank.indexNotes(notesToIndex);
                lastIndexedCountRef.current = notes.length;
                setIsReady(true);
            } catch (error) {
                console.error('[useResoRankSearch] Failed to index notes:', error);
            } finally {
                setIsIndexing(false);
            }
        };

        indexNotes();
    }, [notes]);

    const search = useCallback(async (query: string, limit = 20): Promise<ResoRankResult[]> => {
        if (!query.trim() || notes.length === 0) return [];

        if (!isTauri()) {
            // Fallback: simple substring search
            const queryLower = query.toLowerCase();
            return notes
                .filter(n =>
                    n.title.toLowerCase().includes(queryLower) ||
                    n.content.toLowerCase().includes(queryLower)
                )
                .slice(0, limit)
                .map((n, i) => ({
                    docId: n.id,
                    score: 1 - (i * 0.01), // Simple decreasing score
                }));
        }

        try {
            const results = await tauriResorank.search(query, limit);
            return results.map(r => ({
                docId: r.doc_id,
                score: r.score,
                normalizedScore: r.normalized_score,
            }));
        } catch (error) {
            console.error('[useResoRankSearch] Search failed:', error);
            return [];
        }
    }, [notes]);

    // Sync search wrapper for backward compat
    const searchSync = useCallback((query: string, limit = 20): ResoRankResult[] => {
        // For sync calls, we need to return empty and use async version
        console.warn('[useResoRankSearch] Sync search called - use async version for best results');
        return [];
    }, []);

    return {
        search,
        searchSync, // Legacy fallback
        isReady,
        isIndexing
    };
}

/**
 * Hook for debounced ResoRank search with results state
 */
export function useResoRankSearchWithDebounce(
    notes: Note[],
    query: string,
    options: { debounceMs?: number; minLength?: number; limit?: number } = {}
) {
    const { debounceMs = 150, minLength = 2, limit = 20 } = options;
    const { search, isReady, isIndexing } = useResoRankSearch(notes);
    const [results, setResults] = useState<ResoRankResult[]>([]);
    const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    useEffect(() => {
        if (debounceRef.current) {
            clearTimeout(debounceRef.current);
        }

        if (!isReady || query.trim().length < minLength) {
            setResults([]);
            return;
        }

        debounceRef.current = setTimeout(async () => {
            try {
                const searchResults = await search(query, limit);
                setResults(searchResults);
            } catch (error) {
                console.error('[useResoRankSearchWithDebounce] Search error:', error);
                setResults([]);
            }
        }, debounceMs);

        return () => {
            if (debounceRef.current) {
                clearTimeout(debounceRef.current);
            }
        };
    }, [query, isReady, search, debounceMs, minLength, limit]);

    return { results, isReady, isIndexing, search };
}
