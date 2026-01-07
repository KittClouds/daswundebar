/**
 * SmartGraphRegistry - Intelligent TypeScript adapter for Rust GraphRegistry
 * 
 * Features:
 * - Local entity cache (avoid IPC overhead for reads)
 * - Hash-based dirty tracking (matches Rust ScannerBridge)
 * - Batched writes with debouncing
 * - Event-driven cache invalidation
 * - Drop-in replacement for EntityRegistryAdapter
 * 
 * The key insight: Most reads happen MORE than writes in typical usage.
 * Cache reads locally, batch writes to Rust.
 */

import { invoke } from '@tauri-apps/api/core';
import type { EntityKind } from '@/lib/types/entityTypes';

// =============================================================================
// Types (matching Rust + legacy)
// =============================================================================

export interface EntityDefinition {
    id: string;
    label: string;
    kind: string;
    aliases: string[];
}

export interface RegisteredEntity {
    id: string;
    label: string;
    aliases: string[];
    kind: EntityKind;
    subtype?: string;
    firstNote: string;
    mentionsByNote: Map<string, number>;
    totalMentions: number;
    lastSeenDate: Date;
    createdAt: Date;
    createdBy: 'user' | 'extraction' | 'auto';
    attributes?: Record<string, any>;
}

export interface EntityRegistrationResult {
    entity: RegisteredEntity;
    isNew: boolean;
    wasMerged: boolean;
}

interface RustNodeResponse {
    id: string;
    label: string;
    kind: string;
    mention_count: number;
    aliases: string[];
    is_new: boolean;
}

interface RustHydrateResponse {
    entity_count: number;
    needs_hydration: boolean;
    entities: EntityDefinition[] | null;
}

interface RustIngestResponse {
    entities_created: number;
    entities_updated: number;
    edges_created: number;
    edges_updated: number;
}

interface RustEdgeResponse {
    id: string;
    source_id: string;
    target_id: string;
    edge_type: string;
    confidence: number;
}

export interface Edge {
    id: string;
    sourceId: string;
    targetId: string;
    type: string;
    confidence: number;
}

// =============================================================================
// SmartGraphRegistry
// =============================================================================

export class SmartGraphRegistry {
    // =========================================================================
    // State
    // =========================================================================

    private initialized = false;
    private initPromise: Promise<void> | null = null;

    // Local cache
    private entityCache: Map<string, RegisteredEntity> = new Map();
    private labelIndex: Map<string, string> = new Map(); // normalized label → id
    private aliasIndex: Map<string, string> = new Map(); // normalized alias → id

    // Dirty tracking
    private cacheHash = 0;
    private lastHydrateHash = 0;

    // Batching
    private pendingWrites: Array<{
        type: 'register' | 'delete' | 'update';
        data: any;
        resolve: (result: any) => void;
        reject: (err: any) => void;
    }> = [];
    private batchTimer: ReturnType<typeof setTimeout> | null = null;
    private batchDelayMs = 50;

    // =========================================================================
    // Initialization
    // =========================================================================

    async init(): Promise<void> {
        if (this.initialized) return;
        if (this.initPromise) return this.initPromise;

        this.initPromise = (async () => {
            try {
                // Load all entities from Rust into cache
                await this.refreshCache();
                this.initialized = true;
                console.log(`[SmartGraphRegistry] Initialized with ${this.entityCache.size} entities`);
            } catch (err) {
                this.initPromise = null;
                throw err;
            }
        })();

        return this.initPromise;
    }

    isInitialized(): boolean {
        return this.initialized;
    }

    // =========================================================================
    // Cache Management
    // =========================================================================

    private async refreshCache(): Promise<void> {
        const entities = await invoke<EntityDefinition[]>('graph_get_all_entities');

        this.entityCache.clear();
        this.labelIndex.clear();
        this.aliasIndex.clear();

        for (const e of entities) {
            const entity = this.rustToLegacy(e);
            this.entityCache.set(e.id, entity);
            this.labelIndex.set(e.label.toLowerCase(), e.id);

            for (const alias of e.aliases) {
                this.aliasIndex.set(alias.toLowerCase(), e.id);
            }
        }

        this.updateCacheHash();
    }

