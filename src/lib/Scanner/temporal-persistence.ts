/**
 * Temporal Mention Persistence
 * 
 * Persists temporal mentions from Rust scanner to CozoDB.
 * Extracted from legacy scanner-v3 for standalone use.
 */

import { cozoDb } from '@/lib/cozo-stubs/db';
import { TEMPORAL_MENTION_QUERIES } from '@/lib/cozo-stubs/schema';
import { generateId } from '@/lib/utils/ids';
import type { TemporalMention } from './bridge';

// ==================== CONTEXT EXTRACTION ====================

function extractSentenceContext(text: string, start: number, end: number): string {
    const sentenceEnders = /[.!?]/g;
    let sentenceStart = 0;
    let match;
    while ((match = sentenceEnders.exec(text)) !== null) {
        if (match.index >= start) break;
        sentenceStart = match.index + 1;
    }
    sentenceEnders.lastIndex = end;
    const nextEnd = sentenceEnders.exec(text);
    const sentenceEnd = nextEnd ? nextEnd.index + 1 : Math.min(end + 100, text.length);
    const sentence = text.slice(sentenceStart, sentenceEnd).trim();
    if (sentence.length > 300) {
        const mentionCenter = (start + end) / 2 - sentenceStart;
        const contextStart = Math.max(0, mentionCenter - 150);
        const contextEnd = Math.min(sentence.length, mentionCenter + 150);
        return '...' + sentence.slice(contextStart, contextEnd).trim() + '...';
    }
    return sentence;
}

function extractParagraphContext(text: string, start: number): string | undefined {
    const paragraphs = text.split(/\n\n+/);
    let currentPos = 0;
    for (const para of paragraphs) {
        const paraEnd = currentPos + para.length;
        if (start >= currentPos && start <= paraEnd) {
            return para.length > 500 ? para.slice(0, 500) + '...' : para;
        }
        currentPos = paraEnd + 2;
    }
    return undefined;
}

// ==================== PERSISTENCE ====================

export interface PersistenceResult {
    added: number;
    failed: number;
    durationMs: number;
}

export async function persistTemporalMentions(
    noteId: string,
    mentions: TemporalMention[],
    fullText: string,
    episodeId?: string
): Promise<PersistenceResult> {
    const start = performance.now();
    let added = 0;
    let failed = 0;

    if (!cozoDb.isReady()) {
        return { added: 0, failed: mentions.length, durationMs: 0 };
    }

    const rows: any[][] = [];
    for (const mention of mentions) {
        try {
            const id = generateId();
            const contextSentence = extractSentenceContext(fullText, mention.start, mention.end);
            const contextParagraph = extractParagraphContext(fullText, mention.start);
            const createdAt = Date.now() / 1000;
            rows.push([
                id,
                episodeId || null,
                noteId,
                mention.kind,
                mention.text,
                mention.start,
                mention.end,
                contextSentence,
                contextParagraph || null,
                mention.confidence,
                mention.metadata?.weekday_index ?? null,
                mention.metadata?.month_index ?? null,
                mention.metadata?.narrative_number ?? null,
                mention.metadata?.direction ?? null,
                mention.metadata?.era_name ?? null,
                mention.metadata?.era_year ?? null,
                createdAt,
            ]);
        } catch (err) {
            failed++;
        }
    }

    if (rows.length > 0) {
        try {
            const result = cozoDb.runQuery(TEMPORAL_MENTION_QUERIES.upsertBatch, { rows });
            if (result.ok) {
                added = rows.length;
            } else {
                failed += rows.length;
            }
        } catch (err) {
            failed += rows.length;
        }
    }

    return { added, failed, durationMs: performance.now() - start };
}

export async function clearTemporalMentions(noteId: string): Promise<void> {
    if (!cozoDb.isReady()) return;
    try {
        cozoDb.runQuery(TEMPORAL_MENTION_QUERIES.deleteByNoteId, { note_id: noteId });
    } catch (err) {
        console.error('[TemporalPersist] Failed to clear:', err);
    }
}
