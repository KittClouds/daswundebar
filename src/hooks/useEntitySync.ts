import { useEffect, useRef, useCallback } from 'react';
import { useJotaiNotes } from '@/hooks/useJotaiNotes';
import { useCozoContext } from '@/contexts/CozoContext';
// Use smart graph registry with local caching + smart hydration
import { smartGraphRegistry } from '@/lib/tauri';
import type { EntityKind } from '@/lib/types/entityTypes';

interface ExtractedEntity {
    label: string;
    kind: EntityKind;
    subtype?: string;
}

interface UseEntitySyncOptions {
    debounceMs?: number;
    autoSync?: boolean;
    enabled?: boolean;
}

function hashContent(content: string): string {
    let hash = 0;
    for (let i = 0; i < content.length; i++) {
        const char = content.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
    }
    return hash.toString(36);
}

interface TipTapNode {
    type?: string;
    text?: string;
    content?: TipTapNode[];
}

function extractText(node: TipTapNode | string | null | undefined): string {
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

function parseEntitiesFromText(text: string): ExtractedEntity[] {
    const results: ExtractedEntity[] = [];
    // Match [KIND|Label] or [KIND:SUBTYPE|Label] or [KIND|Label|{attrs}]
    // Capture only the label part, not the optional JSON attributes
    const regex = /\[([A-Z_]+)(?::([A-Z_]+))?\|([^|\]]+)(?:\|[^\]]+)?\]/g;

    let match;
    while ((match = regex.exec(text)) !== null) {
        const [, kind, subtype, label] = match;
        if (label.includes('->')) continue;

        results.push({
            kind: kind as EntityKind,
            label: label.trim(),
            subtype
        });
    }

    return results;
}

function extractEntitiesFromNote(noteContent: string): ExtractedEntity[] {
    let plainText = '';
    try {
        const doc = JSON.parse(noteContent);
        plainText = extractText(doc);
    } catch {
        plainText = noteContent;
    }

    return parseEntitiesFromText(plainText);
}

export function useEntitySync(options: UseEntitySyncOptions = {}) {
    const { debounceMs = 2000, autoSync = true, enabled = true } = options;

    const { state } = useJotaiNotes();
    const { isReady, refreshEntities } = useCozoContext();

    const syncedNotesRef = useRef<Map<string, string>>(new Map());
    const debounceTimerRef = useRef<NodeJS.Timeout | null>(null);
    const isSyncingRef = useRef(false);

    const syncEntities = useCallback(async () => {
        if (!isReady || isSyncingRef.current) return;

        isSyncingRef.current = true;

        try {
            let syncedCount = 0;

            for (const note of state.notes) {
                const contentHash = hashContent(note.content);

                if (syncedNotesRef.current.get(note.id) === contentHash) {
                    continue;
                }

                const entities = extractEntitiesFromNote(note.content);

                for (const entity of entities) {
                    try {
                        await smartGraphRegistry.registerEntity(
                            entity.label,
                            entity.kind,
                            note.id,
                            {
                                subtype: entity.subtype,
                                source: 'extraction',
                            }
                        );
                    } catch (err) {
                        console.warn('[useEntitySync] Failed to register entity:', entity.label, err);
                    }
                }

                syncedNotesRef.current.set(note.id, contentHash);
                syncedCount++;
            }

            if (syncedCount > 0) {
                refreshEntities();

                // SMART HYDRATION: Use orchestrator for unified re-hydration
                try {
                    const { getResolvedPipeline } = await import('@/lib/Scanner/pipeline-config');
                    const pipeline = getResolvedPipeline();

                    if (pipeline === 'tauri') {
                        // Tauri mode: Use orchestrator's unified rehydration
                        const { tauriOrchestrator } = await import('@/lib/tauri');
                        await tauriOrchestrator.rehydrate();
                    } else {
                        // WASM mode: Manual hydration (legacy path)
                        const { scannerFacade } = await import('@/lib/scanner');
                        const { highlighterBridge } = await import('@/lib/highlighter');

                        const entityDefs = await smartGraphRegistry.getEntitiesForHydration();
                        if (entityDefs) {
                            await scannerFacade.hydrateEntities(entityDefs);
                            highlighterBridge.hydrateEntities(entityDefs);
                            console.log(`[useEntitySync] Smart hydration: ${entityDefs.length} entities (WASM)`);
                        } else {
                            console.log('[useEntitySync] Smart hydration: skipped (no changes)');
                        }
                    }
                } catch (err) {
                    console.warn('[useEntitySync] Failed to rehydrate:', err);
                }
            }
        } catch (err) {
            console.error('[useEntitySync] Sync failed:', err);
        } finally {
            isSyncingRef.current = false;
        }
    }, [isReady, state.notes, refreshEntities]);

    useEffect(() => {
        if (!enabled || !autoSync || !isReady) return;

        if (debounceTimerRef.current) {
            clearTimeout(debounceTimerRef.current);
        }

        debounceTimerRef.current = setTimeout(() => {
            syncEntities();
        }, debounceMs);

        return () => {
            if (debounceTimerRef.current) {
                clearTimeout(debounceTimerRef.current);
            }
        };
    }, [enabled, autoSync, isReady, state.notes, debounceMs, syncEntities]);

    const forceSync = useCallback(async () => {
        syncedNotesRef.current.clear();
        await syncEntities();
    }, [syncEntities]);

    const clearSyncCache = useCallback(() => {
        syncedNotesRef.current.clear();
    }, []);

    return {
        forceSync,
        clearSyncCache,
        isSyncing: isSyncingRef.current,
    };
}

export default useEntitySync;
