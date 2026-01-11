import React, { createContext, useContext, useState, useCallback, useEffect, ReactNode } from 'react';
import type { NEREntity, NERModelStatus } from '@/lib/extraction';
import {
    nerGetSuggestions,
    nerAcceptSuggestion,
    nerRejectSuggestion,
    nerGetModelStatus,
    nerGetSettings,
    isTauriNer,
    type NerSuggestion,
    type NerSettings,
    type NerModelStatus as RustModelStatus,
    type AcceptResult,
} from '@/lib/tauri/ner-bridge';
import { smartGraphRegistry } from '@/lib/tauri/smart-graph-registry';
import { useCozoContext } from '@/contexts/CozoContext';

// =============================================================================
// Types
// =============================================================================

interface NERContextValue {
    // Legacy state (frontend NER entities)
    entities: NEREntity[];
    modelStatus: NERModelStatus;
    isAnalyzing: boolean;
    error: string | null;

    // Rust NER suggestions
    suggestions: NerSuggestion[];
    isFetchingSuggestions: boolean;
    rustModelStatus: RustModelStatus | null;
    settings: NerSettings;

    // Current note context
    currentNoteId: string | null;

    // Legacy actions
    setEntities: (entities: NEREntity[] | ((prev: NEREntity[]) => NEREntity[])) => void;
    clearEntities: () => void;
    setModelStatus: (status: NERModelStatus) => void;
    setIsAnalyzing: (analyzing: boolean) => void;
    setError: (error: string | null) => void;

    // Rust NER actions
    setCurrentNoteId: (noteId: string | null) => void;
    refreshSuggestions: (noteId?: string) => Promise<void>;
    acceptSuggestion: (suggestionId: string) => Promise<boolean>;
    rejectSuggestion: (suggestionId: string) => Promise<boolean>;
    refreshModelStatus: () => Promise<void>;
}

const NERContext = createContext<NERContextValue | null>(null);

// =============================================================================
// Provider
// =============================================================================

interface NERProviderProps {
    children: ReactNode;
}

