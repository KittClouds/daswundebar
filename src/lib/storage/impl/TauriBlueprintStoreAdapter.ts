/**
 * Tauri Blueprint Store Adapter
 * 
 * Implements IBlueprintStore interface using Tauri commands.
 * Routes all Blueprint Hub storage through native Rust CozoDB.
 */

import type { IBlueprintStore } from '../interfaces';
import type {
    BlueprintMeta,
    BlueprintVersion,
    EntityTypeDef,
    FieldDef,
    RelationshipTypeDef,
    RelationshipAttributeDef,
    ViewTemplateDef,
    MOCDef,
    CreateBlueprintMetaInput,
    CreateVersionInput,
    CreateEntityTypeInput,
    CreateFieldInput,
    CreateRelationshipTypeInput,
    CreateRelationshipAttributeInput,
    CreateViewTemplateInput,
    CreateMOCInput,
    VersionStatus,
} from '@/features/blueprint-hub/types';
import {
    blueprintInit,
    blueprintCreate,
    blueprintGet,
    blueprintList,
    blueprintUpdate,
    blueprintDelete,
    blueprintVersionCreate,
    blueprintVersionList,
    blueprintVersionDelete,
    blueprintEntityTypeCreate,
    blueprintEntityTypeList,
    blueprintEntityTypeDelete,
    blueprintFieldCreate,
    blueprintFieldList,
    blueprintFieldDelete,
    blueprintRelationshipTypeCreate,
    blueprintRelationshipTypeList,
    blueprintRelationshipTypeDelete,
} from '@/lib/tauri/blueprints';

/**
 * Tauri-backed implementation of IBlueprintStore
 * Uses native Rust CozoDB for persistence
 */
export class TauriBlueprintStoreAdapter implements IBlueprintStore {
    private initialized = false;

    // =========================================================================
    // Initialization
    // =========================================================================

    async initialize(): Promise<void> {
        if (this.initialized) return;
        try {
            await blueprintInit();
            this.initialized = true;
            console.log('[TauriBlueprintStore] Initialized');
        } catch (error) {
            console.error('[TauriBlueprintStore] Init failed:', error);
            throw error;
        }
    }

    // =========================================================================
    // Blueprint Meta
    // =========================================================================

    async getBlueprintMetaById(id: string): Promise<BlueprintMeta | null> {
        await this.ensureInit();
        const result = await blueprintGet(id);
        return result ? this.mapBlueprintMeta(result) : null;
    }

    async createBlueprintMeta(input: CreateBlueprintMetaInput): Promise<BlueprintMeta> {
        await this.ensureInit();
        const result = await blueprintCreate({
            name: input.name,
            description: input.description,
            category: input.category,
            author: input.author,
            tags: input.tags,
            is_system: input.is_system,
        });
        return this.mapBlueprintMeta(result);
    }

    async updateBlueprintMeta(id: string, updates: Partial<CreateBlueprintMetaInput>): Promise<BlueprintMeta> {
        await this.ensureInit();
        const existing = await this.getBlueprintMetaById(id);
        if (!existing) throw new Error(`Blueprint not found: ${id}`);

        const result = await blueprintUpdate(id, {
            name: updates.name ?? existing.name,
            description: updates.description ?? existing.description,
            category: updates.category ?? existing.category,
            author: updates.author ?? existing.author,
            tags: updates.tags ?? existing.tags,
            is_system: updates.is_system ?? existing.is_system,
        });
        return this.mapBlueprintMeta(result);
    }

    async getAllBlueprintMetas(): Promise<BlueprintMeta[]> {
        await this.ensureInit();
        const results = await blueprintList();
        return results.map(r => this.mapBlueprintMeta(r));
    }

    async deleteBlueprintMeta(id: string): Promise<void> {
        await this.ensureInit();
        await blueprintDelete(id);
    }

    // =========================================================================
    // Versions
    // =========================================================================

    async getVersionById(id: string): Promise<BlueprintVersion | null> {
        await this.ensureInit();
        // We need to search all blueprints for this version
        const metas = await blueprintList();
        for (const meta of metas) {
            const versions = await blueprintVersionList(meta.blueprint_id);
            const found = versions.find(v => v.version_id === id);
            if (found) return this.mapVersion(found);
        }
        return null;
    }

    async getVersionsByBlueprintId(blueprintId: string): Promise<BlueprintVersion[]> {
        await this.ensureInit();
        const results = await blueprintVersionList(blueprintId);
        return results.map(r => this.mapVersion(r));
    }

    async createVersion(input: CreateVersionInput): Promise<BlueprintVersion> {
        await this.ensureInit();
        // Get next version number
        const existing = await blueprintVersionList(input.blueprint_id);
        const nextNum = existing.length > 0
            ? Math.max(...existing.map(v => v.version_number)) + 1
            : 1;

        const result = await blueprintVersionCreate({
            blueprint_id: input.blueprint_id,
            version_number: nextNum,
            status: input.status ?? 'draft',
            change_summary: input.change_summary,
        });
        return this.mapVersion(result);
    }

