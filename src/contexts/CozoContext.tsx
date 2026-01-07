import React, { createContext, useContext, useState, useEffect, useCallback, useMemo, useRef, ReactNode } from 'react';
// ADAPTER MIGRATION: Using smartGraphRegistry internally while keeping same external API
// This allows A/B testing and gradual migration
import { smartGraphRegistry, type RegisteredEntity } from '@/lib/tauri';
import type { EntityKind } from '@/lib/types/entityTypes';

interface EntityStats {
    entityKind: EntityKind;
    entityLabel: string;
    mentionsInThisNote: number;
    mentionsAcrossVault: number;
    appearanceCount: number;
}

interface CozoContextValue {
    isReady: boolean;
    isInitializing: boolean;
    error: Error | null;
    entities: RegisteredEntity[];
    entityCount: number;
    refreshEntities: () => void;
    getEntityStats: () => EntityStats[];
    getEntityById: (id: string) => RegisteredEntity | null;
    findEntityByLabel: (label: string) => RegisteredEntity | null;
    getEntitiesByKind: (kind: EntityKind) => RegisteredEntity[];
    deleteEntity: (id: string) => Promise<boolean>;
    clearAllEntities: () => Promise<void>;
}

const CozoContext = createContext<CozoContextValue | null>(null);

interface CozoProviderProps {
    children: ReactNode;
}

export function CozoProvider({ children }: CozoProviderProps) {
    const [isReady, setIsReady] = useState(false);
    const [isInitializing, setIsInitializing] = useState(true);
    const [error, setError] = useState<Error | null>(null);
    const [entities, setEntities] = useState<RegisteredEntity[]>([]);
    const [refreshTrigger, setRefreshTrigger] = useState(0);

    const initAttempted = useRef(false);

    useEffect(() => {
        if (initAttempted.current) return;
        initAttempted.current = true;

        const initialize = async () => {
            try {
                setIsInitializing(true);
                setError(null);

                // Use smartGraphRegistry (Rust backend) instead of old entityRegistry
                await smartGraphRegistry.init();

                // SYNC read from local cache - no IPC overhead!
                const allEntities = smartGraphRegistry.getAllEntities();
                setEntities(allEntities);
                setIsReady(true);

                console.log('[CozoContext] Initialized with', allEntities.length, 'entities (via SmartGraphRegistry)');
            } catch (err) {
                console.error('[CozoContext] Initialization failed:', err);
                setError(err instanceof Error ? err : new Error('Failed to initialize entity registry'));
                setIsReady(false);
            } finally {
                setIsInitializing(false);
            }
        };

        initialize();
    }, []);

    useEffect(() => {
        if (!isReady) return;

        try {
            // SYNC read from local cache
            const allEntities = smartGraphRegistry.getAllEntities();
            setEntities(allEntities);
        } catch (err) {
            console.error('[CozoContext] Failed to refresh entities:', err);
        }
    }, [isReady, refreshTrigger]);

    const refreshEntities = useCallback(() => {
        setRefreshTrigger(prev => prev + 1);
    }, []);

    const getEntityStats = useCallback((): EntityStats[] => {
        return entities.map(entity => ({
            entityKind: entity.kind,
            entityLabel: entity.label,
            mentionsInThisNote: 0,
            mentionsAcrossVault: entity.totalMentions,
            appearanceCount: entity.mentionsByNote.size,
        }));
    }, [entities]);

    const getEntityById = useCallback((id: string): RegisteredEntity | null => {
        if (!isReady) return null;
        return smartGraphRegistry.getEntityById(id);
    }, [isReady]);

    const findEntityByLabel = useCallback((label: string): RegisteredEntity | null => {
        if (!isReady) return null;
        return smartGraphRegistry.findEntityByLabel(label);
    }, [isReady]);

    const getEntitiesByKind = useCallback((kind: EntityKind): RegisteredEntity[] => {
        if (!isReady) return [];
        return smartGraphRegistry.getEntitiesByKind(kind);
    }, [isReady]);

    const deleteEntity = useCallback(async (id: string): Promise<boolean> => {
        if (!isReady) return false;
        try {
            const result = await smartGraphRegistry.deleteEntity(id);
            if (result) {
                refreshEntities();
            }
            return result;
        } catch (err) {
            console.error('[CozoContext] Failed to delete entity:', err);
            return false;
        }
    }, [isReady, refreshEntities]);

    const clearAllEntities = useCallback(async (): Promise<void> => {
        if (!isReady) return;
        try {
            // TODO: Add clear() method to smartGraphRegistry
            console.warn('[CozoContext] clearAllEntities not yet implemented in smartGraphRegistry');
            refreshEntities();
        } catch (err) {
            console.error('[CozoContext] Failed to clear entities:', err);
            throw err;
        }
    }, [isReady, refreshEntities]);

    const value = useMemo<CozoContextValue>(() => ({
        isReady,
        isInitializing,
        error,
        entities,
        entityCount: entities.length,
        refreshEntities,
        getEntityStats,
        getEntityById,
        findEntityByLabel,
        getEntitiesByKind,
        deleteEntity,
        clearAllEntities,
    }), [
        isReady,
        isInitializing,
        error,
        entities,
        refreshEntities,
        getEntityStats,
        getEntityById,
        findEntityByLabel,
        getEntitiesByKind,
        deleteEntity,
        clearAllEntities,
    ]);

    return (
        <CozoContext.Provider value={value}>
            {children}
        </CozoContext.Provider>
    );
}

export function useCozoContext(): CozoContextValue {
    const context = useContext(CozoContext);
    if (!context) {
        throw new Error('useCozoContext must be used within a CozoProvider');
    }
    return context;
}

export function useCozoEntities(): RegisteredEntity[] {
    const context = useContext(CozoContext);
    return context?.entities ?? [];
}

export function useCozoReady(): boolean {
    const context = useContext(CozoContext);
    return context?.isReady ?? false;
}
