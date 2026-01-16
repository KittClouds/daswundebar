/**
 * Act Folder Schema
 * 
 * Defines the semantic structure for ACT-type folders:
 * - Acts contain chapters, which contain scenes and beats
 * - Hierarchy: Act -> Chapter -> Scene -> Beat
 * - Acts and Arcs are interchangeable in structure
 */

import type { FolderSchema } from '../schemas';

export const ACT_FOLDER_SCHEMA: FolderSchema = {
    entityKind: 'ACT',
    name: 'Act',
    description: 'A major division of a story (e.g., three-act structure)',

    allowedSubfolders: [
        // Nested narrative structures
        {
            entityKind: 'ACT',
            label: 'Nested Act',
            icon: 'Drama',
            description: 'Sub-act within this act',
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
            entityKind: 'ARC',
            label: 'Story Arcs',
            icon: 'Waves',
            description: 'Story arcs within this act',
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
            description: 'Major events in this act',
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
            description: 'Chapters within this act',
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
            description: 'Scenes directly under this act',
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
            description: 'Key beats in this act',
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
            description: 'Characters featured in this act',
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
            description: 'NPCs featured in this act',
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
            description: 'Locations in this act',
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
            description: 'Items featured in this act',
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
            entityKind: 'ACT',
            label: 'Act Overview',
            icon: 'Drama',
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

    color: '#2563eb', // Royal Blue - matches ENTITY_COLORS.ACT
    icon: 'Drama',
    propagateKindToChildren: false,

    customAttributes: [
        { name: 'actNumber', type: 'number' },
        { name: 'status', type: 'string' },
        { name: 'purpose', type: 'string' },
        { name: 'turningPoint', type: 'string' },
    ],
};
