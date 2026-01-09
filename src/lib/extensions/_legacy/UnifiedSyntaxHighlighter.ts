import { Extension } from '@tiptap/core';
import { Plugin, PluginKey, Selection } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { Node as ProseMirrorNode } from '@tiptap/pm/model';
import { EntityKind, ENTITY_KINDS, ENTITY_COLORS } from '@/lib/types/entityTypes';
import type { NEREntity } from '../extraction';
// LEGACY: This module is deprecated. Using smartGraphRegistry for cleanliness.
import { smartGraphRegistry } from '@/lib/tauri';
import { patternRegistry, type PatternDefinition, type RefKind } from '../refs';
import { mentionEventQueue } from '@/lib/scanner/mention-event-queue';
import type { EntityMentionEvent, PositionType } from '@/lib/cozo-stubs/types';
import { getOrBuildText } from '@/lib/highlighter/positionMapCache'; // M3: Shared cache

// Legacy scanner-v3 stubs (Rust scanner handles this now)
// This extension is deprecated - use RustSyntaxHighlighter instead
interface PatternMatchEvent {
  kind: string;
  fullMatch: string;
  position: number;
  length: number;
  captures: Record<string, string>;
  patternId: string;
  noteId: string;
  timestamp: number;
  context: string;
}
const scannerEventBus = {
  emit: (_event: string, _data: any) => { },
  on: (_event: string, _handler: any) => { },
};
function computeContentHash(text: string): string {
  let hash = 0;
  for (let i = 0; i < text.length; i++) {
    hash = ((hash << 5) - hash + text.charCodeAt(i)) | 0;
  }
  return hash.toString(16);
}
const allProfanityEntityMatcher = {
  isInitialized: () => false,
  findMentions: (_text: string) => [] as Array<{
    position: number;
    length: number;
    entityId: string;
    kind: string;
    entity: { id: string; kind: string; label: string };
  }>,
  search: (_text: string) => [] as any[],
};

// Cache for tracking last emitted content hash per note
// Prevents redundant scanner events when returning to unchanged notes
const lastEmittedContentHash = new Map<string, string>();
// Timestamp of last emission per note (for 100ms throttle)
const lastEmittedTimestamp = new Map<string, number>();


export interface UnifiedSyntaxOptions {
  onWikilinkClick?: (title: string) => void;
  checkWikilinkExists?: (title: string) => boolean;
  onTemporalClick?: (temporal: string) => void;
  onBacklinkClick?: (title: string) => void;
  onRefClick?: (kind: RefKind, target: string, payload?: any) => void;
  nerEntities?: NEREntity[] | (() => NEREntity[]);
  onNEREntityClick?: (entity: NEREntity) => void;
  useWidgetMode?: boolean;
  enableLinkTracking?: boolean;
  currentNoteId?: string | (() => string | undefined); // Support dynamic getter
  enableScannerEvents?: boolean;
}

/**
 * Helper to resolve noteId from static value or getter
 */
function resolveNoteId(noteId: string | (() => string | undefined) | undefined): string {
  if (typeof noteId === 'function') {
    return noteId() || 'unknown';
  }
  return noteId || 'unknown';
}

const syntaxPluginKey = new PluginKey('unified-syntax-highlighter');

// Track which ranges are currently being edited
interface EditingRange {
  from: number;
  to: number;
}

/**
 * Check if cursor/selection overlaps with a range
 */
function isEditing(selection: { from: number; to: number }, range: EditingRange): boolean {
  // Consider "near" as within 2 characters (allows placing cursor before/after)
  const buffer = 2;
  return (
    (selection.from >= range.from - buffer && selection.from <= range.to + buffer) ||
    (selection.to >= range.from - buffer && selection.to <= range.to + buffer) ||
    (selection.from <= range.from && selection.to >= range.to)
  );
}

/**
 * Range-based overlap detection (O(log n) vs O(n) per-character Set)
 * Maintains sorted ranges for efficient overlap checks
 */
type Range = [number, number]; // [start, end)

function rangesOverlap(ranges: Range[], start: number, end: number): boolean {
  // Binary search for first range that could overlap
  let lo = 0, hi = ranges.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (ranges[mid][1] <= start) lo = mid + 1;
    else hi = mid;
  }
  // Check if found range overlaps
  return lo < ranges.length && ranges[lo][0] < end;
}

