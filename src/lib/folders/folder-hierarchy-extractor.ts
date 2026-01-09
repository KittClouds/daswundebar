/**
 * FolderHierarchyExtractor - Extract relationships from folder tree structure
 * 
 * Walks the folder tree and creates semantic relationships based on:
 * - Folder schema definitions
 * - Parent-child containment
 * - Inherited entity kinds
 */

import { folderSchemaRegistry } from '@/lib/folders/schema-registry';
import { generateId } from '@/lib/utils/ids';
import type { Folder, Note } from '@/types/noteTypes';
import type { EntityKind } from '@/lib/types/entityTypes';
import type { UnifiedRelationship, RelationshipProvenance } from '@/lib/relationships/types';
import { RelationshipSource } from '@/lib/relationships/types';

export class FolderHierarchyExtractor {
    /**
     * Extract relationships from folder tree structure
     * Walks the tree and creates CONTAINS/PART_OF relationships
     */
    extractFromTree(folders: Folder[], notes: Note[]): UnifiedRelationship[] {
        const relationships: UnifiedRelationship[] = [];
        const folderMap = new Map(folders.map(f => [f.id, f]));

        for (const folder of folders) {
            if (!folder.entityKind) continue;

            const schema = folderSchemaRegistry.getSchema(folder.entityKind as EntityKind, folder.entitySubtype);

            const childFolders = folders.filter(f => f.parentId === folder.id);
            for (const child of childFolders) {
                const rel = this.createFolderRelationship(folder, child, schema);
                if (rel) relationships.push(rel);
            }

            const childNotes = notes.filter(n => n.folderId === folder.id);
            for (const note of childNotes) {
                const rel = this.createNoteRelationship(folder, note, schema);
                if (rel) relationships.push(rel);
            }
        }

        return relationships;
    }

    /**
     * Create relationship between parent folder and child folder
     */
    private createFolderRelationship(
        parent: Folder,
        child: Folder,
        schema: ReturnType<typeof folderSchemaRegistry.getSchema>
    ): UnifiedRelationship | null {
        if (!child.entityKind) return null;

        const subfolderDef = schema?.allowedSubfolders.find(
            sf => sf.entityKind === (child.entityKind as EntityKind) &&
                (sf.subtype === undefined || sf.subtype === child.entitySubtype)
        );

        const relDef = subfolderDef?.relationship;
        const relType = relDef?.relationshipType || 'CONTAINS';
        const inverseType = relDef?.inverseType || 'PART_OF';

        const sourceId = relDef?.sourceType === 'CHILD' ? child.id : parent.id;
        const targetId = relDef?.targetType === 'CHILD' ? child.id : parent.id;

        const provenance: RelationshipProvenance = {
            source: RelationshipSource.FOLDER_STRUCTURE,
            originId: parent.id,
            timestamp: new Date(),
            confidence: relDef?.defaultConfidence ?? 1.0,
            context: `Folder hierarchy: ${parent.name} > ${child.name}`,
            metadata: {
                parentFolderName: parent.name,
                childFolderName: child.name,
                category: relDef?.category || 'structural'
            }
        };

        return {
            id: generateId(),
            sourceEntityId: sourceId,
            targetEntityId: targetId,
            type: relType,
            inverseType,
            bidirectional: relDef?.bidirectional ?? false,
            confidence: provenance.confidence,
            confidenceBySource: {
                [RelationshipSource.FOLDER_STRUCTURE]: provenance.confidence
            },
            provenance: [provenance],
            namespace: 'folder_structure',
            attributes: {
                createdViaFolder: true,
                folderHierarchy: true,
                category: relDef?.category
            },
            createdAt: new Date(),
            updatedAt: new Date()
        };
    }

    /**
     * Create relationship between folder and child note
     */
    private createNoteRelationship(
        folder: Folder,
        note: Note,
        schema: ReturnType<typeof folderSchemaRegistry.getSchema>
    ): UnifiedRelationship | null {
        if (!note.isEntity || !note.entityKind) return null;

        const noteTypeDef = schema?.allowedNoteTypes?.find(
            nt => nt.entityKind === (note.entityKind as EntityKind) &&
                (nt.subtype === undefined || nt.subtype === note.entitySubtype)
        );

        const relDef = noteTypeDef?.relationship;
        if (!relDef) {
            return {
                id: generateId(),
                sourceEntityId: folder.id,
                targetEntityId: note.id,
                type: 'CONTAINS',
                inverseType: 'CONTAINED_BY',
                bidirectional: false,
                confidence: 0.8,
                confidenceBySource: {
                    [RelationshipSource.FOLDER_STRUCTURE]: 0.8
                },
                provenance: [{
                    source: RelationshipSource.FOLDER_STRUCTURE,
                    originId: folder.id,
                    timestamp: new Date(),
                    confidence: 0.8,
                    context: `Note in folder: ${folder.name} > ${note.title}`
                }],
                namespace: 'folder_structure',
                attributes: {
                    createdViaFolder: true,
                    impliedContainment: true
                },
                createdAt: new Date(),
                updatedAt: new Date()
            };
        }

        const sourceId = relDef.sourceType === 'CHILD' ? note.id : folder.id;
        const targetId = relDef.targetType === 'CHILD' ? note.id : folder.id;

        const provenance: RelationshipProvenance = {
            source: RelationshipSource.FOLDER_STRUCTURE,
            originId: folder.id,
            timestamp: new Date(),
            confidence: relDef.defaultConfidence ?? 1.0,
            context: `Note in typed folder: ${folder.name} > ${note.title}`,
            metadata: {
                folderName: folder.name,
                noteTitle: note.title,
                category: relDef.category
            }
        };

        return {
            id: generateId(),
            sourceEntityId: sourceId,
            targetEntityId: targetId,
            type: relDef.relationshipType,
            inverseType: relDef.inverseType,
            bidirectional: relDef.bidirectional ?? false,
            confidence: provenance.confidence,
            confidenceBySource: {
                [RelationshipSource.FOLDER_STRUCTURE]: provenance.confidence
            },
            provenance: [provenance],
            namespace: 'folder_structure',
            attributes: {
                createdViaFolder: true,
                category: relDef.category
            },
            createdAt: new Date(),
            updatedAt: new Date()
        };
    }

    /**
     * Infer inherited entity kind for items in typed folders
     */
    inferInheritedKind(item: Note | Folder, ancestorFolders: Folder[]): EntityKind | undefined {
        for (const ancestor of ancestorFolders) {
            if (ancestor.entityKind) {
                const schema = folderSchemaRegistry.getSchema(ancestor.entityKind as EntityKind);
                if (schema?.propagateKindToChildren) {
                    return ancestor.entityKind as EntityKind;
                }
            }
        }
        return undefined;
    }

    /**
     * Get ancestor chain for a folder
     */
    getAncestorChain(folderId: string | undefined, folderMap: Map<string, Folder>): Folder[] {
        const ancestors: Folder[] = [];
        let currentId = folderId;

        while (currentId) {
            const folder = folderMap.get(currentId);
            if (!folder) break;
            ancestors.push(folder);
            currentId = folder.parentId;
        }

        return ancestors;
    }
}

export const folderHierarchyExtractor = new FolderHierarchyExtractor();
