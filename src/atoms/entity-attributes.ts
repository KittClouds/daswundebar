/**
 * Entity Attributes Atoms
 * 
 * First-class fact sheet system - entity-owned attribute storage.
 * Provides reactive state for entity attributes, meta cards, and field schemas.
 * 
 * MEMORY-ONLY: Attributes are not persisted to database.
 * They exist only in the current session for display purposes.
 * Future: Can be persisted to SurrealDB entity.attributes field.
 */

import { atom } from 'jotai';
import { generateId } from '@/lib/utils/ids';
import { atomFamily } from '@/atoms/utils/atomFamily';

// ============================================
// TYPES
// ============================================

export type FieldType =
    | 'text' | 'number' | 'array' | 'object' | 'boolean'
    | 'slider' | 'counter' | 'toggle' | 'date' | 'color'
    | 'rating' | 'tags' | 'entity-link' | 'rich-text' | 'progress';

export interface EntityAttribute {
    id: string;
    entityId: string;
    fieldName: string;
    fieldType: FieldType;
    value: any;
    schemaId?: string;
    cardId?: string;
    createdAt: number;
    updatedAt: number;
}

export interface MetaCard {
    id: string;
    ownerId: string; // entity_id
    name: string;
    color?: string;
    icon?: string;
    displayOrder: number;
    isCollapsed: boolean;
    createdAt: number;
    updatedAt: number;
}

export interface MetaCardField {
    id: string;
    cardId: string;
    fieldName: string;
    schemaId?: string;
    customSchema?: FieldSchema;
    layoutHint?: 'full' | 'half' | 'third' | 'quarter';
    displayOrder: number;
}

export interface FieldSchema {
    id: string;
    name: string;
    fieldType: FieldType;
    label: string;
    description?: string;
    metadata?: Record<string, any>; // min, max, options, step, etc.
    validation?: ValidationRule[];
    defaultValue?: any;
    isSystem: boolean;
    createdAt: number;
    updatedAt: number;
}

export interface ValidationRule {
    type: 'required' | 'min' | 'max' | 'pattern' | 'custom';
    value?: any;
    message: string;
}

// ============================================
// BASE ATOMS (In-Memory Only)
// ============================================

// Cache for entity attributes (keyed by entityId)
const entityAttributesCache = atom<Map<string, EntityAttribute[]>>(new Map());

// Cache for meta cards (keyed by ownerId)
const metaCardsCache = atom<Map<string, MetaCard[]>>(new Map());

// Field schemas cache
const fieldSchemasAtom = atom<FieldSchema[]>([]);

// Loading state
export const isLoadingAttributesAtom = atom<boolean>(false);

// ============================================
// ENTITY ATTRIBUTES - READ
// ============================================

/**
 * Get all attributes for a specific entity
 * Returns from in-memory cache only
 */
export const entityAttributesFamily = atomFamily((entityId: string) =>
    atom((get) => {
        const cache = get(entityAttributesCache);
        return cache.get(entityId) || [];
    })
);

/**
 * Get a single attribute value by field name
 */
export const getAttributeAtom = atomFamily(
    (params: { entityId: string; fieldName: string }) =>
        atom((get) => {
            const attrs = get(entityAttributesFamily(params.entityId));
            const attr = attrs.find(a => a.fieldName === params.fieldName);
            return attr?.value ?? null;
        })
);

// ============================================
// ENTITY ATTRIBUTES - WRITE (Memory Only)
// ============================================

/**
 * Set a single attribute value
 * Stores in memory only - not persisted
 */
export const setAttributeAtom = atom(
    null,
    (get, set, params: {
        entityId: string;
        fieldName: string;
        value: any;
        fieldType?: FieldType;
        schemaId?: string;
        cardId?: string;
    }) => {
        const { entityId, fieldName, value, fieldType = 'text', schemaId, cardId } = params;
        const timestamp = Date.now();

        // Update cache
        const cache = new Map(get(entityAttributesCache));
        const existing = cache.get(entityId) || [];
        const existingIndex = existing.findIndex(a => a.fieldName === fieldName);

        const newAttribute: EntityAttribute = {
            id: existingIndex >= 0 ? existing[existingIndex].id : generateId(),
            entityId,
            fieldName,
            fieldType,
            value,
            schemaId,
            cardId,
            createdAt: existingIndex >= 0 ? existing[existingIndex].createdAt : timestamp,
            updatedAt: timestamp,
        };

        const updatedAttrs = [...existing];
        if (existingIndex >= 0) {
            updatedAttrs[existingIndex] = newAttribute;
        } else {
            updatedAttrs.push(newAttribute);
        }
        cache.set(entityId, updatedAttrs);
        set(entityAttributesCache, cache);

        console.log(`[EntityAttributes] Set ${fieldName} = ${JSON.stringify(value).slice(0, 50)} (memory only)`);
    }
);

/**
 * Set multiple attributes at once
 */
