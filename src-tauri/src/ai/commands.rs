//! NER Tauri Commands
//!
//! Exposes NER functionality to the frontend via Tauri IPC.

use std::sync::Arc;
use parking_lot::Mutex;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc;

use super::{
    NerService, NerServiceConfig, ModelManager, ModelStatus, DownloadProgress,
    SuggestionLayer, Suggestion, AcceptResult, AnalysisResult, InferredEntity,
};

// =============================================================================
// Global State
// =============================================================================

/// Global NER service instance (lazy initialized on first use)
static NER_SERVICE: Lazy<Mutex<Option<NerServiceHandle>>> = Lazy::new(|| {
    Mutex::new(None)
});

/// Global suggestion layer for pending NER suggestions
static SUGGESTION_LAYER: Lazy<SuggestionLayer> = Lazy::new(|| {
    SuggestionLayer::new()
});

/// Handle to the NER service and its result receiver
struct NerServiceHandle {
    service: NerService,
    // Note: Result receiver is stored separately for async processing
}

// =============================================================================
// Settings Types
// =============================================================================

/// NER settings
#[taurpc::ipc_type]
pub struct NerSettings {
    /// Whether NER is enabled
    pub enabled: bool,
    /// Auto-promote threshold (default: 0.90)
    pub auto_promote_threshold: f32,
    /// Suggest threshold (default: 0.60)
    pub suggest_threshold: f32,
}

impl Default for NerSettings {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in by default
            auto_promote_threshold: 0.90,
            suggest_threshold: 0.60,
        }
    }
}

// =============================================================================
// Response Types
// =============================================================================

/// Response for analysis request
#[taurpc::ipc_type]
pub struct AnalysisRequestResponse {
    pub queued: bool,
    pub doc_id: String,
}

/// Response for suggestions
#[taurpc::ipc_type]
pub struct SuggestionsResponse {
    pub suggestions: Vec<Suggestion>,
    pub count: usize,
}

/// Response for accept/reject
#[taurpc::ipc_type]
pub struct ActionResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
}

/// Response for model download
#[taurpc::ipc_type]
pub struct DownloadResponse {
    pub started: bool,
    pub message: String,
}

// =============================================================================
// Tauri Commands
// =============================================================================

/// Get NER model status
#[tauri::command]
pub fn ner_get_model_status() -> Result<ModelStatus, String> {
    // Get app data directory
    let data_dir = dirs::data_dir()
        .ok_or("Could not find data directory")?
        .join("varant");
    
    let manager = ModelManager::new(&data_dir);
    Ok(manager.check_status())
}

/// Download NER model from HuggingFace
#[tauri::command]
pub async fn ner_download_model() -> Result<DownloadResponse, String> {
    let data_dir = dirs::data_dir()
        .ok_or("Could not find data directory")?
        .join("varant");
    
    let manager = ModelManager::new(&data_dir);
    
    // Check if already downloaded
    let status = manager.check_status();
    if status.available {
        return Ok(DownloadResponse {
            started: false,
            message: "Model already downloaded".to_string(),
        });
    }
    
    // Create progress channel (for future progress events)
    let (tx, mut rx) = mpsc::channel::<DownloadProgress>(32);
    
    // Spawn download task
    tokio::spawn(async move {
        if let Err(e) = manager.download_models(tx).await {
            log::error!("[NER] Model download failed: {}", e);
        } else {
            log::info!("[NER] Model download completed");
        }
    });
    
    // Consume progress updates in background (could emit to frontend)
    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            log::info!(
                "[NER] Download progress: {} - {:.1}%",
                progress.file_name,
                progress.percent.unwrap_or(0.0)
            );
        }
    });
    
    Ok(DownloadResponse {
        started: true,
        message: "Model download started".to_string(),
    })
}

/// Request NER analysis for a document
#[tauri::command]
pub async fn ner_request_analysis(
    doc_id: String,
    text: String,
    candidate_labels: Vec<String>,
) -> Result<AnalysisRequestResponse, String> {
    log::info!("[NER Cmd] Analysis request for doc: {}", doc_id);
    
    // Check if service is initialized and running
    let is_running = {
        let service_lock = NER_SERVICE.lock();
        service_lock.as_ref().map(|h| h.service.is_running()).unwrap_or(false)
    };
    
    if !is_running {
        // Service not initialized - try to initialize it
        log::info!("[NER Cmd] Initializing NER service...");
        
        let data_dir = dirs::data_dir()
            .ok_or("Could not find data directory")?
            .join("varant");
        
        let manager = ModelManager::new(&data_dir);
        let paths = manager.get_paths()
            .ok_or("Model not downloaded. Please download the model first.")?;
        
        // Initialize service
        let config = NerServiceConfig::new(paths.1, paths.0);
        let (service, mut result_rx) = NerService::new(config)?;
        
        // Spawn result consumer task - routes results to suggestion layer
        tokio::spawn(async move {
            log::info!("[NER Cmd] Result consumer started");
            while let Some(result) = result_rx.recv().await {
                log::info!("[NER Cmd] Got {} entities for {}", 
                    result.entities.len(), result.doc_id);
                
                for entity in &result.entities {
                    if let Some(_id) = SUGGESTION_LAYER.add_suggestion(
                        "default",
                        &result.doc_id,
                        entity,
                    ) {
                        log::debug!("[NER Cmd] Added suggestion for: {}", entity.text);
                    } else {
                        log::debug!("[NER Cmd] Filtered out: {}", entity.text);
                    }
                }
            }
            log::info!("[NER Cmd] Result consumer ended");
        });
        
        // Store service
        let mut service_lock = NER_SERVICE.lock();
        *service_lock = Some(NerServiceHandle { service });
        log::info!("[NER Cmd] NER service ready");
    }
    
    // Queue the analysis request
    let doc_id_return = doc_id.clone();
    
    // Get sender clone outside of lock
    let tx = {
        let service_lock = NER_SERVICE.lock();
        service_lock.as_ref().map(|h| h.service.request_tx_clone())
    };
    
    if let Some(sender) = tx {
        use super::ner_service::AnalysisRequest;
        let request = AnalysisRequest {
            doc_id,
            text,
            candidate_labels,
        };
        
        sender.send(request).await
            .map_err(|e| format!("Failed to queue: {}", e))?;
        
        log::info!("[NER Cmd] Analysis queued");
    } else {
        return Err("NER service unavailable".to_string());
    }
    
    Ok(AnalysisRequestResponse {
        queued: true,
        doc_id: doc_id_return,
    })
}

