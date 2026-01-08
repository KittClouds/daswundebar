import { useEffect, useState } from "react";
import { Toaster } from "@/components/ui/toaster";
import { Toaster as Sonner } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Provider as JotaiProvider } from "jotai";
import { jotaiStore, initializeJotaiStore } from "@/lib/store";

import Index from "./pages/Index";
import FantasyCalendarPage from "./pages/FantasyCalendarPage";
import GraphExplorerPage from "./pages/GraphExplorerPage";
import { WikiPage } from "./features/wiki";
import NotFound from "./pages/NotFound";
import { initializeStorage, getBlueprintStore } from "@/lib/storage/index";
import { BlueprintHubProvider } from "@/features/blueprint-hub/context/BlueprintHubContext";
import { BlueprintHubPanel } from "@/features/blueprint-hub/components/BlueprintHubPanel";
import { NERProvider } from "@/contexts/NERContext";
import { EntityThemeProvider } from "@/contexts/EntityThemeContext";
import { initializeSQLiteAndHydrate } from "@/lib/db";
import { initCozoGraphSchema } from '@/lib/cozo/schema/init';
import { bindingEngine } from '@/lib/bindings';
import { EntitySelectionProvider } from '@/contexts/EntitySelectionContext';

const queryClient = new QueryClient();

const App = () => {
    const [storageReady, setStorageReady] = useState(false);
    const [jotaiReady, setJotaiReady] = useState(false);
    const [initStatus, setInitStatus] = useState("Initializing...");

    useEffect(() => {
        const initStorage = async () => {
            try {
                // 🚀 UNIFIED TAURI INITIALIZATION
                // TauriOrchestrator handles: backend connection, entity loading, scanner hydration
                setInitStatus("Initializing Tauri pipeline...");
                const { isTauri, tauriOrchestrator } = await import("@/lib/tauri");
                const { logPipelineStatus, getResolvedPipeline } = await import("@/lib/Scanner/pipeline-config");
                logPipelineStatus();

                const pipeline = getResolvedPipeline();
                if (pipeline === 'tauri' && isTauri()) {
                    await tauriOrchestrator.init();
                    console.log("Tauri orchestrator ready");
                } else {
                    // WASM fallback: Initialize just the highlighter
                    const { initializeHighlighter } = await import("@/lib/scanner");
                    await initializeHighlighter();
                    console.log("WASM highlighter ready");
                }

                // Initialize Unified Registry (CozoDB)
                setInitStatus("Initializing knowledge graph...");
                const { entityRegistry, relationshipRegistry } = await import("@/lib/cozo/graph/adapters");
                await entityRegistry.init();
                await relationshipRegistry.init();

                // Initialize Layer 2 Schemas (required for EntityStoreImpl / BlueprintHub)
                await initCozoGraphSchema();

                console.log("Unified Registry and Layer 2 Schemas initialized");

                // Initialize Legacy SQLite (if needed for other components)
                setInitStatus("Initializing legacy storage...");
                const { nodesLoaded, embeddingsLoaded } = await initializeSQLiteAndHydrate();
                console.log(`SQLite initialized: ${nodesLoaded} nodes, ${embeddingsLoaded} embeddings`);

                setInitStatus("Initializing storage service...");
                await initializeStorage();
                console.log("Storage service initialized");

                const blueprintStore = getBlueprintStore();
                await blueprintStore.initialize();
                console.log("Blueprint store initialized");

                // ✅ INITIALIZE JOTAI STORE
                setInitStatus("Initializing Jotai state...");
                await initializeJotaiStore();
                console.log("Jotai store initialized");
                setJotaiReady(true);

                // 🚀 PRE-LOAD ENTITIES (WASM mode only - Tauri orchestrator already handles this)
                if (pipeline === 'wasm') {
                    setInitStatus("Pre-loading entities for highlighting...");
                    try {
                        const { scannerFacade } = await import("@/lib/scanner");
                        const { highlighterBridge } = await import("@/lib/highlighter");
                        const allEntities = await entityRegistry.getAllEntities();

                        if (allEntities.length > 0) {
                            const entityDefs = allEntities.map(e => ({
                                id: e.id,
                                label: e.label,
                                kind: e.kind,
                                aliases: e.aliases || [],
                            }));

                            await scannerFacade.hydrateEntities(entityDefs);
                            highlighterBridge.hydrateEntities(entityDefs);
                            console.log(`Pre-loaded ${allEntities.length} entities for instant highlighting (WASM)`);
                        }
                    } catch (err) {
                        console.warn("Failed to pre-load entities:", err);
                    }
                }

                // Initialize Binding Engine
                setInitStatus("Initializing binding engine...");
                await bindingEngine.initialize();
                console.log("Binding engine initialized");

                setStorageReady(true);
            } catch (e) {
                console.error("Storage initialization failed:", e);
                setStorageReady(true);
            }
        };
        initStorage();
    }, []);

    // ✅ WAIT FOR BOTH STORAGE AND JOTAI
    if (!storageReady || !jotaiReady) {
        return (
            <div className="flex items-center justify-center h-screen">
                <div className="text-center">
                    <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mx-auto mb-4"></div>
                    <p className="text-muted-foreground">{initStatus}</p>
                </div>
            </div>
        );
    }

    return (
        <JotaiProvider store={jotaiStore}>
            <QueryClientProvider client={queryClient}>
                <TooltipProvider>
                    <NERProvider>
                        <EntityThemeProvider>
                            <BlueprintHubProvider>
                                <EntitySelectionProvider>
                                    <Toaster />
                                    <Sonner />
                                    <BlueprintHubPanel />
                                    <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
                                        <Routes>
                                            <Route path="/" element={<Index />} />
                                            <Route path="/calendar" element={<FantasyCalendarPage />} />
                                            <Route path="/graph" element={<GraphExplorerPage />} />
                                            <Route path="/wiki/*" element={<WikiPage />} />
                                            <Route path="*" element={<NotFound />} />
                                        </Routes>
                                    </BrowserRouter>
                                </EntitySelectionProvider>
                            </BlueprintHubProvider>
                        </EntityThemeProvider>
                    </NERProvider>
                </TooltipProvider>
            </QueryClientProvider>
        </JotaiProvider>
    );
};

export default App;
