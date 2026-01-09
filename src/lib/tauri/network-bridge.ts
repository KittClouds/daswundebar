/**
 * Network Bridge (DEPRECATED)
 * 
 * This file is kept for backwards compatibility.
 * New code should use: import { contentAPI } from '@/lib/tauri/content-api'
 */

import { contentAPI, type CozoNetwork, type CozoEntity, type CozoRelationship } from './content-api';

// ============================================================================
// Types (kept for backwards compat)
// ============================================================================

export interface SurrealNetwork {
    id: string;
    world_id: string;
    name: string;
    schema_id: string;
    root_folder_id?: string | null;
    root_entity_id?: string | null;
    namespace: string;
    description?: string | null;
    tags?: string[] | null;
    member_count: number;
    relationship_count: number;
    max_depth: number;
    created_at?: string | null;
    updated_at?: string | null;
}

export interface SurrealEntity {
    id: string;
    world_id: string;
    label: string;
    entity_kind: string;
    entity_subtype?: string | null;
    note_id?: string | null;
    folder_id?: string | null;
    is_active: boolean;
    aliases?: string[] | null;
    attributes?: Record<string, unknown> | null;
    created_at?: string | null;
    updated_at?: string | null;
}

export interface SurrealRelationship {
    id: string;
    world_id: string;
    in_: string;
    out: string;
    network_id?: string | null;
    relationship_code: string;
    inverse_id?: string | null;
    strength: number;
    start_date?: string | null;
    end_date?: string | null;
    notes?: string | null;
    attributes?: Record<string, unknown> | null;
    created_at?: string | null;
}

export interface NetworkMemberSummary {
    entity: SurrealEntity;
    role: string;
    depth_level: number;
    group_id?: string | null;
    joined_at?: string | null;
}

export interface RelationshipSummary {
    id: string;
    source_id: string;
    source_label: string;
    target_id: string;
    target_label: string;
    relationship_code: string;
    strength: number;
    has_inverse: boolean;
}

// ============================================================================
// Adapters
// ============================================================================

function cozoToNetwork(c: CozoNetwork): SurrealNetwork {
    return {
        id: c.id,
        world_id: c.world_id,
        name: c.name,
        schema_id: c.schema_id,
        root_folder_id: c.root_folder_id,
        root_entity_id: c.root_entity_id,
        namespace: c.namespace,
        description: c.description,
        tags: c.tags,
        member_count: c.member_count,
        relationship_count: c.relationship_count,
        max_depth: c.max_depth,
        created_at: new Date(c.created_at * 1000).toISOString(),
        updated_at: new Date(c.updated_at * 1000).toISOString(),
    };
}

function cozoToEntity(c: CozoEntity): SurrealEntity {
    return {
        id: c.id,
        world_id: c.world_id,
        label: c.label,
        entity_kind: c.entity_kind,
        entity_subtype: c.entity_subtype,
        note_id: c.note_id,
        folder_id: c.folder_id,
        is_active: c.is_active,
        aliases: c.aliases,
        attributes: c.attributes,
        created_at: new Date(c.created_at * 1000).toISOString(),
        updated_at: new Date(c.updated_at * 1000).toISOString(),
    };
}

function cozoToRelationship(c: CozoRelationship): SurrealRelationship {
    return {
        id: c.id,
        world_id: c.world_id,
        in_: c.target_id,
        out: c.source_id,
        network_id: c.network_id,
        relationship_code: c.relationship_code,
        inverse_id: c.inverse_id,
        strength: c.strength,
        start_date: c.start_date ? new Date(c.start_date * 1000).toISOString() : null,
        end_date: c.end_date ? new Date(c.end_date * 1000).toISOString() : null,
        notes: c.notes,
        attributes: c.attributes,
        created_at: new Date(c.created_at * 1000).toISOString(),
    };
}

function extractId(id: string | { tb: string; id: { String: string } } | null | undefined): string {
    if (!id) return '';
    if (typeof id === 'string') return id;
    return id.id?.String || '';
}

