import { cozoDb } from '@/lib/cozo-stubs/db';
import { ENTITY_QUERIES } from '@/lib/cozo-stubs/schema';
import { generateId } from '@/lib/utils/ids';
import { MemoryCache } from '../MemoryCache';
import { DebouncedWriter, type WriteOperation } from '../DebouncedWriter';
import type {
  IEntityStore,
  Entity,
  CreateEntityInput,
} from '../interfaces';

export class EntityStoreImpl implements IEntityStore {
  private entityByIdCache = new MemoryCache<Entity>(500, 120000); // 500 entities, 2min TTL
  private entitiesByKindCache = new MemoryCache<Entity[]>(50, 60000); // 50 kind queries, 1min TTL

  private writer = new DebouncedWriter<Entity>(
    async (ops) => {
      await this.batchExecute(ops);
    },
    300 // 300ms debounce
  );

  private async batchExecute(ops: WriteOperation<Entity>[]): Promise<void> {
    if (ops.length === 0) return;

    try {
      // CozoDB supports batching through lists. We can use a single script to process multiple ops.
      // But for simplicity in this implementation, we'll process them in a single transaction if possible.
      // Cozo WASM run() is already relatively atomic for a single script.

      for (const op of ops) {
        if (op.operation === 'upsert') {
          const params = this.entityToParams(op.data);
          cozoDb.run(ENTITY_QUERIES.upsert, params);
        } else if (op.operation === 'delete') {
          cozoDb.run(ENTITY_QUERIES.delete, { id: op.key });
        }
      }
    } catch (err) {
      console.error('EntityStoreImpl: Batch execution failed', err);
    }
  }

  private entityToParams(entity: Entity): Record<string, any> {
    return {
      id: entity.id,
      name: entity.name,
      normalized_name: entity.name.trim().toLowerCase(),
      entity_kind: entity.entity_kind,
      entity_subtype: entity.entity_subtype || null,
      group_id: entity.group_id,
      scope_type: entity.scope_type,
      created_at: entity.created_at,
      extraction_method: entity.extraction_method,
      summary: entity.summary || null,
      aliases: entity.aliases,
      canonical_note_id: entity.canonical_note_id || null,
      frequency: entity.frequency,
      degree_centrality: entity.degree_centrality ?? null,
      betweenness_centrality: entity.betweenness_centrality ?? null,
      closeness_centrality: entity.closeness_centrality ?? null,
      community_id: entity.community_id || null,
      attributes: entity.attributes || null,
      temporal_span: entity.temporal_span || null,
      participants: entity.participants,
      // Some fields from layer2-entities.ts might be missing in Entity interface
      source: (entity as any).source || 'manual',
      confidence: (entity as any).confidence ?? 1.0,
      blueprint_type_id: (entity as any).blueprint_type_id ?? null,
      blueprint_version_id: (entity as any).blueprint_version_id ?? null,
      blueprint_fields: (entity as any).blueprint_fields ?? null,
      provenance_data: (entity as any).provenance_data ?? null,
      alternate_types: (entity as any).alternate_types ?? null
    };
  }

  async upsertEntity(input: CreateEntityInput): Promise<Entity> {
    // Generate an ID if not present (usually new entities)
    const id = (input as any).id || generateId();
    const now = Date.now();

    const entity: Entity = {
      id,
      name: input.name,
      entity_kind: input.entity_kind,
      entity_subtype: input.entity_subtype,
      group_id: input.group_id,
      scope_type: input.scope_type || 'note',
      created_at: now,
      extraction_method: 'manual',
      summary: input.summary,
      aliases: input.aliases || [],
      canonical_note_id: input.canonical_note_id,
      frequency: 1,
      attributes: input.attributes,
      participants: []
    };

    // ✅ Immediate cache update (instant UI)
    this.entityByIdCache.set(entity.id, entity);
    this.invalidateKindCache(entity.entity_kind, entity.group_id);

    // ✅ Debounced DB write
    this.writer.write(entity.id, entity, 'upsert');

    return entity;
  }

  async getEntityById(id: string): Promise<Entity | null> {
    // ✅ Check cache first
    const cached = this.entityByIdCache.get(id);
    if (cached) return cached;

    if (!cozoDb.isReady()) return null;

    try {
      const result = cozoDb.runQuery(ENTITY_QUERIES.getById, { id });
      if (result.rows && result.rows.length > 0) {
        const entity = this.rowToEntity(result.headers, result.rows[0]);
        this.entityByIdCache.set(id, entity);
        return entity;
      }
    } catch (err) {
      console.error('Failed to get entity by ID', err);
    }

    return null;
  }

