/**
 * Content API - Unified CozoDB content layer
 * 
 * Single source of truth for all content operations:
 * - Notes, Folders
 * - Networks, Entities, Relationships
 * - Calendar Events, Periods
 * - Field Bindings
 * 
 * Replaces: notes-api.ts, network-bridge.ts, calendar-bridge.ts, 
 *           calendar-api.ts, binding-bridge.ts, surreal-bridge.ts
 */

import { invoke } from '@tauri-apps/api/core';
import { isTauri } from '@/lib/tauri';

// =============================================================================
// CONFIG
// =============================================================================

const DEFAULT_WORLD = 'default';

// =============================================================================
// TYPES - Notes & Folders
// =============================================================================

export interface CozoNote {
    id: string;
    world_id: string;
    title: string;
    content: string;
    folder_id: string | null;
    entity_kind: string | null;
    entity_subtype: string | null;
    is_entity: boolean;
    is_pinned: boolean;
    favorite: boolean;
    created_at: number;
    updated_at: number;
}

export interface CozoFolder {
    id: string;
    world_id: string;
    name: string;
    parent_id: string | null;
    entity_kind: string | null;
    entity_subtype: string | null;
    color: string | null;
    is_typed_root: boolean;
    collapsed: boolean;
    fantasy_year: number | null;
    fantasy_month: number | null;
    fantasy_day: number | null;
    created_at: number;
    updated_at: number;
}

export interface CozoFolderTreeNode {
    folder: CozoFolder;
    children: CozoFolderTreeNode[];
    notes: { id: string; title: string; is_pinned: boolean; favorite: boolean; entity_kind: string | null; updated_at: number }[];
}

// =============================================================================
// TYPES - Networks, Entities, Relationships
// =============================================================================

export interface CozoNetwork {
    id: string;
    world_id: string;
    name: string;
    schema_id: string;
    root_folder_id: string | null;
    root_entity_id: string | null;
    namespace: string;
    description: string | null;
    tags: string[];
    member_count: number;
    relationship_count: number;
    max_depth: number;
    created_at: number;
    updated_at: number;
}

export interface CozoEntity {
    id: string;
    world_id: string;
    label: string;
    entity_kind: string;
    entity_subtype: string | null;
    note_id: string | null;
    folder_id: string | null;
    is_active: boolean;
    aliases: string[];
    attributes: Record<string, unknown>;
    created_at: number;
    updated_at: number;
}

export interface CozoRelationship {
    id: string;
    world_id: string;
    source_id: string;
    target_id: string;
    network_id: string | null;
    relationship_code: string;
    inverse_id: string | null;
    strength: number;
    start_date: number | null;
    end_date: number | null;
    notes: string | null;
    attributes: Record<string, unknown>;
    created_at: number;
}

// =============================================================================
// TYPES - Calendar Events & Periods
// =============================================================================

export interface CozoCalEvent {
    id: string;
    world_id: string;
    calendar_id: string;
    title: string;
    description: string | null;
    date_year: number;
    date_month: number;
    date_day: number;
    date_hour: number | null;
    date_minute: number | null;
    era_id: string | null;
    end_year: number | null;
    end_month: number | null;
    end_day: number | null;
    is_all_day: boolean;
    recurrence: unknown | null;
    parent_event_id: string | null;
    importance: string;
    category: string;
    tags: string[];
    color: string | null;
    icon: string | null;
    entity_id: string | null;
    entity_kind: string | null;
    source_note_id: string | null;
    created_at: number;
    updated_at: number;
}

export interface CozoPeriod {
    id: string;
    world_id: string;
    calendar_id: string;
    name: string;
    description: string | null;
    start_year: number;
    start_month: number | null;
    end_year: number | null;
    end_month: number | null;
    parent_period_id: string | null;
    period_type: string;
    color: string;
    icon: string | null;
    abbreviation: string | null;
    direction: string;
    triggered_by: string | null;
    ends_when: string | null;
    major_events: string[];
    arc_type: string | null;
    dominant_theme: string | null;
    protagonist_id: string | null;
    antagonist_id: string | null;
    summary: string | null;
    detailed_notes: string | null;
    show_on_timeline: boolean;
    timeline_color: string | null;
    timeline_icon: string | null;
    created_at: number;
    updated_at: number;
}

