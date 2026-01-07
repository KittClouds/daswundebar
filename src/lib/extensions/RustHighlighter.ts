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

// Tauri Native bridge (replaces WASM)
import {
    tauriScanner,
    isTauri,
    type ConductorScanResult,
    type ImplicitMention,
    type TemporalMention
} from '@/lib/tauri/bridge';

// Types
import { EntityKind, ENTITY_KINDS, ENTITY_COLORS } from '@/lib/types/entityTypes';
import type { NEREntity } from '@/lib/extraction';
import type { HighlightMode } from '@/atoms/highlightingAtoms';
import type { EntityMentionEvent, PositionType } from '@/lib/cozo/types';

// Pattern registry (shared with KittHighlighter)
import { patternRegistry, type PatternDefinition, type RefKind } from '@/lib/refs';

// Event queue for link tracking
import { mentionEventQueue } from '@/lib/scanner/mention-event-queue';

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

    // NER entities getter
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

// Cache for last scan result per note
const scanResultCache = new Map<string, { hash: string; result: ConductorScanResult }>();

// ==================== RANGE OVERLAP DETECTION ====================

type Range = [number, number]; // [start, end)

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
 * Build pattern registry decorations (wikilinks, tags, mentions)
 * Mode-aware: respects highlighting mode settings
 */
function buildPatternDecorations(
    doc: ProseMirrorNode,
    options: RustHighlighterOptions,
    selection: { from: number; to: number } | undefined
): Decoration[] {
    const decorations: Decoration[] = [];
    const useWidgets = options.useWidgetMode ?? false;
    const patterns = patternRegistry.getActivePatterns();
    const mode = options.getHighlightMode?.() ?? 'vivid';

    if (mode === 'off') {
        return decorations;
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
                            key: `${pattern.id}-${from}-${fullMatch}`,
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

    return decorations;
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

// ==================== MAIN DECORATION BUILDER ====================

// Last scan result storage (sync access for decoration building)
let lastScanResult: ConductorScanResult | null = null;

function buildAllDecorations(
    doc: ProseMirrorNode,
    options: RustHighlighterOptions,
    selection?: { from: number; to: number }
): DecorationSet {
    const allDecorations: Decoration[] = [];

    // 1. Pattern decorations (wikilinks, tags, entities, etc.) - Highest Priority
    const patternDecorations = buildPatternDecorations(doc, options, selection);
    allDecorations.push(...patternDecorations);

    // Doc-relative ranges for Tauri + NER (these share coordinate space)
    const docProcessedRanges: Range[] = [];

    // 2. Tauri Native decorations (implicit entities + temporal)
    if (tauriScanner.isReady() && lastScanResult) {
        const positionMap = buildPositionMap(doc);

        if (options.logPerformance && !lastScanResult.stats.was_skipped) {
            console.log(`[RustHighlighter] Tauri: ${(lastScanResult.stats.timings.total_us / 1000).toFixed(1)}ms, ${lastScanResult.implicit.length} implicit`);
        }

        const tauriDecorations = buildTauriDecorations(lastScanResult, positionMap, options, docProcessedRanges);
        allDecorations.push(...tauriDecorations);
    }

    // 3. NER suggestions
    const nerDecorations = buildNERDecorations(doc, options, docProcessedRanges);
    allDecorations.push(...nerDecorations);

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

// ==================== ASYNC SCAN TRIGGER ====================

async function triggerTauriScan(doc: ProseMirrorNode, noteId: string): Promise<void> {
    if (!tauriScanner.isReady() || !isTauri()) return;

    const text = extractText(doc);
    const hash = computeContentHash(text);

    // Check cache
    const cached = scanResultCache.get(noteId);
    if (cached && cached.hash === hash) {
        lastScanResult = cached.result;
        return;
    }

    // Perform scan
    const result = await tauriScanner.conductorScanImmediate(text, []);
    if (result) {
        lastScanResult = result;
        scanResultCache.set(noteId, { hash, result });
    }
}

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
        let pendingScan: Promise<void> | null = null;

        return [
            // Main decoration plugin
            new Plugin({
                key: rustPluginKey,
                state: {
                    init(_, { doc }) {
                        const text = extractText(doc);
                        lastDocText = text;

                        // Trigger initial scan async
                        const noteId = resolveNoteId(options.currentNoteId);
                        triggerTauriScan(doc, noteId);

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

                            // Trigger async scan on doc change
                            if (tr.docChanged) {
                                const noteId = resolveNoteId(options.currentNoteId);
                                triggerTauriScan(newState.doc, noteId);
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

                            // NER clicks
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
