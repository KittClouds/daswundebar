/**
 * TimeRegistry - Read API for Time Units
 * 
 * Provides:
 * - getMonths(calendarId) -> ordered list
 * - getWeekdays(calendarId) -> ordered list
 * - validateDate(calendarId, monthName, day) -> { valid, maxDays }
 * - getCalendarDictionary(calendarId) -> for scanner hydration
 */

import { cozoDb } from '@/lib/cozo-stubs/db';
import { TIME_UNIT_QUERIES } from '@/lib/cozo-stubs/schema';
import type { TimeUnitType } from '@/lib/cozo-stubs/schema';

export interface MonthInfo {
    id: string;
    name: string;
    normalizedName: string;
    shortName?: string;
    index: number;
    daysInUnit: number;
}

export interface WeekdayInfo {
    id: string;
    name: string;
    normalizedName: string;
    shortName?: string;
    index: number;
}

export interface EraInfo {
    id: string;
    name: string;
    normalizedName: string;
    abbreviation?: string;
    index: number;
    direction: 'ascending' | 'descending';
    startYear?: number;
    endYear?: number;
}

export interface DateValidation {
    valid: boolean;
    maxDays: number;
    monthName: string;
}

export interface CalendarDictionary {
    calendarId: string;
    months: string[];
    weekdays: string[];
    eras: string[];
    monthIndex: Record<string, number>;
    weekdayIndex: Record<string, number>;
    monthDays: Record<string, number>;
}

// ============================================
// ENTITY TIMELINE TYPES
// ============================================

/**
 * An event or note associated with an entity, with fantasy date context
 */
export interface EntityTimelineEntry {
    id: string;
    type: 'EVENT' | 'NOTE' | 'MENTION';
    title: string;
    description?: string;
    fantasyDate: {
        year: number;
        monthIndex: number;
        dayIndex: number;
        eraId?: string;
    };
    sourceNoteId?: string;
    role?: 'participant' | 'location' | 'artifact' | 'owner';
    metadata?: Record<string, unknown>;
}

/**
 * Query for entity timeline
 */
export interface EntityTimelineQuery {
    entityId: string;
    calendarId?: string;
    startYear?: number;
    endYear?: number;
    includeEvents?: boolean;
    includeNotes?: boolean;
    includeMentions?: boolean;
}

/**
 * Result of entity relationship query
 */
export interface EntityRelationship {
    targetEntityId: string;
    targetLabel: string;
    relationshipType: string;
    sharedEvents: number;
    sharedNotes: number;
    confidence: number;
}


class TimeRegistryImpl {
    /**
     * Get all months for a calendar, ordered by index
     */
    getMonths(calendarId: string): MonthInfo[] {
        if (!cozoDb.isReady()) return [];

        try {
            const result = cozoDb.runQuery(TIME_UNIT_QUERIES.getMonthsByCalendar, {
                calendar_id: calendarId
            });

            if (!result.ok || !result.rows) return [];

            return result.rows.map((row: any[]) => ({
                id: row[0],
                name: row[1],
                normalizedName: row[2],
                shortName: row[3] ?? undefined,
                index: row[4],
                daysInUnit: row[5] ?? 30
            }));
        } catch (err) {
            console.error('[TimeRegistry] getMonths failed:', err);
            return [];
        }
    }

    /**
     * Get all weekdays for a calendar, ordered by index
     */
    getWeekdays(calendarId: string): WeekdayInfo[] {
        if (!cozoDb.isReady()) return [];

        try {
            const result = cozoDb.runQuery(TIME_UNIT_QUERIES.getWeekdaysByCalendar, {
                calendar_id: calendarId
            });

            if (!result.ok || !result.rows) return [];

            return result.rows.map((row: any[]) => ({
                id: row[0],
                name: row[1],
                normalizedName: row[2],
                shortName: row[3] ?? undefined,
                index: row[4]
            }));
        } catch (err) {
            console.error('[TimeRegistry] getWeekdays failed:', err);
            return [];
        }
    }

