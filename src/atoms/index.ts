/**
 * Unified atom exports
 * Import atoms from this file in components/hooks
 */

// Base atoms
export {
    notesAtom,
    foldersAtom,
    selectedNoteIdAtom,
    isSavingAtom,
    lastSavedAtom,
    openNoteIdsAtom,
    openNoteAtom,
    closeNoteAtom,
} from './notes';

// Derived atoms
export {
    notesMapAtom,
    selectedNoteAtom,
    favoriteNotesAtom,
    globalNotesAtom,
    folderTreeAtom,
} from './notes';

// Async atoms
export {
    dbInitAtom,
    hydrateNotesAtom,
    updateNoteContentAtom,
    updateNoteAtom,
    createNoteAtom,
    deleteNoteAtom,
    createFolderAtom,
    updateFolderAtom,
    deleteFolderAtom,
} from './notes-async';

// Search atoms
export {
    searchQueryAtom,
    searchModeAtom,
    selectedModelAtom,
    hybridWeightsAtom,
    searchResultsAtom,
    searchMetadataAtom,
    isSearchingAtom,
    embeddingHealthAtom,
    syncStatusAtom,
    syncProgressAtom,
    syncEmbeddingsAtom,
    cancelSyncAtom,
    initSearchServicesAtom,
} from './search';

// Autosave atoms
export {
    noteContentAtom,
    debouncedNoteContentAtom,
    hasUnsavedChangesAtom,
    autosaveAtom,
    manualSaveAtom,
    initNoteContentAtom,
} from './notes-autosave';

// Optimized atoms
export {
    optimizedFolderTreeAtom,
    foldersByIdAtom,
    notesByFolderAtom,
    folderPathAtom,
} from './notes-optimized';

// Types
export type { FolderWithChildren } from '@/types/noteTypes';

// Calendar atoms
export {
    calendarAtom,
    calendarEventsAtom,
    calendarPeriodsAtom,
    calendarViewStateAtom,
    isCalendarHydratedAtom,
    isCalendarLoadingAtom,
    hydrateCalendarAtom,
    saveCalendarAtom,
    createEventAtom,
    updateEventAtom,
    deleteEventAtom,
    createPeriodAtom,
    updatePeriodAtom,
    deletePeriodAtom,
    updateViewStateAtom,
    setSetupModeAtom,
    // Entity-scoped atoms
    eventsForFocusedEntityAtom,
    periodsForFocusedEntityAtom,
} from './calendar';


// Narrative Focus atoms
export {
    narrativeFocusAtom,
    focusModeAtom,
    focusedEntityIdAtom,
    focusedEntityKindAtom,
    focusedEntityLabelAtom,
    hasEntityFocusAtom,
    setNarrativeFocusAtom,
    clearNarrativeFocusAtom,
    setFocusModeAtom,
} from './narrative-focus';

export type { NarrativeFocus, FocusMode } from './narrative-focus';

// Entity Attributes atoms (First-Class Fact Sheet)
export {
    entityAttributesFamily,
    entityAttributesRecordFamily,
    metaCardsFamily,
    setAttributeAtom,
    setMultipleAttributesAtom,
    deleteAttributeAtom,
    createMetaCardAtom,
    updateMetaCardAtom,
    deleteMetaCardAtom,
    invalidateEntityCacheAtom,
    clearAllCachesAtom,
    isLoadingAttributesAtom,
    getAttributeAtom,
} from './entity-attributes';

export type {
    FieldType,
    EntityAttribute,
    MetaCard,
    MetaCardField,
    FieldSchema,
    ValidationRule,
} from './entity-attributes';
