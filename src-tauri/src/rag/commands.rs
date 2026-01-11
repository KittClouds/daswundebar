// src-tauri/src/rag/commands.rs
//! Tauri IPC commands for RAG pipeline
//!
//! Exposes embedding, indexing, and search to TypeScript frontend.

use serde::{Deserialize, Serialize};
use tauri::command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::embeddings::{embed_texts, init_embedder, is_embedder_ready, get_embedder_dimensions};
use super::chunker::{RagChunker, ChunkConfig};
use super::schema;
use crate::graph::commands::GRAPH_REGISTRY;

// =============================================================================
// Request/Response Types
// =============================================================================

#[taurpc::ipc_type]
pub struct InitEmbedderResponse {
    pub dimensions: usize,
    pub model_id: String,
}

#[taurpc::ipc_type]
pub struct EmbedResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub dimensions: usize,
    pub model_id: String,
}

#[taurpc::ipc_type]
pub struct NoteInput {
    pub id: String,
    pub title: String,
    pub content: String,
}

#[taurpc::ipc_type]
pub struct ChunkOutput {
    pub chunk_id: String,
    pub note_id: String,
    pub chunk_index: usize,
    pub text: String,
    pub start: usize,
    pub end: usize,
}

#[taurpc::ipc_type]
pub struct IndexResult {
    pub chunks_created: usize,
    pub note_id: String,
}

#[taurpc::ipc_type]
pub struct SearchResult {
    pub chunk_id: String,
    pub note_id: String,
    pub text: String,
    pub note_title: String,
    pub score: f32,
    pub start: usize,
    pub end: usize,
}

// =============================================================================
// Helper Functions
// =============================================================================

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

/// Initialize RAG schema in CozoDB (call once on startup)
fn ensure_rag_schema(dimensions: usize) -> Result<(), String> {
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    if dimensions == 384 {
        schema::init_rag_schema_384(registry.db())?;
    } else {
        schema::init_rag_schema_768(registry.db())?;
    }
    
    Ok(())
}

// =============================================================================
// Commands
// =============================================================================

/// Initialize the embedding model
/// 
/// Called when user selects a Tauri-native embedding model.
/// Downloads and caches model weights on first run.
#[command]
pub async fn rag_init_embedder(model_id: String) -> Result<InitEmbedderResponse, String> {
    let (dimensions, model_id_out) = init_embedder(&model_id)?;
    
    // Initialize CozoDB schema for this dimension
    ensure_rag_schema(dimensions)?;
    
    Ok(InitEmbedderResponse {
        dimensions,
        model_id: model_id_out,
    })
}

/// Embed texts using the initialized model
/// 
/// Returns embeddings for each input text.
#[command]
pub async fn rag_embed(texts: Vec<String>) -> Result<EmbedResponse, String> {
    if !is_embedder_ready() {
        return Err("Embedder not initialized. Call rag_init_embedder first.".to_string());
    }
    
    let embeddings = embed_texts(&texts)?;
    let dimensions = embeddings.first().map(|e| e.len()).unwrap_or(0);
    
    Ok(EmbedResponse {
        embeddings,
        dimensions,
        model_id: "active".to_string(),
    })
}

/// Check if the embedder is ready
#[command]
pub fn rag_embedder_ready() -> bool {
    is_embedder_ready()
}

/// Chunk text for indexing
/// 
/// Returns chunks without embeddings (embedding done separately or on TS side)
#[command]
pub fn rag_chunk_text(
    note_id: String,
    text: String,
    max_chunk_size: Option<usize>,
) -> Vec<ChunkOutput> {
    let chunker = if let Some(size) = max_chunk_size {
        RagChunker::with_config(ChunkConfig {
            max_size: size,
            overlap: size / 8,
            min_size: 50,
        })
    } else {
        RagChunker::new()
    };
    
    chunker.chunk(&text)
        .into_iter()
        .map(|c| ChunkOutput {
            chunk_id: format!("{}:{}", note_id, c.index),
            note_id: note_id.clone(),
            chunk_index: c.index,
            text: c.text,
            start: c.start,
            end: c.end,
        })
        .collect()
}

