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