function insertRange(ranges: Range[], start: number, end: number): void {
  // Binary search for insertion point (maintain sorted order by start)
  let lo = 0, hi = ranges.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (ranges[mid][0] < start) lo = mid + 1;
    else hi = mid;
  }
  ranges.splice(lo, 0, [start, end]);
}

/**
 * M1: Check if selection movement crosses a decoration boundary
 * Returns true if cursor moved INTO or OUT OF a decorated region
 * Used to skip full rebuild when cursor moves within undecorated text
 */
function crossesDecorationBoundary(
  decorations: DecorationSet,
  oldSel: { from: number; to: number },
  newSel: { from: number; to: number }
): boolean {
  // Get decorations at old and new positions
  const oldFrom = decorations.find(oldSel.from, oldSel.from);
  const oldTo = decorations.find(oldSel.to, oldSel.to);
  const newFrom = decorations.find(newSel.from, newSel.from);
  const newTo = decorations.find(newSel.to, newSel.to);

  // If decoration count at position changed, boundary was crossed
  return oldFrom.length !== newFrom.length || oldTo.length !== newTo.length;
}

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

/**
 * Create a widget for any pattern match
 */
function createPatternWidget(
  label: string,
  kind: RefKind,
  fullMatch: string,
  color: string,
  backgroundColor: string,
  extraClasses: string = '',
  extraStyles: string = ''
): HTMLElement {
  const span = document.createElement('span');
  span.className = `ref - widget ref - ${kind} ${extraClasses} `;
  span.textContent = label;

  // Base style + overrides
  span.style.cssText = `
background - color: ${backgroundColor};
color: ${color};
padding: 2px 6px;
border - radius: 4px;
font - weight: 500;
font - size: 0.875em;
cursor: text;
display: inline - block;
position: relative;
    ${extraStyles}
`;

  // Data attributes for identifying the widget
  span.setAttribute('data-ref-kind', kind);
  span.setAttribute('data-ref-label', label);
  span.setAttribute('data-ref-full', fullMatch);

  // Make widget "clickable" to enable editing
  span.setAttribute('contenteditable', 'false');
  span.setAttribute('data-editable-widget', 'true');

  return span;
}

/**
 * Build decorations using PatternRegistry
 */
