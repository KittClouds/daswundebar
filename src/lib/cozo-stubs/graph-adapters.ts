/**
 * CozoDB Graph Adapters Stub - DEPRECATED
 * 
 * These adapters have been replaced by:
 * - lib/tauri/smart-graph-registry.ts (entityRegistry replacement)
 * - Rust-side relationship handling
 */

import type { EntityKind } from '@/lib/types/entityTypes';

// Stub RegisteredEntity type
export interface RegisteredEntity {
    id: string;
    label: string;
    kind: EntityKind;
    aliases: string[];
    totalMentions: number;
    mentionsByNote: Map<string, number>;
}

// Stub entityRegistry - consumers should use smartGraphRegistry
export const entityRegistry = {
    init: async () => {
        console.warn('[entityRegistry STUB] init() called - Browser CozoDB removed. Use smartGraphRegistry from @/lib/tauri instead');
    },
    isReady: () => false,
    getAll: (): RegisteredEntity[] => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return [];
    },
    getAllEntities: (): RegisteredEntity[] => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return [];
    },
    getById: (_id: string): RegisteredEntity | null => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return null;
    },
    getEntityById: (_id: string): RegisteredEntity | null => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return null;
    },
    findByLabel: (_label: string): RegisteredEntity | null => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return null;
    },
    findEntity: (_label: string): RegisteredEntity | null => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return null;
    },
    findEntityByLabel: (_label: string): RegisteredEntity | null => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return null;
    },
    getEntitiesByKind: (_kind: EntityKind): RegisteredEntity[] => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return [];
    },
    register: async () => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return null;
    },
    registerEntity: async (_label: string, _kind: EntityKind, _noteId: string, _options?: any) => {
        console.warn('[entityRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return { id: '', label: _label, kind: _kind, aliases: [], totalMentions: 0, mentionsByNote: new Map() };
    },
};

// Stub relationshipRegistry
export const relationshipRegistry = {
    init: async () => {
        console.warn('[relationshipRegistry STUB] init() called - Browser CozoDB removed. Use Tauri graph-registry instead');
    },
    register: () => {
        console.warn('[relationshipRegistry STUB] Use Tauri graph-registry instead');
        return null;
    },
    add: () => {
        console.warn('[relationshipRegistry STUB] Use Tauri graph-registry instead');
        return null;
    },
    addBatch: (_inputs: any[]) => {
        console.warn('[relationshipRegistry STUB] Use Tauri graph-registry instead');
        return 0;
    },
    addWithoutPersist: (_rel: any) => {
        console.warn('[relationshipRegistry STUB] Use Tauri graph-registry instead');
    },
    getAll: () => [],
    get: (_id: string) => null,
    getByEntity: (_entityId: string) => [],
    getByType: (_type: string) => [],
    query: (_params: any) => [],
    getByEntityId: () => [],
    delete: (_id: string) => false,
};

// Re-export RelationshipSource from types
export { RelationshipSource } from '@/lib/relationships/types';
