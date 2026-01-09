/**
 * Calendar Bridge (DEPRECATED)
 * 
 * This file is kept for backwards compatibility.
 * New code should use: import { contentAPI } from '@/lib/tauri/content-api'
 */

import { contentAPI, type CozoCalEvent, type CozoPeriod } from './content-api';

// ============================================================================
// Types (kept for backwards compat)
// ============================================================================

export interface SurrealCalEvent {
    id: string;
    world_id: string;
    calendar_id: string;
    title: string;
    description?: string | null;
    date_year: number;
    date_month: number;
    date_day: number;
    date_hour?: number | null;
    date_minute?: number | null;
    era_id?: string | null;
    end_year?: number | null;
    end_month?: number | null;
    end_day?: number | null;
    is_all_day: boolean;
    recurrence?: unknown;
    parent_event_id?: string | null;
    importance: string;
    category: string;
    tags?: string[] | null;
    color?: string | null;
    icon?: string | null;
    entity_id?: string | null;
    entity_kind?: string | null;
    source_note_id?: string | null;
    created_at?: string | null;
    updated_at?: string | null;
}

export interface SurrealPeriod {
    id: string;
    world_id: string;
    calendar_id: string;
    name: string;
    description?: string | null;
    start_year: number;
    start_month?: number | null;
    end_year?: number | null;
    end_month?: number | null;
    parent_period_id?: string | null;
    period_type: string;
    color: string;
    icon?: string | null;
    abbreviation?: string | null;
    direction: string;
    triggered_by?: string | null;
    ends_when?: string | null;
    major_events?: string[] | null;
    arc_type?: string | null;
    dominant_theme?: string | null;
    protagonist_id?: string | null;
    antagonist_id?: string | null;
    summary?: string | null;
    detailed_notes?: string | null;
    show_on_timeline: boolean;
    timeline_color?: string | null;
    timeline_icon?: string | null;
    created_at?: string | null;
    updated_at?: string | null;
}

export interface SurrealOccursOn {
    id: string;
    world_id: string;
    in_: string;
    out: string;
    role: string;
    significance: string;
    notes?: string | null;
    created_at?: string | null;
}

export interface FantasyDate {
    year: number;
    monthIndex: number;
    dayIndex: number;
    hour?: number;
    minute?: number;
    eraId?: string;
}

// ============================================================================
// Adapters
// ============================================================================

function cozoToCalEvent(c: CozoCalEvent): SurrealCalEvent {
    return {
        id: c.id,
        world_id: c.world_id,
        calendar_id: c.calendar_id,
        title: c.title,
        description: c.description,
        date_year: c.date_year,
        date_month: c.date_month,
        date_day: c.date_day,
        date_hour: c.date_hour,
        date_minute: c.date_minute,
        era_id: c.era_id,
        end_year: c.end_year,
        end_month: c.end_month,
        end_day: c.end_day,
        is_all_day: c.is_all_day,
        recurrence: c.recurrence,
        parent_event_id: c.parent_event_id,
        importance: c.importance,
        category: c.category,
        tags: c.tags,
        color: c.color,
        icon: c.icon,
        entity_id: c.entity_id,
        entity_kind: c.entity_kind,
        source_note_id: c.source_note_id,
        created_at: new Date(c.created_at * 1000).toISOString(),
        updated_at: new Date(c.updated_at * 1000).toISOString(),
    };
}

function cozoToPeriod(c: CozoPeriod): SurrealPeriod {
    return {
        id: c.id,
        world_id: c.world_id,
        calendar_id: c.calendar_id,
        name: c.name,
        description: c.description,
        start_year: c.start_year,
        start_month: c.start_month,
        end_year: c.end_year,
        end_month: c.end_month,
        parent_period_id: c.parent_period_id,
        period_type: c.period_type,
        color: c.color,
        icon: c.icon,
        abbreviation: c.abbreviation,
        direction: c.direction,
        triggered_by: c.triggered_by,
        ends_when: c.ends_when,
        major_events: c.major_events,
        arc_type: c.arc_type,
        dominant_theme: c.dominant_theme,
        protagonist_id: c.protagonist_id,
        antagonist_id: c.antagonist_id,
        summary: c.summary,
        detailed_notes: c.detailed_notes,
        show_on_timeline: c.show_on_timeline,
        timeline_color: c.timeline_color,
        timeline_icon: c.timeline_icon,
        created_at: new Date(c.created_at * 1000).toISOString(),
        updated_at: new Date(c.updated_at * 1000).toISOString(),
    };
}

const DEFAULT_WORLD = 'default';

// ============================================================================
// Calendar Event Functions
// ============================================================================

async function createEvent(
    calendarId: string,
    title: string,
    date: FantasyDate,
    options: {
        description?: string;
        endDate?: { year: number; monthIndex: number; dayIndex: number };
        isAllDay?: boolean;
        importance?: string;
        category?: string;
        tags?: string[];
        color?: string;
        icon?: string;
        entityId?: string;
        entityKind?: string;
        sourceNoteId?: string;
    } = {}
): Promise<SurrealCalEvent> {
    const result = await contentAPI.createCalEvent({
        calendarId,
        title,
        dateYear: date.year,
        dateMonth: date.monthIndex,
        dateDay: date.dayIndex,
        dateHour: date.hour,
        dateMinute: date.minute,
        eraId: date.eraId,
        description: options.description,
        endYear: options.endDate?.year,
        endMonth: options.endDate?.monthIndex,
        endDay: options.endDate?.dayIndex,
        isAllDay: options.isAllDay,
        importance: options.importance,
        category: options.category,
        tags: options.tags,
        color: options.color,
        icon: options.icon,
        entityId: options.entityId,
        entityKind: options.entityKind,
        sourceNoteId: options.sourceNoteId,
    });
    return cozoToCalEvent(result);
}

