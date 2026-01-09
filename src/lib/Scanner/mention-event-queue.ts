import { eventBus } from '@/lib/utils/event-bus';
import type { EntityMentionEvent } from '@/lib/cozo-stubs/types';

class MentionEventQueue {
  private queue: EntityMentionEvent[] = [];
  private processing = false;
  private syncTimer: ReturnType<typeof setTimeout> | null = null;

  enqueue(event: EntityMentionEvent): void {
    this.queue.push(event);
    this.scheduleProcessing();
  }

  private scheduleProcessing(): void {
    if (this.processing) return;

    this.processing = true;

    queueMicrotask(() => {
      this.processQueue();
      this.processing = false;
    });
  }

  private processQueue(): void {
    if (this.queue.length === 0) return;

    if (this.syncTimer) {
      clearTimeout(this.syncTimer);
    }

    this.syncTimer = setTimeout(() => {
      const events = [...this.queue];
      this.queue = [];

      eventBus.emit('mentionEventsBatch', events);
    }, 1000);
  }

  flush(): void {
    if (this.syncTimer) {
      clearTimeout(this.syncTimer);
    }
    if (this.queue.length > 0) {
      const events = [...this.queue];
      this.queue = [];
      eventBus.emit('mentionEventsBatch', events);
    }
  }

  clear(): void {
    this.queue = [];
    if (this.syncTimer) {
      clearTimeout(this.syncTimer);
      this.syncTimer = null;
    }
  }
}

export const mentionEventQueue = new MentionEventQueue();
