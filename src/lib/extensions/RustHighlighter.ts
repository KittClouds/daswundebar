/**
 * RustHighlighter - Tauri Native Syntax Highlighting Extension
 * 
 * PARALLEL to KittHighlighter for A/B testing.
 * Uses Tauri IPC (conductor_scan) instead of WASM.
 * 
 * Single extension that combines:
 * - Rust Native implicit entity detection (via TauriScanner)
 * - Rust Native temporal expression detection
 * - Pattern registry (wikilinks, tags, mentions)
 * - NER entity suggestions
 * - Widget mode (click-to-edit)
 * - Bidirectional link tracking
 * 
 * Supports 4 highlighting modes: clean, vivid, focus, off
 */

import { Extension } from '@tiptap/core';
import { Plugin, PluginKey, Selection } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { Node as ProseMirrorNode } from '@tiptap/pm/model';
import type { EditorView } from '@tiptap/pm/view';

// Tauri Native bridge (replaces WASM)
import {
    tauriScanner,
    isTauri,
    type ConductorScanResult,
    type ImplicitMention,
    type TemporalMention
} from '@/lib/tauri/bridge';

// NER Bridge for suggestions
import {
    type NerSuggestion,
    suggestionToCharOffsets,
    fstNerScan,
    fstMatchToCharOffsets,
    type FstMatch,
    type FstScanResult,
} from '@/lib/tauri/ner-bridge';

// Types
import { EntityKind, ENTITY_KINDS, ENTITY_COLORS } from '@/lib/types/entityTypes';
import type { NEREntity } from '@/lib/extraction';
import type { HighlightMode } from '@/atoms/highlightingAtoms';
import type { EntityMentionEvent, PositionType } from '@/lib/cozo-stubs/types';

// Pattern registry (shared with KittHighlighter)
import { patternRegistry, type PatternDefinition, type RefKind } from '@/lib/refs';

// Event queue for link tracking
import { mentionEventQueue } from '@/lib/scanner/mention-event-queue';

// Phase 3: Cached decoration spans from CozoDB
import { fetchDecorationSpans, type DecorationSpanRecord } from '@/lib/Scanner/decoration-cache';

// ==================== OPTIONS ====================

export interface RustHighlighterOptions {
    // Click handlers
    onWikilinkClick?: (title: string) => void;
    checkWikilinkExists?: (title: string) => boolean;
    onTemporalClick?: (temporal: string) => void;
    onBacklinkClick?: (title: string) => void;
    onImplicitClick?: (entityId: string, entityLabel: string) => void;
    onNEREntityClick?: (entity: NEREntity) => void;
    onRefClick?: (kind: RefKind, target: string, payload?: any) => void;

    // NER suggestion handlers (Rust backend)
    nerSuggestions?: NerSuggestion[] | (() => NerSuggestion[]);
    onNerSuggestionClick?: (suggestion: NerSuggestion) => void;
    onNerSuggestionAccept?: (suggestionId: string) => Promise<boolean>;
    onNerSuggestionReject?: (suggestionId: string) => Promise<boolean>;

    // Legacy NER entities getter (deprecated, use nerSuggestions)
    nerEntities?: NEREntity[] | (() => NEREntity[]);

    // Note context
    currentNoteId?: string | (() => string | undefined);

    // Feature flags
    useWidgetMode?: boolean;
    enableLinkTracking?: boolean;
    logPerformance?: boolean;

    // Highlighting mode getters (reactive)
    getHighlightMode?: () => HighlightMode;
    getFocusEntityKinds?: () => EntityKind[];

    // FST-NER toggle (reactive)
    getFstNerEnabled?: () => boolean;
}

// ==================== HELPERS ====================

const rustPluginKey = new PluginKey('rust-highlighter');
const rustLinkTrackerKey = new PluginKey('rust-link-tracker');

function resolveNoteId(noteId: string | (() => string | undefined) | undefined): string {
    if (typeof noteId === 'function') {
        return noteId() || 'unknown';
    }
    return noteId || 'unknown';
}

// Content hash for caching
function computeContentHash(text: string): string {
    let hash = 0;
    for (let i = 0; i < text.length; i++) {
        hash = ((hash << 5) - hash + text.charCodeAt(i)) | 0;
    }
    return hash.toString(16);
}

// Phase 3: Last cached spans per note (from CozoDB background scan)
const cachedSpansCache = new Map<string, { hash: string; spans: DecorationSpanRecord[] }>();

// ==================== DEBOUNCE ====================

function debounce<T extends (...args: Parameters<T>) => void>(
    fn: T,
    ms: number
): T & { cancel: () => void } {
    let timeoutId: ReturnType<typeof setTimeout> | null = null;
    const debounced = (...args: Parameters<T>) => {
        if (timeoutId) clearTimeout(timeoutId);
        timeoutId = setTimeout(() => fn(...args), ms);
    };
    debounced.cancel = () => {
        if (timeoutId) clearTimeout(timeoutId);
        timeoutId = null;
    };
    return debounced as T & { cancel: () => void };
}

// ==================== RANGE OVERLAP DETECTION ====================

// Range as [start, end) tuple for efficient binary search
type Range = [number, number];

function rangesOverlap(ranges: Range[], start: number, end: number): boolean {
    let lo = 0, hi = ranges.length;
    while (lo < hi) {
        const mid = (lo + hi) >>> 1;
        if (ranges[mid][1] <= start) lo = mid + 1;
        else hi = mid;
    }
    return lo < ranges.length && ranges[lo][0] < end;
}

