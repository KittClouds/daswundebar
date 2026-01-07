/**
 * UnifiedEntityLifecycle - LEGACY FILE
 * 
 * This module was intended to synchronize entity operations across layers.
 * It is NOT actively used - entity registration now flows through:
 * 1. smartGraphRegistry (TypeScript) -> Rust GraphRegistry (CozoDB)
 * 
 * Keeping this file for reference and potential reactivation.
 * All methods are stubbed to log deprecation warnings.
 */

import type { Note, Folder } from '@/types/noteTypes';
import type { EntityKind } from '@/lib/types/entityTypes';

// Stub for missing autoSaveEntityRegistry
const autoSaveEntityRegistry = () => {
    console.debug('[LEGACY] autoSaveEntityRegistry stub - use smartGraphRegistry instead');
};

// Stub for missing folderRelationshipCreator
const folderRelationshipCreator = {
    onSubfolderCreated: () => { },
    onNoteCreated: () => { },
    onNoteMoved: () => { },
    onFolderMoved: () => { },
    onFolderDeleted: () => { },
    getStats: () => ({ totalRelationships: 0, byType: {} }),
};

/**
 * @deprecated Use smartGraphRegistry directly instead
 */
export class UnifiedEntityLifecycle {
    /** @deprecated Use smartGraphRegistry.registerEntity() */
    static onNoteCreated(note: Note): void {
        console.warn('[LEGACY] UnifiedEntityLifecycle.onNoteCreated - use smartGraphRegistry instead');
    }

    /** @deprecated Use smartGraphRegistry directly */
    static onNoteTitleChanged(note: Note, oldTitle: string): void {
        console.warn('[LEGACY] UnifiedEntityLifecycle.onNoteTitleChanged - use smartGraphRegistry instead');
    }

    /** @deprecated Use smartGraphRegistry.deleteEntity() */
    static onNoteDeleted(noteId: string, wasEntity: boolean, entityLabel?: string): void {
        console.warn('[LEGACY] UnifiedEntityLifecycle.onNoteDeleted - use smartGraphRegistry instead');
    }

    /** @deprecated Use smartGraphRegistry.registerEntity() */
    static onFolderCreated(folder: Folder): void {
        console.warn('[LEGACY] UnifiedEntityLifecycle.onFolderCreated - use smartGraphRegistry instead');
    }

    /** @deprecated */
    static onFolderCreatedWithParent(folder: Folder, parentFolder: Folder | null): void {
        console.warn('[LEGACY] UnifiedEntityLifecycle.onFolderCreatedWithParent');
    }

    /** @deprecated */
    static onNoteCreatedWithFolder(note: Note, parentFolder: Folder | null): void {
        console.warn('[LEGACY] UnifiedEntityLifecycle.onNoteCreatedWithFolder');
    }

    /** @deprecated */
    static onNoteMoved(note: Note, oldFolder: Folder | null, newFolder: Folder | null): void {
        console.warn('[LEGACY] UnifiedEntityLifecycle.onNoteMoved');
    }

    /** @deprecated */
    static onFolderMoved(folder: Folder, oldParent: Folder | null, newParent: Folder | null): void {
        console.warn('[LEGACY] UnifiedEntityLifecycle.onFolderMoved');
    }

    /** @deprecated */
    static onFolderDeleted(folderId: string, wasTyped: boolean, entityKind?: EntityKind): void {
        console.warn('[LEGACY] UnifiedEntityLifecycle.onFolderDeleted');
    }

    /** @deprecated */
    static getFolderRelationshipStats(): { totalRelationships: number; byType: Record<string, number> } {
        return folderRelationshipCreator.getStats();
    }

    /** @deprecated */
    static getRelationshipStats() {
        return { total: 0, byType: {}, bySource: {} };
    }
}

