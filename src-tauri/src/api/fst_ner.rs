//! FST-NER API - Hot-path entity recognition

use crate::ner::commands as cmd;

// ============================================================================
// API Trait
// ============================================================================

#[taurpc::procedures(path = "fst_ner", export_to = "../src/bindings.ts")]
pub trait FstNerApi {
    async fn scan(text: String) -> Result<String, String>;
    async fn hydrate(entities_json: String) -> Result<usize, String>;
    async fn clear() -> Result<(), String>;
    async fn add_entity(id: String, label: String, kind: String, aliases: Vec<String>) -> Result<(), String>;
    async fn stats() -> Result<String, String>;
}

// ============================================================================
// Implementation
// ============================================================================

#[derive(Clone, Default)]
pub struct FstNerApiImpl;

#[taurpc::resolvers]
impl FstNerApi for FstNerApiImpl {
    async fn scan(self, text: String) -> Result<String, String> {
        let result = cmd::fst_scan(&text);
        serde_json::to_string(&result).map_err(|e| e.to_string())
    }

    async fn hydrate(self, entities_json: String) -> Result<usize, String> {
        let entities: Vec<cmd::EntityDef> = serde_json::from_str(&entities_json)
            .map_err(|e| format!("Failed to parse entities: {}", e))?;
        cmd::fst_hydrate(entities)
    }

    async fn clear(self) -> Result<(), String> {
        cmd::fst_clear();
        Ok(())
    }

    async fn add_entity(self, id: String, label: String, kind: String, aliases: Vec<String>) -> Result<(), String> {
        cmd::fst_add_entity(&id, &label, &kind, aliases)
    }

    async fn stats(self) -> Result<String, String> {
        let stats = cmd::fst_stats();
        serde_json::to_string(&stats).map_err(|e| e.to_string())
    }
}