function insertRange(ranges: Range[], start: number, end: number): void {
    let lo = 0, hi = ranges.length;
    while (lo < hi) {
        const mid = (lo + hi) >>> 1;
        if (ranges[mid][0] < start) lo = mid + 1;
        else hi = mid;
    }
    ranges.splice(lo, 0, [start, end]);
}

// ==================== EDITING DETECTION ====================

function isEditing(selection: { from: number; to: number }, range: { from: number; to: number }): boolean {
    return (
        (selection.from >= range.from && selection.from <= range.to) ||
        (selection.to >= range.from && selection.to <= range.to) ||
        (selection.from <= range.from && selection.to >= range.to)
    );
}

function crossesDecorationBoundary(
    decorations: DecorationSet,
    oldSel: { from: number; to: number },
    newSel: { from: number; to: number }
): boolean {
    const oldFrom = decorations.find(oldSel.from, oldSel.from);
    const oldTo = decorations.find(oldSel.to, oldSel.to);
    const newFrom = decorations.find(newSel.from, newSel.from);
    const newTo = decorations.find(newSel.to, newSel.to);
    return oldFrom.length !== newFrom.length || oldTo.length !== newTo.length;
}

// ==================== WIDGET CREATION ====================

function createPatternWidget(
    label: string,
    kind: RefKind,
    fullMatch: string,
    color: string,
    backgroundColor: string,
    extraClasses: string = ''
): HTMLElement {
    const span = document.createElement('span');
    span.className = `ref-widget ref-${kind} ${extraClasses}`;
    span.textContent = label;
    span.style.cssText = `
    background-color: ${backgroundColor};
    color: ${color};
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 500;
    font-size: 0.875em;
    cursor: text;
    display: inline-block;
    position: relative;
  `;
    span.setAttribute('data-ref-kind', kind);
    span.setAttribute('data-ref-label', label);
    span.setAttribute('data-ref-full', fullMatch);
    span.setAttribute('contenteditable', 'false');
    span.setAttribute('data-editable-widget', 'true');
    return span;
}

// ==================== TEXT EXTRACTION ====================

function extractText(doc: ProseMirrorNode): string {
    let text = '';
    doc.descendants((node) => {
        if (node.isText && node.text) {
            text += node.text;
        } else if (node.isBlock) {
            text += '\n';
        }
    });
    return text;
}

function buildPositionMap(doc: ProseMirrorNode): number[] {
    const map: number[] = [];
    let textIndex = 0;

    doc.descendants((node, pos) => {
        if (node.isText && node.text) {
            for (let i = 0; i < node.text.length; i++) {
                map[textIndex++] = pos + i;
            }
        } else if (node.isBlock) {
            map[textIndex++] = pos;
        }
    });

    return map;
}

// ==================== DECORATION BUILDERS ====================

/**
 * Build Tauri Native decorations (implicit entities + temporal)
 * Uses ConductorScanResult from Tauri IPC
 */
function buildTauriDecorations(
    result: ConductorScanResult,
    positionMap: number[],
    options: RustHighlighterOptions,
    processedRanges: Range[]
): Decoration[] {
    const decorations: Decoration[] = [];
    const mode = options.getHighlightMode?.() ?? 'clean';
    const focusKinds = options.getFocusEntityKinds?.() ?? [];

    if (mode === 'off' || mode === 'clean') return decorations;

    // Process implicit mentions
    for (const mention of result.implicit) {
        const from = positionMap[mention.start];
        const to = positionMap[mention.end - 1] !== undefined
            ? positionMap[mention.end - 1] + 1
            : positionMap[mention.start] + (mention.end - mention.start);

        if (from === undefined || to === undefined) continue;

        // Focus mode filter
        if (mode === 'focus') {
            const entityKind = mention.entity_kind as EntityKind;
            if (focusKinds.length > 0 && !focusKinds.includes(entityKind)) {
                continue;
            }
        }

        // Check overlap
        if (rangesOverlap(processedRanges, mention.start, mention.end)) {
            continue;
        }
        insertRange(processedRanges, mention.start, mention.end);

        const entityKind = mention.entity_kind || 'CHARACTER';
        const varName = `--entity-${entityKind.toLowerCase().replace('_', '-')}`;
        const color = `hsl(var(${varName}))`;
        const isVivid = mode === 'vivid';
        const borderStyle = isVivid ? 'solid' : (mention.is_alias_match ? 'dotted' : 'solid');
        const bgOpacity = isVivid ? 0.15 : 0.1;

        const style = `
          background-color: hsl(var(${varName}) / ${bgOpacity}); 
          color: ${color}; 
          padding: 0px 2px; 
          border-bottom: 2px ${borderStyle} ${color}; 
          cursor: help;
        `;
        const className = isVivid ? 'kitt-implicit vivid' : 'kitt-implicit';

        decorations.push(
            Decoration.inline(from, to, {
                class: className,
                style,
                'data-entity-id': mention.entity_id || '',
                'data-entity-kind': entityKind,
                'data-entity-label': mention.entity_label,
                'data-confidence': '1.0',
                'title': `${entityKind}: ${mention.entity_label}${mention.is_alias_match ? ' (alias)' : ''}`,
            }, { inclusiveStart: false, inclusiveEnd: false })
        );
    }

    // Process temporal mentions
    for (const temporal of result.temporal || []) {
        const from = positionMap[temporal.start];
        const to = positionMap[temporal.end - 1] !== undefined
            ? positionMap[temporal.end - 1] + 1
            : positionMap[temporal.start] + (temporal.end - temporal.start);

        if (from === undefined || to === undefined) continue;

        if (rangesOverlap(processedRanges, temporal.start, temporal.end)) {
            continue;
        }
        insertRange(processedRanges, temporal.start, temporal.end);

        const isVivid = mode === 'vivid';
        const style = `
          background-color: hsl(var(--warning) / ${isVivid ? 0.2 : 0.15});
          color: hsl(var(--warning));
          padding: 0px 2px;
          border-bottom: 2px ${isVivid ? 'solid' : 'dashed'} hsl(var(--warning));
          cursor: pointer;
        `;
        const className = isVivid ? 'kitt-temporal vivid' : 'kitt-temporal';

        decorations.push(
            Decoration.inline(from, to, {
                class: className,
                style,
                'data-temporal': temporal.text,
                'data-temporal-kind': temporal.kind,
                'title': `Temporal: ${temporal.text}`,
            }, { inclusiveStart: false, inclusiveEnd: false })
        );
    }

    return decorations;
}