// ============================================================================
// Network Operations
// ============================================================================

export async function createNetwork(params: {
    world_id: string;
    name: string;
    schema_id: string;
    root_folder_id?: string;
    root_entity_id?: string;
    namespace?: string;
    description?: string;
    tags?: string[];
}): Promise<SurrealNetwork> {
    const result = await contentAPI.createNetwork({
        name: params.name,
        schemaId: params.schema_id,
        rootFolderId: params.root_folder_id,
        rootEntityId: params.root_entity_id,
        namespace: params.namespace,
        description: params.description,
        tags: params.tags,
    });
    return cozoToNetwork(result);
}

export async function getNetwork(_world_id: string, id: string): Promise<SurrealNetwork | null> {
    const result = await contentAPI.getNetwork(id);
    return result ? cozoToNetwork(result) : null;
}

export async function listNetworks(_world_id: string): Promise<SurrealNetwork[]> {
    const results = await contentAPI.listNetworks();
    return results.map(cozoToNetwork);
}

export async function deleteNetwork(_world_id: string, id: string): Promise<void> {
    await contentAPI.deleteNetwork(id);
}

// ============================================================================
// Entity Operations
// ============================================================================

export async function createEntity(params: {
    world_id: string;
    label: string;
    entity_kind: string;
    entity_subtype?: string;
    note_id?: string;
    folder_id?: string;
    aliases?: string[];
    attributes?: Record<string, unknown>;
}): Promise<SurrealEntity> {
    const result = await contentAPI.createEntity({
        label: params.label,
        entityKind: params.entity_kind,
        entitySubtype: params.entity_subtype,
        noteId: params.note_id,
        folderId: params.folder_id,
        aliases: params.aliases,
        attributes: params.attributes,
    });
    return cozoToEntity(result);
}

export async function getEntity(_world_id: string, id: string): Promise<SurrealEntity | null> {
    const result = await contentAPI.getEntity(id);
    return result ? cozoToEntity(result) : null;
}

export async function listEntities(_world_id: string): Promise<SurrealEntity[]> {
    const results = await contentAPI.listEntities();
    return results.map(cozoToEntity);
}

export async function listEntitiesByKind(_world_id: string, entity_kind: string): Promise<SurrealEntity[]> {
    const results = await contentAPI.listEntitiesByKind(entity_kind);
    return results.map(cozoToEntity);
}

export async function deleteEntity(_world_id: string, id: string): Promise<void> {
    await contentAPI.deleteEntity(id);
}

// Stubs for now-unsupported operations
export async function getEntityByNote(_world_id: string, _note_id: string): Promise<SurrealEntity | null> {
    console.warn('[NetworkBridge] getEntityByNote not yet implemented in CozoDB backend');
    return null;
}

export async function getEntityByLabel(_world_id: string, _label: string): Promise<SurrealEntity | null> {
    console.warn('[NetworkBridge] getEntityByLabel not yet implemented in CozoDB backend');
    return null;
}

export async function updateEntity(_params: Record<string, unknown>): Promise<SurrealEntity> {
    console.warn('[NetworkBridge] updateEntity not yet implemented in CozoDB backend');
    throw new Error('updateEntity not implemented');
}

export async function searchEntities(_world_id: string, _query: string): Promise<SurrealEntity[]> {
    console.warn('[NetworkBridge] searchEntities not yet implemented in CozoDB backend');
    return [];
}

export async function linkEntityToNote(..._args: unknown[]): Promise<SurrealEntity> {
    console.warn('[NetworkBridge] linkEntityToNote not yet implemented in CozoDB backend');
    throw new Error('linkEntityToNote not implemented');
}

export async function linkEntityToFolder(..._args: unknown[]): Promise<SurrealEntity> {
    console.warn('[NetworkBridge] linkEntityToFolder not yet implemented in CozoDB backend');
    throw new Error('linkEntityToFolder not implemented');
}

