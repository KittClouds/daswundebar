/**
 * RelationshipBridgeStore - Unified relationship instance storage
 * 
 * Single source of truth that bridges:
 * - SQLite unified_relationships table
 * - CozoDB network_relationship relation
 * - Blueprint Hub relationship type validation
 * - Network membership detection
 * - Fact Sheet display
 */

import { generateId } from '@/lib/utils/ids';
import { RelationshipSource } from '@/lib/relationships/types';
import type { UnifiedRelationship, RelationshipProvenance } from '@/lib/relationships/types';
import type { EntityKind } from '@/lib/types/entityTypes';
import type {
    RelationshipInstanceInput,
    RelationshipInstanceUpdate,
    ResolvedRelationshipInstance,
    ResolvedEntityRef,
    ResolvedRelationshipType,
    RelationshipInstanceQuery,
    GroupedRelationships,
    CandidateEntity,
    ApplicableRelationshipType,
} from './relationshipBridgeTypes';
import type { RelationshipTypeDef, RelationshipDirection, RelationshipCardinality } from '@/features/blueprint-hub/types';

// ============================================
// RELATIONSHIP BRIDGE STORE
// ============================================

class RelationshipBridgeStoreImpl {
    private initialized = false;
    private relationshipTypeCache: Map<string, RelationshipTypeDef> = new Map();
    private entityCache: Map<string, ResolvedEntityRef> = new Map();

    /**
     * Initialize the bridge store
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;

        // Load relationship types from Blueprint Hub into cache
        await this.refreshRelationshipTypeCache();

        this.initialized = true;
        console.log('[RelationshipBridgeStore] Initialized');
    }

    /**
     * Refresh the relationship type cache from Blueprint Hub
     */
    async refreshRelationshipTypeCache(): Promise<void> {
        try {
            // Use abstracted store (routes to Tauri or WASM automatically)
            const { getBlueprintStore } = await import('@/lib/storage/index');
            const blueprintStore = getBlueprintStore();

            // Get all relationship types from all versions
            const metas = await blueprintStore.getAllBlueprintMetas();

            for (const meta of metas) {
                const versions = await blueprintStore.getVersionsByBlueprintId(meta.blueprint_id);
                for (const version of versions) {
                    if (version.status === 'published' || version.status === 'draft') {
                        const types = await blueprintStore.getRelationshipTypesByVersionId(version.version_id);
                        for (const type of types) {
                            this.relationshipTypeCache.set(type.relationship_type_id, type);
                        }
                    }
                }
            }

            console.log(`[RelationshipBridgeStore] Cached ${this.relationshipTypeCache.size} relationship types`);
        } catch (err) {
            console.warn('[RelationshipBridgeStore] Failed to load relationship types:', err);
        }
    }

