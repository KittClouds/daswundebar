/**
 * Database Facade - Unified API for storage operations
 * 
 * Routes to appropriate backends:
 * - SurrealDB for notes, folders, calendar, entities
 * - Tauri CozoDB for graph/relationships  
 * - Quarantine stubs for deprecated SQLite methods
 * 
 * @module lib/db/facade
 */

import * as surreal from '@/lib/tauri/surreal-bridge';
import { calendarBridge } from '@/lib/tauri/calendar-bridge';
import { networkBridge } from '@/lib/tauri/network-bridge';
import { quarantineStub } from './quarantine';

import type {
    SQLiteNode,
    SQLiteNodeInput,
    SQLiteEdge,
    SQLiteEdgeInput,
    SQLiteEmbedding,
    FTSSearchOptions,
    FTSSearchResult,
    ResoRankCacheEntry,
    RagChunkInput,
    RagChunkRecord,
} from './client/types';

const DEFAULT_WORLD = 'default';

// ============================================================================
// TYPE CONVERTERS
// ============================================================================

function surrealNoteToSQLiteNode(note: surreal.SurrealNote): SQLiteNode {
    const id = typeof note.id === 'string'
        ? note.id
        : (note.id as { id?: { String?: string } })?.id?.String ?? '';

    return {
        id,
        type: 'NOTE',
        label: note.title,
        content: note.content,
        parent_id: note.folder_id,
        depth: 0,
        entity_kind: note.entity_kind,
        entity_subtype: note.entity_subtype,
        is_entity: note.is_entity ? 1 : 0,
        source_note_id: null,
        blueprint_id: null,
        sequence: null,
        color: null,
        is_pinned: note.is_pinned ? 1 : 0,
        favorite: note.favorite ? 1 : 0,
        created_at: new Date(note.created_at).getTime(),
        updated_at: new Date(note.updated_at).getTime(),
        attributes: null,
        extraction: null,
        temporal: null,
        narrative_metadata: null,
        scene_metadata: null,
        event_metadata: null,
        blueprint_data: null,
        inherited_kind: null,
        inherited_subtype: null,
        is_typed_root: 0,
        is_subtype_root: 0,
        owner_entity_id: null,
        fantasy_date_created: null,
    };
}

function surrealFolderToSQLiteNode(folder: surreal.SurrealFolder): SQLiteNode {
    const id = typeof folder.id === 'string'
        ? folder.id
        : (folder.id as { id?: { String?: string } })?.id?.String ?? '';

    return {
        id,
        type: 'FOLDER',
        label: folder.name,
        content: null,
        parent_id: folder.parent_id,
        depth: 0,
        entity_kind: folder.entity_kind,
        entity_subtype: folder.entity_subtype,
        is_entity: 0,
        source_note_id: null,
        blueprint_id: null,
        sequence: null,
        color: folder.color,
        is_pinned: 0,
        favorite: 0,
        created_at: new Date(folder.created_at).getTime(),
        updated_at: new Date(folder.updated_at).getTime(),
        attributes: null,
        extraction: null,
        temporal: null,
        narrative_metadata: null,
        scene_metadata: null,
        event_metadata: null,
        blueprint_data: null,
        inherited_kind: null,
        inherited_subtype: null,
        is_typed_root: folder.is_typed_root ? 1 : 0,
        is_subtype_root: 0,
        owner_entity_id: null,
        fantasy_date_created: folder.fantasy_date ? JSON.stringify(folder.fantasy_date) : null,
    };
}

// ============================================================================
// FACADE IMPLEMENTATION
// ============================================================================

