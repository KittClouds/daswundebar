import React, { useMemo, useCallback } from 'react';
import { useJotaiNotes } from '@/hooks/useJotaiNotes';
import { useUnifiedEntityAttributes } from '@/hooks/useUnifiedEntityAttributes';
import { FactSheetContainer } from '@/components/fact-sheets/FactSheetContainer';
import { EntitySelectionProvider, useEntitySelection } from '@/contexts/EntitySelectionContext';
// Use smartGraphRegistry (SYNC!) instead of async parseNoteConnectionsFromDocument
import { smartGraphRegistry } from '@/lib/tauri';
import type { ParsedEntity, EntityAttributes } from '@/types/factSheetTypes';
import type { EntityKind } from '@/lib/types/entityTypes';
import { toast } from 'sonner';
import { Focus, X } from 'lucide-react';
import { Button } from '@/components/ui/button';

/**
 * Inner component that has access to EntitySelectionContext
 */
function CalendarEntitySidebarInner() {
    const { state: { notes }, updateNoteContent } = useJotaiNotes();
    const {
        isGlobalFocusActive,
        globalFocusEntityLabel,
        setGlobalEntityFocus,
        clearGlobalFocus,
        selectedEntity,
    } = useEntitySelection();

    // Use unified entity attributes hook for bi-directional sync
    const unifiedAttrs = useUnifiedEntityAttributes(selectedEntity);

    // Gather ALL entities from SmartGraphRegistry (SYNC - no async needed!)
    const entityNotes = useMemo(() => {
        const allEntities: ParsedEntity[] = [];
        const seen = new Set<string>();

        // 1. Entity Notes (notes marked as entities)
        for (const note of notes) {
            if (note.isEntity && note.entityKind && note.entityLabel) {
                const key = `${note.entityKind}|${note.entityLabel}`;
                if (!seen.has(key)) {
                    seen.add(key);
                    allEntities.push({
                        kind: note.entityKind as EntityKind,
                        subtype: note.entitySubtype,
                        label: note.entityLabel,
                        noteId: note.id,
                        attributes: (() => {
                            try {
                                if (!note.content) return {};
                                const content = JSON.parse(note.content);
                                const attrKey = `${note.entityKind}|${note.entityLabel}`;
                                return content.entityAttributes?.[attrKey] || {};
                            } catch {
                                return {};
                            }
                        })(),
                    });
                }
            }
        }

        // 2. Get entities from SmartGraphRegistry (SYNC - from local cache!)
        try {
            const registryEntities = smartGraphRegistry.getAllEntities();
            for (const entity of registryEntities) {
                const key = `${entity.kind}|${entity.label}`;
                if (!seen.has(key)) {
                    seen.add(key);
                    allEntities.push({
                        kind: entity.kind as EntityKind,
                        label: entity.label,
                        attributes: {},
                    });
                }
            }
        } catch (err) {
            // Registry not ready yet - just use note entities
            console.log('[CalendarEntitySidebar] SmartGraphRegistry not ready, using note entities only');
        }

        return allEntities;
    }, [notes]);

    // Handle updates from the sidebar - now uses unified hook for bi-directional sync
    const handleEntityUpdate = useCallback(async (entity: ParsedEntity, attributes: EntityAttributes) => {
        // Update via unified hook - this syncs to both SQLite AND legacy note content
        await unifiedAttrs.setFields(attributes);

        // Note: The unified hook handles syncing to the note content automatically,
        // so we don't need to manually update the note here anymore.
        // This ensures Calendar and Notes views stay in sync.
    }, [unifiedAttrs.setFields]);

    // Set global focus when entity is selected
    const handleFocusClick = useCallback(() => {
        if (selectedEntity) {
            setGlobalEntityFocus(selectedEntity);
            toast.success(`Now viewing ${selectedEntity.label}'s timeline`);
        }
    }, [selectedEntity, setGlobalEntityFocus]);

    return (
        <div className="h-full border-l border-border bg-sidebar w-[380px] flex flex-col overflow-hidden transition-all">
            {/* Focus Indicator Banner */}
            {isGlobalFocusActive && (
                <div className="px-4 py-2 bg-primary/10 border-b border-primary/20 flex items-center justify-between">
                    <div className="flex items-center gap-2 text-sm">
                        <Focus className="h-4 w-4 text-primary" />
                        <span className="font-medium text-primary">
                            Viewing: {globalFocusEntityLabel}
                        </span>
                    </div>
                    <Button
                        variant="ghost"
                        size="sm"
                        onClick={clearGlobalFocus}
                        className="h-6 px-2 text-xs hover:bg-primary/10"
                    >
                        <X className="h-3 w-3 mr-1" />
                        Clear
                    </Button>
                </div>
            )}

            {/* Entity selected but not focused - show focus button */}
            {selectedEntity && !isGlobalFocusActive && (
                <div className="px-4 py-2 bg-muted/50 border-b border-border flex items-center justify-between">
                    <span className="text-sm text-muted-foreground">
                        {selectedEntity.label} selected
                    </span>
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={handleFocusClick}
                        className="h-6 px-2 text-xs"
                    >
                        <Focus className="h-3 w-3 mr-1" />
                        Focus View
                    </Button>
                </div>
            )}

            <FactSheetContainer
                externalEntities={entityNotes}
                onEntityUpdate={handleEntityUpdate}
            />
        </div>
    );
}

/**
 * CalendarEntitySidebar - Entity sidebar for the fantasy calendar
 * Wraps content in its own EntitySelectionProvider for independent state
 */
export function CalendarEntitySidebar() {
    return (
        <EntitySelectionProvider>
            <CalendarEntitySidebarInner />
        </EntitySelectionProvider>
    );
}