    /**
     * Create a relationship instance with type validation
     */
    async create(input: RelationshipInstanceInput): Promise<ResolvedRelationshipInstance> {
        // Validate relationship type if provided
        let relationshipType: ResolvedRelationshipType;

        if (input.relationshipTypeId) {
            const typeDef = this.relationshipTypeCache.get(input.relationshipTypeId);
            if (!typeDef) {
                throw new Error(`Unknown relationship type: ${input.relationshipTypeId}`);
            }
            relationshipType = this.typeDefToResolved(typeDef);
        } else if (input.relationshipTypeName) {
            // Find by name
            const typeDef = Array.from(this.relationshipTypeCache.values())
                .find(t => t.relationship_name === input.relationshipTypeName);

            if (typeDef) {
                relationshipType = this.typeDefToResolved(typeDef);
            } else {
                // Create ad-hoc type
                relationshipType = {
                    id: `adhoc_${input.relationshipTypeName}`,
                    name: input.relationshipTypeName,
                    displayLabel: input.relationshipTypeName,
                    direction: 'directed',
                    cardinality: 'many_to_many',
                };
            }
        } else {
            throw new Error('Either relationshipTypeId or relationshipTypeName must be provided');
        }

        // Resolve entities
        const sourceEntity = await this.resolveEntity(input.sourceEntityId);
        const targetEntity = await this.resolveEntity(input.targetEntityId);

        const now = new Date();
        const id = generateId();

        // Build provenance
        const provenance: RelationshipProvenance = {
            source: input.source,
            originId: `fact_sheet_${id}`,
            timestamp: now,
            confidence: input.confidence ?? 1.0,
            context: 'Created via Fact Sheet relationship editor',
        };

        // Create UnifiedRelationship for SQLite storage
        const unifiedRel: UnifiedRelationship = {
            id,
            sourceEntityId: input.sourceEntityId,
            targetEntityId: input.targetEntityId,
            type: relationshipType.name,
            inverseType: relationshipType.inverseLabel,
            bidirectional: relationshipType.direction === 'bidirectional',
            confidence: input.confidence ?? 1.0,
            confidenceBySource: { [input.source]: input.confidence ?? 1.0 },
            provenance: [provenance],
            namespace: input.namespace,
            attributes: {
                ...input.attributes,
                relationshipTypeId: relationshipType.id,
                networkId: input.networkId,
                validFrom: input.validFrom?.toISOString(),
                validTo: input.validTo?.toISOString(),
            },
            createdAt: now,
            updatedAt: now,
        };

        // Save to SQLite via RelationshipRegistry
        await this.saveToSQLite(unifiedRel);

        // Sync to CozoDB for graph queries
        await this.syncToCozoDB(unifiedRel, input.networkId);

        // Auto-enroll in matching networks (Phase 5: Network Auto-Membership)
        if (!input.networkId) {
            try {
                const { checkAndAutoEnroll } = await import('@/lib/networks/autoMembership');
                const enrolledNetworks = await checkAndAutoEnroll(
                    id,
                    input.sourceEntityId,
                    input.targetEntityId,
                    {
                        relationship_name: relationshipType.name,
                        display_label: relationshipType.displayLabel,
                        relationship_type_id: relationshipType.id,
                    },
                    sourceEntity.kind,
                    targetEntity.kind
                );

                if (enrolledNetworks.length > 0) {
                    console.log(`[RelationshipBridge] Auto-enrolled in ${enrolledNetworks.length} network(s):`,
                        enrolledNetworks.map(n => n.name).join(', '));
                }
            } catch (err) {
                // Non-fatal - don't block relationship creation if auto-enrollment fails
                console.warn('[RelationshipBridge] Auto-enrollment check failed:', err);
            }
        }

        // Return resolved instance
        const resolved: ResolvedRelationshipInstance = {
            id,
            sourceEntity,
            targetEntity,
            relationshipType,
            confidence: input.confidence ?? 1.0,
            sources: [input.source],
            validFrom: input.validFrom,
            validTo: input.validTo,
            attributes: input.attributes ?? {},
            createdAt: now,
            updatedAt: now,
        };

        // Add network if present
        if (input.networkId) {
            resolved.network = await this.resolveNetwork(input.networkId);
        }

        return resolved;
    }

    /**
     * Get all relationships for an entity (for Fact Sheet)
     */
    async getByEntity(entityId: string): Promise<ResolvedRelationshipInstance[]> {
        try {
            const { relationshipRegistry } = await import('@/lib/cozo-stubs/graph-adapters');
            const relationships = relationshipRegistry.getByEntity(entityId);
            return Promise.all(relationships.map(rel => this.unifiedToResolved(rel)));
        } catch (err) {
            console.error('[RelationshipBridgeStore] Failed to get relationships:', err);
            return [];
        }
    }

    /**
     * Get relationships grouped by type (for Fact Sheet display)
     */
    async getByEntityGroupedByType(entityId: string): Promise<GroupedRelationships[]> {
        const relationships = await this.getByEntity(entityId);

        // Group by relationship type
        const groups = new Map<string, GroupedRelationships>();

        for (const rel of relationships) {
            const typeId = rel.relationshipType.id;

            if (!groups.has(typeId)) {
                groups.set(typeId, {
                    type: rel.relationshipType,
                    outgoing: [],
                    incoming: [],
                    totalCount: 0,
                });
            }

            const group = groups.get(typeId)!;

            if (rel.sourceEntity.id === entityId) {
                group.outgoing.push(rel);
            } else {
                group.incoming.push(rel);
            }

            group.totalCount++;
        }

        return Array.from(groups.values());
    }

