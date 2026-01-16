/**
 * FolderSchemaRegistry - Central registry for folder type schemas
 * 
 * Provides:
 * - Schema lookup by entity kind and subtype
 * - Allowed subfolder/note type queries
 * - Custom schema registration for extensibility
 */

import type { EntityKind } from '@/lib/types/entityTypes';
import type { FolderSchema, AllowedSubfolderDefinition, AllowedNoteTypeDefinition } from './schemas';

import {
    CHARACTER_FOLDER_SCHEMA,
    LOCATION_FOLDER_SCHEMA,
    ITEM_FOLDER_SCHEMA,
    EVENT_FOLDER_SCHEMA,
    NARRATIVE_FOLDER_SCHEMA,
    SCENE_FOLDER_SCHEMA,
    CHAPTER_FOLDER_SCHEMA,
    ARC_FOLDER_SCHEMA,
    ACT_FOLDER_SCHEMA,
    BEAT_FOLDER_SCHEMA,
    NPC_FOLDER_SCHEMA,
    CONCEPT_FOLDER_SCHEMA,
    TIMELINE_FOLDER_SCHEMA,
} from './schemas/index';

/**
 * Registry for folder schemas.
 * Allows lookup and validation of folder type configurations.
 */
export class FolderSchemaRegistry {
    private schemas: Map<string, FolderSchema>;
    private initialized: boolean = false;

    constructor() {
        this.schemas = new Map();
    }

    /**
     * Initialize with built-in schemas (lazy initialization)
     */
    private ensureInitialized(): void {
        if (this.initialized) return;

        // Register all built-in schemas
        this.registerSchema(CHARACTER_FOLDER_SCHEMA);
        this.registerSchema(LOCATION_FOLDER_SCHEMA);
        this.registerSchema(ITEM_FOLDER_SCHEMA);
        this.registerSchema(EVENT_FOLDER_SCHEMA);
        this.registerSchema(NARRATIVE_FOLDER_SCHEMA);
        this.registerSchema(SCENE_FOLDER_SCHEMA);
        this.registerSchema(CHAPTER_FOLDER_SCHEMA);
        this.registerSchema(ARC_FOLDER_SCHEMA);
        this.registerSchema(ACT_FOLDER_SCHEMA);
        this.registerSchema(BEAT_FOLDER_SCHEMA);
        this.registerSchema(NPC_FOLDER_SCHEMA);
        this.registerSchema(CONCEPT_FOLDER_SCHEMA);
        this.registerSchema(TIMELINE_FOLDER_SCHEMA);

        this.initialized = true;
    }

    /**
     * Generate a unique key for a schema based on kind and optional subtype
     */
    private makeKey(kind: EntityKind, subtype?: string): string {
        return subtype ? `${kind}::${subtype}` : kind;
    }

    /**
     * Register a folder schema.
     * Can be used to add custom schemas or override built-in ones.
     */
    registerSchema(schema: FolderSchema): void {
        const key = this.makeKey(schema.entityKind, schema.subtype);
        this.schemas.set(key, schema);
    }

    /**
     * Unregister a schema by kind and optional subtype
     */
    unregisterSchema(kind: EntityKind, subtype?: string): boolean {
        this.ensureInitialized();
        const key = this.makeKey(kind, subtype);
        return this.schemas.delete(key);
    }

    /**
     * Get schema for an entity kind and optional subtype.
     * Falls back to kind-only schema if subtype-specific not found.
     */
    getSchema(entityKind: EntityKind, subtype?: string): FolderSchema | undefined {
        this.ensureInitialized();

        // Try exact match first (kind + subtype)
        if (subtype) {
            const exactKey = this.makeKey(entityKind, subtype);
            const exactSchema = this.schemas.get(exactKey);
            if (exactSchema) return exactSchema;
        }

        // Fall back to kind-only schema
        const kindKey = this.makeKey(entityKind);
        return this.schemas.get(kindKey);
    }

    /**
     * Check if a schema exists for the given kind/subtype
     */
    hasSchema(entityKind: EntityKind, subtype?: string): boolean {
        return !!this.getSchema(entityKind, subtype);
    }

    /**
     * Get allowed subfolder definitions for a typed folder
     */
    getAllowedSubfolders(entityKind: EntityKind, subtype?: string): AllowedSubfolderDefinition[] {
        const schema = this.getSchema(entityKind, subtype);
        return schema?.allowedSubfolders || [];
    }

