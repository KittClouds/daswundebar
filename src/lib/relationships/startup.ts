/**
 * Relationship System Startup - LEGACY
 * 
 * This module initializes the relationship system.
 * Migrated to use smartGraphRegistry.
 */

import { RelationshipStoreImpl } from '@/lib/storage/impl/RelationshipStoreImpl';
import { smartGraphRegistry } from '@/lib/tauri';

// Legacy compatibility stubs
let relationshipStore: RelationshipStoreImpl | null = null;

export function setRelationshipStore(store: RelationshipStoreImpl) {
    relationshipStore = store;
    console.log('[RelationshipSystem] setRelationshipStore called (legacy compatibility)');
}

export function getRelationshipStore(): RelationshipStoreImpl | null {
    return relationshipStore;
}

export async function initializeRelationshipSystem(): Promise<{ loaded: number }> {
    try {
        // Use smartGraphRegistry for entity initialization
        await smartGraphRegistry.init();
        const entityCount = smartGraphRegistry.getAllEntities().length;
        console.log(`[RelationshipSystem] Initialized via SmartGraphRegistry. ${entityCount} entities loaded.`);
        // TODO: Add relationship support to smartGraphRegistry
        return { loaded: 0 };
    } catch (error) {
        console.error("Failed to initialize relationship system:", error);
        return { loaded: 0 };
    }
}