    /**
     * Get applicable relationship types for an entity kind (from Blueprint Hub)
     */
    async getApplicableTypes(entityKind: EntityKind): Promise<ApplicableRelationshipType[]> {
        const applicable: ApplicableRelationshipType[] = [];

        for (const typeDef of this.relationshipTypeCache.values()) {
            // Check if this type applies to the entity kind
            const isSource = typeDef.source_entity_kind === entityKind;
            const isTarget = typeDef.target_entity_kind === entityKind;

            if (!isSource && !isTarget) continue;

            if (isSource && isTarget) {
                // Same kind on both ends (e.g., CHARACTER knows CHARACTER)
                applicable.push({
                    id: typeDef.relationship_type_id,
                    name: typeDef.relationship_name,
                    displayLabel: typeDef.display_label,
                    inverseLabel: typeDef.inverse_label,
                    direction: 'both',
                    otherEntityKind: entityKind,
                    cardinality: typeDef.cardinality,
                    instanceCount: 0, // TODO: Count actual instances
                    category: typeDef.pattern_category,
                });
            } else if (isSource) {
                // Entity is the source
                applicable.push({
                    id: typeDef.relationship_type_id,
                    name: typeDef.relationship_name,
                    displayLabel: typeDef.display_label,
                    inverseLabel: typeDef.inverse_label,
                    direction: 'outgoing',
                    otherEntityKind: typeDef.target_entity_kind as EntityKind,
                    cardinality: typeDef.cardinality,
                    instanceCount: 0,
                    category: typeDef.pattern_category,
                });
            } else {
                // Entity is the target
                applicable.push({
                    id: typeDef.relationship_type_id,
                    name: typeDef.relationship_name,
                    displayLabel: typeDef.inverse_label || typeDef.display_label,
                    inverseLabel: typeDef.display_label,
                    direction: 'incoming',
                    otherEntityKind: typeDef.source_entity_kind as EntityKind,
                    cardinality: typeDef.cardinality,
                    instanceCount: 0,
                    category: typeDef.pattern_category,
                });
            }
        }

        return applicable;
    }

    /**
     * Get candidate target entities for a relationship type
     */
    async getCandidateTargets(
        sourceEntityId: string,
        relationshipTypeId: string
    ): Promise<CandidateEntity[]> {
        const typeDef = this.relationshipTypeCache.get(relationshipTypeId);
        if (!typeDef) return [];

        try {
            // Get entities of the target kind using EntityRegistryAdapter
            const { entityRegistry } = await import('@/lib/cozo-stubs/graph-adapters');
            const entities = entityRegistry.getEntitiesByKind(typeDef.target_entity_kind as EntityKind);

            // Get existing relationships to mark duplicates
            const existingRels = await this.getByEntity(sourceEntityId);
            const existingTargetIds = new Set(
                existingRels
                    .filter(r => r.relationshipType.id === relationshipTypeId)
                    .map(r => r.targetEntity.id)
            );

            return entities
                .filter(e => e.id !== sourceEntityId) // Exclude self
                .map(e => ({
                    id: e.id,
                    name: e.label,
                    kind: e.kind,
                    noteId: e.firstNote,
                    hasExistingRelationship: existingTargetIds.has(e.id),
                }));
        } catch (err) {
            console.error('[RelationshipBridgeStore] Failed to get candidates:', err);
            return [];
        }
    }

    /**
     * Delete a relationship instance
     */
    async delete(relationshipId: string): Promise<boolean> {
        try {
            const { relationshipRegistry } = await import('@/lib/cozo-stubs/graph-adapters');
            const deleted = relationshipRegistry.delete(relationshipId);

            if (deleted) {
                // Also delete from CozoDB
                await this.deleteFromCozoDB(relationshipId);
            }

            return deleted;
        } catch (err) {
            console.error('[RelationshipBridgeStore] Failed to delete relationship:', err);
            return false;
        }
    }