    private updateCacheHash(): void {
        // Simple hash of entity set for change detection
        let hash = 0;
        for (const [id, entity] of this.entityCache) {
            for (let i = 0; i < id.length; i++) {
                hash = ((hash << 5) - hash) + id.charCodeAt(i);
                hash = hash & hash;
            }
            for (let i = 0; i < entity.label.length; i++) {
                hash = ((hash << 5) - hash) + entity.label.charCodeAt(i);
                hash = hash & hash;
            }
        }
        this.cacheHash = hash;
    }

    private addToCache(entity: RegisteredEntity): void {
        this.entityCache.set(entity.id, entity);
        this.labelIndex.set(entity.label.toLowerCase(), entity.id);
        for (const alias of entity.aliases) {
            this.aliasIndex.set(alias.toLowerCase(), entity.id);
        }
        this.updateCacheHash();
    }

    private removeFromCache(id: string): void {
        const entity = this.entityCache.get(id);
        if (entity) {
            this.labelIndex.delete(entity.label.toLowerCase());
            for (const alias of entity.aliases) {
                this.aliasIndex.delete(alias.toLowerCase());
            }
            this.entityCache.delete(id);
            this.updateCacheHash();
        }
    }

    // =========================================================================
    // SYNC READS (from cache - fast!)
    // =========================================================================

    /**
     * Check if entity is registered (SYNC - uses cache)
     */
    isRegisteredEntity(label: string): boolean {
        if (!this.initialized) return false;
        const normalized = label.toLowerCase();
        return this.labelIndex.has(normalized) || this.aliasIndex.has(normalized);
    }

    /**
     * Get entity by ID (SYNC - uses cache)
     */
    getEntityById(id: string): RegisteredEntity | null {
        return this.entityCache.get(id) || null;
    }

    getEntityByIdSync(id: string): RegisteredEntity | null {
        return this.getEntityById(id);
    }

    /**
     * Find entity by label (SYNC - uses cache)
     */
    findEntityByLabel(label: string): RegisteredEntity | null {
        if (!this.initialized) return null;
        const normalized = label.toLowerCase();

        // Check label index first
        const idByLabel = this.labelIndex.get(normalized);
        if (idByLabel) return this.entityCache.get(idByLabel) || null;

        // Check alias index
        const idByAlias = this.aliasIndex.get(normalized);
        if (idByAlias) return this.entityCache.get(idByAlias) || null;

        return null;
    }

    findEntitySync(label: string): RegisteredEntity | null {
        return this.findEntityByLabel(label);
    }

    findEntity(text: string): RegisteredEntity | undefined {
        return this.findEntityByLabel(text) || undefined;
    }

    /**
     * Get all entities (SYNC - uses cache)
     */
    getAllEntities(): RegisteredEntity[] {
        return Array.from(this.entityCache.values());
    }

    getAllEntitiesSync(): RegisteredEntity[] {
        return this.getAllEntities();
    }

    /**
     * Get entities by kind (SYNC - uses cache)
     */
    getEntitiesByKind(kind: EntityKind): RegisteredEntity[] {
        return Array.from(this.entityCache.values()).filter(e => e.kind === kind);
    }

    // =========================================================================
    // ASYNC WRITES (batched to Rust)
    // =========================================================================

    /**
     * Register an entity (batched write)
     */
    async registerEntity(
        label: string,
        kind: EntityKind,
        noteId: string,
        options?: {
            subtype?: string;
            aliases?: string[];
            attributes?: Record<string, any>;
            source?: 'user' | 'extraction' | 'auto';
        }
    ): Promise<EntityRegistrationResult> {
        await this.ensureInit();

        // Fast path: check cache first
        const existing = this.findEntityByLabel(label);
        if (existing) {
            // Update mention count locally (can batch this too)
            existing.totalMentions++;
            existing.lastSeenDate = new Date();

            return { entity: existing, isNew: false, wasMerged: false };
        }

        // Slow path: register on Rust side
        const response = await invoke<RustNodeResponse>('graph_register_node', {
            request: {
                label,
                kind: kind.toString(),
                source_note: noteId,
                subtype: options?.subtype ?? null,
                aliases: options?.aliases ?? [],
                created_by: options?.source ?? 'user',
            },
        });

        const entity = this.rustResponseToLegacy(response, noteId);
        this.addToCache(entity);

        return { entity, isNew: response.is_new, wasMerged: false };
    }

