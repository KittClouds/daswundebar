/**
 * Arc Folder Schema
 * 
 * Defines the semantic structure for ARC-type folders:
 * - Story arcs contain chapters, which contain scenes and beats
 * - Hierarchy: Arc -> Chapter -> Scene -> Beat
 * - Arcs and Acts are interchangeable in structure
 */

import type { FolderSchema } from '../schemas';

export const ARC_FOLDER_SCHEMA: FolderSchema = {
    entityKind: 'ARC',
    name: 'Story Arc',
    description: 'A major storyline or narrative arc',

    allowedSubfolders: [
        // Nested narrative structures
        {
            entityKind: 'ARC',
            label: 'Nested Arc',
            icon: 'Waves',
            description: 'Sub-arc within this arc',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'ACT',
            label: 'Acts',
            icon: 'Drama',
            description: 'Acts within this arc',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'EVENT',
            label: 'Key Events',
            icon: 'Calendar',
            description: 'Major events in this arc',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'CHAPTER',
            label: 'Chapters',
            icon: 'BookOpen',
            description: 'Chapters within this arc',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'SCENE',
            label: 'Scenes',
            icon: 'Film',
            description: 'Scenes directly under this arc',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'BEAT',
            label: 'Beats',
            icon: 'Zap',
            description: 'Key beats in this arc',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
        // Entity attachments
        {
            entityKind: 'CHARACTER',
            label: 'Characters',
            icon: 'User',
            description: 'Characters featured in this arc',
            relationship: {
                relationshipType: 'FEATURES',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'APPEARS_IN',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'NPC',
            label: 'NPCs',
            icon: 'Users2',
            description: 'NPCs featured in this arc',
            relationship: {
                relationshipType: 'FEATURES',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'APPEARS_IN',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'LOCATION',
            label: 'Locations',
            icon: 'MapPin',
            description: 'Locations in this arc',
            relationship: {
                relationshipType: 'SET_IN',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'SETTING_FOR',
                category: 'spatial',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'ITEM',
            label: 'Items',
            icon: 'Box',
            description: 'Items featured in this arc',
            relationship: {
                relationshipType: 'FEATURES',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'FEATURED_IN',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },
    ],

    allowedNoteTypes: [
        {
            entityKind: 'ARC',
            label: 'Arc Overview',
            icon: 'Waves',
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
            label: 'Scene',
            icon: 'Film',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
    ],

    color: '#a855f7', // Violet - matches ENTITY_COLORS.ARC
    icon: 'Waves',
    propagateKindToChildren: false,

    customAttributes: [
        { name: 'arcType', type: 'string' },
        { name: 'status', type: 'string' },
        { name: 'theme', type: 'string' },
        { name: 'stakes', type: 'string' },
    ],
};
