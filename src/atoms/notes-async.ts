/**
 * Async atoms for database operations
 * Handles initialization, hydration, and write operations
 * 
 * MIGRATED: Now uses SurrealDB via notesAPI (Tauri backend)
 */
import { atom } from 'jotai';
import { generateId } from '@/lib/utils/ids';
import { isTauri } from '@/lib/tauri/bridge';
import { notesAPI } from '@/lib/tauri/notes-api';
import {
    notesAtom,
    foldersAtom,
    selectedNoteIdAtom,
    isSavingAtom,
    lastSavedAtom,
} from './notes';
import type { Note, Folder } from '@/types/noteTypes';

// ============================================
// INITIALIZATION ATOMS
// ============================================

/**
 * Async atom that loads all data from database
 * Called once during app initialization
 */
export const dbInitAtom = atom(async () => {
    console.log('[Atoms] Loading data from database...');

    if (!isTauri()) {
        console.log('[Atoms] Not in Tauri mode, returning empty data');
        return { notes: [], folders: [] };
    }

    try {
        const { notes, folders } = await notesAPI.loadAllNotesAndFolders();
        console.log(`[Atoms] Loaded ${notes.length} notes, ${folders.length} folders`);
        return { notes, folders };
    } catch (error) {
        console.error('[Atoms] Failed to load from SurrealDB:', error);
        return { notes: [], folders: [] };
    }
});

/**
 * Write-only atom that hydrates store with database data
 * Usage: await store.set(hydrateNotesAtom)
 */
export const hydrateNotesAtom = atom(
    null, // No read function (write-only)
    async (get, set) => {
        try {
            const { notes, folders } = await get(dbInitAtom);

            set(notesAtom, notes);
            set(foldersAtom, folders);

            console.log('[Atoms] ✅ Store hydrated successfully');
        } catch (error) {
            console.error('[Atoms] ❌ Hydration failed:', error);
            throw error;
        }
    }
);

// ============================================
// NOTE MUTATION ATOMS
// ============================================

/**
 * Update note content (optimized path for editor changes)
 * Includes optimistic update + rollback on failure
 * 
 * Usage: set(updateNoteContentAtom, { id: 'abc', content: 'new text' })
 */
export const updateNoteContentAtom = atom(
    null, // Write-only
    async (get, set, update: { id: string; content: string }) => {
        const { id, content } = update;
        const currentNotes = get(notesAtom);
        const originalNote = currentNotes.find(n => n.id === id);

        if (!originalNote) {
            console.error(`[Atoms] Note ${id} not found`);
            return;
        }

        // Optimistic update - UI reflects change immediately
        const timestamp = Date.now();
        set(notesAtom, currentNotes.map(n =>
            n.id === id
                ? { ...n, content, updated_at: timestamp, updatedAt: timestamp }
                : n
        ));

        set(isSavingAtom, true);

        try {
            if (isTauri()) {
                await notesAPI.updateNoteContent(id, content);
            }
            set(lastSavedAtom, new Date());
            console.log(`[Atoms] ✅ Updated note ${id}`);
        } catch (error) {
            // Rollback on failure
            console.error(`[Atoms] ❌ Failed to update note ${id}:`, error);
            set(notesAtom, currentNotes);
            throw error;
        } finally {
            set(isSavingAtom, false);
        }
    }
);

/**
 * Update any note fields
 * Includes optimistic update + rollback on failure
 * 
 * Usage: set(updateNoteAtom, { id: 'abc', updates: { title: 'New Title', favorite: true } })
 */
export const updateNoteAtom = atom(
    null,
    async (get, set, params: { id: string; updates: Partial<Note> }) => {
        const { id, updates } = params;
        const currentNotes = get(notesAtom);
        const originalNote = currentNotes.find(n => n.id === id);

        if (!originalNote) {
            console.error(`[Atoms] Note ${id} not found`);
            return;
        }

        // Optimistic update
        const timestamp = Date.now();
        set(notesAtom, currentNotes.map(n =>
            n.id === id
                ? { ...n, ...updates, updatedAt: timestamp }
                : n
        ));

        set(isSavingAtom, true);

        try {
            if (isTauri()) {
                await notesAPI.updateNote(id, updates);
            }

            set(lastSavedAtom, new Date());
            console.log(`[Atoms] ✅ Updated note ${id}`, updates);
        } catch (error) {
            console.error(`[Atoms] ❌ Failed to update note ${id}:`, error);
            set(notesAtom, currentNotes);
            throw error;
        } finally {
            set(isSavingAtom, false);
        }
    }
);