// =============================================================================
// TYPES - Field Bindings
// =============================================================================

export type BindingType = 'mirror' | 'inherit' | 'aggregate';
export type AggregationFunction = 'sum' | 'avg' | 'min' | 'max' | 'count' | 'concat' | 'first' | 'last';

export interface CozoFieldBinding {
    id: string;
    world_id: string;
    source_entity_id: string;
    source_field_name: string;
    target_entity_id: string;
    target_field_name: string;
    binding_type: BindingType;
    transform: unknown | null;
    aggregation_fn: AggregationFunction | null;
    aggregation_filter: unknown | null;
    allow_override: boolean;
    is_active: boolean;
    created_at: number;
    updated_at: number;
}

// =============================================================================
// API CLASS
// =============================================================================

class ContentAPI {
    private worldId = DEFAULT_WORLD;

    setWorld(worldId: string) {
        this.worldId = worldId;
    }

    // =========================================================================
    // NOTES
    // =========================================================================

    async createNote(params: {
        title?: string;
        content?: string;
        folderId?: string;
        entityKind?: string;
        entitySubtype?: string;
        isEntity?: boolean;
    }): Promise<CozoNote> {
        return invoke<CozoNote>('cozo_create_note', {
            worldId: this.worldId,
            title: params.title || 'Untitled Note',
            content: params.content || '',
            folderId: params.folderId,
            entityKind: params.entityKind,
            entitySubtype: params.entitySubtype,
            isEntity: params.isEntity,
        });
    }

    async getNote(id: string): Promise<CozoNote | null> {
        return invoke<CozoNote | null>('cozo_get_note', { worldId: this.worldId, id });
    }

    async listNotes(): Promise<CozoNote[]> {
        return invoke<CozoNote[]>('cozo_list_notes', { worldId: this.worldId });
    }

    async updateNote(id: string, updates: {
        title?: string;
        content?: string;
        folderId?: string;
        entityKind?: string;
        entitySubtype?: string;
        isEntity?: boolean;
        isPinned?: boolean;
        favorite?: boolean;
    }): Promise<CozoNote> {
        return invoke<CozoNote>('cozo_update_note', {
            worldId: this.worldId,
            id,
            ...updates,
        });
    }

    async deleteNote(id: string): Promise<boolean> {
        return invoke<boolean>('cozo_delete_note', { worldId: this.worldId, id });
    }

    // =========================================================================
    // FOLDERS
    // =========================================================================

    async createFolder(params: {
        name: string;
        parentId?: string;
        entityKind?: string;
        entitySubtype?: string;
        color?: string;
        isTypedRoot?: boolean;
    }): Promise<CozoFolder> {
        return invoke<CozoFolder>('cozo_create_folder', {
            worldId: this.worldId,
            ...params,
        });
    }

    async getFolder(id: string): Promise<CozoFolder | null> {
        return invoke<CozoFolder | null>('cozo_get_folder', { worldId: this.worldId, id });
    }

    async listFolders(): Promise<CozoFolder[]> {
        return invoke<CozoFolder[]>('cozo_list_folders', { worldId: this.worldId });
    }

    async getFolderTree(): Promise<CozoFolderTreeNode[]> {
        return invoke<CozoFolderTreeNode[]>('cozo_get_folder_tree', { worldId: this.worldId });
    }

    async updateFolder(id: string, updates: {
        name?: string;
        parentId?: string;
        entityKind?: string;
        entitySubtype?: string;
        color?: string;
        collapsed?: boolean;
    }): Promise<CozoFolder> {
        return invoke<CozoFolder>('cozo_update_folder', {
            worldId: this.worldId,
            id,
            ...updates,
        });
    }

    async deleteFolder(id: string): Promise<boolean> {
        return invoke<boolean>('cozo_delete_folder', { worldId: this.worldId, id });
    }

    // =========================================================================
    // NETWORKS
    // =========================================================================

    async createNetwork(params: {
        name: string;
        schemaId: string;
        rootFolderId?: string;
        rootEntityId?: string;
        namespace?: string;
        description?: string;
        tags?: string[];
    }): Promise<CozoNetwork> {
        return invoke<CozoNetwork>('cozo_create_network', {
            worldId: this.worldId,
            ...params,
        });
    }

