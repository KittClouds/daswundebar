/**
 * Network Storage Adapter
 * 
 * Routes network storage calls to either:
 * - SurrealDB via Tauri (new)
 * - IndexedDB (legacy, deprecated)
 * 
 * This provides backward compatibility for existing consumers.
 */

import { isTauri } from '@/lib/tauri';
import { networkBridge, type SurrealNetwork } from '@/lib/tauri/network-bridge';
import type {
    NetworkInstance,
    NetworkSchema,
    NetworkRelationshipInstance,
    NetworkStats
} from './types';
import { BUILTIN_SCHEMAS, getSchemaById } from './schemas';

// Default world ID
const DEFAULT_WORLD = 'default';

// ============================================================================
// CONVERSION HELPERS
// ============================================================================

function surrealNetworkToInstance(surreal: SurrealNetwork): NetworkInstance {
    const extractId = (id: string | { tb: string; id: { String: string } }): string => {
        if (typeof id === 'string') return id;
        return id.id?.String || '';
    };

    return {
        id: extractId(surreal.id),
        name: surreal.name,
        schemaId: surreal.schema_id,
        rootFolderId: surreal.root_folder_id || '',
        rootEntityId: surreal.root_entity_id ?? undefined,
        entityIds: [], // Members loaded separately
        namespace: surreal.namespace,
        description: surreal.description ?? undefined,
        tags: surreal.tags ?? undefined,
        stats: {
            memberCount: surreal.member_count,
            relationshipCount: surreal.relationship_count,
            maxDepth: surreal.max_depth,
        },
        createdAt: surreal.created_at ? new Date(surreal.created_at) : new Date(),
        updatedAt: surreal.updated_at ? new Date(surreal.updated_at) : new Date(),
    };
}

// ============================================================================
// NETWORK INSTANCE OPERATIONS
// ============================================================================

/**
 * Save a network instance
 */
export async function saveNetworkInstance(network: NetworkInstance): Promise<void> {
    if (!isTauri()) {
        console.warn('[NetworkStorageAdapter] saveNetworkInstance: Tauri not available');
        return;
    }

    // Check if exists
    const existing = await networkBridge.getNetwork(network.id);
    if (existing) {
        await networkBridge.updateNetwork(network.id, {
            name: network.name,
            description: network.description,
            tags: network.tags,
        });
    } else {
        await networkBridge.createNetwork(network.name, network.schemaId, {
            rootFolderId: network.rootFolderId,
            rootEntityId: network.rootEntityId,
            namespace: network.namespace,
            description: network.description,
            tags: network.tags,
        });
    }
}

/**
 * Load a network instance by ID
 */
export async function loadNetworkInstance(id: string): Promise<NetworkInstance | null> {
    if (!isTauri()) {
        console.warn('[NetworkStorageAdapter] loadNetworkInstance: Tauri not available');
        return null;
    }

    const surreal = await networkBridge.getNetwork(id);
    return surreal ? surrealNetworkToInstance(surreal) : null;
}

/**
 * Load network by folder ID
 */
export async function loadNetworkByFolderId(folderId: string): Promise<NetworkInstance | null> {
    if (!isTauri()) {
        console.warn('[NetworkStorageAdapter] loadNetworkByFolderId: Tauri not available');
        return null;
    }

    const surreal = await networkBridge.getNetworkByFolder(folderId);
    return surreal ? surrealNetworkToInstance(surreal) : null;
}

/**
 * Load all networks
 */
export async function loadAllNetworks(): Promise<NetworkInstance[]> {
    if (!isTauri()) {
        console.warn('[NetworkStorageAdapter] loadAllNetworks: Tauri not available');
        return [];
    }

    const networks = await networkBridge.listNetworks();
    return networks.map(surrealNetworkToInstance);
}

/**
 * Load networks by namespace
 */
export async function loadNetworksByNamespace(namespace: string): Promise<NetworkInstance[]> {
    const all = await loadAllNetworks();
    return all.filter(n => n.namespace === namespace);
}

/**
 * Load networks by schema ID
 */
export async function loadNetworksBySchemaId(schemaId: string): Promise<NetworkInstance[]> {
    const all = await loadAllNetworks();
    return all.filter(n => n.schemaId === schemaId);
}

/**
 * Delete a network instance
 */
export async function deleteNetworkInstance(id: string): Promise<void> {
    if (!isTauri()) {
        console.warn('[NetworkStorageAdapter] deleteNetworkInstance: Tauri not available');
        return;
    }

    await networkBridge.deleteNetwork(id);
}

/**
 * Update network instance
 */
export async function updateNetworkInstance(
    id: string,
    updates: Partial<NetworkInstance>
): Promise<NetworkInstance | null> {
    if (!isTauri()) {
        console.warn('[NetworkStorageAdapter] updateNetworkInstance: Tauri not available');
        return null;
    }

    await networkBridge.updateNetwork(id, {
        name: updates.name,
        description: updates.description,
        tags: updates.tags,
    });

    return loadNetworkInstance(id);
}

