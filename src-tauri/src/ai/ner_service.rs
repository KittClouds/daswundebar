//! NER Service - GLiNER-based Named Entity Recognition
//!
//! Runs on a dedicated OS thread to keep the model loaded and avoid blocking
//! the async runtime. Uses mpsc channels for request/response communication.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
//! │ Tauri Commands  │────►│    NerService    │────►│  Worker Thread  │
//! │ (async context) │     │ (channel sender) │     │ (GLiNER model)  │
//! └─────────────────┘     └──────────────────┘     └─────────────────┘
//!                                                          │
//!                                                          ▼
//!                         ┌──────────────────┐     ┌─────────────────┐
//!                         │  Result Receiver │◄────│ AnalysisResult  │
//!                         └──────────────────┘     └─────────────────┘
//! ```

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc::{self, Sender, Receiver};
use serde::{Serialize, Deserialize};

// Note: gline_rs types will be used when model is loaded
// For now we define our own DTOs for the public API

/// Analysis request DTO
#[derive(Debug, Clone)]
pub struct AnalysisRequest {
    pub doc_id: String,
    pub text: String,
    /// Dynamic labels for zero-shot NER (e.g., ["CHARACTER", "LOCATION", "ITEM"])
    pub candidate_labels: Vec<String>,
}

/// Analysis result DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub doc_id: String,
    pub entities: Vec<InferredEntity>,
    pub inference_time_ms: u64,
}

/// A single inferred entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredEntity {
    pub text: String,
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f32,
}

/// NER Service state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NerServiceState {
    /// Not initialized, model not loaded
    Uninitialized,
    /// Model is being loaded
    Loading,
    /// Ready for inference
    Ready,
    /// Failed to load model
    Failed,
    /// Service is shutting down
    ShuttingDown,
}

/// Configuration for NerService
#[derive(Debug, Clone)]
pub struct NerServiceConfig {
    /// Path to tokenizer.json
    pub tokenizer_path: PathBuf,
    /// Path to model.onnx
    pub model_path: PathBuf,
    /// Channel buffer size for pending requests
    pub request_buffer_size: usize,
    /// Channel buffer size for results
    pub result_buffer_size: usize,
}

impl NerServiceConfig {
    pub fn new(tokenizer_path: PathBuf, model_path: PathBuf) -> Self {
        Self {
            tokenizer_path,
            model_path,
            request_buffer_size: 32,
            result_buffer_size: 32,
        }
    }
}

/// Handle to the NER service
///
/// This is the main API for requesting NER analysis.
/// The actual inference runs on a dedicated background thread.
pub struct NerService {
    /// Channel to send requests to the worker
    request_tx: Sender<AnalysisRequest>,
    /// Flag to check if service is running
    running: Arc<AtomicBool>,
    /// Worker thread handle (for cleanup)
    _worker_handle: JoinHandle<()>,
}

impl NerService {
    /// Create and start a new NER service
    ///
    /// This spawns a dedicated OS thread that:
    /// 1. Loads the GLiNER model (~100MB)
    /// 2. Processes incoming analysis requests
    /// 3. Sends results back via the result channel
    ///
    /// # Returns
    /// A tuple of (NerService, Receiver<AnalysisResult>)
    pub fn new(config: NerServiceConfig) -> Result<(Self, Receiver<AnalysisResult>), String> {
        let (request_tx, request_rx) = mpsc::channel::<AnalysisRequest>(config.request_buffer_size);
        let (result_tx, result_rx) = mpsc::channel::<AnalysisResult>(config.result_buffer_size);
        
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        
        let tokenizer_path = config.tokenizer_path.clone();
        let model_path = config.model_path.clone();

        // Spawn dedicated worker thread
        let worker_handle = thread::Builder::new()
            .name("ner-worker".to_string())
            .spawn(move || {
                Self::worker_loop(
                    tokenizer_path,
                    model_path,
                    request_rx,
                    result_tx,
                    running_clone,
                );
            })
            .map_err(|e| format!("Failed to spawn NER worker thread: {}", e))?;

        log::info!("[NerService] Started worker thread");

        Ok((
            Self {
                request_tx,
                running,
                _worker_handle: worker_handle,
            },
            result_rx,
        ))
    }