    /**
     * Update a relationship instance
     */
    async update(relationshipId: string, updates: RelationshipInstanceUpdate): Promise<ResolvedRelationshipInstance | null> {
        try {
            const { relationshipRegistry } = await import('@/lib/cozo-stubs/graph-adapters');
            const existing = relationshipRegistry.get(relationshipId);

            if (!existing) {
                console.warn(`[RelationshipBridgeStore] Relationship not found: ${relationshipId}`);
                return null;
            }

            const now = new Date();

            // Build updated attributes
            const updatedAttributes = {
                ...existing.attributes,
                ...(updates.attributes || {}),
            };

            // Handle temporal bounds
            if (updates.validFrom !== undefined) {
                updatedAttributes.validFrom = updates.validFrom ? updates.validFrom.toISOString() : null;
            }
            if (updates.validTo !== undefined) {
                updatedAttributes.validTo = updates.validTo ? updates.validTo.toISOString() : null;
            }

            // Build the updated relationship
            const updated: UnifiedRelationship = {
                ...existing,
                confidence: updates.confidence ?? existing.confidence,
                attributes: updatedAttributes,
                updatedAt: now,
            };

            // Update confidence by source if changed
            if (updates.confidence !== undefined) {
                updated.confidenceBySource = {
                    ...existing.confidenceBySource,
                    [RelationshipSource.MANUAL]: updates.confidence,
                };
            }

            // Save to SQLite
            await this.saveToSQLite(updated);

            // Sync to CozoDB if has network
            const networkId = updated.attributes?.networkId as string | undefined;
            if (networkId) {
                await this.syncToCozoDB(updated, networkId);
            }

            // Clear cache for affected entities
            this.entityCache.delete(updated.sourceEntityId);
            this.entityCache.delete(updated.targetEntityId);

            return this.unifiedToResolved(updated);
        } catch (err) {
            console.error('[RelationshipBridgeStore] Failed to update relationship:', err);
            return null;
        }
    }

    /**
     * Query relationships with filters
     */
    async query(q: RelationshipInstanceQuery): Promise<ResolvedRelationshipInstance[]> {
        try {
            const { relationshipRegistry } = await import('@/lib/cozo-stubs/graph-adapters');

            const queryParams: any = {
                limit: q.limit,
                offset: q.offset,
                minConfidence: q.minConfidence,
                namespace: q.namespace,
            };

            if (q.entityId) queryParams.entityId = q.entityId;
            if (q.sourceEntityId) queryParams.sourceId = q.sourceEntityId;
            if (q.targetEntityId) queryParams.targetId = q.targetEntityId;
            if (q.relationshipTypeName) queryParams.type = q.relationshipTypeName;
            if (q.sources) queryParams.sources = q.sources;

            const relationships = relationshipRegistry.query(queryParams);
            return Promise.all(relationships.map(rel => this.unifiedToResolved(rel)));
        } catch (err) {
            console.error('[RelationshipBridgeStore] Query failed:', err);
            return [];
        }
    }

    /**
     * Get instance count for a relationship type
     */
    async getInstanceCount(relationshipTypeId: string): Promise<number> {
        const typeDef = this.relationshipTypeCache.get(relationshipTypeId);
        if (!typeDef) return 0;

        try {
            const { relationshipRegistry } = await import('@/lib/cozo-stubs/graph-adapters');
            const relationships = relationshipRegistry.getByType(typeDef.relationship_name);
            return relationships.length;
        } catch {
            return 0;
        }
    }

