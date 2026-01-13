import type { ModelId } from '@/lib/llm/types';

export interface AppSettings {
    llm: LLMSettings;
    embeddings: EmbeddingSettings;
    ui: UISettings;
    appearance: AppearanceSettings;
}

export interface LLMSettings {
    // API Keys
    geminiApiKey?: string;
    openrouterApiKey?: string;

    // Model Selection
    defaultModel: ModelId;
    extractorModel: ModelId;
    agentModel: ModelId;

    // Generation Params
    defaultTemperature: number;
    defaultMaxTokens: number;
}

export interface EmbeddingSettings {
    defaultModel: string; // 'modernbert-base', 'gemini-embedding-004', etc.
    cacheEmbeddings: boolean;
    batchSize: number;
}

export interface AppearanceSettings {
    folderViewTheme: 'default' | 'structured' | 'minimal';
}

export interface UISettings {
    theme: 'light' | 'dark' | 'system';
    sidebarCollapsed: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
    llm: {
        defaultModel: 'gemini-2.5-flash',
        extractorModel: 'nvidia/nemotron-3-nano-30b-a3b:free',
        agentModel: 'gemini-2.5-flash',
        defaultTemperature: 0.7,
        defaultMaxTokens: 2048,
    },
    embeddings: {
        defaultModel: 'mongodb-leaf', // MDBR Leaf - fastest local model
        cacheEmbeddings: true,
        batchSize: 32,
    },
    ui: {
        theme: 'system',
        sidebarCollapsed: false,
    },
    appearance: {
        folderViewTheme: 'structured', // Defaulting to VS Code style per user preference
    }
};

