/**
 * Binding Engine Adapter
 * 
 * Provides backward-compatible interface that routes to either:
 * - SQLite (legacy, deprecated)
 * - Tauri/SurrealDB (new)
 * 
 * This allows gradual migration without breaking existing code.
 */

import { isTauri } from '@/lib/tauri';
import { bindingBridge, type FieldBinding, type CreateBindingParams, type UpdateBindingParams } from '@/lib/tauri/binding-bridge';
import type {
    FieldBinding as LegacyFieldBinding,
    CreateBindingOptions,
    UpdateBindingOptions,
    ResolvedValue,
    BindingChangeEvent,
} from './types';

// Default world ID for now (TODO: get from context)
const DEFAULT_WORLD_ID = 'default';

/**
 * Binding Engine Adapter - routes to Tauri or falls back to SQLite
 */
export class BindingEngineAdapter {
    private static instance: BindingEngineAdapter | null = null;

    // In-memory binding graph (loaded from backend)
    private bindings: Map<string, FieldBinding> = new Map();
    private bindingsByTarget: Map<string, FieldBinding[]> = new Map();
    private bindingsBySource: Map<string, FieldBinding[]> = new Map();

    // Event listeners
    private listeners: Set<(event: BindingChangeEvent) => void> = new Set();

    // Initialization state
    private initialized = false;
    private worldId = DEFAULT_WORLD_ID;

    private constructor() { }

    static getInstance(): BindingEngineAdapter {
        if (!BindingEngineAdapter.instance) {
            BindingEngineAdapter.instance = new BindingEngineAdapter();
        }
        return BindingEngineAdapter.instance;
    }

    /**
     * Set the world ID (for multi-world support)
     */
    setWorldId(worldId: string): void {
        this.worldId = worldId;
    }

