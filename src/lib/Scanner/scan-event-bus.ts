/**
 * ScanEventBus - Central event bus for scan results
 * 
 * Single source of truth for scan result distribution.
 * RustHighlighter emits, all consumers subscribe.
 * 
 * @module lib/Scanner/scan-event-bus
 */

import type { ConductorScanResult } from '@/lib/tauri/bridge';

/**
 * Scan event payload
 */
export interface ScanEvent {
    noteId: string;
    result: ConductorScanResult;
    timestamp: number;
}

/**
 * Event handler type
 */
export type ScanEventHandler = (event: ScanEvent) => void;

/**
 * ScanEventBus - Broadcasts scan results to all subscribers
 * 
 * Design:
 * - One producer (RustHighlighter)
 * - Many consumers (LinkIndex, ExtractorFacade, etc.)
 * - Caches last result per noteId for late subscribers
 * - Isolates errors between handlers
 */
export class ScanEventBus {
    private handlers: Set<ScanEventHandler> = new Set();
    private cache: Map<string, ScanEvent> = new Map();

    /**
     * Subscribe to scan events
     * @returns Unsubscribe function
     */
    subscribe(handler: ScanEventHandler): () => void {
        this.handlers.add(handler);

        return () => {
            this.handlers.delete(handler);
        };
    }

    /**
     * Emit a scan event to all subscribers
     * Errors in one handler don't affect others
     */
    emit(event: ScanEvent): void {
        // Cache the result
        this.cache.set(event.noteId, event);

        // Notify all handlers
        for (const handler of this.handlers) {
            try {
                handler(event);
            } catch (error) {
                console.error('[ScanEventBus] Handler error:', error);
            }
        }
    }

    /**
     * Get the last cached result for a noteId
     */
    getLastResult(noteId: string): ScanEvent | undefined {
        return this.cache.get(noteId);
    }

    /**
     * Get current subscriber count (useful for debugging)
     */
    get subscriberCount(): number {
        return this.handlers.size;
    }

    /**
     * Clear all cached results (useful for testing)
     */
    clearCache(): void {
        this.cache.clear();
    }
}

/**
 * Singleton instance
 */
export const scanEventBus = new ScanEventBus();