function buildAllDecorations(
  doc: ProseMirrorNode,
  options: UnifiedSyntaxOptions,
  selection?: { from: number; to: number }
): DecorationSet {
  const decorations: Decoration[] = [];
  const useWidgets = options.useWidgetMode ?? false;
  const noteId = resolveNoteId(options.currentNoteId);

  // M3: Use shared cache for text extraction (avoids duplicate traversal if RustSyntaxHighlighter also runs)
  const fullDocText = getOrBuildText(doc);

  // Check if content has changed since last emission
  const contentHash = computeContentHash(fullDocText);
  const lastHash = lastEmittedContentHash.get(noteId);
  const lastTime = lastEmittedTimestamp.get(noteId) || 0;
  const now = Date.now();
  // Skip emission if: scanner events disabled, hash unchanged, or within 100ms throttle window
  const shouldEmitEvents = options.enableScannerEvents !== false &&
    (lastHash !== contentHash || (now - lastTime) > 100);

  // Update the hash/timestamp cache if we're going to emit
  if (shouldEmitEvents) {
    lastEmittedContentHash.set(noteId, contentHash);
    lastEmittedTimestamp.set(noteId, now);
  }

  // Get active patterns sorted by priority
  const patterns = patternRegistry.getActivePatterns();

  doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return;

    const text = node.text;
    const processedRanges: Range[] = [];

    // 1. Iterate through registered patterns
    for (const pattern of patterns) {
      if (!pattern.enabled) continue;

      const regex = patternRegistry.getCompiledPattern(pattern.id);
      let match: RegExpExecArray | null;

      // Reset regex state
      regex.lastIndex = 0;

      while ((match = regex.exec(text)) !== null) {
        // Safe check for infinite loops with zero-length matches
        if (match.index === regex.lastIndex) {
          regex.lastIndex++;
        }

        const fullMatch = match[0];
        const from = pos + match.index;
        const to = from + fullMatch.length;

        // Check for overlaps with already processed ranges (O(log n))
        if (rangesOverlap(processedRanges, match.index, match.index + fullMatch.length)) continue;

        // Mark range as processed (O(log n) insert)
        insertRange(processedRanges, match.index, match.index + fullMatch.length);

        // Logic for "is being edited"
        const isCurrentlyEditing = selection
          ? isEditing(selection, { from, to })
          : false;

        // Extract Label/Target for display
        // Use capture mappings if available, otherwise fallback
        let label = fullMatch;
        let target = fullMatch;

        if (pattern.captures) {
          // Find capture keys that might represent label/display/target
          // This is a heuristic based on common capture names
          const labelKeys = ['label', 'displayText', 'displayName', 'username', 'tagName', 'word'];
          const targetKeys = ['target', 'id', 'username', 'tagName'];

          // Helper to find capture value
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

        // Dynamic State (Existence check for wikilinks)
        let exists = true;
        if (pattern.kind === 'wikilink' && options.checkWikilinkExists) {
          exists = options.checkWikilinkExists(target);
        }

        // Determine Colors
        let color = pattern.rendering?.color || 'hsl(var(--primary))';
        let bgColor = pattern.rendering?.backgroundColor || 'hsl(var(--primary) / 0.15)';

        // Entity Kind Colors
        if (pattern.kind === 'entity') {
          // Try to extract kind from regex if possible, or use generic
          // Default entity pattern has Kind in group 1
          const entityKind = match[1] as EntityKind;
          // Check if it's a known kind that would have a CSS var
          if (entityKind && ENTITY_COLORS[entityKind]) {
            const varName = `--entity - ${entityKind.toLowerCase().replace('_', '-')} `;
            color = `hsl(var(${varName}))`;
            bgColor = `hsl(var(${varName}) / 0.15)`;
          }
        }

        // Wikilink Colors (Dynamic)
        if (pattern.kind === 'wikilink') {
          color = exists ? 'hsl(var(--primary))' : 'hsl(var(--destructive))';
          bgColor = exists ? 'hsl(var(--primary) / 0.15)' : 'hsl(var(--destructive) / 0.15)';
        }

        // Render Widget vs Inline
        const shouldRenderWidget = (useWidgets || pattern.rendering?.widgetMode) && !isCurrentlyEditing;

        if (shouldRenderWidget) {
          // WIDGET
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
              key: `${pattern.id} -${from} -${fullMatch} `,
            })
          );

          // Hide original text
          decorations.push(
            Decoration.inline(from, to, {
              class: 'ref-hidden',
              style: 'display: none;',
            })
          );
        } else {
          // INLINE
          let style = `
background - color: ${bgColor};
color: ${color};
padding: 2px 6px;
border - radius: 4px;
font - weight: 500;
font - size: 0.875em;
cursor: pointer;
`;

          // Special inline styles
          if (pattern.kind === 'wikilink') {
            style += ` text - decoration: underline; text - decoration - style: ${exists ? 'dotted' : 'dashed'}; `;
          }

          decorations.push(
            Decoration.inline(from, to, {
              class: `ref - highlight ref - ${pattern.kind} ${pattern.kind === 'wikilink' ? (exists ? 'wikilink-editing' : 'wikilink-broken wikilink-editing') : ''} `,
              style,
              'data-ref-kind': pattern.kind,
              'data-ref-target': target,
              'data-ref-exists': exists.toString(),
            }, { inclusiveStart: false, inclusiveEnd: false })
          );
        }

        // Emit scanner event only if content changed (prevents redundant scans on view navigation)
        if (shouldEmitEvents) {
          const captures: Record<string, string> = {};
          if (pattern.captures) {
            for (const [key, mapping] of Object.entries(pattern.captures)) {
              if (match[mapping.group]) {
                captures[key] = match[mapping.group];
              }
            }
          }

          scannerEventBus.emit('pattern-matched', {
            kind: pattern.kind,
            fullMatch,
            position: from,
            length: fullMatch.length,
            captures,
            patternId: pattern.id,
            noteId,
            timestamp: Date.now(),
            context: text, // Pass full node text (paragraph) as context
          } satisfies PatternMatchEvent);
        }
      }
    }

    // 2. NER-detected entities (Keep existing logic)
    const nerEntities = typeof options.nerEntities === 'function'
      ? options.nerEntities()
      : options.nerEntities || [];

    if (nerEntities.length > 0) {
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

        // Check overlap (O(log n))
        if (rangesOverlap(processedRanges, relativeStart, relativeEnd)) continue;

        // Mark processed (O(log n) insert)
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
    }

    // 3. Registered Implicit Entities - Using AllProfanity's O(n) Aho-Corasick
    // Scanner 3.5 alignment: Same matcher, same Trie, no per-entity regex
    if (allProfanityEntityMatcher.isInitialized()) {
      const implicitMatches = allProfanityEntityMatcher.findMentions(text);

      for (const match of implicitMatches) {
        const from = pos + match.position;
        const to = from + match.length;

        // Check overlap (O(log n))
        if (rangesOverlap(processedRanges, match.position, match.position + match.length)) continue;

        // Mark processed (O(log n) insert)
        insertRange(processedRanges, match.position, match.position + match.length);

        const varName = `--entity - ${match.entity.kind.toLowerCase().replace('_', '-')} `;
        const color = `hsl(var(${varName}))`;
        const bgColor = `hsl(var(${varName}) / 0.1)`;

        decorations.push(
          Decoration.inline(from, to, {
            class: 'entity-implicit-highlight',
            style: `background - color: ${bgColor}; color: ${color}; padding: 0px 2px; border - bottom: 2px dotted ${color}; cursor: help; `,
            'data-entity-id': match.entity.id,
            'data-entity-kind': match.entity.kind,
            'data-entity-label': match.entity.label,
            'title': `${match.entity.kind}: ${match.entity.label} `
          }, { inclusiveStart: false, inclusiveEnd: false })
        );
      }
    }
  });

  return DecorationSet.create(doc, decorations);
}

