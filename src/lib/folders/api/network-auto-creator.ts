/**
 * Network Auto-Creator
 * 
 * Monitors folder changes and triggers network creation when:
 * 1. A subfolder has autoCreateNetwork: true in its schema
 * 2. Child count reaches networkCreationThreshold
 * 3. No network already exists for that root folder
 * 
 * Uses CozoDB as the primary storage for networks.
 */

import { cozoDb } from '@/lib/cozo-stubs/db';
import { folderSchemaRegistry } from '../schema-registry';
import {
    NETWORK_INSTANCE_QUERIES,
    NETWORK_MEMBERSHIP_QUERIES,
    NETWORK_RELATIONSHIP_QUERIES,
    FOLDER_HIERARCHY_QUERIES
} from '@/lib/cozo-stubs/schema';
import type { EntityKind } from '@/lib/types/entityTypes';

// Generate UUID v7 for new entities
function generateId(): string {
    // Simple UUID v4 fallback - in production use uuidv7
    return crypto.randomUUID();
}

export interface NetworkCreationResult {
    created: boolean;
    networkId?: string;
    networkName?: string;
    memberCount?: number;
    reason?: string;
}

export interface NetworkAutoCreateConfig {
    schemaId: string;
    threshold: number;
    rootFolderId: string;
    rootEntityId?: string;
    rootEntityName: string;
    subfolderLabel: string;
    entityKind: EntityKind;
}

/**
 * Check if a network should be auto-created and create it if conditions are met.
 * 
 * Called when an entity is added to a typed folder subfolder.
 */
export async function checkAndCreateNetworkForFolder(
    config: NetworkAutoCreateConfig,
    currentChildCount: number
): Promise<NetworkCreationResult> {
    const { schemaId, threshold, rootFolderId, rootEntityId, rootEntityName, subfolderLabel, entityKind } = config;

    // 1. Check if we've reached the threshold
    if (currentChildCount < threshold) {
        return {
            created: false,
            reason: `Child count ${currentChildCount} below threshold ${threshold}`,
        };
    }

    // 2. Check if network already exists for this root folder
    const existingNetwork = await getNetworkByFolderId(rootFolderId);
    if (existingNetwork) {
        return {
            created: false,
            networkId: existingNetwork.id,
            reason: 'Network already exists for this folder',
        };
    }

    // 3. Create the network with the naming pattern
    const networkName = `${rootEntityName}'s ${subfolderLabel}`;
    const networkId = generateId();
    const now = Date.now();

    try {
        // Create network instance in CozoDB
        const result = cozoDb.runQuery(NETWORK_INSTANCE_QUERIES.upsert, {
            id: networkId,
            name: networkName,
            schema_id: schemaId,
            network_kind: entityKind,
            network_subtype: null,
            root_folder_id: rootFolderId,
            root_entity_id: rootEntityId || null,
            namespace: 'default',
            description: `Auto-created network for ${rootEntityName}`,
            tags: ['auto-created'],
            member_count: currentChildCount + 1, // Include root entity
            relationship_count: currentChildCount, // One edge per child
            max_depth: 1,
            created_at: now,
            updated_at: now,
            group_id: rootFolderId,
            scope_type: 'network',
        });

        if (result.ok === false) {
            console.error('[NetworkAutoCreator] Failed to create network:', result.message);
            return {
                created: false,
                reason: `CozoDB error: ${result.message}`,
            };
        }

        // Add root entity as network member
        if (rootEntityId) {
            await addNetworkMember(networkId, rootEntityId, 'ROOT', 0, rootFolderId);
        }

        console.log(`[NetworkAutoCreator] Created network "${networkName}" (${networkId}) with ${currentChildCount + 1} members`);

        return {
            created: true,
            networkId,
            networkName,
            memberCount: currentChildCount + 1,
        };
    } catch (error) {
        console.error('[NetworkAutoCreator] Error creating network:', error);
        return {
            created: false,
            reason: error instanceof Error ? error.message : String(error),
        };
    }
}

/**
 * Called when an entity is added to a folder.
 * Checks if this triggers network creation and handles membership updates.
 */
export async function onEntityAddedToFolder(
    folderId: string,
    entityId: string,
    entityKind: EntityKind,
    entityName: string
): Promise<NetworkCreationResult | undefined> {
    // Get folder info to determine parent and type
    const folderInfo = await getFolderInfo(folderId);
    if (!folderInfo || !folderInfo.parentId) {
        return undefined; // Not a subfolder, nothing to do
    }

    const parentInfo = await getFolderInfo(folderInfo.parentId);
    if (!parentInfo || !parentInfo.entityKind) {
        return undefined; // Parent is not a typed folder
    }

    // Check if this subfolder type triggers network creation
    const config = folderSchemaRegistry.getNetworkCreationConfig(
        parentInfo.entityKind as EntityKind,
        parentInfo.entitySubtype,
        entityKind,
        folderInfo.entitySubtype
    );

    if (!config) {
        return undefined; // No network auto-creation for this subfolder type
    }

    // Count current children in this subfolder
    const childCount = await countFolderChildren(folderId);

    // Attempt network creation
    const result = await checkAndCreateNetworkForFolder(
        {
            schemaId: config.schemaId,
            threshold: config.threshold,
            rootFolderId: folderInfo.parentId,
            rootEntityId: parentInfo.entityId,
            rootEntityName: parentInfo.name || 'Unknown',
            subfolderLabel: folderInfo.name || entityKind,
            entityKind: parentInfo.entityKind as EntityKind,
        },
        childCount
    );

    // If network exists (new or existing), add this entity as member
    if (result.networkId) {
        await addNetworkMember(
            result.networkId,
            entityId,
            'MEMBER',
            1,
            folderId
        );

        // Create relationship edge to root if we have root entity
        if (parentInfo.entityId) {
            await addNetworkRelationship(
                result.networkId,
                parentInfo.entityId,
                entityId,
                config.schemaId.includes('FAMILY') ? 'FAMILY_OF' :
                    config.schemaId.includes('RIVAL') ? 'ENEMY_OF' : 'ALLY_OF',
                folderId
            );
        }
    }

    return result;
}

