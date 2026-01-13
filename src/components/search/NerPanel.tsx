/**
 * NER Panel - Sidebar panel for NER model management and analysis
 * 
 * Two-stage workflow:
 * 1. Download model (if not available)
 * 2. Analyze current note text
 */

import React, { useState, useEffect, useCallback } from 'react';
import {
    Download,
    Play,
    Loader2,
    CheckCircle2,
    XCircle,
    Brain,
    RefreshCw,
    Trash2,
    AlertTriangle,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useNER } from '@/contexts/NERContext';
import { useJotaiNotes } from '@/hooks/useJotaiNotes';
import {
    nerGetModelStatus,
    nerDownloadModel,
    nerRequestAnalysis,
    nerAddSuggestion,
    isTauriNer,
    type NerModelStatus,
    type NerSuggestion,
} from '@/lib/tauri/ner-bridge';
import { Switch } from '@/components/ui/switch';
import { useAtom } from 'jotai';
import { fstNerEnabledAtom } from '@/atoms/highlightingAtoms';

// Default labels for NER analysis
const DEFAULT_LABELS = [
    'person', 'character', 'location', 'organization',
    'item', 'concept', 'event', 'creature',
];

export function NerPanel() {
    // Model status
    const [modelStatus, setModelStatus] = useState<NerModelStatus | null>(null);
    const [isCheckingModel, setIsCheckingModel] = useState(false);
    const [isDownloading, setIsDownloading] = useState(false);
    const [downloadError, setDownloadError] = useState<string | null>(null);

    // Analysis status
    const [isAnalyzing, setIsAnalyzing] = useState(false);
    const [analysisError, setAnalysisError] = useState<string | null>(null);
    const [lastAnalyzedNoteId, setLastAnalyzedNoteId] = useState<string | null>(null);

    // Logs
    const [logs, setLogs] = useState<string[]>([]);

    // Context
    const { suggestions, refreshSuggestions, acceptSuggestion, rejectSuggestion } = useNER();
    const { state } = useJotaiNotes();

    // FST-NER toggle
    const [fstNerEnabled, setFstNerEnabled] = useAtom(fstNerEnabledAtom);

    // Helper to add log
    const addLog = useCallback((message: string) => {
        const timestamp = new Date().toLocaleTimeString();
        setLogs(prev => [...prev.slice(-50), `[${timestamp}] ${message}`]);
        console.log(`[NerPanel] ${message}`);
    }, []);

    // Check if we're in Tauri
    const inTauri = isTauriNer();

    // Check model status on mount
    useEffect(() => {
        if (inTauri) {
            checkModelStatus();
        }
    }, [inTauri]);

    // Check model status
    const checkModelStatus = async () => {
        setIsCheckingModel(true);
        addLog('Checking model status...');
        try {
            const status = await nerGetModelStatus();
            setModelStatus(status);
            addLog(`Model ${status.available ? 'ready' : 'not downloaded'}`);
        } catch (err) {
            addLog(`Error checking model: ${err}`);
        } finally {
            setIsCheckingModel(false);
        }
    };

    // Download model
    const handleDownloadModel = async () => {
        setIsDownloading(true);
        setDownloadError(null);
        addLog('Starting model download...');

        try {
            const response = await nerDownloadModel();
            if (response.started) {
                addLog('Download started in background');
                // Poll for completion
                const pollInterval = setInterval(async () => {
                    const status = await nerGetModelStatus();
                    setModelStatus(status);
                    if (status.available) {
                        clearInterval(pollInterval);
                        setIsDownloading(false);
                        addLog('Model download complete!');
                    }
                }, 2000);

                // Timeout after 5 minutes
                setTimeout(() => {
                    clearInterval(pollInterval);
                    if (isDownloading) {
                        setIsDownloading(false);
                        setDownloadError('Download timeout - check your connection');
                        addLog('Download timeout');
                    }
                }, 300000);
            } else {
                setDownloadError(response.message);
                addLog(`Download failed: ${response.message}`);
                setIsDownloading(false);
            }
        } catch (err) {
            const msg = err instanceof Error ? err.message : 'Unknown error';
            setDownloadError(msg);
            addLog(`Download error: ${msg}`);
            setIsDownloading(false);
        }
    };

    // Analyze current note
    const handleAnalyze = async () => {
        const selectedNote = state.notes.find(n => n.id === state.selectedNoteId);
        if (!selectedNote) {
            addLog('No note selected');
            setAnalysisError('Please select a note first');
            return;
        }

        setIsAnalyzing(true);
        setAnalysisError(null);
        addLog(`Analyzing note: ${selectedNote.title}`);

        try {
            const response = await nerRequestAnalysis(
                selectedNote.id,
                selectedNote.content || '',
                DEFAULT_LABELS
            );

            if (response.queued) {
                addLog(`Analysis queued for doc_id: ${response.doc_id}`);
                setLastAnalyzedNoteId(selectedNote.id);

                // Refresh suggestions after a delay (mock for now)
                setTimeout(async () => {
                    addLog('Refreshing suggestions...');
                    await refreshSuggestions(selectedNote.id);
                    addLog(`Found ${suggestions.length} suggestions`);
                }, 1000);
            } else {
                addLog('Analysis request failed');
                setAnalysisError('Failed to queue analysis');
            }
        } catch (err) {
            const msg = err instanceof Error ? err.message : 'Unknown error';
            setAnalysisError(msg);
            addLog(`Analysis error: ${msg}`);
        } finally {
            setIsAnalyzing(false);
        }
    };

    // Add test suggestion (for debugging)
    const handleAddTestSuggestion = async () => {
        const selectedNote = state.notes.find(n => n.id === state.selectedNoteId);
        if (!selectedNote) {
            addLog('No note selected for test');
            return;
        }

        addLog('Adding test suggestion...');
        const id = await nerAddSuggestion(
            'test-world',
            selectedNote.id,
            'Test Entity',
            'Test Entity',
            0,
            11,
            0.85
        );

        if (id) {
            addLog(`Test suggestion added: ${id}`);
            await refreshSuggestions(selectedNote.id);
        } else {
            addLog('Failed to add test suggestion');
        }
    };

    // Clear logs
    const handleClearLogs = () => {
        setLogs([]);
    };

    if (!inTauri) {
        return (
            <div className="p-4 text-center text-muted-foreground">
                <AlertTriangle className="h-8 w-8 mx-auto mb-2 text-amber-500" />
                <p>NER requires Tauri runtime</p>
                <p className="text-xs mt-1">Run with `npm tauri dev`</p>
            </div>
        );
    }

    return (
        <div className="flex flex-col h-full">
            {/* Header */}
            <div className="p-4 border-b border-border">
                <div className="flex items-center gap-2 mb-2">
                    <Brain className="h-5 w-5 text-purple-500" />
                    <span className="font-semibold">NER Analysis</span>
                </div>
                <p className="text-xs text-muted-foreground">
                    Extract entities using GLiNER AI model
                </p>
            </div>

            {/* FST Scanner Toggle */}
            <div className="p-4 border-b border-border">
                <div className="flex items-center justify-between">
                    <div className="flex-1">
                        <span className="text-sm font-medium">FST Scanner</span>
                        <p className="text-xs text-muted-foreground">
                            Instant entity detection (no AI model)
                        </p>
                    </div>
                    <Switch
                        checked={fstNerEnabled}
                        onCheckedChange={setFstNerEnabled}
                        className="data-[state=checked]:bg-purple-500"
                    />
                </div>
            </div>

            {/* Model Status Section */}
            <div className="p-4 border-b border-border space-y-3">
                <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">Model Status</span>
                    <Button
                        variant="ghost"
                        size="sm"
                        onClick={checkModelStatus}
                        disabled={isCheckingModel}
                    >
                        <RefreshCw className={`h-3 w-3 ${isCheckingModel ? 'animate-spin' : ''}`} />
                    </Button>
                </div>

                {modelStatus ? (
                    <div className="flex items-center gap-2 text-sm">
                        {modelStatus.available ? (
                            <>
                                <CheckCircle2 className="h-4 w-4 text-green-500" />
                                <span className="text-green-500">Model Ready</span>
                            </>
                        ) : (
                            <>
                                <XCircle className="h-4 w-4 text-amber-500" />
                                <span className="text-amber-500">Not Downloaded</span>
                            </>
                        )}
                    </div>
                ) : (
                    <div className="text-xs text-muted-foreground">
                        {isCheckingModel ? 'Checking...' : 'Click refresh to check'}
                    </div>
                )}

                {/* Download Button */}
                {modelStatus && !modelStatus.available && (
                    <Button
                        className="w-full gap-2"
                        onClick={handleDownloadModel}
                        disabled={isDownloading}
                    >
                        {isDownloading ? (
                            <>
                                <Loader2 className="h-4 w-4 animate-spin" />
                                Downloading...
                            </>
                        ) : (
                            <>
                                <Download className="h-4 w-4" />
                                Download Model (~100MB)
                            </>
                        )}
                    </Button>
                )}

                {downloadError && (
                    <p className="text-xs text-destructive">{downloadError}</p>
                )}
            </div>

            {/* Analysis Section */}
            <div className="p-4 border-b border-border space-y-3">
                <span className="text-sm font-medium">Analyze Note</span>

                <Button
                    className="w-full gap-2"
                    onClick={handleAnalyze}
                    disabled={isAnalyzing || !modelStatus?.available}
                    variant={modelStatus?.available ? "default" : "secondary"}
                >
                    {isAnalyzing ? (
                        <>
                            <Loader2 className="h-4 w-4 animate-spin" />
                            Analyzing...
                        </>
                    ) : (
                        <>
                            <Play className="h-4 w-4" />
                            {modelStatus?.available ? 'Run NER Analysis' : 'Download Model First'}
                        </>
                    )}
                </Button>

                {/* Test button for debugging */}
                <Button
                    variant="outline"
                    size="sm"
                    className="w-full gap-2 text-xs"
                    onClick={handleAddTestSuggestion}
                >
                    Add Test Suggestion
                </Button>

                {analysisError && (
                    <p className="text-xs text-destructive">{analysisError}</p>
                )}

                {lastAnalyzedNoteId && (
                    <p className="text-xs text-muted-foreground">
                        Last analyzed: {state.notes.find(n => n.id === lastAnalyzedNoteId)?.title || lastAnalyzedNoteId}
                    </p>
                )}
            </div>

            {/* Suggestions Section */}
            <div className="p-4 border-b border-border">
                <div className="flex items-center justify-between mb-2">
                    <span className="text-sm font-medium">Pending Suggestions</span>
                    <span className="text-xs bg-muted px-2 py-0.5 rounded">
                        {suggestions.length}
                    </span>
                </div>

                {suggestions.length > 0 ? (
                    <div className="space-y-2 max-h-40 overflow-y-auto">
                        {suggestions.map((s) => (
                            <SuggestionCard
                                key={s.id}
                                suggestion={s}
                                onAccept={acceptSuggestion}
                                onReject={rejectSuggestion}
                            />
                        ))}
                    </div>
                ) : (
                    <p className="text-xs text-muted-foreground">
                        No pending suggestions
                    </p>
                )}
            </div>

            {/* Logs Section */}
            <div className="flex-1 flex flex-col min-h-0">
                <div className="flex items-center justify-between p-2 border-b border-border">
                    <span className="text-xs font-medium text-muted-foreground">Logs</span>
                    <Button variant="ghost" size="sm" onClick={handleClearLogs}>
                        <Trash2 className="h-3 w-3" />
                    </Button>
                </div>
                <ScrollArea className="flex-1 p-2">
                    <div className="space-y-1 font-mono text-[10px] text-muted-foreground">
                        {logs.length === 0 ? (
                            <p className="text-center py-2">No logs yet</p>
                        ) : (
                            logs.map((log, i) => (
                                <div key={i} className="whitespace-pre-wrap break-all">
                                    {log}
                                </div>
                            ))
                        )}
                    </div>
                </ScrollArea>
            </div>
        </div>
    );
}

