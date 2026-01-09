/**
 * SurrealDB Bridge (DEPRECATED - This file should not be used)
 * 
 * All SurrealDB operations have been migrated to CozoDB.
 * Use: import { contentAPI } from '@/lib/tauri/content-api'
 * 
 * This file is kept only for backwards compatibility with old imports.
 */

import { contentAPI, type CozoNote, type CozoFolder, type CozoFolderTreeNode } from './content-api';

// ============================================
// TYPES (kept for backwards compat)
// ============================================

export interface SurrealFolder {
    id: string | null;
    world_id: string;
    name: string;
    parent_id: string | null;
    entity_kind: string | null;
    entity_subtype: string | null;
    color: string | null;
    is_typed_root: boolean;
    network_id: string | null;
    collapsed: boolean;
    fantasy_date?: { year: number; month: number; day: number } | null;
    created_at: string;
    updated_at: string;
}

export interface SurrealNote {
    id: string | null;
    world_id: string;
    title: string;
    content: string;
    folder_id: string | null;
    entity_kind: string | null;
    entity_subtype: string | null;
    is_entity: boolean;
    is_pinned: boolean;
    favorite: boolean;
    created_at: string;
    updated_at: string;
}

export interface NoteSummary {
    id: string;
    title: string;
    is_pinned: boolean;
    favorite: boolean;
    entity_kind: string | null;
    updated_at: string | null;
}

export interface FolderTreeNode {
    folder: SurrealFolder;
    children: FolderTreeNode[];
    notes: NoteSummary[];
    child_count: number;
    note_count: number;
}

// ============================================
// ADAPTERS
// ============================================

function cozoToSurrealFolder(c: CozoFolder): SurrealFolder {
    return {
        id: c.id,
        world_id: c.world_id,
        name: c.name,
        parent_id: c.parent_id,
        entity_kind: c.entity_kind,
        entity_subtype: c.entity_subtype,
        color: c.color,
        is_typed_root: c.is_typed_root,
        network_id: null,
        collapsed: c.collapsed,
        fantasy_date: (c.fantasy_year && c.fantasy_month && c.fantasy_day)
            ? { year: c.fantasy_year, month: c.fantasy_month, day: c.fantasy_day }
            : null,
        created_at: new Date(c.created_at * 1000).toISOString(),
        updated_at: new Date(c.updated_at * 1000).toISOString(),
    };
}

function cozoToSurrealNote(c: CozoNote): SurrealNote {
    return {
        id: c.id,
        world_id: c.world_id,
        title: c.title,
        content: c.content,
        folder_id: c.folder_id,
        entity_kind: c.entity_kind,
        entity_subtype: c.entity_subtype,
        is_entity: c.is_entity,
        is_pinned: c.is_pinned,
        favorite: c.favorite,
        created_at: new Date(c.created_at * 1000).toISOString(),
        updated_at: new Date(c.updated_at * 1000).toISOString(),
    };
}

function cozoTreeToSurrealTree(nodes: CozoFolderTreeNode[]): FolderTreeNode[] {
    return nodes.map(n => ({
        folder: cozoToSurrealFolder(n.folder),
        children: cozoTreeToSurrealTree(n.children),
        notes: n.notes.map(note => ({
            id: note.id,
            title: note.title,
            is_pinned: note.is_pinned,
            favorite: note.favorite,
            entity_kind: note.entity_kind,
            updated_at: new Date(note.updated_at * 1000).toISOString(),
        })),
        child_count: n.children.length,
        note_count: n.notes.length,
    }));
}

// ============================================
// INITIALIZATION (no-ops now - CozoDB inits elsewhere)
// ============================================

export async function initSurrealDb(): Promise<string> {
    // Phase 4: SurrealDB removed, this is a no-op shim for backwards compat
    return 'cozo:memory';
}

export async function isSurrealReady(): Promise<boolean> {
    return true; // CozoDB is always ready
}

export async function shutdownSurrealDb(): Promise<void> {
    // No-op
}

// ============================================
// FOLDER OPERATIONS
// ============================================

export interface CreateFolderParams {
    worldId: string;
    name: string;
    parentId?: string;
    entityKind?: string;
    entitySubtype?: string;
    color?: string;
}

export async function createFolder(params: CreateFolderParams): Promise<SurrealFolder> {
    const result = await contentAPI.createFolder({
        name: params.name,
        parentId: params.parentId,
        entityKind: params.entityKind,
        entitySubtype: params.entitySubtype,
        color: params.color,
    });
    return cozoToSurrealFolder(result);
}