// ===== CozoDB Helper Functions =====

async function getNetworkByFolderId(folderId: string): Promise<{ id: string; name: string } | null> {
    try {
        const result = cozoDb.runQuery(NETWORK_INSTANCE_QUERIES.getByFolderId, {
            folder_id: folderId,
        });

        if (result.rows && result.rows.length > 0) {
            return {
                id: result.rows[0][0] as string,
                name: result.rows[0][1] as string,
            };
        }
        return null;
    } catch {
        return null;
    }
}

async function getFolderInfo(folderId: string): Promise<{
    id: string;
    parentId?: string;
    name?: string;
    entityKind?: string;
    entitySubtype?: string;
    entityId?: string;
} | null> {
    // Query folder hierarchy to get parent
    try {
        const hierarchyResult = cozoDb.runQuery(FOLDER_HIERARCHY_QUERIES.getByChildId, {
            child_id: folderId,
        });

        // For now, return basic info - in production, join with SQLite for full details
        if (hierarchyResult.rows && hierarchyResult.rows.length > 0) {
            const row = hierarchyResult.rows[0];
            return {
                id: folderId,
                parentId: row[1] as string,
                entityKind: row[5] as string | undefined,
            };
        }

        return { id: folderId };
    } catch {
        return null;
    }
}

async function countFolderChildren(folderId: string): Promise<number> {
    try {
        const result = cozoDb.runQuery(FOLDER_HIERARCHY_QUERIES.getByParentId, {
            parent_id: folderId,
        });

        return result.rows?.length || 0;
    } catch {
        return 0;
    }
}

async function addNetworkMember(
    networkId: string,
    entityId: string,
    role: 'ROOT' | 'MEMBER',
    depthLevel: number,
    groupId: string
): Promise<void> {
    const now = Date.now();
    const memberId = generateId();

    try {
        cozoDb.runQuery(NETWORK_MEMBERSHIP_QUERIES.upsert, {
            id: memberId,
            network_id: networkId,
            entity_id: entityId,
            role: role,
            joined_at: now,
            left_at: null,
            is_root: role === 'ROOT',
            depth_level: depthLevel,
            created_at: now,
            updated_at: now,
            group_id: groupId,
            extraction_methods: ['folder_auto_create'],
        });
    } catch (error) {
        console.error('[NetworkAutoCreator] Failed to add network member:', error);
    }
}

async function addNetworkRelationship(
    networkId: string,
    sourceId: string,
    targetId: string,
    relationshipCode: string,
    groupId: string
): Promise<void> {
    const now = Date.now();
    const relationshipId = generateId();

    try {
        cozoDb.runQuery(NETWORK_RELATIONSHIP_QUERIES.upsert, {
            id: relationshipId,
            network_id: networkId,
            source_id: sourceId,
            target_id: targetId,
            relationship_code: relationshipCode,
            inverse_code: null,
            start_date: null,
            end_date: null,
            strength: 1.0,
            notes: null,
            attributes: null,
            created_at: now,
            updated_at: now,
            group_id: groupId,
            scope_type: 'network',
            confidence: 1.0,
            extraction_methods: ['folder_auto_create'],
        });
    } catch (error) {
        console.error('[NetworkAutoCreator] Failed to add network relationship:', error);
    }
}

/**
 * Update network stats after membership changes
 */
export async function updateNetworkStats(networkId: string): Promise<void> {
    try {
        // Count current members
        const memberResult = cozoDb.runQuery(NETWORK_MEMBERSHIP_QUERIES.countByNetwork, {
            network_id: networkId,
        });
        const memberCount = memberResult.rows?.[0]?.[0] || 0;

        // Count current relationships
        const relResult = cozoDb.runQuery(NETWORK_RELATIONSHIP_QUERIES.countByNetwork, {
            network_id: networkId,
        });
        const relationshipCount = relResult.rows?.[0]?.[0] || 0;

        // Update network instance
        cozoDb.runQuery(NETWORK_INSTANCE_QUERIES.updateStats, {
            id: networkId,
            member_count: memberCount,
            relationship_count: relationshipCount,
            max_depth: 1, // For now, single-level networks
            updated_at: Date.now(),
        });
    } catch (error) {
        console.error('[NetworkAutoCreator] Failed to update network stats:', error);
    }
}