// ============================================================================
// SCHEMA OPERATIONS (Use built-in schemas, custom via SurrealDB TODO)
// ============================================================================

/**
 * Save a custom schema
 * @deprecated Custom schemas should be added to SurrealDB
 */
export async function saveNetworkSchema(_schema: NetworkSchema): Promise<void> {
    console.warn('[NetworkStorageAdapter] saveNetworkSchema: Custom schemas not yet supported in SurrealDB');
}

/**
 * Load a schema by ID
 * Uses built-in schemas, falls back to SurrealDB for custom
 */
export async function loadNetworkSchema(id: string): Promise<NetworkSchema | null> {
    return getSchemaById(id) || null;
}

/**
 * Load all custom schemas
 */
export async function loadAllCustomSchemas(): Promise<NetworkSchema[]> {
    // Return empty - custom schemas not yet in SurrealDB
    return [];
}

/**
 * Delete a custom schema
 * @deprecated Custom schemas should be managed in SurrealDB
 */
export async function deleteNetworkSchema(_id: string): Promise<void> {
    console.warn('[NetworkStorageAdapter] deleteNetworkSchema: Custom schemas not yet supported in SurrealDB');
}

// ============================================================================
// RELATIONSHIP OPERATIONS
// ============================================================================

/**
 * Save a network relationship
 */
export async function saveNetworkRelationship(relationship: NetworkRelationshipInstance): Promise<void> {
    if (!isTauri()) {
        console.warn('[NetworkStorageAdapter] saveNetworkRelationship: Tauri not available');
        return;
    }

    await networkBridge.createRelationship(
        relationship.sourceEntityId,
        relationship.targetEntityId,
        relationship.relationshipCode,
        {
            networkId: relationship.networkId,
            strength: relationship.strength,
            startDate: relationship.startDate?.toISOString(),
            endDate: relationship.endDate?.toISOString(),
            notes: relationship.notes,
            attributes: relationship.attributes as Record<string, unknown> | undefined,
        }
    );
}

/**
 * Load relationships by network
 */
export async function loadRelationshipsByNetwork(networkId: string): Promise<NetworkRelationshipInstance[]> {
    if (!isTauri()) {
        return [];
    }

    const relationships = await networkBridge.getNetworkRelationships(networkId);
    return relationships.map(r => {
        const extractId = (id: string | { tb: string; id: { String: string } }): string => {
            if (typeof id === 'string') return id;
            return id.id?.String || '';
        };

        return {
            id: extractId(r.id),
            networkId: r.network_id || networkId,
            relationshipCode: r.relationship_code,
            sourceEntityId: r.out,
            targetEntityId: r.in_,
            strength: r.strength ?? 1.0,
            notes: r.notes ?? undefined,
            attributes: r.attributes ?? undefined,
            createdAt: r.created_at ? new Date(r.created_at) : new Date(),
            updatedAt: new Date(),
        };
    });
}

/**
 * Delete a relationship
 */
export async function deleteNetworkRelationship(id: string): Promise<void> {
    if (!isTauri()) {
        return;
    }

    await networkBridge.deleteRelationship(id);
}

/**
 * Delete all relationships for a network
 */
export async function deleteRelationshipsByNetwork(networkId: string): Promise<void> {
    if (!isTauri()) {
        return;
    }

    const relationships = await networkBridge.getNetworkRelationships(networkId);
    for (const r of relationships) {
        const id = typeof r.id === 'string' ? r.id : r.id.id?.String;
        if (id) {
            await networkBridge.deleteRelationship(id);
        }
    }
}

// ============================================================================
// ENTITY MEMBERSHIP
// ============================================================================

/**
 * Add entity to network
 */
export async function addEntityToNetwork(
    networkId: string,
    entityId: string,
    options: { role?: string; depthLevel?: number; groupId?: string } = {}
): Promise<void> {
    if (!isTauri()) return;
    await networkBridge.addMember(networkId, entityId, options);
}

/**
 * Remove entity from network
 */
export async function removeEntityFromNetwork(networkId: string, entityId: string): Promise<void> {
    if (!isTauri()) return;
    await networkBridge.removeMember(networkId, entityId);
}

/**
 * Get network members
 */
export async function getNetworkMembers(networkId: string): Promise<string[]> {
    if (!isTauri()) return [];
    const members = await networkBridge.getMembers(networkId);
    return members.map(m => {
        const id = m.entity?.id;
        if (!id) return '';
        if (typeof id === 'string') return id;
        return id.id?.String || '';
    }).filter(Boolean);
}

// ============================================================================
// DATABASE LIFECYCLE (No-op for Tauri, handled by backend)
// ============================================================================

/**
 * Get or create the network database
 * @deprecated Database managed by Tauri backend
 */
export async function getNetworkDB(): Promise<null> {
    console.warn('[NetworkStorageAdapter] getNetworkDB: Database managed by Tauri backend');
    return null;
}

/**
 * Close the database connection
 * @deprecated Database managed by Tauri backend
 */
export async function closeNetworkDB(): Promise<void> {
    // No-op - Tauri manages connections
}
