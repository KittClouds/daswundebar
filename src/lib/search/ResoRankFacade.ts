/**
 * ResoRank WASM Facade
 * 
 * Lightweight TypeScript wrapper around the Rust ResoRankScorer WASM module.
 * Used for fast lexical search in the Folders tab.
 */

// NOTE: ResoRank WASM temporarily disabled during Tauri migration
// Phase 4 will implement via Tauri commands
// import init, { ResoRankScorer } from '@/lib/wasm/kittcore/kittcore';

// Stub class for Tauri migration
class ResoRankScorer {
    constructor(_config: any, _stats: any, _mode: string) { }
    indexDocument(_id: string, _meta: any, _tokens: any, _update: boolean) { }
    search(_terms: string[], _limit: number) { return []; }
    clear() { }
}

// Types matching the Rust WASM interface
export interface ResoRankSearchResult {
    doc_id: string;
    score: number;
    normalized_score?: number;
}

export interface TokenMetadata {
    fieldOccurrences: Record<number, { tf: number; fieldLength: number }>;
    segmentMask: number;
    corpusDocFrequency: number;
}

export interface DocumentMetadata {
    fieldLengths: Record<number, number>;
    totalTokenCount: number;
}

export interface CorpusStatistics {
    totalDocuments: number;
    averageFieldLengths: Record<number, number>;
    averageDocumentLength: number;
}

// Simple tokenizer - splits on whitespace and punctuation
function tokenize(text: string): string[] {
    return text
        .toLowerCase()
        .replace(/[^\w\s]/g, ' ')
        .split(/\s+/)
        .filter(t => t.length >= 2);
}

// Calculate segment mask for positional proximity
function calculateSegmentMask(positions: number[], docLength: number, maxSegments = 16): number {
    let mask = 0;
    for (const pos of positions) {
        const segment = Math.floor((pos / docLength) * maxSegments);
        if (segment < 32) {
            mask |= (1 << segment);
        }
    }
    return mask;
}

/**
 * ResoRank WASM Facade
 * 
 * Provides a simple API for indexing and searching notes using the
 * Rust ResoRankScorer WASM module.
 */
export class ResoRankFacade {
    private scorer: ResoRankScorer | null = null;
    private initialized = false;
    private initPromise: Promise<void> | null = null;
    private noteMetadata: Map<string, { title: string; wordCount: number }> = new Map();
    private documentCount = 0;
    private totalTitleLength = 0;
    private totalContentLength = 0;