    /**
     * Get all eras for a calendar
     */
    getEras(calendarId: string): EraInfo[] {
        if (!cozoDb.isReady()) return [];

        try {
            const result = cozoDb.runQuery(TIME_UNIT_QUERIES.getErasByCalendar, {
                calendar_id: calendarId
            });

            if (!result.ok || !result.rows) return [];

            return result.rows.map((row: any[]) => ({
                id: row[0],
                name: row[1],
                normalizedName: row[2],
                abbreviation: row[3] ?? undefined,
                index: row[4],
                direction: (row[5] as 'ascending' | 'descending') ?? 'ascending',
                startYear: row[6] ?? undefined,
                endYear: row[7] ?? undefined
            }));
        } catch (err) {
            console.error('[TimeRegistry] getEras failed:', err);
            return [];
        }
    }

    /**
     * Validate a day number against a month's physics
     */
    validateDate(calendarId: string, monthName: string, day: number): DateValidation {
        const normalized = monthName.toLowerCase().trim();

        if (!cozoDb.isReady()) {
            return { valid: true, maxDays: 31, monthName: monthName };
        }

        try {
            const result = cozoDb.runQuery(TIME_UNIT_QUERIES.findByName, {
                normalized_name: normalized,
                calendar_id: calendarId
            });

            if (!result.ok || !result.rows || result.rows.length === 0) {
                return { valid: true, maxDays: 31, monthName: monthName }; // Unknown month, assume valid
            }

            const row = result.rows[0];
            const daysInUnit = row[5] ?? 30;

            return {
                valid: day >= 1 && day <= daysInUnit,
                maxDays: daysInUnit,
                monthName: row[3] // The original name
            };
        } catch (err) {
            console.error('[TimeRegistry] validateDate failed:', err);
            return { valid: true, maxDays: 31, monthName: monthName };
        }
    }

    /**
     * Build a dictionary for the temporal scanner
     */
    getCalendarDictionary(calendarId: string): CalendarDictionary {
        const months = this.getMonths(calendarId);
        const weekdays = this.getWeekdays(calendarId);
        const eras = this.getEras(calendarId);

        const monthIndex: Record<string, number> = {};
        const weekdayIndex: Record<string, number> = {};
        const monthDays: Record<string, number> = {};

        const monthTerms: string[] = [];
        const weekdayTerms: string[] = [];
        const eraTerms: string[] = [];

        for (const m of months) {
            // Add normalized name
            monthIndex[m.normalizedName] = m.index;
            monthDays[m.normalizedName] = m.daysInUnit;
            monthTerms.push(m.normalizedName);

            // Add short name variant
            if (m.shortName) {
                const shortNorm = m.shortName.toLowerCase();
                monthIndex[shortNorm] = m.index;
                monthDays[shortNorm] = m.daysInUnit;
                monthTerms.push(shortNorm);
            }
        }

        for (const w of weekdays) {
            weekdayIndex[w.normalizedName] = w.index;
            weekdayTerms.push(w.normalizedName);

            if (w.shortName) {
                const shortNorm = w.shortName.toLowerCase();
                weekdayIndex[shortNorm] = w.index;
                weekdayTerms.push(shortNorm);
            }
        }

        for (const e of eras) {
            eraTerms.push(e.normalizedName);
            if (e.abbreviation) {
                eraTerms.push(e.abbreviation.toLowerCase());
            }
        }

        return {
            calendarId,
            months: monthTerms,
            weekdays: weekdayTerms,
            eras: eraTerms,
            monthIndex,
            weekdayIndex,
            monthDays
        };
    }

    /**
     * Check if a calendar has any registered time units
     */
    hasTimeUnits(calendarId: string): boolean {
        const months = this.getMonths(calendarId);
        return months.length > 0;
    }

    // ============================================
    // ENTITY TIMELINE METHODS
    // ============================================

