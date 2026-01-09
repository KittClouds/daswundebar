/**
 * CozoDB Types Stub - PRESERVED
 * 
 * These type definitions are preserved from the old browser CozoDB
 * as they are referenced by highlighters and other components.
 * No runtime code, just types.
 */

export type LinkType = 'mention' | 'explicit' | 'implicit';
export type MentionType = 'explicit' | 'implicit' | 'inferred';
export type LinkCreatedBy = 'user' | 'auto-extraction' | 'ai-inference';
export type PositionType = 'title' | 'heading' | 'body' | 'footnote';

export interface EntityMentionEvent {
    type: 'entityMentioned' | 'entityRemoved';
    noteId: string;
    entityId: string;
    mention: {
        text: string;
        position: number;
        context: string;
        mentionType: MentionType;
        positionType: PositionType;
    };
    timestamp: number;
}

export type GraphScope = 'note' | 'folder' | 'vault';

export interface CozoQueryResult<T = unknown> {
    ok: boolean;
    rows?: T[];
    headers?: string[];
    took?: number;
    message?: string;
}
