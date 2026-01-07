/**
 * ConstraintsFacade - TypeScript bridge to Rust ConstraintEngine
 * 
 * This facade provides 1:1 feature parity with the existing constraints system
 * while delegating all heavy lifting to Rust WASM.
 * 
 * KEEP THIS FILE AS REFERENCE - maintains full functionality even if Rust breaks
 * 
 * @module scanner/constraints-facade
 */

import { getScannerMode } from '@/lib/tauri';

// =============================================================================
// TYPES - Mirror TypeScript refs/constraints.ts exactly
// =============================================================================

/** Validation result (matches Rust ConstraintResult) */
export interface ConstraintResult {
    valid: boolean;
    errors: string[];
    warnings: string[];
}

/** Ref position */
export interface RefPosition {
    note_id: string;
    offset: number;
    length: number;
    context_before?: string;
    context_after?: string;
}

/** Ref payload */
export interface RefPayload {
    entity_kind?: string;
    subject_kind?: string;
    subject_label?: string;
    object_kind?: string;
    object_label?: string;
    aliases?: string[];
}

/** Ref structure for validation */
export interface RefInput {
    id: string;
    kind: string;
    target: string;
    source_note_id: string;
    predicate?: string;
    scope_type: string;
    scope_path?: string;
    positions: RefPosition[];
    payload?: RefPayload;
    attributes?: Record<string, unknown>;
    created_at: number;
    last_seen_at: number;
}

// =============================================================================
// PREDICATE RULES (TypeScript fallback - matches Rust)
// =============================================================================

const PREDICATE_RULES: Record<string, string[]> = {
    CHARACTER: ['KNOWS', 'LOVES', 'HATES', 'RELATED_TO', 'WORKS_WITH', 'MENTORS', 'RIVALS'],
    PERSON: ['KNOWS', 'LOVES', 'HATES', 'RELATED_TO', 'WORKS_WITH', 'MENTORS', 'RIVALS'],
    LOCATION: ['CONTAINS', 'NEAR', 'CONNECTED_TO', 'PART_OF'],
    ORGANIZATION: ['OWNS', 'EMPLOYS', 'ALLIED_WITH', 'RIVALS', 'PART_OF'],
    EVENT: ['INVOLVES', 'CAUSES', 'PRECEDES', 'FOLLOWS'],
    ITEM: ['BELONGS_TO', 'CREATED_BY', 'USED_BY', 'PART_OF'],
    CONCEPT: ['RELATED_TO', 'IMPLIES', 'CONTRADICTS', 'PART_OF'],
};

// =============================================================================
// FACADE CLASS
// =============================================================================

/**
 * ConstraintsFacade - TypeScript bridge to Rust ConstraintEngine
 * 
 * Usage:
 * ```typescript
 * const facade = new ConstraintsFacade();
 * await facade.initialize();
 * 
 * const result = facade.validate(someRef);
 * const unique = facade.enforceUniqueness(refs);
 * ```
 */
export class ConstraintsFacade {
    // Engine removed - using TypeScript fallbacks until Tauri implements
    private initialized = false;
    private initPromise: Promise<void> | null = null;

    /**
     * Initialize the constraint engine
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;
        if (this.initPromise) return this.initPromise;

        this.initPromise = this._doInit();
        return this.initPromise;
    }

    private async _doInit(): Promise<void> {
        try {
            const mode = getScannerMode();

            if (mode === 'tauri') {
                console.log('[ConstraintsFacade] Running with Tauri backend (using TS fallbacks)');
            } else {
                console.log('[ConstraintsFacade] Running in fallback mode');
            }

            this.initialized = true;
            console.log('[ConstraintsFacade] Initialized successfully');
        } catch (error) {
            console.error('[ConstraintsFacade] Failed to initialize:', error);
            throw error;
        }
    }

    /**
     * Check if the facade is ready
     */
    isReady(): boolean {
        return this.initialized;
    }

