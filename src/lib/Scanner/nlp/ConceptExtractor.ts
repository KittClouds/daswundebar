/**
 * ConceptExtractor - Phase 5 Feature: Implicit Concept Extraction
 * 
 * Extracts high-value common noun phrases (Concepts) that are not 
 * explicit entities, enabling the construction of a Concept Graph.
 * 
 * Features:
 * - Common noun phrase (Noun Chunk) extraction
 * - Stop-word and noise filtering
 * - Term Frequency (TF) based concept ranking
 * - Contextual co-occurrence relation extraction
 * - Analysis results caching
 */

import {
    getWinkProcessor,
    type EnhancedLinguisticAnalysis,
    type NounChunk,
    type Token
} from './WinkProcessor';
import { entityRegistry } from '@/lib/cozo-stubs/graph-adapters';

// ==================== TYPE DEFINITIONS ====================

export interface Concept {
    label: string;          // Normalized name ("Dark Magic")
    originalText: string;   // As found in text ("dark magics")
    frequency: number;      // Total mentions in document
    firstPosition: number;  // Character offset of first mention
    isCompound: boolean;    // If it's a multi-token phrase
    mentions: Array<{
        sentenceIndex: number;
        start: number;
        end: number;
    }>;
}

export interface ConceptRelation {
    concept1: string;
    concept2: string;
    frequency: number;
    strength: number;       // Normalized [0-1] based on distance/frequency
    noteId: string;
    context: string;        // Sentence where they co-occur
}

// ==================== CONCEPT EXTRACTOR ====================

export class ConceptExtractor {
    private wink = getWinkProcessor();
    private analysisCache = new Map<string, EnhancedLinguisticAnalysis>();

    /**
     * Clear extraction cache
     */
    clearCache(): void {
        this.analysisCache.clear();
    }

    /**
     * Get or compute enhanced analysis
     */
    private getOrAnalyze(text: string): EnhancedLinguisticAnalysis {
        if (!this.analysisCache.has(text)) {
            this.analysisCache.set(text, this.wink.analyzeEnhanced(text));
        }
        return this.analysisCache.get(text)!;
    }

    /**
     * Extract concepts from text
     */
    extractConcepts(text: string): Concept[] {
        const analysis = this.getOrAnalyze(text);
        const conceptMap = new Map<string, Concept>();

        // 1. Process all noun chunks
        for (const chunk of analysis.nounChunks) {
            const normalized = this.normalizeConcept(chunk.text);

            // 2. Filter noise
            if (!this.isValidConcept(normalized, chunk)) {
                continue;
            }

            // 3. Update or create concept
            const existing = conceptMap.get(normalized);
            if (existing) {
                existing.frequency++;
                existing.mentions.push({
                    sentenceIndex: chunk.tokens[0].sentenceIndex,
                    start: chunk.start,
                    end: chunk.end
                });
            } else {
                conceptMap.set(normalized, {
                    label: normalized,
                    originalText: chunk.text,
                    frequency: 1,
                    firstPosition: chunk.start,
                    isCompound: chunk.tokens.length > 1,
                    mentions: [{
                        sentenceIndex: chunk.tokens[0].sentenceIndex,
                        start: chunk.start,
                        end: chunk.end
                    }]
                });
            }
        }

        return Array.from(conceptMap.values())
            .filter(c => c.frequency >= 1) // Could increase threshold
            .sort((a, b) => b.frequency - a.frequency);
    }

    /**
     * Extract relations between concepts based on co-occurrence
     */
    extractConceptRelations(concepts: Concept[], text: string, noteId: string): ConceptRelation[] {
        const analysis = this.getOrAnalyze(text);
        const relations: ConceptRelation[] = [];
        const labelToConcept = new Map(concepts.map(c => [c.label, c]));

        // Group mentions by sentence for co-occurrence
        for (const sentence of analysis.sentences) {
            const conceptsInSentence: string[] = [];

            // Find which concepts are in this sentence
            for (const concept of concepts) {
                const hasMention = concept.mentions.some(m => m.sentenceIndex === sentence.index);
                if (hasMention) {
                    conceptsInSentence.push(concept.label);
                }
            }

            // Create pairs
            for (let i = 0; i < conceptsInSentence.length; i++) {
                for (let j = i + 1; j < conceptsInSentence.length; j++) {
                    const c1 = conceptsInSentence[i];
                    const c2 = conceptsInSentence[j];

                    relations.push({
                        concept1: c1,
                        concept2: c2,
                        frequency: 1,
                        strength: 0.5, // Default for sentence-level co-occurrence
                        noteId,
                        context: sentence.text
                    });
                }
            }
        }

        // Deduplicate and aggregate relations
        return this.aggregateRelations(relations);
    }

    private aggregateRelations(relations: ConceptRelation[]): ConceptRelation[] {
        const aggMap = new Map<string, ConceptRelation>();

        for (const rel of relations) {
            const key = [rel.concept1, rel.concept2].sort().join('|');
            const existing = aggMap.get(key);

            if (existing) {
                existing.frequency++;
                existing.strength = Math.min(1.0, existing.strength + 0.1);
            } else {
                aggMap.set(key, { ...rel });
            }
        }

        return Array.from(aggMap.values());
    }

    /**
     * Basic normalization: lowercase, trim, remove certain determiners
     */
    private normalizeConcept(text: string): string {
        return text.toLowerCase()
            .replace(/^(the|a|an)\s+/i, '')
            .trim();
    }

    /**
     * Logic to filter out non-meaningful concepts
     */
    private isValidConcept(label: string, chunk: NounChunk): boolean {
        // 1. Core length filter
        if (label.length < 3) return false;

        // 2. Stop words check (Wink already tags them, but double check label)
        if (chunk.tokens.every(t => t.isStopWord)) return false;

        // 3. Character filter (avoid purely numeric or symbolic)
        if (/^[\d\s\p{P}]+$/u.test(label)) return false;

        // 4. Register entity check: If it's ALREADY a registered entity, skip it
        // (Concepts are for the "dark matter" between nodes)
        if (entityRegistry.findEntity(label)) return false;

        return true;
    }
}

// ==================== SINGLETON INSTANCE ====================

let conceptExtractorInstance: ConceptExtractor | null = null;

export function getConceptExtractor(): ConceptExtractor {
    if (!conceptExtractorInstance) {
        conceptExtractorInstance = new ConceptExtractor();
    }
    return conceptExtractorInstance;
}
