/**
 * Calendar Atoms - SurrealDB-backed state management for Fantasy Calendar
 * Uses lazy loading pattern: hydrates when CalendarContext mounts
 * 
 * MIGRATED: Now uses SurrealDB via calendarAPI (Tauri backend)
 */
import { atom, type WritableAtom } from 'jotai';
import { generateId } from '@/lib/utils/ids';
import { isTauri } from '@/lib/tauri/bridge';
import { calendarAPI } from '@/lib/tauri/calendar-api';
import type {
    CalendarDefinition,
    CalendarEvent,
    Period,
    FantasyDate,
    EditorScope,
} from '@/lib/fantasy-calendar/types';

// ============================================
// TYPES
// ============================================

export interface CalendarViewState {
    viewDate: FantasyDate;
    editorScope: EditorScope;
    highlightedEventId: string | null;
    isSetupMode: boolean;
}

// ============================================
// BASE ATOMS (In-Memory State) - Internal
// ============================================

// Using explicit writable atoms pattern for type safety
const _calendarBaseAtom = atom<CalendarDefinition | null>(null);
const _eventsBaseAtom = atom<CalendarEvent[]>([]);
const _periodsBaseAtom = atom<Period[]>([]);
const _viewStateBaseAtom = atom<CalendarViewState>({
    viewDate: { year: 1, monthIndex: 0, dayIndex: 0 },
    editorScope: 'day',
    highlightedEventId: null,
    isSetupMode: false,
});
const _isHydratedBaseAtom = atom(false);
const _isLoadingBaseAtom = atom(false);

// Writable derived atoms (same pattern as notes.ts)
const _calendarAtom: WritableAtom<CalendarDefinition | null, [CalendarDefinition | null], void> = atom(
    (get) => get(_calendarBaseAtom),
    (_get, set, val) => set(_calendarBaseAtom as any, val)
);

const _eventsAtom: WritableAtom<CalendarEvent[], [CalendarEvent[]], void> = atom(
    (get) => get(_eventsBaseAtom),
    (_get, set, val) => set(_eventsBaseAtom as any, val)
);

const _periodsAtom: WritableAtom<Period[], [Period[]], void> = atom(
    (get) => get(_periodsBaseAtom),
    (_get, set, val) => set(_periodsBaseAtom as any, val)
);

const _viewStateAtom: WritableAtom<CalendarViewState, [CalendarViewState], void> = atom(
    (get) => get(_viewStateBaseAtom),
    (_get, set, val) => set(_viewStateBaseAtom as any, val)
);

const _isHydratedAtom: WritableAtom<boolean, [boolean], void> = atom(
    (get) => get(_isHydratedBaseAtom),
    (_get, set, val) => set(_isHydratedBaseAtom as any, val)
);

const _isLoadingAtom: WritableAtom<boolean, [boolean], void> = atom(
    (get) => get(_isLoadingBaseAtom),
    (_get, set, val) => set(_isLoadingBaseAtom as any, val)
);


// ============================================
// EXPORTED READ ATOMS
// ============================================

export const calendarAtom = atom((get) => get(_calendarAtom));
export const calendarEventsAtom = atom((get) => get(_eventsAtom));
export const calendarPeriodsAtom = atom((get) => get(_periodsAtom));
export const calendarViewStateAtom = atom((get) => get(_viewStateAtom));
export const isCalendarHydratedAtom = atom((get) => get(_isHydratedAtom));
export const isCalendarLoadingAtom = atom((get) => get(_isLoadingAtom));

// ============================================
// ENTITY-SCOPED DERIVED ATOMS
// ============================================

// Import the narrative focus atom (lazy import to avoid circular deps)
import { focusedEntityIdAtom, focusModeAtom } from './narrative-focus';

/**
 * Events filtered by the currently focused entity
 * When focusMode is 'entity' and an entity is focused, only shows events
 * where the focused entity is a participant.
 * When focusMode is 'all', shows all events.
 */
export const eventsForFocusedEntityAtom = atom((get) => {
    const events = get(_eventsAtom);
    const focusMode = get(focusModeAtom);
    const focusedEntityId = get(focusedEntityIdAtom);

    // No focus or 'all' mode - return all events
    if (focusMode === 'all' || !focusedEntityId) {
        return events;
    }

    // Filter to events where focused entity is a participant
    return events.filter(event => {
        // Check if entity is a direct participant
        if (event.participants?.some(p => p.id === focusedEntityId)) {
            return true;
        }
        // Check if entity is in locations
        if (event.locations?.some(l => l.id === focusedEntityId)) {
            return true;
        }
        // Check if entity is in artifacts
        if (event.artifacts?.some(a => a.id === focusedEntityId)) {
            return true;
        }
        // Check legacy entityId field
        if (event.entityId === focusedEntityId) {
            return true;
        }
        return false;
    });
});

