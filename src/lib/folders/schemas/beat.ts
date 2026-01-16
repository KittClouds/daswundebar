/**
 * Beat Folder Schema
 * 
 * Defines the semantic structure for BEAT-type folders:
 * - Atomic narrative moments (no sub-hierarchy)
 * - Entity attachments: Characters, NPCs, Locations, Items
 * - Tracks WHO, WHERE, and WHAT for each beat
 */

import type { FolderSchema } from '../schemas';

export const BEAT_FOLDER_SCHEMA: FolderSchema = {
    entityKind: 'BEAT',
    name: 'Beat',
    description: 'Atomic narrative moment — smallest unit of story action',

    allowedSubfolders: [
        // WHO is in this beat? (Characters)
        {
            entityKind: 'CHARACTER',
            label: 'Characters',
            icon: 'User',
            description: 'Characters present in this beat',
            relationship: {
                relationshipType: 'FEATURES',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'APPEARS_IN',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },
        // WHO is in this beat? (NPCs)
        {
            entityKind: 'NPC',
            label: 'NPCs',
            icon: 'Users2',
            description: 'NPCs present in this beat',
            relationship: {
                relationshipType: 'FEATURES',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'APPEARS_IN',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },
        // WHERE does this beat happen?
        {
            entityKind: 'LOCATION',
            label: 'Location',
            icon: 'MapPin',
            description: 'Where this beat takes place',
            relationship: {
                relationshipType: 'SET_AT',
                sourceType: 'PARENT',
                targetType: 'CHILD',
                inverseType: 'SETTING_FOR',
                category: 'spatial',
                defaultConfidence: 1.0,
            },
        },
        // WHAT items are involved?
        {
            entityKind: 'ITEM',
            label: 'Items',
            icon: 'Box',
            description: 'Items featured in this beat',
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
            entityKind: 'BEAT',
            label: 'Beat Description',
            icon: 'Zap',
            relationship: {
                relationshipType: 'DESCRIBES',
                sourceType: 'CHILD',
                targetType: 'PARENT',
                category: 'custom',
                defaultConfidence: 1.0,
            },
        },
    ],

    color: '#facc15', // Yellow - beat color
    icon: 'Zap',
    propagateKindToChildren: false,

    customAttributes: [
        { name: 'intensity', type: 'number' },
        { name: 'emotionalTone', type: 'string' },
        { name: 'conflict', type: 'string' },
    ],
};
