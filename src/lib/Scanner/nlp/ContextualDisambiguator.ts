import { ResoRankScorer, TokenMetadata, CorpusStatistics, ProximityStrategy } from '@/lib/resorank';
import { getWinkProcessor, Token, Sentence } from './WinkProcessor';
import { entityRegistry, type RegisteredEntity } from '@/lib/cozo-stubs/graph-adapters';
import { EntityKind } from '../../types/entityTypes';

// Universal POS tags from wink-eng-lite-web-model
const POS_PATTERNS: Record<EntityKind, { before: string[]; after: string[] }> = {
    CHARACTER: {
        before: ['DET', 'PRON'],
        after: ['VERB', 'AUX'],
    },
    FACTION: {
        before: ['ADP', 'DET'],
        after: ['NOUN', 'VERB', 'AUX'],
    },
    LOCATION: {
        before: ['ADP'],
        after: ['PUNCT', 'CCONJ', 'VERB', 'AUX'],
    },
    NPC: {
        before: ['DET', 'PRON'],
        after: ['VERB', 'AUX'],
    },
    ITEM: {
        before: ['DET', 'ADJ', 'PRON'],
        after: ['VERB', 'AUX', 'ADP'],
    },
    SCENE: {
        before: ['ADP', 'DET'],
        after: ['PUNCT', 'VERB', 'AUX'],
    },
    EVENT: {
        before: ['ADP', 'DET'],
        after: ['VERB', 'AUX', 'ADP'],
    },
    CONCEPT: {
        before: ['ADJ', 'DET'],
        after: ['VERB', 'AUX', 'ADJ'],
    },
    ARC: {
        before: ['DET', 'ADJ'],
        after: ['VERB', 'AUX'],
    },
    ACT: {
        before: ['DET', 'NUM'],
        after: ['PUNCT', 'VERB', 'AUX'],
    },
    CHAPTER: {
        before: ['DET', 'NUM'],
        after: ['PUNCT', 'VERB', 'AUX'],
    },
    BEAT: {
        before: ['DET', 'ADJ'],
        after: ['VERB', 'AUX'],
    },
    TIMELINE: {
        before: ['ADP', 'DET'],
        after: ['NOUN', 'VERB', 'AUX'],
    },
    NARRATIVE: {
        before: ['ADJ', 'DET'],
        after: ['VERB', 'AUX'],
    },
    NETWORK: {
        before: ['ADJ', 'DET'],
        after: ['VERB', 'AUX'],
    },
};

export class ContextualDisambiguator {
    private resoRank: ResoRankScorer<string> | null = null;
    private initialized: boolean = false;
    private lastIndexedEntityCount = 0;

    constructor() { }

    /**
     * Lazy initialization of ResoRank index
     * Rebuilds only if entity count has changed
     */
    private buildResoRankIndexIfNeeded(): void {
        const entities = entityRegistry.getAllEntities();
        const currentCount = entities.length;

        if (this.initialized && this.lastIndexedEntityCount === currentCount) {
            return;
        }

        this.buildResoRankIndex(entities);
        this.lastIndexedEntityCount = currentCount;
    }

    /**
     * Build the ResoRank index from all registered entities
     */
    private buildResoRankIndex(entities: RegisteredEntity[]): void {
        const avgLabelLength = entities.reduce((sum, e) => sum + e.label.length, 0) / Math.max(1, entities.length);

        const corpusStats: CorpusStatistics = {
            totalDocuments: entities.length,
            averageFieldLengths: new Map([[0, avgLabelLength]]),
            averageDocumentLength: avgLabelLength,
        };

        // Create fresh instance to prevent memory leaks and ensure clean state
        this.resoRank = new ResoRankScorer<string>(
            {
                // Optimization: Use scalar config for simple entity matching
                enablePhraseBoost: true,
                proximityAlpha: 0.8, // High proximity importance for multi-word entities
            },
            corpusStats,
            ProximityStrategy.Pairwise
        );

        // Index all entities
        for (const entity of entities) {
            const tokens = this.tokenizeEntity(entity);
            const docMeta = {
                fieldLengths: new Map([[0, entity.label.length]]),
                totalTokenCount: tokens.size,
            };
            this.resoRank.indexDocument(entity.id, docMeta, tokens);
        }

        this.initialized = true;
        // console.log(`[ContextualDisambiguator] Indexed ${entities.length} entities`);
    }