export function NERProvider({ children }: NERProviderProps) {
    // Legacy state
    const [entities, setEntities] = useState<NEREntity[]>([]);
    const [modelStatus, setModelStatus] = useState<NERModelStatus>('idle');
    const [isAnalyzing, setIsAnalyzing] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Rust NER state
    const [suggestions, setSuggestions] = useState<NerSuggestion[]>([]);
    const [isFetchingSuggestions, setIsFetchingSuggestions] = useState(false);
    const [rustModelStatus, setRustModelStatus] = useState<RustModelStatus | null>(null);
    const [settings, setSettings] = useState<NerSettings>({
        enabled: false,
        auto_promote_threshold: 0.90,
        suggest_threshold: 0.60,
    });
    const [currentNoteId, setCurrentNoteId] = useState<string | null>(null);

    // Access CozoContext to refresh entity panel after accept
    const { refreshEntities } = useCozoContext();

    // Clear legacy entities
    const clearEntities = useCallback(() => {
        setEntities([]);
    }, []);

    // Refresh suggestions from Rust backend
    const refreshSuggestions = useCallback(async (noteId?: string) => {
        const targetNoteId = noteId || currentNoteId;
        if (!targetNoteId || !isTauriNer()) {
            return;
        }

        setIsFetchingSuggestions(true);
        try {
            const newSuggestions = await nerGetSuggestions(targetNoteId);
            setSuggestions(newSuggestions);
        } catch (err) {
            console.error('[NERContext] Failed to refresh suggestions:', err);
        } finally {
            setIsFetchingSuggestions(false);
        }
    }, [currentNoteId]);

    // Accept a suggestion - returns AcceptResult with full entity info
    const acceptSuggestion = useCallback(async (suggestionId: string): Promise<boolean> => {
        if (!isTauriNer()) {
            console.warn('[NERContext] Not in Tauri environment');
            return false;
        }

        try {
            const result: AcceptResult = await nerAcceptSuggestion(suggestionId);

            // Remove from local state immediately for responsive UI
            setSuggestions(prev => prev.filter(s => s.id !== suggestionId));

            console.log(
                '[NERContext] Accepted suggestion:', suggestionId,
                '→ Entity:', result.promoted_entity_id,
                '| Kind:', result.kind,
                '| Label:', result.label,
                '| New:', result.is_new
            );

            // Sync to SmartGraphRegistry cache for immediate highlighting
            if (result.is_new) {
                smartGraphRegistry.addEntityFromNER(
                    result.promoted_entity_id,
                    result.label,
                    result.kind
                );
            }

            // Trigger CozoContext refresh so EntitiesPanel updates
            refreshEntities();

            return true;
        } catch (err) {
            console.error('[NERContext] Accept error:', err);
            setError(err instanceof Error ? err.message : 'Unknown error');
            return false;
        }
    }, []);

    // Reject a suggestion
    const rejectSuggestion = useCallback(async (suggestionId: string): Promise<boolean> => {
        if (!isTauriNer()) {
            console.warn('[NERContext] Not in Tauri environment');
            return false;
        }

        try {
            const response = await nerRejectSuggestion(suggestionId);

            if (response.success) {
                // Remove from local state immediately for responsive UI
                setSuggestions(prev => prev.filter(s => s.id !== suggestionId));
                console.log('[NERContext] Rejected suggestion:', suggestionId);
                return true;
            } else {
                console.error('[NERContext] Reject failed:', response.message);
                setError(response.message);
                return false;
            }
        } catch (err) {
            console.error('[NERContext] Reject error:', err);
            setError(err instanceof Error ? err.message : 'Unknown error');
            return false;
        }
    }, []);

    // Refresh model status
    const refreshModelStatus = useCallback(async () => {
        if (!isTauriNer()) {
            return;
        }

        try {
            const status = await nerGetModelStatus();
            setRustModelStatus(status);
        } catch (err) {
            console.error('[NERContext] Failed to get model status:', err);
        }
    }, []);

    // Initial load: fetch model status and settings
    useEffect(() => {
        if (!isTauriNer()) {
            return;
        }

        // Fetch model status on mount
        refreshModelStatus();

        // Fetch settings on mount
        nerGetSettings().then(setSettings).catch(err => {
            console.error('[NERContext] Failed to get settings:', err);
        });
    }, [refreshModelStatus]);

    // Refresh suggestions when note changes
    useEffect(() => {
        if (currentNoteId && isTauriNer()) {
            refreshSuggestions(currentNoteId);
        } else {
            // Clear suggestions when no note selected
            setSuggestions([]);
        }
    }, [currentNoteId, refreshSuggestions]);

    const value: NERContextValue = {
        // Legacy
        entities,
        modelStatus,
        isAnalyzing,
        error,

        // Rust NER
        suggestions,
        isFetchingSuggestions,
        rustModelStatus,
        settings,
        currentNoteId,

        // Legacy actions
        setEntities,
        clearEntities,
        setModelStatus,
        setIsAnalyzing,
        setError,

        // Rust NER actions
        setCurrentNoteId,
        refreshSuggestions,
        acceptSuggestion,
        rejectSuggestion,
        refreshModelStatus,
    };

    return (
        <NERContext.Provider value={value}>
            {children}
        </NERContext.Provider>
    );
}

// =============================================================================
// Hooks
// =============================================================================

export function useNER(): NERContextValue {
    const context = useContext(NERContext);
    if (!context) {
        throw new Error('useNER must be used within a NERProvider');
    }
    return context;
}

// Optional hook for components that don't need full context
export function useNEREntities(): NEREntity[] {
    const context = useContext(NERContext);
    return context?.entities ?? [];
}

// Hook for Rust NER suggestions
export function useNERSuggestions(): NerSuggestion[] {
    const context = useContext(NERContext);
    return context?.suggestions ?? [];
}

// Hook for suggestion actions
export function useNERSuggestionActions() {
    const context = useContext(NERContext);
    return {
        accept: context?.acceptSuggestion ?? (async () => false),
        reject: context?.rejectSuggestion ?? (async () => false),
        refresh: context?.refreshSuggestions ?? (async () => { }),
    };
}

// Hook for model status
export function useNERModelStatus() {
    const context = useContext(NERContext);
    return {
        rustStatus: context?.rustModelStatus ?? null,
        legacyStatus: context?.modelStatus ?? 'idle',
        isModelAvailable: context?.rustModelStatus?.available ?? false,
    };
}
