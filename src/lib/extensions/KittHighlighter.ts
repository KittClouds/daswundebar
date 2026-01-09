/**
 * KittHighlighter - Unified Syntax Highlighting Extension
 * 
 * Single extension that combines:
 * - Rust WASM implicit entity detection (via highlighterBridge)
 * - Rust WASM temporal expression detection
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

// Rust WASM bridge
import { highlighterBridge, type HighlightSpan } from '@/lib/highlighter/HighlighterBridge';
import { getOrBuildPositionMap, getOrBuildText } from '@/lib/highlighter/positionMapCache';

// Highlighter Facade (Rust decoration spans)
import { highlighterFacade, type UnifiedScanResult, type DecorationSpan as UnifiedSpan } from '@/lib/scanner/highlighter-facade';
import {
    getCaptureValue,
    getValidatedEntityKind,
    getEntityColor,
    getEntityBgColor,
    safeProcessSpan,
    isValidRefKind,
} from '@/lib/scanner/unified-scanner-utils';

// Types
import { EntityKind, ENTITY_KINDS, ENTITY_COLORS } from '@/lib/types/entityTypes';
import type { NEREntity } from '@/lib/extraction';
import type { HighlightMode } from '@/atoms/highlightingAtoms';
import type { EntityMentionEvent, PositionType } from '@/lib/cozo-stubs/types';

// Pattern registry
import { patternRegistry, type PatternDefinition, type RefKind } from '@/lib/refs';

// Event queue for link tracking
import { mentionEventQueue } from '@/lib/scanner/mention-event-queue';

// ==================== OPTIONS ====================

export interface KittHighlighterOptions {
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
    useUnifiedScanner?: boolean; // NEW: A/B test flag for Rust scanner

    // Highlighting mode getters (reactive)
    getHighlightMode?: () => HighlightMode;
    getFocusEntityKinds?: () => EntityKind[];
}

// ==================== HELPERS ====================

const kittPluginKey = new PluginKey('kitt-highlighter');
const linkTrackerKey = new PluginKey('kitt-link-tracker');

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

// Cache for tracking last emitted content hash per note
const lastEmittedContentHash = new Map<string, string>();

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
    // No buffer - precise detection. Cursor must be exactly within the range.
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

// ==================== DECORATION BUILDERS ====================

/**
 * Build Rust WASM decorations (implicit entities + temporal)
 */
function buildRustDecorations(
    spans: HighlightSpan[],
    positionMap: number[],
    options: KittHighlighterOptions,
    processedRanges: Range[]
): Decoration[] {
    const decorations: Decoration[] = [];
    const mode = options.getHighlightMode?.() ?? 'clean';
    const focusKinds = options.getFocusEntityKinds?.() ?? [];

    if (mode === 'off' || mode === 'clean') return decorations;

    for (const span of spans) {
        const from = positionMap[span.start];
        const to = positionMap[span.end - 1] !== undefined
            ? positionMap[span.end - 1] + 1
            : positionMap[span.start] + (span.end - span.start);

        if (from === undefined || to === undefined) continue;

        // Focus mode filter
        if (mode === 'focus') {
            const entityKind = span.metadata?.entityKind as EntityKind | undefined;
            if (!entityKind || (focusKinds.length > 0 && !focusKinds.includes(entityKind))) {
                continue;
            }
        }

        // Check overlap with higher priority decorations (e.g. pattern widgets)
        if (rangesOverlap(processedRanges, span.start, span.end)) {
            continue;
        }

        // Track processed range
        insertRange(processedRanges, span.start, span.end);

        let style = '';
        let className = '';
        let dataAttrs: Record<string, string> = {};

        switch (span.kind) {
            case 'implicit': {
                const entityKind = span.metadata.entityKind || 'CHARACTER';
                const varName = `--entity-${entityKind.toLowerCase().replace('_', '-')}`;
                const color = `hsl(var(${varName}))`;
                const isVivid = mode === 'vivid';
                const borderStyle = isVivid ? 'solid' : (span.confidence < 1.0 ? 'dotted' : 'solid');
                const bgOpacity = isVivid ? 0.15 : 0.1;

                style = `
          background-color: hsl(var(${varName}) / ${bgOpacity}); 
          color: ${color}; 
          padding: 0px 2px; 
          border-bottom: 2px ${borderStyle} ${color}; 
          cursor: help;
        `;
                className = isVivid ? 'kitt-implicit vivid' : 'kitt-implicit';
                dataAttrs = {
                    'data-entity-id': span.metadata.entityId || '',
                    'data-entity-kind': entityKind,
                    'data-entity-label': span.label,
                    'data-confidence': span.confidence.toString(),
                    'title': `${entityKind}: ${span.label}${span.metadata.isAlias ? ' (alias)' : ''}`,
                };
                break;
            }

            case 'temporal': {
                const isVivid = mode === 'vivid';
                style = `
          background-color: hsl(var(--warning) / ${isVivid ? 0.2 : 0.15});
          color: hsl(var(--warning));
          padding: 0px 2px;
          border-bottom: 2px ${isVivid ? 'solid' : 'dashed'} hsl(var(--warning));
          cursor: pointer;
        `;
                className = isVivid ? 'kitt-temporal vivid' : 'kitt-temporal';
                dataAttrs = {
                    'data-temporal': span.content,
                    'data-temporal-kind': span.target,
                    'title': `Temporal: ${span.content}`,
                };
                break;
            }
        }

        decorations.push(
            Decoration.inline(from, to, {
                class: className,
                style,
                ...dataAttrs,
            }, { inclusiveStart: false, inclusiveEnd: false })
        );
    }

    return decorations;
}

