/**
 * EntityHotCache - LEGACY CACHE LAYER
 * 
 * This cache is being replaced by smartGraphRegistry's local cache.
 * Keeping for API compatibility but should migrate consumers to smartGraphRegistry.
 * 
 * @deprecated Use smartGraphRegistry directly
 */

import type { QueryClient } from '@tanstack/react-query';
import { smartGraphRegistry, type RegisteredEntity } from '@/lib/tauri';
import type { EntityKind } from '@/lib/types/entityTypes';

export interface HotCacheEntity {
  id: string;
  name: string;
  entity_kind: string;
  entity_subtype?: string | null;
  group_id: string;
  frequency: number;
  aliases: string[];
  canonical_note_id?: string | null;
}

// Adapter type alias
type CozoEntity = RegisteredEntity;

function cozoEntityToHotCache(entity: CozoEntity): HotCacheEntity {
  return {
    id: entity.id,
    name: entity.label,
    entity_kind: entity.kind,
    entity_subtype: entity.subtype,
    group_id: entity.firstNote,
    frequency: entity.totalMentions || 1,
    aliases: entity.aliases || [],
    canonical_note_id: entity.firstNote,
  };
}

/**
 * @deprecated Use smartGraphRegistry directly
 */
export class EntityHotCache {
  private cache: Map<string, HotCacheEntity> = new Map();
  private nameIndex: Map<string, string> = new Map();
  private aliasIndex: Map<string, string> = new Map();
  private queryClient: QueryClient | null = null;
  private currentGroupId: string | null = null;
  private warmupPromise: Promise<void> | null = null;
  private initialized = false;

  setQueryClient(client: QueryClient): void {
    this.queryClient = client;
  }

  getEntityById(id: string): HotCacheEntity | undefined {
    return this.cache.get(id);
  }

  findEntity(text: string): HotCacheEntity | undefined {
    const normalized = text.trim().toLowerCase();

    const byNameId = this.nameIndex.get(normalized);
    if (byNameId) {
      return this.cache.get(byNameId);
    }

    const byAliasId = this.aliasIndex.get(normalized);
    if (byAliasId) {
      return this.cache.get(byAliasId);
    }

    // Fallback to smartGraphRegistry
    if (!this.initialized) {
      const fromRegistry = smartGraphRegistry.findEntityByLabel(text);
      if (fromRegistry) {
        const hotEntity = cozoEntityToHotCache(fromRegistry);
        this.addToCache(hotEntity);
        return hotEntity;
      }
    }

    return undefined;
  }

  getAllEntities(): HotCacheEntity[] {
    return Array.from(this.cache.values());
  }

  getEntitiesByKind(kind: string): HotCacheEntity[] {
    return Array.from(this.cache.values()).filter(e => e.entity_kind === kind);
  }

  async warmCache(groupId?: string): Promise<void> {
    if (this.warmupPromise && (!groupId || this.currentGroupId === groupId)) {
      return this.warmupPromise;
    }

    this.warmupPromise = this._doWarmCache(groupId);
    return this.warmupPromise;
  }

  private async _doWarmCache(groupId?: string): Promise<void> {
    this.currentGroupId = groupId || null;
    this.cache.clear();
    this.nameIndex.clear();
    this.aliasIndex.clear();

    try {
      // Use smartGraphRegistry instead of unifiedRegistry
      await smartGraphRegistry.init();
      const entities = smartGraphRegistry.getAllEntities();

      for (const entity of entities) {
        const hotEntity = cozoEntityToHotCache(entity);
        this.addToCache(hotEntity);
      }

      this.initialized = true;
      console.log(`[EntityHotCache] Warmed cache with ${entities.length} entities (via smartGraphRegistry)`);
    } catch (err) {
      console.error('[EntityHotCache] Failed to warm cache:', err);
    }
  }

  private addToCache(entity: HotCacheEntity): void {
    this.cache.set(entity.id, entity);
    this.nameIndex.set(entity.name.trim().toLowerCase(), entity.id);

    for (const alias of entity.aliases || []) {
      this.aliasIndex.set(alias.trim().toLowerCase(), entity.id);
    }
  }

