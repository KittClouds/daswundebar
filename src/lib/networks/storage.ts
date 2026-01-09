/**
 * Network Storage Layer
 * 
 * IndexedDB-based persistence for network instances, schemas,
 * and relationship instances.
 * 
 * Database: network-db
 * Stores:
 *   - networks: NetworkInstance objects
 *   - schemas: Custom NetworkSchema objects (built-ins live in memory)
 *   - relationships: NetworkRelationshipInstance objects
 */

import { openDB, type IDBPDatabase, type DBSchema } from 'idb';
import { folderNetworkGraphSync } from '@/lib/cozo-stubs/sync';
import type {
    NetworkInstance,
    NetworkSchema,
    NetworkRelationshipInstance,
    NetworkStats
} from './types';
import type { NodeId } from '@/lib/graph/types';

// ===== DATABASE SCHEMA =====

interface NetworkDB extends DBSchema {
    networks: {
        key: string;
        value: NetworkInstanceRow;
        indexes: {
            'by-schema': string;
            'by-namespace': string;
            'by-folder': string;
        };
    };
    schemas: {
        key: string;
        value: NetworkSchemaRow;
        indexes: {
            'by-kind': string;
        };
    };
    relationships: {
        key: string;
        value: NetworkRelationshipRow;
        indexes: {
            'by-network': string;
            'by-source': string;
            'by-target': string;
            'by-code': string;
        };
    };
}

// Row types for IndexedDB (dates serialized as timestamps)
interface NetworkInstanceRow {
    id: string;
    name: string;
    schemaId: string;
    rootFolderId: string;
    rootEntityId?: string;
    entityIds: string[];
    namespace: string;
    description?: string;
    tags?: string[];
    stats?: NetworkStats;
    createdAt: number;
    updatedAt: number;
}

interface NetworkSchemaRow {
    id: string;
    name: string;
    kind: string;
    subtype?: string;
    description: string;
    allowedEntityKinds: string[];
    relationships: string; // JSON stringified
    isHierarchical: boolean;
    rootEntityKind?: string;
    maxDepth?: number;
    allowCycles: boolean;
    requireRootNode: boolean;
    isSystem: boolean;
    autoCreateInverse: boolean;
    icon?: string;
    color?: string;
    createdAt: number;
    updatedAt: number;
}

interface NetworkRelationshipRow {
    id: string;
    networkId: string;
    relationshipCode: string;
    sourceEntityId: string;
    targetEntityId: string;
    startDate?: number;
    endDate?: number;
    strength?: number;
    notes?: string;
    attributes?: string; // JSON stringified
    createdAt: number;
    updatedAt: number;
}

// ===== DATABASE SINGLETON =====

const DB_NAME = 'network-db';
const DB_VERSION = 1;

let dbInstance: IDBPDatabase<NetworkDB> | null = null;

/**
 * Get or create the network database
 */
export async function getNetworkDB(): Promise<IDBPDatabase<NetworkDB>> {
    if (dbInstance) return dbInstance;

    dbInstance = await openDB<NetworkDB>(DB_NAME, DB_VERSION, {
        upgrade(db, oldVersion, newVersion, transaction) {
            // Networks store
            if (!db.objectStoreNames.contains('networks')) {
                const networkStore = db.createObjectStore('networks', { keyPath: 'id' });
                networkStore.createIndex('by-schema', 'schemaId');
                networkStore.createIndex('by-namespace', 'namespace');
                networkStore.createIndex('by-folder', 'rootFolderId');
            }

            // Schemas store (for custom schemas)
            if (!db.objectStoreNames.contains('schemas')) {
                const schemaStore = db.createObjectStore('schemas', { keyPath: 'id' });
                schemaStore.createIndex('by-kind', 'kind');
            }

            // Relationships store
            if (!db.objectStoreNames.contains('relationships')) {
                const relStore = db.createObjectStore('relationships', { keyPath: 'id' });
                relStore.createIndex('by-network', 'networkId');
                relStore.createIndex('by-source', 'sourceEntityId');
                relStore.createIndex('by-target', 'targetEntityId');
                relStore.createIndex('by-code', 'relationshipCode');
            }
        },
    });

    return dbInstance;
}

/**
 * Close the database connection
 */
export async function closeNetworkDB(): Promise<void> {
    if (dbInstance) {
        dbInstance.close();
        dbInstance = null;
    }
}

// ===== NETWORK INSTANCE OPERATIONS =====

/**
 * Convert row to NetworkInstance
 */
function rowToNetworkInstance(row: NetworkInstanceRow): NetworkInstance {
    return {
        ...row,
        createdAt: new Date(row.createdAt),
        updatedAt: new Date(row.updatedAt),
        stats: row.stats ? {
            ...row.stats,
            lastUpdated: new Date(row.stats.lastUpdated),
        } : undefined,
    };
}