    /**
     * Get allowed note type definitions for a typed folder
     */
    getAllowedNoteTypes(entityKind: EntityKind, subtype?: string): AllowedNoteTypeDefinition[] {
        const schema = this.getSchema(entityKind, subtype);
        return schema?.allowedNoteTypes || [];
    }

    /**
     * Check if a specific subfolder type is allowed under a parent folder
     */
    isSubfolderAllowed(
        parentKind: EntityKind,
        parentSubtype: string | undefined,
        childKind: EntityKind,
        childSubtype?: string
    ): boolean {
        const schema = this.getSchema(parentKind, parentSubtype);

        // No schema = no restrictions (allow all)
        if (!schema) return true;

        // Check if child matches any allowed subfolder definition
        return schema.allowedSubfolders.some(
            sf => sf.entityKind === childKind &&
                (sf.subtype === undefined || sf.subtype === childSubtype)
        );
    }

    /**
     * Get the relationship definition for a parent-child folder pair
     */
    getSubfolderRelationship(
        parentKind: EntityKind,
        parentSubtype: string | undefined,
        childKind: EntityKind,
        childSubtype?: string
    ): AllowedSubfolderDefinition | undefined {
        const allowedSubfolders = this.getAllowedSubfolders(parentKind, parentSubtype);

        return allowedSubfolders.find(
            sf => sf.entityKind === childKind &&
                (sf.subtype === undefined || sf.subtype === childSubtype)
        );
    }

    /**
     * Get all registered schemas
     */
    getAllSchemas(): FolderSchema[] {
        this.ensureInitialized();
        return Array.from(this.schemas.values());
    }

    /**
     * Get schemas for a specific entity kind (all subtypes)
     */
    getSchemasForKind(kind: EntityKind): FolderSchema[] {
        this.ensureInitialized();
        return Array.from(this.schemas.values()).filter(s => s.entityKind === kind);
    }

    /**
     * Get default color for an entity kind from schema
     */
    getDefaultColor(entityKind: EntityKind, subtype?: string): string | undefined {
        const schema = this.getSchema(entityKind, subtype);
        return schema?.color;
    }

    /**
     * Get default icon for an entity kind from schema
     */
    getDefaultIcon(entityKind: EntityKind, subtype?: string): string | undefined {
        const schema = this.getSchema(entityKind, subtype);
        return schema?.icon;
    }

    /**
     * Get custom attributes defined for an entity kind
     */
    getCustomAttributes(entityKind: EntityKind, subtype?: string) {
        const schema = this.getSchema(entityKind, subtype);
        return schema?.customAttributes || [];
    }

    /**
     * Check if a folder should propagate its kind to children
     */
    shouldPropagateToChildren(entityKind: EntityKind, subtype?: string): boolean {
        const schema = this.getSchema(entityKind, subtype);
        return schema?.propagateKindToChildren ?? false;
    }

    /**
     * Check if a folder is container-only (no notes allowed)
     */
    isContainerOnly(entityKind: EntityKind, subtype?: string): boolean {
        const schema = this.getSchema(entityKind, subtype);
        return schema?.containerOnly ?? false;
    }

    /**
     * Get network auto-creation configuration for a subfolder type.
     * Returns undefined if the subfolder doesn't trigger network creation.
     */
    getNetworkCreationConfig(
        parentKind: EntityKind,
        parentSubtype: string | undefined,
        childKind: EntityKind,
        childSubtype?: string
    ): { autoCreate: boolean; schemaId: string; threshold: number } | undefined {
        const subfolder = this.getSubfolderRelationship(
            parentKind,
            parentSubtype,
            childKind,
            childSubtype
        );

        if (!subfolder?.autoCreateNetwork || !subfolder.networkSchemaId) {
            return undefined;
        }

        return {
            autoCreate: true,
            schemaId: subfolder.networkSchemaId,
            threshold: subfolder.networkCreationThreshold ?? 2,
        };
    }

    /**
     * Get all subfolder definitions that trigger network auto-creation
     */
    getNetworkTriggerSubfolders(entityKind: EntityKind, subtype?: string): AllowedSubfolderDefinition[] {
        const subfolders = this.getAllowedSubfolders(entityKind, subtype);
        return subfolders.filter(sf => sf.autoCreateNetwork && sf.networkSchemaId);
    }
}

// Singleton instance
export const folderSchemaRegistry = new FolderSchemaRegistry();