/**
 * Build pattern registry decorations result
 */
interface PatternDecorationsResult {
    decorations: Decoration[];
    /** Document-level ranges covered by pattern matches */
    coveredRanges: Range[];
}

/**
 * Build pattern registry decorations (wikilinks, tags, mentions)
 * Mode-aware: respects highlighting mode settings
 * 
 * Returns both decorations AND the ranges they cover (for FST exclusion)
 */
function buildPatternDecorations(
    doc: ProseMirrorNode,
    options: RustHighlighterOptions,
    selection: { from: number; to: number } | undefined
): PatternDecorationsResult {
    const decorations: Decoration[] = [];
    const coveredRanges: Range[] = [];
    const useWidgets = options.useWidgetMode ?? false;
    const patterns = patternRegistry.getActivePatterns();
    const mode = options.getHighlightMode?.() ?? 'vivid';

    if (mode === 'off') {
        return { decorations, coveredRanges };
    }

    doc.descendants((node, pos) => {
        if (!node.isText || !node.text) return;

        const text = node.text;
        const processedRanges: Range[] = [];

        for (const pattern of patterns) {
            if (!pattern.enabled) continue;

            const regex = patternRegistry.getCompiledPattern(pattern.id);
            let match: RegExpExecArray | null;
            regex.lastIndex = 0;

            while ((match = regex.exec(text)) !== null) {
                if (match.index === regex.lastIndex) {
                    regex.lastIndex++;
                }

                const fullMatch = match[0];
                const from = pos + match.index;
                const to = from + fullMatch.length;

                if (rangesOverlap(processedRanges, match.index, match.index + fullMatch.length)) continue;
                insertRange(processedRanges, match.index, match.index + fullMatch.length);

                // Track doc-level range for FST exclusion
                insertRange(coveredRanges, from, to);

                const isCurrentlyEditing = selection ? isEditing(selection, { from, to }) : false;

                // Extract label/target
                let label = fullMatch;
                let target = fullMatch;

                if (pattern.captures) {
                    const labelKeys = ['label', 'displayText', 'displayName', 'username', 'tagName', 'word'];
                    const targetKeys = ['target', 'id', 'username', 'tagName'];

                    const getCapture = (keys: string[]) => {
                        for (const key of keys) {
                            if (pattern.captures[key]) {
                                const groupIndex = pattern.captures[key].group;
                                if (match![groupIndex]) return match![groupIndex];
                            }
                        }
                        return null;
                    };

                    label = getCapture(labelKeys) || fullMatch;
                    target = getCapture(targetKeys) || label;
                }

                // Existence check for wikilinks
                let exists = true;
                if (pattern.kind === 'wikilink' && options.checkWikilinkExists) {
                    exists = options.checkWikilinkExists(target);
                }

                // Colors
                let color = pattern.rendering?.color || 'hsl(var(--primary))';
                let bgColor = pattern.rendering?.backgroundColor || 'hsl(var(--primary) / 0.15)';

                if (pattern.kind === 'entity') {
                    const entityKind = match[1] as EntityKind;
                    if (entityKind && ENTITY_COLORS[entityKind]) {
                        const varName = `--entity-${entityKind.toLowerCase().replace('_', '-')}`;
                        color = `hsl(var(${varName}))`;
                        bgColor = `hsl(var(${varName}) / 0.15)`;
                    }
                }

                if (pattern.kind === 'wikilink') {
                    color = exists ? 'hsl(var(--primary))' : 'hsl(var(--destructive))';
                    bgColor = exists ? 'hsl(var(--primary) / 0.15)' : 'hsl(var(--destructive) / 0.15)';
                }

                // Clean mode: minimal styling
                if (mode === 'clean' && !isCurrentlyEditing) {
                    color = 'inherit';
                    bgColor = 'transparent';
                }

                const shouldRenderWidget = (useWidgets || pattern.rendering?.widgetMode) && !isCurrentlyEditing;

                if (shouldRenderWidget) {
                    const widget = createPatternWidget(
                        label,
                        pattern.kind,
                        fullMatch,
                        color,
                        bgColor,
                        pattern.kind === 'wikilink' ? (exists ? 'wikilink-exists' : 'wikilink-broken') : ''
                    );

                    decorations.push(
                        Decoration.widget(from, widget, {
                            side: -1,
                            key: `${pattern.id}-${from}-${fullMatch}-${mode}`,
                        })
                    );

                    decorations.push(
                        Decoration.inline(from, to, {
                            class: 'ref-hidden',
                            style: 'display: none;',
                        })
                    );
                } else {
                    if (mode === 'clean' && label !== fullMatch) {
                        const labelIndex = fullMatch.indexOf(label);
                        if (labelIndex !== -1) {
                            const labelFrom = from + labelIndex;
                            const labelTo = labelFrom + label.length;

                            decorations.push(
                                Decoration.inline(labelFrom, labelTo, {
                                    class: `ref-highlight ref-${pattern.kind} clean-mode-label`,
                                    style: `
                                        background-color: ${bgColor};
                                        color: ${color};
                                        padding: 2px 4px;
                                        border-radius: 3px;
                                    `,
                                    'data-ref-kind': pattern.kind,
                                    'data-ref-target': target,
                                }, { inclusiveStart: false, inclusiveEnd: false })
                            );
                        }
                    } else {
                        let style = `
                            background-color: ${bgColor};
                            color: ${color};
                            padding: 2px 6px;
                            border-radius: 4px;
                            font-weight: 500;
                            font-size: 0.875em;
                            cursor: pointer;
                        `;

                        if (pattern.kind === 'wikilink') {
                            style += `text-decoration: underline; text-decoration-style: ${exists ? 'dotted' : 'dashed'};`;
                        }

                        decorations.push(
                            Decoration.inline(from, to, {
                                class: `ref-highlight ref-${pattern.kind} ${pattern.kind === 'wikilink' ? (exists ? 'wikilink-editing' : 'wikilink-broken wikilink-editing') : ''}`,
                                style,
                                'data-ref-kind': pattern.kind,
                                'data-ref-target': target,
                                'data-ref-exists': exists.toString(),
                            }, { inclusiveStart: false, inclusiveEnd: false })
                        );
                    }
                }
            }
        }
    });

    return { decorations, coveredRanges };
}