/**
 * Convert NetworkInstance to row
 */
function networkInstanceToRow(network: NetworkInstance): NetworkInstanceRow {
    return {
        ...network,
        createdAt: network.createdAt.getTime(),
        updatedAt: network.updatedAt.getTime(),
        stats: network.stats ? {
            ...network.stats,
            lastUpdated: network.stats.lastUpdated,
        } : undefined,
    };
}

/**
 * Save a network instance
 */
export async function saveNetworkInstance(network: NetworkInstance): Promise<void> {
    const db = await getNetworkDB();
    const row = networkInstanceToRow(network);
    await db.put('networks', row);

    folderNetworkGraphSync.onNetworksChanged([network]);
}

/**
 * Load a network instance by ID
 */
export async function loadNetworkInstance(id: string): Promise<NetworkInstance | null> {
    const db = await getNetworkDB();
    const row = await db.get('networks', id);
    return row ? rowToNetworkInstance(row) : null;
}

/**
 * Load network by folder ID
 */
export async function loadNetworkByFolderId(folderId: string): Promise<NetworkInstance | null> {
    const db = await getNetworkDB();
    const index = db.transaction('networks').store.index('by-folder');
    const row = await index.get(folderId);
    return row ? rowToNetworkInstance(row) : null;
}

/**
 * Load all networks
 */
export async function loadAllNetworks(): Promise<NetworkInstance[]> {
    const db = await getNetworkDB();
    const rows = await db.getAll('networks');
    return rows.map(rowToNetworkInstance);
}

/**
 * Load networks by namespace
 */
export async function loadNetworksByNamespace(namespace: string): Promise<NetworkInstance[]> {
    const db = await getNetworkDB();
    const index = db.transaction('networks').store.index('by-namespace');
    const rows = await index.getAll(namespace);
    return rows.map(rowToNetworkInstance);
}

/**
 * Load networks by schema ID
 */
export async function loadNetworksBySchemaId(schemaId: string): Promise<NetworkInstance[]> {
    const db = await getNetworkDB();
    const index = db.transaction('networks').store.index('by-schema');
    const rows = await index.getAll(schemaId);
    return rows.map(rowToNetworkInstance);
}

/**
 * Delete a network instance
 */
export async function deleteNetworkInstance(id: string): Promise<void> {
    const db = await getNetworkDB();

    // Delete all relationships for this network
    const tx = db.transaction(['networks', 'relationships'], 'readwrite');
    const relIndex = tx.objectStore('relationships').index('by-network');
    const relKeysToDelete: string[] = [];

    let cursor = await relIndex.openCursor(id);
    while (cursor) {
        relKeysToDelete.push(cursor.value.id);
        cursor = await cursor.continue();
    }

    for (const relId of relKeysToDelete) {
        await tx.objectStore('relationships').delete(relId);
    }

    await tx.objectStore('networks').delete(id);
    await tx.done;

    folderNetworkGraphSync.deleteNetwork(id).catch(err => {
        console.warn('[storage] Failed to delete network from graph:', err);
    });
}

/**
 * Update network instance
 */
export async function updateNetworkInstance(
    id: string,
    updates: Partial<NetworkInstance>
): Promise<NetworkInstance | null> {
    const db = await getNetworkDB();
    const existing = await db.get('networks', id);
    if (!existing) return null;

    const updated = {
        ...existing,
        ...updates,
        updatedAt: Date.now(),
    } as NetworkInstanceRow;

    await db.put('networks', updated);
    return rowToNetworkInstance(updated);
}

// ===== NETWORK SCHEMA OPERATIONS (Custom Schemas) =====

/**
 * Convert row to NetworkSchema
 */
function rowToNetworkSchema(row: NetworkSchemaRow): NetworkSchema {
    return {
        ...row,
        kind: row.kind as NetworkSchema['kind'],
        allowedEntityKinds: row.allowedEntityKinds as NetworkSchema['allowedEntityKinds'],
        relationships: JSON.parse(row.relationships),
        rootEntityKind: row.rootEntityKind as NetworkSchema['rootEntityKind'],
        createdAt: new Date(row.createdAt),
        updatedAt: new Date(row.updatedAt),
    };
}

/**
 * Convert NetworkSchema to row
 */
function networkSchemaToRow(schema: NetworkSchema): NetworkSchemaRow {
    return {
        ...schema,
        relationships: JSON.stringify(schema.relationships),
        createdAt: schema.createdAt.getTime(),
        updatedAt: schema.updatedAt.getTime(),
    };
}

/**
 * Save a custom schema
 */
