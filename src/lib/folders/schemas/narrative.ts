/**
 * Narrative Folder Schema
 * 
 * Defines the semantic structure for NARRATIVE-type folders:
 * - Story structure (Acts, Chapters, Scenes, Beats)
 * - Character arcs
 * - Timelines with temporal ordering
 * - World-building elements
 * 
 * All temporal children (Acts, Chapters, Scenes, Events, Beats) 
 * auto-create PRECEDES/FOLLOWS relationships based on sibling order.
 */

import type { FolderSchema } from '../schemas';

export const NARRATIVE_FOLDER_SCHEMA: FolderSchema = {
    entityKind: 'NARRATIVE',
    name: 'Narrative',
    description: 'A complete story, series, or narrative project with temporal structure',

    allowedSubfolders: [
        // Story arcs (temporally ordered) - EQUAL PRIORITY WITH ACTS
        {
            entityKind: 'ARC',
            label: 'Story Arcs',
            icon: 'Waves',
            description: 'Major storylines and arcs (temporally ordered)',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },

        // Acts - major story divisions (temporally ordered)
        {
            entityKind: 'ACT',
            label: 'Acts',
            icon: 'Drama',
            description: 'Major story divisions (temporally ordered)',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },

        // Chapters (temporally ordered)
        {
            entityKind: 'CHAPTER',
            label: 'Chapters',
            icon: 'BookOpen',
            description: 'Chapter divisions (temporally ordered)',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },

        // Scenes (temporally ordered)
        {
            entityKind: 'SCENE',
            label: 'Scenes',
            icon: 'Film',
            description: 'Individual scenes (temporally ordered)',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },

        // Events (temporally ordered)
        {
            entityKind: 'EVENT',
            label: 'Events',
            icon: 'Calendar',
            description: 'Story events (temporally ordered)',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },

        // Beats (temporally ordered)
        {
            entityKind: 'BEAT',
            label: 'Beats',
            icon: 'Zap',
            description: 'Story beats (temporally ordered)',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },

        // Characters in this narrative
        {
            entityKind: 'CHARACTER',
            label: 'Characters',
            icon: 'User',
            description: 'Characters in this narrative',
            relationship: {
                relationshipType: 'FEATURES',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'APPEARS_IN',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },

        // Locations in this narrative
        {
            entityKind: 'LOCATION',
            label: 'Locations',
            icon: 'MapPin',
            description: 'Locations and settings',
            relationship: {
                relationshipType: 'SET_IN',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'SETTING_FOR',
                category: 'spatial',
                defaultConfidence: 1.0,
            },
        },

        // Timelines (sub-timelines within the narrative)
        {
            entityKind: 'TIMELINE',
            label: 'Timelines',
            icon: 'Hourglass',
            description: 'Story timelines and chronologies',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'TIMELINE_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },

        // World-building concepts
        {
            entityKind: 'CONCEPT',
            label: 'World Building',
            icon: 'Lightbulb',
            description: 'Concepts, magic systems, and world rules',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },
    ],

    allowedNoteTypes: [
        {
            entityKind: 'NARRATIVE',
            label: 'Story Overview',
            icon: 'Book',
            relationship: {
                relationshipType: 'DESCRIBES',
                sourceType: 'CHILD',
                targetType: 'PARENT',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'SCENE',
            label: 'New Scene',
            icon: 'Film',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'EVENT',
            label: 'New Event',
            icon: 'Calendar',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'BEAT',
            label: 'New Beat',
            icon: 'Zap',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
    ],

    color: '#4f46e5', // Indigo - matches ENTITY_COLORS.NARRATIVE
    icon: 'Book',
    containerOnly: false,
    propagateKindToChildren: false,

    customAttributes: [
        { name: 'genre', type: 'string' },
        { name: 'status', type: 'string' },
        { name: 'wordCount', type: 'number' },
        { name: 'targetAudience', type: 'string' },
    ],
};
