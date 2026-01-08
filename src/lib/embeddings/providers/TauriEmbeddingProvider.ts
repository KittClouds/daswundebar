// src/lib/embeddings/providers/TauriEmbeddingProvider.ts
//
// Native Tauri embedding provider using embed-anything via IPC
// This is the native Tauri alternative to LocalEmbeddingProvider (Transformers.js)
// and RustEmbeddingProvider (WASM)
//
// Key differences:
// - Uses Tauri IPC instead of WASM
// - Model loaded/cached on Rust side
// - Embeddings generated natively (faster, lower memory)

import type { IEmbeddingProvider } from './types';
import type { EmbeddingModelDefinition } from '../models/ModelRegistry';
import { invoke } from '@tauri-apps/api/core';

/**
 * Response from rag_embed command
 */
interface EmbedResponse {
    embeddings: number[][];
    dimensions: number;
    model_id: string;
}

/**
 * Tauri-native embedding provider using embed-anything
 * Uses IPC to call Rust embedding service
 */
export class TauriEmbeddingProvider implements IEmbeddingProvider {
    readonly name: string;
    readonly provider = 'tauri';

    private modelId: string;
    private modelDef: EmbeddingModelDefinition;
    private initialized = false;
    private dimensions = 384; // Default, updated after init

    constructor(modelId: string) {
        this.modelId = modelId;
        this.modelDef = this.createModelDefinition(modelId);
        this.name = this.modelDef.name;
    }

    private createModelDefinition(modelId: string): EmbeddingModelDefinition {
        switch (modelId) {
            case 'bge-small-tauri':
                return {
                    id: 'bge-small-tauri',
                    name: 'BGE Small EN v1.5 (Tauri Native)',
                    provider: 'local' as any, // Registry compatibility
                    dimensions: 384,
                    maxTokens: 512,
                    speed: 'fast',
                    quality: 'high',
                    costPer1kTokens: 0,
                    description: 'BGE Small via Tauri native embedding (embed-anything)',
                };
            case 'modernbert-tauri':
                return {
                    id: 'modernbert-tauri',
                    name: 'ModernBERT Base (Tauri Native)',
                    provider: 'local' as any,
                    dimensions: 768,
                    maxTokens: 8192,
                    speed: 'medium',
                    quality: 'high',
                    costPer1kTokens: 0,
                    description: 'ModernBERT via Tauri native embedding (embed-anything)',
                };
            default:
                return {
                    id: modelId,
                    name: `${modelId} (Tauri)`,
                    provider: 'local' as any,
                    dimensions: 384,
                    maxTokens: 512,
                    speed: 'fast',
                    quality: 'medium',
                    costPer1kTokens: 0,
                    description: 'Tauri native embedding model',
                };
        }
    }

    /**
     * Initialize the provider - tells Rust to load the model
     */
    async initialize(): Promise<void> {
        if (this.initialized) return;

        console.log(`[TauriEmbeddingProvider] Initializing model: ${this.name}`);

        try {
            // Tell Rust to initialize the embedding model
            const result = await invoke<{ dimensions: number; model_id: string }>('rag_init_embedder', {
                modelId: this.modelId,
            });

            this.dimensions = result.dimensions;
            this.initialized = true;
            console.log(`[TauriEmbeddingProvider] ✓ Model ready: ${this.name} (${this.dimensions}d)`);
        } catch (error) {
            console.error('[TauriEmbeddingProvider] Failed to initialize:', error);
            throw error;
        }
    }

    /**
     * Check if the provider is ready
     */
    isReady(): boolean {
        return this.initialized;
    }

    /**
     * Generate embeddings for text(s) via Tauri IPC
     */
    async embed(texts: string | string[]): Promise<number[][]> {
        if (!this.isReady()) {
            throw new Error('TauriEmbeddingProvider not initialized');
        }

        const inputTexts = Array.isArray(texts) ? texts : [texts];

        if (inputTexts.length === 0) {
            return [];
        }

        try {
            const response = await invoke<EmbedResponse>('rag_embed', {
                texts: inputTexts,
            });

            return response.embeddings;
        } catch (error) {
            console.error('[TauriEmbeddingProvider] Embed failed:', error);
            throw error;
        }
    }

    /**
     * Get model information
     */
    getModelInfo(): EmbeddingModelDefinition {
        // Update dimensions from actual model
        return {
            ...this.modelDef,
            dimensions: this.dimensions,
        };
    }

    /**
     * Cleanup - no-op for Tauri (Rust manages lifecycle)
     */
    async dispose(): Promise<void> {
        this.initialized = false;
    }
}
