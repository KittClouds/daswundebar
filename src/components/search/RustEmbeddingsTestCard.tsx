// src/components/search/RustEmbeddingsTestCard.tsx
//
// Self-contained Rust/WASM embeddings testing UI
// Has its own model loading, text analysis, and search - completely independent from TS pipeline

'use client';

import { useState, useCallback } from 'react';
import { Cpu, Zap, AlertCircle, CheckCircle2, Loader2, Search } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useJotaiNotes } from '@/hooks/useJotaiNotes';

type RustStatus = 'idle' | 'loading-model' | 'ready' | 'embedding' | 'searching' | 'error';

interface EmbedResult {
    dimensions: number;
    preview: number[];
    timeMs: number;
    textLength: number;
}

interface SearchResult {
    noteId: string;
    title: string;
    score: number;
}

// Cosine similarity between two vectors
function cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length) return 0;
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;
    for (let i = 0; i < a.length; i++) {
        dotProduct += a[i] * b[i];
        normA += a[i] * a[i];
        normB += b[i] * b[i];
    }
    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
}

export function RustEmbeddingsTestCard() {
    const { selectedNote, notes } = useJotaiNotes();

    const [status, setStatus] = useState<RustStatus>('idle');
    const [error, setError] = useState<string | null>(null);
    const [result, setResult] = useState<EmbedResult | null>(null);
    const [cortex, setCortex] = useState<any>(null);

    // Search state
    const [searchQuery, setSearchQuery] = useState('');
    const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
    const [searchTime, setSearchTime] = useState<number>(0);

    // Load the Rust WASM model
    const handleLoadModel = useCallback(async () => {
        setStatus('loading-model');
        setError(null);
        setResult(null);
        setSearchResults([]);

        try {
            // NOTE: WASM embeddings temporarily disabled during Tauri migration
            // Phase 4 will implement via Tauri commands
            setError('Embeddings temporarily disabled during Tauri migration');
            setStatus('error');
            return;

            /* Original WASM code - disabled during Tauri migration:
            const kittcore = await import(
                '@kittcore/wasm'
            );
            await kittcore.default();

            const newCortex = new kittcore.EmbedCortex();

            console.log('[RustTest] Fetching BGE-small model files...');

            const [onnxRes, tokenizerRes] = await Promise.all([
                fetch('https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx'),
                fetch('https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json'),
            ]);

            if (!onnxRes.ok || !tokenizerRes.ok) {
                throw new Error('Failed to download model files');
            }

            const onnxBytes = new Uint8Array(await onnxRes.arrayBuffer());
            const tokenizerJson = await tokenizerRes.text();

            console.log('[RustTest] Loading into WASM...');
            newCortex.loadModel(onnxBytes, tokenizerJson);

            setCortex(newCortex);
            setStatus('ready');
            console.log(`[RustTest] ✓ Model ready (${newCortex.getDimensions()}d)`);
            */

        } catch (err) {
            console.error('[RustTest] Load failed:', err);
            setError(err instanceof Error ? err.message : 'Unknown error');
            setStatus('error');
        }
    }, []);

    // Analyze current note text
    const handleAnalyzeText = useCallback(async () => {
        if (!cortex || !selectedNote) return;

        setStatus('embedding');
        setError(null);

        try {
            const text = selectedNote.content || '';
            if (!text.trim()) {
                setError('Note is empty');
                setStatus('ready');
                return;
            }

            const start = performance.now();
            const embedding = cortex.embedText(text) as number[];
            const elapsed = performance.now() - start;

            setResult({
                dimensions: embedding.length,
                preview: embedding.slice(0, 5),
                timeMs: Math.round(elapsed),
                textLength: text.length,
            });

            setStatus('ready');
            console.log(`[RustTest] Embedded ${text.length} chars → ${embedding.length}d in ${elapsed.toFixed(1)}ms`);

        } catch (err) {
            console.error('[RustTest] Embed failed:', err);
            setError(err instanceof Error ? err.message : 'Unknown error');
            setStatus('ready');
        }
    }, [cortex, selectedNote]);

    // Rust-powered semantic search
    const handleSearch = useCallback(async () => {
        if (!cortex || !searchQuery.trim()) return;

        setStatus('searching');
        setError(null);
        const start = performance.now();

        try {
            // Embed the query
            const queryEmbedding = cortex.embedText(searchQuery) as number[];

            // Search through all notes
            const results: SearchResult[] = [];

            for (const note of notes) {
                const text = note.content || '';
                if (!text.trim()) continue;

                try {
                    const noteEmbedding = cortex.embedText(text) as number[];
                    const score = cosineSimilarity(queryEmbedding, noteEmbedding);

                    results.push({
                        noteId: note.id,
                        title: note.title || 'Untitled',
                        score,
                    });
                } catch {
                    // Skip notes that fail to embed
                }
            }

            // Sort by score descending, take top 5
            results.sort((a, b) => b.score - a.score);
            const topResults = results.slice(0, 5);

            setSearchResults(topResults);
            setSearchTime(Math.round(performance.now() - start));
            setStatus('ready');

            console.log(`[RustTest] Search "${searchQuery}" → ${topResults.length} results in ${(performance.now() - start).toFixed(1)}ms`);

        } catch (err) {
            console.error('[RustTest] Search failed:', err);
            setError(err instanceof Error ? err.message : 'Unknown error');
            setStatus('ready');
        }
    }, [cortex, searchQuery, notes]);

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' && status === 'ready') {
            handleSearch();
        }
    };

    return (
        <Card className="bg-orange-950/30 border-orange-500/40">
            <CardContent className="p-3">
                {/* Header */}
                <div className="flex items-center gap-2 mb-3">
                    <Cpu className="h-4 w-4 text-orange-500" />
                    <span className="text-sm font-medium text-orange-400">🦀 Rust Embeddings Test</span>
                </div>

                {/* Status */}
                <div className="text-xs text-muted-foreground mb-2">
                    {status === 'idle' && <span>Model not loaded</span>}
                    {status === 'loading-model' && (
                        <span className="flex items-center gap-1">
                            <Loader2 className="w-3 h-3 animate-spin" />
                            Downloading from HuggingFace...
                        </span>
                    )}
                    {status === 'ready' && (
                        <span className="flex items-center gap-1 text-green-500">
                            <CheckCircle2 className="w-3 h-3" />
                            Ready ({cortex?.getDimensions()}d)
                        </span>
                    )}
                    {status === 'embedding' && (
                        <span className="flex items-center gap-1">
                            <Loader2 className="w-3 h-3 animate-spin" />
                            Embedding...
                        </span>
                    )}
                    {status === 'searching' && (
                        <span className="flex items-center gap-1">
                            <Loader2 className="w-3 h-3 animate-spin" />
                            Searching {notes.length} notes...
                        </span>
                    )}
                    {status === 'error' && (
                        <span className="flex items-center gap-1 text-red-500">
                            <AlertCircle className="w-3 h-3" />
                            {error}
                        </span>
                    )}
                </div>

                {/* Buttons */}
                <div className="flex gap-2 mb-3">
                    <Button
                        variant="outline"
                        size="sm"
                        className="flex-1 text-xs h-7 border-orange-500/40 hover:bg-orange-500/20"
                        onClick={handleLoadModel}
                        disabled={status === 'loading-model' || status === 'embedding' || status === 'searching'}
                    >
                        {status === 'loading-model' ? 'Loading...' : 'Load Model'}
                    </Button>

                    <Button
                        variant="outline"
                        size="sm"
                        className="flex-1 text-xs h-7 border-orange-500/40 hover:bg-orange-500/20"
                        onClick={handleAnalyzeText}
                        disabled={status !== 'ready' || !selectedNote}
                    >
                        <Zap className="w-3 h-3 mr-1" />
                        Analyze Text
                    </Button>
                </div>

                {/* Rust Search Input */}
                {status === 'ready' && (
                    <div className="relative mb-2">
                        <Search className="absolute left-2 top-1/2 -translate-y-1/2 w-3 h-3 text-orange-400" />
                        <Input
                            placeholder="🦀 Rust search..."
                            className="pl-7 h-7 text-xs bg-black/30 border-orange-500/30 focus:border-orange-500"
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            onKeyDown={handleKeyDown}
                        />
                    </div>
                )}

                {/* Analyze Result */}
                {result && (
                    <div className="mt-2 p-2 bg-black/30 rounded text-xs font-mono">
                        <div className="text-green-400">✓ {result.dimensions}d embedding</div>
                        <div className="text-muted-foreground">
                            {result.textLength} chars → {result.timeMs}ms
                        </div>
                        <div className="text-orange-300 truncate mt-1">
                            [{result.preview.map(v => v.toFixed(3)).join(', ')}...]
                        </div>
                    </div>
                )}

                {/* Search Results */}
                {searchResults.length > 0 && (
                    <div className="mt-2 space-y-1">
                        <div className="text-xs text-muted-foreground">
                            {searchResults.length} results • {searchTime}ms
                        </div>
                        {searchResults.map((r, i) => (
                            <div
                                key={r.noteId}
                                className="p-2 bg-black/30 rounded text-xs flex justify-between items-center"
                            >
                                <span className="truncate flex-1 text-orange-200">
                                    {i + 1}. {r.title}
                                </span>
                                <span className="text-green-400 font-mono ml-2">
                                    {(r.score * 100).toFixed(1)}%
                                </span>
                            </div>
                        ))}
                    </div>
                )}
            </CardContent>
        </Card>
    );
}
