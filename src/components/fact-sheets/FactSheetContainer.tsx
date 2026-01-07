import React, { useMemo, useEffect, useCallback, useState } from 'react';
import { useJotaiNotes } from '@/hooks/useJotaiNotes';
import { useEntitySelection, EntitySelectionProvider } from '@/contexts/EntitySelectionContext';
import { useUnifiedEntityAttributes } from '@/hooks/useUnifiedEntityAttributes';
// Use smartGraphRegistry (SYNC!) instead of async parseNoteConnectionsFromDocument
import { smartGraphRegistry } from '@/lib/tauri';
import type { ParsedEntity, EntityAttributes } from '@/types/factSheetTypes';
import type { EntityKind } from '@/lib/types/entityTypes';
import { FileQuestion, Sparkles, BrainCircuit, LayoutGrid, List, Plus } from 'lucide-react';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { SettingsManager } from '@/lib/settings';
import { ModelRegistry, ModelId } from '@/lib/llm';
import { Button } from '@/components/ui/button';

// Import all fact sheet components
import { CharacterFactSheet } from './CharacterFactSheet';
import { LocationFactSheet } from './LocationFactSheet';
import { ItemFactSheet } from './ItemFactSheet';
import { FactionFactSheet } from './FactionFactSheet';
import { EventFactSheet } from './EventFactSheet';
import { ConceptFactSheet } from './ConceptFactSheet';
import { NPCFactSheet } from './NPCFactSheet';
import { SceneFactSheet } from './SceneFactSheet';
import { BlueprintCardsPanel } from './BlueprintCardsPanel';
import { CreateCardDialog } from './MetaCardEditor';

// Map entity kinds to their fact sheet components
const factSheetComponents: Partial<Record<EntityKind, React.ComponentType<{ entity: ParsedEntity; onUpdate: (attributes: EntityAttributes) => void }>>> = {
  CHARACTER: CharacterFactSheet,
  LOCATION: LocationFactSheet,
  ITEM: ItemFactSheet,
  FACTION: FactionFactSheet,
  EVENT: EventFactSheet,
  CONCEPT: ConceptFactSheet,
  NPC: NPCFactSheet,
  SCENE: SceneFactSheet,
};

type PanelMode = 'standard' | 'blueprint';

const PANEL_MODE_STORAGE_KEY = 'entities-panel:mode';

function getPanelMode(): PanelMode {
  const stored = localStorage.getItem(PANEL_MODE_STORAGE_KEY);
  return stored === 'blueprint' ? 'blueprint' : 'standard';
}

function setPanelMode(mode: PanelMode) {
  localStorage.setItem(PANEL_MODE_STORAGE_KEY, mode);
}

interface FactSheetContainerProps {
  externalEntities?: ParsedEntity[];
  onEntityUpdate?: (entity: ParsedEntity, attributes: EntityAttributes) => void;
}

