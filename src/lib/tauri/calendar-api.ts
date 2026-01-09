/**
 * Calendar API (DEPRECATED)
 * 
 * This file is kept for backwards compatibility.
 * New code should use: import { contentAPI } from '@/lib/tauri/content-api'
 */

import { isTauri } from '@/lib/tauri';
import { contentAPI, type CozoCalEvent, type CozoPeriod } from './content-api';
import type { CalendarEvent, Period } from '@/lib/fantasy-calendar/types';

// ============================================================================
// ADAPTERS - Convert CozoDB types to legacy CalendarEvent/Period types
// ============================================================================

function cozoToEvent(c: CozoCalEvent): CalendarEvent {
    return {
        id: c.id,
        calendarId: c.calendar_id,
        title: c.title,
        description: c.description || undefined,
        date: {
            year: c.date_year,
            monthIndex: c.date_month,
            dayIndex: c.date_day,
            hour: c.date_hour ?? undefined,
            minute: c.date_minute ?? undefined,
            eraId: c.era_id ?? undefined,
        },
        endDate: c.end_year ? {
            year: c.end_year,
            monthIndex: c.end_month ?? 0,
            dayIndex: c.end_day ?? 0,
        } : undefined,
        isAllDay: c.is_all_day,
        recurrence: c.recurrence as any,
        importance: c.importance as any,
        category: c.category,
        tags: c.tags || undefined,
        color: c.color || undefined,
        icon: c.icon || undefined,
        entityId: c.entity_id || undefined,
        entityKind: c.entity_kind || undefined,
        sourceNoteId: c.source_note_id || undefined,
        createdAt: c.created_at * 1000,
        updatedAt: c.updated_at * 1000,
    } as CalendarEvent;
}

function cozoToPeriod(c: CozoPeriod): Period {
    return {
        id: c.id,
        calendarId: c.calendar_id,
        name: c.name,
        description: c.description || undefined,
        startYear: c.start_year,
        startMonth: c.start_month ?? undefined,
        endYear: c.end_year ?? undefined,
        endMonth: c.end_month ?? undefined,
        parentPeriodId: c.parent_period_id || undefined,
        periodType: c.period_type as any,
        color: c.color,
        icon: c.icon || undefined,
        abbreviation: c.abbreviation || undefined,
        direction: c.direction as any,
        triggeredBy: c.triggered_by || undefined,
        endsWhen: c.ends_when || undefined,
        majorEvents: c.major_events || undefined,
        arcType: c.arc_type as any || undefined,
        dominantTheme: c.dominant_theme || undefined,
        protagonistId: c.protagonist_id || undefined,
        antagonistId: c.antagonist_id || undefined,
        summary: c.summary || undefined,
        detailedNotes: c.detailed_notes || undefined,
        showOnTimeline: c.show_on_timeline,
        timelineColor: c.timeline_color || undefined,
        timelineIcon: c.timeline_icon || undefined,
        createdAt: c.created_at * 1000,
        updatedAt: c.updated_at * 1000,
    } as Period;
}

// ============================================================================
// EVENT OPERATIONS
// ============================================================================

const DEFAULT_CALENDAR = 'primary';

export async function createEvent(event: Omit<CalendarEvent, 'id'>): Promise<CalendarEvent> {
    if (!isTauri()) throw new Error('Tauri not available');

    const result = await contentAPI.createCalEvent({
        calendarId: event.calendarId || DEFAULT_CALENDAR,
        title: event.title,
        dateYear: event.date.year,
        dateMonth: event.date.monthIndex,
        dateDay: event.date.dayIndex,
        description: event.description,
        dateHour: event.date.hour,
        dateMinute: event.date.minute,
        eraId: event.date.eraId,
        endYear: event.endDate?.year,
        endMonth: event.endDate?.monthIndex,
        endDay: event.endDate?.dayIndex,
        isAllDay: event.isAllDay,
        importance: event.importance,
        category: event.category,
        tags: event.tags,
        color: event.color,
        icon: event.icon,
        entityId: event.entityId,
        entityKind: event.entityKind,
        sourceNoteId: event.sourceNoteId,
    });

    return cozoToEvent(result);
}

export async function getEvent(id: string): Promise<CalendarEvent | null> {
    if (!isTauri()) return null;
    const result = await contentAPI.getCalEvent(id);
    return result ? cozoToEvent(result) : null;
}

