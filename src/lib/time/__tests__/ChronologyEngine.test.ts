/**
 * ChronologyEngine Tests
 * 
 * Tests for the Polymorphic Chronology Engine that registers time units in CozoDB.
 * These tests use mocked CozoDB to avoid database dependencies.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { CalendarConfig } from '@/contexts/CalendarContext';
import type { MonthDefinition } from '@/lib/fantasy-calendar/types';

// Mock CozoDB before importing
vi.mock('@/lib/cozo-stubs/db', () => ({
    cozoDb: {
        isReady: vi.fn(() => true),
        runQuery: vi.fn(() => ({ ok: true, rows: [] }))
    }
}));

// Import after mocking
import { executeGenesis, clearCalendarTimeUnits, getCalendarTimeUnits } from '../ChronologyEngine';
import { cozoDb } from '@/lib/cozo-stubs/db';

describe('ChronologyEngine', () => {
    const mockRunQuery = cozoDb.runQuery as ReturnType<typeof vi.fn>;
    const mockIsReady = cozoDb.isReady as ReturnType<typeof vi.fn>;

    beforeEach(() => {
        vi.clearAllMocks();
        mockIsReady.mockReturnValue(true);
        mockRunQuery.mockReturnValue({ ok: true, rows: [] });
    });

    afterEach(() => {
        vi.clearAllMocks();
    });

    describe('executeGenesis', () => {
        const testConfig: CalendarConfig = {
            name: 'Test World Calendar',
            startingYear: 1,
            eraName: 'Common Era',
            eraAbbreviation: 'CE',
            monthNames: ['Flob', 'Bork', 'Zam'],
            weekdayNames: ['Sunfall', 'Moonrise', 'Stardust'],
            hasYearZero: false
        };

        const testMonths: MonthDefinition[] = [
            { id: 'mo1', index: 0, name: 'Flob', shortName: 'Flo', days: 30 },
            { id: 'mo2', index: 1, name: 'Bork', shortName: 'Bor', days: 28 },
            { id: 'mo3', index: 2, name: 'Zam', shortName: 'Zam', days: 31 }
        ];

        it('should register all months with correct metadata', async () => {
            const result = await executeGenesis(testConfig, 'cal_test_123', testMonths);

            expect(result.monthsRegistered).toBe(3);
            expect(result.calendarId).toBe('cal_test_123');

            // Verify query was called for each month
            const calls = mockRunQuery.mock.calls;
            const monthUpserts = calls.filter(call =>
                typeof call[0] === 'string' &&
                call[0].includes(':put time_unit') &&
                call[1]?.unit_type === 'MONTH'
            );
            expect(monthUpserts.length).toBe(3);
        });

        it('should register weekdays with correct indices', async () => {
            const result = await executeGenesis(testConfig, 'cal_test_123', testMonths);

            expect(result.weekdaysRegistered).toBe(3);

            const calls = mockRunQuery.mock.calls;
            const weekdayUpserts = calls.filter(call =>
                typeof call[0] === 'string' &&
                call[0].includes(':put time_unit') &&
                call[1]?.unit_type === 'WEEKDAY'
            );
            expect(weekdayUpserts.length).toBe(3);
        });

        it('should register eras with direction metadata', async () => {
            const result = await executeGenesis(testConfig, 'cal_test_123', testMonths);

            expect(result.erasRegistered).toBe(1); // Default era

            const calls = mockRunQuery.mock.calls;
            const eraUpserts = calls.filter(call =>
                typeof call[0] === 'string' &&
                call[0].includes(':put time_unit') &&
                call[1]?.unit_type === 'ERA'
            );
            expect(eraUpserts.length).toBe(1);
        });

        it('should create circular sequence edges for months', async () => {
            const result = await executeGenesis(testConfig, 'cal_test_123', testMonths);

            // 3 months = 3 edges (0->1, 1->2, 2->0)
            expect(result.sequenceEdges).toBeGreaterThanOrEqual(3);

            const calls = mockRunQuery.mock.calls;
            const linkCalls = calls.filter(call =>
                typeof call[0] === 'string' &&
                call[0].includes(':put time_unit_sequence')
            );
            expect(linkCalls.length).toBeGreaterThanOrEqual(3);
        });

        it('should skip genesis when CozoDB is not ready', async () => {
            mockIsReady.mockReturnValue(false);

            const result = await executeGenesis(testConfig, 'cal_test_123', testMonths);

            expect(result.monthsRegistered).toBe(0);
            expect(result.weekdaysRegistered).toBe(0);
            expect(result.erasRegistered).toBe(0);
        });

        it('should handle custom eras with multiple entries', async () => {
            const configWithEras: CalendarConfig = {
                ...testConfig,
                eras: [
                    { id: 'era1', name: 'First Age', abbreviation: 'FA', startYear: 1, direction: 'ascending' },
                    { id: 'era2', name: 'Second Age', abbreviation: 'SA', startYear: 1000, direction: 'ascending' }
                ]
            };

            const result = await executeGenesis(configWithEras, 'cal_test_123', testMonths);

            expect(result.erasRegistered).toBe(2);
        });

        it('should preserve days_in_unit metadata for validation', async () => {
            await executeGenesis(testConfig, 'cal_test_123', testMonths);

            const calls = mockRunQuery.mock.calls;
            const flobUpsert = calls.find(call =>
                call[1]?.name === 'Flob' && call[1]?.unit_type === 'MONTH'
            );

            expect(flobUpsert).toBeDefined();
            expect(flobUpsert![1].days_in_unit).toBe(30);
        });
    });

    describe('clearCalendarTimeUnits', () => {
        it('should delete sequences before time units', async () => {
            await clearCalendarTimeUnits('cal_test_123');

            const calls = mockRunQuery.mock.calls;

            // Should delete sequences first
            const sequenceDelete = calls.findIndex(call =>
                typeof call[0] === 'string' && call[0].includes(':rm time_unit_sequence')
            );
            const unitDelete = calls.findIndex(call =>
                typeof call[0] === 'string' && call[0].includes(':rm time_unit')
            );

            expect(sequenceDelete).toBeLessThan(unitDelete);
        });

        it('should skip when CozoDB is not ready', async () => {
            mockIsReady.mockReturnValue(false);

            await clearCalendarTimeUnits('cal_test_123');

            expect(mockRunQuery).not.toHaveBeenCalled();
        });
    });

    describe('getCalendarTimeUnits', () => {
        it('should return empty array when CozoDB is not ready', () => {
            mockIsReady.mockReturnValue(false);

            const result = getCalendarTimeUnits('cal_test_123');

            expect(result).toEqual([]);
        });

        it('should parse rows into TimeUnitRow objects', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [
                    ['id1', 'MONTH', 'Flob', 'flob', 'Flo', 0, 30, null, null, null]
                ]
            });

            const result = getCalendarTimeUnits('cal_test_123');

            expect(result.length).toBe(1);
            expect(result[0].name).toBe('Flob');
            expect(result[0].unitType).toBe('MONTH');
            expect(result[0].index).toBe(0);
        });
    });
});
