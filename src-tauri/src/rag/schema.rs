// src-tauri/src/rag/schema.rs
//! CozoDB schema for RAG chunks and HNSW indices
//!
//! Replaces the old kittcore/hnsw custom implementation with CozoDB native HNSW.

use cozo::{DbInstance, DataValue};
use std::collections::BTreeMap;

/// Initialize RAG schema in CozoDB (384d for BGE-small)
pub fn init_rag_schema_384(db: &DbInstance) -> Result<(), String> {
    // Create chunks table with 384d vector
    let result = db.run_script(
        r#"
        {
            ?[chunk_id, note_id, chunk_index, text, embedding, note_title, start_pos, end_pos, created_at] <- [[]]
            :create rag_chunks_384 {
                chunk_id: String,
                note_id: String,
                chunk_index: Int,
                text: String,
                embedding: <F32; 384>,
                note_title: String,
                start_pos: Int,
                end_pos: Int,
                created_at: Float,
                =>
            }
        }
        "#,
        Default::default(),
        cozo::ScriptMutability::Mutable,
    );

    // Ignore "already exists" errors
    if let Err(e) = result {
        let err_str = e.to_string();
        if !err_str.contains("already exists") && !err_str.contains("AlreadyExists") {
            return Err(format!("Failed to create rag_chunks_384: {}", err_str));
        }
    }

    // Create HNSW index
    let result = db.run_script(
        r#"
        ::hnsw create rag_chunks_384:embedding_idx {
            dim: 384,
            m: 16,
            dtype: F32,
            fields: [embedding],
            distance: Cosine,
            ef_construction: 64,
        }
        "#,
        Default::default(),
        cozo::ScriptMutability::Mutable,
    );

    if let Err(e) = result {
        let err_str = e.to_string();
        if !err_str.contains("already exists") && !err_str.contains("AlreadyExists") {
            return Err(format!("Failed to create HNSW index: {}", err_str));
        }
    }

    Ok(())
}

/// Initialize RAG schema in CozoDB (768d for BGE-base)
pub fn init_rag_schema_768(db: &DbInstance) -> Result<(), String> {
    // Create chunks table with 768d vector
    let result = db.run_script(
        r#"
        {
            ?[chunk_id, note_id, chunk_index, text, embedding, note_title, start_pos, end_pos, created_at] <- [[]]
            :create rag_chunks_768 {
                chunk_id: String,
                note_id: String,
                chunk_index: Int,
                text: String,
                embedding: <F32; 768>,
                note_title: String,
                start_pos: Int,
                end_pos: Int,
                created_at: Float,
                =>
            }
        }
        "#,
        Default::default(),
        cozo::ScriptMutability::Mutable,
    );

    if let Err(e) = result {
        let err_str = e.to_string();
        if !err_str.contains("already exists") && !err_str.contains("AlreadyExists") {
            return Err(format!("Failed to create rag_chunks_768: {}", err_str));
        }
    }

    // Create HNSW index
    let result = db.run_script(
        r#"
        ::hnsw create rag_chunks_768:embedding_idx {
            dim: 768,
            m: 16,
            dtype: F32,
            fields: [embedding],
            distance: Cosine,
            ef_construction: 64,
        }
        "#,
        Default::default(),
        cozo::ScriptMutability::Mutable,
    );

    if let Err(e) = result {
        let err_str = e.to_string();
        if !err_str.contains("already exists") && !err_str.contains("AlreadyExists") {
            return Err(format!("Failed to create HNSW index: {}", err_str));
        }
    }

    Ok(())
}