export async function getFolder(_worldId: string, id: string): Promise<SurrealFolder | null> {
    const result = await contentAPI.getFolder(id);
    return result ? cozoToSurrealFolder(result) : null;
}

export async function renameFolder(_worldId: string, id: string, newName: string): Promise<SurrealFolder> {
    const result = await contentAPI.updateFolder(id, { name: newName });
    return cozoToSurrealFolder(result);
}

export async function moveFolder(_worldId: string, id: string, newParentId: string | null): Promise<SurrealFolder> {
    const result = await contentAPI.updateFolder(id, { parentId: newParentId || undefined });
    return cozoToSurrealFolder(result);
}

export async function deleteFolder(_worldId: string, id: string, _recursive = false): Promise<number> {
    await contentAPI.deleteFolder(id);
    return 1;
}

export async function getFolderTree(_worldId: string, _rootId?: string): Promise<FolderTreeNode[]> {
    const result = await contentAPI.getFolderTree();
    return cozoTreeToSurrealTree(result);
}

export async function getRootFolders(_worldId: string): Promise<SurrealFolder[]> {
    const result = await contentAPI.listFolders();
    return result.filter(f => !f.parent_id).map(cozoToSurrealFolder);
}

export async function getFolderChildren(_worldId: string, parentId: string): Promise<SurrealFolder[]> {
    const result = await contentAPI.listFolders();
    return result.filter(f => f.parent_id === parentId).map(cozoToSurrealFolder);
}

// ============================================
// NOTE OPERATIONS
// ============================================

export interface CreateNoteParams {
    worldId: string;
    title: string;
    folderId: string;
    content?: string;
    entityKind?: string;
    entitySubtype?: string;
}

export async function createNote(params: CreateNoteParams): Promise<SurrealNote> {
    const result = await contentAPI.createNote({
        title: params.title,
        content: params.content,
        folderId: params.folderId,
        entityKind: params.entityKind,
        entitySubtype: params.entitySubtype,
    });
    return cozoToSurrealNote(result);
}

export async function getNote(_worldId: string, id: string): Promise<SurrealNote | null> {
    const result = await contentAPI.getNote(id);
    return result ? cozoToSurrealNote(result) : null;
}

export async function renameNote(_worldId: string, id: string, newTitle: string): Promise<SurrealNote> {
    const result = await contentAPI.updateNote(id, { title: newTitle });
    return cozoToSurrealNote(result);
}

export async function updateNoteContent(_worldId: string, id: string, content: string): Promise<SurrealNote> {
    const result = await contentAPI.updateNote(id, { content });
    return cozoToSurrealNote(result);
}

export interface UpdateNoteParams {
    title?: string;
    content?: string;
    isPinned?: boolean;
    favorite?: boolean;
    entityKind?: string;
    isEntity?: boolean;
}

export async function updateNote(_worldId: string, id: string, params: UpdateNoteParams): Promise<SurrealNote> {
    const result = await contentAPI.updateNote(id, params);
    return cozoToSurrealNote(result);
}

export async function moveNote(_worldId: string, id: string, folderId: string): Promise<SurrealNote> {
    const result = await contentAPI.updateNote(id, { folderId });
    return cozoToSurrealNote(result);
}

export async function deleteNote(_worldId: string, id: string): Promise<boolean> {
    return contentAPI.deleteNote(id);
}

export async function getNotesByFolder(_worldId: string, folderId: string): Promise<SurrealNote[]> {
    const allNotes = await contentAPI.listNotes();
    return allNotes.filter(n => n.folder_id === folderId).map(cozoToSurrealNote);
}

export async function getAllNotes(_worldId: string): Promise<SurrealNote[]> {
    const result = await contentAPI.listNotes();
    return result.map(cozoToSurrealNote);
}

export async function searchNotes(_worldId: string, _query: string): Promise<SurrealNote[]> {
    console.warn('[SurrealBridge] searchNotes not implemented in CozoDB backend');
    return [];
}

// ============================================
// NAMESPACE EXPORT
// ============================================

export const surrealBridge = {
    // Init
    initSurrealDb,
    isSurrealReady,
    shutdownSurrealDb,

    // Folders
    createFolder,
    getFolder,
    renameFolder,
    moveFolder,
    deleteFolder,
    getFolderTree,
    getRootFolders,
    getFolderChildren,

    // Notes
    createNote,
    getNote,
    renameNote,
    updateNoteContent,
    updateNote,
    moveNote,
    deleteNote,
    getNotesByFolder,
    getAllNotes,
    searchNotes,
};

export default surrealBridge;