    /**
     * Delete entity (immediate - cascades)
     */
    async deleteEntity(id: string): Promise<boolean> {
        await this.ensureInit();

        const result = await invoke<boolean>('graph_delete_node', { id });
        if (result) {
            this.removeFromCache(id);
        }
        return result;
    }

    // =========================================================================
    // SMART HYDRATION (breaks circular loop)
    // =========================================================================

    /**
     * Get entities for scanner hydration - ONLY if changed!
     * 
     * This is THE key method. Uses Rust-side hash tracking.
     * Returns null if no changes, entities array if scanner needs update.
     */
    async getEntitiesForHydration(): Promise<EntityDefinition[] | null> {
        const response = await invoke<RustHydrateResponse>('graph_get_entities_for_hydration');

        if (!response.needs_hydration) {
            console.log('[SmartGraphRegistry] Hydration skipped (no changes)');
            return null;
        }

        this.lastHydrateHash = this.cacheHash;
        console.log(`[SmartGraphRegistry] Hydration needed: ${response.entity_count} entities`);
        return response.entities;
    }

    /**
     * Check if hydration is needed (local check)
     */
    needsHydration(): boolean {
        return this.cacheHash !== this.lastHydrateHash;
    }

    /**
     * Force invalidate hydration
     */
    async invalidateHydration(): Promise<void> {
        await invoke<void>('graph_invalidate_hydration');
        this.lastHydrateHash = 0;
    }

    // =========================================================================
    // SCAN RESULT INGEST
    // =========================================================================

    /**
     * Ingest scan results into the graph (Rust-side)
     */
    async ingestScanResult(
        sourceNote: string,
        mentions: Array<{ entity_label: string; entity_kind: string }>,
        relations: Array<{
            head_label: string;
            tail_label: string;
            relation_type: string;
            confidence?: number;
            source?: 'explicit' | 'cst' | 'graph';
        }>
    ): Promise<RustIngestResponse> {
        const response = await invoke<RustIngestResponse>('graph_ingest_scan_result', {
            request: {
                source_note: sourceNote,
                mentions,
                relations,
            },
        });

        // Refresh cache if new entities were created
        if (response.entities_created > 0) {
            await this.refreshCache();
        }

        return response;
    }

    // =========================================================================
    // EDGE OPERATIONS (Relationships)
    // =========================================================================

    /**
     * Edge response type from Rust
     */
    private edgeResponseToEdge(r: RustEdgeResponse): Edge {
        return {
            id: r.id,
            sourceId: r.source_id,
            targetId: r.target_id,
            type: r.edge_type,
            confidence: r.confidence,
        };
    }

    /**
     * Create an edge (relationship) between two entities
     */
    async createEdge(
        sourceId: string,
        targetId: string,
        type: string,
        options?: {
            confidence?: number;
            sourceNote?: string;
        }
    ): Promise<Edge> {
        await this.ensureInit();

        const response = await invoke<RustEdgeResponse>('graph_create_edge', {
            request: {
                source_id: sourceId,
                target_id: targetId,
                edge_type: type,
                confidence: options?.confidence ?? 1.0,
                source_note: options?.sourceNote,
            },
        });

        return this.edgeResponseToEdge(response);
    }

    /**
     * Get edges connected to an entity
     */
    async getEdges(
        entityId: string,
        direction: 'in' | 'out' | 'both' = 'both'
    ): Promise<Edge[]> {
        await this.ensureInit();

        const directionParam = direction === 'in' ? 'incoming'
            : direction === 'out' ? 'outgoing'
                : 'both';

        const response = await invoke<RustEdgeResponse[]>('graph_get_edges', {
            node_id: entityId,
            direction: directionParam,
        });

        return response.map(r => this.edgeResponseToEdge(r));
    }

