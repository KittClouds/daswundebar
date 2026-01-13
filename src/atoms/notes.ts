import { atom, type WritableAtom } from 'jotai';
import type { Note, Folder, FolderWithChildren } from '@/types/noteTypes';

// ============================================
// BASE ATOMS (Internal State)
// ============================================

const _notesAtom = atom<Note[]>([]);
const _foldersAtom = atom<Folder[]>([]);
const _selectedNoteIdAtom = atom<string | null>(null);
const _openNoteIdsAtom = atom<string[]>([]);
const _isSavingAtom = atom<boolean>(false);
const _lastSavedAtom = atom<Date | null>(null);

// ============================================
// EXPORTED ATOMS (Strictly Writable)
// ============================================

export const notesAtom: WritableAtom<Note[], [Note[]], void> = atom(
    (get) => get(_notesAtom),
    (_get, set, update: Note[]) => set(_notesAtom as any, update)
);

export const foldersAtom: WritableAtom<Folder[], [Folder[]], void> = atom(
    (get) => get(_foldersAtom),
    (_get, set, update: Folder[]) => set(_foldersAtom as any, update)
);

export const selectedNoteIdAtom: WritableAtom<string | null, [string | null], void> = atom(
    (get) => get(_selectedNoteIdAtom),
    (_get, set, update: string | null) => set(_selectedNoteIdAtom as any, update)
);

export const openNoteIdsAtom: WritableAtom<string[], [string[]], void> = atom(
    (get) => get(_openNoteIdsAtom),
    (_get, set, update: string[]) => set(_openNoteIdsAtom as any, update)
);

// Action: Open a note (adds to list and selects it)
export const openNoteAtom = atom(
    null,
    (get, set, noteId: string) => {
        const currentOpen = get(_openNoteIdsAtom);
        if (!currentOpen.includes(noteId)) {
            set(_openNoteIdsAtom, [...currentOpen, noteId]);
        }
        set(_selectedNoteIdAtom, noteId);
    }
);

// Action: Close a note (removes from list, selects neighbor if active)
export const closeNoteAtom = atom(
    null,
    (get, set, noteIdToClose: string) => {
        const currentOpen = get(_openNoteIdsAtom);
        const isActive = get(_selectedNoteIdAtom) === noteIdToClose;

        const newOpen = currentOpen.filter(id => id !== noteIdToClose);
        set(_openNoteIdsAtom, newOpen);

        if (isActive) {
            if (newOpen.length > 0) {
                // Determine new active note (MRU or neighbor)
                // For simplicity here, we'll pick the neighbor to the left, or the first one.
                // User asked for "most-recently-used" but we aren't tracking history yet.
                // I will use "neighbor to the left" as a stable fallback common in browsers,
                // or if we closed the first one, pick the new first one.
                // To do true MRU, we need a separate history stack.
                // For now: pick the note at the same index or previous.
                const closingIndex = currentOpen.indexOf(noteIdToClose);
                const nextIndex = Math.max(0, closingIndex - 1);
                // If the array is now empty (handled by guard above), this logic won't run.
                // Wait, newOpen has length > 0.
                // If we closed index 0, nextIndex is 0. New array shifted left. Correct.
                // If we closed index 5, nextIndex is 4. Correct.
                const nextId = newOpen[Math.min(nextIndex, newOpen.length - 1)];
                set(_selectedNoteIdAtom, nextId);
            } else {
                set(_selectedNoteIdAtom, null);
            }
        }
    }
);


export const isSavingAtom: WritableAtom<boolean, [boolean], void> = atom(
    (get) => get(_isSavingAtom),
    (_get, set, update: boolean) => set(_isSavingAtom as any, update)
);

export const lastSavedAtom: WritableAtom<Date | null, [Date | null], void> = atom(
    (get) => get(_lastSavedAtom),
    (_get, set, update: Date | null) => set(_lastSavedAtom as any, update)
);

// ============================================
// DERIVED ATOMS (Computed State)
// ============================================

/**
 * Map of note ID -> Note for O(1) lookups
 */
export const notesMapAtom = atom((get) => {
    const notes = get(notesAtom);
    return new Map(notes.map(n => [n.id, n]));
});

/**
 * Currently selected note object
 */
export const selectedNoteAtom = atom((get) => {
    const selectedId = get(selectedNoteIdAtom);
    if (!selectedId) return null;

    const notesMap = get(notesMapAtom);
    return notesMap.get(selectedId) ?? null;
});

/**
 * All favorite notes (favorite = 1 in SQLite)
 */
export const favoriteNotesAtom = atom((get) => {
    const notes = get(notesAtom);
    return notes.filter(n => Number(n.favorite) === 1);
});

/**
 * Notes without a parent folder (root-level notes)
 */
export const globalNotesAtom = atom((get) => {
    const notes = get(notesAtom);
    return notes.filter(n => !n.parent_id);
});

/**
 * Hierarchical folder tree with nested children and notes
 */
export const folderTreeAtom = atom((get) => {
    const folders = get(foldersAtom);
    const notes = get(notesAtom);

    const buildTree = (parentId: string | null): FolderWithChildren[] => {
        return folders
            .filter(f => f.parent_id === parentId)
            .map(folder => ({
                ...folder,
                children: buildTree(folder.id),
                notes: notes.filter(n => n.parent_id === folder.id)
            }));
    };

    return buildTree(null);
});

// ============================================
// TYPE EXPORTS
// ============================================