export async function updateEvent(id: string, _updates: Partial<CalendarEvent>): Promise<CalendarEvent> {
    if (!isTauri()) throw new Error('Tauri not available');
    // Note: CozoDB doesn't have an update cal_event command yet
    // For now, fetch the existing event
    const existing = await contentAPI.getCalEvent(id);
    if (!existing) throw new Error(`Event not found: ${id}`);
    return cozoToEvent(existing);
}

export async function deleteEvent(id: string): Promise<void> {
    if (!isTauri()) throw new Error('Tauri not available');
    await contentAPI.deleteCalEvent(id);
}

export async function listEvents(_calendarId = DEFAULT_CALENDAR): Promise<CalendarEvent[]> {
    if (!isTauri()) return [];
    const results = await contentAPI.listCalEvents();
    return results.map(cozoToEvent);
}

export async function getEventsByMonth(
    _calendarId: string,
    year: number,
    month: number
): Promise<CalendarEvent[]> {
    if (!isTauri()) return [];
    const results = await contentAPI.listCalEventsByMonth(year, month);
    return results.map(cozoToEvent);
}

// ============================================================================
// PERIOD OPERATIONS
// ============================================================================

export async function createPeriod(period: Omit<Period, 'id'>): Promise<Period> {
    if (!isTauri()) throw new Error('Tauri not available');

    const result = await contentAPI.createPeriod({
        calendarId: period.calendarId || DEFAULT_CALENDAR,
        name: period.name,
        startYear: period.startYear,
        color: period.color || '#6366f1',
        description: period.description,
        startMonth: period.startMonth,
        endYear: period.endYear,
        endMonth: period.endMonth,
        parentPeriodId: period.parentPeriodId,
        periodType: period.periodType,
        icon: period.icon,
        abbreviation: period.abbreviation,
        direction: period.direction,
        triggeredBy: period.triggeredBy,
        endsWhen: period.endsWhen,
        arcType: period.arcType,
        dominantTheme: period.dominantTheme,
        protagonistId: period.protagonistId,
        antagonistId: period.antagonistId,
        summary: period.summary,
        detailedNotes: period.detailedNotes,
        showOnTimeline: period.showOnTimeline,
        timelineColor: period.timelineColor,
        timelineIcon: period.timelineIcon,
    });

    return cozoToPeriod(result);
}

export async function getPeriod(id: string): Promise<Period | null> {
    if (!isTauri()) return null;
    const result = await contentAPI.getPeriod(id);
    return result ? cozoToPeriod(result) : null;
}

export async function updatePeriod(id: string, _updates: Partial<Period>): Promise<Period> {
    if (!isTauri()) throw new Error('Tauri not available');
    // Note: CozoDB doesn't have an update period command yet
    const existing = await contentAPI.getPeriod(id);
    if (!existing) throw new Error(`Period not found: ${id}`);
    return cozoToPeriod(existing);
}

export async function deletePeriod(id: string): Promise<void> {
    if (!isTauri()) throw new Error('Tauri not available');
    await contentAPI.deletePeriod(id);
}

export async function listPeriods(_calendarId = DEFAULT_CALENDAR): Promise<Period[]> {
    if (!isTauri()) return [];
    const results = await contentAPI.listPeriods();
    return results.map(cozoToPeriod);
}

// ============================================================================
// COMBINED LOAD (for initial hydration)
// ============================================================================

export async function loadCalendarData(calendarId = DEFAULT_CALENDAR): Promise<{
    events: CalendarEvent[];
    periods: Period[];
}> {
    if (!isTauri()) {
        console.warn('[CalendarAPI] Tauri not available, returning empty data');
        return { events: [], periods: [] };
    }

    try {
        const { events, periods } = await contentAPI.loadCalendarContent(calendarId);
        return {
            events: events.map(cozoToEvent),
            periods: periods.map(cozoToPeriod),
        };
    } catch (error) {
        console.error('[CalendarAPI] Failed to load:', error);
        return { events: [], periods: [] };
    }
}

// ============================================================================
// NAMESPACE EXPORT
// ============================================================================

export const calendarAPI = {
    createEvent,
    getEvent,
    updateEvent,
    deleteEvent,
    listEvents,
    getEventsByMonth,
    createPeriod,
    getPeriod,
    updatePeriod,
    deletePeriod,
    listPeriods,
    loadCalendarData,
};
