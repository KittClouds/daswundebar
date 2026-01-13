
import { atom, type WritableAtom } from 'jotai';

// Internal atom to store the list of open note IDs
export const _openNoteIdsAtom = atom<string[]>([]);

// Read-only atom to get the list
export const openNoteIdsAtom: WritableAtom<string[], [string[]], void> = atom(
    (get) => get(_openNoteIdsAtom),
    (_get, set, update: string[]) => set(_openNoteIdsAtom, update)
);

// Derived atom to open a note (and select it)
// Usage: const openNote = useSetAtom(openNoteAtom); openNote(noteId);
export const openNoteAtom = atom(
    null,
    (get, set, noteId: string) => {
        const currentOpen = get(_openNoteIdsAtom);
        // Add if not present
        if (!currentOpen.includes(noteId)) {
            set(_openNoteIdsAtom, [...currentOpen, noteId]);
        }
        // Also select it
        // We need to import the selectedNoteIdAtom from './notes' but circular deps might be an issue if we put this in a separate file.
        // For now, I'll modify src/atoms/notes.ts directly instead of creating a new file to keep access to _selectedNoteIdAtom if needed,
        // or just rely on the consumer to update both. 
        // Better: let's put this logic in the hook or a coordinated atom in `src/atoms/notes.ts`.
    }
);
