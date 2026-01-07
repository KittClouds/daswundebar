/**
 * Tauri IPC Mock Utilities for Testing
 * 
 * Provides mock implementations of Tauri's invoke API
 * for unit testing without running the full Tauri app.
 */

import { vi } from 'vitest';

// =============================================================================
// Mock Response Types
// =============================================================================

export interface MockNodeResponse {
    id: string;
    label: string;
    kind: string;
    mention_count: number;
    aliases: string[];
    is_new: boolean;
}

export interface MockHydrateResponse {
    entity_count: number;
    needs_hydration: boolean;
    entities: Array<{
        id: string;
        label: string;
        kind: string;
        aliases: string[];
    }> | null;
}

export interface MockIngestResponse {
    entities_created: number;
    entities_updated: number;
    edges_created: number;
    edges_updated: number;
}

export interface MockEdgeResponse {
    id: string;
    source_id: string;
    target_id: string;
    edge_type: string;
    confidence: number;
}

// =============================================================================
// Mock Invoke Implementation
// =============================================================================

type InvokeHandler = (cmd: string, args?: any) => any;

let invokeHandlers: Map<string, InvokeHandler> = new Map();
let invokeCalls: Array<{ cmd: string; args?: any }> = [];

/**
 * Create a mock for @tauri-apps/api/core
 */
export function createTauriMock() {
    return {
        invoke: vi.fn((cmd: string, args?: any) => {
            invokeCalls.push({ cmd, args });

            const handler = invokeHandlers.get(cmd);
            if (handler) {
                return Promise.resolve(handler(cmd, args));
            }

            // Default responses
            switch (cmd) {
                case 'graph_register_node':
                    return Promise.resolve<MockNodeResponse>({
                        id: `node_${Date.now()}`,
                        label: args?.request?.label || 'Unknown',
                        kind: args?.request?.kind || 'CONCEPT',
                        mention_count: 1,
                        aliases: args?.request?.aliases || [],
                        is_new: true,
                    });

                case 'graph_get_entities_for_hydration':
                    return Promise.resolve<MockHydrateResponse>({
                        entity_count: 0,
                        needs_hydration: false,
                        entities: null,
                    });

                case 'graph_ingest_scan_result':
                    return Promise.resolve<MockIngestResponse>({
                        entities_created: args?.request?.mentions?.length || 0,
                        entities_updated: 0,
                        edges_created: args?.request?.relations?.length || 0,
                        edges_updated: 0,
                    });

                case 'graph_get_nodes':
                    return Promise.resolve<MockNodeResponse[]>([]);

                case 'graph_find_node':
                    return Promise.resolve<MockNodeResponse | null>(null);

                case 'graph_create_edge':
                    return Promise.resolve<MockEdgeResponse>({
                        id: `edge_${Date.now()}`,
                        source_id: args?.request?.source_id || '',
                        target_id: args?.request?.target_id || '',
                        edge_type: args?.request?.edge_type || 'RELATED',
                        confidence: args?.request?.confidence || 1.0,
                    });

                case 'conductor_scan':
                    return Promise.resolve(JSON.stringify({
                        implicit: [],
                        triples: [],
                        structured: [],
                        temporal: [],
                        unified_relations: [],
                        stats: {
                            timings: { total_us: 100 },
                            content_hash: 'test',
                            was_skipped: false,
                            was_incremental: false,
                        },
                    }));

                case 'conductor_hydrate':
                    return Promise.resolve(JSON.stringify({ hydrated: 0 }));

                default:
                    console.warn(`[TauriMock] Unhandled command: ${cmd}`);
                    return Promise.resolve(null);
            }
        }),
    };
}

/**
 * Register a custom handler for a specific command
 */
export function setInvokeHandler(cmd: string, handler: InvokeHandler) {
    invokeHandlers.set(cmd, handler);
}

/**
 * Clear all registered handlers
 */
export function clearInvokeHandlers() {
    invokeHandlers.clear();
}

/**
 * Get recorded invoke calls
 */
export function getInvokeCalls() {
    return [...invokeCalls];
}

/**
 * Clear recorded invoke calls
 */
export function clearInvokeCalls() {
    invokeCalls = [];
}

/**
 * Reset all mock state
 */
export function resetTauriMock() {
    clearInvokeHandlers();
    clearInvokeCalls();
}

// =============================================================================
// Test Fixtures
// =============================================================================

export function createMockEntity(label: string, kind: string = 'CHARACTER') {
    return {
        id: `entity_${label.toLowerCase().replace(/\s+/g, '_')}`,
        label,
        kind,
        aliases: [],
        firstNote: 'test-note-1',
        mentionsByNote: new Map<string, number>(),
        totalMentions: 1,
        lastSeenDate: new Date(),
        createdAt: new Date(),
        createdBy: 'user' as const,
    };
}

export function createMockRelation(head: string, tail: string, type: string = 'RELATED') {
    return {
        head_label: head,
        tail_label: tail,
        relation_type: type,
        confidence: 0.9,
        source: 'extraction',
    };
}

export function createMockScanResult(options: {
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
