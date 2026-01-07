/**
 * RustHighlighter Unit Tests
 * 
 * Pure logic tests for the decoration building utilities.
 * No external dependencies, no mocking required.
 */

import { describe, it, expect } from 'vitest';

describe('RustHighlighter Logic', () => {

    describe('Text Extraction', () => {
        // extractText function logic
        function extractText(doc: { descendants: (cb: (node: any) => void) => void }): string {
            let text = '';
            doc.descendants((node: any) => {
                if (node.isText && node.text) {
                    text += node.text;
                } else if (node.isBlock) {
                    text += '\n';
                }
            });
            return text;
        }

        it('should extract text from simple document structure', () => {
            const mockDoc = {
                descendants: (callback: (node: any) => void) => {
                    callback({ isText: true, text: 'Hello ' });
                    callback({ isText: true, text: 'World' });
                    callback({ isBlock: true });
                },
            };

            expect(extractText(mockDoc)).toBe('Hello World\n');
        });

        it('should handle empty documents', () => {
            const mockDoc = {
                descendants: (_callback: (node: any) => void) => { },
            };

            expect(extractText(mockDoc)).toBe('');
        });

        it('should handle mixed content', () => {
            const mockDoc = {
                descendants: (callback: (node: any) => void) => {
                    callback({ isText: true, text: 'Line 1' });
                    callback({ isBlock: true });
                    callback({ isText: true, text: 'Line 2' });
                },
            };

            expect(extractText(mockDoc)).toBe('Line 1\nLine 2');
        });
    });

    describe('Position Map Building', () => {
        function buildPositionMap(nodes: Array<{ isText: boolean; text?: string; pos: number }>): number[] {
            const map: number[] = [];
            let textIndex = 0;

            for (const node of nodes) {
                if (node.isText && node.text) {
                    for (let i = 0; i < node.text.length; i++) {
                        map[textIndex++] = node.pos + i;
                    }
                }
            }

            return map;
        }

        it('should build correct position map for single text node', () => {
            const nodes = [{ isText: true, text: 'ABC', pos: 1 }];
            expect(buildPositionMap(nodes)).toEqual([1, 2, 3]);
        });

        it('should handle multiple text nodes', () => {
            const nodes = [
                { isText: true, text: 'ABC', pos: 1 },
                { isText: true, text: 'DE', pos: 5 },
            ];
            expect(buildPositionMap(nodes)).toEqual([1, 2, 3, 5, 6]);
        });

        it('should handle gaps between nodes', () => {
            const nodes = [
                { isText: true, text: 'A', pos: 1 },
                { isText: true, text: 'B', pos: 10 },
            ];
            expect(buildPositionMap(nodes)).toEqual([1, 10]);
        });
    });

    describe('Highlighting Modes', () => {
        type HighlightMode = 'off' | 'clean' | 'vivid' | 'focus';

        function shouldDecorate(mode: HighlightMode): boolean {
            return mode !== 'off';
        }

        function getStyleForMode(mode: HighlightMode): { color: string; bgColor: string } {
            if (mode === 'clean') {
                return { color: 'inherit', bgColor: 'transparent' };
            }
            return { color: 'hsl(var(--primary))', bgColor: 'hsl(var(--primary) / 0.15)' };
        }

        it('should NOT decorate in OFF mode', () => {
            expect(shouldDecorate('off')).toBe(false);
        });

        it('should decorate in all other modes', () => {
            expect(shouldDecorate('clean')).toBe(true);
            expect(shouldDecorate('vivid')).toBe(true);
            expect(shouldDecorate('focus')).toBe(true);
        });

        it('should use minimal styling in CLEAN mode', () => {
            const style = getStyleForMode('clean');
            expect(style.color).toBe('inherit');
            expect(style.bgColor).toBe('transparent');
        });

        it('should use full colors in VIVID mode', () => {
            const style = getStyleForMode('vivid');
            expect(style.color).toContain('primary');
            expect(style.bgColor).toContain('primary');
        });
    });

    describe('Focus Mode Filtering', () => {
        function shouldShowInFocusMode(entityKind: string, focusKinds: string[]): boolean {
            if (focusKinds.length === 0) return true;
            return focusKinds.includes(entityKind);
        }

        it('should show all entities when focus list is empty', () => {
            expect(shouldShowInFocusMode('CHARACTER', [])).toBe(true);
            expect(shouldShowInFocusMode('LOCATION', [])).toBe(true);
            expect(shouldShowInFocusMode('ITEM', [])).toBe(true);
        });

        it('should show included entities', () => {
            const focusKinds = ['CHARACTER', 'LOCATION'];
            expect(shouldShowInFocusMode('CHARACTER', focusKinds)).toBe(true);
            expect(shouldShowInFocusMode('LOCATION', focusKinds)).toBe(true);
        });

        it('should hide excluded entities', () => {
            const focusKinds = ['CHARACTER', 'LOCATION'];
            expect(shouldShowInFocusMode('ITEM', focusKinds)).toBe(false);
            expect(shouldShowInFocusMode('EVENT', focusKinds)).toBe(false);
        });
    });

    describe('Range Overlap Detection', () => {
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

        it('should detect overlapping ranges', () => {
            const ranges: Range[] = [[0, 5], [10, 15], [20, 25]];

            expect(rangesOverlap(ranges, 3, 7)).toBe(true);   // Overlaps [0,5]
            expect(rangesOverlap(ranges, 12, 18)).toBe(true); // Overlaps [10,15]
            expect(rangesOverlap(ranges, 0, 5)).toBe(true);   // Exact match
        });

        it('should detect non-overlapping ranges', () => {
            const ranges: Range[] = [[0, 5], [10, 15], [20, 25]];

            expect(rangesOverlap(ranges, 5, 10)).toBe(false); // Between ranges
            expect(rangesOverlap(ranges, 25, 30)).toBe(false); // After all
            expect(rangesOverlap(ranges, -5, 0)).toBe(false);  // Before all
        });

        it('should handle empty range list', () => {
            const ranges: Range[] = [];
            expect(rangesOverlap(ranges, 0, 10)).toBe(false);
        });

        it('should insert ranges in sorted order', () => {
            const ranges: Range[] = [];
            insertRange(ranges, 10, 15);
            insertRange(ranges, 0, 5);
            insertRange(ranges, 20, 25);
            insertRange(ranges, 5, 10);

            expect(ranges).toEqual([[0, 5], [5, 10], [10, 15], [20, 25]]);
        });
    });

    describe('Editing Detection', () => {
        function isEditing(
            selection: { from: number; to: number },
            range: { from: number; to: number }
        ): boolean {
            return (
                (selection.from >= range.from && selection.from <= range.to) ||
                (selection.to >= range.from && selection.to <= range.to) ||
                (selection.from <= range.from && selection.to >= range.to)
            );
        }

        it('should detect cursor inside range', () => {
            expect(isEditing({ from: 5, to: 5 }, { from: 0, to: 10 })).toBe(true);
            expect(isEditing({ from: 0, to: 0 }, { from: 0, to: 10 })).toBe(true);
            expect(isEditing({ from: 10, to: 10 }, { from: 0, to: 10 })).toBe(true);
        });

        it('should detect cursor outside range', () => {
            expect(isEditing({ from: 15, to: 15 }, { from: 0, to: 10 })).toBe(false);
            expect(isEditing({ from: -5, to: -5 }, { from: 0, to: 10 })).toBe(false);
        });

        it('should detect selection spanning range', () => {
            expect(isEditing({ from: 0, to: 20 }, { from: 5, to: 15 })).toBe(true);
        });

        it('should detect partial selection overlap', () => {
            expect(isEditing({ from: 8, to: 12 }, { from: 0, to: 10 })).toBe(true);
            expect(isEditing({ from: -5, to: 5 }, { from: 0, to: 10 })).toBe(true);
        });
    });

    describe('Widget Mode Logic', () => {
        function shouldRenderWidget(
            useWidgetMode: boolean,
            patternWidgetMode: boolean,
            isEditing: boolean
        ): boolean {
            return (useWidgetMode || patternWidgetMode) && !isEditing;
        }

        it('should render widget when enabled and not editing', () => {
            expect(shouldRenderWidget(true, false, false)).toBe(true);
            expect(shouldRenderWidget(false, true, false)).toBe(true);
            expect(shouldRenderWidget(true, true, false)).toBe(true);
        });

        it('should NOT render widget when editing', () => {
            expect(shouldRenderWidget(true, false, true)).toBe(false);
            expect(shouldRenderWidget(false, true, true)).toBe(false);
            expect(shouldRenderWidget(true, true, true)).toBe(false);
        });

        it('should NOT render widget when disabled', () => {
            expect(shouldRenderWidget(false, false, false)).toBe(false);
        });
    });

    describe('Content Hash Caching', () => {
        function computeContentHash(text: string): string {
            let hash = 0;
            for (let i = 0; i < text.length; i++) {
                hash = ((hash << 5) - hash + text.charCodeAt(i)) | 0;
            }
            return hash.toString(16);
        }

        it('should compute consistent hash', () => {
            const text = 'Hello World';
            const hash1 = computeContentHash(text);
            const hash2 = computeContentHash(text);

            expect(hash1).toBe(hash2);
        });

        it('should produce different hashes for different content', () => {
            const hash1 = computeContentHash('Hello');
            const hash2 = computeContentHash('World');
            const hash3 = computeContentHash('Hello World');

            expect(hash1).not.toBe(hash2);
            expect(hash1).not.toBe(hash3);
            expect(hash2).not.toBe(hash3);
        });

        it('should handle empty string', () => {
            const hash = computeContentHash('');
            expect(hash).toBe('0');
        });

        it('should handle unicode', () => {
            const hash1 = computeContentHash('日本語');
            const hash2 = computeContentHash('日本語');
            expect(hash1).toBe(hash2);
        });
    });

    describe('Implicit Decoration Positioning', () => {
        function calculateDecorationPosition(
            mention: { start: number; end: number },
            positionMap: number[]
        ): { from: number; to: number } | null {
            const from = positionMap[mention.start];
            const to = positionMap[mention.end - 1] !== undefined
                ? positionMap[mention.end - 1] + 1
                : positionMap[mention.start] + (mention.end - mention.start);

            if (from === undefined || to === undefined) return null;
            return { from, to };
        }

        it('should calculate correct positions for mention', () => {
            const positionMap = [1, 2, 3, 4, 5, 6, 7];
            const mention = { start: 0, end: 7 };

            const result = calculateDecorationPosition(mention, positionMap);
            expect(result).toEqual({ from: 1, to: 8 });
        });

        it('should handle partial mentions', () => {
            const positionMap = [1, 2, 3, 4, 5];
            const mention = { start: 1, end: 4 };

            const result = calculateDecorationPosition(mention, positionMap);
            expect(result).toEqual({ from: 2, to: 5 });
        });

        it('should return null for out-of-bounds mention', () => {
            const positionMap = [1, 2, 3];
            const mention = { start: 10, end: 15 };

            const result = calculateDecorationPosition(mention, positionMap);
            expect(result).toBeNull();
        });
    });

    describe('Entity Kind Color Mapping', () => {
        function getEntityColor(entityKind: string): string {
            const varName = `--entity-${entityKind.toLowerCase().replace('_', '-')}`;
            return `hsl(var(${varName}))`;
        }

        it('should generate correct CSS variable name', () => {
            expect(getEntityColor('CHARACTER')).toBe('hsl(var(--entity-character))');
            expect(getEntityColor('LOCATION')).toBe('hsl(var(--entity-location))');
            expect(getEntityColor('EVENT')).toBe('hsl(var(--entity-event))');
        });

        it('should handle underscored kinds', () => {
            expect(getEntityColor('MAGIC_SYSTEM')).toBe('hsl(var(--entity-magic-system))');
        });
    });

    describe('Alias Match Styling', () => {
        function getBorderStyle(isAliasMatch: boolean, isVivid: boolean): string {
            if (isVivid) return 'solid';
            return isAliasMatch ? 'dotted' : 'solid';
        }

        it('should use solid border in vivid mode', () => {
            expect(getBorderStyle(false, true)).toBe('solid');
            expect(getBorderStyle(true, true)).toBe('solid');
        });

        it('should use dotted border for alias matches in non-vivid mode', () => {
            expect(getBorderStyle(true, false)).toBe('dotted');
        });

        it('should use solid border for direct matches', () => {
            expect(getBorderStyle(false, false)).toBe('solid');
        });
    });
});