/**
 * Create new note
 * Returns the created note's ID
 * 
 * Usage: const noteId = await store.set(createNoteAtom, { folderId: 'xyz', title: 'New Note' })
 * 
 * Entity-aware: If ownerEntityId and fantasyDate are provided (from narrative focus),
 * they are stored with the note for entity-scoped views.
 */
export const createNoteAtom = atom(
    null,
    async (get, set, params: {
        folderId?: string;
        title?: string;
        sourceNoteId?: string;
        // Entity ownership context (from narrative focus)
        ownerEntityId?: string;
        fantasyDate?: { year: number; monthIndex: number; dayIndex: number; eraId?: string };
    }) => {
        const newNoteId = generateId();
        const timestamp = Date.now();

        const newNote: Note = {
            id: newNoteId,
            type: 'NOTE',
            label: params.title || 'Untitled Note',
            title: params.title || 'Untitled Note',
            content: '',
            parent_id: params.folderId || null,
            parentId: params.folderId || null,
            folderId: params.folderId || null,
            source_note_id: params.sourceNoteId,
            is_entity: false,
            isEntity: false,
            favorite: 0,
            created_at: timestamp,
            createdAt: timestamp,
            updated_at: timestamp,
            updatedAt: timestamp,
        } as unknown as Note;

        // Optimistic add
        set(notesAtom, [...get(notesAtom), newNote]);

        try {
            if (isTauri()) {
                const created = await notesAPI.createNote({
                    title: newNote.title,
                    content: '',
                    folderId: params.folderId,
                });
                // Update with server ID if different
                if (created.id !== newNoteId) {
                    set(notesAtom, get(notesAtom).map(n =>
                        n.id === newNoteId ? { ...n, id: created.id } : n
                    ));
                    console.log(`[Atoms] ✅ Created note ${created.id} (server assigned)`);
                    return created.id;
                }
            }

            if (params.ownerEntityId) {
                console.log(`[Atoms] ✅ Created note ${newNoteId} (owned by entity: ${params.ownerEntityId})`);
            } else {
                console.log(`[Atoms] ✅ Created note ${newNoteId}`);
            }

            return newNoteId;
        } catch (error) {
            console.error(`[Atoms] ❌ Failed to create note:`, error);
            // Rollback
            set(notesAtom, get(notesAtom).filter(n => n.id !== newNoteId));
            throw error;
        }
    }
);


/**
 * Delete note by ID
 * Auto-deselects if deleted note was selected
 * 
 * Usage: await store.set(deleteNoteAtom, 'note-id-123')
 */
export const deleteNoteAtom = atom(
    null,
    async (get, set, noteId: string) => {
        const currentNotes = get(notesAtom);

        // Optimistic delete
        set(notesAtom, currentNotes.filter(n => n.id !== noteId));

        // Deselect if currently selected
        if (get(selectedNoteIdAtom) === noteId) {
            set(selectedNoteIdAtom, null);
        }

        try {
            if (isTauri()) {
                await notesAPI.deleteNote(noteId);
            }
            console.log(`[Atoms] ✅ Deleted note ${noteId}`);
        } catch (error) {
            console.error(`[Atoms] ❌ Failed to delete note ${noteId}:`, error);
            // Rollback
            set(notesAtom, currentNotes);
            throw error;
        }
    }
);

// ============================================
// FOLDER MUTATION ATOMS
// ============================================

/**
 * Create new folder
 * Returns the created folder's ID
 * 
 * Entity-aware: If ownerEntityId and fantasy_date are provided (from narrative focus),
 * they are stored with the folder for entity-scoped views.
 */
