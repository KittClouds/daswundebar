/**
 * ScanEventBus Unit Tests (TDD)
 * 
 * Tests for the event bus that broadcasts scan results
 * from RustHighlighter to all consumers (LinkIndex, Extractor, etc.)
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ScanEventBus, type ScanEvent } from '../scan-event-bus';

// Mock ConductorScanResult
const createMockResult = (noteId: string, implicit = 5) => ({
    noteId,
    result: {
        implicit: Array(implicit).fill({ entity_id: 'e1', entity_label: 'Test', entity_kind: 'CHARACTER', matched_text: 'Test', start: 0, end: 4, is_alias_match: false }),
        triples: [],
        structured: [],
        temporal: [],
        unified_relations: [],
        stats: {
            timings: { total_us: 1000, relation_us: 100, temporal_us: 50, implicit_us: 200, structured_us: 150, triple_us: 100, unified_us: 300 },
            content_hash: 'abc123',
            was_skipped: false,
            was_incremental: false,
            entities_found: implicit,
            temporal_found: 0,
            implicit_found: implicit,
            triples_found: 0,
            structured_found: 0,
            unified_found: 0,
        },
    },
    timestamp: Date.now(),
} as ScanEvent);

describe('ScanEventBus', () => {
    let bus: ScanEventBus;

    beforeEach(() => {
        bus = new ScanEventBus();
    });

    // --- CORE FUNCTIONALITY ---

    describe('emit and subscribe', () => {
        it('should emit events to all subscribers', () => {
            const handler1 = vi.fn();
            const handler2 = vi.fn();
            const handler3 = vi.fn();

            bus.subscribe(handler1);
            bus.subscribe(handler2);
            bus.subscribe(handler3);

            const event = createMockResult('note-1');
            bus.emit(event);

            expect(handler1).toHaveBeenCalledWith(event);
            expect(handler2).toHaveBeenCalledWith(event);
            expect(handler3).toHaveBeenCalledWith(event);
            expect(handler1).toHaveBeenCalledTimes(1);
        });

        it('should not emit to unsubscribed handlers', () => {
            const handler1 = vi.fn();
            const handler2 = vi.fn();

            const unsub1 = bus.subscribe(handler1);
            bus.subscribe(handler2);

            // Unsubscribe handler1
            unsub1();

            const event = createMockResult('note-1');
            bus.emit(event);

            expect(handler1).not.toHaveBeenCalled();
            expect(handler2).toHaveBeenCalledWith(event);
        });

        it('should handle multiple unsubscribes gracefully', () => {
            const handler = vi.fn();
            const unsub = bus.subscribe(handler);

            // Double unsubscribe should not throw
            unsub();
            expect(() => unsub()).not.toThrow();
        });
    });

    // --- CACHING ---

    describe('result caching', () => {
        it('should cache last result per noteId', () => {
            const event = createMockResult('note-abc');
            bus.emit(event);

            const cached = bus.getLastResult('note-abc');
            expect(cached).toBeDefined();
            expect(cached?.noteId).toBe('note-abc');
        });

        it('should replace cached result on new emit', () => {
            const event1 = createMockResult('note-abc', 5);
            const event2 = createMockResult('note-abc', 10);

            bus.emit(event1);
            bus.emit(event2);

            const cached = bus.getLastResult('note-abc');
            expect(cached?.result.stats.implicit_found).toBe(10);
        });

        it('should maintain separate caches per noteId', () => {
            const event1 = createMockResult('note-a', 5);
            const event2 = createMockResult('note-b', 10);

            bus.emit(event1);
            bus.emit(event2);

            expect(bus.getLastResult('note-a')?.result.stats.implicit_found).toBe(5);
            expect(bus.getLastResult('note-b')?.result.stats.implicit_found).toBe(10);
        });

        it('should return undefined for unknown noteId', () => {
            expect(bus.getLastResult('nonexistent')).toBeUndefined();
        });
    });

    // --- ERROR ISOLATION ---

    describe('error isolation', () => {
        it('should isolate errors in one subscriber from others', () => {
            const errorHandler = vi.fn(() => {
                throw new Error('Handler error');
            });
            const safeHandler = vi.fn();

            bus.subscribe(errorHandler);
            bus.subscribe(safeHandler);

            const event = createMockResult('note-1');

            // Should not throw, should still call safeHandler
            expect(() => bus.emit(event)).not.toThrow();
            expect(safeHandler).toHaveBeenCalledWith(event);
        });

        it('should log errors from failing handlers', () => {
            const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => { });

            const errorHandler = vi.fn(() => {
                throw new Error('Test error');
            });

            bus.subscribe(errorHandler);
            bus.emit(createMockResult('note-1'));

            expect(consoleSpy).toHaveBeenCalled();
            consoleSpy.mockRestore();
        });
    });

    // --- SUBSCRIBER COUNT ---

    describe('subscriber management', () => {
        it('should track subscriber count', () => {
            expect(bus.subscriberCount).toBe(0);

            const unsub1 = bus.subscribe(vi.fn());
            expect(bus.subscriberCount).toBe(1);

            const unsub2 = bus.subscribe(vi.fn());
            expect(bus.subscriberCount).toBe(2);

            unsub1();
            expect(bus.subscriberCount).toBe(1);

            unsub2();
            expect(bus.subscriberCount).toBe(0);
        });
    });
});
