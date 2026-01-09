/**
 * Network Editor Module
 * 
 * Main entry point for the Network Editor system.
 * 
 * This module provides:
 * - Type definitions for networks, schemas, and relationships
 * - Built-in schemas (Family, Organization, Faction, Alliance, Guild)
 * - IndexedDB storage layer
 * - Validation utilities
 * - Query API for network traversal
 * 
 * @example
 * ```ts
 * import { 
 *   FAMILY_SCHEMA,
 *   saveNetworkInstance,
 *   networkValidator,
 *   getAncestors
 * } from '@/lib/networks';
 * 
 * // Create a new family network
 * const network = {
 *   id: generateId(),
 *   name: 'Stark Family',
 *   schemaId: FAMILY_SCHEMA.id,
 *   rootFolderId: folderId,
 *   entityIds: [],
 *   namespace: 'default',
 *   createdAt: new Date(),
 *   updatedAt: new Date(),
 * };
 * 
 * await saveNetworkInstance(network);
 * ```
 */

// Core types
export * from './types';

// Built-in schemas
export * from './schemas';

// Storage layer (new - uses SurrealDB via Tauri)
export * from './storage-adapter';

// Storage layer (legacy - IndexedDB, deprecated)
/** @deprecated Use storage-adapter.ts instead */
export * as legacyStorage from './storage';

// Validation
export { NetworkValidator, networkValidator } from './validator';

// Queries
export * from './queries';

// Re-export commonly used types for convenience
export type {
    NetworkKind,
    NetworkSchema,
    NetworkInstance,
    NetworkRelationshipDef,
    NetworkRelationshipInstance,
    NetworkRelationshipDirection,
    NetworkValidationResult,
    NetworkFolderMeta,
    NetworkNoteMeta,
    NetworkLineageResult,
    NetworkQueryOptions,
    NetworkStats,
} from './types';

// Schema helpers
export {
    FAMILY_SCHEMA,
    ORG_SCHEMA,
    FACTION_SCHEMA,
    ALLIANCE_SCHEMA,
    GUILD_SCHEMA,
    BUILTIN_SCHEMAS,
    getDefaultSchemaForKind,
    getSchemaById,
    getSchemasForKind,
} from './schemas';

// Auto-membership (Phase 5)
export {
    networkAutoMembership,
    findMatchingSchemas,
    checkAndAutoEnroll,
    getRelationshipNetworkMemberships,
    getAvailableNetworksForRelationship,
    addRelationshipToNetwork,
    removeRelationshipFromNetwork,
    syncRelationshipsToNetworks,
} from './autoMembership';

export type {
    NetworkMatchResult,
    RelationshipNetworkMembership,
    AddToNetworkInput,
} from './autoMembership';
