/**
 * Graph Data Service
 * 
 * Transforms CozoDB query results into renderer-compatible GraphData structures.
 * Uses SmartGraphRegistry for proper data access (entities/relationships from Rust).
 */

import { cozoDb } from '@/lib/cozo-stubs/db';
// MIGRATED: Use smartGraphRegistry instead of unifiedRegistry
import { smartGraphRegistry, type RegisteredEntity } from '@/lib/tauri';
import type { GraphData, GraphNode, GraphEdge, GraphStats } from '../types/graph-types';
import { getEntityColor } from '../types/graph-types';

// Adapter type for compatibility
type CozoEntity = RegisteredEntity;
interface CozoRelationship {
    id: string;
    sourceId: string;
    targetId: string;
    type: string;
    inverseType?: string;
    bidirectional: boolean;
    confidence: number;
    namespace?: string;
    createdAt: Date;
    updatedAt: Date;
}

export class GraphDataService {
    /**
     * Fetch global graph (all entities/relationships)
     */
    async getGlobalGraph(limit = 500): Promise<GraphData> {
        await this.ensureReady();

        try {
            // Use SmartGraphRegistry (SYNC - from local cache!)
            const entities = smartGraphRegistry.getAllEntities();
            const nodes = this.transformCozoEntities(entities.slice(0, limit));

            if (nodes.length === 0) {
                console.log('[GraphDataService] No entities found in SmartGraphRegistry');
                return { nodes: [], links: [] };
            }

            // Fetch relationships
            const nodeIds = new Set(nodes.map(n => n.id));
            const relationships = await this.fetchAllRelationships();
            const links = this.transformCozoRelationships(relationships, nodeIds);

            console.log(`[GraphDataService] Global graph: ${nodes.length} nodes, ${links.length} links`);
            return { nodes, links };
        } catch (err) {
            console.error('[GraphDataService] getGlobalGraph failed:', err);
            return { nodes: [], links: [] };
        }
    }

    /**
     * Fetch graph for a specific scope (note or folder)
     */
    async getGraphForScope(groupId: string): Promise<GraphData> {
        await this.ensureReady();

        try {
            // For now, filter global entities by checking if they mention this note
            const allEntities = smartGraphRegistry.getAllEntities();

            // Filter entities that have the noteId in their mentions
            const scopedEntities = allEntities.filter(e => {
                if (e.mentionsByNote && e.mentionsByNote.has(groupId)) return true;
                if (e.firstNote === groupId) return true;
                return false;
            });

            const nodes = this.transformCozoEntities(scopedEntities);
            const nodeIds = new Set(nodes.map(n => n.id));

            const relationships = await this.fetchAllRelationships();
            const links = this.transformCozoRelationships(relationships, nodeIds);

            return { nodes, links };
        } catch (err) {
            console.error('[GraphDataService] getGraphForScope failed:', err);
            return { nodes: [], links: [] };
        }
    }

    /**
     * Alias for backwards compatibility
     */
    async getVisualizationGraph(groupId: string): Promise<GraphData> {
        return this.getGraphForScope(groupId);
    }

    /**
     * Get graph statistics
     */
    calculateStats(data: GraphData): GraphStats {
        const nodeCount = data.nodes.length;
        const edgeCount = data.links.length;

        // Graph density = actual edges / possible edges
        const possibleEdges = nodeCount > 1 ? (nodeCount * (nodeCount - 1)) / 2 : 1;
        const density = edgeCount / possibleEdges;

        // Average degree = 2 * edges / nodes
        const averageDegree = nodeCount > 0 ? (2 * edgeCount) / nodeCount : 0;

        return {
            nodeCount,
            edgeCount,
            density: Math.min(density, 1),
            averageDegree,
        };
    }

    // ==================== Private Helpers ====================

    private async ensureReady(): Promise<void> {
        if (!cozoDb.isReady()) {
            await cozoDb.init();
        }
        // SmartGraphRegistry init
        await smartGraphRegistry.init();
    }

    /**
     * Transform CozoEntity[] to GraphNode[]
     */
    private transformCozoEntities(entities: CozoEntity[]): GraphNode[] {
        return entities.map(entity => ({
            id: entity.id,
            label: entity.label,
            type: entity.kind,
            color: getEntityColor(entity.kind),
            size: Math.min(10 + (entity.totalMentions || 1), 30),
            metadata: {
                subtype: entity.subtype,
                totalMentions: entity.totalMentions,
                firstNote: entity.firstNote,
                createdBy: entity.createdBy,
            },
        }));
    }

    /**
     * Fetch all relationships from CozoDB using correct schema
     */
    private async fetchAllRelationships(): Promise<CozoRelationship[]> {
        try {
            // Query the relationships relation directly
            const query = `
                ?[id, source_id, target_id, type, inverse_type, bidirectional, confidence, namespace, created_at, updated_at] := 
                    *relationships{id, source_id, target_id, type, inverse_type, bidirectional, confidence, namespace, created_at, updated_at}
            `;

            const result = cozoDb.runQuery(query);

            if (!result.rows || result.rows.length === 0) {
                return [];
            }

            return result.rows.map((row: any[]) => ({
                id: row[0],
                sourceId: row[1],
                targetId: row[2],
                type: row[3],
                inverseType: row[4],
                bidirectional: row[5],
                confidence: row[6],
                namespace: row[7],
                createdAt: new Date(row[8]),
                updatedAt: new Date(row[9]),
            }));
        } catch (err) {
            // Relation might not exist yet
            console.log('[GraphDataService] No relationships found (relation may not exist)');
            return [];
        }
    }

    /**
     * Transform CozoRelationship[] to GraphEdge[]
     */
    private transformCozoRelationships(relationships: CozoRelationship[], validNodeIds: Set<string>): GraphEdge[] {
        return relationships
            .filter(rel => {
                // Only include edges where both nodes exist
                return validNodeIds.has(rel.sourceId) && validNodeIds.has(rel.targetId);
            })
            .filter(rel => {
                // Skip self-loops
                return rel.sourceId !== rel.targetId;
            })
            .map(rel => ({
                id: rel.id,
                source: rel.sourceId,
                target: rel.targetId,
                type: rel.type,
                weight: Math.round(rel.confidence * 10) || 1,
                label: rel.type,
            }));
    }
}

// Singleton instance
export const graphDataService = new GraphDataService();