export const dbFacade = {
    // ========================================================================
    // INITIALIZATION
    // ========================================================================

    /**
     * Initialize the database facade.
     * Routes to SurrealDB initialization.
     */
    async init(): Promise<void> {
        try {
            await surreal.initSurrealDb();
        } catch (error) {
            console.warn('[dbFacade] SurrealDB init failed, may already be connected:', error);
        }
    },

    // ========================================================================
    // NOTE OPERATIONS (→ SurrealDB)
    // ========================================================================

    async insertNode(input: SQLiteNodeInput): Promise<SQLiteNode> {
        if (input.type === 'NOTE') {
            const note = await surreal.createNote({
                worldId: DEFAULT_WORLD,
                title: input.label,
                folderId: input.parent_id ?? 'root',
                content: input.content ?? undefined,
                entityKind: input.entity_kind ?? undefined,
                entitySubtype: input.entity_subtype ?? undefined,
            });
            return surrealNoteToSQLiteNode(note);
        } else if (input.type === 'FOLDER') {
            const folder = await surreal.createFolder({
                worldId: DEFAULT_WORLD,
                name: input.label,
                parentId: input.parent_id ?? undefined,
                entityKind: input.entity_kind ?? undefined,
                entitySubtype: input.entity_subtype ?? undefined,
                color: input.color ?? undefined,
            });
            return surrealFolderToSQLiteNode(folder);
        }

        // Other types quarantined
        console.warn(`[dbFacade] insertNode type ${input.type} not yet routed to SurrealDB`);
        return quarantineStub<SQLiteNode>('insertNode:' + input.type, {
            id: input.id ?? 'stub-' + Date.now(),
            type: input.type,
            label: input.label,
            content: input.content ?? null,
            parent_id: input.parent_id ?? null,
            depth: input.depth ?? 0,
            entity_kind: input.entity_kind ?? null,
            entity_subtype: input.entity_subtype ?? null,
            is_entity: input.is_entity ? 1 : 0,
            source_note_id: null,
            blueprint_id: null,
            sequence: null,
            color: input.color ?? null,
            is_pinned: input.is_pinned ? 1 : 0,
            favorite: input.favorite ? 1 : 0,
            created_at: Date.now(),
            updated_at: Date.now(),
            attributes: null,
            extraction: null,
            temporal: null,
            narrative_metadata: null,
            scene_metadata: null,
            event_metadata: null,
            blueprint_data: null,
            inherited_kind: null,
            inherited_subtype: null,
            is_typed_root: 0,
            is_subtype_root: 0,
            owner_entity_id: null,
            fantasy_date_created: null,
        })();
    },

    async getNode(id: string): Promise<SQLiteNode | null> {
        try {
            // Try as note first
            const note = await surreal.getNote(DEFAULT_WORLD, id);
            return surrealNoteToSQLiteNode(note);
        } catch {
            try {
                // Try as folder
                const folder = await surreal.getFolder(DEFAULT_WORLD, id);
                return surrealFolderToSQLiteNode(folder);
            } catch {
                return null;
            }
        }
    },

    async getAllNodes(): Promise<SQLiteNode[]> {
        // Get folder tree and flatten
        const tree = await surreal.getFolderTree(DEFAULT_WORLD);
        const nodes: SQLiteNode[] = [];

        function traverse(node: surreal.FolderTreeNode) {
            nodes.push(surrealFolderToSQLiteNode(node.folder));
            for (const note of node.notes) {
                // NoteSummary doesn't have full data, would need to fetch
                nodes.push({
                    id: note.id,
                    type: 'NOTE',
                    label: note.title,
                    content: null,
                    parent_id: null,
                    depth: 0,
                    entity_kind: note.entity_kind,
                    entity_subtype: null,
                    is_entity: 0,
                    source_note_id: null,
                    blueprint_id: null,
                    sequence: null,
                    color: null,
                    is_pinned: note.is_pinned ? 1 : 0,
                    favorite: note.favorite ? 1 : 0,
                    created_at: note.updated_at ? new Date(note.updated_at).getTime() : Date.now(),
                    updated_at: note.updated_at ? new Date(note.updated_at).getTime() : Date.now(),
                    attributes: null,
                    extraction: null,
                    temporal: null,
                    narrative_metadata: null,
                    scene_metadata: null,
                    event_metadata: null,
                    blueprint_data: null,
                    inherited_kind: null,
                    inherited_subtype: null,
                    is_typed_root: 0,
                    is_subtype_root: 0,
                    owner_entity_id: null,
                    fantasy_date_created: null,
                });
            }
            for (const child of node.children) {
                traverse(child);
            }
        }

        for (const root of tree) {
            traverse(root);
        }

        return nodes;
    },

    async updateNode(id: string, updates: Partial<SQLiteNodeInput>): Promise<void> {
        // Try as note
        try {
            await surreal.updateNote(DEFAULT_WORLD, id, {
                title: updates.label,
                content: updates.content ?? undefined,
                isPinned: updates.is_pinned,
                favorite: updates.favorite,
                entityKind: updates.entity_kind ?? undefined,
                isEntity: updates.is_entity,
            });
            return;
        } catch {
            // May be a folder - folders have different update path
            if (updates.label) {
                await surreal.renameFolder(DEFAULT_WORLD, id, updates.label);
            }
        }
    },

    async deleteNode(id: string): Promise<void> {
        try {
            await surreal.deleteNote(DEFAULT_WORLD, id);
        } catch {
            try {
                await surreal.deleteFolder(DEFAULT_WORLD, id, false);
            } catch {
                console.warn('[dbFacade] deleteNode failed for:', id);
            }
        }
    },

    async getNodesByType(type: string): Promise<SQLiteNode[]> {
        if (type === 'FOLDER') {
            const roots = await surreal.getRootFolders(DEFAULT_WORLD);
            return roots.map(surrealFolderToSQLiteNode);
        }
        // For notes, get via folder tree
        return this.getAllNodes().then(nodes => nodes.filter(n => n.type === type));
    },

    async getNodesByParent(parentId: string): Promise<SQLiteNode[]> {
        // Get folder children
        const folders = await surreal.getFolderChildren(DEFAULT_WORLD, parentId);
        const notes = await surreal.getNotesByFolder(DEFAULT_WORLD, parentId);

        return [
            ...folders.map(surrealFolderToSQLiteNode),
            ...notes.map(surrealNoteToSQLiteNode),
        ];
    },

    // ========================================================================
    // EDGE OPERATIONS (→ Quarantine)
    // ========================================================================

    insertEdge: quarantineStub<SQLiteEdge>('insertEdge', {
        id: 'stub',
        source: '',
        target: '',
        type: '',
        weight: 0,
        context: null,
        bidirectional: 0,
        temporal_relation: null,
        causality: null,
        note_ids: null,
        extraction_method: null,
        created_at: Date.now(),
        properties: null,
    }),

    getEdge: quarantineStub<SQLiteEdge | null>('getEdge', null),
    getAllEdges: quarantineStub<SQLiteEdge[]>('getAllEdges', []),
    getEdgesBySource: quarantineStub<SQLiteEdge[]>('getEdgesBySource', []),
    getEdgesByTarget: quarantineStub<SQLiteEdge[]>('getEdgesByTarget', []),
    getEdgesBetween: quarantineStub<SQLiteEdge[]>('getEdgesBetween', []),
    updateEdge: quarantineStub<void>('updateEdge', undefined),
    deleteEdge: quarantineStub<void>('deleteEdge', undefined),
    batchInsertEdges: quarantineStub<void>('batchInsertEdges', undefined),

    // ========================================================================
    // EMBEDDING OPERATIONS (→ Quarantine - Rust handles this)
    // ========================================================================

    saveEmbedding: quarantineStub<void>('saveEmbedding', undefined),
    getEmbedding: quarantineStub<SQLiteEmbedding | null>('getEmbedding', null),
    getAllEmbeddings: quarantineStub<SQLiteEmbedding[]>('getAllEmbeddings', []),
    deleteEmbedding: quarantineStub<void>('deleteEmbedding', undefined),

    // ========================================================================
    // FTS OPERATIONS (→ Quarantine - Rust ResoRank handles this)
    // ========================================================================

    ftsSearch: quarantineStub<FTSSearchResult[]>('ftsSearch', []),

    // ========================================================================
    // RESORANK CACHE (→ Quarantine - Rust handles this)
    // ========================================================================

    getResoRankCache: quarantineStub<ResoRankCacheEntry[]>('getResoRankCache', []),
    setResoRankCache: quarantineStub<void>('setResoRankCache', undefined),
    clearResoRankCache: quarantineStub<void>('clearResoRankCache', undefined),

    // ========================================================================
    // RAG CHUNKS (→ Quarantine - Rust handles this)
    // ========================================================================

    saveRagChunks: quarantineStub<void>('saveRagChunks', undefined),
    getRagChunks: quarantineStub<RagChunkRecord[]>('getRagChunks', []),
    deleteRagChunksByNote: quarantineStub<void>('deleteRagChunksByNote', undefined),
    clearRagChunks: quarantineStub<void>('clearRagChunks', undefined),

    // ========================================================================
    // METADATA (→ Quarantine)
    // ========================================================================

    getMeta: quarantineStub<string | null>('getMeta', null),
    setMeta: quarantineStub<void>('setMeta', undefined),

    // ========================================================================
    // BATCH/SYNC (→ Quarantine)
    // ========================================================================

    batchSync: quarantineStub<void>('batchSync', undefined),
    transactionExecute: quarantineStub<void>('transactionExecute', undefined),
};

// Export for compatibility
export default dbFacade;