    /**
     * Initialize the binding engine
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;

        try {
            if (isTauri()) {
                // Load from SurrealDB via Tauri
                const bindings = await bindingBridge.listBindings(this.worldId);
                this.bindings.clear();
                this.bindingsByTarget.clear();
                this.bindingsBySource.clear();

                for (const binding of bindings) {
                    this.indexBinding(binding);
                }

                console.log(`[BindingEngineAdapter] Initialized with ${this.bindings.size} bindings (SurrealDB)`);
            } else {
                // WASM fallback - no bindings in browser mode
                console.log('[BindingEngineAdapter] Initialized with 0 bindings (WASM mode - no persistence)');
            }

            this.initialized = true;
        } catch (error) {
            console.error('[BindingEngineAdapter] Failed to initialize:', error);
            // Don't throw - bindings are optional
            this.initialized = true;
        }
    }

    private createFieldKey(entityId: string, fieldName: string): string {
        return `${entityId}::${fieldName}`;
    }

    private indexBinding(binding: FieldBinding): void {
        this.bindings.set(binding.id, binding);

        // Index by target
        const targetKey = this.createFieldKey(binding.target_entity_id, binding.target_field_name);
        const targetBindings = this.bindingsByTarget.get(targetKey) || [];
        targetBindings.push(binding);
        this.bindingsByTarget.set(targetKey, targetBindings);

        // Index by source
        const sourceKey = this.createFieldKey(binding.source_entity_id, binding.source_field_name);
        const sourceBindings = this.bindingsBySource.get(sourceKey) || [];
        sourceBindings.push(binding);
        this.bindingsBySource.set(sourceKey, sourceBindings);
    }

    private removeBindingFromIndex(bindingId: string): void {
        const binding = this.bindings.get(bindingId);
        if (!binding) return;

        this.bindings.delete(bindingId);

        // Remove from target index
        const targetKey = this.createFieldKey(binding.target_entity_id, binding.target_field_name);
        const targetBindings = this.bindingsByTarget.get(targetKey) || [];
        this.bindingsByTarget.set(targetKey, targetBindings.filter(b => b.id !== bindingId));

        // Remove from source index
        const sourceKey = this.createFieldKey(binding.source_entity_id, binding.source_field_name);
        const sourceBindings = this.bindingsBySource.get(sourceKey) || [];
        this.bindingsBySource.set(sourceKey, sourceBindings.filter(b => b.id !== bindingId));
    }

    // ============================================
    // CRUD OPERATIONS
    // ============================================

    async createBinding(options: CreateBindingOptions): Promise<LegacyFieldBinding> {
        if (!isTauri()) {
            throw new Error('Bindings require Tauri mode');
        }

        const params: CreateBindingParams = {
            world_id: this.worldId,
            source_entity_id: options.sourceEntityId,
            source_field_name: options.sourceFieldName,
            target_entity_id: options.targetEntityId,
            target_field_name: options.targetFieldName,
            binding_type: options.bindingType,
            transform: options.transform ? { transform_type: options.transform.type, params: options.transform.params } : undefined,
            aggregation_fn: options.aggregationFn,
            aggregation_filter: options.aggregationFilter,
            allow_override: options.allowOverride,
        };

        const binding = await bindingBridge.createBinding(params);
        this.indexBinding(binding);

        // Emit event
        this.emit({
            type: 'created',
            bindingId: binding.id,
            affectedFields: [{ entityId: options.sourceEntityId, fieldName: options.sourceFieldName }],
            timestamp: Date.now(),
        });

        return this.toLegacyBinding(binding);
    }

    async deleteBinding(bindingId: string): Promise<boolean> {
        if (!isTauri()) return false;

        const binding = this.bindings.get(bindingId);
        if (!binding) return false;

        await bindingBridge.deleteBinding(this.worldId, bindingId);
        this.removeBindingFromIndex(bindingId);

        // Emit event
        this.emit({
            type: 'deleted',
            bindingId,
            affectedFields: [{ entityId: binding.source_entity_id, fieldName: binding.source_field_name }],
            timestamp: Date.now(),
        });

        return true;
    }

    getBinding(bindingId: string): LegacyFieldBinding | undefined {
        const binding = this.bindings.get(bindingId);
        return binding ? this.toLegacyBinding(binding) : undefined;
    }

    getBindingsForSource(entityId: string, fieldName: string): LegacyFieldBinding[] {
        const key = this.createFieldKey(entityId, fieldName);
        const bindings = this.bindingsBySource.get(key) || [];
        return bindings.map(b => this.toLegacyBinding(b));
    }

    getBindingsForTarget(entityId: string, fieldName: string): LegacyFieldBinding[] {
        const key = this.createFieldKey(entityId, fieldName);
        const bindings = this.bindingsByTarget.get(key) || [];
        return bindings.map(b => this.toLegacyBinding(b));
    }

    getBindingsForEntity(entityId: string): LegacyFieldBinding[] {
        return Array.from(this.bindings.values())
            .filter(b => b.source_entity_id === entityId || b.target_entity_id === entityId)
            .map(b => this.toLegacyBinding(b));
    }

    getAllBindings(): LegacyFieldBinding[] {
        return Array.from(this.bindings.values()).map(b => this.toLegacyBinding(b));
    }

    hasBindings(entityId: string, fieldName: string): boolean {
        const key = this.createFieldKey(entityId, fieldName);
        const asSource = this.bindingsBySource.get(key);
        const asTarget = this.bindingsByTarget.get(key);
        return (asSource && asSource.length > 0) || (asTarget && asTarget.length > 0);
    }

    // ============================================
    // EVENT SYSTEM
    // ============================================

    subscribe(callback: (event: BindingChangeEvent) => void): () => void {
        this.listeners.add(callback);
        return () => this.listeners.delete(callback);
    }

    private emit(event: BindingChangeEvent): void {
        for (const listener of this.listeners) {
            try {
                listener(event);
            } catch (error) {
                console.error('[BindingEngineAdapter] Error in event listener:', error);
            }
        }
    }

    // ============================================
    // CONVERSION
    // ============================================

    private toLegacyBinding(binding: FieldBinding): LegacyFieldBinding {
        return {
            id: binding.id,
            sourceEntityId: binding.source_entity_id,
            sourceFieldName: binding.source_field_name,
            targetEntityId: binding.target_entity_id,
            targetFieldName: binding.target_field_name,
            bindingType: binding.binding_type,
            transform: binding.transform ? { type: binding.transform.transform_type || 'identity', params: binding.transform.params } : undefined,
            aggregationFn: binding.aggregation_fn,
            aggregationFilter: binding.aggregation_filter,
            allowOverride: binding.allow_override,
            isActive: binding.is_active,
            createdAt: new Date(binding.created_at).getTime(),
            updatedAt: new Date(binding.updated_at).getTime(),
        };
    }
}

// Export singleton instance
export const bindingEngineAdapter = BindingEngineAdapter.getInstance();
