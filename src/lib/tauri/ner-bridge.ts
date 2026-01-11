/**
 * NER Bridge - Tauri IPC wrappers for NER commands
 * 
 * This module provides TypeScript bindings for the Rust NER backend,
 * including model management, analysis requests, and suggestion handling.
 */

import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Types
// =============================================================================

/** Model availability status from Rust backend */
export interface NerModelStatus {
    available: boolean;
    model_path: string | null;
    tokenizer_path: string | null;
    size_bytes: number | null;
}

/** NER settings */
export interface NerSettings {
    enabled: boolean;
    auto_promote_threshold: number;
    suggest_threshold: number;
}

/** A pending NER suggestion from the backend */
export interface NerSuggestion {
    id: string;
    world_id: string;
    source_note_id: string;
    text: string;
    label: string;
    entity_type: string;
    byte_start: number;
    byte_end: number;
    confidence: number;
    inferred_at: number;
}

/** Response for analysis request */
export interface AnalysisRequestResponse {
    queued: boolean;
    doc_id: string;
}

/** Response for suggestions query */
export interface SuggestionsResponse {
    suggestions: NerSuggestion[];
    count: number;
}

/** Response for accept/reject actions (legacy) */
export interface ActionResponse {
    success: boolean;
    message: string;
    entity_id?: string;
}

/** Result of accepting a suggestion - includes full entity info for cache sync */
export interface AcceptResult {
    suggestion_id: string;
    promoted_entity_id: string;
    label: string;
    kind: string;
    source_note_id: string;
    is_new: boolean;
}

/** Response for model download */
export interface DownloadResponse {
    started: boolean;
    message: string;
}

// =============================================================================
// Model Management
// =============================================================================

/**
 * Check if the NER model is downloaded and available
 */
export async function nerGetModelStatus(): Promise<NerModelStatus> {
    try {
        return await invoke<NerModelStatus>('ner_get_model_status');
    } catch (error) {
        console.error('[NER Bridge] Failed to get model status:', error);
        return {
            available: false,
            model_path: null,
            tokenizer_path: null,
            model_name: 'gliner_small-v2.1',
        };
    }
}

/**
 * Start downloading the NER model from HuggingFace
 */
export async function nerDownloadModel(): Promise<DownloadResponse> {
    try {
        return await invoke<DownloadResponse>('ner_download_model');
    } catch (error) {
        console.error('[NER Bridge] Failed to start download:', error);
        return {
            started: false,
            message: error instanceof Error ? error.message : 'Unknown error',
        };
    }
}

// =============================================================================
// Analysis Requests
// =============================================================================

/**
 * Request NER analysis for a document
 * 
 * @param docId - The document/note ID
 * @param text - The text content to analyze
 * @param candidateLabels - Entity types to look for (e.g., ['person', 'location'])
 */
export async function nerRequestAnalysis(
    docId: string,
    text: string,
    candidateLabels: string[]
): Promise<AnalysisRequestResponse> {
    try {
        return await invoke<AnalysisRequestResponse>('ner_request_analysis', {
            docId,
            text,
            candidateLabels,
        });
    } catch (error) {
        console.error('[NER Bridge] Failed to request analysis:', error);
        return {
            queued: false,
            doc_id: docId,
        };
    }
}

// =============================================================================
// Suggestion Management
// =============================================================================

/**
 * Get pending NER suggestions for a specific note
 */
export async function nerGetSuggestions(noteId: string): Promise<NerSuggestion[]> {
    try {
        const response = await invoke<SuggestionsResponse>('ner_get_suggestions', {
            noteId,
        });
        return response.suggestions;
    } catch (error) {
        console.error('[NER Bridge] Failed to get suggestions:', error);
        return [];
    }
}

/**
 * Get all pending NER suggestions across all notes
 */
export async function nerGetAllSuggestions(): Promise<NerSuggestion[]> {
    try {
        const response = await invoke<SuggestionsResponse>('ner_get_all_suggestions');
        return response.suggestions;
    } catch (error) {
        console.error('[NER Bridge] Failed to get all suggestions:', error);
        return [];
    }
}