  private rowToEntity(headers: string[], row: any[]): Entity {
    const obj: any = {};
    headers.forEach((h, i) => {
      obj[h] = row[i];
    });

    return obj as Entity;
  }

  async findEntityByName(name: string, kind: string, groupId: string): Promise<Entity | null> {
    if (!cozoDb.isReady()) return null;

    try {
      const result = cozoDb.runQuery(ENTITY_QUERIES.findByNameAndKind, { name, kind, group_id: groupId });
      if (result.rows && result.rows.length > 0) {
        // Since findByNameAndKind only returns a subset of fields in current schema, we might need a full fetch
        const partial = this.rowToEntity(result.headers, result.rows[0]);
        return this.getEntityById(partial.id);
      }
    } catch (err) {
      console.error('Failed to find entity by name and kind', err);
    }
    return null;
  }

  async findEntityByNameOnly(name: string, groupId: string): Promise<Entity | null> {
    if (!cozoDb.isReady()) return null;

    try {
      const result = cozoDb.runQuery(ENTITY_QUERIES.findByName, { name, group_id: groupId });
      if (result.rows && result.rows.length > 0) {
        const partial = this.rowToEntity(result.headers, result.rows[0]);
        return this.getEntityById(partial.id);
      }
    } catch (err) {
      console.error('Failed to find entity by name only', err);
    }
    return null;
  }

  async deleteEntity(id: string): Promise<void> {
    const entity = await this.getEntityById(id);
    if (entity) {
      // ✅ Invalidate cache on delete
      this.entityByIdCache.delete(id);
      this.invalidateKindCache(entity.entity_kind, entity.group_id);

      // ✅ Debounced DB delete
      this.writer.write(id, entity, 'delete');
    }
  }

  async getEntitiesByKind(kind: string, groupId: string): Promise<Entity[]> {
    const cacheKey = `${groupId}:${kind}`;

    // ✅ Check cache first
    const cached = this.entitiesByKindCache.get(cacheKey);
    if (cached) return cached;

    if (!cozoDb.isReady()) return [];

    try {
      const result = cozoDb.runQuery(ENTITY_QUERIES.getByKind, { kind });
      const entities: Entity[] = [];

      // The getByKind query in schema returns limited fields. For correctness, we fetch all details.
      if (!result?.rows || !Array.isArray(result.rows)) return [];

      for (const row of result.rows) {
        const id = row[0]; // first header is id
        const fullEntity = await this.getEntityById(id);
        if (fullEntity && fullEntity.group_id === groupId) {
          entities.push(fullEntity);
        }
      }

      this.entitiesByKindCache.set(cacheKey, entities);
      return entities;
    } catch (err) {
      console.error('Failed to get entities by kind', err);
    }
    return [];
  }

  async getAllEntities(groupId: string): Promise<Entity[]> {
    if (!cozoDb.isReady()) return [];

    try {
      const result = cozoDb.runQuery(ENTITY_QUERIES.getByGroupId, { group_id: groupId });
      const entities: Entity[] = [];

      if (!result?.rows || !Array.isArray(result.rows)) {
        console.warn('[EntityStore] getAllEntities: No rows returned', result);
        return [];
      }

      for (const row of result.rows) {
        const id = row[0];
        const fullEntity = await this.getEntityById(id);
        if (fullEntity) entities.push(fullEntity);
      }
      return entities;
    } catch (err) {
      console.error('Failed to get all entities', err);
    }
    return [];
  }

  async updateEntityFrequency(id: string, frequency: number): Promise<void> {
    const entity = await this.getEntityById(id);
    if (entity) {
      const updated = { ...entity, frequency };
      this.entityByIdCache.set(id, updated);
      this.writer.write(id, updated, 'upsert');
    }
  }

  private invalidateKindCache(kind: string, groupId: string): void {
    this.entitiesByKindCache.delete(`${groupId}:${kind}`);
  }

  clearCaches(): void {
    this.entityByIdCache.clear();
    this.entitiesByKindCache.clear();
  }
}

let entityStoreInstance: EntityStoreImpl | null = null;

export function getEntityStoreImpl(): EntityStoreImpl {
  if (!entityStoreInstance) {
    entityStoreInstance = new EntityStoreImpl();
  }
  return entityStoreInstance;
}

export function resetEntityStore(): void {
  entityStoreInstance = null;
}
