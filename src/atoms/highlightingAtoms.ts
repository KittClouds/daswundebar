/**
 * Highlighting Mode Settings
 * 
 * Controls how entities are decorated in the editor.
 * 
 * Modes:
 * - clean: Minimal (default) - only shows implicit underlines, reveals on click
 * - vivid: Full colorful highlighting - all entities always visible with colors
 * - focus: Only highlights selected entity types
 * - off: No highlighting at all
 */

import { atom } from 'jotai';
import { atomWithStorage } from 'jotai/utils';
import type { EntityKind } from '@/lib/types/entityTypes';

export type HighlightMode = 'clean' | 'vivid' | 'focus' | 'off';

export interface HighlightSettings {
    /** Current highlighting mode */
    mode: HighlightMode;
    /** Entity kinds to highlight in Focus mode (multiple selection) */
    focusEntityKinds: EntityKind[];
    /** Whether to show wikilink decorations */
    showWikilinks: boolean;
    /** Whether to show tag decorations */
    showTags: boolean;
    /** Whether to show @mention decorations */
    showMentions: boolean;
    /** Whether to show temporal expression decorations */
    showTemporal: boolean;
}

/** Default settings - Clean mode as discovered default */
export const DEFAULT_HIGHLIGHT_SETTINGS: HighlightSettings = {
    mode: 'clean',
    focusEntityKinds: [],
    showWikilinks: true,
    showTags: true,
    showMentions: true,
    showTemporal: true,
};

/** 
 * Persisted highlighting settings
 * Uses atomWithStorage for localStorage persistence
 */
export const highlightSettingsAtom = atomWithStorage<HighlightSettings>(
    'highlighting-settings',
    DEFAULT_HIGHLIGHT_SETTINGS
);

/** Derived atom for just the mode */
export const highlightModeAtom = atom(
    (get) => get(highlightSettingsAtom).mode,
    (get, set, mode: HighlightMode) => {
        set(highlightSettingsAtom, { ...get(highlightSettingsAtom), mode });
    }
);

/** Derived atom for focus entity kinds */
export const focusEntityKindsAtom = atom(
    (get) => get(highlightSettingsAtom).focusEntityKinds,
    (get, set, kinds: EntityKind[]) => {
        set(highlightSettingsAtom, { ...get(highlightSettingsAtom), focusEntityKinds: kinds });
    }
);

/** Human-readable mode labels */
export const HIGHLIGHT_MODE_LABELS: Record<HighlightMode, string> = {
    clean: 'Clean',
    vivid: 'Vivid',
    focus: 'Focus',
    off: 'Off',
};

/** Mode descriptions for UI tooltips */
export const HIGHLIGHT_MODE_DESCRIPTIONS: Record<HighlightMode, string> = {
    clean: 'Minimal highlighting - shows entities on interaction',
    vivid: 'Full colorful highlighting - all entities always visible',
    focus: 'Only highlight selected entity types',
    off: 'No entity highlighting',
};

// =============================================================================
// FST-NER Settings
// =============================================================================

/**
 * FST-NER enabled state (persisted)
 * When enabled, runs FST-based entity detection alongside ImplicitCortex
 */
export const fstNerEnabledAtom = atomWithStorage<boolean>(
    'fst-ner-enabled',
    false // Opt-in by default
);

/**
 * FST-NER stats (runtime, not persisted)
 */
export interface FstNerStats {
    gazetteerPatterns: number;
    ruleCount: number;
    lastScanMs?: number;
}

export const fstNerStatsAtom = atom<FstNerStats>({
    gazetteerPatterns: 0,
    ruleCount: 0,
});