    async getNetwork(id: string): Promise<CozoNetwork | null> {
        return invoke<CozoNetwork | null>('cozo_get_network', { worldId: this.worldId, id });
    }

    async listNetworks(): Promise<CozoNetwork[]> {
        return invoke<CozoNetwork[]>('cozo_list_networks', { worldId: this.worldId });
    }

    async deleteNetwork(id: string): Promise<boolean> {
        return invoke<boolean>('cozo_delete_network', { worldId: this.worldId, id });
    }

    // =========================================================================
    // ENTITIES
    // =========================================================================

    async createEntity(params: {
        label: string;
        entityKind: string;
        entitySubtype?: string;
        noteId?: string;
        folderId?: string;
        aliases?: string[];
        attributes?: Record<string, unknown>;
    }): Promise<CozoEntity> {
        return invoke<CozoEntity>('cozo_create_entity', {
            worldId: this.worldId,
            ...params,
        });
    }

    async getEntity(id: string): Promise<CozoEntity | null> {
        return invoke<CozoEntity | null>('cozo_get_entity', { worldId: this.worldId, id });
    }

    async listEntities(): Promise<CozoEntity[]> {
        return invoke<CozoEntity[]>('cozo_list_entities', { worldId: this.worldId });
    }

    async listEntitiesByKind(kind: string): Promise<CozoEntity[]> {
        return invoke<CozoEntity[]>('cozo_list_entities_by_kind', { worldId: this.worldId, kind });
    }

    async deleteEntity(id: string): Promise<boolean> {
        return invoke<boolean>('cozo_delete_entity', { worldId: this.worldId, id });
    }

    // =========================================================================
    // RELATIONSHIPS
    // =========================================================================

    async createRelationship(params: {
        sourceId: string;
        targetId: string;
        relationshipCode: string;
        networkId?: string;
        strength?: number;
        startDate?: number;
        endDate?: number;
        notes?: string;
        attributes?: Record<string, unknown>;
    }): Promise<CozoRelationship> {
        return invoke<CozoRelationship>('cozo_create_relationship', {
            worldId: this.worldId,
            ...params,
        });
    }

    async getRelationship(id: string): Promise<CozoRelationship | null> {
        return invoke<CozoRelationship | null>('cozo_get_relationship', { worldId: this.worldId, id });
    }

    async getEntityRelationships(entityId: string): Promise<CozoRelationship[]> {
        return invoke<CozoRelationship[]>('cozo_get_entity_relationships', { worldId: this.worldId, entityId });
    }

    async deleteRelationship(id: string): Promise<boolean> {
        return invoke<boolean>('cozo_delete_relationship', { worldId: this.worldId, id });
    }

    // =========================================================================
    // CALENDAR EVENTS
    // =========================================================================

    async createCalEvent(params: {
        calendarId: string;
        title: string;
        dateYear: number;
        dateMonth: number;
        dateDay: number;
        description?: string;
        dateHour?: number;
        dateMinute?: number;
        eraId?: string;
        endYear?: number;
        endMonth?: number;
        endDay?: number;
        isAllDay?: boolean;
        importance?: string;
        category?: string;
        tags?: string[];
        color?: string;
        icon?: string;
        entityId?: string;
        entityKind?: string;
        sourceNoteId?: string;
    }): Promise<CozoCalEvent> {
        return invoke<CozoCalEvent>('cozo_create_cal_event', {
            worldId: this.worldId,
            ...params,
        });
    }

    async getCalEvent(id: string): Promise<CozoCalEvent | null> {
        return invoke<CozoCalEvent | null>('cozo_get_cal_event', { worldId: this.worldId, id });
    }

    async listCalEvents(): Promise<CozoCalEvent[]> {
        return invoke<CozoCalEvent[]>('cozo_list_cal_events', { worldId: this.worldId });
    }

    async listCalEventsByMonth(year: number, month: number): Promise<CozoCalEvent[]> {
        return invoke<CozoCalEvent[]>('cozo_list_cal_events_by_month', { worldId: this.worldId, year, month });
    }

    async deleteCalEvent(id: string): Promise<boolean> {
        return invoke<boolean>('cozo_delete_cal_event', { worldId: this.worldId, id });
    }

    // =========================================================================
    // PERIODS
    // =========================================================================