    async updateVersionStatus(id: string, status: VersionStatus): Promise<void> {
        await this.ensureInit();
        // For now, we don't have a direct update - we'd need to add one
        // This is a limitation we can address later
        console.warn('[TauriBlueprintStore] updateVersionStatus not fully implemented yet');
    }

    async deleteVersion(id: string): Promise<void> {
        await this.ensureInit();
        await blueprintVersionDelete(id);
    }

    // =========================================================================
    // Entity Types
    // =========================================================================

    async getEntityTypeById(id: string): Promise<EntityTypeDef | null> {
        await this.ensureInit();
        // Would need to search all versions
        // For now, return null - we can optimize later
        return null;
    }

    async getEntityTypesByVersionId(versionId: string): Promise<EntityTypeDef[]> {
        await this.ensureInit();
        const results = await blueprintEntityTypeList(versionId);
        return results.map(r => this.mapEntityType(r));
    }

    async createEntityType(input: CreateEntityTypeInput): Promise<EntityTypeDef> {
        await this.ensureInit();
        const result = await blueprintEntityTypeCreate({
            version_id: input.version_id,
            entity_kind: input.entity_kind,
            entity_subtype: input.entity_subtype,
            display_name: input.display_name,
            description: input.description,
            icon: input.icon,
            color: input.color,
            is_abstract: input.is_abstract,
            parent_type_id: input.parent_type_id,
        });
        return this.mapEntityType(result);
    }

    async updateEntityType(id: string, updates: Partial<CreateEntityTypeInput>): Promise<EntityTypeDef> {
        await this.ensureInit();
        // Would need to add an update command
        throw new Error('updateEntityType not implemented yet');
    }

    async deleteEntityType(id: string): Promise<void> {
        await this.ensureInit();
        // First delete all fields for this entity type
        const fields = await blueprintFieldList(id);
        for (const field of fields) {
            await blueprintFieldDelete(field.field_id);
        }
        await blueprintEntityTypeDelete(id);
    }

    // =========================================================================
    // Fields
    // =========================================================================

    async getFieldById(id: string): Promise<FieldDef | null> {
        await this.ensureInit();
        return null; // Would need to search all entity types
    }

    async getFieldsByEntityTypeId(entityTypeId: string): Promise<FieldDef[]> {
        await this.ensureInit();
        const results = await blueprintFieldList(entityTypeId);
        return results.map(r => this.mapField(r));
    }

    async createField(input: CreateFieldInput): Promise<FieldDef> {
        await this.ensureInit();
        const result = await blueprintFieldCreate({
            entity_type_id: input.entity_type_id,
            field_name: input.field_name,
            display_label: input.display_label,
            data_type: input.data_type,
            is_required: input.is_required,
            is_array: input.is_array,
            default_value: input.default_value,
            validation_rules: input.validation_rules,
            ui_hints: input.ui_hints,
            display_order: input.display_order,
            group_name: input.group_name,
            description: input.description,
        });
        return this.mapField(result);
    }

    async updateField(id: string, updates: Partial<CreateFieldInput>): Promise<FieldDef> {
        await this.ensureInit();
        throw new Error('updateField not implemented yet');
    }

    async deleteField(id: string): Promise<void> {
        await this.ensureInit();
        await blueprintFieldDelete(id);
    }

    // =========================================================================
    // Relationship Types
    // =========================================================================

    async getRelationshipTypeById(id: string): Promise<RelationshipTypeDef | null> {
        await this.ensureInit();
        return null; // Would need to search all versions
    }

    async getRelationshipTypesByVersionId(versionId: string): Promise<RelationshipTypeDef[]> {
        await this.ensureInit();
        const results = await blueprintRelationshipTypeList(versionId);
        return results.map(r => this.mapRelationshipType(r));
    }

    async createRelationshipType(input: CreateRelationshipTypeInput): Promise<RelationshipTypeDef> {
        await this.ensureInit();
        const result = await blueprintRelationshipTypeCreate({
            version_id: input.version_id,
            relationship_name: input.relationship_name,
            display_label: input.display_label,
            source_entity_kind: input.source_entity_kind,
            target_entity_kind: input.target_entity_kind,
            direction: input.direction,
            cardinality: input.cardinality,
            is_symmetric: input.is_symmetric,
            inverse_label: input.inverse_label,
            description: input.description,
            verb_patterns: input.verb_patterns,
            confidence: input.confidence,
            pattern_category: input.pattern_category,
        });
        return this.mapRelationshipType(result);
    }

    async updateRelationshipType(id: string, updates: Partial<CreateRelationshipTypeInput>): Promise<RelationshipTypeDef> {
        await this.ensureInit();
        throw new Error('updateRelationshipType not implemented yet');
    }

    async deleteRelationshipType(id: string): Promise<void> {
        await this.ensureInit();
        await blueprintRelationshipTypeDelete(id);
    }

    // =========================================================================
    // Relationship Attributes (Not yet in Rust - stub)
    // =========================================================================

    async getRelationshipAttributeById(id: string): Promise<RelationshipAttributeDef | null> {
        return null;
    }

