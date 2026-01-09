import React, { useRef, useCallback, useMemo, useEffect } from 'react';
import { Tree, TreeApi, NodeApi } from 'react-arborist';
import { ArboristNode } from '@/lib/arborist/types';
import { ArboristTreeNode } from './ArboristTreeNode';
import { buildArboristTree } from '@/lib/arborist/adapter';
import { TypedFolderMenu } from './TypedFolderMenu';
import type { EntityKind } from '@/lib/types/entityTypes';

import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useJotaiNotes } from '@/hooks/useJotaiNotes';
import { cn } from '@/lib/utils';
import { Pencil, Plus, X, FolderPlus, Star, StarOff, LinkIcon } from 'lucide-react';

interface ArboristTreeViewProps {
    searchTerm?: string;
    className?: string;
}

/**
 * ArboristTreeView - Sleeker tree view using V2 node component (now standard)
 * 
 * Visual changes:
 * - Smaller row height (28px vs 32px)
 * - Tighter indent (16px vs 20px)
 * - Uses V2 CSS class for refined styling
 * 
 * Functionality preserved 1:1:
 * - All selection, rename, move callbacks
 * - Context menu with full functionality
 * - TypedFolderMenu integration
 * - Search filtering
 * - Resize observer
 */
export function ArboristTreeView({
    searchTerm = '',
    className
}: ArboristTreeViewProps) {
    const {
        folderTree,
        globalNotes,
        createNote,
        createFolder,
        updateNote,
        updateFolder,
        deleteNote,
        deleteFolder,
        selectNote,
    } = useJotaiNotes();

    const treeRef = useRef<TreeApi<ArboristNode>>(null);
    const [treeHeight, setTreeHeight] = React.useState(600);
    const [treeWidth, setTreeWidth] = React.useState(300);
    const containerRef = useRef<HTMLDivElement>(null);
    const [contextNode, setContextNode] = React.useState<ArboristNode | null>(null);
    const [contextMenuPos, setContextMenuPos] = React.useState({ x: 0, y: 0 });

    useEffect(() => {
        if (!containerRef.current) return;

        const observer = new ResizeObserver((entries) => {
            for (const entry of entries) {
                setTreeHeight(entry.contentRect.height || 600);
                setTreeWidth(entry.contentRect.width || 300);
            }
        });

        observer.observe(containerRef.current);
        return () => observer.disconnect();
    }, []);

    // Build Arborist-compatible tree - PRESERVED 1:1
    const treeData = useMemo(
        () => buildArboristTree(folderTree, globalNotes),
        [folderTree, globalNotes]
    );

    // Handle node selection - PRESERVED 1:1
    const handleSelect = useCallback((nodes: NodeApi<ArboristNode>[]) => {
        const node = nodes[0];
        if (!node) return;

        if (node.data.type === 'note') {
            selectNote(node.id);
        }
    }, [selectNote]);

    // Handle node rename - TYPE-AWARE with silent conversion
    const handleRename = useCallback(({ node, name }: { node: any; name: string }) => {
        const nodeId = node.id;
        const nodeData = node.data;

        // Import type-aware utilities
        const { parseTypedName, applyTypeAwareRename } = require('@/lib/arborist/type-aware-rename');

        // Build rename context
        const context = {
            currentName: nodeData.name,
            entityKind: nodeData.entityKind,
            inheritedKind: nodeData.inheritedKind,
            isTypedRoot: nodeData.isTypedRoot,
            hasChildren: nodeData.children?.length > 0,
        };

        // Apply type-aware rename
        const result = applyTypeAwareRename(name, context);

        if (nodeData.type === 'folder') {
            const updates: Record<string, any> = { name: result.newName };

            // Silent type conversion
            if (result.shouldUpdateType) {
                updates.entityKind = result.newEntityKind || null;
                updates.entity_kind = result.newEntityKind || null;
                updates.entitySubtype = result.newEntitySubtype || null;
                updates.entity_subtype = result.newEntitySubtype || null;
                // Auto-set as typed root if converting to typed
                if (result.newEntityKind && !nodeData.entityKind) {
                    updates.isTypedRoot = true;
                    updates.is_typed_root = true;
                }
            }

            updateFolder(nodeId, updates);
        } else {
            const updates: Record<string, any> = { title: result.newName };

            // Silent type conversion for notes
            if (result.shouldUpdateType) {
                updates.entityKind = result.newEntityKind || null;
                updates.entity_kind = result.newEntityKind || null;
                updates.entitySubtype = result.newEntitySubtype || null;
                updates.entity_subtype = result.newEntitySubtype || null;
                updates.isEntity = !!result.newEntityKind;
                updates.is_entity = !!result.newEntityKind;
            }

            updateNote(nodeId, updates);
        }
    }, [updateFolder, updateNote]);

    // Handle node move - PRESERVED 1:1
    const handleMove = useCallback(async ({ dragIds, parentId, index }: {
        dragIds: string[];
        parentId: string | null;
        index: number;
    }) => {
        const dragId = dragIds[0];
        const node = treeRef.current?.get(dragId);
        if (!node) return;

        const targetParent = parentId ? treeRef.current?.get(parentId) : null;

        if (node.data.type === 'folder') {
            await updateFolder(dragId, { parent_id: parentId || null });

            if (targetParent?.data.entityKind && node.data.entityKind) {
                try {
                    const { onEntityAddedToFolder } = await import('@/lib/folders');
                    await onEntityAddedToFolder(
                        parentId!,
                        dragId,
                        node.data.entityKind,
                        node.data.name
                    );
                } catch (error) {
                    console.log('[ArboristTreeView] Network auto-creation check:', error);
                }
            }
        } else {
            await updateNote(dragId, { parent_id: parentId || null });

            if (targetParent?.data.entityKind && node.data.entityKind) {
                try {
                    const { onEntityAddedToFolder } = await import('@/lib/folders');
                    await onEntityAddedToFolder(
                        parentId!,
                        dragId,
                        node.data.entityKind,
                        node.data.name
                    );
                } catch (error) {
                    console.log('[ArboristTreeView] Network auto-creation check:', error);
                }
            }
        }
    }, [updateFolder, updateNote]);

    // Handle context menu - PRESERVED 1:1
    const handleContextMenu = useCallback((node: ArboristNode, e: React.MouseEvent) => {
        e.preventDefault();
        setContextNode(node);
        setContextMenuPos({ x: e.clientX, y: e.clientY });
    }, []);

    // Context menu actions - ALL PRESERVED 1:1
    const handleDeleteNode = useCallback(() => {
        if (!contextNode) return;
        if (contextNode.type === 'folder') {
            deleteFolder(contextNode.id);
        } else {
            deleteNote(contextNode.id);
        }
        setContextNode(null);
    }, [contextNode, deleteFolder, deleteNote]);

    const handleToggleFavorite = useCallback(() => {
        if (!contextNode || contextNode.type !== 'note') return;
        updateNote(contextNode.id, { favorite: contextNode.favorite === 1 ? 0 : 1 });
        setContextNode(null);
    }, [contextNode, updateNote]);

    const handleCopyLink = useCallback(() => {
        if (!contextNode || contextNode.type !== 'note') return;
        const url = `${window.location.origin}/note/${contextNode.id}`;
        navigator.clipboard.writeText(url);
        setContextNode(null);
    }, [contextNode]);

    const handleTypedNoteCreate = useCallback((kind?: EntityKind, subtype?: string) => {
        if (!contextNode || contextNode.type !== 'folder') return;

        let title = "New Note";
        if (kind) {
            title = subtype ? `[${kind}:${subtype}] New ${subtype}` : `[${kind}] New ${kind}`;
        }

        createNote(contextNode.id, title);
        setContextNode(null);
    }, [contextNode, createNote]);

    const handleTypedSubfolderCreate = useCallback((kind: EntityKind, subtype?: string, label?: string) => {
        if (!contextNode || contextNode.type !== 'folder') return;

        const name = label
            ? (subtype ? `[${kind}:${subtype}|${label}]` : `[${kind}|${label}]`)
            : (subtype ? `[${kind}:${subtype}]` : `[${kind}]`);

        createFolder(name, contextNode.id, {
            entityKind: kind,
            entitySubtype: subtype,
            entityLabel: label,
            isTypedRoot: !label && !subtype,
            isSubtypeRoot: !label && !!subtype,
        });
        setContextNode(null);
    }, [contextNode, createFolder]);

    const handleCreateNote = useCallback(() => {
        if (!contextNode || contextNode.type !== 'folder') return;
        createNote(contextNode.id);
        setContextNode(null);
    }, [contextNode, createNote]);

    const handleCreateSubfolder = useCallback(() => {
        if (!contextNode || contextNode.type !== 'folder') return;
        createFolder("New Folder", contextNode.id);
        setContextNode(null);
    }, [contextNode, createFolder]);

    // Search filtering - PRESERVED 1:1
    const searchMatch = useCallback((node: any) => {
        if (!searchTerm) return true;
        const query = searchTerm.toLowerCase();
        return node.data.name.toLowerCase().includes(query);
    }, [searchTerm]);

    return (
        <div ref={containerRef} className={cn("flex-1 overflow-hidden min-h-[400px]", className)}>
            <Tree
                ref={treeRef}
                data={treeData}
                width={treeWidth}
                height={treeHeight}
                rowHeight={28}  // V2: Smaller row height
                indent={16}     // V2: Tighter indent
                overscanCount={10}
                searchTerm={searchTerm}
                searchMatch={searchMatch}
                onSelect={handleSelect}
                onRename={handleRename}
                onMove={handleMove}
                disableDrag={false}
                disableDrop={false}
                className="arborist-tree-v2"
            >
                {(props) => (
                    <ArboristTreeNode
                        {...props}
                        onContextMenu={handleContextMenu}
                    />
                )}
            </Tree>

            {/* Context Menu - PRESERVED 1:1 */}
            <DropdownMenu
                open={!!contextNode}
                onOpenChange={(open) => !open && setContextNode(null)}
            >
                <DropdownMenuTrigger asChild>
                    <div
                        style={{
                            position: 'fixed',
                            left: contextMenuPos.x,
                            top: contextMenuPos.y,
                            width: 1,
                            height: 1,
                            visibility: 'hidden'
                        }}
                    />
                </DropdownMenuTrigger>
                <DropdownMenuContent align="start">
                    {contextNode?.type === 'folder' ? (
                        <>
                            <DropdownMenuItem onClick={handleCreateNote}>
                                <Plus className="mr-2 h-4 w-4" />
                                New note
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={handleCreateSubfolder}>
                                <FolderPlus className="mr-2 h-4 w-4" />
                                New subfolder
                            </DropdownMenuItem>

                            {contextNode.entityKind && (
                                <>
                                    <DropdownMenuSeparator />
                                    <TypedFolderMenu
                                        parentFolder={contextNode.folderData!}
                                        onCreateSubfolder={handleTypedSubfolderCreate}
                                        onCreateNote={handleTypedNoteCreate}
                                    />
                                </>
                            )}
                            <DropdownMenuSeparator />
                            <DropdownMenuItem onClick={() => {
                                const nodeId = contextNode.id;
                                setContextNode(null); // Close menu first
                                setTimeout(() => treeRef.current?.edit(nodeId), 0);
                            }}>
                                <Pencil className="mr-2 h-4 w-4" />
                                Rename
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                                onClick={handleDeleteNode}
                                className="text-destructive focus:text-destructive"
                            >
                                <X className="mr-2 h-4 w-4" />
                                Delete folder
                            </DropdownMenuItem>
                        </>
                    ) : (
                        <>
                            <DropdownMenuItem onClick={() => {
                                const nodeId = contextNode?.id || '';
                                setContextNode(null); // Close menu first
                                setTimeout(() => treeRef.current?.edit(nodeId), 0);
                            }}>
                                <Pencil className="mr-2 h-4 w-4" />
                                Rename note
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem onClick={handleToggleFavorite}>
                                {contextNode?.favorite ? (
                                    <>
                                        <StarOff className="mr-2 h-4 w-4" />
                                        Remove from favorites
                                    </>
                                ) : (
                                    <>
                                        <Star className="mr-2 h-4 w-4" />
                                        Add to favorites
                                    </>
                                )}
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={handleCopyLink}>
                                <LinkIcon className="mr-2 h-4 w-4" />
                                Copy link
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem
                                onClick={handleDeleteNode}
                                className="text-destructive focus:text-destructive"
                            >
                                <X className="mr-2 h-4 w-4" />
                                Delete note
                            </DropdownMenuItem>
                        </>
                    )}
                </DropdownMenuContent>
            </DropdownMenu>
        </div>
    );
}