/**
 * Build pattern registry decorations (wikilinks, tags, mentions)
 * Note: processedRanges is PER-NODE (node-relative offsets)
 * Mode-aware: respects highlighting mode settings
 */
function buildPatternDecorations(
    doc: ProseMirrorNode,
    options: KittHighlighterOptions,
    selection: { from: number; to: number } | undefined
): Decoration[] {
    const decorations: Decoration[] = [];
    const useWidgets = options.useWidgetMode ?? false;
    const patterns = patternRegistry.getActivePatterns();

    // Get highlighting mode
    const mode = options.getHighlightMode?.() ?? 'vivid';

    // Off mode: no pattern decorations at all
    if (mode === 'off') {
        return decorations;
    }

    // Note: Clean mode still creates widgets (to collapse raw syntax)
    // but with minimal styling. See styling logic below.

    doc.descendants((node, pos) => {
        if (!node.isText || !node.text) return;

        const text = node.text;
        const processedRanges: Range[] = [];  // PER-NODE: node-relative offsets

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

                // Check overlap
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

                // Clean mode: override with minimal styling (plain text appearance)
                // Only for collapsed widgets - when editing (expanded), show full colors
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
                    // Clean mode expanded: only highlight the label portion, not the full syntax
                    if (mode === 'clean' && label !== fullMatch) {
                        // Find label position within fullMatch
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
                        // Normal vivid/focus mode: highlight entire match
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
    options: KittHighlighterOptions,
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

// ==================== UNIFIED RUST DECORATIONS (A/B TEST) ====================

/**
 * Build decorations using the new Rust Unified Scanner
 * Replaces buildPatternDecorations when enabled
 */
function buildUnifiedDecorations(
    doc: ProseMirrorNode,
    options: KittHighlighterOptions,
    selection: { from: number; to: number } | undefined
): Decoration[] {
    const decorations: Decoration[] = [];
    const mode = options.getHighlightMode?.() ?? 'vivid';

    if (mode === 'off') return decorations;

    const useWidgets = options.useWidgetMode ?? false;
    const focusKinds = options.getFocusEntityKinds?.() ?? [];

    // Convert entity keys to match scanner format (lowercase)
    const focusConfig = { kinds: focusKinds.map(k => k.toLowerCase()) };

    // Scan per text node to maintain parity with legacy behavior
    doc.descendants((node, pos) => {
        if (!node.isText || !node.text) return;

        // Scan this text node
        const results = highlighterFacade.scan(node.text);

        // Get decoration specs from facade
        // Pass local cursor position if selection is inside this node
        let localCursor: number | undefined = undefined;
        if (selection && selection.from >= pos && selection.from <= pos + node.text.length) {
            localCursor = selection.from - pos;
        }

        const specs = highlighterFacade.getDecorations(results, mode, focusConfig, localCursor);

        for (const { from: localFrom, to: localTo, spec, span } of specs) {
            const absFrom = pos + localFrom;
            const absTo = pos + localTo;

            // Existence check for wikilinks
            let exists = true;
            if (span.kind === 'Wikilink' && options.checkWikilinkExists) {
                exists = options.checkWikilinkExists(span.captures['target'] || span.label);
            }

            if (spec.showWidget) {
                // Calculate entity-specific colors for widgets
                let widgetColor = 'inherit';
                let widgetBgColor = 'transparent';

                if (spec.applyColor) {
                    if (span.kind === 'Entity') {
                        // Use utility function that handles Map vs Object serialization
                        const entityKind = getValidatedEntityKind(span.captures);
                        widgetColor = getEntityColor(entityKind);
                        widgetBgColor = getEntityBgColor(entityKind);
                    } else if (span.kind === 'Wikilink' || span.kind === 'Backlink') {
                        widgetColor = exists ? 'hsl(var(--primary))' : 'hsl(var(--destructive))';
                        widgetBgColor = exists ? 'hsl(var(--primary) / 0.15)' : 'hsl(var(--destructive) / 0.15)';
                    } else if (span.kind === 'Tag') {
                        widgetColor = '#3b82f6';
                        widgetBgColor = '#3b82f620';
                    } else if (span.kind === 'Mention') {
                        widgetColor = '#8b5cf6';
                        widgetBgColor = '#8b5cf620';
                    } else if (span.kind === 'Triple') {
                        widgetColor = 'var(--entity-relationship, #f59e0b)';
                        widgetBgColor = 'rgba(245, 158, 11, 0.15)';
                    }
                }

                // Widget logic
                const widget = createPatternWidget(
                    span.label,
                    span.kind.toLowerCase() as RefKind,
                    span.raw_text,
                    widgetColor,
                    widgetBgColor,
                    span.kind === 'Wikilink' ? (exists ? 'wikilink-exists' : 'wikilink-broken') : ''
                );

                decorations.push(
                    Decoration.widget(absFrom, widget, {
                        side: -1,
                        key: `u-${span.kind}-${absFrom}-${span.raw_text}`,
                    })
                );

                decorations.push(
                    Decoration.inline(absFrom, absTo, {
                        class: 'ref-hidden',
                        style: 'display: none;',
                    })
                );
            } else {
                // Inline decoration (not widget mode)
                const isCleanModeEditing = mode === 'clean' && spec.modeClass.includes('highlight-editing');

                // Get entity-specific colors
                let color = 'inherit';
                let bgColor = 'transparent';

                if (span.kind === 'Entity') {
                    // Use utility function that handles Map vs Object serialization
                    const entityKind = getValidatedEntityKind(span.captures);
                    color = getEntityColor(entityKind);
                    bgColor = getEntityBgColor(entityKind);
                } else if (span.kind === 'Wikilink' || span.kind === 'Backlink') {
                    color = exists ? 'hsl(var(--primary))' : 'hsl(var(--destructive))';
                    bgColor = exists ? 'hsl(var(--primary) / 0.15)' : 'hsl(var(--destructive) / 0.15)';
                } else if (span.kind === 'Tag') {
                    color = '#3b82f6';
                    bgColor = '#3b82f620';
                } else if (span.kind === 'Mention') {
                    color = '#8b5cf6';
                    bgColor = '#8b5cf620';
                }

                // Clean mode expanded: only highlight the label portion, not the full syntax
                if (isCleanModeEditing && span.label !== span.raw_text) {
                    const labelIndex = span.raw_text.indexOf(span.label);
                    if (labelIndex !== -1) {
                        const labelFrom = absFrom + labelIndex;
                        const labelTo = labelFrom + span.label.length;

                        decorations.push(
                            Decoration.inline(labelFrom, labelTo, {
                                class: `ref-highlight ref-${span.kind.toLowerCase()} clean-mode-label`,
                                style: `
                                    background-color: ${bgColor};
                                    color: ${color};
                                    padding: 2px 4px;
                                    border-radius: 3px;
                                `,
                                'data-ref-kind': span.kind.toLowerCase(),
                                'data-ref-target': span.captures['target'] || span.label,
                            }, { inclusiveStart: false, inclusiveEnd: false })
                        );
                    }
                } else {
                    // Normal vivid/focus mode or clean mode with matching label
                    const className = `${spec.baseClass} ${spec.modeClass} ${span.kind === 'Wikilink' ? (exists ? 'wikilink-exists' : 'wikilink-broken') : ''
                        }`;

                    const style = spec.applyColor ? `
                        background-color: ${bgColor};
                        color: ${color};
                        padding: 2px 6px;
                        border-radius: 4px;
                        font-weight: 500;
                        font-size: 0.875em;
                        cursor: pointer;
                    ` : '';

                    const attrs: Record<string, string> = {
                        'class': className,
                        'data-ref-kind': span.kind.toLowerCase(),
                        'data-ref-target': span.captures['target'] || span.label,
                    };

                    if (style) {
                        attrs['style'] = style;
                    }

                    if (span.kind === 'Wikilink') {
                        attrs['data-ref-exists'] = exists.toString();
                    }

                    decorations.push(
                        Decoration.inline(absFrom, absTo, attrs, { inclusiveStart: false, inclusiveEnd: false })
                    );
                }
            }
        }
    });

    return decorations;
}

// ==================== MAIN DECORATION BUILDER ====================


function buildAllDecorations(
    doc: ProseMirrorNode,
    options: KittHighlighterOptions,
    selection?: { from: number; to: number }
): DecorationSet {
    const allDecorations: Decoration[] = [];

    // 1. Pattern decorations (Legacy vs Unified) - Highest Priority
    if (options.useUnifiedScanner && highlighterFacade.isReady()) {
        const unifiedDecorations = buildUnifiedDecorations(doc, options, selection);
        allDecorations.push(...unifiedDecorations);
        if (options.logPerformance) {
            // console.log(`[KittHighlighter] Using Unified Rust Scanner (${unifiedDecorations.length} decos)`);
        }
    } else {
        const patternDecorations = buildPatternDecorations(doc, options, selection);
        allDecorations.push(...patternDecorations);
    }

    // Doc-relative ranges for Rust + NER (these share coordinate space)
    const docProcessedRanges: Range[] = [];

    // 2. Rust WASM decorations (implicit entities + temporal) - Secondary Priority
    if (highlighterBridge.isReady()) {
        const { text, positionMap } = getOrBuildPositionMap(doc);
        const noteId = resolveNoteId(options.currentNoteId);
        const result = highlighterBridge.highlight(text, noteId);

        if (options.logPerformance && !result.wasCached) {
            console.log(`[KittHighlighter] Rust: ${result.scanTimeMs.toFixed(1)}ms, ${result.spans.length} spans`);
        }

        const rustDecorations = buildRustDecorations(result.spans, positionMap, options, docProcessedRanges);
        allDecorations.push(...rustDecorations);
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

// ==================== EXTENSION ====================

export const KittHighlighter = Extension.create<KittHighlighterOptions>({
    name: 'kittHighlighter',

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

        return [
            // Main decoration plugin
            new Plugin({
                key: kittPluginKey,
                state: {
                    init(_, { doc }) {
                        if (!highlighterBridge.isReady()) {
                            // Still build pattern decorations even if Rust isn't ready
                            return buildAllDecorations(doc, options);
                        }

                        const { text } = getOrBuildPositionMap(doc);
                        lastDocText = text;
                        return buildAllDecorations(doc, options);
                    },

                    apply(tr, oldDecorations, oldState, newState) {
                        const useWidgets = options.useWidgetMode ?? false;

                        // Check for forced rebuild triggers
                        const forceRebuild =
                            tr.getMeta('entityHydration') ||
                            tr.getMeta('forceRescan') ||
                            tr.getMeta('highlightModeChange');

                        // Rebuild on doc change or forced rebuild
                        if (tr.docChanged || forceRebuild) {
                            const { text } = getOrBuildPositionMap(newState.doc);

                            // Skip if text unchanged and not forced
                            if (text === lastDocText && !forceRebuild) {
                                return oldDecorations.map(tr.mapping, tr.doc);
                            }

                            lastDocText = text;

                            // Invalidate cache on forced rescan
                            if (forceRebuild && tr.getMeta('forceRescan')) {
                                const noteId = resolveNoteId(options.currentNoteId);
                                highlighterBridge.invalidateCache(noteId);
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

                            // Clean mode: always rebuild on selection change for expand/collapse
                            // Widget mode: use boundary optimization
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
                        return kittPluginKey.getState(state);
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
                key: linkTrackerKey,
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
