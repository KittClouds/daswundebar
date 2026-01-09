/**
 * Network Auto-Creator Tests
 * 
 * Tests for automatic network creation when folder schema conditions are met.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { folderSchemaRegistry } from '../../schema-registry';
import type { NetworkAutoCreateConfig } from '../network-auto-creator';

// Mock CozoDB
vi.mock('@/lib/cozo-stubs/db', () => ({
    cozoDb: {
        runQuery: vi.fn().mockReturnValue({ ok: true, rows: [] }),
        isReady: vi.fn().mockReturnValue(true),
    },
}));

describe('NetworkAutoCreator', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    describe('Schema Registry Integration', () => {
        it('should return network config for Allies subfolder', () => {
            const config = folderSchemaRegistry.getNetworkCreationConfig(
                'CHARACTER',
                undefined,
                'CHARACTER',
                'ALLY'
            );

            expect(config).toBeDefined();
            expect(config?.autoCreate).toBe(true);
            expect(config?.schemaId).toBe('SOCIAL_CIRCLE');
            expect(config?.threshold).toBe(2);
        });

        it('should return network config for Enemies subfolder', () => {
            const config = folderSchemaRegistry.getNetworkCreationConfig(
                'CHARACTER',
                undefined,
                'CHARACTER',
                'ENEMY'
            );

            expect(config).toBeDefined();
            expect(config?.autoCreate).toBe(true);
            expect(config?.schemaId).toBe('RIVALRY');
            expect(config?.threshold).toBe(2);
        });

        it('should return network config for Family subfolder', () => {
            // Family uses subtype 'ALLY' per the schema definition
            const config = folderSchemaRegistry.getNetworkCreationConfig(
                'CHARACTER',
                undefined,
                'CHARACTER',
                'ALLY' // Family uses ALLY subtype
            );

            expect(config).toBeDefined();
            expect(config?.autoCreate).toBe(true);
        });

        it('should return undefined for Possessions subfolder (no network)', () => {
            const config = folderSchemaRegistry.getNetworkCreationConfig(
                'CHARACTER',
                undefined,
                'ITEM',
                undefined
            );

            // Possessions doesn't have autoCreateNetwork
            expect(config).toBeUndefined();
        });

        it('should return undefined for non-typed folder', () => {
            const config = folderSchemaRegistry.getNetworkCreationConfig(
                'UNKNOWN' as any,
                undefined,
                'CHARACTER',
                undefined
            );

            expect(config).toBeUndefined();
        });

        it('should return all network trigger subfolders for CHARACTER', () => {
            const triggers = folderSchemaRegistry.getNetworkTriggerSubfolders('CHARACTER');

            // Allies, Enemies, Family have autoCreateNetwork: true
            expect(triggers.length).toBeGreaterThanOrEqual(3);
            expect(triggers.some(t => t.networkSchemaId === 'SOCIAL_CIRCLE')).toBe(true);
            expect(triggers.some(t => t.networkSchemaId === 'RIVALRY')).toBe(true);
            expect(triggers.some(t => t.networkSchemaId === 'FAMILY')).toBe(true);
        });
    });

    describe('Network Creation Logic', () => {
        it('should NOT create network when threshold not met', async () => {
            // Import dynamically to use mocked cozoDb
            const { checkAndCreateNetworkForFolder } = await import('../network-auto-creator');

            const config: NetworkAutoCreateConfig = {
                schemaId: 'SOCIAL_CIRCLE',
                threshold: 2,
                rootFolderId: 'folder-123',
                rootEntityId: 'entity-123',
                rootEntityName: 'Jon Snow',
                subfolderLabel: 'Allies',
                entityKind: 'CHARACTER',
            };

            const result = await checkAndCreateNetworkForFolder(config, 1);

            expect(result.created).toBe(false);
            expect(result.reason).toContain('below threshold');
        });

        it('should create network when threshold reached', async () => {
            const { checkAndCreateNetworkForFolder } = await import('../network-auto-creator');

            const config: NetworkAutoCreateConfig = {
                schemaId: 'SOCIAL_CIRCLE',
                threshold: 2,
                rootFolderId: 'folder-123',
                rootEntityId: 'entity-123',
                rootEntityName: 'Jon Snow',
                subfolderLabel: 'Allies',
                entityKind: 'CHARACTER',
            };

            const result = await checkAndCreateNetworkForFolder(config, 2);

            expect(result.created).toBe(true);
            expect(result.networkName).toBe("Jon Snow's Allies");
            expect(result.memberCount).toBe(3); // 2 allies + root
        });

        it('should NOT duplicate network on additional children', async () => {
            const { cozoDb } = await import('@/lib/cozo-stubs/db');

            // Mock existing network
            vi.mocked(cozoDb.runQuery).mockReturnValueOnce({
                ok: true,
                rows: [['existing-network-id', 'Existing Network']],
            });

            const { checkAndCreateNetworkForFolder } = await import('../network-auto-creator');

            const config: NetworkAutoCreateConfig = {
                schemaId: 'SOCIAL_CIRCLE',
                threshold: 2,
                rootFolderId: 'folder-123',
                rootEntityId: 'entity-123',
                rootEntityName: 'Jon Snow',
                subfolderLabel: 'Allies',
                entityKind: 'CHARACTER',
            };

            const result = await checkAndCreateNetworkForFolder(config, 5);

            expect(result.created).toBe(false);
            expect(result.networkId).toBe('existing-network-id');
            expect(result.reason).toContain('already exists');
        });

        it('should use correct naming pattern', async () => {
            const { checkAndCreateNetworkForFolder } = await import('../network-auto-creator');

            const result = await checkAndCreateNetworkForFolder({
                schemaId: 'FAMILY',
                threshold: 2,
                rootFolderId: 'folder-456',
                rootEntityId: 'entity-456',
                rootEntityName: 'Ned Stark',
                subfolderLabel: 'Family Members',
                entityKind: 'CHARACTER',
            }, 3);

            expect(result.created).toBe(true);
            expect(result.networkName).toBe("Ned Stark's Family Members");
        });
    });
});
