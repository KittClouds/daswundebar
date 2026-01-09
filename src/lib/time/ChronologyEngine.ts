/**
 * ChronologyEngine - The Polymorphic Chronology Machine
 * 
 * Transforms calendar configuration into:
 * 1. CozoDB entities (TIME_UNIT kind)
 * 2. Causal sequence edges (MONTH_1 -> NEXT -> MONTH_2)
 * 3. Physics metadata (days_in_unit for validation)
 * 
 * @module time
 */

import { v4 as uuidv4 } from 'uuid';
import { cozoDb } from '@/lib/cozo-stubs/db';
import { TIME_UNIT_QUERIES } from '@/lib/cozo-stubs/schema';
import type { TimeUnitType } from '@/lib/cozo-stubs/schema';

// TimeUnitRow stub type for ChronologyEngine
interface TimeUnitRow {
    id: string;
    calendarId: string;
    unitType: string;
    name: string;
    normalizedName: string;
    shortName?: string;
    index: number;
    daysInUnit?: number;
    direction?: string;
    startYear?: number;
    endYear?: number;
    createdAt: number;
}
import type { CalendarConfig } from '@/contexts/CalendarContext';
import type { MonthDefinition, EraDefinition } from '@/lib/fantasy-calendar/types';

export interface GenesisResult {
    calendarId: string;
    monthsRegistered: number;
    weekdaysRegistered: number;
    erasRegistered: number;
    sequenceEdges: number;
}

/**
 * Execute world genesis — register all time units from calendar config
 */
export async function executeGenesis(
    config: CalendarConfig,
    calendarId: string,
    months: MonthDefinition[]
): Promise<GenesisResult> {
    console.log('[ChronologyEngine] Executing world genesis for calendar:', calendarId);

    const result: GenesisResult = {
        calendarId,
        monthsRegistered: 0,
        weekdaysRegistered: 0,
        erasRegistered: 0,
        sequenceEdges: 0
    };

    if (!cozoDb.isReady()) {
        console.warn('[ChronologyEngine] CozoDB not ready, skipping genesis');
        return result;
    }

    const now = Date.now();
    const monthIds: string[] = [];
    const weekdayIds: string[] = [];
    const eraIds: string[] = [];

    // === REGISTER MONTHS ===
    for (let i = 0; i < months.length; i++) {
        const month = months[i];
        const id = uuidv4();
        monthIds.push(id);

        await registerTimeUnit({
            id,
            calendarId,
            unitType: 'MONTH',
            name: month.name,
            normalizedName: month.name.toLowerCase().trim(),
            shortName: month.shortName,
            index: i,
            daysInUnit: month.days, // Critical for validation!
            createdAt: now
        });
        result.monthsRegistered++;
    }

    // === REGISTER WEEKDAYS ===
    for (let i = 0; i < config.weekdayNames.length; i++) {
        const name = config.weekdayNames[i];
        if (!name || name.trim() === '') continue;

        const id = uuidv4();
        weekdayIds.push(id);

        await registerTimeUnit({
            id,
            calendarId,
            unitType: 'WEEKDAY',
            name,
            normalizedName: name.toLowerCase().trim(),
            shortName: name.substring(0, 3),
            index: i,
            createdAt: now
        });
        result.weekdaysRegistered++;
    }

    // === REGISTER ERAS ===
    const eras = config.eras && config.eras.length > 0 ? config.eras : [{
        id: 'default_era',
        name: config.eraName || 'Common Era',
        abbreviation: config.eraAbbreviation || 'CE',
        startYear: 1,
        direction: 'ascending' as const
    }];

    for (let i = 0; i < eras.length; i++) {
        const era = eras[i];
        const id = uuidv4();
        eraIds.push(id);

        await registerTimeUnit({
            id,
            calendarId,
            unitType: 'ERA',
            name: era.name,
            normalizedName: era.name.toLowerCase().trim(),
            shortName: era.abbreviation,
            index: i,
            direction: era.direction,
            startYear: era.startYear,
            endYear: era.endYear,
            createdAt: now
        });
        result.erasRegistered++;
    }

    // === CREATE SEQUENCE EDGES ===
    // Months: 0 -> 1 -> 2 -> ... -> N -> 0 (circular)
    for (let i = 0; i < monthIds.length; i++) {
        const nextIndex = (i + 1) % monthIds.length;
        await linkSequence(monthIds[i], monthIds[nextIndex], calendarId);
        result.sequenceEdges++;
    }

    // Weekdays: 0 -> 1 -> 2 -> ... -> N -> 0 (circular)
    for (let i = 0; i < weekdayIds.length; i++) {
        const nextIndex = (i + 1) % weekdayIds.length;
        await linkSequence(weekdayIds[i], weekdayIds[nextIndex], calendarId);
        result.sequenceEdges++;
    }

    console.log('[ChronologyEngine] Genesis complete:', result);
    return result;
}