/**
 * Periods filtered by the currently focused entity
 */
export const periodsForFocusedEntityAtom = atom((get) => {
    const periods = get(_periodsAtom);
    const focusMode = get(focusModeAtom);
    const focusedEntityId = get(focusedEntityIdAtom);

    // No focus or 'all' mode - return all periods
    if (focusMode === 'all' || !focusedEntityId) {
        return periods;
    }

    // Filter periods linked to this entity
    return periods.filter(period => {
        // Check protagonist
        if (period.protagonist?.id === focusedEntityId) {
            return true;
        }
        // Check antagonist
        if (period.antagonist?.id === focusedEntityId) {
            return true;
        }
        return false;
    });
});


// ============================================
// HYDRATION ATOM (Lazy Load)
// ============================================

/**
 * Hydrate calendar data from SurrealDB
 * Called when CalendarProvider mounts
 */
export const hydrateCalendarAtom = atom(
    null,
    async (get, set) => {
        // Skip if already hydrated or loading
        if (get(_isHydratedAtom) || get(_isLoadingAtom)) {
            return;
        }

        set(_isLoadingAtom, true);

        try {
            if (!isTauri()) {
                console.log('[Calendar] Not in Tauri mode, using empty data');
                set(_isHydratedAtom, true);
                return;
            }

            const { events, periods } = await calendarAPI.loadCalendarData();

            set(_eventsAtom, events);
            set(_periodsAtom, periods);
            set(_isHydratedAtom, true);

            console.log(`[Calendar] Hydrated: ${events.length} events, ${periods.length} periods`);
        } catch (error) {
            console.error('[Calendar] Hydration failed:', error);
            // Don't throw - calendar is optional
            set(_isHydratedAtom, true);
        } finally {
            set(_isLoadingAtom, false);
        }
    }
);

// ============================================
// CALENDAR MUTATION ATOMS
// ============================================

/**
 * Create or update calendar definition
 * Note: Calendar definitions are stored in memory for now
 * TODO: Persist to SurrealDB when needed
 */
export const saveCalendarAtom = atom(
    null,
    async (get, set, calendar: CalendarDefinition) => {
        const previous = get(_calendarAtom);

        // Optimistic update
        set(_calendarAtom, calendar);

        console.log(`[Calendar] Saved calendar: ${calendar.name} (in-memory)`);
        // TODO: Persist to SurrealDB if needed
    }
);

// ============================================
// EVENT MUTATION ATOMS
// ============================================

/**
 * Add a new event
 */
export const createEventAtom = atom(
    null,
    async (get, set, event: Omit<CalendarEvent, 'id'>) => {
        const newEvent: CalendarEvent = {
            ...event,
            id: generateId(),
        };

        const currentEvents = get(_eventsAtom);

        // Optimistic add
        set(_eventsAtom, [...currentEvents, newEvent]);

        try {
            if (isTauri()) {
                const created = await calendarAPI.createEvent(event);
                // Update with server-assigned ID if different
                if (created.id !== newEvent.id) {
                    set(_eventsAtom, get(_eventsAtom).map(e =>
                        e.id === newEvent.id ? { ...e, id: created.id } : e
                    ));
                    console.log(`[Calendar] Created event: ${created.title} (server ID: ${created.id})`);
                    return { ...newEvent, id: created.id };
                }
            }
            console.log(`[Calendar] Created event: ${newEvent.title}`);
            return newEvent;
        } catch (error) {
            // Rollback
            set(_eventsAtom, currentEvents);
            console.error('[Calendar] Failed to create event:', error);
            throw error;
        }
    }
);

/**
 * Update an existing event
 */
export const updateEventAtom = atom(
    null,
    async (get, set, params: { id: string; updates: Partial<CalendarEvent> }) => {
        const { id, updates } = params;
        const currentEvents = get(_eventsAtom);
        const originalEvent = currentEvents.find(e => e.id === id);

        if (!originalEvent) {
            console.error(`[Calendar] Event ${id} not found`);
            return;
        }

        const updatedEvent = { ...originalEvent, ...updates };

        // Optimistic update
        set(_eventsAtom, currentEvents.map(e => e.id === id ? updatedEvent : e));

        try {
            if (isTauri()) {
                await calendarAPI.updateEvent(id, updates);
            }
            console.log(`[Calendar] Updated event: ${updatedEvent.title}`);
        } catch (error) {
            // Rollback
            set(_eventsAtom, currentEvents);
            console.error('[Calendar] Failed to update event:', error);
            throw error;
        }
    }
);