    /**
     * Get instance counts for all relationship types (batch query)
     */
    async getInstanceCountsByType(): Promise<Map<string, number>> {
        const counts = new Map<string, number>();

        try {
            const { relationshipRegistry } = await import('@/lib/cozo-stubs/graph-adapters');
            const allRelationships = relationshipRegistry.getAll();

            // Count by type name
            const typeNameCounts = new Map<string, number>();
            for (const rel of allRelationships) {
                const count = typeNameCounts.get(rel.type) || 0;
                typeNameCounts.set(rel.type, count + 1);
            }

            // Map back to type IDs
            for (const [typeId, typeDef] of this.relationshipTypeCache) {
                const count = typeNameCounts.get(typeDef.relationship_name) || 0;
                counts.set(typeId, count);
            }
        } catch (err) {
            console.error('[RelationshipBridgeStore] Failed to get instance counts:', err);
        }

        return counts;
    }

    /**
     * Get all instances of a specific relationship type
     */
    async getInstancesByType(relationshipTypeId: string): Promise<ResolvedRelationshipInstance[]> {
        const typeDef = this.relationshipTypeCache.get(relationshipTypeId);
        if (!typeDef) return [];

        try {
            const { relationshipRegistry } = await import('@/lib/cozo-stubs/graph-adapters');
            const relationships = relationshipRegistry.getByType(typeDef.relationship_name);
            return Promise.all(relationships.map(rel => this.unifiedToResolved(rel)));
        } catch (err) {
            console.error('[RelationshipBridgeStore] Failed to get instances by type:', err);
            return [];
        }
    }

    /**
     * Get all cached relationship types
     */
    getAllRelationshipTypes(): RelationshipTypeDef[] {
        return Array.from(this.relationshipTypeCache.values());
    }

    // ==================== PRIVATE HELPERS ====================

    private typeDefToResolved(typeDef: RelationshipTypeDef): ResolvedRelationshipType {
        return {
            id: typeDef.relationship_type_id,
            name: typeDef.relationship_name,
            displayLabel: typeDef.display_label,
            inverseLabel: typeDef.inverse_label,
            direction: typeDef.direction,
            cardinality: typeDef.cardinality,
            verbPatterns: typeDef.verb_patterns,
        };
    }

    private async resolveEntity(entityId: string): Promise<ResolvedEntityRef> {
        // Check cache first
        if (this.entityCache.has(entityId)) {
            return this.entityCache.get(entityId)!;
        }

        try {
            // Use EntityRegistryAdapter which works synchronously with CozoDB
            const { entityRegistry } = await import('@/lib/cozo-stubs/graph-adapters');
            const entity = entityRegistry.getEntityById(entityId);

            if (entity) {
                const resolved: ResolvedEntityRef = {
                    id: entity.id,
                    name: entity.label,
                    kind: entity.kind,
                    noteId: entity.firstNote,
                };
                this.entityCache.set(entityId, resolved);
                return resolved;
            }
        } catch {
            // Entity not found
        }

        // Return placeholder
        return {
            id: entityId,
            name: `Unknown (${entityId.substring(0, 8)}...)`,
            kind: 'CHARACTER' as EntityKind, // Default fallback
        };
    }

    private async resolveNetwork(networkId: string): Promise<{ id: string; name: string; kind: any } | undefined> {
        try {
            const { loadNetworkInstance } = await import('@/lib/networks/storage');
            const network = await loadNetworkInstance(networkId);

            if (network) {
                return {
                    id: network.id,
                    name: network.name,
                    kind: network.schemaId, // Simplified; could resolve schema's kind
                };
            }
        } catch {
            return undefined;
        }
    }