/**
 * Register a single time unit in CozoDB
 */
async function registerTimeUnit(unit: TimeUnitRow): Promise<void> {
    try {
        cozoDb.runQuery(TIME_UNIT_QUERIES.upsert, {
            id: unit.id,
            calendar_id: unit.calendarId,
            unit_type: unit.unitType,
            name: unit.name,
            normalized_name: unit.normalizedName,
            short_name: unit.shortName ?? null,
            idx: unit.index,
            days_in_unit: unit.daysInUnit ?? null,
            direction: unit.direction ?? null,
            start_year: unit.startYear ?? null,
            end_year: unit.endYear ?? null,
            created_at: unit.createdAt
        });
    } catch (err) {
        console.error('[ChronologyEngine] Failed to register time unit:', unit.name, err);
        throw err;
    }
}

/**
 * Create a NEXT edge between two time units
 */
async function linkSequence(fromId: string, toId: string, calendarId: string): Promise<void> {
    try {
        cozoDb.runQuery(TIME_UNIT_QUERIES.linkSequence, {
            from_id: fromId,
            to_id: toId,
            calendar_id: calendarId
        });
    } catch (err) {
        console.error('[ChronologyEngine] Failed to link sequence:', fromId, '->', toId, err);
    }
}

/**
 * Clear all time units for a calendar (for re-genesis)
 */
export async function clearCalendarTimeUnits(calendarId: string): Promise<void> {
    if (!cozoDb.isReady()) {
        console.warn('[ChronologyEngine] CozoDB not ready, skipping clear');
        return;
    }

    try {
        // First delete sequence edges
        cozoDb.runQuery(TIME_UNIT_QUERIES.deleteSequencesByCalendar, { calendar_id: calendarId });
        // Then delete time units
        cozoDb.runQuery(TIME_UNIT_QUERIES.deleteByCalendar, { calendar_id: calendarId });
        console.log('[ChronologyEngine] Cleared time units for calendar:', calendarId);
    } catch (err) {
        console.error('[ChronologyEngine] Failed to clear calendar:', err);
    }
}

/**
 * Get all time units for a calendar
 */
export function getCalendarTimeUnits(calendarId: string): TimeUnitRow[] {
    if (!cozoDb.isReady()) return [];

    try {
        const result = cozoDb.runQuery(TIME_UNIT_QUERIES.getByCalendar, {
            calendar_id: calendarId
        });

        if (!result.ok || !result.rows) return [];

        return result.rows.map((row: any[]) => ({
            id: row[0],
            calendarId,
            unitType: row[1] as TimeUnitType,
            name: row[2],
            normalizedName: row[3],
            shortName: row[4] ?? undefined,
            index: row[5],
            daysInUnit: row[6] ?? undefined,
            direction: row[7] ?? undefined,
            startYear: row[8] ?? undefined,
            endYear: row[9] ?? undefined,
            createdAt: Date.now()
        }));
    } catch (err) {
        console.error('[ChronologyEngine] getCalendarTimeUnits failed:', err);
        return [];
    }
}