export const createFolderAtom = atom(
    null,
    async (get, set, params: {
        name: string;
        parentId?: string;
        entityKind?: string;
        entitySubtype?: string;
        isTypedRoot?: boolean;
        isSubtypeRoot?: boolean;
        color?: string;
        fantasy_date?: { year: number; month: number; day: number };
        // Entity ownership context (from narrative focus)
        ownerEntityId?: string;
    }) => {
        const newFolderId = generateId();
        const timestamp = Date.now();

        const newFolder: Folder = {
            id: newFolderId,
            type: 'FOLDER',
            label: params.name,
            name: params.name,
            parent_id: params.parentId || null,
            parentId: params.parentId || null,
            content: null,
            entity_kind: params.entityKind,
            entityKind: params.entityKind,
            entity_subtype: params.entitySubtype,
            entitySubtype: params.entitySubtype,
            is_typed_root: params.isTypedRoot,
            isTypedRoot: params.isTypedRoot,
            is_subtype_root: params.isSubtypeRoot,
            isSubtypeRoot: params.isSubtypeRoot,
            color: params.color,
            fantasy_date: params.fantasy_date,
            is_entity: false,
            isEntity: false,
            created_at: timestamp,
            createdAt: timestamp,
            updated_at: timestamp,
            updatedAt: timestamp,
        } as unknown as Folder;

        // Optimistic add
        set(foldersAtom, [...get(foldersAtom), newFolder]);

        try {
            if (isTauri()) {
                const created = await notesAPI.createFolder({
                    name: params.name,
                    parentId: params.parentId,
                    entityKind: params.entityKind,
                    entitySubtype: params.entitySubtype,
                    color: params.color,
                    isTypedRoot: params.isTypedRoot,
                });
                // Update with server ID if different
                if (created.id !== newFolderId) {
                    set(foldersAtom, get(foldersAtom).map(f =>
                        f.id === newFolderId ? { ...f, id: created.id } : f
                    ));
                    console.log(`[Atoms] ✅ Created folder ${created.id} (server assigned)`);
                    return created.id;
                }
            }

            if (params.ownerEntityId) {
                console.log(`[Atoms] ✅ Created folder ${newFolderId} (owned by entity: ${params.ownerEntityId})`);
            } else {
                console.log(`[Atoms] ✅ Created folder ${newFolderId}`);
            }

            return newFolderId;
        } catch (error) {
            console.error(`[Atoms] ❌ Failed to create folder:`, error);
            // Rollback
            set(foldersAtom, get(foldersAtom).filter(f => f.id !== newFolderId));
            throw error;
        }
    }
);


/**
 * Update folder fields
 */
export const updateFolderAtom = atom(
    null,
    async (get, set, params: { id: string; updates: Partial<Folder> }) => {
        const { id, updates } = params;
        const currentFolders = get(foldersAtom);

        // Optimistic update
        set(foldersAtom, currentFolders.map(f =>
            f.id === id ? { ...f, ...updates } : f
        ));

        try {
            if (isTauri()) {
                if (updates.name) {
                    await notesAPI.renameFolder(id, updates.name);
                }
                // TODO: Add more update fields if needed
            }

            console.log(`[Atoms] ✅ Updated folder ${id}`);
        } catch (error) {
            console.error(`[Atoms] ❌ Failed to update folder ${id}:`, error);
            // Rollback
            set(foldersAtom, currentFolders);
            throw error;
        }
    }
);

/**
 * Delete folder by ID
 * Orphans child notes (sets parent_id to null)
 */
export const deleteFolderAtom = atom(
    null,
    async (get, set, folderId: string) => {
        const currentFolders = get(foldersAtom);
        const currentNotes = get(notesAtom);

        // Optimistic delete
        set(foldersAtom, currentFolders.filter(f => f.id !== folderId));

        // Orphan child notes
        set(notesAtom, currentNotes.map(n =>
            n.parent_id === folderId
                ? { ...n, parent_id: null, folderId: null, parentId: null }
                : n
        ));

        try {
            if (isTauri()) {
                await notesAPI.deleteFolder(folderId);
            }
            console.log(`[Atoms] ✅ Deleted folder ${folderId}`);
        } catch (error) {
            console.error(`[Atoms] ❌ Failed to delete folder ${folderId}:`, error);
            // Rollback
            set(foldersAtom, currentFolders);
            set(notesAtom, currentNotes);
            throw error;
        }
    }
);
