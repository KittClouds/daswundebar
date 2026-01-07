import type { EntityKind } from '@/lib/types/entityTypes';
// LEGACY: This file is not actively used but kept for future LLM extraction
import type { RegisteredEntity } from '@/lib/tauri';

/**
 * Schema for entity extraction
 */
const EXTRACTION_SCHEMA = `{
  "entities": [
    {
      "label": "string (entity name)",
      "kind": "CHARACTER|LOCATION|FACTION|ITEM|EVENT|SCENE|CONCEPT",
      "confidence": "number (0-1)"
    }
  ],
  "relationships": [
    {
      "source": "string (entity label)",
      "target": "string (entity label)",
      "type": "string (relationship type: ally_of, enemy_of, sibling, located_in, belongs_to, etc.)"
    }
  ],
  "coOccurrences": [
    {
      "entities": ["string (entity labels that appear together)"],
      "context": "string (brief description of where they appear together)"
    }
  ]
}`;

/**
 * Build extraction prompt from document context
 */
export class PromptTemplateBuilder {
  /**
   * Build system prompt for entity extraction
   */
  buildSystemPrompt(options: {
    explicitEntities: Array<{ label: string; kind: EntityKind }>;
    registryEntities: RegisteredEntity[];
    includeRelationships: boolean;
    includeCoOccurrences: boolean;
  }): string {
    const {
      explicitEntities,
      registryEntities,
      includeRelationships,
      includeCoOccurrences,
    } = options;

    let prompt = `You are an entity extraction assistant for narrative writing.
Your task is to extract entities, relationships, and co-occurrences from text.

Return data as a JSON object with the following schema:
${EXTRACTION_SCHEMA}

`;

    // Add known entities section
    if (explicitEntities.length > 0) {
      prompt += `\nKNOWN ENTITIES IN THIS DOCUMENT (Do not duplicate these if already extracted given context):\n`;
      for (const entity of explicitEntities) {
        prompt += `- "${entity.label}" is a ${entity.kind}\n`;
      }
    }

    // Add entity type hints based on registry patterns
    if (registryEntities.length > 0) {
      const kindCounts: Record<string, number> = {};
      for (const entity of registryEntities) {
        kindCounts[entity.kind] = (kindCounts[entity.kind] || 0) + 1;
      }

      prompt += `\nREGISTERED ENTITY TYPES IN THIS PROJECT (Use these as strong hints):\n`;
      // Take top 5 most common kinds
      const topKinds = Object.entries(kindCounts).sort((a, b) => b[1] - a[1]).slice(0, 5);

      for (const [kind, count] of topKinds) {
        prompt += `- ${kind} (${count} registered)\n`;
      }
    }

    // Add relationship extraction rules
    if (includeRelationships) {
      prompt += `\nRELATIONSHIPS TO EXTRACT:
- Family: "X's brother/sister/father/mother Y" → sibling/parent_of
- Social: "X and Y are friends/enemies/allies" → friend_of/enemy_of/ally_of
- Location: "X lives in/travels to Y" → located_in/travels_to
- Faction: "X belongs to/leads Y" → member_of/leads

Only extract relationships you can clearly identify from the text.
`;
    }

    // Add co-occurrence rules
    if (includeCoOccurrences) {
      prompt += `\nCO-OCCURRENCES:
- List entities that appear in the same paragraph or scene interaction
- Include brief context describing where/how they appear together
`;
    }

    prompt += `\nIMPORTANT RULES:
1. Only extract named entities (proper nouns like "Jon", "Winterfell")
2. Do NOT extract common nouns (like "the king", "a soldier") unless they're proper names
3. Confidence should be 0.0-1.0 (use 0.8+ for clear entities)
4. Return valid JSON only.

Begin extraction.`;

    return prompt;
  }
}

export const promptTemplateBuilder = new PromptTemplateBuilder();
