/**
 * Scene Folder Schema
 * 
 * Defines the semantic structure for SCENE-type folders:
 * - Beats as subfolders (the atomic units of narrative action)
 * - Characters present in the scene
 * - Location where scene takes place
 */

import type { FolderSchema } from '../schemas';

export const SCENE_FOLDER_SCHEMA: FolderSchema = {
    entityKind: 'SCENE',
    name: 'Scene',
    description: 'A continuous unit of action in a single location and time',

    allowedSubfolders: [
        // Temporal structure
        {
            entityKind: 'EVENT',
            label: 'Events',
            icon: 'Calendar',
            description: 'Events that occur within this scene',
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
            description: 'Individual moments or beats within this scene',
            relationship: {
                relationshipType: 'CONTAINS',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'PART_OF',
                category: 'temporal',
                defaultConfidence: 1.0,
            },
        },
        // Entity attachments (WHO/WHERE/WHAT)
        {
            entityKind: 'CHARACTER',
            label: 'Characters Present',
            icon: 'User',
            description: 'Characters who appear in this scene',
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
            label: 'NPCs Present',
            icon: 'Users2',
            description: 'NPCs who appear in this scene',
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
            label: 'Scene Location',
            icon: 'MapPin',
            description: 'Where this scene takes place',
            relationship: {
                relationshipType: 'SET_AT',
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
            description: 'Items featured in this scene',
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
            entityKind: 'SCENE',
            label: 'Scene Description',
            icon: 'Film',
            relationship: {
                relationshipType: 'DESCRIBES',
                sourceType: 'CHILD',
                targetType: 'PARENT',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },
        {
            entityKind: 'BEAT',
            label: 'Beat',
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

    color: '#ec4899', // Pink - matches ENTITY_COLORS.SCENE
    icon: 'Film',
    propagateKindToChildren: false,

    customAttributes: [
        { name: 'pov', type: 'string' },
        { name: 'mood', type: 'string' },
        { name: 'tension', type: 'number' },
        { name: 'purpose', type: 'string' },
    ],
};
