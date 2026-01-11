//! NER API - Model management, analysis, and suggestions
//!
//! Delegates directly to existing AI commands.

use crate::ai::commands;

// ============================================================================
// API Trait
// ============================================================================

#[taurpc::procedures(path = "ner", export_to = "../src/bindings.ts")]
pub trait NerApi {
    // Model Management
    async fn get_model_status() -> Result<String, String>;
    async fn download_model() -> Result<String, String>;
    
    // Analysis
    async fn request_analysis(
        doc_id: String,
        text: String,
        candidate_labels: Vec<String>,
    ) -> Result<String, String>;
    
    // Suggestions
    async fn get_suggestions(note_id: String) -> Result<String, String>;
    async fn get_all_suggestions() -> Result<String, String>;
    async fn accept_suggestion(suggestion_id: String) -> Result<String, String>;
    async fn reject_suggestion(suggestion_id: String) -> Result<String, String>;
    async fn pending_count() -> Result<usize, String>;
    async fn clear_note_suggestions(note_id: String) -> Result<String, String>;
    async fn add_suggestion(
        world_id: String,
        note_id: String,
        text: String,
        label: String,
        start: usize,
        end: usize,
        confidence: f32,
    ) -> Result<Option<String>, String>;
    
    // Settings
    async fn get_settings() -> Result<String, String>;
    async fn update_settings(settings: String) -> Result<String, String>;
}

// ============================================================================
// Implementation
// ============================================================================

#[derive(Clone, Default)]
pub struct NerApiImpl;

#[taurpc::resolvers]
impl NerApi for NerApiImpl {
    async fn get_model_status(self) -> Result<String, String> {
        let status = commands::ner_get_model_status()?;
        serde_json::to_string(&status).map_err(|e| e.to_string())
    }
    
    async fn download_model(self) -> Result<String, String> {
        let result = commands::ner_download_model().await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn request_analysis(
        self,
        doc_id: String,
        text: String,
        candidate_labels: Vec<String>,
    ) -> Result<String, String> {
        let result = commands::ner_request_analysis(doc_id, text, candidate_labels).await?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_suggestions(self, note_id: String) -> Result<String, String> {
        let result = commands::ner_get_suggestions(note_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn get_all_suggestions(self) -> Result<String, String> {
        let result = commands::ner_get_all_suggestions()?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn accept_suggestion(self, suggestion_id: String) -> Result<String, String> {
        let result = commands::ner_accept_suggestion(suggestion_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn reject_suggestion(self, suggestion_id: String) -> Result<String, String> {
        let result = commands::ner_reject_suggestion(suggestion_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn pending_count(self) -> Result<usize, String> {
        commands::ner_pending_count()
    }
    
    async fn clear_note_suggestions(self, note_id: String) -> Result<String, String> {
        let result = commands::ner_clear_note_suggestions(note_id)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
    
    async fn add_suggestion(
        self,
        world_id: String,
        note_id: String,
        text: String,
        label: String,
        start: usize,
        end: usize,
        confidence: f32,
    ) -> Result<Option<String>, String> {
        commands::ner_add_suggestion(world_id, note_id, text, label, start, end, confidence)
    }
    
    async fn get_settings(self) -> Result<String, String> {
        let settings = commands::ner_get_settings()?;
        serde_json::to_string(&settings).map_err(|e| e.to_string())
    }
    
    async fn update_settings(self, settings: String) -> Result<String, String> {
        let parsed: crate::ai::commands::NerSettings = serde_json::from_str(&settings)
            .map_err(|e| format!("Failed to parse settings: {}", e))?;
        let result = commands::ner_update_settings(parsed)?;
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }
}
