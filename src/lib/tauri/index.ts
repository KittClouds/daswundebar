/**
 * Tauri Module Index
 * 
 * Provides the public API for the Tauri bridge.
 * Import from '@/lib/tauri' to get all Tauri-related functionality.
 * 
 * @module lib/tauri
 */

// Orchestrator - single entry point for Tauri initialization
export {
    TauriOrchestrator,
    tauriOrchestrator,
    TauriState,
    type TauriOrchestratorStatus,
} from './orchestrator';

// Bridge exports (scanner commands)
export * from './bridge';

// Graph registry - basic adapter
export {
    TauriGraphRegistry,
    tauriGraphRegistry,
    type NodeResponse,
    type EdgeResponse,
    type IngestResponse,
    type HydrateResponse,
    type GraphStats,
    type MentionInput,
    type RelationInput,
} from './graph-registry';

// Smart graph registry - cached adapter with dirty tracking
export {
    SmartGraphRegistry,
    smartGraphRegistry,
    smartGraphRegistry as tauriEntityRegistry,
    type RegisteredEntity,
    type EntityRegistrationResult,
    type Edge,
} from './smart-graph-registry';

// Re-export EntityDefinition from bridge (canonical source)
export type { EntityDefinition } from './bridge';

// Re-export singletons
export { tauriScanner, conductorBridge } from './bridge';

// ResoRank - native BM25F search
export {
    tauriResorank,
    resorankSearch,
    resorankIndex,
    resorankClear,
    resorankStats,
    type ResoRankSearchResult,
    type ResoRankStats,
} from './resorank';

// Blueprint Hub - native blueprint storage (V2 Phase 1)
export {
    tauriBlueprint,
    // Meta commands
    blueprintInit,
    blueprintCreate,
    blueprintGet,
    blueprintList,
    blueprintUpdate,
    blueprintDelete,
    // Version commands
    blueprintVersionCreate,
    blueprintVersionList,
    blueprintVersionDelete,
    // EntityType commands
    blueprintEntityTypeCreate,
    blueprintEntityTypeList,
    blueprintEntityTypeDelete,
    // Field commands
    blueprintFieldCreate,
    blueprintFieldList,
    blueprintFieldDelete,
    // RelationshipType commands
    blueprintRelationshipTypeCreate,
    blueprintRelationshipTypeList,
    blueprintRelationshipTypeDelete,
    // Types
    type BlueprintMeta,
    type CreateBlueprintInput,
    type BlueprintVersion,
    type CreateVersionInput,
    type EntityTypeDef,
    type CreateEntityTypeInput,
    type FieldDef,
    type CreateFieldInput,
    type RelationshipTypeDef,
    type CreateRelationshipTypeInput,
} from './blueprints';

// Time Registry - change history tracking (V2 Phase 2)
export {
    tauriTimeRegistry,
    timeRegistryInit,
    timeRegistryRecordEntityChange,
    timeRegistryGetEntityHistory,
    timeRegistryRecordEdgeChange,
    timeRegistryGetEdgeHistory,
    TauriTimeRegistryFacade,
    type HistoryEntry,
    type EdgeHistoryEntry,
    type RecordChangeInput,
    type RecordEdgeChangeInput,
} from './time-registry';

// SurrealDB Bridge (DEPRECATED - routes to CozoDB now)
export {
    surrealBridge,
    surrealBridge as surreal,
    initSurrealDb,
    isSurrealReady,
    shutdownSurrealDb,
    // Folder commands
    createFolder as surrealCreateFolder,
    getFolder as surrealGetFolder,
    renameFolder as surrealRenameFolder,
    moveFolder as surrealMoveFolder,
    deleteFolder as surrealDeleteFolder,
    getFolderTree as surrealGetFolderTree,
    getRootFolders as surrealGetRootFolders,
    getFolderChildren as surrealGetFolderChildren,
    // Note commands
    createNote as surrealCreateNote,
    getNote as surrealGetNote,
    renameNote as surrealRenameNote,
    updateNoteContent as surrealUpdateNoteContent,
    updateNote as surrealUpdateNote,
    moveNote as surrealMoveNote,
    deleteNote as surrealDeleteNote,
    getNotesByFolder as surrealGetNotesByFolder,
    searchNotes as surrealSearchNotes,
    // Types
    type SurrealFolder,
    type SurrealNote,
    type NoteSummary as SurrealNoteSummary,
    type FolderTreeNode as SurrealFolderTreeNode,
    type CreateFolderParams,
    type CreateNoteParams,
    type UpdateNoteParams,
} from './surreal-bridge';

// Content API - unified CozoDB content layer (replaces all above)
export { contentAPI, ContentAPI } from './content-api';
export type {
    CozoNote,
    CozoFolder,
    CozoFolderTreeNode,
    CozoNetwork,
    CozoEntity,
    CozoRelationship,
    CozoCalEvent,
    CozoPeriod,
    CozoFieldBinding,
} from './content-api';

// Network Bridge - Phase 2 network/entity/relationship persistence
export {
    networkBridge,
    // Network commands
    createNetwork,
    getNetwork,
    getNetworkByFolder,
    updateNetwork,
    deleteNetwork,
    listNetworks,
    addNetworkMember,
    removeNetworkMember,
    getNetworkMembers,
    // Entity commands
    createEntity,
    getEntity,
    getEntityByNote,
    getEntityByLabel,
    updateEntity,
    deleteEntity,
    listEntitiesByKind,
    listEntities,
    searchEntities,
    linkEntityToNote,
    linkEntityToFolder,
    // Relationship commands
    createRelationship,
    getRelationship,
    deleteRelationship,
    getEntityRelationships,
    getNetworkRelationships,
    getRelationshipsByCode,
    getNetworkRelationshipSummaries,
    deleteRelationshipsBetween,
    // Types
    type SurrealNetwork,
    type SurrealEntity,
    type SurrealRelationship,
    type NetworkMemberSummary,
    type RelationshipSummary,
} from './network-bridge';

// Calendar Bridge (DEPRECATED - routes to CozoDB now)
export {
    calendarBridge,
    // Types
    type SurrealCalEvent,
    type SurrealPeriod,
    type SurrealOccursOn,
    type FantasyDate as TauriFantasyDate,
} from './calendar-bridge';

// Notes API - centralized notes/folders operations (Phase 4)
export { notesAPI } from './notes-api';

// Calendar API - centralized events/periods operations (Phase 4)
export { calendarAPI } from './calendar-api';

// Binding Bridge - field bindings (Phase 4)
export { bindingBridge } from './binding-bridge';

