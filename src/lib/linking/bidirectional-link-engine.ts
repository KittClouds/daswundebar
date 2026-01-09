import { v4 as uuidv4 } from 'uuid';
import { cozoDb } from '@/lib/cozo-stubs/db';
import { eventBus } from '@/lib/utils/event-bus';
import { BIDIRECTIONAL_LINK_QUERIES } from '@/lib/cozo-stubs/schema';
import type { EntityMentionEvent, PositionType } from '@/lib/cozo-stubs/types';

interface EntityInNoteResult {
  entityId: string;
  entityName: string;
  entityKind: string;
  mentionCount: number;
  avgRelevance: number;
}

interface NoteWithEntityResult {
  noteId: string;
  noteTitle: string;
  mentionCount: number;
  avgRelevance: number;
  updatedAt: number;
}

class BidirectionalLinkEngine {
  private initialized = false;

  constructor() {
    eventBus.on<EntityMentionEvent[]>('mentionEventsBatch', (events) => {
      this.processMentionBatch(events);
    });
  }

  private async ensureInitialized(): Promise<boolean> {
    if (this.initialized) return true;

    try {
      await cozoDb.init();
      this.initialized = true;
      return true;
    } catch {
      return false;
    }
  }

  private async processMentionBatch(events: EntityMentionEvent[]): Promise<void> {
    const ready = await this.ensureInitialized();
    if (!ready) return;

    try {
      const byNote = new Map<string, EntityMentionEvent[]>();

      for (const event of events) {
        if (!byNote.has(event.noteId)) {
          byNote.set(event.noteId, []);
        }
        byNote.get(event.noteId)!.push(event);
      }

      for (const [noteId, noteEvents] of byNote) {
        await this.syncNoteLinks(noteId, noteEvents);
      }

      eventBus.emit('linksUpdated', { noteCount: byNote.size, eventCount: events.length });
    } catch (error) {
      console.error('[BidirectionalLinkEngine] Error processing batch:', error);
    }
  }

  private async syncNoteLinks(noteId: string, events: EntityMentionEvent[]): Promise<void> {
    try {
      cozoDb.runQuery(BIDIRECTIONAL_LINK_QUERIES.deleteLinksByNote, { note_id: noteId });
    } catch {
      // Relation may not exist yet
    }

    try {
      cozoDb.runQuery(BIDIRECTIONAL_LINK_QUERIES.deleteBacklinksByNote, { note_id: noteId });
    } catch {
      // Relation may not exist yet
    }

    for (const event of events) {
      if (event.type === 'entityMentioned') {
        await this.createLink(event);
      }
    }

    await this.updateBacklinksForNote(noteId, events);
  }

  private async createLink(event: EntityMentionEvent): Promise<void> {
    const linkId = uuidv4();
    const { noteId, entityId, mention } = event;

    const relevance = this.calculateRelevance(mention);

    try {
      cozoDb.runQuery(BIDIRECTIONAL_LINK_QUERIES.createLink, {
        id: linkId,
        source_id: noteId,
        target_id: entityId,
        link_type: 'mention',
        mention_type: mention.mentionType,
        created_by: 'auto-extraction',
        context: mention.context,
        char_position: mention.position,
        sentence_index: null,
        position_type: mention.positionType,
        position_weight: this.getPositionWeight(mention.positionType),
        relevance,
        frequency_score: 0.0,
        context_score: 0.0,
        temporal_score: 0.0,
        confidence: 1.0,
        validated: false,
      });
    } catch (error) {
      console.error('[BidirectionalLinkEngine] Failed to create link:', error);
    }
  }

  private async updateBacklinksForNote(noteId: string, events: EntityMentionEvent[]): Promise<void> {
    const byEntity = new Map<string, EntityMentionEvent[]>();

    for (const event of events) {
      if (event.type === 'entityMentioned') {
        if (!byEntity.has(event.entityId)) {
          byEntity.set(event.entityId, []);
        }
        byEntity.get(event.entityId)!.push(event);
      }
    }

    const noteTitle = 'Untitled';

    for (const [entityId, entityEvents] of byEntity) {
      const mentionCount = entityEvents.length;
      const avgRelevance = entityEvents.reduce(
        (sum, e) => sum + this.calculateRelevance(e.mention),
        0
      ) / mentionCount;
      const positions = entityEvents.map(e => e.mention.position).sort((a, b) => a - b);

      try {
        cozoDb.runQuery(BIDIRECTIONAL_LINK_QUERIES.upsertBacklink, {
          id: uuidv4(),
          entity_id: entityId,
          note_id: noteId,
          mention_count: mentionCount,
          avg_relevance: avgRelevance,
          first_mention_pos: positions[0],
          last_mention_pos: positions[positions.length - 1],
          note_title: noteTitle,
          created_at: Date.now() / 1000,
        });
      } catch (error) {
        console.error('[BidirectionalLinkEngine] Failed to upsert backlink:', error);
      }
    }
  }

  private calculateRelevance(mention: EntityMentionEvent['mention']): number {
    const positionWeight = this.getPositionWeight(mention.positionType);
    const contextWeight = mention.context.length > 50 ? 0.8 : 0.5;

    return positionWeight * 0.6 + contextWeight * 0.4;
  }

  private getPositionWeight(positionType: PositionType): number {
    const weights: Record<PositionType, number> = {
      title: 1.0,
      heading: 0.8,
      body: 0.5,
      footnote: 0.3,
    };
    return weights[positionType] || 0.5;
  }

  async getEntitiesInNote(noteId: string): Promise<EntityInNoteResult[]> {
    const ready = await this.ensureInitialized();
    if (!ready) return [];

    try {
      const result = cozoDb.runQuery(BIDIRECTIONAL_LINK_QUERIES.getEntitiesInNote, { note_id: noteId });
      if (!result.ok || !result.rows) return [];

      return result.rows.map((row: unknown[]) => ({
        entityId: row[0] as string,
        entityName: row[1] as string,
        entityKind: row[2] as string,
        mentionCount: row[3] as number,
        avgRelevance: row[4] as number,
      }));
    } catch {
      return [];
    }
  }

  async getNotesWithEntity(entityId: string): Promise<NoteWithEntityResult[]> {
    const ready = await this.ensureInitialized();
    if (!ready) return [];

    try {
      const result = cozoDb.runQuery(BIDIRECTIONAL_LINK_QUERIES.getNotesWithEntity, { entity_id: entityId });
      if (!result.ok || !result.rows) return [];

      return result.rows.map((row: unknown[]) => ({
        noteId: row[0] as string,
        noteTitle: row[1] as string,
        mentionCount: row[2] as number,
        avgRelevance: row[3] as number,
        updatedAt: row[4] as number,
      }));
    } catch {
      return [];
    }
  }
}

export const bidirectionalLinkEngine = new BidirectionalLinkEngine();
