/**
 * Blueprint Hub - Tauri Bridge
 * 
 * TypeScript interface to the Rust Blueprint storage.
 * Provides CRUD operations for blueprint metadata.
 */

import { invoke } from '@tauri-apps/api/core';

// =============================================================================
// Types
// =============================================================================

export interface BlueprintMeta {
    blueprint_id: string;
    name: string;
    description?: string;
    category?: string;
    author?: string;
    tags: string[];
    is_system: boolean;
    created_at: number;
    updated_at: number;
}

export interface CreateBlueprintInput {
    name: string;
    description?: string;
    category?: string;
    author?: string;
    tags?: string[];
    is_system?: boolean;
}

export interface BlueprintVersion {
    version_id: string;
    blueprint_id: string;
    version_number: number;
    status: string;
    change_summary?: string;
    published_at?: number;
    created_at: number;
}

export interface CreateVersionInput {
    blueprint_id: string;
    version_number: number;
    status: string;
    change_summary?: string;
}

export interface EntityTypeDef {
    entity_type_id: string;
    version_id: string;
    entity_kind: string;
    entity_subtype?: string;
    display_name: string;
    description?: string;
    icon?: string;
    color?: string;
    is_abstract: boolean;
    parent_type_id?: string;
    created_at: number;
}

export interface CreateEntityTypeInput {
    version_id: string;
    entity_kind: string;
    entity_subtype?: string;
    display_name: string;
    description?: string;
    icon?: string;
    color?: string;
    is_abstract?: boolean;
    parent_type_id?: string;
}

export interface FieldDef {
    field_id: string;
    entity_type_id: string;
    field_name: string;
    display_label: string;
    data_type: string;
    is_required: boolean;
    is_array: boolean;
    default_value?: string;
    validation_rules?: Record<string, unknown>;
    ui_hints?: Record<string, unknown>;
    display_order: number;
    group_name?: string;
    description?: string;
    created_at: number;
}

export interface CreateFieldInput {
    entity_type_id: string;
    field_name: string;
    display_label: string;
    data_type: string;
    is_required?: boolean;
    is_array?: boolean;
    default_value?: string;
    validation_rules?: Record<string, unknown>;
    ui_hints?: Record<string, unknown>;
    display_order?: number;
    group_name?: string;
    description?: string;
}

export interface RelationshipTypeDef {
    relationship_type_id: string;
    version_id: string;
    relationship_name: string;
    display_label: string;
    source_entity_kind: string;
    target_entity_kind: string;
    direction: string;
    cardinality: string;
    is_symmetric: boolean;
    inverse_label?: string;
    description?: string;
    verb_patterns?: string[];
    confidence?: number;
    pattern_category?: string;
    created_at: number;
}

export interface CreateRelationshipTypeInput {
    version_id: string;
    relationship_name: string;
    display_label: string;
    source_entity_kind: string;
    target_entity_kind: string;
    direction?: string;
    cardinality?: string;
    is_symmetric?: boolean;
    inverse_label?: string;
    description?: string;
    verb_patterns?: string[];
    confidence?: number;
    pattern_category?: string;
}

// =============================================================================
// Tauri Commands - Blueprint Meta
// =============================================================================

/**
 * Initialize the blueprint schema (called during app startup)
 */
export async function blueprintInit(): Promise<void> {
    return invoke('blueprint_init');
}

/**
 * Create a new blueprint
 */
export async function blueprintCreate(input: CreateBlueprintInput): Promise<BlueprintMeta> {
    return invoke('blueprint_create', { input });
}

/**
 * Get a blueprint by ID
 */
export async function blueprintGet(blueprintId: string): Promise<BlueprintMeta | null> {
    return invoke('blueprint_get', { blueprintId });
}

/**
 * List all blueprints
 */
export async function blueprintList(): Promise<BlueprintMeta[]> {
    return invoke('blueprint_list');
}

/**
 * Update a blueprint
 */
export async function blueprintUpdate(
    blueprintId: string,
    updates: CreateBlueprintInput
): Promise<BlueprintMeta> {
    return invoke('blueprint_update', { blueprintId, updates });
}

/**
 * Delete a blueprint
 */
export async function blueprintDelete(blueprintId: string): Promise<void> {
    return invoke('blueprint_delete', { blueprintId });
}