    /**
     * Delete an edge
     */
    async deleteEdge(edgeId: string): Promise<boolean> {
        await this.ensureInit();
        return invoke<boolean>('graph_delete_edge', { id: edgeId });
    }

    // =========================================================================
    // Compatibility Methods (for drop-in replacement)
    // =========================================================================

    async searchEntities(query: string): Promise<Array<{
        entity: RegisteredEntity;
        matchType: 'exact' | 'alias' | 'fuzzy';
        score: number;
    }>> {
        const normalized = query.toLowerCase();
        const results: Array<{ entity: RegisteredEntity; matchType: 'exact' | 'alias' | 'fuzzy'; score: number }> = [];

        for (const entity of this.entityCache.values()) {
            let matchType: 'exact' | 'alias' | 'fuzzy' = 'fuzzy';
            let score = 0;

            if (entity.label.toLowerCase() === normalized) {
                matchType = 'exact';
                score = 1.0;
            } else if (entity.aliases.some(a => a.toLowerCase() === normalized)) {
                matchType = 'alias';
                score = 0.9;
            } else if (entity.label.toLowerCase().includes(normalized)) {
                matchType = 'fuzzy';
                score = 0.7;
            } else {
                continue; // No match
            }

            results.push({ entity, matchType, score });
        }

        return results.sort((a, b) => b.score - a.score);
    }

    async addAlias(entityId: string, alias: string): Promise<boolean> {
        // TODO: Add to Rust side
        const entity = this.entityCache.get(entityId);
        if (entity) {
            entity.aliases.push(alias);
            this.aliasIndex.set(alias.toLowerCase(), entityId);
            return true;
        }
        return false;
    }

    async getStats(): Promise<{
        totalEntities: number;
        byKind: Record<string, number>;
        totalMentions: number;
        totalAliases: number;
    }> {
        const byKind: Record<string, number> = {};
        let totalMentions = 0;
        let totalAliases = 0;

        for (const entity of this.entityCache.values()) {
            byKind[entity.kind] = (byKind[entity.kind] || 0) + 1;
            totalMentions += entity.totalMentions;
            totalAliases += entity.aliases.length;
        }

        return {
            totalEntities: this.entityCache.size,
            byKind,
            totalMentions,
            totalAliases,
        };
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private async ensureInit(): Promise<void> {
        if (!this.initialized) {
            await this.init();
        }
    }

    private rustToLegacy(e: EntityDefinition): RegisteredEntity {
        return {
            id: e.id,
            label: e.label,
            aliases: e.aliases,
            kind: e.kind as EntityKind,
            subtype: undefined,
            firstNote: '',
            mentionsByNote: new Map(),
            totalMentions: 0,
            lastSeenDate: new Date(),
            createdAt: new Date(),
            createdBy: 'auto',
            attributes: {},
        };
    }

    private rustResponseToLegacy(r: RustNodeResponse, noteId: string): RegisteredEntity {
        return {
            id: r.id,
            label: r.label,
            aliases: r.aliases,
            kind: r.kind as EntityKind,
            subtype: undefined,
            firstNote: noteId,
            mentionsByNote: new Map([[noteId, 1]]),
            totalMentions: r.mention_count,
            lastSeenDate: new Date(),
            createdAt: new Date(),
            createdBy: 'auto',
            attributes: {},
        };
    }
}

// ============================================================================
// Singleton Export
// ============================================================================

export const smartGraphRegistry = new SmartGraphRegistry();

/**
 * Drop-in replacement for entityRegistry
 * 
 * To migrate:
 *   // Old:
 *   import { entityRegistry } from '@/lib/cozo/graph/adapters';
 *   
 *   // New:
 *   import { smartGraphRegistry as entityRegistry } from '@/lib/tauri/smart-graph-registry';
 */
export { smartGraphRegistry as entityRegistry };