    /**
     * Initialize the scorer (no WASM during Tauri migration)
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;
        if (this.initPromise) return this.initPromise;

        this.initPromise = (async () => {
            try {
                // WASM init disabled during Tauri migration
                // await init();
                this.createScorer();
                this.initialized = true;
                console.log('[ResoRankFacade] Initialized (stub mode)');
            } catch (e) {
                console.error('[ResoRankFacade] Init failed:', e);
                throw e;
            }
        })();

        return this.initPromise;
    }

    private createScorer(): void {
        const corpusStats: CorpusStatistics = {
            totalDocuments: Math.max(1, this.documentCount),
            averageFieldLengths: {
                0: this.documentCount > 0 ? this.totalTitleLength / this.documentCount : 5,
                1: this.documentCount > 0 ? this.totalContentLength / this.documentCount : 100,
            },
            averageDocumentLength: this.documentCount > 0
                ? (this.totalTitleLength + this.totalContentLength) / this.documentCount
                : 100,
        };

        const config = {
            k1: 1.2,
            proximityAlpha: 0.5,
            maxSegments: 16,
            proximityDecayLambda: 0.5,
            fieldParams: {
                0: { weight: 2.0, b: 0.75 }, // Title - boosted
                1: { weight: 1.0, b: 0.75 }, // Content
            },
            idfProximityScale: 5.0,
            enablePhraseBoost: true,
            phraseBoostMultiplier: 1.5,
            enableBMXEntropy: false,
            enableBMXSimilarity: false,
            useAdaptiveAlpha: false,
            entropyDenomWeight: null,
        };

        this.scorer = new ResoRankScorer(config, corpusStats, 'pairwise');
    }

    /**
     * Index a note for search
     */
    indexNote(id: string, title: string, content: string): void {
        if (!this.initialized || !this.scorer) {
            console.warn('[ResoRankFacade] Not initialized, queueing index');
            return;
        }

        // Remove old document if exists
        try {
            // Note: WASM scorer may not have removeDocument exposed
            // We'll re-index which overwrites
        } catch { }

        const titleTokens = tokenize(title);
        const contentTokens = tokenize(content);
        const allTokens = [...titleTokens, ...contentTokens];

        if (allTokens.length === 0) return;

        // Build token metadata map
        const tokenMap: Record<string, TokenMetadata> = {};
        const termPositions: Map<string, number[]> = new Map();

        // Track positions for segment mask
        let position = 0;
        for (const token of titleTokens) {
            if (!termPositions.has(token)) termPositions.set(token, []);
            termPositions.get(token)!.push(position++);
        }
        for (const token of contentTokens) {
            if (!termPositions.has(token)) termPositions.set(token, []);
            termPositions.get(token)!.push(position++);
        }

        // Build term frequencies per field
        const titleTf: Map<string, number> = new Map();
        const contentTf: Map<string, number> = new Map();

        for (const token of titleTokens) {
            titleTf.set(token, (titleTf.get(token) || 0) + 1);
        }
        for (const token of contentTokens) {
            contentTf.set(token, (contentTf.get(token) || 0) + 1);
        }

        // Build token metadata
        const uniqueTerms = new Set([...titleTokens, ...contentTokens]);
        for (const term of uniqueTerms) {
            const fieldOccurrences: Record<number, { tf: number; fieldLength: number }> = {};

            if (titleTf.has(term)) {
                fieldOccurrences[0] = { tf: titleTf.get(term)!, fieldLength: titleTokens.length };
            }
            if (contentTf.has(term)) {
                fieldOccurrences[1] = { tf: contentTf.get(term)!, fieldLength: contentTokens.length };
            }

            tokenMap[term] = {
                fieldOccurrences,
                segmentMask: calculateSegmentMask(
                    termPositions.get(term) || [],
                    allTokens.length,
                    16
                ),
                corpusDocFrequency: 1, // Will be updated across corpus
            };
        }

        const docMeta: DocumentMetadata = {
            fieldLengths: {
                0: titleTokens.length,
                1: contentTokens.length,
            },
            totalTokenCount: allTokens.length,
        };

        try {
            this.scorer.indexDocument(id, docMeta, tokenMap, true);

            // Update metadata
            this.noteMetadata.set(id, { title, wordCount: allTokens.length });
            this.documentCount++;
            this.totalTitleLength += titleTokens.length;
            this.totalContentLength += contentTokens.length;
        } catch (e) {
            console.error('[ResoRankFacade] Index failed:', e);
        }
    }

    /**
     * Bulk index multiple notes
     */
    indexNotes(notes: Array<{ id: string; title: string; content: string }>): void {
        for (const note of notes) {
            this.indexNote(note.id, note.title, note.content);
        }
        console.log(`[ResoRankFacade] Indexed ${notes.length} notes`);
    }

    /**
     * Search for notes matching the query
     */
    search(query: string, limit = 20): ResoRankSearchResult[] {
        if (!this.initialized || !this.scorer) {
            console.warn('[ResoRankFacade] Not initialized');
            return [];
        }

        const trimmed = query.trim();
        if (trimmed.length < 2) return [];

        const queryTerms = tokenize(trimmed);
        if (queryTerms.length === 0) return [];

        try {
            const results = this.scorer.search(queryTerms, limit) as ResoRankSearchResult[];
            return results;
        } catch (e) {
            console.error('[ResoRankFacade] Search failed:', e);
            return [];
        }
    }

    /**
     * Get note metadata by ID
     */
    getNoteMetadata(id: string): { title: string; wordCount: number } | undefined {
        return this.noteMetadata.get(id);
    }

    /**
     * Clear all indexed data
     */
    clear(): void {
        if (this.scorer) {
            try {
                this.scorer.clear();
            } catch { }
        }
        this.noteMetadata.clear();
        this.documentCount = 0;
        this.totalTitleLength = 0;
        this.totalContentLength = 0;
    }

    /**
     * Check if initialized
     */
    isReady(): boolean {
        return this.initialized && this.scorer !== null;
    }

    /**
     * Get statistics
     */
    getStats(): { documentCount: number; ready: boolean } {
        return {
            documentCount: this.documentCount,
            ready: this.isReady(),
        };
    }
}

// Singleton instance
export const resorankFacade = new ResoRankFacade();
