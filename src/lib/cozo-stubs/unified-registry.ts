/**
 * CozoDB UnifiedRegistry Stub - DEPRECATED
 * 
 * Use smartGraphRegistry from @/lib/tauri instead.
 */

import type { EntityKind } from '@/lib/types/entityTypes';

export interface CozoEntity {
    id: string;
    name: string;
    entityKind: EntityKind;
    aliases: string[];
}

export const unifiedRegistry = {
    isReady: () => false,
    getAll: (): CozoEntity[] => {
        console.warn('[unifiedRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return [];
    },
    getById: (_id: string): CozoEntity | null => {
        console.warn('[unifiedRegistry STUB] Use smartGraphRegistry from @/lib/tauri instead');
        return null;
    },
};
