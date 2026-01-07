/**
 * RegexEntityParser - Extract explicit entity syntax from documents
 * 
 * Phase 1: Full implementation
 * Handles:
 * - [KIND|Label]
 * - [KIND:SUBTYPE|Label]
 * - [KIND|Label|{metadata}]
 * - [KIND|Label|{" aliases":["Alias1"]}]
 */

import type { JSONContent } from '@tiptap/react';
import type { EntityKind } from '@/lib/types/entityTypes';
import type { RegisteredEntity } from '@/lib/tauri';
import { getContextExtractor } from './context-extractor';

export interface ParsedEntity {
    type: 'explicit' | 'implicit';
    kind: EntityKind;
    label: string;
    subtype?: string;
    metadata?: Record<string, any>;
    position?: { start: number; end: number };
    context?: string;
}

export class RegexEntityParser {
    /**
     * Parse explicit entity declarations from TipTap document
     */
    parseFromDocument(content: JSONContent): ParsedEntity[] {
        const fullText = this.extractPlainText(content);
        return this.parseFromText(fullText);
    }

    /**
     * Parse explicit entity declarations from plain text
     */
    parseFromText(text: string): ParsedEntity[] {
        const entities: ParsedEntity[] = [];

        // Regex for entity syntax: [KIND|Label] or [KIND:SUBTYPE|Label|{attrs}]
        const regex = /\[([A-Z_]+)(?::([A-Z_]+))?\|([^\]|]+)(?:\|(\{[^}]+\}))?\]/g;
        let match: RegExpExecArray | null;

        while ((match = regex.exec(text)) !== null) {
            const [fullMatch, kind, subtype, label, attrsJSON] = match;
            const position = {
                start: match.index,
                end: match.index + fullMatch.length,
            };

            // Extract surrounding context
            const contextExtractor = getContextExtractor();
            const extraction = contextExtractor.extractContext(text, position.start, position.end, 1);
            const context = extraction.snippet;

            // Parse metadata/attributes JSON if present
            let metadata: Record<string, any> | undefined;
            if (attrsJSON) {
                try {
                    // Handle both single and double quotes
                    const normalized = attrsJSON.replace(/'/g, '"');
                    metadata = JSON.parse(normalized);
                } catch (error) {
                    console.warn(`Failed to parse entity metadata: ${attrsJSON}`, error);
                }
            }

            entities.push({
                type: 'explicit',
                kind: kind as EntityKind,
                label: label.trim(),
                subtype: subtype?.trim(),
                metadata,
                position,
                context,
            });
        }

        return entities;
    }

    /**
     * Extract plain text from TipTap JSONContent
     */
    private extractPlainText(node: JSONContent): string {
        if (!node) return '';

        if (node.type === 'text' && node.text) {
            return node.text;
        }

        if (node.content && Array.isArray(node.content)) {
            return node.content.map(child => this.extractPlainText(child)).join(' ');
        }

        return '';
    }

}

// Singleton instance
export const regexEntityParser = new RegexEntityParser();