/**
 * Build NER suggestion decorations
 */
function buildNERDecorations(
    doc: ProseMirrorNode,
    options: RustHighlighterOptions,
    processedRanges: Range[]
): Decoration[] {
    const decorations: Decoration[] = [];
    const nerEntities = typeof options.nerEntities === 'function'
        ? options.nerEntities()
        : options.nerEntities || [];

    if (nerEntities.length === 0) return decorations;

    doc.descendants((node, pos) => {
        if (!node.isText || !node.text) return;

        const text = node.text;

        for (const entity of nerEntities) {
            const entityStart = entity.start;
            const entityEnd = entity.end;
            const nodeStart = pos;
            const nodeEnd = pos + text.length;

            if (entityEnd <= nodeStart || entityStart >= nodeEnd) continue;

            const relativeStart = Math.max(0, entityStart - nodeStart);
            const relativeEnd = Math.min(text.length, entityEnd - nodeStart);
            const from = pos + relativeStart;
            const to = pos + relativeEnd;

            if (rangesOverlap(processedRanges, relativeStart, relativeEnd)) continue;
            insertRange(processedRanges, relativeStart, relativeEnd);

            decorations.push(
                Decoration.inline(from, to, {
                    class: 'ner-suggestion',
                    style: 'background-color: #fbbf2415; border-bottom: 2px dashed #fbbf24; padding: 0px 2px; cursor: pointer;',
                    'data-ner-entity': entity.word,
                    'data-ner-type': entity.entity_type,
                    'data-ner-start': entity.start.toString(),
                    'data-ner-end': entity.end.toString(),
                }, { inclusiveStart: false, inclusiveEnd: false })
            );
        }
    });

    return decorations;
}

/**
 * Build decorations for Rust NER suggestions (from Tauri backend)
 * 
 * These are styled with amber dashed underlines and include suggestion ID
 * for accept/reject actions.
 */