async function getEvent(id: string): Promise<SurrealCalEvent | null> {
    const result = await contentAPI.getCalEvent(id);
    return result ? cozoToCalEvent(result) : null;
}

async function listEvents(_calendarId: string): Promise<SurrealCalEvent[]> {
    const results = await contentAPI.listCalEvents();
    return results.map(cozoToCalEvent);
}

async function getEventsByMonth(_calendarId: string, year: number, month: number): Promise<SurrealCalEvent[]> {
    const results = await contentAPI.listCalEventsByMonth(year, month);
    return results.map(cozoToCalEvent);
}

async function updateEvent(_id: string, _updates: Record<string, unknown>): Promise<SurrealCalEvent> {
    console.warn('[CalendarBridge] updateEvent not yet implemented in CozoDB backend');
    throw new Error('updateEvent not implemented');
}

async function deleteEvent(id: string): Promise<void> {
    await contentAPI.deleteCalEvent(id);
}

// ============================================================================
// Period Functions
// ============================================================================

async function createPeriod(
    calendarId: string,
    name: string,
    startYear: number,
    color: string,
    options: {
        description?: string;
        startMonth?: number;
        endYear?: number;
        endMonth?: number;
        parentPeriodId?: string;
        periodType?: string;
        icon?: string;
        abbreviation?: string;
        direction?: string;
        triggeredBy?: string;
        endsWhen?: string;
        arcType?: string;
        dominantTheme?: string;
        protagonistId?: string;
        antagonistId?: string;
        summary?: string;
        detailedNotes?: string;
        showOnTimeline?: boolean;
        timelineColor?: string;
        timelineIcon?: string;
    } = {}
): Promise<SurrealPeriod> {
    const result = await contentAPI.createPeriod({
        calendarId,
        name,
        startYear,
        color,
        ...options,
    });
    return cozoToPeriod(result);
}

async function getPeriod(id: string): Promise<SurrealPeriod | null> {
    const result = await contentAPI.getPeriod(id);
    return result ? cozoToPeriod(result) : null;
}

async function listPeriods(_calendarId: string): Promise<SurrealPeriod[]> {
    const results = await contentAPI.listPeriods();
    return results.map(cozoToPeriod);
}

async function updatePeriod(_id: string, _updates: Record<string, unknown>): Promise<SurrealPeriod> {
    console.warn('[CalendarBridge] updatePeriod not yet implemented in CozoDB backend');
    throw new Error('updatePeriod not implemented');
}

async function deletePeriod(id: string): Promise<void> {
    await contentAPI.deletePeriod(id);
}

// ============================================================================
// Stubs for unsupported operations
// ============================================================================

async function getEventsRange(..._args: unknown[]): Promise<SurrealCalEvent[]> {
    console.warn('[CalendarBridge] getEventsRange not yet implemented');
    return [];
}

async function getEventsByEntity(..._args: unknown[]): Promise<SurrealCalEvent[]> {
    console.warn('[CalendarBridge] getEventsByEntity not yet implemented');
    return [];
}

async function linkEntityToEvent(..._args: unknown[]): Promise<SurrealOccursOn> {
    console.warn('[CalendarBridge] linkEntityToEvent not yet implemented');
    throw new Error('linkEntityToEvent not implemented');
}

async function unlinkEntityFromEvent(..._args: unknown[]): Promise<void> {
    console.warn('[CalendarBridge] unlinkEntityFromEvent not yet implemented');
}

async function getEventParticipants(..._args: unknown[]): Promise<SurrealOccursOn[]> {
    console.warn('[CalendarBridge] getEventParticipants not yet implemented');
    return [];
}

async function getPeriodChildren(parentId: string): Promise<SurrealPeriod[]> {
    const results = await contentAPI.getPeriodChildren(parentId);
    return results.map(cozoToPeriod);
}

async function getPeriodEvents(..._args: unknown[]): Promise<SurrealCalEvent[]> {
    console.warn('[CalendarBridge] getPeriodEvents not yet implemented');
    return [];
}

async function linkEventToPeriod(..._args: unknown[]): Promise<unknown> {
    console.warn('[CalendarBridge] linkEventToPeriod not yet implemented');
    throw new Error('linkEventToPeriod not implemented');
}

async function unlinkEventFromPeriod(..._args: unknown[]): Promise<void> {
    console.warn('[CalendarBridge] unlinkEventFromPeriod not yet implemented');
}

// ============================================================================
// Convenience Wrapper
// ============================================================================

export const calendarBridge = {
    // Events
    createEvent,
    getEvent,
    listEvents,
    getEventsRange,
    getEventsByMonth,
    updateEvent,
    deleteEvent,
    getEventsByEntity,
    linkEntityToEvent,
    unlinkEntityFromEvent,
    getEventParticipants,

    // Periods
    createPeriod,
    getPeriod,
    listPeriods,
    updatePeriod,
    deletePeriod,
    getPeriodChildren,
    getPeriodEvents,
    linkEventToPeriod,
    unlinkEventFromPeriod,
};

export default calendarBridge;
