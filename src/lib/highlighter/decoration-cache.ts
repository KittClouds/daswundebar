/**
 * Decoration Cache - CozoDB-backed decoration span cache
 * 
 * Phase 3: Uses Tauri commands to fetch/invalidate cached spans from CozoDB.
 * The ScanWorker background thread populates decoration_spans after note saves,
 * and we just fetch the results.
 */

import type { HighlightSpan } from './types';
import {
    fetchDecorationSpans,
    invalidateAllDecorations,
    extractImplicitMentions,
    type DecorationSpanRecord
} from '@/lib/Scanner/decoration-cache';
import { isTauri } from '@/lib/tauri/bridge';

// Entity version - bumps when entities are hydrated, invalidating all caches
let currentEntityVersion = 0;

/**
 * Convert DecorationSpanRecord to HighlightSpan
 */
function toHighlightSpan(record: DecorationSpanRecord): HighlightSpan {
    return {
        kind: record.span_type as HighlightSpan['kind'],
        start: record.start,
        end: record.end,
        content: record.entity_label || '',
        label: record.entity_label || '',
        target: record.entity_id || '',
        confidence: 1.0,
        metadata: {
            entityKind: record.entity_kind || undefined,
            entityId: record.entity_id || undefined,
            isAlias: record.is_alias,
        },
    };
}

class DecorationCache {
    /**
     * Get cached decorations from CozoDB
     * 
     * @param noteId - The note ID
     * @param contentHash - Hash of current content (for cache validation)
     * @returns Cached spans if available and valid, null if cache miss
     */
    async get(noteId: string, contentHash: string): Promise<HighlightSpan[] | null> {
        if (!isTauri()) {
            return null;
        }

        try {
            const records = await fetchDecorationSpans(noteId, contentHash);
            if (!records) {
                return null;
            }

            return records.map(toHighlightSpan);
        } catch (error) {
            console.warn('[DecorationCache] Failed to get cached spans:', error);
            return null;
        }
    }

    /**
     * Store decorations - NO-OP in Phase 3
     * 
     * The ScanWorker background thread handles persistence now.
     * This method exists for backward compatibility but does nothing.
     */
    async set(_noteId: string, _contentHash: string, _spans: HighlightSpan[]): Promise<void> {
        // NO-OP: ScanWorker handles persistence asynchronously
        // after cozo_update_note queues the scan
    }

    /**
     * Invalidate cache for a specific note - NO-OP in Phase 3
     * 
     * Cache invalidation happens automatically when content changes
     * because the ScanWorker creates a new entry with the new content hash.
     */
    async invalidate(_noteId: string): Promise<void> {
        // NO-OP: New content = new hash = automatic cache miss
    }

    /**
     * Invalidate ALL caches (called when entities change)
     * 
     * This triggers a CozoDB query to delete all decoration_spans entries,
     * forcing a re-scan of all notes on next access.
     */
    async invalidateAll(): Promise<void> {
        currentEntityVersion++;
        console.log(`[DecorationCache] Entity version → ${currentEntityVersion}, invalidating all spans...`);

        if (isTauri()) {
            try {
                const count = await invalidateAllDecorations();
                console.log(`[DecorationCache] Cleared ${count} cached entries from CozoDB`);
            } catch (error) {
                console.warn('[DecorationCache] Failed to invalidate CozoDB cache:', error);
            }
        }
    }

    /**
     * Get current entity version
     */
    getEntityVersion(): number {
        return currentEntityVersion;
    }

    /**
     * Clean up stale entries - NO-OP
     * 
     * CozoDB handles cleanup via content hash validation.
     */
    async cleanup(_olderThanDays: number = 7): Promise<number> {
        return 0;
    }
}

export const decorationCache = new DecorationCache();