function buildNerSuggestionDecorations(
    doc: ProseMirrorNode,
    options: RustHighlighterOptions,
    processedRanges: Range[]
): Decoration[] {
    const decorations: Decoration[] = [];

    try {
        const mode = options.getHighlightMode?.() ?? 'vivid';

        // Skip in off or clean mode
        if (mode === 'off' || mode === 'clean') return decorations;

        // Safely get suggestions - handle both array and getter function
        let nerSuggestions: NerSuggestion[] = [];
        try {
            const raw = typeof options.nerSuggestions === 'function'
                ? options.nerSuggestions()
                : options.nerSuggestions;
            nerSuggestions = Array.isArray(raw) ? raw : [];
        } catch {
            // Getter threw - return empty
            return decorations;
        }

        if (nerSuggestions.length === 0) return decorations;

        // Extract full document text for byte-to-char offset conversion
        const fullText = extractText(doc);
        if (!fullText || fullText.length === 0) return decorations;

        const positionMap = buildPositionMap(doc);
        if (!positionMap || positionMap.length === 0) return decorations;

        for (const suggestion of nerSuggestions) {
            if (!suggestion || !suggestion.id) continue;

            // Convert byte offsets to character offsets
            const charOffsets = suggestionToCharOffsets(suggestion, fullText);

            // Validate offsets are within bounds
            if (charOffsets.start < 0 || charOffsets.end <= charOffsets.start) continue;
            if (charOffsets.start >= fullText.length) continue;
            if (charOffsets.end > fullText.length) continue;

            // Map text indices to ProseMirror positions - with safety
            const from = positionMap[charOffsets.start];
            if (from === undefined || typeof from !== 'number' || isNaN(from)) continue;

            const endIdx = Math.min(charOffsets.end - 1, positionMap.length - 1);
            const toBase = positionMap[endIdx];
            if (toBase === undefined || typeof toBase !== 'number' || isNaN(toBase)) continue;

            const to = toBase + 1;

            // Final validation - positions must be valid for doc
            if (from < 0 || to <= from || to > doc.content.size + 1) continue;

            // Check overlap with already-processed ranges
            if (rangesOverlap(processedRanges, charOffsets.start, charOffsets.end)) continue;
            insertRange(processedRanges, charOffsets.start, charOffsets.end);

            // Confidence-based styling intensity
            const confidencePercent = Math.round((suggestion.confidence || 0) * 100);
            const bgOpacity = mode === 'vivid' ? 0.20 : 0.15;

            // Style: amber dashed underline (matches reference images)
            const style = `
                background-color: hsl(45 93% 47% / ${bgOpacity});
                border-bottom: 2px dashed hsl(45 93% 47%);
                padding: 0px 2px;
                cursor: pointer;
            `;

            const className = mode === 'vivid' ? 'ner-suggestion vivid' : 'ner-suggestion';
            const tooltip = `${suggestion.entity_type || 'entity'}: ${suggestion.text || '?'} (${confidencePercent}% confidence)`;

            decorations.push(
                Decoration.inline(from, to, {
                    class: className,
                    style,
                    // Data attributes for click handlers
                    'data-ner-suggestion-id': suggestion.id,
                    'data-ner-suggestion-text': suggestion.text || '',
                    'data-ner-suggestion-type': suggestion.entity_type || '',
                    'data-ner-suggestion-confidence': String(suggestion.confidence || 0),
                    'data-ner-suggestion-label': suggestion.label || '',
                    'title': tooltip,
                }, { inclusiveStart: false, inclusiveEnd: false })
            );
        }
    } catch (err) {
        console.error('[RustHighlighter] Error building NER suggestion decorations:', err);
    }

    return decorations;
}

// ==================== MAIN DECORATION BUILDER ====================

// Last scan result storage (sync access for decoration building)
let lastScanResult: ConductorScanResult | null = null;

// FST-NER scan result storage
let lastFstResult: FstScanResult | null = null;

/**
 * Build FST-NER decorations
 * Similar to implicit decorations but from the FST pipeline
 */
function buildFstDecorations(
    doc: ProseMirrorNode,
    fstResult: FstScanResult,
    options: RustHighlighterOptions,
    processedRanges: Range[]
): Decoration[] {
    const decorations: Decoration[] = [];
    const mode = options.getHighlightMode?.() || 'clean';

    if (mode === 'off') return decorations;

    const docText = doc.textContent;
    const positionMap = buildPositionMap(doc);

    for (const match of fstResult.matches) {
        // Convert byte to char offsets
        const { start: charStart, end: charEnd } = fstMatchToCharOffsets(match, docText);

        // Resolve to ProseMirror positions (positionMap is a number[])
        const posStart = positionMap[charStart];
        const posEnd = positionMap[charEnd];
        if (posStart === undefined || posEnd === undefined) continue;

        const from = posStart + 1; // +1 for doc node offset
        const to = posEnd + 1;

        if (from >= to || from < 1) continue;

        // Check for overlap with already-processed ranges
        const overlaps = processedRanges.some(r => (from < r.end && to > r.start));
        if (overlaps) continue;

        processedRanges.push({ start: from, end: to });

        // Determine styling based on source type
        const borderStyles: Record<string, string> = {
            'gazetteer': 'solid',   // Known entity
            'rule': 'dashed',       // Rule match
            'orthographic': 'dotted' // Shape heuristic
        };
        const borderStyle = borderStyles[match.source] || 'solid';

        // Get color based on entity kind
        const kindNormalized = (match.kind || 'UNKNOWN').toUpperCase() as EntityKind;
        const color = ENTITY_COLORS[kindNormalized] || ENTITY_COLORS.UNKNOWN || '#8b5cf6';

        let style = '';
        if (mode === 'vivid') {
            style = `
                background-color: ${color}20;
                border-bottom: 2px ${borderStyle} ${color};
                border-radius: 2px;
                padding: 0 2px;
                cursor: pointer;
            `;
        } else if (mode === 'clean') {
            style = `
                border-bottom: 1px ${borderStyle} ${color}60;
                cursor: pointer;
            `;
        }

        const tooltip = `${match.kind}: ${match.canonical_label || match.text} (${Math.round(match.confidence * 100)}% - ${match.source})`;

        decorations.push(
            Decoration.inline(from, to, {
                class: `fst-match fst-${match.source}`,
                style,
                'data-fst-kind': match.kind,
                'data-fst-source': match.source,
                'data-fst-text': match.text,
                'data-fst-entity-id': match.entity_id || '',
                'title': tooltip,
            }, { inclusiveStart: false, inclusiveEnd: false })
        );
    }

    return decorations;
}

