import type { AppSettings, LLMSettings } from './types';
import { DEFAULT_SETTINGS } from './types';
import type { LLMProvider } from '@/lib/llm/types';

const STORAGE_KEY = 'collaborative-canvas-settings';

/**
 * Manages app settings with localStorage persistence
 * Falls back to environment variables when keys not set
 */
export class SettingsManager {
    private static settings: AppSettings | null = null;

    /**
     * Load settings from localStorage or defaults
     */
    static load(): AppSettings {
        if (this.settings) return this.settings;

        try {
            const stored = localStorage.getItem(STORAGE_KEY);
            if (stored) {
                const parsed = JSON.parse(stored);
                // Merge with defaults to handle new fields
                this.settings = {
                    ...DEFAULT_SETTINGS,
                    ...parsed,
                    llm: { ...DEFAULT_SETTINGS.llm, ...parsed.llm },
                    embeddings: { ...DEFAULT_SETTINGS.embeddings, ...parsed.embeddings },
                    ui: { ...DEFAULT_SETTINGS.ui, ...parsed.ui },
                    appearance: { ...DEFAULT_SETTINGS.appearance, ...parsed.appearance },
                };
                return this.settings!;
            }
        } catch (error) {
            console.warn('Failed to load settings from localStorage:', error);
        }

        // Return defaults with env fallback for API keys
        this.settings = {
            ...DEFAULT_SETTINGS,
            llm: {
                ...DEFAULT_SETTINGS.llm,
                geminiApiKey: import.meta.env.VITE_GEMINI_API_KEY,
                openrouterApiKey: import.meta.env.VITE_OPENROUTER_API_KEY,
            },
        };

        return this.settings;
    }

    /**
     * Save settings to localStorage
     */
    static save(settings: AppSettings): void {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
            this.settings = settings;
            window.dispatchEvent(new CustomEvent('settings-changed', { detail: settings }));
        } catch (error) {
            console.error('Failed to save settings to localStorage:', error);
            throw new Error('Failed to save settings');
        }
    }

    /**
     * Update partial settings
     */
    static update(partial: Partial<AppSettings>): void {
        const current = this.load();
        const updated = {
            ...current,
            ...partial,
            llm: { ...current.llm, ...partial.llm },
            embeddings: { ...current.embeddings, ...partial.embeddings },
            ui: { ...current.ui, ...partial.ui },
            appearance: { ...current.appearance, ...partial.appearance },
        };
        this.save(updated);
    }

    /**
     * Get LLM settings
     */
    static getLLMSettings(): LLMSettings {
        return this.load().llm;
    }

    /**
     * Update LLM settings
     */
    static updateLLMSettings(llm: Partial<LLMSettings>): void {
        const current = this.load();
        this.save({
            ...current,
            llm: { ...current.llm, ...llm },
        });
    }

    /**
     * Get API key for provider (with env fallback)
     */
    static getApiKey(provider: LLMProvider): string | undefined {
        const settings = this.load();

        if (provider === 'gemini') {
            return settings.llm.geminiApiKey || import.meta.env.VITE_GEMINI_API_KEY;
        }

        if (provider === 'openrouter') {
            return settings.llm.openrouterApiKey || import.meta.env.VITE_OPENROUTER_API_KEY;
        }

        return undefined;
    }

    /**
     * Check if API key is configured
     */
    static hasApiKey(provider: LLMProvider): boolean {
        const key = this.getApiKey(provider);
        return !!key && key.length > 0;
    }

    /**
     * Reset to defaults
     */
    static reset(): void {
        localStorage.removeItem(STORAGE_KEY);
        this.settings = null;
    }

    /**
     * Clear cached settings (force reload on next access)
     */
    static clearCache(): void {
        this.settings = null;
    }
}
