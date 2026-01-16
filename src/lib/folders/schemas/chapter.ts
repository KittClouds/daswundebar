/**
 * Chapter Folder Schema
 * 
 * Defines the semantic structure for CHAPTER-type folders:
 * - Scene subfolders (which contain Beats)
 * - Beat subfolders directly
 * - This creates the hierarchy: Chapter -> Scene -> Beat
 */

import type { FolderSchema } from '../schemas';

export const CHAPTER_FOLDER_SCHEMA: FolderSchema = {
    entityKind: 'CHAPTER',
    name: 'Chapter',
    description: 'A chapter or major section of narrative content',

    allowedSubfolders: [
        // Temporal structure
        {
            entityKind: 'EVENT',
            label: 'Events',
            icon: 'Calendar',
            description: 'Events that occur in this chapter',
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
            description: 'Scenes within this chapter',
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
            description: 'Individual beats within this chapter',
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
            description: 'Characters featured in this chapter',
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
            description: 'NPCs featured in this chapter',
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
            description: 'Locations in this chapter',
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
            description: 'Items featured in this chapter',
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
            entityKind: 'CHAPTER',
            label: 'Chapter Overview',
            icon: 'BookOpen',
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

    color: '#14b8a6', // Teal - matches ENTITY_COLORS.CHAPTER
    icon: 'BookOpen',
    propagateKindToChildren: false,

    customAttributes: [
        { name: 'wordCount', type: 'number' },
        { name: 'status', type: 'string' },
        { name: 'pov', type: 'string' },
        { name: 'summary', type: 'string' },
    ],
};