function buildAllDecorations(
    doc: ProseMirrorNode,
    options: RustHighlighterOptions,
    selection?: { from: number; to: number }
): DecorationSet {
    const allDecorations: Decoration[] = [];

    // 1. Pattern decorations (wikilinks, tags, entities, etc.) - Highest Priority
    // These are processed first and their ranges are used to exclude FST/Tauri matches
    const { decorations: patternDecorations, coveredRanges: patternRanges } =
        buildPatternDecorations(doc, options, selection);
    allDecorations.push(...patternDecorations);

    // Doc-relative ranges for Tauri + NER (these share coordinate space)
    // Pre-populate with pattern ranges so FST won't underline already-registered entities
    const docProcessedRanges: Range[] = [...patternRanges];

    // 2. Tauri Native decorations (implicit entities + temporal)
    if (tauriScanner.isReady() && lastScanResult) {
        const positionMap = buildPositionMap(doc);

        if (options.logPerformance && !lastScanResult.stats.was_skipped) {
            console.log(`[RustHighlighter] Tauri: ${(lastScanResult.stats.timings.total_us / 1000).toFixed(1)}ms, ${lastScanResult.implicit.length} implicit`);
        }

        const tauriDecorations = buildTauriDecorations(lastScanResult, positionMap, options, docProcessedRanges);
        allDecorations.push(...tauriDecorations);
    }

    // 2.5. FST-NER decorations (if enabled)
    const fstEnabled = options.getFstNerEnabled?.() ?? false;
    if (fstEnabled && lastFstResult && lastFstResult.matches.length > 0) {
        if (options.logPerformance) {
            console.log(`[RustHighlighter] FST: ${(lastFstResult.timing_us / 1000).toFixed(2)}ms, ${lastFstResult.matches.length} matches`);
        }
        const fstDecorations = buildFstDecorations(doc, lastFstResult, options, docProcessedRanges);
        allDecorations.push(...fstDecorations);
    }

    // 3. Legacy NER suggestions (frontend entities)
    const nerDecorations = buildNERDecorations(doc, options, docProcessedRanges);
    allDecorations.push(...nerDecorations);

    // 4. Rust NER suggestions (from Tauri backend)
    // TEMPORARILY DISABLED - debugging ProseMirror crash
    // const nerSuggestionDecorations = buildNerSuggestionDecorations(doc, options, docProcessedRanges);
    // allDecorations.push(...nerSuggestionDecorations);

    return DecorationSet.create(doc, allDecorations);
}

// ==================== LINK TRACKING ====================

function extractEntityMentionsFromDoc(
    doc: ProseMirrorNode,
    noteId: string
): EntityMentionEvent[] {
    const mentions: EntityMentionEvent[] = [];
    const patterns = patternRegistry.getActivePatterns().filter(p => p.kind === 'entity');

    doc.descendants((node, pos) => {
        if (!node.isText || !node.text) return;

        const text = node.text;

        for (const pattern of patterns) {
            const regex = patternRegistry.getCompiledPattern(pattern.id);
            let match: RegExpExecArray | null;
            regex.lastIndex = 0;

            while ((match = regex.exec(text)) !== null) {
                if (match.index === regex.lastIndex) {
                    regex.lastIndex++;
                }

                const entityId = match[2] || match[1] || match[0];
                if (!entityId) continue;

                const contextStart = Math.max(0, match.index - 50);
                const contextEnd = Math.min(text.length, match.index + match[0].length + 50);
                const context = text.substring(contextStart, contextEnd);

                mentions.push({
                    type: 'entityMentioned',
                    noteId,
                    entityId,
                    mention: {
                        text: match[0],
                        position: pos + match.index,
                        context,
                        mentionType: 'explicit',
                        positionType: 'body' as PositionType,
                    },
                    timestamp: Date.now(),
                });
            }
        }
    });

    return mentions;
}

// ==================== CACHED SPANS FETCH (Phase 3) ====================

/**
 * Fetch cached decoration spans from CozoDB.
 * 
 * Phase 3: Instead of sending text to Rust via IPC, we fetch pre-computed
 * spans from the decoration_spans cache (populated by ScanWorker on save).
 */
async function fetchCachedSpans(doc: ProseMirrorNode, noteId: string): Promise<void> {
    if (!isTauri()) return;

    const text = extractText(doc);
    const hash = computeContentHash(text);

    // Check in-memory cache first
    const memoryCached = cachedSpansCache.get(noteId);
    if (memoryCached && memoryCached.hash === hash) {
        // Already have valid cached spans - convert to lastScanResult format
        lastScanResult = spansToScanResult(memoryCached.spans);
        return;
    }

    // Fetch from CozoDB cache
    const spans = await fetchDecorationSpans(noteId, hash);
    if (spans) {
        cachedSpansCache.set(noteId, { hash, spans });
        lastScanResult = spansToScanResult(spans);
    } else {
        // Cache miss - decorations will appear after background scan completes
        // Clear stale result so we don't show outdated highlights
        lastScanResult = null;
    }
}

/**
 * Convert DecorationSpanRecord[] to ConductorScanResult format
 * (for compatibility with buildTauriDecorations)
 */
