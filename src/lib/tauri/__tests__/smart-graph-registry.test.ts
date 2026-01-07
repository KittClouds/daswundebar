/**
 * SmartGraphRegistry Unit Tests
 * 
 * Pure logic tests focused on the registry behavior.
 * Each test creates a fresh registry instance for isolation.
 */

import { describe, it, expect, vi } from 'vitest';

// Mock Tauri invoke with deterministic responses
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn(async (cmd: string, args?: any) => {
        switch (cmd) {
            case 'graph_register_node':
                // Return a response that matches RustNodeResponse
                return {
                    id: `node_${args?.request?.label?.toLowerCase().replace(/\s+/g, '_') || 'unknown'}`,
                    label: args?.request?.label || 'Unknown',
                    kind: args?.request?.kind || 'CONCEPT',
                    mention_count: 1,
                    aliases: args?.request?.aliases || [],
                    is_new: true,
                };
            case 'graph_get_all_entities':
                return []; // Start with empty entities
            case 'graph_get_entities_for_hydration':
                return { entity_count: 0, needs_hydration: false, entities: null };
            case 'graph_ingest_scan_result':
                return { entities_created: 0, entities_updated: 0, edges_created: 0, edges_updated: 0 };
            case 'graph_create_edge':
                return {
                    id: 'edge_1',
                    source_id: args?.request?.source_id || '',
                    target_id: args?.request?.target_id || '',
                    edge_type: args?.request?.edge_type || 'RELATED',
                    confidence: args?.request?.confidence ?? 1.0,
                };
            default:
                console.log(`[Mock] Unhandled command: ${cmd}`);
                return null;
        }
    }),
}));

// Import after mock
import { SmartGraphRegistry } from '../smart-graph-registry';

describe('SmartGraphRegistry', () => {

    describe('Basic Initialization', () => {
        it('should become ready after init', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();
            expect(registry.isReady()).toBe(true);
        });
    });

    describe('Entity Registration', () => {
        it('should register entity and return result', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();

            const result = await registry.registerEntity('Gandalf', 'CHARACTER' as any, 'note');

            expect(result.entity.label).toBe('Gandalf');
            expect(result.entity.kind).toBe('CHARACTER');
            expect(result.isNew).toBe(true);
        });

        it('should return cached entity on duplicate registration', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();

            const first = await registry.registerEntity('Frodo', 'CHARACTER' as any, 'note1');
            const second = await registry.registerEntity('Frodo', 'CHARACTER' as any, 'note2');

            expect(first.isNew).toBe(true);
            expect(second.isNew).toBe(false);
            expect(first.entity.id).toBe(second.entity.id);
        });
    });

    describe('Entity Lookup by Label', () => {
        it('should find by exact label', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();
            await registry.registerEntity('Gandalf', 'CHARACTER' as any, 'note');

            const found = registry.findEntityByLabel('Gandalf');
            expect(found).not.toBeNull();
            expect(found?.label).toBe('Gandalf');
        });

        it('should find by label case-insensitive (lowercase)', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();
            await registry.registerEntity('Gandalf', 'CHARACTER' as any, 'note');

            expect(registry.findEntityByLabel('gandalf')?.label).toBe('Gandalf');
        });

        it('should find by label case-insensitive (uppercase)', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();
            await registry.registerEntity('Gandalf', 'CHARACTER' as any, 'note');

            expect(registry.findEntityByLabel('GANDALF')?.label).toBe('Gandalf');
        });

        it('should return null for unknown label', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();

            expect(registry.findEntityByLabel('Unknown')).toBeNull();
        });
    });

    describe('Get All Entities', () => {
        it('should return empty array when no entities', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();

            expect(registry.getAllEntities()).toEqual([]);
        });

        it('should return registered entities', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();

            await registry.registerEntity('A', 'CONCEPT' as any, 'note');
            await registry.registerEntity('B', 'CONCEPT' as any, 'note');

            const all = registry.getAllEntities();
            expect(all.length).toBe(2);
            expect(all.map(e => e.label).sort()).toEqual(['A', 'B']);
        });
    });

    describe('Filter by Kind', () => {
        it('should filter entities by kind', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();

            await registry.registerEntity('Char', 'CHARACTER' as any, 'note');
            await registry.registerEntity('Loc', 'LOCATION' as any, 'note');

            const characters = registry.getEntitiesByKind('CHARACTER' as any);
            expect(characters.length).toBe(1);
            expect(characters[0].label).toBe('Char');

            const locations = registry.getEntitiesByKind('LOCATION' as any);
            expect(locations.length).toBe(1);
            expect(locations[0].label).toBe('Loc');
        });
    });

    describe('Scan Ingestion', () => {
        it('should return ingestion result', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();

            const result = await registry.ingestScanResult(
                'note',
                [{ entity_label: 'A', entity_kind: 'CONCEPT' }],
                []
            );

            expect(result).toHaveProperty('entities_created');
            expect(result).toHaveProperty('edges_created');
        });
    });

    describe('Edge Creation', () => {
        it('should create edge and return it', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();

            const a = await registry.registerEntity('A', 'CONCEPT' as any, 'note');
            const b = await registry.registerEntity('B', 'CONCEPT' as any, 'note');

            const edge = await registry.createEdge(a.entity.id, b.entity.id, 'RELATED', { confidence: 0.9 });

            expect(edge).not.toBeNull();
            expect(edge.type).toBe('RELATED');
        });
    });

    describe('Hydration', () => {
        it('should return null when no hydration needed', async () => {
            const registry = new SmartGraphRegistry();
            await registry.init();

            const entities = await registry.getEntitiesForHydration();
            expect(entities).toBeNull();
        });
    });
});