/// Get pending NER suggestions for a note
#[tauri::command]
pub fn ner_get_suggestions(note_id: String) -> Result<SuggestionsResponse, String> {
    let suggestions = SUGGESTION_LAYER.get_suggestions(&note_id);
    let count = suggestions.len();
    
    Ok(SuggestionsResponse {
        suggestions,
        count,
    })
}

/// Get all pending NER suggestions across all notes
#[tauri::command]
pub fn ner_get_all_suggestions() -> Result<SuggestionsResponse, String> {
    let suggestions = SUGGESTION_LAYER.get_all_suggestions();
    let count = suggestions.len();
    
    Ok(SuggestionsResponse {
        suggestions,
        count,
    })
}

/// Accept a NER suggestion (promote to entity in CozoDB)
#[tauri::command]
pub fn ner_accept_suggestion(suggestion_id: String) -> Result<AcceptResult, String> {
    // Get the graph registry for CozoDB access
    let registry = crate::graph::commands::GRAPH_REGISTRY
        .lock()
        .map_err(|e| e.to_string())?;
    let db = registry.db();
    
    // Accept suggestion - this creates the entity in CozoDB
    SUGGESTION_LAYER.accept(db, &suggestion_id)
}

/// Reject a NER suggestion
#[tauri::command]
pub fn ner_reject_suggestion(suggestion_id: String) -> Result<ActionResponse, String> {
    // Get the graph registry for CozoDB access
    let registry = crate::graph::commands::GRAPH_REGISTRY
        .lock()
        .map_err(|e| e.to_string())?;
    let db = registry.db();
    
    match SUGGESTION_LAYER.reject(db, &suggestion_id) {
        Ok(()) => Ok(ActionResponse {
            success: true,
            message: "Suggestion rejected".to_string(),
            entity_id: None,
        }),
        Err(e) => Ok(ActionResponse {
            success: false,
            message: e,
            entity_id: None,
        }),
    }
}

/// Get NER settings
#[tauri::command]
pub fn ner_get_settings() -> Result<NerSettings, String> {
    // TODO: Load from persistent storage (CozoDB settings relation)
    // For now, return defaults
    Ok(NerSettings::default())
}

/// Update NER settings
#[tauri::command]
pub fn ner_update_settings(settings: NerSettings) -> Result<NerSettings, String> {
    // TODO: Persist to storage
    log::info!("[NER] Settings updated: enabled={}", settings.enabled);
    Ok(settings)
}

/// Get count of pending suggestions
#[tauri::command]
pub fn ner_pending_count() -> Result<usize, String> {
    Ok(SUGGESTION_LAYER.pending_count())
}

/// Clear all pending suggestions for a note
#[tauri::command]
pub fn ner_clear_note_suggestions(note_id: String) -> Result<ActionResponse, String> {
    SUGGESTION_LAYER.clear_note(&note_id);
    Ok(ActionResponse {
        success: true,
        message: format!("Cleared suggestions for note {}", note_id),
        entity_id: None,
    })
}

/// Add a suggestion (for testing/manual injection)
#[tauri::command]
pub fn ner_add_suggestion(
    world_id: String,
    note_id: String,
    text: String,
    label: String,
    start: usize,
    end: usize,
    confidence: f32,
) -> Result<Option<String>, String> {
    let entity = InferredEntity {
        text,
        label,
        start,
        end,
        confidence,
    };
    
    let id = SUGGESTION_LAYER.add_suggestion(&world_id, &note_id, &entity);
    Ok(id)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ner_settings_default() {
        let settings = NerSettings::default();
        assert!(!settings.enabled);
        assert!((settings.auto_promote_threshold - 0.90).abs() < 0.001);
        assert!((settings.suggest_threshold - 0.60).abs() < 0.001);
    }

    #[test]
    fn test_suggestions_response_serialization() {
        let response = SuggestionsResponse {
            suggestions: vec![],
            count: 0,
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"count\":0"));
    }

    #[test]
    fn test_action_response_with_entity() {
        let response = ActionResponse {
            success: true,
            message: "Done".to_string(),
            entity_id: Some("ent_123".to_string()),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ent_123"));
    }

    #[test]
    fn test_action_response_without_entity() {
        let response = ActionResponse {
            success: true,
            message: "Done".to_string(),
            entity_id: None,
        };
        
        let json = serde_json::to_string(&response).unwrap();
        // entity_id should be skipped when None
        assert!(!json.contains("entity_id"));
    }
}
