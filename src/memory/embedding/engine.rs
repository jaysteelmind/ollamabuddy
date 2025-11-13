// PRD 11: Embedding Engine - Local embeddings via Nomic-embed-text
use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::sync::Arc;
use tokenizers::Tokenizer;

const MODEL_ID: &str = "nomic-ai/nomic-embed-text-v1.5";
const EMBEDDING_DIM: usize = 768;

/// Embedding engine using Nomic-embed-text model via Candle
pub struct EmbeddingEngine {
    model: Arc<BertModel>,
    tokenizer: Arc<Tokenizer>,
    device: Device,
}

impl EmbeddingEngine {
    /// Create new embedding engine (downloads model on first use)
    pub fn new() -> Result<Self> {
        // Determine device (CPU for now, GPU support later)
        let device = Device::Cpu;
        
        // Download model from HuggingFace Hub
        let api = Api::new().context("Failed to create HuggingFace API client")?;
        let repo = api.repo(Repo::new(MODEL_ID.to_string(), RepoType::Model));
        
        // Download required files
        let config_path = repo.get("config.json")
            .context("Failed to download model config")?;
        let tokenizer_path = repo.get("tokenizer.json")
            .context("Failed to download tokenizer")?;
        let weights_path = repo.get("model.safetensors")
            .context("Failed to download model weights")?;
        
        // Load configuration
        let config_contents = std::fs::read_to_string(config_path)
            .context("Failed to read config file")?;
        let config: Config = serde_json::from_str(&config_contents)
            .context("Failed to parse model config")?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        
        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[weights_path],
                candle_core::DType::F32,
                &device,
            ).context("Failed to load model weights")?
        };
        
        // Create model
        let model = BertModel::load(vb, &config)
            .context("Failed to create BERT model")?;
        
        Ok(Self {
            model: Arc::new(model),
            tokenizer: Arc::new(tokenizer),
            device,
        })
    }
    
    /// Generate embedding for a single text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_batch(&[text]).map(|mut v| v.pop().unwrap())
    }
    
    /// Generate embeddings for multiple texts (batched for efficiency)
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        
        // Tokenize all texts
        let encodings = self.tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;
        
        // Get token IDs and attention masks
        let mut token_ids_vec = Vec::new();
        let mut attention_mask_vec = Vec::new();
        
        for encoding in &encodings {
            token_ids_vec.push(encoding.get_ids().to_vec());
            attention_mask_vec.push(encoding.get_attention_mask().to_vec());
        }
        
        // Convert to tensors
        let max_len = token_ids_vec.iter().map(|ids| ids.len()).max().unwrap_or(0);
        let batch_size = texts.len();
        
        // Pad sequences
        let mut padded_ids = vec![vec![0u32; max_len]; batch_size];
        let mut padded_mask = vec![vec![0u32; max_len]; batch_size];
        
        for (i, (ids, mask)) in token_ids_vec.iter().zip(attention_mask_vec.iter()).enumerate() {
            padded_ids[i][..ids.len()].copy_from_slice(ids);
            padded_mask[i][..mask.len()].copy_from_slice(mask);
        }
        
        // Flatten for tensor creation
        let flat_ids: Vec<u32> = padded_ids.into_iter().flatten().collect();
        let flat_mask: Vec<u32> = padded_mask.into_iter().flatten().collect();
        
        let token_ids = Tensor::from_vec(flat_ids, (batch_size, max_len), &self.device)?;
        let attention_mask = Tensor::from_vec(flat_mask, (batch_size, max_len), &self.device)?;
        
        // Forward pass through model
        let embeddings = self.model.forward(&token_ids, &attention_mask, None)?;
        
        // Mean pooling over sequence length
        let pooled = Self::mean_pool(&embeddings, &attention_mask)?;
        
        // Convert to Vec<Vec<f32>>
        let embedding_data = pooled.to_vec2::<f32>()?;
        
        Ok(embedding_data)
    }
    
    /// Mean pooling with attention mask
    fn mean_pool(embeddings: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        // Expand attention mask to match embeddings shape
        let mask_expanded = attention_mask
            .unsqueeze(2)?
            .expand(embeddings.shape())?
            .to_dtype(embeddings.dtype())?;
        
        // Sum embeddings with mask
        let sum_embeddings = (embeddings * &mask_expanded)?.sum(1)?;
        let sum_mask = mask_expanded.sum(1)?.clamp(1e-9, f64::MAX)?;
        
        // Divide for mean
        let pooled = sum_embeddings.broadcast_div(&sum_mask)?;
        
        Ok(pooled)
    }
    
    /// Get embedding dimension (always 768 for Nomic-embed-text)
    pub fn dimension(&self) -> usize {
        EMBEDDING_DIM
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore]  // Integration test - requires model download
    fn test_embedding_dimension() {
        let engine = EmbeddingEngine::new().expect("Failed to create engine");
        assert_eq!(engine.dimension(), 768);
    }
    
    #[test]
    #[ignore]  // Integration test - requires model download
    fn test_embed_single_text() {
        let engine = EmbeddingEngine::new().expect("Failed to create engine");
        let embedding = engine.embed("Hello world").expect("Failed to embed");
        assert_eq!(embedding.len(), 768);
    }
    
    #[test]
    #[ignore]  // Integration test - requires model download
    fn test_embed_batch() {
        let engine = EmbeddingEngine::new().expect("Failed to create engine");
        let texts = vec!["Hello", "World", "Test"];
        let embeddings = engine.embed_batch(&texts).expect("Failed to embed batch");
        assert_eq!(embeddings.len(), 3);
        assert!(embeddings.iter().all(|e| e.len() == 768));
    }
    
    #[test]
    #[ignore]  // Integration test - requires model download
    fn test_embed_empty_batch() {
        let engine = EmbeddingEngine::new().expect("Failed to create engine");
        let embeddings = engine.embed_batch(&[]).expect("Failed to embed empty batch");
        assert_eq!(embeddings.len(), 0);
    }
}
