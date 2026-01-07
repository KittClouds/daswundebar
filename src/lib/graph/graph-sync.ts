/**
 * Graph Sync Utilities - PIPELINE A ONLY (WASM/Legacy)
 * 
 * Synchronizes scan results with Browser CozoDB (via cozoDb).
 * This is ONLY used by extractor-facade.wasm.ts.
 * 
 * For Pipeline B (Tauri), use smartGraphRegistry.ingestScanResult() instead.
 * 
 * @see extractor-facade.wasm.ts - uses fullGraphSync()
 * @see extractor-facade.tauri.ts - uses smartGraphRegistry.ingestScanResult()
 */

import { cozoDb } from '../cozo/db';
import type { ScanResult } from '../scanner/bridge';

export interface GraphSyncOptions {
    groupId: string;
    noteId: string;
    overwrite?: boolean;
}

export interface GraphSyncResult {
    entitiesSynced: number;
    edgesSynced: number;
    durationMs: number;
}

/**
 * Sync entities extracted from a scan to CozoDB
 */
export async function syncEntitiesToCozo(
    result: ScanResult,
    options: GraphSyncOptions
): Promise<GraphSyncResult> {
    const startTime = Date.now();
    let entitiesSynced = 0;
    let edgesSynced = 0;

    // Extract entities from scan result
    const entities: Array<{
        id: string;
        name: string;
        kind: string;
    }> = [];

    // From implicit mentions
    for (const mention of result.implicit || []) {
        entities.push({
            id: `entity:${mention.entity_label.toLowerCase().replace(/\s+/g, '_')}`,
            name: mention.entity_label,
            kind: mention.entity_kind,
        });
    }

    // From triples (source and target entities) - preserve entity kinds
    for (const triple of result.triples || []) {
        entities.push({
            id: `entity:${triple.source.toLowerCase().replace(/\s+/g, '_')}`,
            name: triple.source,
            kind: (triple as any).source_kind || 'CONCEPT',
        });
        entities.push({
            id: `entity:${triple.target.toLowerCase().replace(/\s+/g, '_')}`,
            name: triple.target,
            kind: (triple as any).target_kind || 'CONCEPT',
        });
    }

    // Deduplicate
    const uniqueEntities = Array.from(
        new Map(entities.map(e => [e.id, e])).values()
    );

    // Upsert entities to CozoDB
    if (uniqueEntities.length > 0) {
        try {
            const entityData = uniqueEntities.map(e => [
                e.id,
                e.name,
                e.kind,
                options.groupId,
                1,  // frequency
                Date.now() / 1000,  // created_at
            ]);

            const query = `
                ?[id, name, entity_kind, group_id, frequency, created_at] <- $data
                
                :put entity {
                    id,
                    name,
                    entity_kind,
                    group_id,
                    frequency,
                    created_at
                }
            `;

            await cozoDb.runQuery(query, { data: entityData });
            entitiesSynced = uniqueEntities.length;
        } catch (err) {
            console.error('[GraphSync] Entity sync failed:', err);
        }
    }

    // Sync edges from triples
    if ((result.triples || []).length > 0) {
        try {
            const edgeData = result.triples.map(triple => {
                const sourceId = `entity:${triple.source.toLowerCase().replace(/\s+/g, '_')}`;
                const targetId = `entity:${triple.target.toLowerCase().replace(/\s+/g, '_')}`;
                return [
                    sourceId,
                    targetId,
                    triple.confidence || 0.8,
                    options.groupId,
                ];
            });

            const query = `
                ?[source_id, target_id, weight, group_id] <- $data
                
                :put entity_edge {
                    source_id,
                    target_id,
                    weight,
                    group_id
                }
            `;

            await cozoDb.runQuery(query, { data: edgeData });
            edgesSynced = result.triples.length;
        } catch (err) {
            console.error('[GraphSync] Edge sync failed:', err);
        }
    }

    // Update mention tracking (entity ↔ note)
    if (uniqueEntities.length > 0) {
        try {
            const mentionData = uniqueEntities.map(e => [
                e.id,
                options.noteId,
                1,  // count
            ]);

            const query = `
                ?[entity_id, episode_id, count] <- $data
                
                :put mentions {
                    entity_id,
                    episode_id,
                    count
                }
            `;

            await cozoDb.runQuery(query, { data: mentionData });
        } catch (err) {
            console.error('[GraphSync] Mention sync failed:', err);
        }
    }

    return {
        entitiesSynced,
        edgesSynced,
        durationMs: Date.now() - startTime,
    };
}

/**
 * Sync relations from scan result to CozoDB
 */
export async function syncRelationsToCozo(
    result: ScanResult,
    options: GraphSyncOptions
): Promise<number> {
    if (!result.relations || result.relations.length === 0) {
        return 0;
    }

    try {
        const relationData = result.relations.map(rel => [
            `rel:${Date.now()}_${Math.random().toString(36).slice(2)}`,
            `entity:${rel.head_entity.toLowerCase().replace(/\s+/g, '_')}`,
            `entity:${rel.tail_entity.toLowerCase().replace(/\s+/g, '_')}`,
            rel.relation_type,
            rel.confidence,
            options.groupId,
            Date.now() / 1000,
        ]);

        const query = `
            ?[id, source_id, target_id, relation_type, confidence, group_id, created_at] <- $data
            
            :put relationship {
                id,
                source_id,
                target_id,
                relation_type,
                confidence,
                group_id,
                created_at
            }
        `;

        await cozoDb.runQuery(query, { data: relationData });
        return result.relations.length;
    } catch (err) {
        console.error('[GraphSync] Relations sync failed:', err);
        return 0;
    }
}

/**
 * Full sync: entities, edges, and relations
 */
export async function fullGraphSync(
    result: ScanResult,
    options: GraphSyncOptions
): Promise<GraphSyncResult & { relationsSynced: number }> {
    const entityResult = await syncEntitiesToCozo(result, options);
    const relationsSynced = await syncRelationsToCozo(result, options);

    return {
        ...entityResult,
        relationsSynced,
    };
}
