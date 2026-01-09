/**
 * TimeRegistry Tests
 * 
 * Tests for the TimeRegistry read API that provides access to calendar time units.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock CozoDB before importing TimeRegistry
vi.mock('@/lib/cozo-stubs/db', () => ({
    cozoDb: {
        isReady: vi.fn(() => true),
        runQuery: vi.fn(() => ({ ok: true, rows: [] }))
    }
}));

import { TimeRegistry } from '../TimeRegistry';
import { cozoDb } from '@/lib/cozo-stubs/db';

describe('TimeRegistry', () => {
    const mockRunQuery = cozoDb.runQuery as ReturnType<typeof vi.fn>;
    const mockIsReady = cozoDb.isReady as ReturnType<typeof vi.fn>;

    beforeEach(() => {
        vi.clearAllMocks();
        mockIsReady.mockReturnValue(true);
    });

    afterEach(() => {
        vi.clearAllMocks();
    });

    describe('getMonths', () => {
        it('should return empty array when CozoDB is not ready', () => {
            mockIsReady.mockReturnValue(false);

            const result = TimeRegistry.getMonths('cal_test');

            expect(result).toEqual([]);
        });

        it('should parse month rows correctly', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [
                    ['id1', 'Flob', 'flob', 'Flo', 0, 30],
                    ['id2', 'Bork', 'bork', 'Bor', 1, 28],
                    ['id3', 'Zam', 'zam', 'Zam', 2, 31]
                ]
            });

            const result = TimeRegistry.getMonths('cal_test');

            expect(result.length).toBe(3);
            expect(result[0]).toEqual({
                id: 'id1',
                name: 'Flob',
                normalizedName: 'flob',
                shortName: 'Flo',
                index: 0,
                daysInUnit: 30
            });
            expect(result[2].daysInUnit).toBe(31);
        });

        it('should handle null shortName', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [
                    ['id1', 'Flob', 'flob', null, 0, 30]
                ]
            });

            const result = TimeRegistry.getMonths('cal_test');

            expect(result[0].shortName).toBeUndefined();
        });
    });

    describe('getWeekdays', () => {
        it('should parse weekday rows correctly', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [
                    ['id1', 'Sunfall', 'sunfall', 'Sun', 0],
                    ['id2', 'Moonrise', 'moonrise', 'Moo', 1]
                ]
            });

            const result = TimeRegistry.getWeekdays('cal_test');

            expect(result.length).toBe(2);
            expect(result[0].name).toBe('Sunfall');
            expect(result[1].index).toBe(1);
        });
    });

    describe('getEras', () => {
        it('should parse era rows with direction', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [
                    ['id1', 'First Age', 'first age', 'FA', 0, 'ascending', 1, null]
                ]
            });

            const result = TimeRegistry.getEras('cal_test');

            expect(result.length).toBe(1);
            expect(result[0].direction).toBe('ascending');
            expect(result[0].startYear).toBe(1);
            expect(result[0].endYear).toBeUndefined();
        });
    });

    describe('validateDate', () => {
        it('should return valid for known month with valid day', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [
                    ['id1', 'cal_test', 'MONTH', 'Flob', 0, 30]
                ]
            });

            const result = TimeRegistry.validateDate('cal_test', 'Flob', 15);

            expect(result.valid).toBe(true);
            expect(result.maxDays).toBe(30);
        });

        it('should return invalid for day exceeding month limit', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [
                    ['id1', 'cal_test', 'MONTH', 'Flob', 0, 30]
                ]
            });

            const result = TimeRegistry.validateDate('cal_test', 'Flob', 35);

            expect(result.valid).toBe(false);
            expect(result.maxDays).toBe(30);
        });

        it('should return valid=true for unknown month (graceful fallback)', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: []
            });

            const result = TimeRegistry.validateDate('cal_test', 'UnknownMonth', 15);

            expect(result.valid).toBe(true);
            expect(result.maxDays).toBe(31);
        });

        it('should normalize month name case', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [
                    ['id1', 'cal_test', 'MONTH', 'Flob', 0, 30]
                ]
            });

            // Should work regardless of case
            const result = TimeRegistry.validateDate('cal_test', 'FLOB', 15);

            expect(result.valid).toBe(true);
        });

        it('should return invalid for day less than 1', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [
                    ['id1', 'cal_test', 'MONTH', 'Flob', 0, 30]
                ]
            });

            const result = TimeRegistry.validateDate('cal_test', 'Flob', 0);

            expect(result.valid).toBe(false);
        });
    });

    describe('getCalendarDictionary', () => {
        it('should build complete dictionary from all time units', () => {
            // Mock getMonths
            mockRunQuery.mockReturnValueOnce({
                ok: true,
                rows: [
                    ['id1', 'Flob', 'flob', 'Flo', 0, 30],
                    ['id2', 'Bork', 'bork', 'Bor', 1, 28]
                ]
            });
            // Mock getWeekdays
            mockRunQuery.mockReturnValueOnce({
                ok: true,
                rows: [
                    ['id3', 'Sunfall', 'sunfall', 'Sun', 0]
                ]
            });
            // Mock getEras
            mockRunQuery.mockReturnValueOnce({
                ok: true,
                rows: [
                    ['id4', 'First Age', 'first age', 'FA', 0, 'ascending', 1, null]
                ]
            });

            const result = TimeRegistry.getCalendarDictionary('cal_test');

            expect(result.calendarId).toBe('cal_test');
            expect(result.months).toContain('flob');
            expect(result.months).toContain('flo'); // Short name
            expect(result.weekdays).toContain('sunfall');
            expect(result.eras).toContain('first age');
            expect(result.eras).toContain('fa');
            expect(result.monthIndex['flob']).toBe(0);
            expect(result.monthDays['flob']).toBe(30);
        });

        it('should handle short names in index mapping', () => {
            mockRunQuery.mockReturnValueOnce({
                ok: true,
                rows: [
                    ['id1', 'Flob', 'flob', 'Flo', 0, 30]
                ]
            });
            mockRunQuery.mockReturnValueOnce({ ok: true, rows: [] });
            mockRunQuery.mockReturnValueOnce({ ok: true, rows: [] });

            const result = TimeRegistry.getCalendarDictionary('cal_test');

            // Both full name and short name should map to same index
            expect(result.monthIndex['flob']).toBe(0);
            expect(result.monthIndex['flo']).toBe(0);
            expect(result.monthDays['flo']).toBe(30);
        });
    });

    describe('hasTimeUnits', () => {
        it('should return true when months exist', () => {
            mockRunQuery.mockReturnValue({
                ok: true,
                rows: [['id1', 'Flob', 'flob', 'Flo', 0, 30]]
            });

            const result = TimeRegistry.hasTimeUnits('cal_test');

            expect(result).toBe(true);
        });

        it('should return false when no months exist', () => {
            mockRunQuery.mockReturnValue({ ok: true, rows: [] });

            const result = TimeRegistry.hasTimeUnits('cal_test');

            expect(result).toBe(false);
        });
    });
});