/// Index a note (chunk + embed + store in CozoDB HNSW)
/// 
/// Full pipeline: chunks the note, embeds each chunk, stores in CozoDB with HNSW index
#[command]
pub async fn rag_index_note(note: NoteInput) -> Result<IndexResult, String> {
    if !is_embedder_ready() {
        return Err("Embedder not initialized. Call rag_init_embedder first.".to_string());
    }
    
    let dimensions = get_embedder_dimensions()
        .ok_or("Could not get embedder dimensions")?;
    
    // Chunk the content
    let chunker = RagChunker::new();
    let chunks = chunker.chunk(&note.content);
    
    if chunks.is_empty() {
        return Ok(IndexResult {
            chunks_created: 0,
            note_id: note.id,
        });
    }
    
    // Delete existing chunks for this note first
    {
        let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
        let _ = schema::delete_note_chunks(registry.db(), dimensions, &note.id);
    }
    
    // Embed all chunks
    let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
    let embeddings = embed_texts(&texts)?;
    
    // Store in CozoDB
    let created_at = now();
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    
    for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
        let chunk_id = format!("{}:{}", note.id, chunk.index);
        
        schema::insert_chunk(
            registry.db(),
            dimensions,
            &chunk_id,
            &note.id,
            chunk.index as i64,
            &chunk.text,
            embedding,
            &note.title,
            chunk.start as i64,
            chunk.end as i64,
            created_at,
        )?;
    }
    
    println!(
        "[RAG] Indexed {} chunks for note '{}' (id: {})",
        chunks.len(),
        note.title,
        note.id
    );
    
    Ok(IndexResult {
        chunks_created: chunks.len(),
        note_id: note.id,
    })
}

/// Search for similar chunks using CozoDB HNSW
/// 
/// Embeds the query and searches HNSW index
#[command]
pub async fn rag_search(query: String, k: usize) -> Result<Vec<SearchResult>, String> {
    if !is_embedder_ready() {
        return Err("Embedder not initialized. Call rag_init_embedder first.".to_string());
    }
    
    let dimensions = get_embedder_dimensions()
        .ok_or("Could not get embedder dimensions")?;
    
    // Embed the query
    let query_embeddings = embed_texts(&[query.clone()])?;
    let query_vec = query_embeddings.into_iter().next()
        .ok_or("Failed to embed query")?;
    
    // Search CozoDB HNSW
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    let ef = (k * 2).max(20); // ef should be >= k
    
    let results = schema::search_chunks(registry.db(), dimensions, &query_vec, k, ef)?;
    
    Ok(results.into_iter().map(|r| SearchResult {
        chunk_id: r.chunk_id,
        note_id: r.note_id,
        text: r.text,
        note_title: r.note_title,
        score: r.score,
        start: r.start,
        end: r.end,
    }).collect())
}

/// Get all chunks for a note
#[command]
pub async fn rag_get_chunks(note_id: String) -> Result<Vec<ChunkOutput>, String> {
    let dimensions = get_embedder_dimensions().unwrap_or(384);
    
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    let chunks = schema::get_note_chunks(registry.db(), dimensions, &note_id)?;
    
    Ok(chunks.into_iter().map(|c| ChunkOutput {
        chunk_id: c.chunk_id,
        note_id: c.note_id,
        chunk_index: c.chunk_index,
        text: c.text,
        start: c.start,
        end: c.end,
    }).collect())
}

/// Delete chunks for a note
#[command]
pub async fn rag_delete_note_chunks(note_id: String) -> Result<usize, String> {
    let dimensions = get_embedder_dimensions().unwrap_or(384);
    
    let registry = GRAPH_REGISTRY.lock().map_err(|e| e.to_string())?;
    schema::delete_note_chunks(registry.db(), dimensions, &note_id)
}