    /**
     * Validate a single ref
     */
    validate(ref: RefInput): ConstraintResult {
        // Always use TypeScript fallback for now
        return this.validateTS(ref);
    }

    /**
     * Validate predicate for entity kind
     */
    validatePredicate(entityKind: string, predicate: string): boolean {
        return this.validatePredicateTS(entityKind, predicate);
    }

    /**
     * Get allowed predicates for entity kind
     */
    getAllowedPredicates(entityKind: string): string[] {
        return PREDICATE_RULES[entityKind] || [];
    }

    /**
     * Enforce uniqueness across refs
     */
    enforceUniqueness(refs: RefInput[]): RefInput[] {
        return this.enforceUniquenessTS(refs);
    }

    /**
     * Filter refs by scope
     */
    filterByScope(
        refs: RefInput[],
        scope: { type: 'note' | 'folder' | 'vault'; path?: string }
    ): RefInput[] {
        return refs.filter(ref => {
            switch (scope.type) {
                case 'note':
                    return ref.scope_type === 'note' && ref.scope_path === scope.path;
                case 'folder':
                    return ref.scope_path?.startsWith(scope.path || '') ?? true;
                case 'vault':
                    return true;
                default:
                    return true;
            }
        });
    }

    // =========================================================================
    // TypeScript Fallbacks (maintain full functionality)
    // =========================================================================

    private validateTS(ref: RefInput): ConstraintResult {
        const errors: string[] = [];
        const warnings: string[] = [];

        if (!ref.id) errors.push('Ref must have an id');
        if (!ref.kind) errors.push('Ref must have a kind');
        if (!ref.target) errors.push('Ref must have a target');
        if (!ref.source_note_id) errors.push('Ref must have a source_note_id');

        if (ref.kind === 'entity') {
            if (ref.target.length < 2) {
                warnings.push(`Entity label "${ref.target}" is very short`);
            }
            if (/[<>{}[\]|]/.test(ref.target)) {
                warnings.push(`Entity label "${ref.target}" contains suspicious characters`);
            }
        }

        return {
            valid: errors.length === 0,
            errors,
            warnings,
        };
    }

    private validatePredicateTS(entityKind: string, predicate: string): boolean {
        const allowed = PREDICATE_RULES[entityKind];
        return !allowed || allowed.includes(predicate);
    }

    private enforceUniquenessTS(refs: RefInput[]): RefInput[] {
        const seen = new Map<string, RefInput>();

        for (const ref of refs) {
            const key = this.getUniqueKey(ref);
            const existing = seen.get(key);

            if (existing) {
                existing.positions.push(...ref.positions);
                existing.last_seen_at = Math.max(existing.last_seen_at, ref.last_seen_at);
            } else {
                seen.set(key, { ...ref, positions: [...ref.positions] });
            }
        }

        return Array.from(seen.values());
    }

    private getUniqueKey(ref: RefInput): string {
        const scopePath = ref.scope_path || '';

        switch (ref.kind) {
            case 'entity': {
                const entityKind = ref.payload?.entity_kind || 'UNKNOWN';
                return `entity:${entityKind}:${ref.target.toLowerCase()}:${scopePath}`;
            }
            case 'wikilink':
                return `wikilink:${ref.target.toLowerCase()}:${scopePath}`;
            case 'backlink':
                return `backlink:${ref.target.toLowerCase()}:${scopePath}`;
            case 'tag':
                return `tag:${ref.target.toLowerCase()}`;
            case 'mention':
                return `mention:${ref.target.toLowerCase()}`;
            case 'triple':
                return `triple:${ref.target}:${ref.predicate}`;
            case 'temporal':
                return `temporal:${ref.target}:${ref.source_note_id}:${ref.positions[0]?.offset}`;
            default:
                return `${ref.kind}:${ref.target}:${ref.source_note_id}`;
        }
    }
}

// =============================================================================
// SINGLETON
// =============================================================================

export const constraintsFacade = new ConstraintsFacade();