/**
 * Delete an event
 */
export const deleteEventAtom = atom(
    null,
    async (get, set, eventId: string) => {
        const currentEvents = get(_eventsAtom);

        // Optimistic delete
        set(_eventsAtom, currentEvents.filter(e => e.id !== eventId));

        try {
            if (isTauri()) {
                await calendarAPI.deleteEvent(eventId);
            }
            console.log(`[Calendar] Deleted event: ${eventId}`);
        } catch (error) {
            // Rollback
            set(_eventsAtom, currentEvents);
            console.error('[Calendar] Failed to delete event:', error);
            throw error;
        }
    }
);

// ============================================
// PERIOD MUTATION ATOMS
// ============================================

/**
 * Add a new period
 */
export const createPeriodAtom = atom(
    null,
    async (get, set, period: Omit<Period, 'id'>) => {
        const newPeriod: Period = {
            ...period,
            id: generateId(),
        };

        const currentPeriods = get(_periodsAtom);

        // Optimistic add
        set(_periodsAtom, [...currentPeriods, newPeriod]);

        try {
            if (isTauri()) {
                const created = await calendarAPI.createPeriod(period);
                // Update with server-assigned ID if different
                if (created.id !== newPeriod.id) {
                    set(_periodsAtom, get(_periodsAtom).map(p =>
                        p.id === newPeriod.id ? { ...p, id: created.id } : p
                    ));
                    console.log(`[Calendar] Created period: ${created.name} (server ID: ${created.id})`);
                    return { ...newPeriod, id: created.id };
                }
            }
            console.log(`[Calendar] Created period: ${newPeriod.name}`);
            return newPeriod;
        } catch (error) {
            // Rollback
            set(_periodsAtom, currentPeriods);
            console.error('[Calendar] Failed to create period:', error);
            throw error;
        }
    }
);

/**
 * Update an existing period
 */
export const updatePeriodAtom = atom(
    null,
    async (get, set, params: { id: string; updates: Partial<Period> }) => {
        const { id, updates } = params;
        const currentPeriods = get(_periodsAtom);
        const originalPeriod = currentPeriods.find(p => p.id === id);

        if (!originalPeriod) {
            console.error(`[Calendar] Period ${id} not found`);
            return;
        }

        const updatedPeriod = { ...originalPeriod, ...updates };

        // Optimistic update
        set(_periodsAtom, currentPeriods.map(p => p.id === id ? updatedPeriod : p));

        try {
            if (isTauri()) {
                await calendarAPI.updatePeriod(id, updates);
            }
            console.log(`[Calendar] Updated period: ${updatedPeriod.name}`);
        } catch (error) {
            // Rollback
            set(_periodsAtom, currentPeriods);
            console.error('[Calendar] Failed to update period:', error);
            throw error;
        }
    }
);

/**
 * Delete a period
 */
export const deletePeriodAtom = atom(
    null,
    async (get, set, periodId: string) => {
        const currentPeriods = get(_periodsAtom);
        const currentEvents = get(_eventsAtom);

        // Optimistic delete
        set(_periodsAtom, currentPeriods.filter(p => p.id !== periodId));
        // Also clear periodId from events referencing this period
        set(_eventsAtom, currentEvents.map(e =>
            e.periodId === periodId ? { ...e, periodId: undefined } : e
        ));

        try {
            if (isTauri()) {
                await calendarAPI.deletePeriod(periodId);
            }
            console.log(`[Calendar] Deleted period: ${periodId}`);
        } catch (error) {
            // Rollback
            set(_periodsAtom, currentPeriods);
            set(_eventsAtom, currentEvents);
            console.error('[Calendar] Failed to delete period:', error);
            throw error;
        }
    }
);

// ============================================
// VIEW STATE ATOMS (Memory-only, no persistence)
// ============================================

/**
 * Update view state (viewDate, editorScope, etc.)
 */
export const updateViewStateAtom = atom(
    null,
    (get, set, updates: Partial<CalendarViewState>) => {
        const current = get(_viewStateAtom);
        set(_viewStateAtom, { ...current, ...updates });
    }
);

/**
 * Set setup mode
 */
export const setSetupModeAtom = atom(
    null,
    (get, set, isSetupMode: boolean) => {
        const current = get(_viewStateAtom);
        set(_viewStateAtom, { ...current, isSetupMode });
    }
);
