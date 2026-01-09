/**
 * Tauri Temporal Store Adapter
 * 
 * Implements ITemporalStore interface using Tauri commands.
 * Routes temporal storage to native Rust CozoDB.
 */

import type { ITemporalStore, GraphSnapshot, HistoryEntry as IHistoryEntry } from '../interfaces';
import {
    timeRegistryInit,
    timeRegistryRecordEntityChange,
    timeRegistryGetEntityHistory,
    timeRegistryRecordEdgeChange,
    timeRegistryGetEdgeHistory,
} from '@/lib/tauri/time-registry';

/**
 * Tauri-backed implementation of ITemporalStore
 * Uses native Rust CozoDB for persistence
 */
export class TauriTemporalStoreAdapter implements ITemporalStore {
    private initialized = false;

    /**
     * Initialize temporal storage
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;
        try {
            await timeRegistryInit();
            this.initialized = true;
            console.log('[TauriTemporalStore] Initialized');
        } catch (error) {
            console.error('[TauriTemporalStore] Init failed:', error);
            throw error;
        }
    }

    /**
     * Get a snapshot of the graph at a specific point in time
     * Note: This is a complex operation that requires replaying history
     */
    async getSnapshot(groupId: string, timestamp: number): Promise<GraphSnapshot> {
        await this.ensureInit();
        // TODO: Implement proper snapshot reconstruction
        // For now, return empty snapshot
        console.warn('[TauriTemporalStore] getSnapshot not fully implemented yet');
        return {
            entities: [],
            edges: [],
            timestamp,
        };
    }

    /**
     * Get history of changes for an entity
     */
    async getEntityHistory(entityId: string): Promise<IHistoryEntry[]> {
        await this.ensureInit();
        const entries = await timeRegistryGetEntityHistory(entityId);
        return entries.map(e => ({
            timestamp: e.timestamp,
            action: e.action as 'create' | 'update' | 'delete',
            data: JSON.parse(e.data_json || '{}'),
        }));
    }

    /**
     * Get history of changes for an edge
     */
    async getEdgeHistory(sourceId: string, targetId: string): Promise<IHistoryEntry[]> {
        await this.ensureInit();
        const entries = await timeRegistryGetEdgeHistory(sourceId, targetId);
        return entries.map(e => ({
            timestamp: e.timestamp,
            action: e.action as 'create' | 'update' | 'delete',
            data: JSON.parse(e.data_json || '{}'),
        }));
    }

    /**
     * Record a change to an entity
     */
    recordChange(entityId: string, action: 'create' | 'update' | 'delete', data: unknown): void {
        // Fire and forget - don't block
        this.ensureInit().then(() => {
            timeRegistryRecordEntityChange({
                entity_id: entityId,
                action,
                data,
            }).catch(err => {
                console.error('[TauriTemporalStore] Failed to record change:', err);
            });
        });
    }

    /**
     * Record a change to an edge
     */
    recordEdgeChange(sourceId: string, targetId: string, action: 'create' | 'update' | 'delete', data: unknown): void {
        // Fire and forget - don't block
        this.ensureInit().then(() => {
            timeRegistryRecordEdgeChange({
                source_id: sourceId,
                target_id: targetId,
                action,
                data,
            }).catch(err => {
                console.error('[TauriTemporalStore] Failed to record edge change:', err);
            });
        });
    }

    /**
     * Clear all history (used for testing)
     */
    clear(): void {
        // TODO: Implement clear in Rust backend
        console.warn('[TauriTemporalStore] clear not implemented yet');
    }

    private async ensureInit(): Promise<void> {
        if (!this.initialized) {
            await this.initialize();
        }
    }
}

// Singleton
let tauriTemporalStore: TauriTemporalStoreAdapter | null = null;

export function getTauriTemporalStore(): TauriTemporalStoreAdapter {
    if (!tauriTemporalStore) {
        tauriTemporalStore = new TauriTemporalStoreAdapter();
    }
    return tauriTemporalStore;
}

export function resetTauriTemporalStore(): void {
    tauriTemporalStore = null;
}