    /**
     * Get timeline entries for a specific entity
     * Aggregates events, notes, and mentions where the entity is involved
     */
    getEntityTimeline(query: EntityTimelineQuery): EntityTimelineEntry[] {
        const entries: EntityTimelineEntry[] = [];

        // This would integrate with dbClient and CozoDB
        // For now, we provide a structure that can be populated by consumers

        if (!cozoDb.isReady()) {
            console.warn('[TimeRegistry] getEntityTimeline: CozoDB not ready');
            return entries;
        }

        try {
            // Query events where entity is a participant
            const eventQuery = `
                ?[event_id, title, year, month_index, day_index, era_id, role] :=
                    *calendar_event_participants{event_id, entity_id, role},
                    entity_id == $entity_id,
                    *calendar_events{id: event_id, title, year, month_index, day_index, era_id}
                :order year, month_index, day_index
            `;

            const result = cozoDb.runQuery(eventQuery, { entity_id: query.entityId });

            if (result.ok && result.rows) {
                for (const row of result.rows as any[]) {
                    const [eventId, title, year, monthIndex, dayIndex, eraId, role] = row;

                    // Apply date filters if specified
                    if (query.startYear !== undefined && year < query.startYear) continue;
                    if (query.endYear !== undefined && year > query.endYear) continue;

                    entries.push({
                        id: eventId,
                        type: 'EVENT',
                        title: title || 'Untitled Event',
                        fantasyDate: {
                            year: year || 1,
                            monthIndex: monthIndex || 0,
                            dayIndex: dayIndex || 0,
                            eraId: eraId,
                        },
                        role: role || 'participant',
                    });
                }
            }
        } catch (err) {
            console.warn('[TimeRegistry] getEntityTimeline failed:', err);
        }

        // Sort by fantasy date
        entries.sort((a, b) => {
            if (a.fantasyDate.year !== b.fantasyDate.year) {
                return a.fantasyDate.year - b.fantasyDate.year;
            }
            if (a.fantasyDate.monthIndex !== b.fantasyDate.monthIndex) {
                return a.fantasyDate.monthIndex - b.fantasyDate.monthIndex;
            }
            return a.fantasyDate.dayIndex - b.fantasyDate.dayIndex;
        });

        return entries;
    }

    /**
     * Get entities that share timeline presence with the given entity
     */
    getEntityRelationships(entityId: string): EntityRelationship[] {
        const relationships: EntityRelationship[] = [];

        if (!cozoDb.isReady()) {
            return relationships;
        }

        try {
            // Find entities that appear in the same events
            const coOccurrenceQuery = `
                ?[other_id, other_label, rel_type, shared_events, confidence] :=
                    *relationships{source_id, target_id, type, confidence},
                    (source_id == $entity_id, other_id = target_id) or 
                    (target_id == $entity_id, other_id = source_id),
                    rel_type = type,
                    *entity{id: other_id, label: other_label},
                    shared_events = 1
                :limit 50
            `;

            const result = cozoDb.runQuery(coOccurrenceQuery, { entity_id: entityId });

            if (result.ok && result.rows) {
                for (const row of result.rows as any[]) {
                    const [otherId, otherLabel, relType, sharedEvents, confidence] = row;

                    relationships.push({
                        targetEntityId: otherId,
                        targetLabel: otherLabel || 'Unknown',
                        relationshipType: relType || 'related_to',
                        sharedEvents: sharedEvents || 0,
                        sharedNotes: 0,
                        confidence: confidence || 0.5,
                    });
                }
            }
        } catch (err) {
            console.warn('[TimeRegistry] getEntityRelationships failed:', err);
        }

        return relationships;
    }

    /**
     * Get all entities that have timeline entries within a date range
     */
    getEntitiesInDateRange(
        calendarId: string,
        startYear: number,
        endYear: number
    ): Array<{ entityId: string; label: string; entryCount: number }> {
        const entities: Array<{ entityId: string; label: string; entryCount: number }> = [];

        if (!cozoDb.isReady()) {
            return entities;
        }

        try {
            const rangeQuery = `
                ?[entity_id, entity_label, count(event_id)] :=
                    *calendar_event_participants{event_id, entity_id},
                    *calendar_events{id: event_id, year, calendar_id},
                    calendar_id == $calendar_id,
                    year >= $start_year,
                    year <= $end_year,
                    *entity{id: entity_id, label: entity_label}
                :order -count(event_id)
                :limit 100
            `;

            const result = cozoDb.runQuery(rangeQuery, {
                calendar_id: calendarId,
                start_year: startYear,
                end_year: endYear,
            });

            if (result.ok && result.rows) {
                for (const row of result.rows as any[]) {
                    const [entityId, label, count] = row;
                    entities.push({
                        entityId,
                        label: label || 'Unknown',
                        entryCount: count || 0,
                    });
                }
            }
        } catch (err) {
            console.warn('[TimeRegistry] getEntitiesInDateRange failed:', err);
        }

        return entities;
    }
}

export const TimeRegistry = new TimeRegistryImpl();
