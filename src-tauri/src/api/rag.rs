//! RAG API - Embeddings and semantic search

use crate::rag::commands as cmd;

// ============================================================================
// API Trait
// ============================================================================

#[taurpc::procedures(path = "rag", export_to = "../src/bindings.ts")]
pub trait RagApi {
    async fn init_embedder(model_id: String) -> Result<String, String>;
    async fn embed(texts: Vec<String>) -> Result<String, String>;
    async fn embedder_ready() -> bool;
    async fn chunk_text(note_id: String, text: String, max_chunk_size: Option<usize>) -> Result<String, String>;
    async fn index_note(params: String) -> Result<String, String>;
    async fn search(query: String, k: usize) -> Result<String, String>;
    async fn get_chunks(note_id: String) -> Result<String, String>;
    async fn delete_note_chunks(note_id: String) -> Result<usize, String>;
}

// ============================================================================
// Implementation
// ============================================================================

#[derive(Clone, Default)]
pub struct RagApiImpl;

#[taurpc::resolvers]
impl RagApi for RagApiImpl {
    async fn init_embedder(self, model_id: String) -> Result<String, String> {
        let result = cmd::rag_init_embedder(model_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn embed(self, texts: Vec<String>) -> Result<String, String> {
        let result = cmd::rag_embed(texts).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn embedder_ready(self) -> bool {
        cmd::rag_embedder_ready()
    }
    
    async fn chunk_text(self, note_id: String, text: String, max_chunk_size: Option<usize>) -> Result<String, String> {
        let result = cmd::rag_chunk_text(note_id, text, max_chunk_size);
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn index_note(self, params: String) -> Result<String, String> {
        let input: cmd::NoteInput = serde_json::from_str(&params)
            .map_err(|e| format!("Failed to parse params: {}", e))?;
        let result = cmd::rag_index_note(input).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn search(self, query: String, k: usize) -> Result<String, String> {
        let result = cmd::rag_search(query, k).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_chunks(self, note_id: String) -> Result<String, String> {
        let result = cmd::rag_get_chunks(note_id).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn delete_note_chunks(self, note_id: String) -> Result<usize, String> {
        cmd::rag_delete_note_chunks(note_id).await
    }
}
