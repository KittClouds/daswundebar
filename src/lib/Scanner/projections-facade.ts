/**
 * ProjectionsFacade - TypeScript bridge to Rust Projector
 * 
 * This facade provides 1:1 feature parity with the existing projections system
 * while delegating all heavy lifting to Rust WASM.
 * 
 * KEEP THIS FILE AS REFERENCE - maintains full functionality even if Rust breaks
 * 
 * @module scanner/projections-facade
 */

import { getScannerMode } from '@/lib/tauri';
import type { EntityKind } from '@/lib/entities/entityTypes';

// =============================================================================
// TYPES - Mirror TypeScript refs/projections.ts exactly
// =============================================================================

/** Timeline event */
export interface TimelineEvent {
    timestamp: number | null;
    expression: string;
    entities: Array<{ kind: string; label: string }>;
    description: string;
    source_note_id: string;
    position: number;
    is_relative: boolean;
}

/** Character sheet */
export interface CharacterSheet {
    id: string;
    name: string;
    kind: string;
    aliases: string[];
    relationships: CharacterRelationship[];
    appearances: NoteAppearance[];
    traits: string[];
    timeline: TimelineEvent[];
    stats: CharacterStats;
}

export interface CharacterRelationship {
    target_id: string;
    target_label: string;
    target_kind: string;
    predicate: string;
    source_note_id: string;
    bidirectional: boolean;
}

export interface NoteAppearance {
    note_id: string;
    mention_count: number;
    contexts: string[];
}

export interface CharacterStats {
    total_mentions: number;
    unique_notes: number;
    relationship_count: number;
    first_seen: number;
    last_seen: number;
}

/** Relationship graph */
export interface RelationshipGraph {
    nodes: Array<{ id: string; label: string; kind: string }>;
    edges: Array<{ source: string; target: string; predicate: string }>;
}

/** Link graph */
export interface LinkGraph {
    nodes: Array<{ id: string; exists: boolean }>;
    edges: Array<{ source: string; target: string }>;
}

/** Ref input for projections */
export interface ProjectionRef {
    id: string;
    kind: string;
    target: string;
    source_note_id: string;
    predicate?: string;
    positions: RefPosition[];
    payload?: ProjectionPayload;
    attributes?: Record<string, unknown>;
    created_at: number;
    last_seen_at: number;
}

export interface RefPosition {
    note_id: string;
    offset: number;
    length: number;
    context_before?: string;
    context_after?: string;
}

export interface ProjectionPayload {
    entity_kind?: string;
    subject_kind?: string;
    subject_label?: string;
    subject_id?: string;
    object_kind?: string;
    object_label?: string;
    object_id?: string;
    aliases?: string[];
    expression?: string;
    parsed_date?: string;
    temporal_type?: string;
    exists?: boolean;
}

// =============================================================================
// FACADE CLASS
// =============================================================================

/**
 * ProjectionsFacade - TypeScript bridge to Rust Projector
 * 
 * Usage:
 * ```typescript
 * const facade = new ProjectionsFacade();
 * await facade.initialize();
 * 
 * const timeline = facade.buildTimeline(refs);
 * const graph = facade.buildRelationshipGraph(refs);
 * const sheet = facade.buildCharacterSheet(characterRef, allRefs);
 * ```
 */
export class ProjectionsFacade {
    // Projector removed - using TypeScript fallbacks until Tauri implements
    private initialized = false;
    private initPromise: Promise<void> | null = null;

    /**
     * Initialize the WASM projector
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;
        if (this.initPromise) return this.initPromise;

        this.initPromise = this._doInit();
        return this.initPromise;
    }

    private async _doInit(): Promise<void> {
        try {
            // Use Tauri/fallback mode - projections run in TypeScript fallback for now
            const mode = getScannerMode();

            if (mode === 'tauri') {
                console.log('[ProjectionsFacade] Running with Tauri backend (using TS fallbacks)');
            } else {
                console.log('[ProjectionsFacade] Running in fallback mode');
            }

            // Projections use TypeScript fallback until Rust backend implements them
            this.initialized = true;
            console.log('[ProjectionsFacade] Initialized successfully');
        } catch (error) {
            console.error('[ProjectionsFacade] Failed to initialize:', error);
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
     * Build timeline from refs
     */
    buildTimeline(refs: ProjectionRef[]): TimelineEvent[] {
        // Always use TypeScript fallback for now
        return this.buildTimelineTS(refs);
    }

