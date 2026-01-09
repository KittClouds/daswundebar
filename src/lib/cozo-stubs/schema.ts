/**
 * CozoDB Schema Stubs - DEPRECATED
 * 
 * These query definitions are stubs for deprecated browser CozoDB.
 * All operations now go through Tauri.
 */

// Stub query objects - return no-op queries
export const ENTITY_QUERIES = {
    upsert: '',
    delete: '',
    getById: '',
    findByNameAndKind: '',
    findByName: '',
    getByKind: '',
    getByGroupId: '',
};

export const TIME_UNIT_QUERIES = {
    getMonthsByCalendar: '',
    getWeekdaysByCalendar: '',
    getErasByCalendar: '',
    findByName: '',
};

export const TEMPORAL_MENTION_QUERIES = {
    upsert: '',
    getByNoteId: '',
};

export const BIDIRECTIONAL_LINK_QUERIES = {
    upsert: '',
    getBySourceId: '',
    getByTargetId: '',
};

export const NETWORK_INSTANCE_QUERIES = {
    create: '',
    getById: '',
};

export const NETWORK_MEMBERSHIP_QUERIES = {
    create: '',
    getByNetworkId: '',
};

export const NETWORK_RELATIONSHIP_QUERIES = {
    create: '',
    getByNetworkId: '',
};

export const FOLDER_HIERARCHY_QUERIES = {
    create: '',
    getByParentId: '',
};

// Stub TimeUnitType
export enum TimeUnitType {
    MONTH = 'month',
    WEEKDAY = 'weekday',
    ERA = 'era',
}

export interface TimeUnitRow {
    id: string;
    name: string;
    unit_type: TimeUnitType;
}