/**
 * Accept a NER suggestion (promote to entity)
 * 
 * @param suggestionId - The suggestion ID to accept
 * @returns AcceptResult with full entity info for cache sync
 */
export async function nerAcceptSuggestion(suggestionId: string): Promise<AcceptResult> {
    // Backend now returns AcceptResult directly (not wrapped in ActionResponse)
    return await invoke<AcceptResult>('ner_accept_suggestion', {
        suggestionId,
    });
}

/**
 * Reject a NER suggestion
 * 
 * @param suggestionId - The suggestion ID to reject
 */
export async function nerRejectSuggestion(suggestionId: string): Promise<ActionResponse> {
    try {
        return await invoke<ActionResponse>('ner_reject_suggestion', {
            suggestionId,
        });
    } catch (error) {
        console.error('[NER Bridge] Failed to reject suggestion:', error);
        return {
            success: false,
            message: error instanceof Error ? error.message : 'Unknown error',
        };
    }
}

/**
 * Get the total count of pending suggestions
 */
export async function nerPendingCount(): Promise<number> {
    try {
        return await invoke<number>('ner_pending_count');
    } catch (error) {
        console.error('[NER Bridge] Failed to get pending count:', error);
        return 0;
    }
}

/**
 * Clear all pending suggestions for a note
 */
export async function nerClearNoteSuggestions(noteId: string): Promise<ActionResponse> {
    try {
        return await invoke<ActionResponse>('ner_clear_note_suggestions', {
            noteId,
        });
    } catch (error) {
        console.error('[NER Bridge] Failed to clear suggestions:', error);
        return {
            success: false,
            message: error instanceof Error ? error.message : 'Unknown error',
        };
    }
}

// =============================================================================
// Settings Management
// =============================================================================

/**
 * Get current NER settings
 */
export async function nerGetSettings(): Promise<NerSettings> {
    try {
        return await invoke<NerSettings>('ner_get_settings');
    } catch (error) {
        console.error('[NER Bridge] Failed to get settings:', error);
        return {
            enabled: false,
            auto_promote_threshold: 0.90,
            suggest_threshold: 0.60,
        };
    }
}

/**
 * Update NER settings
 */
export async function nerUpdateSettings(settings: NerSettings): Promise<NerSettings> {
    try {
        return await invoke<NerSettings>('ner_update_settings', {
            settings,
        });
    } catch (error) {
        console.error('[NER Bridge] Failed to update settings:', error);
        return settings;
    }
}

// =============================================================================
// Testing / Manual Injection
// =============================================================================

/**
 * Manually add a suggestion (for testing)
 */
export async function nerAddSuggestion(
    worldId: string,
    noteId: string,
    text: string,
    label: string,
    start: number,
    end: number,
    confidence: number
): Promise<string | null> {
    try {
        return await invoke<string>('ner_add_suggestion', {
            worldId,
            noteId,
            text,
            label,
            start,
            end,
            confidence,
        });
    } catch (error) {
        console.error('[NER Bridge] Failed to add suggestion:', error);
        return null;
    }
}

// =============================================================================
// Convenience Helpers
// =============================================================================

/**
 * Check if we're running in Tauri environment
 */
export function isTauriNer(): boolean {
    return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Convert byte offset to character offset for ProseMirror
 * (UTF-8 bytes -> JS string indices)
 * 
 * This is needed because Rust uses byte offsets, but JS uses character offsets.
 * For ASCII text, they're the same. For UTF-8 with multibyte chars, they differ.
 */
export function byteToCharOffset(text: string, byteOffset: number): number {
    const encoder = new TextEncoder();
    let charIndex = 0;
    let byteIndex = 0;

    for (const char of text) {
        if (byteIndex >= byteOffset) break;
        byteIndex += encoder.encode(char).length;
        charIndex++;
    }

    return charIndex;
}

/**
 * Convert a NerSuggestion to character-based offsets for use in ProseMirror
 */
export function suggestionToCharOffsets(
    suggestion: NerSuggestion,
    text: string
): { start: number; end: number } {
    return {
        start: byteToCharOffset(text, suggestion.byte_start),
        end: byteToCharOffset(text, suggestion.byte_end),
    };
}
