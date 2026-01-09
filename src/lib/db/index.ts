// ============================================================================
// NEW: Database Facade (routes to SurrealDB, stubs deprecated SQLite)
// ============================================================================

export { dbFacade } from './facade';
export {
  quarantineStub,
  getQuarantineReport,
  clearQuarantineReport,
  hasQuarantineActivity,
  logQuarantineSummary,
} from './quarantine';

// ============================================================================
// LEGACY: SQLite (deprecated, being quarantined)
// ============================================================================

/** @deprecated Use dbFacade instead */
export { dbClient } from './client/db-client';

/** @deprecated Use Tauri graph registry instead */
export { graphSQLiteSync } from './sync/GraphSQLiteSync';

export { syncState } from './sync/SyncState';
export * from './client/types';
export * from './sync/types';

export {
  initializeSQLiteAndHydrate,
} from './sync/sqliteInit';
export type { SQLiteInitResult } from './sync/sqliteInit';

// Legacy sync components (deprecated, kept for backward compatibility)
/** @deprecated */
export { DirtyTracker } from './sync/DirtyTracker';
/** @deprecated */
export { BatchWriter } from './sync/BatchWriter';
// Hydration.ts deleted - entities now come from Rust CozoDB
/** @deprecated */
export { SyncState } from './sync/SyncState';

// Weapons-Grade Sync Engine V2 (deprecated - SurrealDB replaces this)
/** @deprecated Use dbFacade instead */
export { SyncEngineV2, syncEngineV2 } from './sync/SyncEngineV2';
/** @deprecated */
export { DeltaCollector } from './sync/DeltaCollector';
/** @deprecated */
export { TransactionBuilder } from './sync/TransactionBuilder';
/** @deprecated */
export { StreamingCozoSync, streamingCozoSync } from './sync/StreamingCozoSync';
