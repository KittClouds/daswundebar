/**
 * Notes & Folders API (DEPRECATED)
 * 
 * This file is kept for backwards compatibility.
 * New code should use: import { contentAPI } from '@/lib/tauri/content-api'
 */

import { contentAPI, type CozoNote, type CozoFolder, type CozoFolderTreeNode } from './content-api';
import type { Note, Folder } from '@/types/noteTypes';

// =============================================================================
// ADAPTER - Convert CozoDB types to legacy Note/Folder types
// =============================================================================

function cozoToNote(c: CozoNote): Note {
    return {
        id: c.id,
        type: 'NOTE',
        label: c.title,
        title: c.title,
        content: c.content || '',
        parent_id: c.folder_id,
        parentId: c.folder_id || undefined,
        folderId: c.folder_id || undefined,
        entity_kind: c.entity_kind || undefined,
        entityKind: c.entity_kind || undefined,
        entity_subtype: c.entity_subtype || undefined,
        entitySubtype: c.entity_subtype || undefined,
        is_entity: c.is_entity,
        is_pinned: c.is_pinned,
        favorite: c.favorite,
        created_at: c.created_at * 1000,
        createdAt: c.created_at * 1000,
        updated_at: c.updated_at * 1000,
        updatedAt: c.updated_at * 1000,
    } as Note;
}

function cozoToFolder(c: CozoFolder): Folder {
    return {
        id: c.id,
        type: 'FOLDER',
        label: c.name,
        name: c.name,
        parent_id: c.parent_id,
        parentId: c.parent_id || undefined,
        entity_kind: c.entity_kind || undefined,
        entityKind: c.entity_kind || undefined,
        entity_subtype: c.entity_subtype || undefined,
        entitySubtype: c.entity_subtype || undefined,
        color: c.color || undefined,
        is_typed_root: c.is_typed_root,
        isTypedRoot: c.is_typed_root,
        collapsed: c.collapsed,
        fantasy_date: (c.fantasy_year && c.fantasy_month && c.fantasy_day)
            ? { year: c.fantasy_year, month: c.fantasy_month, day: c.fantasy_day }
            : undefined,
        created_at: c.created_at * 1000,
        createdAt: c.created_at * 1000,
        updated_at: c.updated_at * 1000,
        updatedAt: c.updated_at * 1000,
    } as Folder;
}

function flattenFolderTree(nodes: CozoFolderTreeNode[]): CozoFolder[] {
    const result: CozoFolder[] = [];
    function traverse(node: CozoFolderTreeNode) {
        result.push(node.folder);
        for (const child of node.children) traverse(child);
    }
    for (const node of nodes) traverse(node);
    return result;
}

// =============================================================================
// LEGACY API (wraps contentAPI)
// =============================================================================

export async function createNote(params: {
    title?: string;
    content?: string;
    folderId?: string;
    entityKind?: string;
    entitySubtype?: string;
}): Promise<Note> {
    const result = await contentAPI.createNote(params);
    return cozoToNote(result);
}

export async function getNote(id: string): Promise<Note | null> {
    const result = await contentAPI.getNote(id);
    return result ? cozoToNote(result) : null;
}

export async function updateNote(id: string, updates: Partial<Note>): Promise<Note> {
    const result = await contentAPI.updateNote(id, {
        title: updates.title || updates.label,
        content: updates.content,
        folderId: updates.folderId || updates.parent_id || undefined,
        entityKind: updates.entityKind || updates.entity_kind,
        entitySubtype: updates.entitySubtype || updates.entity_subtype,
        isPinned: updates.is_pinned,
        favorite: updates.favorite,
    });
    return cozoToNote(result);
}

export async function updateNoteContent(id: string, content: string): Promise<Note> {
    const result = await contentAPI.updateNote(id, { content });
    return cozoToNote(result);
}

export async function renameNote(id: string, title: string): Promise<Note> {
    const result = await contentAPI.updateNote(id, { title });
    return cozoToNote(result);
}

export async function moveNote(id: string, folderId: string | null): Promise<Note> {
    const result = await contentAPI.updateNote(id, { folderId: folderId || undefined });
    return cozoToNote(result);
}

export async function deleteNote(id: string): Promise<void> {
    await contentAPI.deleteNote(id);
}

export async function listNotes(): Promise<Note[]> {
    const results = await contentAPI.listNotes();
    return results.map(cozoToNote);
}

export async function getNotesByFolder(folderId: string): Promise<Note[]> {
    const allNotes = await listNotes();
    return allNotes.filter(n => n.folderId === folderId);
}

export async function createFolder(params: {
    name: string;
    parentId?: string;
    entityKind?: string;
    entitySubtype?: string;
    color?: string;
    isTypedRoot?: boolean;
}): Promise<Folder> {
    const result = await contentAPI.createFolder(params);
    return cozoToFolder(result);
}

export async function getFolder(id: string): Promise<Folder | null> {
    const result = await contentAPI.getFolder(id);
    return result ? cozoToFolder(result) : null;
}

export async function renameFolder(id: string, name: string): Promise<Folder> {
    const result = await contentAPI.updateFolder(id, { name });
    return cozoToFolder(result);
}

export async function moveFolder(id: string, parentId: string | null): Promise<Folder> {
    const result = await contentAPI.updateFolder(id, { parentId: parentId || undefined });
    return cozoToFolder(result);
}

export async function deleteFolder(id: string): Promise<void> {
    await contentAPI.deleteFolder(id);
}

export async function listFolders(): Promise<Folder[]> {
    const results = await contentAPI.listFolders();
    return results.map(cozoToFolder);
}

export async function getFolderChildren(parentId: string): Promise<Folder[]> {
    const allFolders = await listFolders();
    return allFolders.filter(f => f.parentId === parentId);
}

export async function loadAllNotesAndFolders(): Promise<{ notes: Note[]; folders: Folder[] }> {
    const { notes, folderTree } = await contentAPI.loadAllContent();
    return {
        notes: notes.map(cozoToNote),
        folders: flattenFolderTree(folderTree).map(cozoToFolder),
    };
}

// =============================================================================
// NAMESPACE EXPORT (backwards compat)
// =============================================================================

export const notesAPI = {
    createNote,
    getNote,
    updateNote,
    updateNoteContent,
    renameNote,
    moveNote,
    deleteNote,
    listNotes,
    getNotesByFolder,
    createFolder,
    getFolder,
    renameFolder,
    moveFolder,
    deleteFolder,
    listFolders,
    getFolderChildren,
    loadAllNotesAndFolders,
};
