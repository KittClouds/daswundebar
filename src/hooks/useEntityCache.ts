import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { unifiedRegistry, type CozoEntity } from '@/lib/cozo-stubs/unified-registry';
import { entityHotCache, type HotCacheEntity } from '@/lib/storage/cache/EntityHotCache';
import { useEffect } from 'react';
import type { EntityKind } from '@/lib/types/entityTypes';

export function useEntity(entityId: string | null) {
  return useQuery({
    queryKey: ['entity', entityId],
    queryFn: async () => {
      if (!entityId) return null;

      const cached = entityHotCache.getEntityById(entityId);
      if (cached) return cached;

      const entity = await unifiedRegistry.getEntityById(entityId);
      return entity ? {
        id: entity.id,
        name: entity.label,
        entity_kind: entity.kind,
        entity_subtype: entity.subtype,
        group_id: entity.firstNote,
        frequency: entity.totalMentions || 1,
        aliases: entity.aliases || [],
        canonical_note_id: entity.firstNote,
      } : null;
    },
    enabled: !!entityId,
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  });
}

export function useEntities() {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (queryClient) {
      entityHotCache.setQueryClient(queryClient);
    }
  }, [queryClient]);

  const query = useQuery({
    queryKey: ['entities'],
    queryFn: async () => {
      await entityHotCache.warmCache();
      return entityHotCache.getAllEntities();
    },
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
  });

  return {
    ...query,
    findEntity: (text: string) => entityHotCache.findEntity(text),
    getEntityById: (id: string) => entityHotCache.getEntityById(id),
  };
}

export function useEntitySearch(query: string) {
  return useQuery({
    queryKey: ['entities', 'search', query],
    queryFn: async () => {
      if (!query || query.length < 2) return [];
      return entityHotCache.search(query, 50);
    },
    enabled: query.length >= 2,
    staleTime: 30 * 1000,
    gcTime: 60 * 1000,
  });
}

export function useEntityStats(entityId: string | null) {
  return useQuery({
    queryKey: ['entity', entityId, 'stats'],
    queryFn: async () => {
      if (!entityId) return null;
      const stats = await unifiedRegistry.getEntityStats(entityId);
      return stats;
    },
    enabled: !!entityId,
    staleTime: 60 * 1000,
  });
}

export function useEntityMutations() {
  const queryClient = useQueryClient();

  const registerMutation = useMutation({
    mutationFn: ({ label, kind, noteId, options }: {
      label: string;
      kind: EntityKind;
      noteId: string;
      options?: { subtype?: string; aliases?: string[]; attributes?: Record<string, any> };
    }) => entityHotCache.registerEntity(label, kind, noteId, options),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['entities'] });
    },
  });

  const mergeMutation = useMutation({
    mutationFn: ({ targetId, sourceId }: { targetId: string; sourceId: string }) =>
      entityHotCache.mergeEntities(targetId, sourceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['entities'] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (entityId: string) => entityHotCache.deleteEntity(entityId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['entities'] });
    },
  });

  const addAliasMutation = useMutation({
    mutationFn: ({ entityId, alias }: { entityId: string; alias: string }) =>
      entityHotCache.addAlias(entityId, alias),
    onSuccess: (_, { entityId }) => {
      queryClient.invalidateQueries({ queryKey: ['entity', entityId] });
    },
  });

  const removeAliasMutation = useMutation({
    mutationFn: ({ entityId, alias }: { entityId: string; alias: string }) =>
      entityHotCache.removeAlias(entityId, alias),
    onSuccess: (_, { entityId }) => {
      queryClient.invalidateQueries({ queryKey: ['entity', entityId] });
    },
  });

  return {
    register: registerMutation,
    merge: mergeMutation,
    delete: deleteMutation,
    addAlias: addAliasMutation,
    removeAlias: removeAliasMutation,
  };
}
