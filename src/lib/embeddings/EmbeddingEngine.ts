import type { IEmbeddingProvider } from './providers/types';
import { LocalEmbeddingProvider } from './providers/LocalEmbeddingProvider';
import { GeminiEmbeddingProvider } from './providers/GeminiEmbeddingProvider';
import { EmbeddingModelRegistry } from './models/ModelRegistry';
import { SettingsManager } from '@/lib/settings/SettingsManager';

/**
 * Unified Embedding Engine
 * 
 * Provides embeddings from local models (Transformers.js), cloud APIs (Gemini, HuggingFace),
 * or Rust/WASM models (via kittcore) for A/B testing.
 */
export class EmbeddingEngine {
    private static providers: Map<string, IEmbeddingProvider> = new Map();
    private static currentProvider: IEmbeddingProvider | null = null;

    /**
     * Initialize embedding engine with configured model
     */
    static async initialize(modelId?: string): Promise<void> {
        const settings = SettingsManager.load();
        // Use optional chaining and default to modernbert-base
        const targetModelId = modelId || (settings as any).embeddings?.defaultModel || 'mongodb-leaf';

        // Check if already initialized with this model
        if (this.currentProvider?.getModelInfo().id === targetModelId) {
            return;
        }

        // Get or create provider
        let provider = this.providers.get(targetModelId);

        if (!provider) {
            const model = EmbeddingModelRegistry.getModel(targetModelId);
            if (!model) {
                throw new Error(`Unknown embedding model: ${targetModelId}`);
            }

            // Create appropriate provider
            switch (model.provider) {
                case 'local':
                    provider = new LocalEmbeddingProvider(targetModelId);
                    break;
                case 'gemini':
                    provider = new GeminiEmbeddingProvider(targetModelId);
                    break;
                case 'rust': {
                    // Lazy import to avoid loading WASM unless needed
                    const { RustEmbeddingProvider } = await import('./providers/RustEmbeddingProvider');
                    provider = new RustEmbeddingProvider(targetModelId);
                    break;
                }
                case 'tauri': {
                    // Lazy import Tauri provider
                    const { TauriEmbeddingProvider } = await import('./providers/TauriEmbeddingProvider');
                    provider = new TauriEmbeddingProvider(targetModelId);
                    break;
                }
                default:
                    throw new Error(`Unsupported provider: ${model.provider}`);
            }

            this.providers.set(targetModelId, provider);
        }

        // Initialize provider
        await provider.initialize();
        this.currentProvider = provider;
    }

    /**
     * Generate embeddings for text(s)
     */
    static async embed(texts: string | string[]): Promise<number[][]> {
        if (!this.currentProvider) {
            await this.initialize(); // Auto-initialize with defaults
        }

        return this.currentProvider!.embed(texts);
    }

    /**
     * Get current model info
     */
    static getCurrentModel(): string {
        return this.currentProvider?.getModelInfo().id || 'none';
    }

    /**
     * Get current model dimensions
     */
    static getDimensions(): number {
        if (!this.currentProvider) {
            // Try to initialize first
            throw new Error('Embedding engine not initialized');
        }
        return this.currentProvider.getModelInfo().dimensions;
    }

    /**
     * Check if engine is ready
     */
    static isReady(): boolean {
        return this.currentProvider?.isReady() ?? false;
    }

    /**
     * Switch to different model
     */
    static async switchModel(modelId: string): Promise<void> {
        await this.initialize(modelId);
    }

    /**
     * Get the active provider type for hybrid pipeline routing
     * 
     * Returns:
     * - 'rust': Embeddings are done in Rust/WASM (send raw text to worker)
     * - 'tauri': Embeddings are done via Tauri native (send raw text to Rust)
     * - 'local': Embeddings are done in TypeScript (send vectors to worker)
     * - 'cloud': Embeddings are done via cloud API (send vectors to worker)
     */
    static getActiveProviderType(): 'rust' | 'tauri' | 'local' | 'cloud' | 'none' {
        if (!this.currentProvider) {
            return 'none';
        }

        const modelInfo = this.currentProvider.getModelInfo();

        // Check provider type from model definition
        if (modelInfo.provider === 'rust') {
            return 'rust';
        } else if (modelInfo.provider === 'tauri') {
            return 'tauri';
        } else if (modelInfo.provider === 'local') {
            return 'local';
        } else if (modelInfo.provider === 'gemini' || modelInfo.provider === 'huggingface') {
            return 'cloud';
        }

        return 'local'; // Default to local for safety
    }

    /**
     * Get current model dimensions (safe version that returns 0 if not initialized)
     */
    static getDimensionsSafe(): number {
        return this.currentProvider?.getModelInfo().dimensions ?? 0;
    }

    /**
     * Cleanup
     */
    static async dispose(): Promise<void> {
        for (const provider of this.providers.values()) {
            await provider.dispose();
        }
        this.providers.clear();
        this.currentProvider = null;
    }
}
