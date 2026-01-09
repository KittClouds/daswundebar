# SQLite Database Module - DEPRECATED

**Status: DEPRECATED - Do not use for new code**

This module is being phased out in favor of:
- **SurrealDB** for document storage (folders, notes, calendar, bindings)
- **Rust CozoDB** for graph storage (entities, relationships)

## Migration Status

| Component | Old (SQLite) | New |
|-----------|--------------|-----|
| Entities | `entityRegistry` | `smartGraphRegistry` |
| Relationships | `relationshipRegistry` | `smartGraphRegistry` |
| Folders | `dbClient` | `surreal-bridge.ts` |
| Notes | `dbClient` | `surreal-bridge.ts` |
| Calendar | `dbClient` | `calendar-bridge.ts` |
| Networks | IndexedDB | `network-bridge.ts` |
| Bindings | `dbClient` | `binding-bridge.ts` |

## Files to Delete (After Full Migration)

### Phase 1 (Safe to delete now)
- `sync/Hydration.ts` - ✅ DELETED

### Phase 2 (After atoms migration)
- `sync/BatchWriter.ts`
- `sync/DirtyTracker.ts`
- `sync/GraphSQLiteSync.ts`
- `sync/SyncEngineV2.ts`
- `sync/DeltaCollector.ts`
- `sync/TransactionBuilder.ts`
- `sync/StreamingCozoSync.ts`

### Phase 3 (After all consumers migrated)
- `client/db-client.ts`
- `worker/` directory
- `search/` directory

## Active Consumers (Need Migration)

These files still use SQLite and need to be updated:

1. `atoms/notes-async.ts` - Uses dbClient for note CRUD
2. `atoms/notes-sync.ts` - Uses dbClient for sync
3. `atoms/calendar.ts` - Uses dbClient for calendar
4. `atoms/entity-attributes.ts` - Uses dbClient for attributes
5. `lib/fact-sheet/api.ts` - Uses dbClient for fact sheets
6. `lib/bindings/BindingEngine.ts` - Uses dbClient (has new adapter)
7. `lib/embeddings/pipeline/` - Uses dbClient for embeddings

## Recommended Approach

1. Use `dbFacade` from `lib/db/facade.ts` which routes to appropriate backend
2. Use `bindingEngineAdapter` instead of `bindingEngine`
3. Import from `@/lib/tauri/*-bridge.ts` for new code