/// Insert a chunk with embedding into CozoDB
pub fn insert_chunk(
    db: &DbInstance,
    dim: usize,
    chunk_id: &str,
    note_id: &str,
    chunk_index: i64,
    text: &str,
    embedding: &[f32],
    note_title: &str,
    start_pos: i64,
    end_pos: i64,
    created_at: f64,
) -> Result<(), String> {
    let table = if dim == 384 { "rag_chunks_384" } else { "rag_chunks_768" };
    
    // Convert embedding to DataValue list
    let embedding_vals: Vec<DataValue> = embedding.iter()
        .map(|&v| DataValue::from(v as f64))
        .collect();
    
    let mut params = BTreeMap::new();
    params.insert("chunk_id".to_string(), DataValue::Str(chunk_id.into()));
    params.insert("note_id".to_string(), DataValue::Str(note_id.into()));
    params.insert("chunk_index".to_string(), DataValue::from(chunk_index));
    params.insert("text".to_string(), DataValue::Str(text.into()));
    params.insert("embedding".to_string(), DataValue::List(embedding_vals));
    params.insert("note_title".to_string(), DataValue::Str(note_title.into()));
    params.insert("start_pos".to_string(), DataValue::from(start_pos));
    params.insert("end_pos".to_string(), DataValue::from(end_pos));
    params.insert("created_at".to_string(), DataValue::from(created_at));
    
    let query = format!(
        r#"
        ?[chunk_id, note_id, chunk_index, text, embedding, note_title, start_pos, end_pos, created_at] <- [[
            $chunk_id, $note_id, $chunk_index, $text, $embedding, $note_title, $start_pos, $end_pos, $created_at
        ]]
        :put {} {{ chunk_id, note_id, chunk_index, text, embedding, note_title, start_pos, end_pos, created_at => }}
        "#,
        table
    );
    
    db.run_script(&query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to insert chunk: {}", e))?;
    
    Ok(())
}

/// Search for similar chunks using HNSW
pub fn search_chunks(
    db: &DbInstance,
    dim: usize,
    query_embedding: &[f32],
    k: usize,
    ef: usize,
) -> Result<Vec<ChunkSearchResult>, String> {
    let table = if dim == 384 { "rag_chunks_384" } else { "rag_chunks_768" };
    
    // Convert query embedding to vec format for CozoDB
    let query_vec_str = query_embedding.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    
    let query = format!(
        r#"
        q_vec[v] <- [[vec([{}])]]
        ?[dist, chunk_id, note_id, text, note_title, start_pos, end_pos] := 
            q_vec[q],
            ~{}:embedding_idx{{
                chunk_id, note_id, text, note_title, start_pos, end_pos 
                | query: q, k: {}, ef: {}, bind_distance: dist
            }}
        :order dist
        :limit {}
        "#,
        query_vec_str, table, k, ef, k
    );
    
    let result = db.run_script(&query, Default::default(), cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("HNSW search failed: {}", e))?;
    
    // Parse results
    let mut chunks = Vec::new();
    for row in result.rows {
        if row.len() >= 7 {
            let dist = match &row[0] {
                DataValue::Num(n) => match n {
                    cozo::Num::Float(f) => *f as f32,
                    cozo::Num::Int(i) => *i as f32,
                },
                _ => 0.0,
            };
            let chunk_id = match &row[1] {
                DataValue::Str(s) => s.to_string(),
                _ => String::new(),
            };
            let note_id = match &row[2] {
                DataValue::Str(s) => s.to_string(),
                _ => String::new(),
            };
            let text = match &row[3] {
                DataValue::Str(s) => s.to_string(),
                _ => String::new(),
            };
            let note_title = match &row[4] {
                DataValue::Str(s) => s.to_string(),
                _ => String::new(),
            };
            let start_pos = match &row[5] {
                DataValue::Num(n) => match n {
                    cozo::Num::Int(i) => *i as usize,
                    cozo::Num::Float(f) => *f as usize,
                },
                _ => 0,
            };
            let end_pos = match &row[6] {
                DataValue::Num(n) => match n {
                    cozo::Num::Int(i) => *i as usize,
                    cozo::Num::Float(f) => *f as usize,
                },
                _ => 0,
            };
            
            // Convert distance to similarity score (cosine distance -> similarity)
            let score = 1.0 - dist;
            
            chunks.push(ChunkSearchResult {
                chunk_id,
                note_id,
                text,
                note_title,
                score,
                start: start_pos,
                end: end_pos,
            });
        }
    }
    
    Ok(chunks)
}

/// Delete all chunks for a note
pub fn delete_note_chunks(db: &DbInstance, dim: usize, note_id: &str) -> Result<usize, String> {
    let table = if dim == 384 { "rag_chunks_384" } else { "rag_chunks_768" };
    
    let mut params = BTreeMap::new();
    params.insert("note_id".to_string(), DataValue::Str(note_id.into()));
    
    let query = format!(
        r#"
        ?[chunk_id] := *{}[chunk_id, note_id, _, _, _, _, _, _, _], note_id == $note_id
        :rm {} {{ chunk_id }}
        "#,
        table, table
    );
    
    let result = db.run_script(&query, params, cozo::ScriptMutability::Mutable)
        .map_err(|e| format!("Failed to delete chunks: {}", e))?;
    
    Ok(result.rows.len())
}

/// Get all chunks for a note
pub fn get_note_chunks(db: &DbInstance, dim: usize, note_id: &str) -> Result<Vec<ChunkData>, String> {
    let table = if dim == 384 { "rag_chunks_384" } else { "rag_chunks_768" };
    
    let mut params = BTreeMap::new();
    params.insert("note_id".to_string(), DataValue::Str(note_id.into()));
    
    let query = format!(
        r#"
        ?[chunk_id, note_id, chunk_index, text, note_title, start_pos, end_pos] := 
            *{}[chunk_id, note_id, chunk_index, text, _, note_title, start_pos, end_pos, _],
            note_id == $note_id
        :order chunk_index
        "#,
        table
    );
    
    let result = db.run_script(&query, params, cozo::ScriptMutability::Immutable)
        .map_err(|e| format!("Failed to get chunks: {}", e))?;
    
    let mut chunks = Vec::new();
    for row in result.rows {
        if row.len() >= 7 {
            chunks.push(ChunkData {
                chunk_id: match &row[0] { DataValue::Str(s) => s.to_string(), _ => String::new() },
                note_id: match &row[1] { DataValue::Str(s) => s.to_string(), _ => String::new() },
                chunk_index: match &row[2] { 
                    DataValue::Num(n) => match n {
                        cozo::Num::Int(i) => *i as usize,
                        cozo::Num::Float(f) => *f as usize,
                    },
                    _ => 0 
                },
                text: match &row[3] { DataValue::Str(s) => s.to_string(), _ => String::new() },
                note_title: match &row[4] { DataValue::Str(s) => s.to_string(), _ => String::new() },
                start: match &row[5] { 
                    DataValue::Num(n) => match n {
                        cozo::Num::Int(i) => *i as usize,
                        cozo::Num::Float(f) => *f as usize,
                    },
                    _ => 0 
                },
                end: match &row[6] { 
                    DataValue::Num(n) => match n {
                        cozo::Num::Int(i) => *i as usize,
                        cozo::Num::Float(f) => *f as usize,
                    },
                    _ => 0 
                },
            });
        }
    }
    
    Ok(chunks)
}

/// Search result from HNSW
#[derive(Debug, Clone)]
pub struct ChunkSearchResult {
    pub chunk_id: String,
    pub note_id: String,
    pub text: String,
    pub note_title: String,
    pub score: f32,
    pub start: usize,
    pub end: usize,
}

/// Chunk data without embedding
#[derive(Debug, Clone)]
pub struct ChunkData {
    pub chunk_id: String,
    pub note_id: String,
    pub chunk_index: usize,
    pub text: String,
    pub note_title: String,
    pub start: usize,
    pub end: usize,
}
