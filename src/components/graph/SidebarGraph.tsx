
import { useMemo } from 'react';
import { useJotaiNotes } from '@/hooks/useJotaiNotes';
import Force3DGraphView from './Force3DGraphView';
import type { Force3DGraphData, Force3DNode, Force3DLink } from '@/lib/graph/types/graph-types';
import { ENTITY_COLORS } from '@/lib/graph/types/graph-types';

interface SidebarGraphProps {
    className?: string;
}

export function SidebarGraph({ className }: SidebarGraphProps) {
    const { state, selectNote } = useJotaiNotes();

    const graphData = useMemo<Force3DGraphData>(() => {
        const nodes: Force3DNode[] = [];
        const links: Force3DLink[] = [];

        // Map Folders
        state.folders.forEach(folder => {
            nodes.push({
                id: folder.id,
                label: folder.name,
                type: 'FOLDER',
                color: folder.color || ENTITY_COLORS.FOLDER || '#52525b',
                size: 8
            });

            if (folder.parent_id) {
                links.push({
                    id: `${folder.parent_id}->${folder.id}`,
                    source: folder.parent_id,
                    target: folder.id,
                    type: 'PARENT',
                    color: 'rgba(239, 68, 68, 0.5)', // Red-500 with opacity
                });
            }
        });

        // Map Notes
        state.notes.forEach(note => {
            // Determine color from entity kind if available
            let color = ENTITY_COLORS.NOTE || '#71717a';
            if (note.isEntity && note.entityKind && ENTITY_COLORS[note.entityKind]) {
                color = ENTITY_COLORS[note.entityKind];
            }

            nodes.push({
                id: note.id,
                label: note.title,
                type: 'NOTE',
                color: color,
                size: 6
            });

            if (note.folderId) {
                links.push({
                    id: `${note.folderId}->${note.id}`,
                    source: note.folderId,
                    target: note.id,
                    type: 'CONTAINS',
                    color: 'rgba(239, 68, 68, 0.5)', // Red-500 with opacity
                });
            }
        });

        return { nodes, links };
    }, [state.folders, state.notes]);

    return (
        <div className={className}>
            <Force3DGraphView
                data={graphData}
                onNodeClick={(node) => {
                    if (node.type === 'NOTE') {
                        selectNote(node.id);
                    }
                }}
            />
        </div>
    );
}

export default SidebarGraph;
