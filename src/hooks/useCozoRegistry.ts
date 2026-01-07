import { useContext } from 'react';
import { useCozoContext, useCozoEntities, useCozoReady } from '@/contexts/CozoContext';
// Import RegisteredEntity type from tauri (same type, new location)
import type { RegisteredEntity } from '@/lib/tauri';
import type { EntityKind } from '@/lib/types/entityTypes';

export function useCozoRegistry() {
    return useCozoContext();
}

export function useEntities(): RegisteredEntity[] {
    return useCozoEntities();
}

export function useRegistryReady(): boolean {
    return useCozoReady();
}

export function useEntityById(id: string | null): RegisteredEntity | null {
    const { getEntityById, isReady } = useCozoContext();
    if (!id || !isReady) return null;
    return getEntityById(id);
}

export function useEntityByLabel(label: string | null): RegisteredEntity | null {
    const { findEntityByLabel, isReady } = useCozoContext();
    if (!label || !isReady) return null;
    return findEntityByLabel(label);
}

export function useEntitiesByKind(kind: EntityKind): RegisteredEntity[] {
    const { getEntitiesByKind, isReady } = useCozoContext();
    if (!isReady) return [];
    return getEntitiesByKind(kind);
}

export { useCozoContext, useCozoEntities, useCozoReady };
