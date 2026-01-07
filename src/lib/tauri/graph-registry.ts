/**
 * TauriGraphRegistry - TypeScript adapter for Rust GraphRegistry
 * 
 * This provides a drop-in replacement for the existing entityRegistry.
 * Uses Tauri IPC to communicate with the Rust GraphRegistry.
 * 
 * Key feature: Smart hydration that only re-hydrates when entity set changes.
 */

import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Types (match Rust command types)
// =============================================================================

export interface EntityDefinition {
    id: string;
    label: string;
    kind: string;
    aliases: string[];
}

export interface NodeResponse {
    id: string;
    label: string;
    kind: string;
    mention_count: number;
    aliases: string[];
    is_new: boolean;
}

export interface EdgeResponse {
    id: string;
    source_id: string;
    target_id: string;
    edge_type: string;
    confidence: number;
}

export interface IngestResponse {
    entities_created: number;
    entities_updated: number;
    edges_created: number;
    edges_updated: number;
}

export interface HydrateResponse {
    entity_count: number;
    needs_hydration: boolean;
    entities: EntityDefinition[] | null;
}

export interface GraphStats {
    node_count: number;
    edge_count: number;
    avg_degree: number;
    density: number;
    component_count: number;
}

export interface MentionInput {
    entity_label: string;
    entity_kind: string;
}

export interface RelationInput {
    head_label: string;
    tail_label: string;
    relation_type: string;
    confidence?: number;
    source?: 'explicit' | 'cst' | 'graph';
}

// =============================================================================
// TauriGraphRegistry
// =============================================================================

export class TauriGraphRegistry {
    private initialized = false;

    async initialize(): Promise<void> {
        // Graph is initialized on Rust side at startup
        this.initialized = true;
        console.log('[TauriGraphRegistry] Ready');
    }

    // =========================================================================
    // Node Operations
    // =========================================================================

    /**
     * Register a node (entity). Returns existing if found by label.
     */
    async registerEntity(
        label: string,
        kind: string,
        sourceNote: string,
        options?: { subtype?: string; aliases?: string[]; source?: 'user' | 'extraction' }
    ): Promise<{ entity: NodeResponse; isNew: boolean }> {
        const response = await invoke<NodeResponse>('graph_register_node', {
            request: {
                label,
                kind,
                source_note: sourceNote,
                subtype: options?.subtype ?? null,
                aliases: options?.aliases ?? [],
                created_by: options?.source ?? 'user',
            },
        });

        return { entity: response, isNew: response.is_new };
    }

    /**
     * Get a node by ID
     */
    async getEntity(id: string): Promise<NodeResponse | null> {
        return invoke<NodeResponse | null>('graph_get_node', { id });
    }

    /**
     * Find a node by label (case-insensitive, checks aliases)
     */
    async findEntityByLabel(label: string): Promise<NodeResponse | null> {
        return invoke<NodeResponse | null>('graph_find_node', { label });
    }

    /**
     * Get all entities, optionally filtered by kind
     */
    async getAllEntities(kind?: string): Promise<NodeResponse[]> {
        return invoke<NodeResponse[]>('graph_get_nodes', { kind: kind ?? null });
    }

    /**
     * Delete a node (cascades to edges)
     */
    async deleteEntity(id: string): Promise<boolean> {
        return invoke<boolean>('graph_delete_node', { id });
    }

    // =========================================================================
    // Edge Operations  
    // =========================================================================

    /**
     * Create an edge between two entities
     */
    async createRelationship(
        sourceId: string,
        targetId: string,
        edgeType: string,
        options?: { confidence?: number; sourceNote?: string }
    ): Promise<EdgeResponse> {
        return invoke<EdgeResponse>('graph_create_edge', {
            request: {
                source_id: sourceId,
                target_id: targetId,
                edge_type: edgeType,
                confidence: options?.confidence ?? 1.0,
                source_note: options?.sourceNote ?? null,
            },
        });
    }

    /**
     * Get edges connected to a node
     */
    async getRelationships(
        nodeId: string,
        direction?: 'outgoing' | 'incoming' | 'both'
    ): Promise<EdgeResponse[]> {
        return invoke<EdgeResponse[]>('graph_get_edges', {
            node_id: nodeId,
            direction: direction ?? 'both',
        });
    }

    /**
     * Delete an edge
     */
    async deleteRelationship(id: string): Promise<boolean> {
        return invoke<boolean>('graph_delete_edge', { id });
    }

    // =========================================================================
    // Smart Hydration (KEY FEATURE)
    // =========================================================================

    /**
     * Get entities for scanner hydration - ONLY if set has changed.
     * 
     * This is the key method that breaks the circular hydration loop.
     * Call this instead of getAllEntities() when hydrating the scanner.
     * 
     * @returns null if no hydration needed, entities array if scanner needs update
     */
    async getEntitiesForHydration(): Promise<EntityDefinition[] | null> {
        const response = await invoke<HydrateResponse>('graph_get_entities_for_hydration');

        if (!response.needs_hydration) {
            console.log('[TauriGraphRegistry] Skipping hydration - no changes');
            return null;
        }

        console.log(`[TauriGraphRegistry] Hydration needed - ${response.entity_count} entities`);
        return response.entities;
    }

    /**
     * Force get all entities (bypass change detection)
     */
    async forceGetAllEntities(): Promise<EntityDefinition[]> {
        return invoke<EntityDefinition[]>('graph_get_all_entities');
    }

    /**
     * Invalidate hydration cache (force next hydration)
     */
    async invalidateHydration(): Promise<void> {
        await invoke<void>('graph_invalidate_hydration');
    }

    // =========================================================================
    // Scan Ingest
    // =========================================================================

    /**
     * Ingest scan results into the graph.
     * 
     * Call this after scanning a document to:
     * 1. Register new entities from mentions
     * 2. Create edges from relations
     * 
     * This automatically invalidates hydration cache if new entities added.
     */
    async ingestScanResult(
        sourceNote: string,
        mentions: MentionInput[],
        relations: RelationInput[]
    ): Promise<IngestResponse> {
        return invoke<IngestResponse>('graph_ingest_scan_result', {
            request: {
                source_note: sourceNote,
                mentions,
                relations,
            },
        });
    }

    // =========================================================================
    // Stats
    // =========================================================================

    async getStats(): Promise<GraphStats> {
        return invoke<GraphStats>('graph_stats');
    }

    isReady(): boolean {
        return this.initialized;
    }
}

// Singleton instance
export const tauriGraphRegistry = new TauriGraphRegistry();
