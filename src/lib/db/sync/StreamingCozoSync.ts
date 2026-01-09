/**
 * StreamingCozoSync - Weapons-Grade Sync Engine Component
 * 
 * Syncs deltas incrementally to CozoDB using :put (upsert) instead of :replace (full wipe).
 * Batches multiple deltas into single Datalog queries.
 * 
 * Key improvements over full graph rebuild:
 * - O(delta) instead of O(n) per sync
 * - Uses :put for upserts (no full relation wipe)
 * - Uses :rm for deletes
 * - Batches operations for efficiency
 */

import { cozoDb } from '@/lib/cozo-stubs/db';
import type { Delta } from './types';
import type { SQLiteNodeInput, SQLiteEdgeInput } from '../client/types';

export class StreamingCozoSync {
    private enabled = true;

    /**
     * Sync deltas incrementally to CozoDB
     */
    async syncDeltas(deltas: Delta[]): Promise<void> {
        if (!this.enabled || deltas.length === 0) return;

        if (!cozoDb.isReady()) {
            console.warn('[StreamingCozoSync] CozoDB not ready, skipping sync');
            return;
        }

        const nodeDeltas = deltas.filter(d => d.type === 'node');
        const edgeDeltas = deltas.filter(d => d.type === 'edge');

        try {
            // Process nodes first (edges depend on nodes)
            if (nodeDeltas.length > 0) {
                await this.syncNodeDeltas(nodeDeltas);
            }

            // Then process edges
            if (edgeDeltas.length > 0) {
                await this.syncEdgeDeltas(edgeDeltas);
            }
        } catch (err) {
            console.error('[StreamingCozoSync] Sync failed:', err);
            // Don't throw - CozoDB sync failures shouldn't block SQLite writes
        }
    }

    /**
     * Sync node deltas to CozoDB entity relation
     */
    private async syncNodeDeltas(deltas: Delta[]): Promise<void> {
        const inserts: string[] = [];
        const deletes: string[] = [];

        for (const delta of deltas) {
            if (delta.operation === 'DELETE') {
                deletes.push(`"${this.escape(delta.id)}"`);
            } else if (delta.operation === 'INSERT' || delta.operation === 'UPDATE') {
                const data = delta.fullData as SQLiteNodeInput & { id: string };
                if (!data) continue;

                const name = this.escape(data.label || '');
                const kind = this.escape(data.entity_kind || data.type || 'NOTE');
                const nodeType = this.escape(data.type || 'NOTE');
                const parentId = data.parent_id ? `"${this.escape(data.parent_id)}"` : 'null';

                inserts.push(
                    `["${this.escape(delta.id)}", "${nodeType}", "${name}", "${kind}", ${parentId}, ${delta.timestamp}]`
                );
            }
        }

        // Execute deletes first (avoid constraint violations)
        if (deletes.length > 0) {
            try {
                const deleteQuery = `
          ?[id] <- [[${deletes.join('], [')}]]
          :rm node {id}
        `;
                cozoDb.run(deleteQuery);
            } catch (err) {
                console.warn('[StreamingCozoSync] Node delete failed:', err);
            }
        }

        // Execute inserts/updates (upsert via :put)
        if (inserts.length > 0) {
            try {
                const insertQuery = `
          ?[id, type, name, kind, parent_id, updated_at] <- [
            ${inserts.join(',\n            ')}
          ]
          :put node {id => type, name, kind, parent_id, updated_at}
        `;
                cozoDb.run(insertQuery);
            } catch (err) {
                console.warn('[StreamingCozoSync] Node upsert failed:', err);
            }
        }
    }

    /**
     * Sync edge deltas to CozoDB edge relation
     */
    private async syncEdgeDeltas(deltas: Delta[]): Promise<void> {
        const inserts: string[] = [];
        const deletes: Array<[string, string]> = [];

        for (const delta of deltas) {
            const data = delta.fullData as SQLiteEdgeInput & { id?: string };

            if (delta.operation === 'DELETE') {
                if (data?.source && data?.target) {
                    deletes.push([this.escape(data.source), this.escape(data.target)]);
                }
            } else if (delta.operation === 'INSERT' || delta.operation === 'UPDATE') {
                if (!data?.source || !data?.target) continue;

                const edgeType = this.escape(data.type || 'RELATED_TO');
                const weight = data.weight ?? 1.0;

                inserts.push(
                    `["${this.escape(data.source)}", "${this.escape(data.target)}", "${edgeType}", ${weight}, ${delta.timestamp}]`
                );
            }
        }

        // Execute deletes first
        if (deletes.length > 0) {
            try {
                const deleteRows = deletes.map(([s, t]) => `["${s}", "${t}"]`).join(', ');
                const deleteQuery = `
          ?[from_id, to_id] <- [${deleteRows}]
          :rm edge {from_id, to_id}
        `;
                cozoDb.run(deleteQuery);
            } catch (err) {
                console.warn('[StreamingCozoSync] Edge delete failed:', err);
            }
        }

        // Execute inserts/updates
        if (inserts.length > 0) {
            try {
                const insertQuery = `
          ?[from_id, to_id, type, weight, updated_at] <- [
            ${inserts.join(',\n            ')}
          ]
          :put edge {from_id, to_id => type, weight, updated_at}
        `;
                cozoDb.run(insertQuery);
            } catch (err) {
                console.warn('[StreamingCozoSync] Edge upsert failed:', err);
            }
        }
    }

    /**
     * Escape string for CozoDB Datalog queries
     */
    private escape(str: string): string {
        return str
            .replace(/\\/g, '\\\\')
            .replace(/"/g, '\\"')
            .replace(/\n/g, '\\n')
            .replace(/\r/g, '\\r')
            .replace(/\t/g, '\\t');
    }

    /**
     * Enable/disable CozoDB sync
     */
    setEnabled(enabled: boolean): void {
        this.enabled = enabled;
    }

    /**
     * Check if sync is enabled
     */
    isEnabled(): boolean {
        return this.enabled;
    }
}

// Singleton instance
export const streamingCozoSync = new StreamingCozoSync();
