/**
 * Time Registry - Tauri Bridge
 * 
 * TypeScript interface to the Rust Time Registry.
 * Tracks changes to entities and edges over time.
 */

import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Types
// =============================================================================

export interface HistoryEntry {
    entry_id: string;
    entity_id: string;
    action: 'create' | 'update' | 'delete';
    data_json: string;
    timestamp: number;
}

export interface EdgeHistoryEntry {
    entry_id: string;
    source_id: string;
    target_id: string;
    action: 'create' | 'update' | 'delete';
    data_json: string;
    timestamp: number;
}

export interface RecordChangeInput {
    entity_id: string;
    action: 'create' | 'update' | 'delete';
    data: unknown;
}

export interface RecordEdgeChangeInput {
    source_id: string;
    target_id: string;
    action: 'create' | 'update' | 'delete';
    data: unknown;
}

// =============================================================================
// Tauri Commands
// =============================================================================

/**
 * Initialize the time registry schema (called during app startup)
 */
export async function timeRegistryInit(): Promise<void> {
    return invoke('time_registry_init');
}

/**
 * Record an entity change
 */
export async function timeRegistryRecordEntityChange(input: RecordChangeInput): Promise<void> {
    return invoke('time_registry_record_entity_change', { input });
}

/**
 * Get entity change history
 */
export async function timeRegistryGetEntityHistory(entityId: string): Promise<HistoryEntry[]> {
    return invoke('time_registry_get_entity_history', { entityId });
}

/**
 * Record an edge change
 */
export async function timeRegistryRecordEdgeChange(input: RecordEdgeChangeInput): Promise<void> {
    return invoke('time_registry_record_edge_change', { input });
}

/**
 * Get edge change history
 */
export async function timeRegistryGetEdgeHistory(sourceId: string, targetId: string): Promise<EdgeHistoryEntry[]> {
    return invoke('time_registry_get_edge_history', { sourceId, targetId });
}

// =============================================================================
// Facade Class
// =============================================================================

/**
 * Time Registry Facade - mirrors the ITemporalStore interface
 */
export class TauriTimeRegistryFacade {
    private initialized = false;

    async init(): Promise<void> {
        if (this.initialized) return;
        try {
            await timeRegistryInit();
            this.initialized = true;
            console.log('[TauriTimeRegistry] Schema initialized');
        } catch (error) {
            console.error('[TauriTimeRegistry] Init failed:', error);
            throw error;
        }
    }

    async recordChange(entityId: string, action: 'create' | 'update' | 'delete', data: unknown): Promise<void> {
        await this.ensureInit();
        return timeRegistryRecordEntityChange({
            entity_id: entityId,
            action,
            data,
        });
    }

    async getEntityHistory(entityId: string): Promise<HistoryEntry[]> {
        await this.ensureInit();
        return timeRegistryGetEntityHistory(entityId);
    }

    async recordEdgeChange(
        sourceId: string,
        targetId: string,
        action: 'create' | 'update' | 'delete',
        data: unknown
    ): Promise<void> {
        await this.ensureInit();
        return timeRegistryRecordEdgeChange({
            source_id: sourceId,
            target_id: targetId,
            action,
            data,
        });
    }

    async getEdgeHistory(sourceId: string, targetId: string): Promise<EdgeHistoryEntry[]> {
        await this.ensureInit();
        return timeRegistryGetEdgeHistory(sourceId, targetId);
    }

    private async ensureInit(): Promise<void> {
        if (!this.initialized) {
            await this.init();
        }
    }
}

// Singleton instance
export const tauriTimeRegistry = new TauriTimeRegistryFacade();