    async getRelationshipAttributesByTypeId(relationshipTypeId: string): Promise<RelationshipAttributeDef[]> {
        return [];
    }

    async createRelationshipAttribute(input: CreateRelationshipAttributeInput): Promise<RelationshipAttributeDef> {
        throw new Error('RelationshipAttributes not yet implemented in Rust');
    }

    async deleteRelationshipAttribute(id: string): Promise<void> {
        // No-op for now
    }

    // =========================================================================
    // View Templates (Not yet in Rust - stub)
    // =========================================================================

    async getViewTemplateById(id: string): Promise<ViewTemplateDef | null> {
        return null;
    }

    async getViewTemplatesByVersionId(versionId: string): Promise<ViewTemplateDef[]> {
        return [];
    }

    async createViewTemplate(input: CreateViewTemplateInput): Promise<ViewTemplateDef> {
        throw new Error('ViewTemplates not yet implemented in Rust');
    }

    async updateViewTemplate(id: string, updates: Partial<CreateViewTemplateInput>): Promise<ViewTemplateDef> {
        throw new Error('ViewTemplates not yet implemented in Rust');
    }

    async deleteViewTemplate(id: string): Promise<void> {
        // No-op
    }

    // =========================================================================
    // MOCs (Not yet in Rust - stub)
    // =========================================================================

    async getMOCById(id: string): Promise<MOCDef | null> {
        return null;
    }

    async getMOCsByVersionId(versionId: string): Promise<MOCDef[]> {
        return [];
    }

    async createMOC(input: CreateMOCInput): Promise<MOCDef> {
        throw new Error('MOCs not yet implemented in Rust');
    }

    async updateMOC(id: string, updates: Partial<CreateMOCInput>): Promise<MOCDef> {
        throw new Error('MOCs not yet implemented in Rust');
    }

    async deleteMOC(id: string): Promise<void> {
        // No-op
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private async ensureInit(): Promise<void> {
        if (!this.initialized) {
            await this.initialize();
        }
    }

    private mapBlueprintMeta(r: {
        blueprint_id: string;
        name: string;
        description?: string;
        category?: string;
        author?: string;
        tags: string[];
        is_system: boolean;
        created_at: number;
        updated_at: number;
    }): BlueprintMeta {
        return {
            blueprint_id: r.blueprint_id,
            name: r.name,
            description: r.description,
            category: r.category,
            author: r.author,
            tags: r.tags,
            is_system: r.is_system,
            created_at: r.created_at,
            updated_at: r.updated_at,
        };
    }

    private mapVersion(r: {
        version_id: string;
        blueprint_id: string;
        version_number: number;
        status: string;
        change_summary?: string;
        published_at?: number;
        created_at: number;
    }): BlueprintVersion {
        return {
            version_id: r.version_id,
            blueprint_id: r.blueprint_id,
            version_number: r.version_number,
            status: r.status as VersionStatus,
            change_summary: r.change_summary,
            published_at: r.published_at,
            created_at: r.created_at,
        };
    }

    private mapEntityType(r: {
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
    }): EntityTypeDef {
        return {
            entity_type_id: r.entity_type_id,
            version_id: r.version_id,
            entity_kind: r.entity_kind,
            entity_subtype: r.entity_subtype,
            display_name: r.display_name,
            description: r.description,
            icon: r.icon,
            color: r.color,
            is_abstract: r.is_abstract,
            parent_type_id: r.parent_type_id,
            created_at: r.created_at,
        };
    }

    private mapField(r: {
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
    }): FieldDef {
        return {
            field_id: r.field_id,
            entity_type_id: r.entity_type_id,
            field_name: r.field_name,
            display_label: r.display_label,
            data_type: r.data_type as any,
            is_required: r.is_required,
            is_array: r.is_array,
            default_value: r.default_value,
            validation_rules: r.validation_rules,
            ui_hints: r.ui_hints,
            display_order: r.display_order,
            group_name: r.group_name,
            description: r.description,
            created_at: r.created_at,
        };
    }

    private mapRelationshipType(r: {
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
    }): RelationshipTypeDef {
        return {
            relationship_type_id: r.relationship_type_id,
            version_id: r.version_id,
            relationship_name: r.relationship_name,
            display_label: r.display_label,
            source_entity_kind: r.source_entity_kind,
            target_entity_kind: r.target_entity_kind,
            direction: r.direction as any,
            cardinality: r.cardinality as any,
            is_symmetric: r.is_symmetric,
            inverse_label: r.inverse_label,
            description: r.description,
            verb_patterns: r.verb_patterns,
            confidence: r.confidence,
            pattern_category: r.pattern_category,
            created_at: r.created_at,
        };
    }
}

// Singleton
let tauriBlueprintStore: TauriBlueprintStoreAdapter | null = null;

export function getTauriBlueprintStore(): TauriBlueprintStoreAdapter {
    if (!tauriBlueprintStore) {
        tauriBlueprintStore = new TauriBlueprintStoreAdapter();
    }
    return tauriBlueprintStore;
}

export function resetTauriBlueprintStore(): void {
    tauriBlueprintStore = null;
}
