/**
 * CozoDB Stubs Index
 * 
 * Re-exports stubs for deprecated browser CozoDB.
 * Import from here to get quarantined stubs.
 */

export { cozoDb } from './db';
export * from './types';
export * from './schema';
export { entityRegistry, relationshipRegistry, type RegisteredEntity } from './graph-adapters';
export { unifiedRegistry, type CozoEntity } from './unified-registry';
export { folderNetworkGraphSync } from './sync';