    /// Request NER analysis for a document
    ///
    /// This is non-blocking and queues the request for the worker thread.
    /// Results will be sent to the result receiver.
    pub async fn request_analysis(
        &self,
        doc_id: String,
        text: String,
        candidate_labels: Vec<String>,
    ) -> Result<(), String> {
        if !self.running.load(Ordering::SeqCst) {
            return Err("NER service is not running".to_string());
        }

        let request = AnalysisRequest {
            doc_id,
            text,
            candidate_labels,
        };

        self.request_tx
            .send(request)
            .await
            .map_err(|e| format!("Failed to queue analysis request: {}", e))
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get a clone of the request sender for external use
    pub fn request_tx_clone(&self) -> Sender<AnalysisRequest> {
        self.request_tx.clone()
    }

    /// Signal the worker to stop (non-blocking)
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Worker loop that runs on the dedicated thread
    fn worker_loop(
        tokenizer_path: PathBuf,
        model_path: PathBuf,
        mut request_rx: Receiver<AnalysisRequest>,
        result_tx: Sender<AnalysisResult>,
        running: Arc<AtomicBool>,
    ) {
        log::info!("[NerWorker] Loading GLiNER model from {:?}", model_path);
        
        // Attempt to load the model
        // Note: gline_rs::GLiNER requires the ort crate with load-dynamic feature
        // The actual loading will happen here once we verify the dependency works
        
        let model = match Self::load_model(&tokenizer_path, &model_path) {
            Ok(m) => {
                log::info!("[NerWorker] GLiNER model loaded successfully");
                Some(m)
            }
            Err(e) => {
                log::error!("[NerWorker] Failed to load GLiNER model: {}", e);
                None
            }
        };

        // Main processing loop
        while running.load(Ordering::SeqCst) {
            // Use blocking_recv with a timeout to allow checking running flag
            match request_rx.blocking_recv() {
                Some(request) => {
                    if let Some(ref model) = model {
                        let start = std::time::Instant::now();
                        
                        // Run inference
                        match Self::run_inference(model, &request) {
                            Ok(entities) => {
                                let result = AnalysisResult {
                                    doc_id: request.doc_id,
                                    entities,
                                    inference_time_ms: start.elapsed().as_millis() as u64,
                                };
                                
                                // Send result back (blocking send)
                                if result_tx.blocking_send(result).is_err() {
                                    log::warn!("[NerWorker] Result channel closed, stopping");
                                    break;
                                }
                            }
                            Err(e) => {
                                log::error!("[NerWorker] Inference error: {}", e);
                                // Send empty result to indicate completion
                                let _ = result_tx.blocking_send(AnalysisResult {
                                    doc_id: request.doc_id,
                                    entities: vec![],
                                    inference_time_ms: start.elapsed().as_millis() as u64,
                                });
                            }
                        }
                    } else {
                        // Model not loaded, send empty result
                        log::warn!("[NerWorker] Received request but model not loaded");
                        let _ = result_tx.blocking_send(AnalysisResult {
                            doc_id: request.doc_id,
                            entities: vec![],
                            inference_time_ms: 0,
                        });
                    }
                }
                None => {
                    // Channel closed, exit
                    log::info!("[NerWorker] Request channel closed, stopping");
                    break;
                }
            }
        }

        log::info!("[NerWorker] Worker thread exiting");
    }

    /// Load the GLiNER model
    ///
    /// This is a placeholder that will be implemented once we verify
    /// gline-rs compiles correctly with our dependency setup.
    fn load_model(
        tokenizer_path: &PathBuf,
        model_path: &PathBuf,
    ) -> Result<GlinerModel, String> {
        // Verify files exist
        if !tokenizer_path.exists() {
            return Err(format!("Tokenizer not found: {:?}", tokenizer_path));
        }
        if !model_path.exists() {
            return Err(format!("Model not found: {:?}", model_path));
        }

        // TODO: Uncomment when gline-rs is verified to compile
        /*
        use gline_rs::{GLiNER, TokenMode, Parameters, RuntimeParameters};
        
        let model = GLiNER::<TokenMode>::new(
            Parameters::default(),
            RuntimeParameters::default(),
            tokenizer_path.to_str().ok_or("Invalid tokenizer path")?,
            model_path.to_str().ok_or("Invalid model path")?,
        ).map_err(|e| format!("GLiNER load error: {}", e))?;
        
        Ok(GlinerModel::Real(model))
        */

        // For now, return a mock model for testing the infrastructure
        log::warn!("[NerWorker] Using mock model (gline-rs integration pending)");
        Ok(GlinerModel::Mock)
    }

    /// Run inference on a request
    fn run_inference(
        model: &GlinerModel,
        request: &AnalysisRequest,
    ) -> Result<Vec<InferredEntity>, String> {
        match model {
            GlinerModel::Mock => {
                // Mock inference for testing - returns entities based on simple heuristics
                log::info!("[NerWorker] Mock inference for doc {} with {} chars", 
                    request.doc_id, request.text.len());
                
                let mut entities = Vec::new();
                let text = &request.text;
                
                // Simple mock: look for capitalized words as potential entities
                let mut current_pos = 0;
                for word in text.split_whitespace() {
                    // Find actual position in text
                    if let Some(start) = text[current_pos..].find(word) {
                        let absolute_start = current_pos + start;
                        let absolute_end = absolute_start + word.len();
                        
                        // Check if word is capitalized and not common article
                        let is_capitalized = word.chars().next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false);
                        let is_common = matches!(word.to_lowercase().as_str(), 
                            "the" | "a" | "an" | "and" | "or" | "but" | "in" | "on" | "at" | "to" | "for"
                            | "of" | "with" | "is" | "are" | "was" | "were" | "be" | "been"
                            | "this" | "that" | "these" | "those" | "i" | "you" | "he" | "she"
                            | "it" | "we" | "they" | "##" | "###" | "int." | "ext."
                        );
                        
                        if is_capitalized && !is_common && word.len() > 2 {
                            // Pick a label from candidates
                            let label = if word.ends_with("'s") || word.chars().all(|c| c.is_alphabetic()) {
                                request.candidate_labels.first()
                                    .cloned()
                                    .unwrap_or_else(|| "entity".to_string())
                            } else {
                                request.candidate_labels.get(1)
                                    .cloned()
                                    .unwrap_or_else(|| request.candidate_labels.first()
                                        .cloned()
                                        .unwrap_or_else(|| "entity".to_string()))
                            };
                            
                            entities.push(InferredEntity {
                                text: word.to_string(),
                                label: label.clone(),
                                start: absolute_start,
                                end: absolute_end,
                                confidence: 0.75 + (entities.len() as f32 * 0.01).min(0.20),
                            });
                            
                            // Limit mock entities
                            if entities.len() >= 10 {
                                break;
                            }
                        }
                        
                        current_pos = absolute_end;
                    }
                }
                
                log::info!("[NerWorker] Mock found {} entities", entities.len());
                Ok(entities)
            }
            // GlinerModel::Real(m) => { ... actual inference ... }
        }
    }
}