    private async unifiedToResolved(rel: UnifiedRelationship): Promise<ResolvedRelationshipInstance> {
        const sourceEntity = await this.resolveEntity(rel.sourceEntityId);
        const targetEntity = await this.resolveEntity(rel.targetEntityId);

        // Try to find the relationship type
        let relationshipType: ResolvedRelationshipType;
        const typeId = rel.attributes?.relationshipTypeId as string | undefined;

        if (typeId && this.relationshipTypeCache.has(typeId)) {
            relationshipType = this.typeDefToResolved(this.relationshipTypeCache.get(typeId)!);
        } else {
            // Ad-hoc type from the relationship name
            relationshipType = {
                id: `adhoc_${rel.type}`,
                name: rel.type,
                displayLabel: rel.type,
                inverseLabel: rel.inverseType,
                direction: rel.bidirectional ? 'bidirectional' : 'directed',
                cardinality: 'many_to_many',
            };
        }

        const sources = Object.keys(rel.confidenceBySource) as RelationshipSource[];

        const resolved: ResolvedRelationshipInstance = {
            id: rel.id,
            sourceEntity,
            targetEntity,
            relationshipType,
            confidence: rel.confidence,
            sources,
            validFrom: rel.attributes?.validFrom ? new Date(rel.attributes.validFrom as string) : undefined,
            validTo: rel.attributes?.validTo ? new Date(rel.attributes.validTo as string) : undefined,
            attributes: rel.attributes,
            createdAt: rel.createdAt,
            updatedAt: rel.updatedAt,
        };

        // Resolve network if present
        const networkId = rel.attributes?.networkId as string | undefined;
        if (networkId) {
            resolved.network = await this.resolveNetwork(networkId);
        }

        // Fetch network memberships (Phase 5: Network Auto-Membership)
        try {
            const { getRelationshipNetworkMemberships } = await import('@/lib/networks/autoMembership');
            resolved.networkMemberships = await getRelationshipNetworkMemberships(
                rel.sourceEntityId,
                rel.targetEntityId,
                rel.type
            );
        } catch (err) {
            // Non-fatal - network membership lookup is optional
            console.warn('[RelationshipBridgeStore] Failed to fetch network memberships:', err);
        }

        return resolved;
    }

    private async saveToSQLite(rel: UnifiedRelationship): Promise<void> {
        try {
            const { relationshipRegistry } = await import('@/lib/cozo-stubs/graph-adapters');
            relationshipRegistry.addWithoutPersist(rel);
        } catch (err) {
            console.error('[RelationshipBridgeStore] Failed to save to SQLite:', err);
        }
    }

    private async syncToCozoDB(rel: UnifiedRelationship, networkId?: string): Promise<void> {
        if (!networkId) return;

        try {
            const { cozoDb } = await import('@/lib/cozo-stubs/db');
            if (!cozoDb.isReady()) return;

            const { NETWORK_RELATIONSHIP_QUERIES } = await import('@/lib/cozo-stubs/schema');

            await cozoDb.run(NETWORK_RELATIONSHIP_QUERIES.upsert, {
                id: rel.id,
                network_id: networkId,
                source_id: rel.sourceEntityId,
                target_id: rel.targetEntityId,
                relationship_code: rel.type,
                inverse_code: rel.inverseType || null,
                start_date: rel.attributes?.validFrom ? new Date(rel.attributes.validFrom as string).getTime() / 1000 : null,
                end_date: rel.attributes?.validTo ? new Date(rel.attributes.validTo as string).getTime() / 1000 : null,
                strength: rel.confidence,
                notes: null,
                attributes: JSON.stringify(rel.attributes),
                created_at: rel.createdAt.getTime() / 1000,
                updated_at: rel.updatedAt.getTime() / 1000,
                group_id: networkId,
                scope_type: 'network',
                confidence: rel.confidence,
                extraction_methods: rel.provenance.map(p => p.source),
            });
        } catch (err) {
            console.error('[RelationshipBridgeStore] Failed to sync to CozoDB:', err);
        }
    }

    private async deleteFromCozoDB(relationshipId: string): Promise<void> {
        try {
            const { cozoDb } = await import('@/lib/cozo-stubs/db');
            if (!cozoDb.isReady()) return;

            const { NETWORK_RELATIONSHIP_QUERIES } = await import('@/lib/cozo-stubs/schema');
            await cozoDb.run(NETWORK_RELATIONSHIP_QUERIES.delete, { id: relationshipId });
        } catch (err) {
            console.error('[RelationshipBridgeStore] Failed to delete from CozoDB:', err);
        }
    }
}

// Singleton export
export const relationshipBridgeStore = new RelationshipBridgeStoreImpl();
