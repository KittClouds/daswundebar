export * from './interfaces';

export { EntityStoreImpl, getEntityStoreImpl, resetEntityStore } from './impl/EntityStoreImpl';
export { EdgeStoreImpl, getEdgeStoreImpl, resetEdgeStore } from './impl/EdgeStoreImpl';
export { MentionStoreImpl, getMentionStoreImpl, resetMentionStore } from './impl/MentionStoreImpl';
export { BlueprintStoreImpl, getBlueprintStoreImpl, resetBlueprintStore } from './impl/BlueprintStoreImpl';
export { TemporalStoreImpl, getTemporalStoreImpl, resetTemporalStore } from './impl/TemporalStoreImpl';
export { EmbeddingStoreImpl, getEmbeddingStoreImpl, resetEmbeddingStore } from './impl/EmbeddingStoreImpl';
export { TauriBlueprintStoreAdapter, getTauriBlueprintStore, resetTauriBlueprintStore } from './impl/TauriBlueprintStoreAdapter';
export { TauriTemporalStoreAdapter, getTauriTemporalStore, resetTauriTemporalStore } from './impl/TauriTemporalStoreAdapter';

import type {
  IEntityStore,
  IEdgeStore,
  IMentionStore,
  IBlueprintStore,
  ITemporalStore,
  IEmbeddingStore,
  IStorageService,
} from './interfaces';

import { getEntityStoreImpl } from './impl/EntityStoreImpl';
import { getEdgeStoreImpl } from './impl/EdgeStoreImpl';
import { getMentionStoreImpl } from './impl/MentionStoreImpl';
import { getBlueprintStoreImpl } from './impl/BlueprintStoreImpl';
import { getTemporalStoreImpl } from './impl/TemporalStoreImpl';
import { getEmbeddingStoreImpl } from './impl/EmbeddingStoreImpl';
import { getTauriBlueprintStore } from './impl/TauriBlueprintStoreAdapter';
import { getTauriTemporalStore } from './impl/TauriTemporalStoreAdapter';
import { isTauri } from '@/lib/tauri/bridge';

export function getEntityStore(): IEntityStore {
  return getEntityStoreImpl();
}

export function getEdgeStore(): IEdgeStore {
  return getEdgeStoreImpl();
}

export function getMentionStore(): IMentionStore {
  return getMentionStoreImpl();
}

/**
 * Get the Blueprint Store.
 * Routes to native Rust CozoDB when in Tauri, falls back to browser WASM for web.
 */
export function getBlueprintStore(): IBlueprintStore {
  if (isTauri()) {
    return getTauriBlueprintStore();
  }
  return getBlueprintStoreImpl();
}

/**
 * Get the Temporal Store.
 * Routes to native Rust CozoDB when in Tauri, falls back to in-memory for web.
 */
export function getTemporalStore(): ITemporalStore {
  if (isTauri()) {
    return getTauriTemporalStore();
  }
  return getTemporalStoreImpl();
}

export function getEmbeddingStore(): IEmbeddingStore {
  return getEmbeddingStoreImpl();
}

export function getStorageService(): IStorageService {
  return {
    entities: getEntityStore(),
    edges: getEdgeStore(),
    mentions: getMentionStore(),
    blueprints: getBlueprintStore(),
    temporal: getTemporalStore(),
    embeddings: getEmbeddingStore(),
  };
}

export async function initializeStorage(): Promise<void> {
  const blueprintStore = getBlueprintStore();
  await blueprintStore.initialize();
  console.log('Storage service initialized');
}