function spansToScanResult(spans: DecorationSpanRecord[]): ConductorScanResult {
    const implicit: ImplicitMention[] = [];
    const temporal: TemporalMention[] = [];

    for (const span of spans) {
        if (span.span_type === 'implicit') {
            implicit.push({
                start: span.start,
                end: span.end,
                entity_id: span.entity_id || '',
                entity_label: span.entity_label || '',
                entity_kind: span.entity_kind || 'ENTITY',
                is_alias_match: span.is_alias,
                matched_text: span.entity_label || '',
            });
        } else if (span.span_type === 'temporal') {
            temporal.push({
                start: span.start,
                end: span.end,
                text: span.entity_label || '',
                kind: span.metadata || 'RELATIVE',
            });
        }
    }

    return {
        implicit,
        temporal,
        explicit: [],
        triples: [],
        unified_relations: [],
        stats: {
            was_skipped: false,
            skip_reason: null,
            implicit_found: implicit.length,
            temporal_found: temporal.length,
            triples_found: 0,
            unified_found: 0,
            timings: { total_us: 0, implicit_us: 0, temporal_us: 0, triple_us: 0 },
        },
    };
}

// ==================== FST-NER SCAN ====================

/**
 * Trigger FST-NER scan if enabled.
 * Stores result in lastFstResult for use by buildAllDecorations.
 */
async function triggerFstScan(doc: ProseMirrorNode, fstEnabled: boolean): Promise<void> {
    if (!fstEnabled || !isTauri()) {
        lastFstResult = null;
        return;
    }

    const text = extractText(doc);
    if (!text || text.length < 2) {
        lastFstResult = null;
        return;
    }

    try {
        lastFstResult = await fstNerScan(text);
    } catch (err) {
        console.error('[RustHighlighter] FST scan error:', err);
        lastFstResult = null;
    }
}

/**
 * Debounced FST-NER scan with view callback.
 * 
 * Waits 400ms after last keystroke before scanning, then dispatches
 * `tauriScanComplete` meta to trigger decoration rebuild.
 */
const debouncedFstScan = debounce(async (
    doc: ProseMirrorNode,
    fstEnabled: boolean,
    viewRef: { current: EditorView | null }
) => {
    await triggerFstScan(doc, fstEnabled);
    // Dispatch transaction to trigger decoration rebuild with new FST results
    if (viewRef.current && lastFstResult) {
        viewRef.current.dispatch(
            viewRef.current.state.tr.setMeta('tauriScanComplete', true)
        );
    }
}, 400);

// ==================== EXTENSION ====================