export const UnifiedSyntaxHighlighter = Extension.create<UnifiedSyntaxOptions>({
  name: 'unifiedSyntaxHighlighter',

  addOptions() {
    return {
      onWikilinkClick: undefined,
      checkWikilinkExists: undefined,
      onTemporalClick: undefined,
      onBacklinkClick: undefined,
      onRefClick: undefined,
      nerEntities: undefined,
      onNEREntityClick: undefined,
      useWidgetMode: false,
      enableLinkTracking: true,

      currentNoteId: undefined,
      enableScannerEvents: true,
    };
  },

  addProseMirrorPlugins() {
    const options = this.options;

    return [
      new Plugin({
        key: syntaxPluginKey,
        state: {
          init(_, { doc }) {
            return buildAllDecorations(doc, options);
          },
          apply(tr, oldDecorations, oldState, newState) {
            const useWidgets = options.useWidgetMode ?? false;
            // Rebuild on doc change
            if (tr.docChanged) {
              const selection = { from: newState.selection.from, to: newState.selection.to };
              return buildAllDecorations(newState.doc, options, selection);
            }
            // M1: Check selection change for widget mode editing
            // Only rebuild if cursor crossed INTO or OUT OF a decorated region
            if (tr.selectionSet && useWidgets) {
              const oldSel = { from: oldState.selection.from, to: oldState.selection.to };
              const newSel = { from: newState.selection.from, to: newState.selection.to };

              // Skip rebuild if cursor didn't cross a decoration boundary
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
            return syntaxPluginKey.getState(state);
          },

          handleDOMEvents: {
            // Handle clicks on widgets to enable editing
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

              // 1. NER Logic (First, because it's distinct)
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

              // 2. Universal Ref Logic (from pattern registry)
              // Only trigger if NOT editing (check for editing class, e.g. .ref-hidden is for widget)
              // If we are in inline mode, we check specific classes.
              // Actually, simpler: if it has data-ref-kind, and we are not in edit mode (cursor nearby).

              const refKind = target.getAttribute('data-ref-kind') as RefKind | null;
              const refTarget = target.getAttribute('data-ref-target');

              if (refKind && refTarget) {
                // Prevent click if we are editing this exact element
                // But click handler fires usually when NOT editing.
                event.preventDefault();
                event.stopPropagation();

                // Route to specific handlers for backward compatibility
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

              // Fallback for legacy data attributes (just in case)
              const wikilinkTitle = target.getAttribute('data-wikilink-title');
              if (wikilinkTitle && options.onWikilinkClick) {
                event.preventDefault();
                options.onWikilinkClick(wikilinkTitle);
                return true;
              }

              return false;
            },
          },
        },
      }),
      new Plugin({
        key: new PluginKey('bidirectional-link-tracker'),
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