    /**
     * Build relationship graph
     */
    buildRelationshipGraph(refs: ProjectionRef[]): RelationshipGraph {
        // Always use TypeScript fallback for now
        return this.buildRelationshipGraphTS(refs);
    }

    /**
     * Build link graph from wikilinks
     */
    buildLinkGraph(refs: ProjectionRef[]): LinkGraph {
        // Always use TypeScript fallback for now
        return this.buildLinkGraphTS(refs);
    }

    /**
     * Build character sheet
     */
    buildCharacterSheet(characterRef: ProjectionRef, allRefs: ProjectionRef[]): CharacterSheet {
        // Always use TypeScript fallback for now
        return this.buildCharacterSheetTS(characterRef, allRefs);
    }

    // =========================================================================
    // TypeScript Fallbacks (maintain full functionality)
    // =========================================================================

    private buildTimelineTS(refs: ProjectionRef[]): TimelineEvent[] {
        const events: TimelineEvent[] = [];
        const temporalRefs = refs.filter(r => r.kind === 'temporal');

        for (const ref of temporalRefs) {
            const payload = ref.payload;
            const position = ref.positions[0];

            // Find nearby entities
            const nearbyEntities = this.findNearbyEntities(
                refs,
                ref.source_note_id,
                position?.offset || 0,
                200
            );

            events.push({
                timestamp: payload?.parsed_date ? new Date(payload.parsed_date).getTime() : null,
                expression: payload?.expression || ref.target,
                entities: nearbyEntities.map(e => ({
                    kind: e.payload?.entity_kind || '',
                    label: e.target,
                })),
                description: ref.target,
                source_note_id: ref.source_note_id,
                position: position?.offset || 0,
                is_relative: payload?.temporal_type === 'relative',
            });
        }

        // Sort by timestamp
        return events.sort((a, b) => {
            if (a.timestamp !== null && b.timestamp !== null) {
                return a.timestamp - b.timestamp;
            }
            if (a.timestamp === null && b.timestamp === null) {
                return a.position - b.position;
            }
            return a.timestamp === null ? 1 : -1;
        });
    }

    private buildRelationshipGraphTS(refs: ProjectionRef[]): RelationshipGraph {
        const nodes = new Map<string, { id: string; label: string; kind: string }>();
        const edges: Array<{ source: string; target: string; predicate: string }> = [];

        // Add entity nodes
        for (const ref of refs.filter(r => r.kind === 'entity')) {
            const kind = ref.payload?.entity_kind || '';
            const key = `${kind}:${ref.target.toLowerCase()}`;
            if (!nodes.has(key)) {
                nodes.set(key, { id: key, label: ref.target, kind });
            }
        }

        // Add edges from triples
        for (const ref of refs.filter(r => r.kind === 'triple')) {
            const payload = ref.payload;
            if (!payload) continue;

            const sourceKey = `${payload.subject_kind}:${payload.subject_label?.toLowerCase()}`;
            const targetKey = `${payload.object_kind}:${payload.object_label?.toLowerCase()}`;

            if (!nodes.has(sourceKey)) {
                nodes.set(sourceKey, {
                    id: sourceKey,
                    label: payload.subject_label || '',
                    kind: payload.subject_kind || '',
                });
            }
            if (!nodes.has(targetKey)) {
                nodes.set(targetKey, {
                    id: targetKey,
                    label: payload.object_label || '',
                    kind: payload.object_kind || '',
                });
            }

            edges.push({
                source: sourceKey,
                target: targetKey,
                predicate: ref.predicate || '',
            });
        }

        return {
            nodes: Array.from(nodes.values()),
            edges,
        };
    }

    private buildLinkGraphTS(refs: ProjectionRef[]): LinkGraph {
        const nodes = new Map<string, { id: string; exists: boolean }>();
        const edges: Array<{ source: string; target: string }> = [];

        for (const ref of refs.filter(r => r.kind === 'wikilink')) {
            const target = ref.target.toLowerCase();
            const exists = ref.payload?.exists ?? false;

            if (!nodes.has(target)) {
                nodes.set(target, { id: target, exists });
            }

            edges.push({ source: ref.source_note_id, target });

            if (!nodes.has(ref.source_note_id)) {
                nodes.set(ref.source_note_id, { id: ref.source_note_id, exists: true });
            }
        }

        return {
            nodes: Array.from(nodes.values()),
            edges,
        };
    }

