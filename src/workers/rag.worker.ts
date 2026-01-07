/// <reference lib="webworker" />

// NOTE: RAG WASM temporarily disabled during Tauri migration
// Phase 4 will implement via Tauri commands
// import init, { RagPipeline } from '@/lib/wasm/kittcore/kittcore';

import { normalizeEmbedding, getEmbeddingMeta, validateDimension, truncateEmbedding } from '@/lib/rag/embedding-utils';

// Stub class for Tauri migration
class RagPipeline {
    loadModel(_onnx: Uint8Array, _tokenizer: string) { }
    indexNotes(_notes: any[]) { return 0; }
    insertChunk(_chunk: any) { }
    buildRaptorTree(_clusterSize: number) { return {}; }
    search(_query: string, _k: number) { return []; }
    searchHybrid(_query: string, _k: number, _weight: number) { return []; }
    searchRaptor(_embedding: Float32Array, _k: number, _mode: string, _n: number) { return []; }
    embed(_text: string) { return new Float32Array(); }
    getChunks() { return []; }
    getStats() { return { total_chunks: 0 }; }
    isModelLoaded() { return false; }
}

// Stub init function
async function init() {
    console.log('[RagWorker] Stub mode - WASM disabled during Tauri migration');
}

// Types for messages
type WorkerMessage =
    | { type: 'INIT' }
    | { type: 'LOAD_MODEL'; payload: { onnx: ArrayBuffer; tokenizer: string; dims?: number; truncate?: string } }
    | { type: 'SET_DIMENSIONS'; payload: { dims: number } } // NEW: For TypeScript embedding mode
    | { type: 'INDEX_NOTES'; payload: { notes: Array<{ id: string; title: string; content: string }> } }
    | { type: 'INSERT_VECTORS'; payload: { chunks: Array<{ id: string; note_id: string; note_title: string; chunk_index: number; text: string; embedding: Float32Array; start: number; end: number }> } } // NEW: Pre-computed embeddings
    | { type: 'BUILD_RAPTOR'; payload: { clusterSize: number } }
    | { type: 'SEARCH'; payload: { query: string; k: number } }
    | { type: 'SEARCH_WITH_VECTOR'; payload: { embedding: Float32Array; k: number } } // NEW: For TS-side query embedding
    | { type: 'SEARCH_HYBRID'; payload: { query: string; k: number; vectorWeight: number; lexicalWeight: number } }
    | { type: 'SEARCH_WITH_DIVERSITY'; payload: { query: string; k: number; lambda: number } } // NEW: MMR diversity search
    | { type: 'SEARCH_RAPTOR'; payload: { query: string; k: number; mode: string } }
    | { type: 'HYDRATE'; payload: { chunks: Array<any> } }
    | { type: 'GET_CHUNKS' }
    | { type: 'GET_STATUS' }; // NEW: Get pipeline status

type ResponseMessage =
    | { type: 'INIT_COMPLETE' }
    | { type: 'MODEL_LOADED' }
    | { type: 'DIMENSIONS_SET'; payload: { dims: number } } // NEW
    | { type: 'INDEX_COMPLETE'; payload: { notes: number; chunks: number } }
    | { type: 'RAPTOR_BUILT'; payload: { stats: any } }
    | { type: 'SEARCH_RESULTS'; payload: { results: any[] } }
    | { type: 'CHUNKS_RETRIEVED'; payload: { chunks: any[] } }
    | { type: 'STATUS'; payload: { dims: number; modelLoaded: boolean; externalMode: boolean; chunkCount: number } } // NEW
    | { type: 'ERROR'; payload: { message: string } };

// Worker state
let pipeline: RagPipeline | null = null;
let initialized = false;
let currentModelDim = 256; // Default to MDBR Leaf dimensions
let currentTruncateDim: number | null = null;
let useExternalEmbedding = false; // NEW: True when using TS-side embeddings