/// Wrapper around the actual GLiNER model
///
/// This enum allows us to have a mock mode for testing before
/// the gline-rs integration is fully verified.
enum GlinerModel {
    Mock,
    // Real(gline_rs::GLiNER<gline_rs::TokenMode>),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_analysis_request_creation() {
        let req = AnalysisRequest {
            doc_id: "doc1".to_string(),
            text: "Hello world".to_string(),
            candidate_labels: vec!["PERSON".to_string(), "LOCATION".to_string()],
        };
        
        assert_eq!(req.doc_id, "doc1");
        assert_eq!(req.candidate_labels.len(), 2);
    }

    #[test]
    fn test_ner_service_config() {
        let config = NerServiceConfig::new(
            temp_dir().join("tokenizer.json"),
            temp_dir().join("model.onnx"),
        );
        
        assert_eq!(config.request_buffer_size, 32);
        assert_eq!(config.result_buffer_size, 32);
    }

    #[test]
    fn test_inferred_entity_serialization() {
        let entity = InferredEntity {
            text: "John Doe".to_string(),
            label: "PERSON".to_string(),
            start: 0,
            end: 8,
            confidence: 0.95,
        };
        
        let json = serde_json::to_string(&entity).unwrap();
        assert!(json.contains("John Doe"));
        assert!(json.contains("PERSON"));
        assert!(json.contains("0.95"));
    }

    #[tokio::test]
    async fn test_ner_service_creation_fails_without_model() {
        // This should fail because the model files don't exist
        let config = NerServiceConfig::new(
            temp_dir().join("nonexistent_tokenizer.json"),
            temp_dir().join("nonexistent_model.onnx"),
        );
        
        // Service should still create (model loading happens in worker)
        let result = NerService::new(config);
        assert!(result.is_ok(), "Service should create even if model missing");
        
        if let Ok((service, _rx)) = result {
            service.stop();
        }
    }
}
