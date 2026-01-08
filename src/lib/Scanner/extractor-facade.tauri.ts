/**
 * ExtractorFacade - Tauri/Rust Pipeline (Pipeline B)
 * 
 * Entity and relationship extraction using native Tauri IPC.
 * Persists ALL data to Rust GraphRegistry (CozoDB/SQLite).
 * 
 * This is the NEW unified pipeline:
 * - Entities → Rust GraphRegistry (nodes)
 * - Relationships → Rust GraphRegistry (edges)
 * - Temporal → SQLite (via Rust)
 * 
 * NO writes to Browser CozoDB (UnifiedRegistry).
 */

import { smartGraphRegistry, tauriScanner, isTauri } from '@/lib/tauri';
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

// Import types
import {
    type ScanResult,
    type EntityDefinition,
} from './bridge';

// Persistence layers (Tauri-only)
import { persistTemporalMentions, clearTemporalMentions } from './temporal-persistence';

import { regexEntityParser } from '@/lib/utils/regex-entity-parser';
import type { DocumentConnections, EntityReference, Triple } from '@/lib/types/entityTypes';

// Track last scanned text for context extraction
const lastScannedText = new Map<string, string>();

class ExtractorFacadeTauri {
    private initialized = false;

    /**
     * Initialize the Tauri scanner and wire up persistence
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;

        // Verify Tauri environment
        if (!isTauri()) {
            throw new Error('[ExtractorFacade.Tauri] Cannot initialize: not running in Tauri');
        }

        // Wait for orchestrator (handles: backend connection, entity loading, scanner hydration)
        const { tauriOrchestrator } = await import('@/lib/tauri');
        await tauriOrchestrator.waitForReady();
        console.log('[Extractor.Tauri] Orchestrator ready');

        // Wire up persistence handlers (Tauri Pipeline ONLY)
        tauriScanner.onResult(async (noteId, result) => {
            // Type guard: result is ConductorScanResult from conductor_scan
            const conductorResult = result as any;

            if (conductorResult.stats?.was_skipped) return;

            // Persist temporal mentions to SQLite
            if (conductorResult.temporal && conductorResult.temporal.length > 0) {
                const fullText = lastScannedText.get(noteId) || '';
                await clearTemporalMentions(noteId);
                await persistTemporalMentions(noteId, conductorResult.temporal, fullText);
            }

            // UNIFIED PERSISTENCE: Collect all entities and relations, then deduplicate
            // This matches WASM's fullGraphSync() approach
            const entityMap = new Map<string, { label: string; kind: string }>();
            const allRelations: Array<{
                head_label: string;
                tail_label: string;
                relation_type: string;
                confidence: number;
                source: 'explicit' | 'cst' | 'graph';
            }> = [];

            // From triples (explicit syntax - highest priority)
            for (const triple of conductorResult.triples || []) {
                const t = triple as any;
                const sourceKey = triple.source.toLowerCase();
                const targetKey = triple.target.toLowerCase();

                // Deduplicate by lowercase key
                if (!entityMap.has(sourceKey)) {
                    entityMap.set(sourceKey, {
                        label: triple.source,
                        kind: t.source_kind || 'CONCEPT'
                    });
                }
                if (!entityMap.has(targetKey)) {
                    entityMap.set(targetKey, {
                        label: triple.target,
                        kind: t.target_kind || 'CONCEPT'
                    });
                }

                allRelations.push({
                    head_label: triple.source,
                    tail_label: triple.target,
                    relation_type: triple.predicate,
                    confidence: 1.0,
                    source: 'explicit',
                });
            }

            // From unified relations (CST + Graph inference)
            const unifiedRelations = conductorResult.unified_relations || [];
            for (const rel of unifiedRelations) {
                const headKey = rel.head.toLowerCase();
                const tailKey = rel.tail.toLowerCase();

                if (!entityMap.has(headKey)) {
                    entityMap.set(headKey, { label: rel.head, kind: 'CONCEPT' });
                }
                if (!entityMap.has(tailKey)) {
                    entityMap.set(tailKey, { label: rel.tail, kind: 'CONCEPT' });
                }

                allRelations.push({
                    head_label: rel.head,
                    tail_label: rel.tail,
                    relation_type: rel.relation_type,
                    confidence: rel.confidence || 0.8,
                    source: rel.source === 'CST' ? 'cst' : 'graph',
                });
            }

            // Single ingest call with deduplicated data
            if (entityMap.size > 0 || allRelations.length > 0) {
                try {
                    const mentions = Array.from(entityMap.values()).map(e => ({
                        entity_label: e.label,
                        entity_kind: e.kind,
                    }));

                    const ingestResult = await smartGraphRegistry.ingestScanResult(
                        noteId,
                        mentions,
                        allRelations
                    );

                    console.log(`[Extractor.Tauri] 📊 Graph synced:`, {
                        entities: ingestResult.entities_created + ingestResult.entities_updated,
                        edges: ingestResult.edges_created + ingestResult.edges_updated,
                    });
                } catch (err) {
                    console.warn('[Extractor.Tauri] Graph sync failed:', err);
                }
            }

            // Log extraction results (using ConductorStats fields)
            const stats = conductorResult.stats;
            console.log(`[Extractor.Tauri] ✅ Extraction complete:`, {
                implicit: stats?.implicit_found ?? 0,
                unified: stats?.unified_found ?? 0,
                triples: stats?.triples_found ?? 0,
                temporal: stats?.temporal_found ?? 0,
                structured: stats?.structured_found ?? 0,
                time_us: stats?.timings?.total_us ?? 0,
                wasIncremental: stats?.was_incremental ?? false,
                wasSkipped: stats?.was_skipped ?? false,
            });
        });

        this.initialized = true;
        console.log(`[Extractor.Tauri] Initialized`);
    }

    /**
     * Hydrate with entities (call when entities change)
     */
    async hydrateEntities(entities: EntityDefinition[]): Promise<void> {
        if (!this.initialized) return;
        await tauriScanner.hydrateEntities(entities);
        console.log(`[Extractor.Tauri] Re-hydrated with ${entities.length} entities`);
    }