self.onmessage = async (e: MessageEvent<WorkerMessage>) => {
    const msg = e.data;
    console.log('[RagWorker] Received:', msg.type);

    try {
        switch (msg.type) {
            case 'INIT':
                if (!initialized) {
                    await init();
                    pipeline = new RagPipeline();
                    initialized = true;
                }
                self.postMessage({ type: 'INIT_COMPLETE' });
                break;

            case 'LOAD_MODEL':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const { onnx, tokenizer, dims, truncate } = msg.payload as any;

                pipeline.loadModel(new Uint8Array(onnx), tokenizer);

                // Update state - Rust model mode
                currentModelDim = dims || 384;
                currentTruncateDim = truncate && truncate !== 'full' ? Number(truncate) : null;
                useExternalEmbedding = false; // Using Rust embeddings

                console.log(`[RagWorker] Model loaded. Native Dim: ${currentModelDim}, Truncate: ${currentTruncateDim || 'None'}`);

                self.postMessage({ type: 'MODEL_LOADED' });
                break;

            // NEW: Set dimensions for TypeScript embedding mode (no Rust model)
            case 'SET_DIMENSIONS':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const newDims = msg.payload.dims;

                // Use type assertion since wasm-bindgen may not export this yet
                // The method exists in Rust but TypeScript types may be stale
                const pipelineAny = pipeline as any;
                if (typeof pipelineAny.setDimensions === 'function') {
                    pipelineAny.setDimensions(newDims);
                } else {
                    // Fallback: reinitialize pipeline with new dimensions
                    // Note: This loses any indexed data, but for fresh init it's fine
                    console.warn('[RagWorker] setDimensions not available, dimensions will be set on first insert');
                }
                currentModelDim = newDims;
                useExternalEmbedding = true; // Using TypeScript embeddings

                console.log(`[RagWorker] External embedding mode. Dims: ${newDims}`);
                self.postMessage({ type: 'DIMENSIONS_SET', payload: { dims: newDims } });
                break;

            case 'INDEX_NOTES':
                if (!pipeline) throw new Error('Pipeline not initialized');
                if (useExternalEmbedding) {
                    throw new Error('INDEX_NOTES requires Rust model. Use INSERT_VECTORS for external embeddings.');
                }
                const notes = msg.payload.notes;

                const totalChunks = pipeline.indexNotes(notes);

                self.postMessage({
                    type: 'INDEX_COMPLETE',
                    payload: { notes: notes.length, chunks: totalChunks }
                });
                break;

            // NEW: Insert pre-computed embeddings from TypeScript
            case 'INSERT_VECTORS':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const vectorChunks = msg.payload.chunks;
                let insertedCount = 0;
                let insertErrors = 0;

                for (const chunk of vectorChunks) {
                    try {
                        // Convert Float32Array to regular array for serde
                        const embeddingArray = Array.from(chunk.embedding);

                        // Validate dimensions
                        if (embeddingArray.length !== currentModelDim) {
                            console.warn(`[RagWorker] Dimension mismatch: got ${embeddingArray.length}, expected ${currentModelDim}`);
                            insertErrors++;
                            continue;
                        }

                        pipeline.insertChunk({
                            id: chunk.id,
                            note_id: chunk.note_id,
                            note_title: chunk.note_title,
                            chunk_index: chunk.chunk_index,
                            text: chunk.text,
                            embedding: embeddingArray,
                            start: chunk.start,
                            end: chunk.end,
                        });
                        insertedCount++;
                    } catch (e) {
                        console.warn('[RagWorker] INSERT_VECTORS chunk failed:', e);
                        insertErrors++;
                    }
                }

                console.log(`[RagWorker] INSERT_VECTORS: ${insertedCount} inserted, ${insertErrors} errors`);
                self.postMessage({
                    type: 'INDEX_COMPLETE',
                    payload: { notes: 0, chunks: insertedCount }
                });
                break;

            case 'BUILD_RAPTOR':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const stats = pipeline.buildRaptorTree(msg.payload.clusterSize);
                self.postMessage({ type: 'RAPTOR_BUILT', payload: { stats } });
                break;

            case 'SEARCH':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const results = pipeline.search(msg.payload.query, msg.payload.k);
                self.postMessage({ type: 'SEARCH_RESULTS', payload: { results } });
                break;

            case 'SEARCH_HYBRID':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const { query, k, vectorWeight } = msg.payload;
                const hResults = pipeline.searchHybrid(query, k, vectorWeight);
                self.postMessage({ type: 'SEARCH_RESULTS', payload: { results: hResults } });
                break;

            case 'SEARCH_RAPTOR':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const { query: rQuery, k: rK, mode } = msg.payload;
                const embedding = pipeline.embed(rQuery);
                const raptorResults = pipeline.searchRaptor(new Float32Array(embedding), rK, mode, 10);
                self.postMessage({ type: 'SEARCH_RESULTS', payload: { results: raptorResults } });
                break;

            // NEW: Diversity search using MMR reranking
            case 'SEARCH_WITH_DIVERSITY':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const { query: dQuery, k: dK, lambda } = msg.payload;
                const diverseResults = (pipeline as any).searchWithDiversity(dQuery, dK, lambda);
                self.postMessage({ type: 'SEARCH_RESULTS', payload: { results: diverseResults } });
                break;

            case 'HYDRATE':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const { chunks } = msg.payload;
                let hydratedCount = 0;
                let skippedCount = 0;

                // Filter chunks that match current dimensionality
                const targetDim = currentTruncateDim || currentModelDim; // Ideally match model

                for (const chunk of chunks) {
                    // Use centralized utility for robust format conversion
                    let emb = normalizeEmbedding(chunk.embedding);
                    const meta = getEmbeddingMeta(chunk.embedding);

                    // Validate dimension matches model
                    if (!validateDimension(emb, currentModelDim)) {
                        // Truncation support: if source is larger, we can slice
                        if (currentTruncateDim && emb.length > currentTruncateDim) {
                            emb = truncateEmbedding(emb, currentTruncateDim);
                        } else {
                            console.debug(`[RagWorker] Skipping chunk: dim ${emb.length} != expected ${currentModelDim}`);
                            skippedCount++;
                            continue;
                        }
                    } else if (currentTruncateDim && currentTruncateDim < emb.length) {
                        // Model dim matches but truncation requested
                        emb = truncateEmbedding(emb, currentTruncateDim);
                    }

                    try {
                        pipeline.insertChunk({
                            ...chunk,
                            embedding: emb
                        });
                        hydratedCount++;
                    } catch (e) {
                        console.warn('[RagWorker] insertChunk failed:', e);
                        skippedCount++;
                    }
                }
                console.log(`[RagWorker] Hydrated ${hydratedCount} chunks, skipped ${skippedCount}`);
                self.postMessage({ type: 'INDEX_COMPLETE', payload: { notes: 0, chunks: hydratedCount } });
                break;

            case 'GET_CHUNKS':
                // ... existing
                if (!pipeline) throw new Error('Pipeline not initialized');
                const allChunks = pipeline.getChunks();
                self.postMessage({ type: 'CHUNKS_RETRIEVED', payload: { chunks: allChunks } });
                break;

            // NEW: Search with pre-computed query embedding (TypeScript embedding mode)
            case 'SEARCH_WITH_VECTOR':
                if (!pipeline) throw new Error('Pipeline not initialized');
                const { embedding: queryEmb, k: searchK } = msg.payload;

                // Convert to array if needed
                const queryArray = Array.from(queryEmb);

                // Validate dimensions
                if (queryArray.length !== currentModelDim) {
                    throw new Error(`Query dimension mismatch: got ${queryArray.length}, expected ${currentModelDim}`);
                }

                // Use RAPTOR search with vector (most flexible)
                const vectorResults = pipeline.searchRaptor(new Float32Array(queryArray), searchK, 'collapsed_leaves', 10);
                self.postMessage({ type: 'SEARCH_RESULTS', payload: { results: vectorResults } });
                break;

            // NEW: Get pipeline status
            case 'GET_STATUS':
                if (!pipeline) {
                    self.postMessage({
                        type: 'STATUS',
                        payload: { dims: 0, modelLoaded: false, externalMode: false, chunkCount: 0 }
                    });
                } else {
                    const pipelineStats = pipeline.getStats();
                    self.postMessage({
                        type: 'STATUS',
                        payload: {
                            dims: currentModelDim,
                            modelLoaded: pipeline.isModelLoaded(),
                            externalMode: useExternalEmbedding,
                            chunkCount: pipelineStats?.total_chunks ?? 0,
                        }
                    });
                }
                break;
        }
    } catch (e) {
        console.error('[RagWorker] Error:', e);
        self.postMessage({ type: 'ERROR', payload: { message: e instanceof Error ? e.message : String(e) } });
    }
};
