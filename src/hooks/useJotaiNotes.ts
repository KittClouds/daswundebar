/**
 * Jotai-powered hooks that mirror NotesContext API
 * Drop-in replacements for migrating components
 */
import { useAtom, useAtomValue, useSetAtom } from 'jotai';
// ... imports
import {
    notesAtom,
    foldersAtom,
    selectedNoteIdAtom,
    selectedNoteAtom,
    favoriteNotesAtom,
    globalNotesAtom,
    optimizedFolderTreeAtom as folderTreeAtom, // Alias to keep API compatible
    isSavingAtom,
    lastSavedAtom,
    searchQueryAtom,
    updateNoteContentAtom,
    updateNoteAtom,
    createNoteAtom,
    deleteNoteAtom,
    createFolderAtom,
    updateFolderAtom,
    deleteFolderAtom,
    openNoteIdsAtom,
    openNoteAtom,
    closeNoteAtom,
} from '@/atoms';
import type { Note, Folder, FolderWithChildren } from '@/types/noteTypes';


/**
 * Hook that mirrors useNotes() from NotesContext
 * Use this to migrate components from Context to Jotai
 */
export function useJotaiNotes() {
    // Read atoms
    const notes = useAtomValue(notesAtom);
    const folders = useAtomValue(foldersAtom);
    const selectedNoteId = useAtomValue(selectedNoteIdAtom);
    const selectedNote = useAtomValue(selectedNoteAtom);
    const favoriteNotes = useAtomValue(favoriteNotesAtom);
    const globalNotes = useAtomValue(globalNotesAtom);
    const folderTree = useAtomValue(folderTreeAtom);
    const isSaving = useAtomValue(isSavingAtom);
    const lastSaved = useAtomValue(lastSavedAtom);
    const [searchQuery, setSearchQuery] = useAtom(searchQueryAtom);

    // Write atoms (setters)
    const setSelectedNoteId = useSetAtom(selectedNoteIdAtom);
    const updateContent = useSetAtom(updateNoteContentAtom);
    const updateNote = useSetAtom(updateNoteAtom);
    const createNote = useSetAtom(createNoteAtom);
    const deleteNote = useSetAtom(deleteNoteAtom);
    const createFolder = useSetAtom(createFolderAtom);
    const updateFolder = useSetAtom(updateFolderAtom);
    const deleteFolder = useSetAtom(deleteFolderAtom);

    // Open/Close tabs
    const openNoteIds = useAtomValue(openNoteIdsAtom);
    const openNoteAction = useSetAtom(openNoteAtom);
    const closeNoteAction = useSetAtom(closeNoteAtom);

    /**
     * Select a note by ID (and ensure it's open)
     */
    const selectNote = (id: string) => {
        // setSelectedNoteId(id); // Old behavior
        openNoteAction(id);       // New behavior: Open tab + Select
    };

    const closeNote = (id: string) => {
        closeNoteAction(id);
    };

    /**
     * Update note content (optimized path)
     */
    const updateNoteContent = async (id: string, content: string) => {
        await updateContent({ id, content });
    };

    /**
     * Update note fields
     */
    const handleUpdateNote = async (id: string, updates: Partial<Note>) => {
        await updateNote({ id, updates });
    };

    /**
     * Create new note
     * Note: We return a constructed note object instead of looking it up
     * because the Jotai store update is synchronous but React's useAtomValue
     * won't reflect the change until the next render cycle.
     */
    const handleCreateNote = async (
        folderId?: string,
        title?: string,
        sourceNoteId?: string
    ): Promise<Note> => {
        const noteId = await createNote({ folderId, title, sourceNoteId });
        const timestamp = Date.now();

        // Return a note object matching what createNoteAtom created
        // This avoids a race condition where notes.find() uses stale state
        return {
            id: noteId,
            type: 'NOTE',
            label: title || 'Untitled Note',
            title: title || 'Untitled Note',
            content: '',
            parent_id: folderId || null,
            parentId: folderId || null,
            folderId: folderId || null,
            source_note_id: sourceNoteId,
            is_entity: false,
            isEntity: false,
            favorite: 0,
            created_at: timestamp,
            createdAt: timestamp,
            updated_at: timestamp,
            updatedAt: timestamp,
        } as unknown as Note;
    };

    /**
     * Delete note
     */
    const handleDeleteNote = async (id: string) => {
        await deleteNote(id);
    };

    /**
     * Create folder
     * Note: We return a constructed folder object instead of looking it up
     * because the Jotai store update is synchronous but React's useAtomValue
     * won't reflect the change until the next render cycle.
     */
    const handleCreateFolder = async (
        name: string,
        parentId?: string,
        options?: {
            entityKind?: string;
            entitySubtype?: string;
            entityLabel?: string;
            isTypedRoot?: boolean;
            isSubtypeRoot?: boolean;
            color?: string;
            fantasy_date?: { year: number; month: number; day: number };
        }
    ): Promise<Folder> => {
        const folderId = await createFolder({ name, parentId, ...options });
        const timestamp = Date.now();

        // Return a folder object matching what createFolderAtom created
        // This avoids a race condition where folders.find() uses stale state
        return {
            id: folderId,
            type: 'FOLDER',
            label: name,
            name,
            parent_id: parentId || null,
            parentId: parentId || null,
            content: null,
            entity_kind: options?.entityKind,
            entityKind: options?.entityKind,
            entity_subtype: options?.entitySubtype,
            entitySubtype: options?.entitySubtype,
            is_typed_root: options?.isTypedRoot,
            isTypedRoot: options?.isTypedRoot,
            is_subtype_root: options?.isSubtypeRoot,
            isSubtypeRoot: options?.isSubtypeRoot,
            color: options?.color,
            fantasy_date: options?.fantasy_date,
            is_entity: false,
            isEntity: false,
            created_at: timestamp,
            createdAt: timestamp,
            updated_at: timestamp,
            updatedAt: timestamp,
        } as unknown as Folder;
    };

    /**
     * Update folder
     */
    const handleUpdateFolder = async (id: string, updates: Partial<Folder>) => {
        await updateFolder({ id, updates });
    };

    /**
     * Delete folder
     */
    const handleDeleteFolder = async (id: string) => {
        await deleteFolder(id);
    };

    /**
     * Get entity note by ID
     */
    const getEntityNote = (id: string): Note | undefined => {
        return notes.find(n => n.id === id);
    };

    // Return same interface as NotesContext
    return {
        state: {
            notes,
            folders,
            isSaving,
            lastSaved,
            searchQuery,
            searchQuery,
            selectedNoteId,
            openNoteIds,
        },
        selectedNote,
        favoriteNotes,
        globalNotes,
        folderTree,
        selectNote,
        closeNote,
        setSearchQuery,
        createNote: handleCreateNote,
        updateNote: handleUpdateNote,
        updateNoteContent,
        deleteNote: handleDeleteNote,
        getEntityNote,
        createFolder: handleCreateFolder,
        updateFolder: handleUpdateFolder,
        deleteFolder: handleDeleteFolder,
    };
}

/**
 * Granular hooks for specific use cases (more performant)
 */

// Only subscribe to selected note (doesn't re-render on other note changes)
export function useSelectedNote() {
    return useAtomValue(selectedNoteAtom);
}

// Only subscribe to selected note ID
export function useSelectedNoteId() {
    return useAtom(selectedNoteIdAtom);
}

// Only subscribe to folder tree
export function useFolderTree() {
    return useAtomValue(folderTreeAtom);
}

// Only subscribe to favorite notes
export function useFavoriteNotes() {
    return useAtomValue(favoriteNotesAtom);
}

// Get note update function without subscribing to state
export function useNoteUpdater() {
    const updateContent = useSetAtom(updateNoteContentAtom);
    const updateNote = useSetAtom(updateNoteAtom);

    return {
        updateContent: async (id: string, content: string) => {
            await updateContent({ id, content });
        },
        updateNote: async (id: string, updates: Partial<Note>) => {
            await updateNote({ id, updates });
        },
    };
}