// =============================================================================
// Tauri Commands - Version
// =============================================================================

export async function blueprintVersionCreate(input: CreateVersionInput): Promise<BlueprintVersion> {
    return invoke('blueprint_version_create', { input });
}

export async function blueprintVersionList(blueprintId: string): Promise<BlueprintVersion[]> {
    return invoke('blueprint_version_list', { blueprintId });
}

export async function blueprintVersionDelete(versionId: string): Promise<void> {
    return invoke('blueprint_version_delete', { versionId });
}

// =============================================================================
// Tauri Commands - EntityType
// =============================================================================

export async function blueprintEntityTypeCreate(input: CreateEntityTypeInput): Promise<EntityTypeDef> {
    return invoke('blueprint_entity_type_create', { input });
}

export async function blueprintEntityTypeList(versionId: string): Promise<EntityTypeDef[]> {
    return invoke('blueprint_entity_type_list', { versionId });
}

export async function blueprintEntityTypeDelete(entityTypeId: string): Promise<void> {
    return invoke('blueprint_entity_type_delete', { entityTypeId });
}

// =============================================================================
// Tauri Commands - Field
// =============================================================================

export async function blueprintFieldCreate(input: CreateFieldInput): Promise<FieldDef> {
    return invoke('blueprint_field_create', { input });
}

export async function blueprintFieldList(entityTypeId: string): Promise<FieldDef[]> {
    return invoke('blueprint_field_list', { entityTypeId });
}

export async function blueprintFieldDelete(fieldId: string): Promise<void> {
    return invoke('blueprint_field_delete', { fieldId });
}

// =============================================================================
// Tauri Commands - RelationshipType
// =============================================================================

export async function blueprintRelationshipTypeCreate(input: CreateRelationshipTypeInput): Promise<RelationshipTypeDef> {
    return invoke('blueprint_relationship_type_create', { input });
}

export async function blueprintRelationshipTypeList(versionId: string): Promise<RelationshipTypeDef[]> {
    return invoke('blueprint_relationship_type_list', { versionId });
}

export async function blueprintRelationshipTypeDelete(relationshipTypeId: string): Promise<void> {
    return invoke('blueprint_relationship_type_delete', { relationshipTypeId });
}

// =============================================================================
// Facade Class
// =============================================================================

/**
 * Blueprint Hub Facade - mirrors the IBlueprintStore interface
 */
export class TauriBlueprintFacade {
    private initialized = false;

    async init(): Promise<void> {
        if (this.initialized) return;
        try {
            await blueprintInit();
            this.initialized = true;
            console.log('[TauriBlueprintFacade] Schema initialized');
        } catch (error) {
            console.error('[TauriBlueprintFacade] Init failed:', error);
            throw error;
        }
    }

    async createBlueprintMeta(input: CreateBlueprintInput): Promise<BlueprintMeta> {
        await this.ensureInit();
        return blueprintCreate(input);
    }

    async getBlueprintMetaById(id: string): Promise<BlueprintMeta | null> {
        await this.ensureInit();
        return blueprintGet(id);
    }

    async getAllBlueprintMetas(): Promise<BlueprintMeta[]> {
        await this.ensureInit();
        return blueprintList();
    }

    async updateBlueprintMeta(id: string, updates: Partial<CreateBlueprintInput>): Promise<BlueprintMeta> {
        await this.ensureInit();
        const existing = await this.getBlueprintMetaById(id);
        if (!existing) {
            throw new Error(`Blueprint not found: ${id}`);
        }
        // Merge updates with existing data
        const merged: CreateBlueprintInput = {
            name: updates.name ?? existing.name,
            description: updates.description ?? existing.description,
            category: updates.category ?? existing.category,
            author: updates.author ?? existing.author,
            tags: updates.tags ?? existing.tags,
            is_system: updates.is_system ?? existing.is_system,
        };
        return blueprintUpdate(id, merged);
    }

    async deleteBlueprintMeta(id: string): Promise<void> {
        await this.ensureInit();
        return blueprintDelete(id);
    }

    private async ensureInit(): Promise<void> {
        if (!this.initialized) {
            await this.init();
        }
    }
}

// Singleton instance
export const tauriBlueprint = new TauriBlueprintFacade();