export async function saveNetworkSchema(schema: NetworkSchema): Promise<void> {
    const db = await getNetworkDB();
    const row = networkSchemaToRow(schema);
    await db.put('schemas', row);
}

/**
 * Load a schema by ID
 */
export async function loadNetworkSchema(id: string): Promise<NetworkSchema | null> {
    const db = await getNetworkDB();
    const row = await db.get('schemas', id);
    return row ? rowToNetworkSchema(row) : null;
}

/**
 * Load all custom schemas
 */
export async function loadAllCustomSchemas(): Promise<NetworkSchema[]> {
    const db = await getNetworkDB();
    const rows = await db.getAll('schemas');
    return rows.map(rowToNetworkSchema);
}

/**
 * Delete a custom schema
 */
export async function deleteNetworkSchema(id: string): Promise<void> {
    const db = await getNetworkDB();
    await db.delete('schemas', id);
}

// ===== RELATIONSHIP INSTANCE OPERATIONS =====

/**
 * Convert row to NetworkRelationshipInstance
 */
function rowToRelationshipInstance(row: NetworkRelationshipRow): NetworkRelationshipInstance {
    return {
        ...row,
        startDate: row.startDate ? new Date(row.startDate) : undefined,
        endDate: row.endDate ? new Date(row.endDate) : undefined,
        attributes: row.attributes ? JSON.parse(row.attributes) : undefined,
        createdAt: new Date(row.createdAt),
        updatedAt: new Date(row.updatedAt),
    };
}

/**
 * Convert NetworkRelationshipInstance to row
 */
function relationshipInstanceToRow(rel: NetworkRelationshipInstance): NetworkRelationshipRow {
    return {
        ...rel,
        startDate: rel.startDate?.getTime(),
        endDate: rel.endDate?.getTime(),
        attributes: rel.attributes ? JSON.stringify(rel.attributes) : undefined,
        createdAt: rel.createdAt.getTime(),
        updatedAt: rel.updatedAt.getTime(),
    };
}

/**
 * Save a relationship instance
 */
export async function saveNetworkRelationship(rel: NetworkRelationshipInstance): Promise<void> {
    const db = await getNetworkDB();
    const row = relationshipInstanceToRow(rel);
    await db.put('relationships', row);

    folderNetworkGraphSync.onNetworkRelationshipsChanged(rel.networkId, [rel]);
}

/**
 * Load relationship by ID
 */
export async function loadNetworkRelationship(id: string): Promise<NetworkRelationshipInstance | null> {
    const db = await getNetworkDB();
    const row = await db.get('relationships', id);
    return row ? rowToRelationshipInstance(row) : null;
}

/**
 * Load all relationships for a network
 */
export async function loadNetworkRelationships(networkId: string): Promise<NetworkRelationshipInstance[]> {
    const db = await getNetworkDB();
    const index = db.transaction('relationships').store.index('by-network');
    const rows = await index.getAll(networkId);
    return rows.map(rowToRelationshipInstance);
}

/**
 * Load relationships by entity (as source or target)
 */
export async function loadEntityRelationships(
    entityId: NodeId,
    networkId?: string
): Promise<NetworkRelationshipInstance[]> {
    const db = await getNetworkDB();
    const tx = db.transaction('relationships');

    // Get by source
    const sourceIndex = tx.store.index('by-source');
    const sourceRows = await sourceIndex.getAll(entityId);

    // Get by target
    const targetIndex = tx.store.index('by-target');
    const targetRows = await targetIndex.getAll(entityId);

    // Combine and dedupe
    const allRows = [...sourceRows, ...targetRows];
    const uniqueRows = Array.from(new Map(allRows.map(r => [r.id, r])).values());

    // Filter by network if specified
    const filteredRows = networkId
        ? uniqueRows.filter(r => r.networkId === networkId)
        : uniqueRows;

    return filteredRows.map(rowToRelationshipInstance);
}

/**
 * Delete a relationship instance
 */
export async function deleteNetworkRelationship(id: string): Promise<void> {
    const db = await getNetworkDB();
    await db.delete('relationships', id);
}

/**
 * Delete all relationships between two entities in a network
 */
export async function deleteRelationshipBetween(
    networkId: string,
    sourceId: NodeId,
    targetId: NodeId,
    relationshipCode?: string
): Promise<number> {
    const db = await getNetworkDB();
    const rels = await loadNetworkRelationships(networkId);

    const toDelete = rels.filter(r =>
        r.sourceEntityId === sourceId &&
        r.targetEntityId === targetId &&
        (!relationshipCode || r.relationshipCode === relationshipCode)
    );

    const tx = db.transaction('relationships', 'readwrite');
    for (const rel of toDelete) {
        await tx.store.delete(rel.id);
    }
    await tx.done;

    return toDelete.length;
}

