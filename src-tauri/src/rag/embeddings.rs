// src-tauri/src/rag/embeddings.rs
//! Native embedding service using fastembed
//!
//! Supports:
//! - BGE-small-en-v1.5 (384d) - fast, good quality (default)
//! - BGE-base-en-v1.5 (768d) - higher quality, slower

use fastembed::{TextEmbedding, InitOptions, EmbeddingModel as FastEmbedModel};
use parking_lot::RwLock;

/// Supported embedding models
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddingModel {
    BGESmallEN,    // BAAI/bge-small-en-v1.5 - 384 dimensions
    BGEBaseEN,     // BAAI/bge-base-en-v1.5 - 768 dimensions
}

impl EmbeddingModel {
    pub fn dimensions(&self) -> usize {
        match self {
            EmbeddingModel::BGESmallEN => 384,
            EmbeddingModel::BGEBaseEN => 768,
        }
    }

    pub fn model_id(&self) -> &'static str {
        match self {
            EmbeddingModel::BGESmallEN => "BAAI/bge-small-en-v1.5",
            EmbeddingModel::BGEBaseEN => "BAAI/bge-base-en-v1.5",
        }
    }

    /// Convert to fastembed's EmbeddingModel enum
    pub fn to_fastembed(&self) -> FastEmbedModel {
        match self {
            EmbeddingModel::BGESmallEN => FastEmbedModel::BGESmallENV15,
            EmbeddingModel::BGEBaseEN => FastEmbedModel::BGEBaseENV15,
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "bge-small-tauri" | "bge-small" | "BAAI/bge-small-en-v1.5" | "BGESmallENV15" => {
                Some(EmbeddingModel::BGESmallEN)
            }
            "bge-base-tauri" | "bge-base" | "BAAI/bge-base-en-v1.5" | "BGEBaseENV15" |
            "modernbert-tauri" | "modernbert" => {
                // Map modernbert to BGE-base as a similar-quality alternative
                Some(EmbeddingModel::BGEBaseEN)
            }
            _ => None,
        }
    }
}

/// Embedding service - wraps fastembed for text embedding
pub struct EmbeddingService {
    model_type: EmbeddingModel,
    embedder: Option<TextEmbedding>,
    initialized: bool,
}

impl EmbeddingService {
    /// Create a new embedding service (not yet initialized)
    pub fn new(model: EmbeddingModel) -> Self {
        Self {
            model_type: model,
            embedder: None,
            initialized: false,
        }
    }

    /// Initialize the embedding model
    /// Downloads model weights on first run (cached in ~/.cache/huggingface)
    pub fn init(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        println!(
            "[EmbeddingService] Initializing {} ({}d)...",
            self.model_type.model_id(),
            self.model_type.dimensions()
        );

        // Create fastembed TextEmbedding with the appropriate model
        let options = InitOptions::new(self.model_type.to_fastembed())
            .with_show_download_progress(true);

        let embedder = TextEmbedding::try_new(options)
            .map_err(|e| format!("Failed to load embedding model: {}", e))?;

        self.embedder = Some(embedder);
        self.initialized = true;

        println!(
            "[EmbeddingService] ✓ Model loaded: {}",
            self.model_type.model_id()
        );

        Ok(())
    }

    /// Check if the service is ready
    pub fn is_ready(&self) -> bool {
        self.initialized && self.embedder.is_some()
    }

    /// Get model dimensions
    pub fn dimensions(&self) -> usize {
        self.model_type.dimensions()
    }

    /// Get model ID
    pub fn model_id(&self) -> &str {
        self.model_type.model_id()
    }

    /// Embed a single text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let embeddings = self.embed_batch(&[text.to_string()])?;
        embeddings.into_iter().next().ok_or_else(|| "No embedding returned".to_string())
    }

    /// Embed multiple texts (batch)
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let embedder = self.embedder.as_ref()
            .ok_or("Embedding service not initialized")?;

        if texts.is_empty() {
            return Ok(vec![]);
        }

        // fastembed expects Vec<&str>
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        // Generate embeddings
        let embeddings = embedder
            .embed(text_refs, None)
            .map_err(|e| format!("Embedding failed: {}", e))?;

        Ok(embeddings)
    }
}

/// Global embedding service singleton
/// Initialized lazily on first use
static EMBEDDING_SERVICE: once_cell::sync::Lazy<RwLock<Option<EmbeddingService>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(None));

/// Initialize the global embedding service
pub fn init_embedder(model_id: &str) -> Result<(usize, String), String> {
    let model = EmbeddingModel::from_id(model_id)
        .ok_or_else(|| format!("Unknown model: {}. Supported: bge-small-tauri, bge-base-tauri", model_id))?;

    let mut service = EmbeddingService::new(model.clone());
    service.init()?;

    let dimensions = service.dimensions();
    let model_id_out = service.model_id().to_string();

    *EMBEDDING_SERVICE.write() = Some(service);

    Ok((dimensions, model_id_out))
}

/// Embed texts using the global service
pub fn embed_texts(texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
    let guard = EMBEDDING_SERVICE.read();
    let service = guard
        .as_ref()
        .ok_or("Embedding service not initialized. Call init_embedder first.")?;

    service.embed_batch(texts)
}

/// Check if embedder is ready
pub fn is_embedder_ready() -> bool {
    EMBEDDING_SERVICE
        .read()
        .as_ref()
        .map(|s| s.is_ready())
        .unwrap_or(false)
}

/// Get current embedder dimensions
pub fn get_embedder_dimensions() -> Option<usize> {
    EMBEDDING_SERVICE
        .read()
        .as_ref()
        .map(|s| s.dimensions())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_model_dimensions() {
        assert_eq!(EmbeddingModel::BGESmallEN.dimensions(), 384);
        assert_eq!(EmbeddingModel::BGEBaseEN.dimensions(), 768);
    }

    #[test]
    fn test_model_from_id() {
        assert_eq!(
            EmbeddingModel::from_id("bge-small-tauri"),
            Some(EmbeddingModel::BGESmallEN)
        );
        assert_eq!(
            EmbeddingModel::from_id("bge-base-tauri"),
            Some(EmbeddingModel::BGEBaseEN)
        );
        assert_eq!(EmbeddingModel::from_id("unknown"), None);
    }

    #[test]
    fn test_fastembed_model_mapping() {
        assert!(matches!(
            EmbeddingModel::BGESmallEN.to_fastembed(),
            FastEmbedModel::BGESmallENV15
        ));
    }
}