    /**
     * Hydrate temporal patterns from a calendar
     */
    async hydrateTemporal(calendarId: string): Promise<void> {
        if (!this.initialized) return;
        const dictionary = TimeRegistry.getCalendarDictionary(calendarId);
        console.log(`[Extractor.Tauri] Temporal hydration requested for calendar: ${calendarId}`, dictionary);
    }

    /**
     * Scan a document (debounced)
     */
    scan(noteId: string, text: string): void {
        if (!this.initialized) return;
        lastScannedText.set(noteId, text);
        tauriScanner.scan(noteId, text, []);
    }

    /**
     * Scan immediately (bypasses debounce)
     */
    async scanImmediate(noteId: string, text: string): Promise<ScanResult | null> {
        if (!this.initialized) return null;
        lastScannedText.set(noteId, text);
        return await tauriScanner.scanImmediate(noteId, text, []);
    }

    /**
     * Register a result handler
     */
    onResult(handler: (noteId: string, result: ScanResult) => void): () => void {
        return tauriScanner.onResult(handler);
    }

    /**
     * Check if ready
     */
    isReady(): boolean {
        return this.initialized && tauriScanner.isReady();
    }

    /**
     * Shutdown
     */
    shutdown(): void {
        tauriScanner.shutdown();
        this.initialized = false;
        console.log('[Extractor.Tauri] Shutdown');
    }
}

// Singleton
export const extractorFacade = new ExtractorFacadeTauri();
/** @deprecated Use extractorFacade instead */
export const scannerFacade = extractorFacade;

// =============================================================================
// Legacy Utilities (Tauri version)
// =============================================================================

/**
 * Parse connections from a document using Tauri pipeline
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

    // 2. Implicit Entities & Triples (Tauri Scanner)
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