// ============================================================================
// Relationship Operations
// ============================================================================

export async function createRelationship(params: {
    world_id: string;
    source_id: string;
    target_id: string;
    relationship_code: string;
    network_id?: string;
    strength?: number;
    start_date?: string;
    end_date?: string;
    notes?: string;
    attributes?: Record<string, unknown>;
}): Promise<SurrealRelationship> {
    const result = await contentAPI.createRelationship({
        sourceId: params.source_id,
        targetId: params.target_id,
        relationshipCode: params.relationship_code,
        networkId: params.network_id,
        strength: params.strength,
        startDate: params.start_date ? new Date(params.start_date).getTime() / 1000 : undefined,
        endDate: params.end_date ? new Date(params.end_date).getTime() / 1000 : undefined,
        notes: params.notes,
        attributes: params.attributes,
    });
    return cozoToRelationship(result);
}

export async function getRelationship(_world_id: string, id: string): Promise<SurrealRelationship | null> {
    const result = await contentAPI.getRelationship(id);
    return result ? cozoToRelationship(result) : null;
}

export async function getEntityRelationships(_world_id: string, entity_id: string): Promise<SurrealRelationship[]> {
    const results = await contentAPI.getEntityRelationships(entity_id);
    return results.map(cozoToRelationship);
}

export async function deleteRelationship(_world_id: string, id: string): Promise<void> {
    await contentAPI.deleteRelationship(id);
}

// Stubs for now-unsupported operations
export async function getNetworkRelationships(..._args: unknown[]): Promise<SurrealRelationship[]> {
    console.warn('[NetworkBridge] getNetworkRelationships not yet implemented');
    return [];
}

export async function getRelationshipsByCode(..._args: unknown[]): Promise<SurrealRelationship[]> {
    console.warn('[NetworkBridge] getRelationshipsByCode not yet implemented');
    return [];
}

export async function getNetworkRelationshipSummaries(..._args: unknown[]): Promise<RelationshipSummary[]> {
    console.warn('[NetworkBridge] getNetworkRelationshipSummaries not yet implemented');
    return [];
}

export async function deleteRelationshipsBetween(..._args: unknown[]): Promise<number> {
    console.warn('[NetworkBridge] deleteRelationshipsBetween not yet implemented');
    return 0;
}

// Network membership - stubs
export async function addNetworkMember(..._args: unknown[]): Promise<void> {
    console.warn('[NetworkBridge] addNetworkMember not yet implemented');
}

export async function removeNetworkMember(..._args: unknown[]): Promise<void> {
    console.warn('[NetworkBridge] removeNetworkMember not yet implemented');
}

export async function getNetworkMembers(..._args: unknown[]): Promise<NetworkMemberSummary[]> {
    console.warn('[NetworkBridge] getNetworkMembers not yet implemented');
    return [];
}

export async function updateNetwork(..._args: unknown[]): Promise<SurrealNetwork> {
    console.warn('[NetworkBridge] updateNetwork not yet implemented');
    throw new Error('updateNetwork not implemented');
}

export async function getNetworkByFolder(..._args: unknown[]): Promise<SurrealNetwork | null> {
    console.warn('[NetworkBridge] getNetworkByFolder not yet implemented');
    return null;
}

// ============================================================================
// Convenience Wrapper with Default World
// ============================================================================

const DEFAULT_WORLD = 'default';

