/**
 * Type-Aware Name Utilities
 * 
 * Parses and validates entity type patterns like:
 * - [CHARACTER|Hero] → entityKind: CHARACTER, label: Hero
 * - [LOCATION:CITY|New York] → entityKind: LOCATION, entitySubtype: CITY, label: New York
 * - Plain text → no type
 */

import type { EntityKind } from '@/lib/types/entityTypes';

export interface ParsedName {
    rawName: string;
    entityKind: EntityKind | null;
    entitySubtype: string | null;
    label: string;
    isTyped: boolean;
}

/**
 * Regex to match typed name patterns:
 * [TYPE|Label] or [TYPE:SUBTYPE|Label]
 */
const TYPED_NAME_REGEX = /^\[([A-Z_]+)(?::([A-Z_]+))?\|(.+)\]$/;

/**
 * Parse a name string to extract type information
 */
export function parseTypedName(name: string): ParsedName {
    const trimmed = name.trim();
    const match = trimmed.match(TYPED_NAME_REGEX);

    if (match) {
        const [, kind, subtype, label] = match;
        return {
            rawName: trimmed,
            entityKind: kind as EntityKind,
            entitySubtype: subtype || null,
            label: label.trim(),
            isTyped: true,
        };
    }

    // Not a typed pattern - entire string is the label
    return {
        rawName: trimmed,
        entityKind: null,
        entitySubtype: null,
        label: trimmed,
        isTyped: false,
    };
}

/**
 * Build a typed name string from components
 */
export function buildTypedName(
    label: string,
    entityKind?: EntityKind | null,
    entitySubtype?: string | null
): string {
    if (!entityKind) {
        return label;
    }

    if (entitySubtype) {
        return `[${entityKind}:${entitySubtype}|${label}]`;
    }

    return `[${entityKind}|${label}]`;
}

/**
 * Determine what can be edited in a name based on context
 */
export interface RenameContext {
    currentName: string;
    entityKind?: EntityKind | null;
    inheritedKind?: EntityKind | null;
    isTypedRoot?: boolean;
    hasChildren?: boolean;
}

export interface RenameRules {
    canEditType: boolean;
    canEditLabel: boolean;
    lockedPrefix: string | null;
    editableValue: string;
}

export function getRenameRules(context: RenameContext): RenameRules {
    const parsed = parseTypedName(context.currentName);

    // Case 1: Item inherits type from parent folder
    if (context.inheritedKind && !context.isTypedRoot) {
        return {
            canEditType: false,
            canEditLabel: true,
            lockedPrefix: `[${context.inheritedKind}|`,
            editableValue: parsed.label,
        };
    }

    // Case 2: Typed root with children - type is locked
    if (context.isTypedRoot && context.hasChildren) {
        return {
            canEditType: false,
            canEditLabel: true,
            lockedPrefix: context.entityKind
                ? `[${context.entityKind}|`
                : null,
            editableValue: parsed.label,
        };
    }

    // Case 3: Global item or typed root without children - fully editable
    return {
        canEditType: true,
        canEditLabel: true,
        lockedPrefix: null,
        editableValue: context.currentName,
    };
}

/**
 * Apply rename with type-awareness
 * Returns the new name and any type updates to apply
 */
export interface RenameResult {
    newName: string;
    newEntityKind?: EntityKind | null;
    newEntitySubtype?: string | null;
    shouldUpdateType: boolean;
}

export function applyTypeAwareRename(
    newValue: string,
    context: RenameContext
): RenameResult {
    const parsed = parseTypedName(newValue);

    // Case 1: Type locked (inherited or root with children)
    if (context.inheritedKind && !context.isTypedRoot) {
        // Build name with inherited type
        return {
            newName: buildTypedName(parsed.label, context.inheritedKind),
            shouldUpdateType: false,
        };
    }

    // Case 2: User typed a type pattern - convert to typed
    if (parsed.isTyped) {
        return {
            newName: parsed.rawName,
            newEntityKind: parsed.entityKind,
            newEntitySubtype: parsed.entitySubtype,
            shouldUpdateType: true,
        };
    }

    // Case 3: Plain label - no type change
    return {
        newName: parsed.label,
        shouldUpdateType: false,
    };
}
