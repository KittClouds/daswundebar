/**
 * ExtractorFacade - Pipeline Router
 * 
 * Automatically selects between two extraction pipelines:
 * 
 * - **Pipeline B (Tauri)**: Rust GraphRegistry → native CozoDB/SQLite
 *   File: ./extractor-facade.tauri.ts
 *   
 * - **Pipeline A (WASM)**: Browser CozoDB → lib/db SQLite (via sync)
 *   File: ./extractor-facade.wasm.ts
 * 
 * Pipeline selection is controlled by ACTIVE_PIPELINE in pipeline-config.ts
 */

import { getResolvedPipeline, logPipelineStatus, PIPELINE_VERBOSE_LOGGING } from './pipeline-config';

// Re-export types (same for both pipelines)
export type {
    ScanResult,
    ExtractedRelation,
    ExtractedTriple,
    ImplicitMention,
    TemporalMention,
    EntityDefinition,
    EntitySpan,
} from './bridge';

// Dynamic import based on pipeline config
let _extractorFacade: any = null;
let _parseNoteConnectionsFromDocument: any = null;

/**
 * Get the appropriate ExtractorFacade based on pipeline config
 */
async function getExtractor() {
    if (_extractorFacade) return _extractorFacade;

    const pipeline = getResolvedPipeline();

    if (PIPELINE_VERBOSE_LOGGING) {
        logPipelineStatus();
    }

    if (pipeline === 'tauri') {
        console.log('[ExtractorFacade] Using Tauri pipeline (Pipeline B)');
        const module = await import('./extractor-facade.tauri');
        _extractorFacade = module.extractorFacade;
        _parseNoteConnectionsFromDocument = module.parseNoteConnectionsFromDocument;
    } else {
        console.log('[ExtractorFacade] Using WASM pipeline (Pipeline A)');
        const module = await import('./extractor-facade.wasm');
        _extractorFacade = module.extractorFacade;
        _parseNoteConnectionsFromDocument = module.parseNoteConnectionsFromDocument;
    }

    return _extractorFacade;
}

// =============================================================================
// Proxy ExtractorFacade (for synchronous imports)
// =============================================================================

/**
 * ExtractorFacade proxy - delegates to the correct pipeline
 */
class ExtractorFacadeProxy {
    private initialized = false;
    private facade: any = null;

    async initialize(): Promise<void> {
        if (this.initialized) return;
        this.facade = await getExtractor();
        await this.facade.initialize();
        this.initialized = true;
    }

    async hydrateEntities(entities: any[]): Promise<void> {
        if (!this.facade) await this.initialize();
        return this.facade.hydrateEntities(entities);
    }

    async hydrateTemporal(calendarId: string): Promise<void> {
        if (!this.facade) await this.initialize();
        return this.facade.hydrateTemporal(calendarId);
    }

    scan(noteId: string, text: string): void {
        if (!this.facade) {
            console.warn('[ExtractorFacade] Not initialized, scan deferred');
            return;
        }
        this.facade.scan(noteId, text);
    }

    async scanImmediate(noteId: string, text: string): Promise<any> {
        if (!this.facade) await this.initialize();
        return this.facade.scanImmediate(noteId, text);
    }

    onResult(handler: (noteId: string, result: any) => void): () => void {
        if (!this.facade) {
            console.warn('[ExtractorFacade] Not initialized, result handler deferred');
            return () => { };
        }
        return this.facade.onResult(handler);
    }

    isReady(): boolean {
        return this.initialized && this.facade?.isReady?.() || false;
    }

    shutdown(): void {
        if (this.facade) {
            this.facade.shutdown();
        }
        this.initialized = false;
    }
}

// Singleton
export const extractorFacade = new ExtractorFacadeProxy();

/** @deprecated Use extractorFacade instead */
export const scannerFacade = extractorFacade;

// =============================================================================
// Legacy Utilities (Routed)
// =============================================================================

import { regexEntityParser } from '@/lib/utils/regex-entity-parser';
import type { DocumentConnections, EntityReference, Triple } from '@/lib/types/entityTypes';

/**
 * Parse connections from a document
 * Routes to appropriate pipeline's parseNoteConnectionsFromDocument
 */
export async function parseNoteConnectionsFromDocument(content: any): Promise<DocumentConnections> {
    // Ensure module is loaded
    if (!_parseNoteConnectionsFromDocument) {
        await getExtractor();
    }

    // If still not available (module loading issue), fall back to basic parsing
    if (!_parseNoteConnectionsFromDocument) {
        return parseNoteConnectionsBasic(content);
    }

    return _parseNoteConnectionsFromDocument(content);
}

/**
 * Basic fallback parser (no scanner needed)
 */
function parseNoteConnectionsBasic(content: any): DocumentConnections {
    const text = extractText(content);

    const entities: EntityReference[] = [];
    const explicitEntities = regexEntityParser.parseFromText(text);
    for (const entity of explicitEntities) {
        entities.push({
            kind: entity.kind,
            label: entity.label,
            subtype: entity.subtype,
            attributes: entity.metadata
        });
    }

    const wikilinks: string[] = [];
    const tags: string[] = [];
    const mentions: string[] = [];

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
        triples: [],
        backlinks: []
    };
}

function extractText(node: any): string {
    if (!node) return '';
    if (typeof node === 'string') return node;
    if (node.type === 'text' && node.text) return node.text;
    if (node.content && Array.isArray(node.content)) {
        return node.content.map(extractText).join('\n');
    }
    return '';
}