export const networkBridge = {
    createNetwork: (name: string, schemaId: string, options: {
        rootFolderId?: string;
        rootEntityId?: string;
        namespace?: string;
        description?: string;
        tags?: string[];
    } = {}) => createNetwork({
        world_id: DEFAULT_WORLD,
        name,
        schema_id: schemaId,
        ...options,
    }),

    getNetwork: (id: string) => getNetwork(DEFAULT_WORLD, id),
    getNetworkByFolder: (folderId: string) => getNetworkByFolder(DEFAULT_WORLD, folderId),
    updateNetwork: (id: string, updates: { name?: string; description?: string; tags?: string[] }) =>
        updateNetwork({ world_id: DEFAULT_WORLD, id, ...updates }),
    deleteNetwork: (id: string) => deleteNetwork(DEFAULT_WORLD, id),
    listNetworks: () => listNetworks(DEFAULT_WORLD),

    addMember: (networkId: string, entityId: string, options = {}) =>
        addNetworkMember({ world_id: DEFAULT_WORLD, network_id: networkId, entity_id: entityId, ...options }),
    removeMember: (networkId: string, entityId: string) => removeNetworkMember(DEFAULT_WORLD, networkId, entityId),
    getMembers: (networkId: string) => getNetworkMembers(DEFAULT_WORLD, networkId),

    createEntity: (label: string, entityKind: string, options: {
        entitySubtype?: string;
        noteId?: string;
        folderId?: string;
        aliases?: string[];
        attributes?: Record<string, unknown>;
    } = {}) => createEntity({
        world_id: DEFAULT_WORLD,
        label,
        entity_kind: entityKind,
        entity_subtype: options.entitySubtype,
        note_id: options.noteId,
        folder_id: options.folderId,
        aliases: options.aliases,
        attributes: options.attributes,
    }),

    getEntity: (id: string) => getEntity(DEFAULT_WORLD, id),
    getEntityByNote: (noteId: string) => getEntityByNote(DEFAULT_WORLD, noteId),
    getEntityByLabel: (label: string) => getEntityByLabel(DEFAULT_WORLD, label),
    updateEntity: (id: string, updates: Record<string, unknown>) => updateEntity({ world_id: DEFAULT_WORLD, id, ...updates }),
    deleteEntity: (id: string) => deleteEntity(DEFAULT_WORLD, id),
    listEntitiesByKind: (entityKind: string) => listEntitiesByKind(DEFAULT_WORLD, entityKind),
    listEntities: () => listEntities(DEFAULT_WORLD),
    searchEntities: (query: string) => searchEntities(DEFAULT_WORLD, query),
    linkEntityToNote: (entityId: string, noteId: string) => linkEntityToNote(DEFAULT_WORLD, entityId, noteId),
    linkEntityToFolder: (entityId: string, folderId: string) => linkEntityToFolder(DEFAULT_WORLD, entityId, folderId),

    createRelationship: (sourceId: string, targetId: string, code: string, options: {
        networkId?: string;
        strength?: number;
        startDate?: string;
        endDate?: string;
        notes?: string;
        attributes?: Record<string, unknown>;
        createInverse?: boolean;
    } = {}) => createRelationship({
        world_id: DEFAULT_WORLD,
        source_id: sourceId,
        target_id: targetId,
        relationship_code: code,
        network_id: options.networkId,
        strength: options.strength,
        start_date: options.startDate,
        end_date: options.endDate,
        notes: options.notes,
        attributes: options.attributes,
    }),

    getRelationship: (id: string) => getRelationship(DEFAULT_WORLD, id),
    deleteRelationship: (id: string) => deleteRelationship(DEFAULT_WORLD, id),
    getEntityRelationships: (entityId: string) => getEntityRelationships(DEFAULT_WORLD, entityId),
    getNetworkRelationships: (networkId: string) => getNetworkRelationships(DEFAULT_WORLD, networkId),
    getRelationshipsByCode: (code: string) => getRelationshipsByCode(DEFAULT_WORLD, code),
    getNetworkRelationshipSummaries: (networkId: string) => getNetworkRelationshipSummaries(DEFAULT_WORLD, networkId),
    deleteRelationshipsBetween: (sourceId: string, targetId: string, code?: string) =>
        deleteRelationshipsBetween({ world_id: DEFAULT_WORLD, source_id: sourceId, target_id: targetId, relationship_code: code }),

    extractId,
};

export default networkBridge;
