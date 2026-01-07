/**
 * ExtractorFacade - WASM/Legacy Pipeline (Pipeline A)
 * 
 * Entity and relationship extraction using browser-based WASM.
 * Persists data to Browser CozoDB synced to lib/db SQLite.
 * 
 * This is the LEGACY pipeline:
 * - Entities → EntityRegistry (adapters) → Browser CozoDB
 * - Relationships → RelationshipRegistry (adapters) → Browser CozoDB
 * - Graph Sync → fullGraphSync() → Browser CozoDB tables
 * 
 * Kept for A/B testing comparison with Pipeline B (Tauri).
 */

// PIPELINE A IMPORTS: All from Browser CozoDB adapters
import { entityRegistry, relationshipRegistry, RelationshipSource } from '@/lib/cozo/graph/adapters';
import { TimeRegistry } from '@/lib/time';

// Re-export types for consumers
export type {
    ScanResult,
    ExtractedRelation,
    ExtractedTriple,
    ImplicitMention,
    TemporalMention,
    EntityDefinition,
    EntitySpan,
} from './bridge';

// Import the scanner bridge (WASM-compatible)
import {
    conductorBridge,
    type ScanResult,
    type EntityDefinition,
} from './bridge';

// Persistence layers (Browser CozoDB)
import { persistTemporalMentions, clearTemporalMentions } from './temporal-persistence';
import { fullGraphSync } from '@/lib/graph/graph-sync';

import { regexEntityParser } from '@/lib/utils/regex-entity-parser';
import type { DocumentConnections, EntityReference, Triple, EntityKind } from '@/lib/types/entityTypes';

// Track last scanned text for context extraction
const lastScannedText = new Map<string, string>();

class ExtractorFacadeWasm {
    private initialized = false;

    private get activeScanner() {
        return conductorBridge;
    }

    /**
     * Initialize the scanner and wire up persistence
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;

        // Initialize scanner (WASM or Tauri fallback)
        await this.activeScanner.initialize();

        // PIPELINE A: Initialize Browser CozoDB Entity Registry
        await entityRegistry.init();

        // Hydrate entities from EntityRegistry (Browser CozoDB)
        const entities = entityRegistry.getAllEntities();
        const hydrationPayload: EntityDefinition[] = entities.map(e => ({
            id: e.id,
            label: e.label,
            kind: e.kind,
            aliases: e.aliases || [],
        }));

        if (hydrationPayload.length > 0) {
            await this.activeScanner.hydrateEntities(hydrationPayload);
            console.log(`[Extractor.WASM] Initial hydration: ${hydrationPayload.length} entities`);
        } else {
            console.log('[Extractor.WASM] Initial hydration: 0 entities');
        }

        // Wire up persistence handlers (WASM Pipeline ONLY)
        this.activeScanner.onResult(async (noteId, result) => {
            if (result.stats.was_skipped) return;

            // Persist temporal mentions
            if (result.temporal && result.temporal.length > 0) {
                const fullText = lastScannedText.get(noteId) || '';
                await clearTemporalMentions(noteId);
                await persistTemporalMentions(noteId, result.temporal, fullText);
            }

            // PIPELINE A: Persist unified relations to RelationshipRegistry (Browser CozoDB)
            const unifiedRelations = (result as any).unified_relations;
            if (unifiedRelations && unifiedRelations.length > 0) {
                const inputs = unifiedRelations
                    .map((rel: any) => {
                        const head = entityRegistry.findEntity(rel.head);
                        const tail = entityRegistry.findEntity(rel.tail);
                        if (!head || !tail) return null;

                        const source = rel.source === 'CST'
                            ? RelationshipSource.NER_EXTRACTION
                            : RelationshipSource.NER_EXTRACTION;

                        return {
                            sourceEntityId: head.id,
                            targetEntityId: tail.id,
                            type: rel.relation_type,
                            provenance: [{
                                source,
                                originId: noteId,
                                confidence: rel.confidence,
                                timestamp: new Date()
                            }]
                        };
                    })
                    .filter((input: any): input is NonNullable<typeof input> => input !== null);

                if (inputs.length > 0) {
                    const persistedCount = relationshipRegistry.addBatch(inputs);
                    if (persistedCount > 0) {
                        console.log(`[Extractor.WASM] Persisted ${persistedCount}/${unifiedRelations.length} unified relations`);
                    }
                }
            }

            // PIPELINE A: Persist triples to EntityRegistry + RelationshipRegistry
            if (result.triples && result.triples.length > 0) {
                let triplesConverted = 0;
                for (const triple of result.triples) {
                    try {
                        const t = triple as any;
                        const sourceKind = (t.source_kind || 'CONCEPT') as EntityKind;
                        const targetKind = (t.target_kind || 'CONCEPT') as EntityKind;

                        // Auto-register entities to Browser CozoDB
                        const sourceEntity = await entityRegistry.registerEntity(
                            triple.source,
                            sourceKind,
                            noteId,
                            { source: 'extraction' }
                        );
                        const targetEntity = await entityRegistry.registerEntity(
                            triple.target,
                            targetKind,
                            noteId,
                            { source: 'extraction' }
                        );

                        // Create relationship in Browser CozoDB
                        relationshipRegistry.add({
                            sourceEntityId: sourceEntity.id,
                            targetEntityId: targetEntity.id,
                            type: triple.predicate,
                            provenance: [{
                                source: RelationshipSource.EXPLICIT_SYNTAX,
                                originId: noteId,
                                confidence: 1.0,
                                timestamp: new Date(),
                            }]
                        });
                        triplesConverted++;
                    } catch (err) {
                        console.warn(`[Extractor.WASM] Failed to convert triple: ${triple.source} -> ${triple.predicate} -> ${triple.target}`, err);
                    }
                }
                if (triplesConverted > 0) {
                    console.log(`[Extractor.WASM] Converted ${triplesConverted}/${result.triples.length} triples → relationships`);
                }
            }

            // Log extraction results
            const stats = result.stats as any;
            console.log(`[Extractor.WASM] Extraction complete:`, {
                implicit: stats.implicit_found,
                unified: stats.unified_found || 0,
                triples: stats.triples_found,
                temporal: stats.temporal_found,
                time_us: stats.timings?.total_us,
            });

            // PIPELINE A: Sync graph data to Browser CozoDB
            try {
                const syncResult = await fullGraphSync(result, {
                    groupId: 'vault:global',
                    noteId,
                });
                if (syncResult.entitiesSynced > 0 || syncResult.edgesSynced > 0) {
                    console.log(`[Extractor.WASM] Graph synced:`, {
                        entities: syncResult.entitiesSynced,
                        edges: syncResult.edgesSynced,
                        relations: syncResult.relationsSynced,
                        ms: syncResult.durationMs,
                    });
                }
            } catch (err) {
                console.warn('[Extractor.WASM] Graph sync failed:', err);
            }
        });

        this.initialized = true;
        console.log(`[Extractor.WASM] Initialized`);
    }

    /**
     * Hydrate with entities (call when entities change)
     */
    async hydrateEntities(entities: EntityDefinition[]): Promise<void> {
        if (!this.initialized) return;
        await this.activeScanner.hydrateEntities(entities);
        console.log(`[Extractor.WASM] Re-hydrated with ${entities.length} entities`);
    }

