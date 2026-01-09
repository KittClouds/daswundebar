import React, { createContext, useContext, useState, useCallback, useEffect } from 'react';
import type { CompiledBlueprint } from '../types';
import {
  getAllBlueprintMetas,
  createBlueprintMeta,
  getVersionsByBlueprintId,
  createVersion,
} from '../api/storage';
import { compileBlueprint } from '../services/compiler';

interface BlueprintHubContextType {
  activeBlueprint: CompiledBlueprint | null;
  compiledBlueprint: CompiledBlueprint | null;
  projectId: string;
  versionId: string | null;
  isLoading: boolean;
  error: string | null;
  isHubOpen: boolean;
  openHub: () => void;
  closeHub: () => void;
  toggleHub: () => void;
  refresh: () => Promise<void>;
  reloadActiveVersion: () => Promise<void>;
  setActiveBlueprint: (blueprint: CompiledBlueprint | null) => void;
}

const BlueprintHubContext = createContext<BlueprintHubContextType | undefined>(undefined);

export function BlueprintHubProvider({ children }: { children: React.ReactNode }) {
  const [activeBlueprint, setActiveBlueprint] = useState<CompiledBlueprint | null>(null);
  const [compiledBlueprint, setCompiledBlueprint] = useState<CompiledBlueprint | null>(null);
  const [projectId] = useState<string>('default');
  const [versionId, setVersionId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isHubOpen, setIsHubOpen] = useState(false);

  useEffect(() => {
    const initializeProject = async () => {
      setIsLoading(true);
      try {
        // First, try to find any existing blueprint (prefer system blueprints)
        const allMetas = await getAllBlueprintMetas();
        let meta = allMetas.find(m => m.is_system) ?? allMetas[0] ?? null;

        if (!meta) {
          // Create default blueprint using the storage API
          meta = await createBlueprintMeta({
            name: 'Default Knowledge Graph',
            description: 'The default blueprint for your knowledge graph',
            category: 'system',
            is_system: true,
            tags: ['default', 'system'],
          });
          console.log('[BlueprintHub] Created new default blueprint:', meta.blueprint_id);
        }

        // Use the actual blueprint ID from the meta
        const blueprintId = meta.blueprint_id;
        const versions = await getVersionsByBlueprintId(blueprintId);

        let activeVersion = versions.find(v => v.status === 'draft');

        if (!activeVersion) {
          activeVersion = await createVersion({
            blueprint_id: blueprintId,
            status: 'draft',
            change_summary: 'Initial draft version',
          });
          console.log('[BlueprintHub] Created initial version:', activeVersion.version_id);
        }

        setVersionId(activeVersion.version_id);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to initialize project');
        console.error('Blueprint Hub initialization error:', err);
      } finally {
        setIsLoading(false);
      }
    };

    initializeProject();
  }, []);

  const openHub = useCallback(() => {
    setIsHubOpen(true);
  }, []);

  const closeHub = useCallback(() => {
    setIsHubOpen(false);
  }, []);

  const toggleHub = useCallback(() => {
    setIsHubOpen(prev => !prev);
  }, []);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      console.log('Blueprint Hub: Refresh called');

      if (!versionId) {
        console.log('Blueprint Hub: No version ID available');
        setCompiledBlueprint(null);
        return;
      }

      // Compile the blueprint
      const compiled = await compileBlueprint(versionId);
      setCompiledBlueprint(compiled);

      // Also set as active blueprint for backward compatibility
      setActiveBlueprint(compiled);

      console.log('Blueprint Hub: Successfully compiled blueprint for version', versionId);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to refresh blueprint data');
      console.error('Blueprint Hub refresh error:', err);
      setCompiledBlueprint(null);
    } finally {
      setIsLoading(false);
    }
  }, [versionId]);

  const reloadActiveVersion = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      console.log('Blueprint Hub: Reloading active version');

      // Get versions for the project
      const versions = await getVersionsByBlueprintId(projectId);

      // Find active version (draft or latest published)
      let activeVersion = versions.find(v => v.status === 'draft');

      // If no draft, create one
      if (!activeVersion) {
        activeVersion = await createVersion({
          blueprint_id: projectId,
          status: 'draft',
          change_summary: 'New draft version',
        });
      }

      // Update the version ID, which will trigger a recompile
      setVersionId(activeVersion.version_id);

      console.log('Blueprint Hub: Switched to version', activeVersion.version_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to reload active version');
      console.error('Blueprint Hub reload version error:', err);
    } finally {
      setIsLoading(false);
    }
  }, [projectId]);

  // Auto-compile when versionId changes
  useEffect(() => {
    if (versionId) {
      refresh();
    }
  }, [versionId, refresh]);

  const value: BlueprintHubContextType = {
    activeBlueprint,
    compiledBlueprint,
    projectId,
    versionId,
    isLoading,
    error,
    isHubOpen,
    openHub,
    closeHub,
    toggleHub,
    refresh,
    reloadActiveVersion,
    setActiveBlueprint,
  };

  return (
    <BlueprintHubContext.Provider value={value}>
      {children}
    </BlueprintHubContext.Provider>
  );
}

export function useBlueprintHubContext() {
  const context = useContext(BlueprintHubContext);
  if (context === undefined) {
    throw new Error('useBlueprintHubContext must be used within a BlueprintHubProvider');
  }
  return context;
}
