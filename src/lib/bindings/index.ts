/**
 * Field Bindings Module
 * 
 * Entity-to-entity field binding system.
 * Enables reactive data flow between entities.
 */

// Types
export type {
    BindingType,
    AggregationFunction,
    TransformType,
    FieldBinding,
    Transform,
    AggregationFilter,
    BindingMetadata,
    BindingGraphNode,
    BindingGraphEdge,
    ResolvedValue,
    BindingChangeEvent,
    CreateBindingOptions,
    UpdateBindingOptions,
} from './types';

export {
    parseTransform,
    serializeTransform,
    createFieldKey,
    parseFieldKey,
} from './types';

// Transforms
export {
    applyTransform,
    applyTransformChain,
    isTransformValidForType,
    getAvailableTransforms,
} from './transforms';

// Aggregations
export {
    applyAggregation,
    isAggregationValidForType,
    getAvailableAggregations,
    getAggregationDescription,
} from './aggregations';

// Engine (legacy - uses SQLite)
/** @deprecated Use BindingEngineAdapter instead */
export { BindingEngine, bindingEngine } from './BindingEngine';

// Engine Adapter (new - uses SurrealDB via Tauri)
export { BindingEngineAdapter, bindingEngineAdapter } from './BindingEngineAdapter';

// Atoms
export {
    bindingEngineInitializedAtom,
    initializeBindingEngineAtom,
    allBindingsAtom,
    entityBindingsFamily,
    sourceBindingsFamily,
    targetBindingsFamily,
    hasBindingsFamily,
    createBindingAtom,
    updateBindingAtom,
    deleteBindingAtom,
    bindingChangeVersionAtom,
    subscribeToBindingEventsAtom,
    bindingCountAtom,
    wouldCreateCycleAtom,
} from './atoms';