    /**
     * Hydrate temporal patterns from a calendar
     */
    async hydrateTemporal(calendarId: string): Promise<void> {
        if (!this.initialized) return;
        const dictionary = TimeRegistry.getCalendarDictionary(calendarId);
        console.log(`[Extractor.WASM] Temporal hydration requested for calendar: ${calendarId}`, dictionary);
    }

    /**
     * Scan a document (debounced)
     */
    scan(noteId: string, text: string): void {
        if (!this.initialized) return;
        lastScannedText.set(noteId, text);
        this.activeScanner.scan(noteId, text, []);
    }

    /**
     * Scan immediately (bypasses debounce)
     */
    async scanImmediate(noteId: string, text: string): Promise<ScanResult | null> {
        if (!this.initialized) return null;
        lastScannedText.set(noteId, text);
        return await this.activeScanner.scanImmediate(noteId, text, []);
    }

    /**
     * Register a result handler
     */
    onResult(handler: (noteId: string, result: ScanResult) => void): () => void {
        return this.activeScanner.onResult(handler);
    }

    /**
     * Check if ready
     */
    isReady(): boolean {
        return this.initialized && this.activeScanner.isReady();
    }

    /**
     * Shutdown
     */
    shutdown(): void {
        this.activeScanner.shutdown();
        this.initialized = false;
        console.log('[Extractor.WASM] Shutdown');
    }
}

// Singleton
export const extractorFacade = new ExtractorFacadeWasm();
/** @deprecated Use extractorFacade instead */
export const scannerFacade = extractorFacade;

// =============================================================================
// Legacy Utilities (WASM version)
// =============================================================================

/**
 * Parse connections from a document using WASM pipeline
 */
export async function parseNoteConnectionsFromDocument(content: any): Promise<DocumentConnections> {
    const text = extractText(content);
    const noteId = 'temp';

    const entities: EntityReference[] = [];
    const triples: Triple[] = [];
    const wikilinks: string[] = [];
    const tags: string[] = [];
    const mentions: string[] = [];

    // 1. Explicit Entities ([KIND|Label])
    const explicitEntities = regexEntityParser.parseFromText(text);
    for (const entity of explicitEntities) {
        entities.push({
            kind: entity.kind,
            label: entity.label,
            subtype: entity.subtype,
            attributes: entity.metadata
        });
    }

    // 2. Implicit Entities & Triples (Scanner)
    if (extractorFacade.isReady()) {
        const scanResult = await extractorFacade.scanImmediate(noteId, text);
        if (scanResult) {
            // Implicit Mentions
            for (const mention of scanResult.implicit) {
                const exists = entities.some(e => e.label === mention.entity_label && e.kind === mention.entity_kind);
                if (!exists) {
                    entities.push({
                        kind: mention.entity_kind as any,
                        label: mention.entity_label,
                    });
                }
            }

            // Triples
            for (const triple of scanResult.triples) {
                triples.push({
                    subject: { label: triple.source, kind: 'CONCEPT' },
                    predicate: triple.predicate,
                    object: { label: triple.target, kind: 'CONCEPT' }
                });
            }
        }
    }

    // 3. Regex for WikiLinks, Tags, Mentions
    const linkRegex = /\[\[([^\]]+)\]\]/g;
    const tagRegex = /#([\w-]+)/g;
    const mentionRegex = /@([\w-]+)/g;

    let match;
    while ((match = linkRegex.exec(text)) !== null) {
        wikilinks.push(match[1]);
    }
    while ((match = tagRegex.exec(text)) !== null) {
        tags.push(match[1]);
    }
    while ((match = mentionRegex.exec(text)) !== null) {
        mentions.push(match[1]);
    }

    return {
        tags,
        mentions,
        links: [],
        wikilinks,
        entities,
        triples,
        backlinks: []
    };
}

/**
 * Extract plain text from TipTap JSONContent
 */
function extractText(node: any): string {
    if (!node) return '';
    if (typeof node === 'string') return node;

    if (node.type === 'text' && node.text) {
        return node.text;
    }

    if (node.content && Array.isArray(node.content)) {
        return node.content.map(extractText).join('\n');
    }

    return '';
}