  private removeFromCache(entityId: string): void {
    const entity = this.cache.get(entityId);
    if (!entity) return;

    this.nameIndex.delete(entity.name.trim().toLowerCase());
    for (const alias of entity.aliases || []) {
      this.aliasIndex.delete(alias.trim().toLowerCase());
    }
    this.cache.delete(entityId);
  }

  invalidate(entityId: string): void {
    this.removeFromCache(entityId);

    if (this.queryClient) {
      this.queryClient.invalidateQueries({ queryKey: ['entity', entityId] });
      this.queryClient.invalidateQueries({ queryKey: ['entities'] });
    }
  }

  invalidateAll(): void {
    this.cache.clear();
    this.nameIndex.clear();
    this.aliasIndex.clear();
    this.currentGroupId = null;
    this.warmupPromise = null;
    this.initialized = false;

    if (this.queryClient) {
      this.queryClient.invalidateQueries({ queryKey: ['entities'] });
    }
  }

  /** @deprecated Use smartGraphRegistry.registerEntity() */
  async registerEntity(
    label: string,
    kind: EntityKind,
    noteId: string,
    options?: { subtype?: string; aliases?: string[]; attributes?: Record<string, any> }
  ): Promise<HotCacheEntity> {
    const entity = await smartGraphRegistry.registerEntity(label, kind, noteId, {
      subtype: options?.subtype,
      source: 'user',
    });

    const hotEntity = cozoEntityToHotCache(entity);
    this.addToCache(hotEntity);

    if (this.queryClient) {
      this.queryClient.invalidateQueries({ queryKey: ['entities'] });
    }

    return hotEntity;
  }

  /** @deprecated Not supported by smartGraphRegistry */
  async mergeEntities(targetId: string, sourceId: string): Promise<boolean> {
    console.warn('[EntityHotCache] mergeEntities not supported - use direct smartGraphRegistry call');
    return false;
  }

  /** @deprecated Use smartGraphRegistry.deleteEntity() */
  async deleteEntity(entityId: string): Promise<void> {
    await smartGraphRegistry.deleteEntity(entityId);
    this.invalidate(entityId);
  }

  /** @deprecated Not supported */
  async addAlias(entityId: string, alias: string): Promise<boolean> {
    console.warn('[EntityHotCache] addAlias not supported by smartGraphRegistry');
    return false;
  }

  /** @deprecated Not supported */
  async removeAlias(entityId: string, alias: string): Promise<boolean> {
    console.warn('[EntityHotCache] removeAlias not supported by smartGraphRegistry');
    return false;
  }

  async search(query: string, limit: number = 50): Promise<HotCacheEntity[]> {
    const normalized = query.trim().toLowerCase();

    const localResults: HotCacheEntity[] = [];
    for (const entity of this.cache.values()) {
      if (entity.name.toLowerCase().includes(normalized)) {
        localResults.push(entity);
        continue;
      }
      for (const alias of entity.aliases || []) {
        if (alias.toLowerCase().includes(normalized)) {
          localResults.push(entity);
          break;
        }
      }
    }

    return localResults.slice(0, limit);
  }

  async refreshEntity(entityId: string): Promise<HotCacheEntity | null> {
    const entity = smartGraphRegistry.getEntityById(entityId);

    if (entity) {
      const hotEntity = cozoEntityToHotCache(entity);
      this.removeFromCache(entityId);
      this.addToCache(hotEntity);
      return hotEntity;
    } else {
      this.removeFromCache(entityId);
    }

    return null;
  }

  async findOrFetch(text: string): Promise<HotCacheEntity | null> {
    const cached = this.findEntity(text);
    if (cached) return cached;

    const entity = smartGraphRegistry.findEntityByLabel(text);
    if (entity) {
      const hotEntity = cozoEntityToHotCache(entity);
      this.addToCache(hotEntity);
      return hotEntity;
    }

    return null;
  }

  get size(): number {
    return this.cache.size;
  }

  get isWarmed(): boolean {
    return this.initialized && this.cache.size > 0;
  }
}

export const entityHotCache = new EntityHotCache();
