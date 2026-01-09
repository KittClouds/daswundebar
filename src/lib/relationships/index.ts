/**
 * Relationship System Exports
 * 
 * Central entry point for relationship management.
 * Redirects to the new Cozo-native adapter for the unified registry.
 */

// Export types
export * from './types';
export * from './relationshipBridgeTypes';

// Export registry instance (stubbed - Browser CozoDB removed)
export { relationshipRegistry } from '@/lib/cozo-stubs/graph-adapters';

// Export bridge store (Phase 7D - unified Fact Sheet + Blueprint Hub + Networks)
export { relationshipBridgeStore } from './RelationshipBridgeStore';

// Export unified extractor types (implementation now in Rust)
// TODO: Phase 3 - Port these type definitions to lib/scanner/types.ts
export interface ExtractedRelationship {
    headEntity: string;
    tailEntity: string;
    relationType: string;
    confidence: number;
    patternMatched: string;
}

export interface EntitySpan {
    label: string;
    start: number;
    end: number;
    kind?: string;
    entityId?: string;
}

export interface RelationshipPattern {
    id: string;
    type: string;
    pattern: string;
    confidence: number;
}

// Stub functions for API compatibility (Rust handles extraction now)

export function matchVerbPatterns() { return []; }
export function refreshPatternsFromStorage() { return Promise.resolve(); }
export function getActivePatterns() { return []; }
export function getPatternCategories() { return []; }