export const setMultipleAttributesAtom = atom(
    null,
    (get, set, params: {
        entityId: string;
        attributes: Record<string, any>;
        fieldTypes?: Record<string, FieldType>;
    }) => {
        const { entityId, attributes, fieldTypes = {} } = params;

        for (const [fieldName, value] of Object.entries(attributes)) {
            set(setAttributeAtom, {
                entityId,
                fieldName,
                value,
                fieldType: fieldTypes[fieldName] || 'text',
            });
        }
    }
);

/**
 * Delete an attribute
 */
export const deleteAttributeAtom = atom(
    null,
    (get, set, params: { entityId: string; fieldName: string }) => {
        const { entityId, fieldName } = params;

        const cache = new Map(get(entityAttributesCache));
        const existing = cache.get(entityId) || [];
        const filtered = existing.filter(a => a.fieldName !== fieldName);
        cache.set(entityId, filtered);
        set(entityAttributesCache, cache);

        console.log(`[EntityAttributes] Deleted ${fieldName} (memory only)`);
    }
);

// ============================================
// META CARDS - READ
// ============================================

/**
 * Get all meta cards for an entity
 * Returns from in-memory cache only
 */
export const metaCardsFamily = atomFamily((entityId: string) =>
    atom((get) => {
        const cache = get(metaCardsCache);
        return cache.get(entityId) || [];
    })
);

// ============================================
// META CARDS - WRITE (Memory Only)
// ============================================

/**
 * Create a new meta card
 */
export const createMetaCardAtom = atom(
    null,
    (get, set, params: {
        ownerId: string;
        name: string;
        color?: string;
        icon?: string;
    }) => {
        const { ownerId, name, color, icon } = params;
        const timestamp = Date.now();
        const id = generateId();

        const cache = new Map(get(metaCardsCache));
        const existingCards = cache.get(ownerId) || [];
        const displayOrder = existingCards.length;

        const newCard: MetaCard = {
            id,
            ownerId,
            name,
            color,
            icon,
            displayOrder,
            isCollapsed: false,
            createdAt: timestamp,
            updatedAt: timestamp,
        };

        cache.set(ownerId, [...existingCards, newCard]);
        set(metaCardsCache, cache);

        console.log(`[EntityAttributes] Created meta card ${name} (memory only)`);
        return newCard;
    }
);

/**
 * Update a meta card
 */
export const updateMetaCardAtom = atom(
    null,
    (get, set, params: {
        cardId: string;
        updates: Partial<Pick<MetaCard, 'name' | 'color' | 'icon' | 'displayOrder' | 'isCollapsed'>>;
    }) => {
        const { cardId, updates } = params;
        const timestamp = Date.now();

        const cache = new Map(get(metaCardsCache));

        for (const [ownerId, cards] of cache.entries()) {
            const idx = cards.findIndex(c => c.id === cardId);
            if (idx >= 0) {
                const updatedCards = [...cards];
                updatedCards[idx] = { ...updatedCards[idx], ...updates, updatedAt: timestamp };
                cache.set(ownerId, updatedCards);
                set(metaCardsCache, cache);
                console.log(`[EntityAttributes] Updated meta card ${cardId} (memory only)`);
                return;
            }
        }

        console.warn(`[EntityAttributes] Card ${cardId} not found`);
    }
);

/**
 * Delete a meta card
 */
export const deleteMetaCardAtom = atom(
    null,
    (get, set, cardId: string) => {
        const cache = new Map(get(metaCardsCache));

        for (const [ownerId, cards] of cache.entries()) {
            if (cards.some(c => c.id === cardId)) {
                cache.set(ownerId, cards.filter(c => c.id !== cardId));
                set(metaCardsCache, cache);
                console.log(`[EntityAttributes] Deleted meta card ${cardId} (memory only)`);
                return;
            }
        }

        console.warn(`[EntityAttributes] Card ${cardId} not found`);
    }
);

// ============================================
// CACHE MANAGEMENT
// ============================================

/**
 * Invalidate cache for an entity
 */
export const invalidateEntityCacheAtom = atom(
    null,
    (get, set, entityId: string) => {
        const attrCache = new Map(get(entityAttributesCache));
        attrCache.delete(entityId);
        set(entityAttributesCache, attrCache);

        const cardCache = new Map(get(metaCardsCache));
        cardCache.delete(entityId);
        set(metaCardsCache, cardCache);
    }
);

/**
 * Clear all caches
 */
export const clearAllCachesAtom = atom(
    null,
    (_get, set) => {
        set(entityAttributesCache, new Map());
        set(metaCardsCache, new Map());
        set(fieldSchemasAtom, []);
    }
);

// ============================================
// CONVENIENCE: GET ALL ATTRIBUTES AS RECORD
// ============================================

/**
 * Get all attributes as a key-value record (for compatibility)
 */
export const entityAttributesRecordFamily = atomFamily((entityId: string) =>
    atom((get) => {
        const attrs = get(entityAttributesFamily(entityId));
        const record: Record<string, any> = {};

        for (const attr of attrs) {
            record[attr.fieldName] = attr.value;
        }

        return record;
    })
);
