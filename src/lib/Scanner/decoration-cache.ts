/**
 * Decoration Cache - Fetch cached decoration spans from CozoDB
 * 
 * Phase 3: IPC-free highlighting. The ScanWorker background thread
 * populates decoration_spans, and we just fetch the results.
 */

import { invoke } from '@tauri-apps/api/core';
import { isTauri } from '@/lib/tauri/bridge';

// Re-use types from existing scanner
import type { ImplicitMention } from '@/lib/Scanner/types';

/**
 * Cached decoration span record (matches Rust DecorationSpanRecord)
 */
export interface DecorationSpanRecord {
    span_type: string;
    start: number;
    end: number;
    entity_id: string | null;
    entity_label: string | null;
    entity_kind: string | null;
    is_alias: boolean;
    metadata: string | null;
}

/**
 * Fetch cached decoration spans for a note from CozoDB
 * 
 * @param noteId - The note ID
 * @param contentHash - Hash of current content (for cache validation)
 * @returns Cached spans if available and valid, null if cache miss
 */
export async function fetchDecorationSpans(
    noteId: string,
    contentHash: string
): Promise<DecorationSpanRecord[] | null> {
    if (!isTauri()) {
        return null;
    }

    try {
        const json = await invoke<string | null>('get_decoration_spans', {
            noteId,
            contentHash,
        });

        if (!json) {
            return null;
        }

        return JSON.parse(json) as DecorationSpanRecord[];
    } catch (error) {
        console.warn('[DecorationCache] Failed to fetch:', error);
        return null;
    }
}

/**
 * Invalidate all cached decoration spans
 * 
 * Called when entity list changes (hydration) to force re-scan of all notes.
 * Returns the number of cache entries cleared.
 */
export async function invalidateAllDecorations(): Promise<number> {
    if (!isTauri()) {
        return 0;
    }

    try {
        const count = await invoke<number>('invalidate_all_decoration_spans');
        console.log(`[DecorationCache] Cleared ${count} cached decoration spans`);
        return count;
    } catch (error) {
        console.warn('[DecorationCache] Failed to invalidate:', error);
        return 0;
    }
}

/**
 * Convert DecorationSpanRecord to ImplicitMention format
 * (for compatibility with existing RustHighlighter)
 */
export function toImplicitMention(span: DecorationSpanRecord): ImplicitMention {
    return {
        start: span.start,
        end: span.end,
        entity_id: span.entity_id || '',
        entity_label: span.entity_label || '',
        entity_kind: span.entity_kind || 'ENTITY',
        is_alias_match: span.is_alias,
    };
}

/**
 * Filter spans to only implicit mentions
 */
export function extractImplicitMentions(spans: DecorationSpanRecord[]): ImplicitMention[] {
    return spans
        .filter(s => s.span_type === 'implicit')
        .map(toImplicitMention);
}
