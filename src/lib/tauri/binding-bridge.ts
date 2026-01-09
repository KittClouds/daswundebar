/**
 * Binding Bridge (DEPRECATED)
 * 
 * This file is kept for backwards compatibility.
 * New code should use: import { contentAPI } from '@/lib/tauri/content-api'
 */

import { contentAPI, type CozoFieldBinding } from './content-api';

// =============================================================================
// TYPES (kept for backwards compat)
// =============================================================================

export type BindingType = 'mirror' | 'inherit' | 'aggregate';
export type AggregationFunction = 'sum' | 'avg' | 'min' | 'max' | 'count' | 'concat' | 'first' | 'last';

export interface BindingTransform {
    transform_type?: string;
    params?: unknown;
}

export interface AggregationFilter {
    relationship_type?: string;
    link_field_name?: string;
}

export interface FieldBinding {
    id: string;
    world_id: string;
    source_entity_id: string;
    source_field_name: string;
    target_entity_id: string;
    target_field_name: string;
    binding_type: BindingType;
    transform?: BindingTransform;
    aggregation_fn?: AggregationFunction;
    aggregation_filter?: AggregationFilter;
    allow_override: boolean;
    is_active: boolean;
    created_at: string;
    updated_at: string;
}

export interface CreateBindingParams {
    world_id: string;
    source_entity_id: string;
    source_field_name: string;
    target_entity_id: string;
    target_field_name: string;
    binding_type: BindingType;
    transform?: BindingTransform;
    aggregation_fn?: AggregationFunction;
    aggregation_filter?: AggregationFilter;
    allow_override?: boolean;
}

export interface UpdateBindingParams {
    transform?: BindingTransform;
    aggregation_fn?: AggregationFunction;
    aggregation_filter?: AggregationFilter;
    allow_override?: boolean;
    is_active?: boolean;
}

// =============================================================================
// ADAPTERS
// =============================================================================

function cozoToBinding(c: CozoFieldBinding): FieldBinding {
    return {
        id: c.id,
        world_id: c.world_id,
        source_entity_id: c.source_entity_id,
        source_field_name: c.source_field_name,
        target_entity_id: c.target_entity_id,
        target_field_name: c.target_field_name,
        binding_type: c.binding_type,
        transform: c.transform as BindingTransform,
        aggregation_fn: c.aggregation_fn as AggregationFunction,
        aggregation_filter: c.aggregation_filter as AggregationFilter,
        allow_override: c.allow_override,
        is_active: c.is_active,
        created_at: new Date(c.created_at * 1000).toISOString(),
        updated_at: new Date(c.updated_at * 1000).toISOString(),
    };
}

// =============================================================================
// BRIDGE FUNCTIONS
// =============================================================================

export async function createBinding(params: CreateBindingParams): Promise<FieldBinding> {
    const result = await contentAPI.createBinding({
        sourceEntityId: params.source_entity_id,
        sourceFieldName: params.source_field_name,
        targetEntityId: params.target_entity_id,
        targetFieldName: params.target_field_name,
        bindingType: params.binding_type,
        transform: params.transform,
        aggregationFn: params.aggregation_fn,
        aggregationFilter: params.aggregation_filter,
        allowOverride: params.allow_override,
    });
    return cozoToBinding(result);
}

export async function getBinding(_worldId: string, id: string): Promise<FieldBinding | null> {
    const result = await contentAPI.getBinding(id);
    return result ? cozoToBinding(result) : null;
}

export async function listBindings(_worldId: string): Promise<FieldBinding[]> {
    const results = await contentAPI.listBindings();
    return results.map(cozoToBinding);
}

export async function getBindingsBySource(
    _worldId: string,
    entityId: string,
    _fieldName: string
): Promise<FieldBinding[]> {
    // Note: CozoDB version uses list_by_entity which gets all bindings for an entity
    const results = await contentAPI.listBindingsByEntity(entityId);
    return results.map(cozoToBinding).filter(b => b.source_entity_id === entityId);
}

export async function getBindingsByTarget(
    _worldId: string,
    entityId: string,
    _fieldName: string
): Promise<FieldBinding[]> {
    const results = await contentAPI.listBindingsByEntity(entityId);
    return results.map(cozoToBinding).filter(b => b.target_entity_id === entityId);
}

export async function getBindingsByEntity(
    _worldId: string,
    entityId: string
): Promise<FieldBinding[]> {
    const results = await contentAPI.listBindingsByEntity(entityId);
    return results.map(cozoToBinding);
}

export async function updateBinding(
    _worldId: string,
    _id: string,
    _params: UpdateBindingParams
): Promise<FieldBinding> {
    console.warn('[BindingBridge] updateBinding not yet implemented in CozoDB backend');
    throw new Error('updateBinding not implemented');
}

export async function deleteBinding(_worldId: string, id: string): Promise<boolean> {
    return contentAPI.deleteBinding(id);
}

export async function deleteBindingsByEntity(
    _worldId: string,
    entityId: string
): Promise<number> {
    const bindings = await contentAPI.listBindingsByEntity(entityId);
    let deleted = 0;
    for (const b of bindings) {
        await contentAPI.deleteBinding(b.id);
        deleted++;
    }
    return deleted;
}

// =============================================================================
// BINDING BRIDGE NAMESPACE
// =============================================================================

export const bindingBridge = {
    createBinding,
    getBinding,
    listBindings,
    getBindingsBySource,
    getBindingsByTarget,
    getBindingsByEntity,
    updateBinding,
    deleteBinding,
    deleteBindingsByEntity,
};
