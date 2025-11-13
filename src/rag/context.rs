// PRD 11 Phase 2: Context builder for RAG-augmented prompts
use serde::{Deserialize, Serialize};

use crate::rag::reranking::RankedDocument;

/// Context assembly configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Maximum tokens for retrieved context
    pub max_context_tokens: usize,
    /// Include document metadata in context
    pub include_metadata: bool,
    /// Format for context presentation
    pub format: ContextFormat,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 2000,
            include_metadata: true,
            format: ContextFormat::Structured,
        }
    }
}

/// Format for presenting context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextFormat {
    /// Structured with clear document boundaries
    Structured,
    /// Compact inline format
    Inline,
    /// Numbered list format
    Numbered,
}

/// Assembled context for prompt augmentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledContext {
    /// The formatted context text
    pub text: String,
    /// Number of documents included
    pub document_count: usize,
    /// Estimated token count
    pub estimated_tokens: usize,
    /// Document IDs included
    pub document_ids: Vec<String>,
}

/// Context builder for assembling RAG context
pub struct ContextBuilder {
    config: ContextConfig,
}

impl ContextBuilder {
    /// Create new context builder with default config
    pub fn new() -> Self {
        Self {
            config: ContextConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ContextConfig) -> Self {
        Self { config }
    }

    /// Build context from ranked documents
    pub fn build(&self, documents: &[RankedDocument]) -> AssembledContext {
        let mut context_parts = Vec::new();
        let mut total_tokens = 0;
        let mut included_ids = Vec::new();
        let mut doc_count = 0;

        for (idx, ranked_doc) in documents.iter().enumerate() {
            let doc = &ranked_doc.document;
            
            // Estimate tokens (rough: ~4 chars per token)
            let doc_tokens = doc.content.len() / 4;
            
            // Check if adding this document would exceed token limit
            if total_tokens + doc_tokens > self.config.max_context_tokens {
                break;
            }

            // Format document based on config
            let formatted = self.format_document(doc_count + 1, doc, ranked_doc);
            context_parts.push(formatted);
            
            total_tokens += doc_tokens;
            included_ids.push(doc.id.clone());
            doc_count += 1;
        }

        let text = match self.config.format {
            ContextFormat::Structured => {
                format!(
                    "Retrieved Context ({} documents):\n\n{}\n",
                    doc_count,
                    context_parts.join("\n\n")
                )
            }
            ContextFormat::Inline => {
                format!("Context: {}", context_parts.join(" | "))
            }
            ContextFormat::Numbered => {
                format!("Context:\n{}\n", context_parts.join("\n"))
            }
        };

        AssembledContext {
            text,
            document_count: doc_count,
            estimated_tokens: total_tokens,
            document_ids: included_ids,
        }
    }

    /// Format a single document
    fn format_document(
        &self,
        index: usize,
        doc: &crate::rag::retrieval::RetrievedDocument,
        ranked: &RankedDocument,
    ) -> String {
        match self.config.format {
            ContextFormat::Structured => {
                if self.config.include_metadata {
                    format!(
                        "[Document {}] (score: {:.2}, category: {})\n{}",
                        index, ranked.reranked_score, doc.category, doc.content
                    )
                } else {
                    format!("[Document {}]\n{}", index, doc.content)
                }
            }
            ContextFormat::Inline => {
                if self.config.include_metadata {
                    format!("[{}|{:.2}] {}", doc.category, ranked.reranked_score, doc.content)
                } else {
                    doc.content.clone()
                }
            }
            ContextFormat::Numbered => {
                if self.config.include_metadata {
                    format!(
                        "{}. (score: {:.2}) {}",
                        index, ranked.reranked_score, doc.content
                    )
                } else {
                    format!("{}. {}", index, doc.content)
                }
            }
        }
    }

    /// Build context and augment a prompt
    pub fn augment_prompt(&self, prompt: &str, documents: &[RankedDocument]) -> String {
        let context = self.build(documents);
        
        if context.document_count == 0 {
            // No relevant context found
            return prompt.to_string();
        }

        // Prepend context to prompt
        format!("{}\n\nUser Query: {}", context.text, prompt)
    }

    /// Get current configuration
    pub fn config(&self) -> &ContextConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ContextConfig) {
        self.config = config;
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::retrieval::RetrievedDocument;

    fn create_ranked_doc(id: &str, content: &str, score: f32) -> RankedDocument {
        let doc = RetrievedDocument {
            id: id.to_string(),
            content: content.to_string(),
            category: "test".to_string(),
            score,
            metadata: serde_json::Map::new(),
        };

        RankedDocument {
            document: doc,
            original_score: score,
            reranked_score: score,
            boost_applied: 0.0,
        }
    }

    #[test]
    fn test_context_builder_creation() {
        let builder = ContextBuilder::new();
        assert_eq!(builder.config.max_context_tokens, 2000);
        assert!(builder.config.include_metadata);
    }

    #[test]
    fn test_build_empty_documents() {
        let builder = ContextBuilder::new();
        let context = builder.build(&[]);
        assert_eq!(context.document_count, 0);
        assert_eq!(context.estimated_tokens, 0);
    }

    #[test]
    fn test_build_single_document() {
        let builder = ContextBuilder::new();
        let docs = vec![create_ranked_doc("1", "Test content", 0.9)];
        
        let context = builder.build(&docs);
        assert_eq!(context.document_count, 1);
        assert!(context.text.contains("Test content"));
        assert_eq!(context.document_ids.len(), 1);
    }

    #[test]
    fn test_build_respects_token_limit() {
        let config = ContextConfig {
            max_context_tokens: 10, // Very low limit
            include_metadata: false,
            format: ContextFormat::Inline,
        };
        let builder = ContextBuilder::with_config(config);
        
        let docs = vec![
            create_ranked_doc("1", "Short", 0.9),
            create_ranked_doc("2", "This is a much longer document that will exceed the token limit", 0.8),
        ];
        
        let context = builder.build(&docs);
        // Should only include first document due to token limit
        assert_eq!(context.document_count, 1);
    }

    #[test]
    fn test_augment_prompt() {
        let builder = ContextBuilder::new();
        let docs = vec![create_ranked_doc("1", "Background info", 0.9)];
        
        let augmented = builder.augment_prompt("What is the answer?", &docs);
        assert!(augmented.contains("Background info"));
        assert!(augmented.contains("What is the answer?"));
    }

    #[test]
    fn test_context_formats() {
        let builder_structured = ContextBuilder::with_config(ContextConfig {
            format: ContextFormat::Structured,
            ..Default::default()
        });
        
        let builder_inline = ContextBuilder::with_config(ContextConfig {
            format: ContextFormat::Inline,
            ..Default::default()
        });
        
        let docs = vec![create_ranked_doc("1", "Test", 0.9)];
        
        let ctx_structured = builder_structured.build(&docs);
        let ctx_inline = builder_inline.build(&docs);
        
        assert!(ctx_structured.text.contains("[Document 1]"));
        assert!(ctx_inline.text.contains("Context:"));
    }
}