// Suggestion card component
interface SuggestionCardProps {
    suggestion: NerSuggestion;
    onAccept: (id: string) => Promise<boolean>;
    onReject: (id: string) => Promise<boolean>;
}

function SuggestionCard({ suggestion, onAccept, onReject }: SuggestionCardProps) {
    const [isProcessing, setIsProcessing] = useState(false);

    const handleAccept = async () => {
        setIsProcessing(true);
        await onAccept(suggestion.id);
        setIsProcessing(false);
    };

    const handleReject = async () => {
        setIsProcessing(true);
        await onReject(suggestion.id);
        setIsProcessing(false);
    };

    const confidencePercent = Math.round(suggestion.confidence * 100);

    return (
        <div className="flex items-center justify-between p-2 bg-muted/50 rounded text-xs">
            <div className="flex-1 min-w-0">
                <span className="font-medium truncate block">{suggestion.text}</span>
                <span className="text-muted-foreground">
                    {suggestion.entity_type} • {confidencePercent}%
                </span>
            </div>
            <div className="flex gap-1 shrink-0 ml-2">
                <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 w-6 p-0 text-green-500 hover:text-green-600"
                    onClick={handleAccept}
                    disabled={isProcessing}
                >
                    <CheckCircle2 className="h-3 w-3" />
                </Button>
                <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 w-6 p-0 text-red-500 hover:text-red-600"
                    onClick={handleReject}
                    disabled={isProcessing}
                >
                    <XCircle className="h-3 w-3" />
                </Button>
            </div>
        </div>
    );
}
