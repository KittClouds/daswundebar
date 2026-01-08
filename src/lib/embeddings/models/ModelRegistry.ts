export type EmbeddingProvider = 'local' | 'gemini' | 'huggingface' | 'rust' | 'tauri';

export interface EmbeddingModelDefinition {
    id: string;
    name: string;
    provider: EmbeddingProvider;
    dimensions: number;
    maxTokens: number;

    // Performance characteristics
    speed: 'fast' | 'medium' | 'slow';
    quality: 'high' | 'medium' | 'low';

    // Cost
    costPer1kTokens: number; // 0 for local models

    // Local model info (if provider === 'local')
    localModel?: {
        modelId: string; // HuggingFace model ID
        quantization?: 'q8' | 'q4' | 'fp16';
        memoryMB: number; // Estimated memory usage
    };

    description: string;
}

export class EmbeddingModelRegistry {
    private static models: Map<string, EmbeddingModelDefinition> = new Map([
        // ===== LOCAL MODELS (In-Browser) =====
        // MDBR Leaf is first - recommended default (fastest, smallest, high quality)
        [
            'mongodb-leaf',
            {
                id: 'mongodb-leaf',
                name: 'MDBR Leaf (256d)',
                provider: 'local',
                dimensions: 256,
                maxTokens: 512,
                speed: 'fast',
                quality: 'high',
                costPer1kTokens: 0,
                localModel: {
                    modelId: 'MongoDB/mxbai-embed-large-v1',
                    quantization: 'q8',
                    memoryMB: 50,
                },
                description: 'MDBR Leaf - Fastest, smallest, excellent quality. Recommended.',
            },
        ],
        [
            'modernbert-base',
            {
                id: 'modernbert-base',
                name: 'MiniLM-L6-v2 (Local)',
                provider: 'local',
                dimensions: 384,
                maxTokens: 512,
                speed: 'fast',
                quality: 'high',
                costPer1kTokens: 0,
                localModel: {
                    modelId: 'Xenova/all-MiniLM-L6-v2',
                    quantization: 'q8',
                    memoryMB: 90,
                },
                description: 'all-MiniLM-L6-v2 via Transformers.js (ONNX converted).',
            },
        ],

        // ===== CLOUD MODELS (API) =====
        [
            'gemini-embedding-004',
            {
                id: 'gemini-embedding-004',
                name: 'Gemini Text Embedding 004',
                provider: 'gemini',
                dimensions: 768,
                maxTokens: 2048,
                speed: 'fast',
                quality: 'high',
                costPer1kTokens: 0.00001,
                description: 'Google Gemini embeddings. High quality, cloud-based.',
            },
        ],
        [
            'text-embedding-3-small',
            {
                id: 'text-embedding-3-small',
                name: 'OpenAI Text Embedding 3 Small',
                provider: 'huggingface', // Via OpenRouter/Mastra
                dimensions: 1536,
                maxTokens: 8191,
                speed: 'fast',
                quality: 'high',
                costPer1kTokens: 0.00002,
                description: 'OpenAI embeddings via OpenRouter.',
            },
        ],

        // ===== RUST/WASM MODELS (A/B Testing) =====
        [
            'bge-small-rust',
            {
                id: 'bge-small-rust',
                name: 'BGE Small EN v1.5 (Rust)',
                provider: 'rust' as EmbeddingProvider,
                dimensions: 384,
                maxTokens: 512,
                speed: 'fast',
                quality: 'high',
                costPer1kTokens: 0,
                localModel: {
                    modelId: 'BAAI/bge-small-en-v1.5',
                    memoryMB: 130,
                },
                description: 'BGE Small via Rust/WASM ONNX (A/B test alternative to TS)',
            },
        ],
        [
            'modernbert-rust',
            {
                id: 'modernbert-rust',
                name: 'ModernBERT Base (Rust)',
                provider: 'rust' as EmbeddingProvider,
                dimensions: 768,
                maxTokens: 8192,
                speed: 'medium',
                quality: 'high',
                costPer1kTokens: 0,
                localModel: {
                    modelId: 'nomic-ai/modernbert-embed-base',
                    memoryMB: 350,
                },
                description: 'ModernBERT via Rust/WASM ONNX (A/B test alternative to TS)',
            },
        ],

        // ===== TAURI NATIVE MODELS (embed-anything) =====
        [
            'bge-small-tauri',
            {
                id: 'bge-small-tauri',
                name: 'BGE Small EN v1.5 (Tauri Native)',
                provider: 'tauri' as EmbeddingProvider,
                dimensions: 384,
                maxTokens: 512,
                speed: 'fast',
                quality: 'high',
                costPer1kTokens: 0,
                localModel: {
                    modelId: 'BAAI/bge-small-en-v1.5',
                    memoryMB: 130,
                },
                description: 'BGE Small via Tauri native embed-anything (recommended)',
            },
        ],
        [
            'modernbert-tauri',
            {
                id: 'modernbert-tauri',
                name: 'ModernBERT Base (Tauri Native)',
                provider: 'tauri' as EmbeddingProvider,
                dimensions: 768,
                maxTokens: 8192,
                speed: 'medium',
                quality: 'high',
                costPer1kTokens: 0,
                localModel: {
                    modelId: 'nomic-ai/modernbert-embed-base',
                    memoryMB: 350,
                },
                description: 'ModernBERT via Tauri native embed-anything',
            },
        ],
    ]);

    static getModel(id: string): EmbeddingModelDefinition | undefined {
        return this.models.get(id);
    }

    static getLocalModels(): EmbeddingModelDefinition[] {
        return Array.from(this.models.values()).filter(m => m.provider === 'local');
    }

    static getRustModels(): EmbeddingModelDefinition[] {
        return Array.from(this.models.values()).filter(m => m.provider === 'rust');
    }

    static getCloudModels(): EmbeddingModelDefinition[] {
        return Array.from(this.models.values()).filter(m => m.provider !== 'local' && m.provider !== 'rust');
    }

    static getByDimension(dim: number): EmbeddingModelDefinition[] {
        return Array.from(this.models.values()).filter(m => m.dimensions === dim);
    }

    static getAllModels(): EmbeddingModelDefinition[] {
        return Array.from(this.models.values());
    }

    static getRecommended(preference: 'speed' | 'quality' | 'privacy'): string {
        switch (preference) {
            case 'speed':
                return 'mongodb-leaf'; // Fastest local
            case 'quality':
                return 'mongodb-leaf'; // High quality AND fast
            case 'privacy':
                return 'mongodb-leaf'; // All local, privacy-first
            default:
                return 'mongodb-leaf';
        }
    }
}