    /**
     * Convert entity label/aliases into ResoRank tokens with corpus statistics
     */
    private tokenizeEntity(entity: RegisteredEntity): Map<string, TokenMetadata> {
        const tokens = new Map<string, TokenMetadata>();
        const allEntities = entityRegistry.getAllEntities();

        // Tokenize label
        const labelTokens = entity.label.toLowerCase().split(/\s+/);

        labelTokens.forEach((token, idx) => {
            // Calculate corpus doc frequency (how many entities contain this token)
            // This is O(N*M) where N=entities, M=tokens. For 10k entities this might be slow.
            // Optimization: In a real incremental system, we'd maintain a global term frequency index.
            // For now, iterating is safe for <1000 entities.
            let corpusDocFreq = 0;
            for (const other of allEntities) {
                if (other.normalizedLabel.includes(token) ||
                    other.aliases?.some(a => a.toLowerCase().includes(token))) {
                    corpusDocFreq++;
                }
            }
            corpusDocFreq = Math.max(1, corpusDocFreq);

            tokens.set(token, {
                fieldOccurrences: new Map([[0, {
                    tf: 1,
                    fieldLength: entity.label.length
                }]]),
                segmentMask: 1 << (Math.min(idx, 31)),
                corpusDocFrequency: corpusDocFreq,
            });
        });

        // Add aliases
        for (const alias of entity.aliases || []) {
            const aliasTokens = alias.toLowerCase().split(/\s+/);
            aliasTokens.forEach((token, idx) => {
                if (!tokens.has(token)) {
                    // Calculate CDF if new token
                    let corpusDocFreq = 0;
                    for (const other of allEntities) {
                        if (other.normalizedLabel.includes(token) ||
                            other.aliases?.some(a => a.toLowerCase().includes(token))) {
                            corpusDocFreq++;
                        }
                    }
                    corpusDocFreq = Math.max(1, corpusDocFreq);

                    tokens.set(token, {
                        fieldOccurrences: new Map([[0, {
                            tf: 1,
                            fieldLength: alias.length
                        }]]),
                        segmentMask: 1 << (Math.min(idx, 31)), // Simple segment mask for aliases
                        corpusDocFrequency: corpusDocFreq,
                    });
                }
            });
        }

        return tokens;
    }

    /**
     * Disambiguate an entity mention using context and ResoRank
     * @param mentionText - The text found in document
     * @param contextSentence - The sentence containing the mention
     * @param position - Character offset of mention in doc
     */
    public disambiguate(
        mentionText: string,
        contextSentence: Sentence,
        position: number
    ): Array<{ entity: RegisteredEntity; score: number; confidence: string }> {
        this.buildResoRankIndexIfNeeded();
        if (!this.resoRank) return [];

        const wink = getWinkProcessor();

        // 1. Contextual Verification (POS)
        // Check if consistent with expected POS patterns for candidate kinds
        const posContext = wink.getContextualPOS(contextSentence.text, position - contextSentence.start, 3);

        // 2. Semantic Resonance (ResoRank)
        // Extract context terms (nouns, verbs) from sentence to boost scoring
        const contextTerms = contextSentence.tokens
            .filter(t => t.pos === 'NOUN' || t.pos === 'PROPN' || t.pos === 'VERB')
            .map(t => t.lemma.toLowerCase())
            .filter(t => t !== mentionText.toLowerCase());

        // Construct query: Mention + Context Terms
        // We emphasize the mention itself but allow context to steer ranking
        const query = [
            ...mentionText.toLowerCase().split(/\s+/),
            // ...contextTerms // TODO: Weighted query expansion in ResoRank?
            // For now, ResoRank `search` just takes terms. 
            // If we add context terms, we might match wrong entities if they just share context.
            // Better strategy: Search for mention, then re-rank by context overlap?
            // Or: ResoRank supports "Boost" terms?
            // Current ResoRank API is simple search.
            // Let's search for the mention primarily.
        ];

        // Basic search first
        const results = this.resoRank.search(query, { limit: 5 });

        // Hydrate results
        const candidates = results
            .map(r => {
                const entity = entityRegistry.getEntityById(r.docId);
                return entity ? { entity, score: r.score, confidence: 'medium' } : null;
            })
            .filter((r): r is { entity: RegisteredEntity; score: number; confidence: string } => r !== null);

        // 3. Post-Ranking Refinement (heuristic)
        return candidates.map(candidate => {
            let confidence = 'medium';
            const kindPatterns = POS_PATTERNS[candidate.entity.kind];

            if (kindPatterns) {
                const matchesBefore = posContext.before.some(p => kindPatterns.before.includes(p));
                const matchesAfter = posContext.after.some(p => kindPatterns.after.includes(p));

                if (matchesBefore || matchesAfter) {
                    candidate.score *= 1.2; // Boost score
                    confidence = 'high';
                }
            }

            return { ...candidate, confidence };
        }).sort((a, b) => b.score - a.score);
    }
}