// ===== ENTITY MEMBERSHIP QUERIES =====

/**
 * Get all networks an entity belongs to
 */
export async function getEntityNetworks(entityId: NodeId): Promise<NetworkInstance[]> {
    const allNetworks = await loadAllNetworks();
    return allNetworks.filter(n => n.entityIds.includes(entityId));
}

/**
 * Add entity to network
 */
export async function addEntityToNetwork(
    networkId: string,
    entityId: NodeId
): Promise<boolean> {
    const network = await loadNetworkInstance(networkId);
    if (!network) return false;

    if (!network.entityIds.includes(entityId)) {
        network.entityIds.push(entityId);
        network.updatedAt = new Date();
        await saveNetworkInstance(network);
    }

    return true;
}

/**
 * Remove entity from network
 */
export async function removeEntityFromNetwork(
    networkId: string,
    entityId: NodeId
): Promise<boolean> {
    const network = await loadNetworkInstance(networkId);
    if (!network) return false;

    network.entityIds = network.entityIds.filter(id => id !== entityId);
    network.updatedAt = new Date();
    await saveNetworkInstance(network);

    // Also remove any relationships involving this entity
    const rels = await loadNetworkRelationships(networkId);
    const db = await getNetworkDB();
    const tx = db.transaction('relationships', 'readwrite');

    for (const rel of rels) {
        if (rel.sourceEntityId === entityId || rel.targetEntityId === entityId) {
            await tx.store.delete(rel.id);
        }
    }
    await tx.done;

    return true;
}

// ===== STATISTICS =====

/**
 * Compute and update network statistics
 */
export async function updateNetworkStats(networkId: string): Promise<NetworkStats | null> {
    const network = await loadNetworkInstance(networkId);
    if (!network) return null;

    const relationships = await loadNetworkRelationships(networkId);

    // Compute max depth (BFS from root)
    let maxDepth = 0;
    if (network.rootEntityId) {
        const visited = new Set<NodeId>();
        const queue: Array<{ id: NodeId; depth: number }> = [
            { id: network.rootEntityId, depth: 0 }
        ];

        while (queue.length > 0) {
            const { id, depth } = queue.shift()!;
            if (visited.has(id)) continue;
            visited.add(id);

            maxDepth = Math.max(maxDepth, depth);

            // Find children (PARENT_OF, MANAGES, etc.)
            const childRels = relationships.filter(r =>
                r.sourceEntityId === id &&
                ['PARENT_OF', 'MANAGES', 'SUPERIOR_OF', 'LEADS'].includes(r.relationshipCode)
            );

            for (const rel of childRels) {
                queue.push({ id: rel.targetEntityId, depth: depth + 1 });
            }
        }
    }

    const stats: NetworkStats = {
        memberCount: network.entityIds.length,
        relationshipCount: relationships.length,
        maxDepth,
        generationCount: maxDepth + 1, // Root is generation 0
        lastUpdated: new Date(),
    };

    // Update network with new stats
    network.stats = stats;
    await saveNetworkInstance(network);

    return stats;
}

// ===== EXPORT/IMPORT =====

/**
 * Export all network data
 */
export async function exportNetworkData(): Promise<{
    networks: NetworkInstance[];
    schemas: NetworkSchema[];
    relationships: NetworkRelationshipInstance[];
}> {
    const networks = await loadAllNetworks();
    const schemas = await loadAllCustomSchemas();

    const allRelationships: NetworkRelationshipInstance[] = [];
    for (const network of networks) {
        const rels = await loadNetworkRelationships(network.id);
        allRelationships.push(...rels);
    }

    return { networks, schemas, relationships: allRelationships };
}

/**
 * Import network data
 */
export async function importNetworkData(data: {
    networks: NetworkInstance[];
    schemas?: NetworkSchema[];
    relationships: NetworkRelationshipInstance[];
}): Promise<void> {
    // Import schemas first
    if (data.schemas) {
        for (const schema of data.schemas) {
            await saveNetworkSchema(schema);
        }
    }

    // Import networks
    for (const network of data.networks) {
        await saveNetworkInstance(network);
    }

    // Import relationships
    for (const rel of data.relationships) {
        await saveNetworkRelationship(rel);
    }
}

/**
 * Clear all network data
 */
export async function clearAllNetworkData(): Promise<void> {
    const db = await getNetworkDB();
    const tx = db.transaction(['networks', 'schemas', 'relationships'], 'readwrite');
    await tx.objectStore('networks').clear();
    await tx.objectStore('schemas').clear();
    await tx.objectStore('relationships').clear();
    await tx.done;
}
