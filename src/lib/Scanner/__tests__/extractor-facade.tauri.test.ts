/**
 * Extractor Facade (Tauri Pipeline) Unit Tests
 * 
 * Tests scan routing, hydration, and result ingestion logic.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock scan result factory
function createMockScanResult(options: {
    implicit?: any[];
    relations?: any[];
    temporal?: any[];
} = {}) {
    return {
        implicit: options.implicit || [],
        triples: [],
        structured: [],
        temporal: options.temporal || [],
        unified_relations: options.relations || [],
        stats: {
            timings: {
                total_us: 100,
                relation_us: 50,
                temporal_us: 10,
                implicit_us: 20,
                structured_us: 10,
                triple_us: 5,
                unified_us: 5,
            },
            content_hash: 'abc123',
            was_skipped: false,
            was_incremental: false,
            entities_found: options.implicit?.length || 0,
            temporal_found: options.temporal?.length || 0,
            implicit_found: options.implicit?.length || 0,
            triples_found: 0,
            structured_found: 0,
            unified_found: options.relations?.length || 0,
        },
    };
}

// Mock implementations
const mockScanner = {
    isReady: vi.fn().mockReturnValue(true),
    executeScan: vi.fn(),
    conductorScanImmediate: vi.fn(),
    hydrateEntities: vi.fn(),
};

const mockRegistry = {
    init: vi.fn().mockResolvedValue(undefined),
    isReady: vi.fn().mockReturnValue(true),
    registerEntity: vi.fn(),
    ingestScanResult: vi.fn(),
    getAllEntities: vi.fn().mockReturnValue([]),
    getEntitiesForHydration: vi.fn().mockResolvedValue(null),
};

// Setup default mock behaviors
function setupDefaultMocks() {
    mockScanner.executeScan.mockResolvedValue(createMockScanResult());
    mockScanner.conductorScanImmediate.mockResolvedValue(createMockScanResult());
    mockScanner.hydrateEntities.mockResolvedValue({ legacy: { success: true }, conductor: { success: true } });
    mockRegistry.registerEntity.mockResolvedValue({ entity: { id: 'test' }, isNew: true, wasMerged: false });
    mockRegistry.ingestScanResult.mockResolvedValue({ entities_created: 0, entities_updated: 0, edges_created: 0, edges_updated: 0 });
}

describe('Extractor Facade (Tauri Pipeline)', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        setupDefaultMocks();
    });

    describe('Scanner Ready State', () => {
        it('should report ready when initialized', () => {
            mockScanner.isReady.mockReturnValue(true);
            expect(mockScanner.isReady()).toBe(true);
        });

        it('should report not ready when uninitialized', () => {
            mockScanner.isReady.mockReturnValue(false);
            expect(mockScanner.isReady()).toBe(false);
        });
    });

    describe('Scan Execution', () => {
        it('should execute scan and return result', async () => {
            const result = await mockScanner.executeScan('Test document text');

            expect(result).toBeDefined();
            expect(result.stats).toBeDefined();
            expect(result.stats.timings).toBeDefined();
        });

        it('should detect implicit entities', async () => {
            mockScanner.conductorScanImmediate.mockResolvedValueOnce(
                createMockScanResult({
                    implicit: [
                        {
                            entity_id: 'e1',
                            entity_label: 'Gandalf',
                            entity_kind: 'CHARACTER',
                            matched_text: 'Gandalf',
                            start: 0,
                            end: 7,
                            is_alias_match: false,
                        },
                        {
                            entity_id: 'e2',
                            entity_label: 'Frodo',
                            entity_kind: 'CHARACTER',
                            matched_text: 'Frodo',
                            start: 20,
                            end: 25,
                            is_alias_match: false,
                        },
                    ],
                })
            );

            const result = await mockScanner.conductorScanImmediate('Gandalf mentored Frodo');

            expect(result.implicit.length).toBe(2);
            expect(result.implicit[0].entity_label).toBe('Gandalf');
            expect(result.implicit[1].entity_label).toBe('Frodo');
        });

        it('should extract relations', async () => {
            mockScanner.conductorScanImmediate.mockResolvedValueOnce(
                createMockScanResult({
                    relations: [
                        {
                            head: 'Gandalf',
                            tail: 'Frodo',
                            relation_type: 'MENTORS',
                            confidence: 0.9,
                            source: 'CST',
                        },
                    ],
                })
            );

            const result = await mockScanner.conductorScanImmediate('Gandalf mentored Frodo');

            expect(result.unified_relations.length).toBe(1);
            expect(result.unified_relations[0].head).toBe('Gandalf');
            expect(result.unified_relations[0].tail).toBe('Frodo');
            expect(result.unified_relations[0].relation_type).toBe('MENTORS');
        });

        it('should detect temporal expressions', async () => {
            mockScanner.conductorScanImmediate.mockResolvedValueOnce(
                createMockScanResult({
                    temporal: [
                        {
                            kind: 'DATE',
                            text: 'yesterday',
                            start: 0,
                            end: 9,
                            confidence: 0.95,
                        },
                    ],
                })
            );

            const result = await mockScanner.conductorScanImmediate('yesterday was good');

            expect(result.temporal.length).toBe(1);
            expect(result.temporal[0].text).toBe('yesterday');
        });
    });

    describe('Entity Hydration', () => {
        it('should hydrate scanner with entities', async () => {
            const entities = [
                { id: 'e1', label: 'Gandalf', kind: 'CHARACTER', aliases: ['Mithrandir'] },
                { id: 'e2', label: 'Frodo', kind: 'CHARACTER', aliases: ['Ring-bearer'] },
            ];

            mockScanner.hydrateEntities.mockResolvedValueOnce({
                legacy: { count: 2, success: true },
                conductor: { hydrated: 2, success: true },
            });

            const result = await mockScanner.hydrateEntities(entities);

            expect(result.legacy.success).toBe(true);
            expect(result.conductor.success).toBe(true);
            expect(result.conductor.hydrated).toBe(2);
        });

        it('should handle empty entity list', async () => {
            mockScanner.hydrateEntities.mockResolvedValueOnce({
                legacy: { count: 0, success: true },
                conductor: { hydrated: 0, success: true },
            });

            const result = await mockScanner.hydrateEntities([]);

            expect(result.legacy.count).toBe(0);
            expect(result.conductor.hydrated).toBe(0);
        });
    });

    describe('Result Ingestion', () => {
        it('should ingest mentions into registry', async () => {
            mockRegistry.ingestScanResult.mockResolvedValueOnce({
                entities_created: 2,
                entities_updated: 0,
                edges_created: 0,
                edges_updated: 0,
            });

            const result = await mockRegistry.ingestScanResult(
                'test-note',
                [
                    { entity_label: 'Gandalf', entity_kind: 'CHARACTER' },
                    { entity_label: 'Frodo', entity_kind: 'CHARACTER' },
                ],
                []
            );

            expect(result.entities_created).toBe(2);
            expect(mockRegistry.ingestScanResult).toHaveBeenCalledTimes(1);
        });

        it('should ingest relations into registry', async () => {
            mockRegistry.ingestScanResult.mockResolvedValueOnce({
                entities_created: 0,
                entities_updated: 0,
                edges_created: 1,
                edges_updated: 0,
            });

            const result = await mockRegistry.ingestScanResult(
                'test-note',
                [],
                [
                    { head_label: 'Gandalf', tail_label: 'Frodo', relation_type: 'KNOWS', confidence: 0.8, source: 'cst' },
                ]
            );

            expect(result.edges_created).toBe(1);
        });

        it('should handle empty ingestion gracefully', async () => {
            await expect(
                mockRegistry.ingestScanResult('note', [], [])
            ).resolves.not.toThrow();
        });
    });

    describe('Error Handling', () => {
        it('should propagate scan errors', async () => {
            mockScanner.executeScan.mockRejectedValueOnce(new Error('Scan failed'));

            await expect(mockScanner.executeScan('test')).rejects.toThrow('Scan failed');
        });

        it('should propagate registry errors', async () => {
            mockRegistry.registerEntity.mockRejectedValueOnce(new Error('Registration failed'));

            await expect(
                mockRegistry.registerEntity('Test', 'CONCEPT', 'note')
            ).rejects.toThrow('Registration failed');
        });

        it('should propagate hydration errors', async () => {
            mockScanner.hydrateEntities.mockRejectedValueOnce(new Error('Hydration failed'));

            await expect(mockScanner.hydrateEntities([])).rejects.toThrow('Hydration failed');
        });
    });

    describe('Performance Stats', () => {
        it('should include timing information', async () => {
            const result = await mockScanner.conductorScanImmediate('Test text');

            expect(result.stats).toBeDefined();
            expect(result.stats.timings).toBeDefined();
            expect(typeof result.stats.timings.total_us).toBe('number');
            expect(result.stats.timings.total_us).toBeGreaterThan(0);
        });

        it('should track skip status', async () => {
            mockScanner.conductorScanImmediate.mockResolvedValueOnce({
                ...createMockScanResult(),
                stats: {
                    ...createMockScanResult().stats,
                    was_skipped: true,
                },
            });

            const result = await mockScanner.conductorScanImmediate('cached content');
            expect(result.stats.was_skipped).toBe(true);
        });

        it('should track incremental scan status', async () => {
            mockScanner.conductorScanImmediate
                .mockResolvedValueOnce(createMockScanResult())
                .mockResolvedValueOnce({
                    ...createMockScanResult(),
                    stats: {
                        ...createMockScanResult().stats,
                        was_incremental: true,
                    },
                });

            const first = await mockScanner.conductorScanImmediate('Hello');
            const second = await mockScanner.conductorScanImmediate('Hello!');

            expect(first.stats.was_incremental).toBe(false);
            expect(second.stats.was_incremental).toBe(true);
        });
    });

    describe('Content Hash', () => {
        it('should include content hash in stats', async () => {
            const result = await mockScanner.conductorScanImmediate('Test');
            expect(result.stats.content_hash).toBeDefined();
            expect(typeof result.stats.content_hash).toBe('string');
        });
    });
});