export function FactSheetContainer({ externalEntities, onEntityUpdate }: FactSheetContainerProps = {}) {
  const { selectedNote, updateNoteContent } = useJotaiNotes();
  const {
    selectedEntity,
    setSelectedEntity,
    entitiesInCurrentNote,
    setEntitiesInCurrentNote
  } = useEntitySelection();

  // Panel Mode State
  const [panelMode, setPanelModeState] = useState<PanelMode>(getPanelMode());

  // Extraction Model State (Synced with global settings by default)
  const [extModel, setExtModel] = useState<ModelId>(() => {
    return SettingsManager.getLLMSettings().extractorModel;
  });

  // Sync with global settings changes
  useEffect(() => {
    const settings = SettingsManager.load();
    setExtModel(settings.llm.extractorModel);
  }, [SettingsManager]);

  const handlePanelModeChange = useCallback((mode: PanelMode) => {
    setPanelModeState(mode);
    setPanelMode(mode);
  }, []);

  const handleModelChange = useCallback((modelId: string) => {
    const id = modelId as ModelId;
    setExtModel(id);
    // Optionally update global settings if user changes it here
    SettingsManager.updateLLMSettings({ extractorModel: id });
  }, []);

  // Compute entities (either from selected note or external prop or SmartGraphRegistry)
  const allEntities = useMemo(() => {
    if (externalEntities) {
      return externalEntities;
    }

    const entities: ParsedEntity[] = [];
    const seen = new Set<string>();

    // Check if note itself is an entity
    if (selectedNote?.isEntity && selectedNote.entityKind && selectedNote.entityLabel) {
      const key = `${selectedNote.entityKind}|${selectedNote.entityLabel}`;
      seen.add(key);
      entities.push({
        kind: selectedNote.entityKind as EntityKind,
        subtype: selectedNote.entitySubtype,
        label: selectedNote.entityLabel,
        noteId: selectedNote.id,
        attributes: {}, // TODO: Fetch from content if stored there
      });
    }

    // Get entities from SmartGraphRegistry (SYNC - from local cache!)
    try {
      const registryEntities = smartGraphRegistry.getAllEntities();
      for (const entity of registryEntities) {
        const key = `${entity.kind}|${entity.label}`;
        if (!seen.has(key)) {
          seen.add(key);
          entities.push({
            kind: entity.kind as EntityKind,
            label: entity.label,
            attributes: {},
          });
        }
      }
    } catch (err) {
      // Registry not ready yet - just use note entities
      console.log('[FactSheetContainer] SmartGraphRegistry not ready');
    }

    return entities;
  }, [selectedNote?.content, selectedNote?.isEntity, selectedNote?.entityKind, selectedNote?.entityLabel, selectedNote?.entitySubtype, selectedNote?.id, externalEntities]);

  // Update context when entities change
  useEffect(() => {
    // Only update if the list is different to avoid loops
    // Simple length check for now, could be more robust
    if (entitiesInCurrentNote.length !== allEntities.length ||
      !entitiesInCurrentNote.every((e, i) => e.label === allEntities[i].label)) {
      setEntitiesInCurrentNote(allEntities);
    }

    // Auto-select first entity if none selected or current selection not in list
    // BUT only if we have entities and no current selection (or invalid selection)
    if (allEntities.length > 0) {
      const currentStillValid = selectedEntity && allEntities.some(
        e => e.kind === selectedEntity.kind && e.label === selectedEntity.label
      );
      if (!currentStillValid) {
        // If externalEntities are provided (Calendar Mode), we might NOT want to auto-select 
        // to avoid jumping focus around unless the user explicitly picks one.
        // However, existing behavior is to auto-select. Let's keep consistency for now.
        setSelectedEntity(allEntities[0]);
      }
    } else {
      if (selectedEntity) setSelectedEntity(null);
    }
  }, [allEntities, selectedEntity, setSelectedEntity, setEntitiesInCurrentNote, entitiesInCurrentNote]);

  // Use unified entity attributes hook for bi-directional sync
  const unifiedAttrs = useUnifiedEntityAttributes(selectedEntity);

  // Handle attribute updates - now uses unified hook for bi-directional sync
  const handleAttributeUpdate = useCallback(
    async (attributes: EntityAttributes) => {
      if (!selectedEntity) return;

      // If external handler provided (e.g., from CalendarEntitySidebar), use it
      if (onEntityUpdate) {
        onEntityUpdate(selectedEntity, attributes);
      }

      // Always update via unified hook for bi-directional sync
      // This syncs to both SQLite entity_attributes AND legacy note content
      await unifiedAttrs.setFields(attributes);
    },
    [selectedEntity, onEntityUpdate, unifiedAttrs.setFields]
  );

  // Render Loading/Empty states
  // If externalEntities is undefined, we are in "Note Mode", so we check for selectedNote
  if (!externalEntities && !selectedNote) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-6 text-center">
        <FileQuestion className="h-12 w-12 text-muted-foreground/50 mb-4" />
        <p className="text-sm text-muted-foreground">
          Select a note to view entity details
        </p>
      </div>
    );
  }

  // No entities found
  if (allEntities.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-6 text-center">
        <Sparkles className="h-12 w-12 text-muted-foreground/50 mb-4" />
        <p className="text-sm text-muted-foreground mb-2">
          No entities found
        </p>
        {!externalEntities && (
          <p className="text-xs text-muted-foreground/70 max-w-[280px]">
            Add entities using syntax like <code className="bg-muted px-1 rounded">[CHARACTER|Name]</code> or create an entity note
          </p>
        )}
      </div>
    );
  }

  // Get the correct fact sheet component
  const FactSheetComponent = selectedEntity
    ? factSheetComponents[selectedEntity.kind]
    : null;

  return (
    <div className="flex flex-col h-full">
      {/* Extraction Model Selector */}
      <div className="p-2 border-b border-border bg-muted/20 flex items-center gap-2">
        <BrainCircuit className="h-4 w-4 text-muted-foreground shrink-0" />
        <Select value={extModel} onValueChange={handleModelChange}>
          <SelectTrigger className="h-7 text-xs w-full bg-background">
            <SelectValue placeholder="Select Extraction Model" />
          </SelectTrigger>
          <SelectContent>
            <div className="px-2 py-1.5 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
              Gemini
            </div>
            {ModelRegistry.getModelsByProvider('gemini').map(model => (
              <SelectItem key={model.id} value={model.id} className="text-xs">
                {model.name} {model.costPer1kTokens === 0 && '(FREE)'}
              </SelectItem>
            ))}
            <div className="px-2 py-1.5 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider mt-1">
              OpenRouter
            </div>
            {ModelRegistry.getModelsByProvider('openrouter').map(model => (
              <SelectItem key={model.id} value={model.id} className="text-xs">
                {model.name} {model.costPer1kTokens === 0 && '(FREE)'}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Panel Mode Toggle */}
      <div className="p-2 border-b border-border bg-muted/10 flex items-center gap-2 justify-center">
        <div className="inline-flex rounded-md border border-border bg-background p-0.5">
          <Button
            variant={panelMode === 'standard' ? 'default' : 'ghost'}
            size="sm"
            className="h-7 px-3 text-xs"
            onClick={() => handlePanelModeChange('standard')}
          >
            <List className="h-3 w-3 mr-1.5" />
            Standard
          </Button>
          <Button
            variant={panelMode === 'blueprint' ? 'default' : 'ghost'}
            size="sm"
            className="h-7 px-3 text-xs"
            onClick={() => handlePanelModeChange('blueprint')}
          >
            <LayoutGrid className="h-3 w-3 mr-1.5" />
            Blueprint
          </Button>
        </div>
      </div>

      {/* Entity selector (shown when multiple entities) */}
      {allEntities.length > 1 && (
        <div className="p-3 border-b border-border">
          <Select
            value={selectedEntity ? `${selectedEntity.kind}|${selectedEntity.label}` : ''}
            onValueChange={(value) => {
              const [kind, ...labelParts] = value.split('|');
              const label = labelParts.join('|');
              const entity = allEntities.find(e => e.kind === kind && e.label === label);
              if (entity) setSelectedEntity(entity);
            }}
          >
            <SelectTrigger className="w-full bg-background">
              <SelectValue placeholder="Select entity" />
            </SelectTrigger>
            <SelectContent className="bg-popover border border-border">
              {allEntities.map((entity) => (
                <SelectItem
                  key={`${entity.kind}|${entity.label}`}
                  value={`${entity.kind}|${entity.label}`}
                >
                  <span className="flex items-center gap-2">
                    <span className="text-xs text-muted-foreground font-mono">
                      {entity.kind}
                    </span>
                    <span>{entity.label}</span>
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )}

      {/* Fact sheet content */}
      <div className="flex-1 overflow-auto custom-scrollbar">
        <div className="pb-20">
          {selectedEntity && panelMode === 'standard' && FactSheetComponent && (
            <FactSheetComponent
              entity={selectedEntity}
              onUpdate={handleAttributeUpdate}
            />
          )}
          {selectedEntity && panelMode === 'blueprint' && (
            <BlueprintCardsPanel
              entity={selectedEntity}
              onUpdate={handleAttributeUpdate}
            />
          )}
          {/* Fallback if no entity selected but we have them (shouldn't happen with auto-select) */}
          {!selectedEntity && allEntities.length > 0 && (
            <div className="p-6 text-center text-muted-foreground text-sm">
              Select an entity to view details
            </div>
          )}
        </div>
      </div>

      {/* Add Card Footer */}
      {selectedEntity && (
        <div className="sticky bottom-0 p-3 border-t border-border/50 bg-background/95 backdrop-blur-sm">
          <CreateCardDialog
            onCreateCard={async (data) => {
              await unifiedAttrs.createCard(data.name, `gradient:${data.gradientId}`, data.iconId);
            }}
            trigger={
              <Button variant="outline" className="w-full gap-2">
                <Plus className="h-4 w-4" />
                Add Custom Card
              </Button>
            }
          />
        </div>
      )}
    </div>
  );
}

