// src-tauri/src/rag/chunker.rs
//! Text chunking for RAG pipeline
//!
//! Splits documents into overlapping chunks for embedding and retrieval.

use serde::{Deserialize, Serialize};

/// A chunk of text with position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub index: usize,
}

/// Text chunking configuration
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum chunk size in characters
    pub max_size: usize,
    /// Overlap between chunks in characters
    pub overlap: usize,
    /// Minimum chunk size (avoid tiny chunks)
    pub min_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_size: 512,
            overlap: 64,
            min_size: 50,
        }
    }
}

/// RAG text chunker
pub struct RagChunker {
    config: ChunkConfig,
}

impl RagChunker {
    pub fn new() -> Self {
        Self {
            config: ChunkConfig::default(),
        }
    }

    pub fn with_config(config: ChunkConfig) -> Self {
        Self { config }
    }

    /// Chunk text into overlapping segments
    pub fn chunk(&self, text: &str) -> Vec<Chunk> {
        if text.is_empty() {
            return vec![];
        }

        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let total_len = chars.len();

        if total_len <= self.config.max_size {
            // Single chunk for short text
            return vec![Chunk {
                text: text.to_string(),
                start: 0,
                end: total_len,
                index: 0,
            }];
        }

        let mut start = 0;
        let mut index = 0;

        while start < total_len {
            // Calculate end position
            let mut end = (start + self.config.max_size).min(total_len);

            // Try to break at sentence or word boundary
            if end < total_len {
                end = self.find_break_point(&chars, start, end);
            }

            // Extract chunk text
            let chunk_text: String = chars[start..end].iter().collect();

            // Skip if chunk is too small (unless it's the last one)
            if chunk_text.len() >= self.config.min_size || start + self.config.max_size >= total_len
            {
                chunks.push(Chunk {
                    text: chunk_text,
                    start,
                    end,
                    index,
                });
                index += 1;
            }

            // Move to next chunk with overlap
            let step = self.config.max_size.saturating_sub(self.config.overlap);
            start += step.max(1);
        }

        chunks
    }

    /// Find a good break point (sentence end, paragraph, or word boundary)
    fn find_break_point(&self, chars: &[char], start: usize, mut end: usize) -> usize {
        let search_start = start + (self.config.max_size / 2);

        // Look for paragraph break first
        for i in (search_start..end).rev() {
            if i + 1 < chars.len() && chars[i] == '\n' && chars[i + 1] == '\n' {
                return i + 2;
            }
        }

        // Look for sentence end
        for i in (search_start..end).rev() {
            if chars[i] == '.' || chars[i] == '!' || chars[i] == '?' {
                if i + 1 < chars.len() && chars[i + 1].is_whitespace() {
                    return i + 2;
                }
            }
        }

        // Fall back to word boundary
        for i in (search_start..end).rev() {
            if chars[i].is_whitespace() {
                return i + 1;
            }
        }

        end
    }
}

impl Default for RagChunker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_short_text() {
        let chunker = RagChunker::new();
        let chunks = chunker.chunk("Hello world");

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello world");
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_chunk_long_text() {
        let chunker = RagChunker::with_config(ChunkConfig {
            max_size: 100,
            overlap: 20,
            min_size: 10,
        });

        let text = "a".repeat(250);
        let chunks = chunker.chunk(&text);

        assert!(chunks.len() >= 2);
        // Verify chunks overlap
        if chunks.len() >= 2 {
            assert!(chunks[1].start < chunks[0].end);
        }
    }

    #[test]
    fn test_chunk_empty_text() {
        let chunker = RagChunker::new();
        let chunks = chunker.chunk("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_preserves_order() {
        let chunker = RagChunker::with_config(ChunkConfig {
            max_size: 50,
            overlap: 10,
            min_size: 5,
        });

        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let chunks = chunker.chunk(text);

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }
}