    private buildCharacterSheetTS(characterRef: ProjectionRef, allRefs: ProjectionRef[]): CharacterSheet {
        const payload = characterRef.payload;

        // Extract relationships
        const relationships = this.extractRelationships(characterRef.target, allRefs);

        // Extract appearances
        const appearances = this.extractAppearances(
            characterRef.target,
            payload?.aliases || [],
            allRefs
        );

        // Extract traits
        const traits = this.extractTraits(characterRef, allRefs);

        // Build timeline
        const timeline = this.buildEntityTimeline(characterRef.target, allRefs);

        // Calculate stats
        const uniqueNotes = new Set(characterRef.positions.map(p => p.note_id));

        return {
            id: characterRef.id,
            name: characterRef.target,
            kind: payload?.entity_kind || '',
            aliases: payload?.aliases || [],
            relationships,
            appearances,
            traits,
            timeline,
            stats: {
                total_mentions: characterRef.positions.length,
                unique_notes: uniqueNotes.size,
                relationship_count: relationships.length,
                first_seen: characterRef.created_at,
                last_seen: characterRef.last_seen_at,
            },
        };
    }

    private findNearbyEntities(
        refs: ProjectionRef[],
        noteId: string,
        position: number,
        radius: number
    ): ProjectionRef[] {
        return refs
            .filter(r => r.kind === 'entity')
            .filter(r => {
                const pos = r.positions.find(p => p.note_id === noteId);
                if (!pos) return false;
                return Math.abs(pos.offset - position) <= radius;
            });
    }

    private extractRelationships(label: string, refs: ProjectionRef[]): CharacterRelationship[] {
        const relationships: CharacterRelationship[] = [];
        const labelLower = label.toLowerCase();

        for (const ref of refs.filter(r => r.kind === 'triple')) {
            const payload = ref.payload;
            if (!payload) continue;

            // Character is subject
            if (payload.subject_label?.toLowerCase() === labelLower) {
                relationships.push({
                    target_id: payload.object_id || '',
                    target_label: payload.object_label || '',
                    target_kind: payload.object_kind || '',
                    predicate: ref.predicate || '',
                    source_note_id: ref.source_note_id,
                    bidirectional: false,
                });
            }

            // Character is object
            if (payload.object_label?.toLowerCase() === labelLower) {
                relationships.push({
                    target_id: payload.subject_id || '',
                    target_label: payload.subject_label || '',
                    target_kind: payload.subject_kind || '',
                    predicate: `←${ref.predicate || ''}`,
                    source_note_id: ref.source_note_id,
                    bidirectional: false,
                });
            }
        }

        return relationships;
    }

    private extractAppearances(label: string, aliases: string[], refs: ProjectionRef[]): NoteAppearance[] {
        const appearances = new Map<string, NoteAppearance>();
        const patterns = [label.toLowerCase(), ...aliases.map(a => a.toLowerCase())];

        for (const ref of refs.filter(r => r.kind === 'entity')) {
            if (!patterns.includes(ref.target.toLowerCase())) continue;

            for (const pos of ref.positions) {
                const noteId = pos.note_id;
                const entry = appearances.get(noteId) || {
                    note_id: noteId,
                    mention_count: 0,
                    contexts: [],
                };

                entry.mention_count++;

                if (pos.context_before || pos.context_after) {
                    entry.contexts.push(
                        `${pos.context_before || ''}${ref.target}${pos.context_after || ''}`
                    );
                }

                appearances.set(noteId, entry);
            }
        }

        return Array.from(appearances.values());
    }

    private extractTraits(characterRef: ProjectionRef, allRefs: ProjectionRef[]): string[] {
        const traits: string[] = [];
        const labelLower = characterRef.target.toLowerCase();

        // Look for HAS_TRAIT relationships
        for (const ref of allRefs.filter(r => r.kind === 'triple')) {
            if (
                ref.payload?.subject_label?.toLowerCase() === labelLower &&
                ref.predicate === 'HAS_TRAIT'
            ) {
                if (ref.payload.object_label) {
                    traits.push(ref.payload.object_label);
                }
            }
        }

        return [...new Set(traits)];
    }

    private buildEntityTimeline(label: string, refs: ProjectionRef[]): TimelineEvent[] {
        const timeline = this.buildTimelineTS(refs);
        const labelLower = label.toLowerCase();

        return timeline.filter(event =>
            event.entities.some(e => e.label.toLowerCase() === labelLower)
        );
    }
}

// =============================================================================
// SINGLETON
// =============================================================================

export const projectionsFacade = new ProjectionsFacade();
