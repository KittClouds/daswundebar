/**
 * TauriOrchestrator - Unified coordinator for all Tauri-related initialization
 * 
 * Single entry point for:
 * - Backend connection
 * - Entity loading from GraphRegistry
 * - Scanner hydration
 * - Ready state management
 * 
 * @module lib/tauri/orchestrator
 */

import { tauriScanner, isTauri, version } from './bridge';
import { smartGraphRegistry, type RegisteredEntity } from './smart-graph-registry';
import { initSurrealDb } from './surreal-bridge';
import type { EntityDefinition } from './bridge';

// =============================================================================
// State Machine
// =============================================================================

export enum TauriState {
    Uninitialized = 'uninitialized',
    Connecting = 'connecting',
    Connected = 'connected',
    LoadingEntities = 'loading_entities',
    Hydrating = 'hydrating',
    Ready = 'ready',
    Failed = 'failed',
}

export interface TauriOrchestratorStatus {
    state: TauriState;
    backendVersion?: string;
    entityCount?: number;
    lastError?: string;
}

// =============================================================================
// TauriOrchestrator
// =============================================================================

export class TauriOrchestrator {
    private state: TauriState = TauriState.Uninitialized;
    private backendVersion: string | null = null;
    private entityCount = 0;
    private lastError: string | null = null;
    private initPromise: Promise<void> | null = null;
    private readyCallbacks: Array<() => void> = [];
    private _surrealReady = false;

    /**
     * Full initialization - connects, loads entities, hydrates scanner
     * Safe to call multiple times (idempotent)
     */
    async init(): Promise<void> {
        // Skip if not in Tauri
        if (!isTauri()) {
            console.log('[TauriOrchestrator] Not in Tauri environment, skipping');
            return;
        }

        // Already initialized or in progress
        if (this.state === TauriState.Ready) return;
        if (this.initPromise) return this.initPromise;

        this.initPromise = this._doInit();
        return this.initPromise;
    }

    private async _doInit(): Promise<void> {
        try {
            // Phase 1: Connect to backend
            this.state = TauriState.Connecting;
            console.log('[TauriOrchestrator] Connecting to backend...');

            await tauriScanner.initialize();
            this.backendVersion = await version();
            this.state = TauriState.Connected;
            console.log(`[TauriOrchestrator] Connected: ${this.backendVersion}`);

            // Phase 1.5: Initialize SurrealDB (required before notes/calendar atoms)
            console.log('[TauriOrchestrator] Initializing SurrealDB...');
            try {
                await initSurrealDb();
                this._surrealReady = true;
                console.log('[TauriOrchestrator] SurrealDB ready');
            } catch (surrealError) {
                // Graceful degradation - SurrealDB failure doesn't block the app
                console.error('[TauriOrchestrator] SurrealDB init failed (app will continue with limited functionality):', surrealError);
                this._surrealReady = false;
            }

            // Phase 2: Load entities from GraphRegistry
            this.state = TauriState.LoadingEntities;
            await smartGraphRegistry.init();
            const entities = smartGraphRegistry.getAllEntities();
            this.entityCount = entities.length;
            console.log(`[TauriOrchestrator] Loaded ${this.entityCount} entities from GraphRegistry`);

            // Phase 3: Hydrate scanner with entities
            this.state = TauriState.Hydrating;
            if (entities.length > 0) {
                const entityDefs = this.toEntityDefinitions(entities);
                await tauriScanner.hydrateEntities(entityDefs);
                console.log(`[TauriOrchestrator] Hydrated scanner with ${entities.length} entities`);
            } else {
                // Hydrate with empty to mark scanner as ready
                await tauriScanner.hydrateEntities([]);
                console.log('[TauriOrchestrator] Hydrated scanner (no entities yet)');
            }

            // Done!
            this.state = TauriState.Ready;
            this.fireReadyCallbacks();
            console.log('[TauriOrchestrator] ✅ Ready');

        } catch (error) {
            this.lastError = String(error);
            this.state = TauriState.Failed;
            console.error('[TauriOrchestrator] Init failed:', error);
            throw error;
        }
    }

    /**
     * Check if orchestrator is fully ready for scanning
     */
    isReady(): boolean {
        return this.state === TauriState.Ready;
    }

    /**
     * Check if SurrealDB is ready for notes/calendar operations
     */
    isSurrealReady(): boolean {
        return this._surrealReady;
    }

    /**
     * Get current status
     */
    getStatus(): TauriOrchestratorStatus {
        return {
            state: this.state,
            backendVersion: this.backendVersion ?? undefined,
            entityCount: this.entityCount,
            lastError: this.lastError ?? undefined,
        };
    }

    /**
     * Wait for ready state (with optional timeout)
     */
    async waitForReady(timeoutMs = 30000): Promise<void> {
        if (this.state === TauriState.Ready) return;

        // If not initialized, start initialization
        if (this.state === TauriState.Uninitialized) {
            return this.init();
        }

        // Wait for ready via callback
        return new Promise<void>((resolve, reject) => {
            const timeout = setTimeout(() => {
                reject(new Error(`[TauriOrchestrator] Timeout waiting for ready (${timeoutMs}ms)`));
            }, timeoutMs);

            this.onReady(() => {
                clearTimeout(timeout);
                resolve();
            });
        });
    }

    /**
     * Register callback for when ready
     * Returns unsubscribe function
     */
    onReady(callback: () => void): () => void {
        if (this.state === TauriState.Ready) {
            // Already ready, call immediately
            try { callback(); } catch (e) { console.warn('[TauriOrchestrator] Ready callback error:', e); }
            return () => { };
        }

        this.readyCallbacks.push(callback);
        return () => {
            const idx = this.readyCallbacks.indexOf(callback);
            if (idx >= 0) this.readyCallbacks.splice(idx, 1);
        };
    }

    /**
     * Trigger entity re-hydration (after entity registry changes)
     */
    async rehydrate(): Promise<void> {
        if (!isTauri()) return;
        if (this.state !== TauriState.Ready) {
            console.warn('[TauriOrchestrator] Cannot rehydrate - not ready');
            return;
        }

        try {
            // Get entities that need hydration (dirty tracking)
            const entities = await smartGraphRegistry.getEntitiesForHydration();

            if (entities) {
                const entityDefs = this.toEntityDefinitions(entities);
                await tauriScanner.hydrateEntities(entityDefs);
                this.entityCount = entities.length;
                console.log(`[TauriOrchestrator] Rehydrated with ${entities.length} entities`);
            } else {
                console.log('[TauriOrchestrator] Rehydration skipped (no changes)');
            }
        } catch (error) {
            console.error('[TauriOrchestrator] Rehydration failed:', error);
        }
    }

    /**
     * Get scanner instance (for direct access if needed)
     */
    get scanner() {
        return tauriScanner;
    }

    /**
     * Get graph registry instance (for direct access if needed)
     */
    get graphRegistry() {
        return smartGraphRegistry;
    }

    // =========================================================================
    // Private Helpers
    // =========================================================================

    private fireReadyCallbacks(): void {
        for (const cb of this.readyCallbacks) {
            try { cb(); } catch (e) { console.warn('[TauriOrchestrator] Ready callback error:', e); }
        }
        this.readyCallbacks = [];
    }

    private toEntityDefinitions(entities: RegisteredEntity[]): EntityDefinition[] {
        return entities.map(e => ({
            id: e.id,
            label: e.label,
            kind: e.kind,
            aliases: e.aliases || [],
        }));
    }
}

// =============================================================================
// Singleton Export
// =============================================================================

export const tauriOrchestrator = new TauriOrchestrator();