    async createPeriod(params: {
        calendarId: string;
        name: string;
        startYear: number;
        color: string;
        description?: string;
        startMonth?: number;
        endYear?: number;
        endMonth?: number;
        parentPeriodId?: string;
        periodType?: string;
        icon?: string;
        abbreviation?: string;
        direction?: string;
        triggeredBy?: string;
        endsWhen?: string;
        arcType?: string;
        dominantTheme?: string;
        protagonistId?: string;
        antagonistId?: string;
        summary?: string;
        detailedNotes?: string;
        showOnTimeline?: boolean;
        timelineColor?: string;
        timelineIcon?: string;
    }): Promise<CozoPeriod> {
        return invoke<CozoPeriod>('cozo_create_period', {
            worldId: this.worldId,
            ...params,
        });
    }

    async getPeriod(id: string): Promise<CozoPeriod | null> {
        return invoke<CozoPeriod | null>('cozo_get_period', { worldId: this.worldId, id });
    }

    async listPeriods(): Promise<CozoPeriod[]> {
        return invoke<CozoPeriod[]>('cozo_list_periods', { worldId: this.worldId });
    }

    async getPeriodChildren(parentId: string): Promise<CozoPeriod[]> {
        return invoke<CozoPeriod[]>('cozo_get_period_children', { worldId: this.worldId, parentId });
    }

    async deletePeriod(id: string): Promise<boolean> {
        return invoke<boolean>('cozo_delete_period', { worldId: this.worldId, id });
    }

    // =========================================================================
    // FIELD BINDINGS
    // =========================================================================

    async createBinding(params: {
        sourceEntityId: string;
        sourceFieldName: string;
        targetEntityId: string;
        targetFieldName: string;
        bindingType: BindingType;
        transform?: unknown;
        aggregationFn?: AggregationFunction;
        aggregationFilter?: unknown;
        allowOverride?: boolean;
    }): Promise<CozoFieldBinding> {
        return invoke<CozoFieldBinding>('cozo_create_binding', {
            worldId: this.worldId,
            ...params,
        });
    }

    async getBinding(id: string): Promise<CozoFieldBinding | null> {
        return invoke<CozoFieldBinding | null>('cozo_get_binding', { worldId: this.worldId, id });
    }

    async listBindings(): Promise<CozoFieldBinding[]> {
        return invoke<CozoFieldBinding[]>('cozo_list_bindings', { worldId: this.worldId });
    }

    async listBindingsByEntity(entityId: string): Promise<CozoFieldBinding[]> {
        return invoke<CozoFieldBinding[]>('cozo_list_bindings_by_entity', { worldId: this.worldId, entityId });
    }

    async deleteBinding(id: string): Promise<boolean> {
        return invoke<boolean>('cozo_delete_binding', { worldId: this.worldId, id });
    }

    // =========================================================================
    // BULK OPERATIONS
    // =========================================================================

    async loadAllContent(): Promise<{
        notes: CozoNote[];
        folders: CozoFolder[];
        folderTree: CozoFolderTreeNode[];
    }> {
        if (!isTauri()) {
            return { notes: [], folders: [], folderTree: [] };
        }

        const [notes, folders, folderTree] = await Promise.all([
            this.listNotes(),
            this.listFolders(),
            this.getFolderTree(),
        ]);

        return { notes, folders, folderTree };
    }

    async loadCalendarContent(calendarId = 'primary'): Promise<{
        events: CozoCalEvent[];
        periods: CozoPeriod[];
    }> {
        if (!isTauri()) {
            return { events: [], periods: [] };
        }

        const [events, periods] = await Promise.all([
            this.listCalEvents(),
            this.listPeriods(),
        ]);

        return { events, periods };
    }

    async loadNetworkContent(): Promise<{
        networks: CozoNetwork[];
        entities: CozoEntity[];
    }> {
        if (!isTauri()) {
            return { networks: [], entities: [] };
        }

        const [networks, entities] = await Promise.all([
            this.listNetworks(),
            this.listEntities(),
        ]);

        return { networks, entities };
    }
}

// =============================================================================
// SINGLETON EXPORT
// =============================================================================

export const contentAPI = new ContentAPI();

// Also export the class for testing/extension
export { ContentAPI };
