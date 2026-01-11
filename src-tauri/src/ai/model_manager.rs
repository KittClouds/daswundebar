//! Model Manager - Download and manage GLiNER model files
//!
//! Downloads models from HuggingFace on first use.
//! Models are stored in the user's data directory.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use futures_util::StreamExt;
use serde::{Serialize, Deserialize};

/// Model download/status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub available: bool,
    pub model_path: Option<String>,
    pub tokenizer_path: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Download progress callback data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<f32>,
}

/// Model file URLs for gliner_small-v2.1
const MODEL_REPO: &str = "onnx-community/gliner_small-v2.1";
const MODEL_FILE: &str = "onnx/model.onnx";
const TOKENIZER_FILE: &str = "tokenizer.json";

/// Model manager handles downloading and locating GLiNER models
pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    /// Create a new model manager
    ///
    /// Models are stored in: `{app_data_dir}/models/gliner_small-v2.1/`
    pub fn new(app_data_dir: &Path) -> Self {
        let models_dir = app_data_dir.join("models").join("gliner_small-v2.1");
        Self { models_dir }
    }

    /// Get the models directory path
    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }

    /// Check if models are already downloaded
    pub fn check_status(&self) -> ModelStatus {
        let model_path = self.models_dir.join("model.onnx");
        let tokenizer_path = self.models_dir.join("tokenizer.json");

        let model_exists = model_path.exists();
        let tokenizer_exists = tokenizer_path.exists();

        if model_exists && tokenizer_exists {
            let size = std::fs::metadata(&model_path)
                .ok()
                .map(|m| m.len());

            ModelStatus {
                available: true,
                model_path: Some(model_path.to_string_lossy().to_string()),
                tokenizer_path: Some(tokenizer_path.to_string_lossy().to_string()),
                size_bytes: size,
            }
        } else {
            ModelStatus {
                available: false,
                model_path: None,
                tokenizer_path: None,
                size_bytes: None,
            }
        }
    }

    /// Get paths if models are available
    pub fn get_paths(&self) -> Option<(PathBuf, PathBuf)> {
        let status = self.check_status();
        if status.available {
            Some((
                self.models_dir.join("model.onnx"),
                self.models_dir.join("tokenizer.json"),
            ))
        } else {
            None
        }
    }

    /// Download models from HuggingFace
    ///
    /// Returns a receiver for progress updates
    pub async fn download_models(
        &self,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<(), String> {
        // Ensure directory exists
        std::fs::create_dir_all(&self.models_dir)
            .map_err(|e| format!("Failed to create models directory: {}", e))?;

        // Download tokenizer.json
        self.download_file(
            &format!(
                "https://huggingface.co/{}/resolve/main/{}",
                MODEL_REPO, TOKENIZER_FILE
            ),
            &self.models_dir.join("tokenizer.json"),
            "tokenizer.json".to_string(),
            progress_tx.clone(),
        )
        .await?;

        // Download model.onnx
        self.download_file(
            &format!(
                "https://huggingface.co/{}/resolve/main/{}",
                MODEL_REPO, MODEL_FILE
            ),
            &self.models_dir.join("model.onnx"),
            "model.onnx".to_string(),
            progress_tx,
        )
        .await?;

        Ok(())
    }

    /// Download a single file with progress reporting
    async fn download_file(
        &self,
        url: &str,
        dest: &Path,
        file_name: String,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<(), String> {
        log::info!("[ModelManager] Downloading {} from {}", file_name, url);

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch {}: {}", file_name, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download {}: HTTP {}",
                file_name,
                response.status()
            ));
        }

        let total_size = response.content_length();

        // Create temp file for atomic write
        let temp_path = dest.with_extension("tmp");
        let mut file = tokio::fs::File::create(&temp_path)
            .await
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .map_err(|e| format!("Write error: {}", e))?;

            downloaded += chunk.len() as u64;

            // Send progress update
            let _ = progress_tx
                .send(DownloadProgress {
                    file_name: file_name.clone(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    percent: total_size.map(|t| (downloaded as f32 / t as f32) * 100.0),
                })
                .await;
        }

        // Rename temp file to final destination
        tokio::fs::rename(&temp_path, dest)
            .await
            .map_err(|e| format!("Failed to finalize download: {}", e))?;

        log::info!(
            "[ModelManager] Downloaded {} ({} bytes)",
            file_name,
            downloaded
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_model_manager_status_not_available() {
        let mgr = ModelManager::new(&temp_dir().join("nonexistent_test_dir"));
        let status = mgr.check_status();
        assert!(!status.available);
        assert!(status.model_path.is_none());
    }

    #[test]
    fn test_model_manager_paths() {
        let mgr = ModelManager::new(&temp_dir());
        let models_dir = mgr.models_dir();
        assert!(models_dir.ends_with("gliner_small-v2.1"));
    }
}