export const RustHighlighter = Extension.create<RustHighlighterOptions>({
    name: 'rustHighlighter',

    addOptions() {
        return {
            onWikilinkClick: undefined,
            checkWikilinkExists: undefined,
            onTemporalClick: undefined,
            onBacklinkClick: undefined,
            onImplicitClick: undefined,
            onNEREntityClick: undefined,
            onRefClick: undefined,
            nerSuggestions: undefined,
            onNerSuggestionClick: undefined,
            onNerSuggestionAccept: undefined,
            onNerSuggestionReject: undefined,
            nerEntities: undefined,
            currentNoteId: undefined,
            useWidgetMode: false,
            enableLinkTracking: true,
            logPerformance: false,
            getHighlightMode: undefined,
            getFocusEntityKinds: undefined,
        };
    },

    addProseMirrorPlugins() {
        const options = this.options;
        let lastDocText = '';
        // Mutable ref to hold EditorView for debounced callback
        const viewRef: { current: EditorView | null } = { current: null };

        return [
            // Main decoration plugin
            new Plugin({
                key: rustPluginKey,
                state: {
                    init(_, { doc }) {
                        const text = extractText(doc);
                        lastDocText = text;

                        // Phase 3: Fetch cached spans from CozoDB (no IPC scan)
                        const noteId = resolveNoteId(options.currentNoteId);
                        fetchCachedSpans(doc, noteId);

                        return buildAllDecorations(doc, options);
                    },

                    apply(tr, oldDecorations, oldState, newState) {
                        const useWidgets = options.useWidgetMode ?? false;

                        // Check for forced rebuild triggers
                        const forceRebuild =
                            tr.getMeta('entityHydration') ||
                            tr.getMeta('forceRescan') ||
                            tr.getMeta('highlightModeChange') ||
                            tr.getMeta('tauriScanComplete');

                        // Rebuild on doc change or forced rebuild
                        if (tr.docChanged || forceRebuild) {
                            const text = extractText(newState.doc);

                            // Skip if text unchanged and not forced
                            if (text === lastDocText && !forceRebuild) {
                                return oldDecorations.map(tr.mapping, tr.doc);
                            }

                            lastDocText = text;

                            // Phase 3: Fetch cached spans on doc change
                            // Note: Actual scan happens in background via ScanWorker
                            if (tr.docChanged) {
                                const noteId = resolveNoteId(options.currentNoteId);
                                fetchCachedSpans(newState.doc, noteId);

                                // FST-NER scan (debounced - waits 400ms after last keystroke)
                                const fstEnabled = options.getFstNerEnabled?.() ?? false;
                                if (fstEnabled) {
                                    debouncedFstScan(newState.doc, fstEnabled, viewRef);
                                }
                            }

                            const selection = { from: newState.selection.from, to: newState.selection.to };
                            return buildAllDecorations(newState.doc, options, selection);
                        }

                        // Widget mode OR Clean mode: check selection for expand/collapse
                        const mode = options.getHighlightMode?.() ?? 'vivid';
                        const needsSelectionCheck = useWidgets || mode === 'clean';

                        if (tr.selectionSet && needsSelectionCheck) {
                            const oldSel = { from: oldState.selection.from, to: oldState.selection.to };
                            const newSel = { from: newState.selection.from, to: newState.selection.to };

                            if (mode === 'clean') {
                                return buildAllDecorations(newState.doc, options, newSel);
                            }

                            if (!crossesDecorationBoundary(oldDecorations, oldSel, newSel)) {
                                return oldDecorations.map(tr.mapping, tr.doc);
                            }
                            return buildAllDecorations(newState.doc, options, newSel);
                        }

                        return oldDecorations.map(tr.mapping, tr.doc);
                    },
                },

                // Capture view reference for debounced FST callback
                view(editorView) {
                    viewRef.current = editorView;
                    return {
                        destroy() {
                            viewRef.current = null;
                            debouncedFstScan.cancel();
                        },
                    };
                },

                props: {
                    decorations(state) {
                        return rustPluginKey.getState(state);
                    },

                    handleDOMEvents: {
                        mousedown: (view, event) => {
                            const target = event.target as HTMLElement;
                            if (target.getAttribute('data-editable-widget') === 'true') {
                                const pos = view.posAtDOM(target, 0);
                                const tr = view.state.tr.setSelection(
                                    Selection.near(view.state.doc.resolve(pos))
                                );
                                view.dispatch(tr);
                                return true;
                            }
                            return false;
                        },

                        click: (view, event) => {
                            const target = event.target as HTMLElement;

                            // Implicit entity clicks
                            const entityId = target.getAttribute('data-entity-id');
                            if (entityId && options.onImplicitClick) {
                                const entityLabel = target.getAttribute('data-entity-label') || '';
                                event.preventDefault();
                                event.stopPropagation();
                                options.onImplicitClick(entityId, entityLabel);
                                return true;
                            }

                            // Temporal clicks
                            const temporal = target.getAttribute('data-temporal');
                            if (temporal && options.onTemporalClick) {
                                event.preventDefault();
                                event.stopPropagation();
                                options.onTemporalClick(temporal);
                                return true;
                            }

                            // Legacy NER clicks (frontend entities)
                            const nerEntity = target.getAttribute('data-ner-entity');
                            if (nerEntity && options.onNEREntityClick) {
                                event.preventDefault();
                                event.stopPropagation();
                                const nerType = target.getAttribute('data-ner-type') || '';
                                const nerStart = parseInt(target.getAttribute('data-ner-start') || '0', 10);
                                const nerEnd = parseInt(target.getAttribute('data-ner-end') || '0', 10);
                                options.onNEREntityClick({
                                    word: nerEntity,
                                    entity_type: nerType,
                                    start: nerStart,
                                    end: nerEnd,
                                    score: 0,
                                });
                                return true;
                            }

                            // Rust NER suggestion clicks (from Tauri backend)
                            const suggestionId = target.getAttribute('data-ner-suggestion-id');
                            if (suggestionId) {
                                event.preventDefault();
                                event.stopPropagation();

                                // Extract suggestion data from attributes
                                const suggestionText = target.getAttribute('data-ner-suggestion-text') || '';
                                const suggestionType = target.getAttribute('data-ner-suggestion-type') || '';
                                const suggestionConfidence = parseFloat(target.getAttribute('data-ner-suggestion-confidence') || '0');
                                const suggestionLabel = target.getAttribute('data-ner-suggestion-label') || '';

                                // Call the suggestion click handler if provided
                                if (options.onNerSuggestionClick) {
                                    options.onNerSuggestionClick({
                                        id: suggestionId,
                                        world_id: '', // Not stored in DOM
                                        source_note_id: resolveNoteId(options.currentNoteId),
                                        text: suggestionText,
                                        label: suggestionLabel,
                                        entity_type: suggestionType,
                                        byte_start: 0, // Byte offsets not stored in DOM
                                        byte_end: 0,
                                        confidence: suggestionConfidence,
                                        inferred_at: 0,
                                    });
                                }
                                return true;
                            }

                            // Pattern ref clicks
                            const refKind = target.getAttribute('data-ref-kind') as RefKind | null;
                            const refTarget = target.getAttribute('data-ref-target');

                            if (refKind && refTarget) {
                                event.preventDefault();
                                event.stopPropagation();

                                if (refKind === 'wikilink' && options.onWikilinkClick) {
                                    options.onWikilinkClick(refTarget);
                                } else if (refKind === 'backlink' && options.onBacklinkClick) {
                                    options.onBacklinkClick(refTarget);
                                } else if (refKind === 'temporal' && options.onTemporalClick) {
                                    options.onTemporalClick(refTarget);
                                } else if (options.onRefClick) {
                                    options.onRefClick(refKind, refTarget);
                                }
                                return true;
                            }

                            return false;
                        },
                    },
                },
            }),

            // Link tracker plugin
            new Plugin({
                key: rustLinkTrackerKey,
                appendTransaction(transactions, oldState, newState) {
                    const noteId = resolveNoteId(options.currentNoteId);
                    if (options.enableLinkTracking === false || noteId === 'unknown') {
                        return null;
                    }

                    if (!transactions.some(tr => tr.docChanged)) {
                        return null;
                    }

                    const mentions = extractEntityMentionsFromDoc(newState.doc, noteId);

                    if (mentions.length > 0) {
                        mentions.forEach(mention => mentionEventQueue.enqueue(mention));
                    }

                    return null;
                },
            }),
        ];
    },
});
