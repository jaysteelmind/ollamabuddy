// PRD 11: Vector Database Manager - Qdrant embedded integration
use anyhow::{Context, Result};
use qdrant_client::{
    client::QdrantClient,
    qdrant::{
        vectors_config::Config, with_payload_selector::SelectorOptions, Condition, CreateCollection,
        Distance, FieldCondition, Filter, Match, PointStruct, SearchPoints, VectorParams,
        VectorsConfig, WithPayloadSelector, PointsSelector, PointsIdsList, Value as QdrantValue,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;

const EMBEDDING_DIM: u64 = 768;

/// Vector database manager using Qdrant embedded
pub struct VectorDBManager {
    client: QdrantClient,
    db_path: PathBuf,
}

/// Query result from vector search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub id: String,
    pub score: f32,
    pub document: String,
    pub metadata: HashMap<String, JsonValue>,
}

impl VectorDBManager {
    /// Create new vector database manager (embedded mode)
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        // Create directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create vector DB directory")?;
        }

        // Initialize Qdrant client in embedded mode
        // Use absolute path for file:// URL
        let absolute_path = db_path.canonicalize()
            .unwrap_or_else(|_| db_path.clone());
        let url = format!("http://localhost:6334"); // Use HTTP for now
        let client = QdrantClient::from_url(&url)
            .build()
            .context("Failed to create Qdrant client")?;

        let manager = Self { client, db_path };

        // Initialize default collections
        manager.init_collections().await?;

        Ok(manager)
    }

    /// Initialize standard collections
    async fn init_collections(&self) -> Result<()> {
        let collections = vec!["episodes", "knowledge", "code", "documents"];

        for collection_name in collections {
            // Check if collection exists
            let collections_list = self.client.list_collections().await?;
            let exists = collections_list
                .collections
                .iter()
                .any(|c| c.name == collection_name);

            if !exists {
                // Create collection with cosine similarity
                self.client
                    .create_collection(&CreateCollection {
                        collection_name: collection_name.to_string(),
                        vectors_config: Some(VectorsConfig {
                            config: Some(Config::Params(VectorParams {
                                size: EMBEDDING_DIM,
                                distance: Distance::Cosine.into(),
                                ..Default::default()
                            })),
                        }),
                        ..Default::default()
                    })
                    .await
                    .context(format!("Failed to create collection: {}", collection_name))?;
            }
        }

        Ok(())
    }

    /// Add a document with its embedding to a collection
    pub async fn add(
        &self,
        collection: &str,
        id: &str,
        document: &str,
        embedding: &[f32],
        metadata: HashMap<String, JsonValue>,
    ) -> Result<()> {
        // Prepare payload - convert to Qdrant format
        let mut payload_map = HashMap::new();
        for (key, value) in metadata {
            payload_map.insert(key, json_to_qdrant_value(value));
        }
        payload_map.insert("document".to_string(), QdrantValue::from(document.to_string()));

        // Create point
        let point = PointStruct::new(
            id.to_string(),
            embedding.to_vec(),
            payload_map,
        );

        // Upsert point
        self.client
            .upsert_points_blocking(collection, None, vec![point], None)
            .await
            .context("Failed to upsert point")?;

        Ok(())
    }

    /// Add multiple documents in batch
    pub async fn add_batch(
        &self,
        collection: &str,
        items: Vec<(String, String, Vec<f32>, HashMap<String, JsonValue>)>,
    ) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let points: Vec<PointStruct> = items
            .into_iter()
            .map(|(id, document, embedding, metadata)| {
                let mut payload_map = HashMap::new();
                for (key, value) in metadata {
                    payload_map.insert(key, json_to_qdrant_value(value));
                }
                payload_map.insert("document".to_string(), QdrantValue::from(document));
                PointStruct::new(id, embedding, payload_map)
            })
            .collect();

        self.client
            .upsert_points_blocking(collection, None, points, None)
            .await
            .context("Failed to batch upsert points")?;

        Ok(())
    }

    /// Query similar documents
    pub async fn query(
        &self,
        collection: &str,
        query_embedding: &[f32],
        n_results: usize,
        threshold: f64,
    ) -> Result<Vec<QueryResult>> {
        let search_result = self.client
            .search_points(&SearchPoints {
                collection_name: collection.to_string(),
                vector: query_embedding.to_vec(),
                limit: n_results as u64,
                with_payload: Some(WithPayloadSelector {
                    selector_options: Some(SelectorOptions::Enable(true)),
                }),
                score_threshold: Some(threshold as f32),
                ..Default::default()
            })
            .await
            .context("Failed to search points")?;

        let results = search_result
            .result
            .into_iter()
            .map(|point| {
                let payload = point.payload;
                let document = payload
                    .get("document")
                    .and_then(|v| qdrant_value_to_string(v))
                    .unwrap_or_default();

                let mut metadata = HashMap::new();
                for (key, value) in payload {
                    if key != "document" {
                        if let Some(json_val) = qdrant_to_json_value(&value) {
                            metadata.insert(key, json_val);
                        }
                    }
                }

                QueryResult {
                    id: point_id_to_string(&point.id),
                    score: point.score,
                    document,
                    metadata,
                }
            })
            .collect();

        Ok(results)
    }

    /// Query with metadata filters
    pub async fn query_with_filter(
        &self,
        collection: &str,
        query_embedding: &[f32],
        n_results: usize,
        threshold: f64,
        filter_key: &str,
        filter_value: &str,
    ) -> Result<Vec<QueryResult>> {
        let filter = Filter {
            must: vec![Condition {
                condition_one_of: Some(
                    qdrant_client::qdrant::condition::ConditionOneOf::Field(FieldCondition {
                        key: filter_key.to_string(),
                        r#match: Some(Match {
                            match_value: Some(
                                qdrant_client::qdrant::r#match::MatchValue::Keyword(
                                    filter_value.to_string(),
                                ),
                            ),
                        }),
                        ..Default::default()
                    }),
                ),
            }],
            ..Default::default()
        };

        let search_result = self.client
            .search_points(&SearchPoints {
                collection_name: collection.to_string(),
                vector: query_embedding.to_vec(),
                limit: n_results as u64,
                with_payload: Some(WithPayloadSelector {
                    selector_options: Some(SelectorOptions::Enable(true)),
                }),
                score_threshold: Some(threshold as f32),
                filter: Some(filter),
                ..Default::default()
            })
            .await
            .context("Failed to search points with filter")?;

        let results = search_result
            .result
            .into_iter()
            .map(|point| {
                let payload = point.payload;
                let document = payload
                    .get("document")
                    .and_then(|v| qdrant_value_to_string(v))
                    .unwrap_or_default();

                let mut metadata = HashMap::new();
                for (key, value) in payload {
                    if key != "document" {
                        if let Some(json_val) = qdrant_to_json_value(&value) {
                            metadata.insert(key, json_val);
                        }
                    }
                }

                QueryResult {
                    id: point_id_to_string(&point.id),
                    score: point.score,
                    document,
                    metadata,
                }
            })
            .collect();

        Ok(results)
    }

    /// Delete a document by ID
    pub async fn delete(&self, collection: &str, id: &str) -> Result<()> {
        self.client
            .delete_points(
                collection,
                None,
                &PointsSelector {
                    points_selector_one_of: Some(
                        qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Points(
                            PointsIdsList {
                                ids: vec![qdrant_client::qdrant::PointId::from(id.to_string())],
                            },
                        ),
                    ),
                },
                None,
            )
            .await
            .context("Failed to delete point")?;

        Ok(())
    }

    /// Get collection statistics
    pub async fn collection_stats(&self, collection: &str) -> Result<u64> {
        let info = self.client
            .collection_info(collection)
            .await
            .context("Failed to get collection info")?;

        Ok(info.result.and_then(|r| r.points_count).unwrap_or(0))
    }

    /// Get database path
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }
}


// Helper functions for type conversions
fn json_to_qdrant_value(json: JsonValue) -> QdrantValue {
    match json {
        JsonValue::String(s) => QdrantValue::from(s),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                QdrantValue::from(i)
            } else if let Some(f) = n.as_f64() {
                QdrantValue::from(f)
            } else {
                QdrantValue::from(0)
            }
        }
        JsonValue::Bool(b) => QdrantValue::from(b),
        _ => QdrantValue::from(""),
    }
}

fn qdrant_to_json_value(value: &QdrantValue) -> Option<JsonValue> {
    value.kind.as_ref().and_then(|kind| {
        use qdrant_client::qdrant::value::Kind;
        match kind {
            Kind::StringValue(s) => Some(JsonValue::String(s.clone())),
            Kind::IntegerValue(i) => Some(JsonValue::Number((*i).into())),
            Kind::DoubleValue(f) => serde_json::Number::from_f64(*f).map(JsonValue::Number),
            Kind::BoolValue(b) => Some(JsonValue::Bool(*b)),
            _ => None,
        }
    })
}

fn qdrant_value_to_string(value: &QdrantValue) -> Option<String> {
    value.kind.as_ref().and_then(|kind| {
        use qdrant_client::qdrant::value::Kind;
        match kind {
            Kind::StringValue(s) => Some(s.clone()),
            _ => None,
        }
    })
}

fn point_id_to_string(point_id: &Option<qdrant_client::qdrant::PointId>) -> String {
    point_id.as_ref().map(|id| {
        use qdrant_client::qdrant::point_id::PointIdOptions;
        match &id.point_id_options {
            Some(PointIdOptions::Num(n)) => n.to_string(),
            Some(PointIdOptions::Uuid(u)) => u.clone(),
            None => "unknown".to_string(),
        }
    }).unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> (VectorDBManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let manager = VectorDBManager::new(db_path).await.unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    #[ignore]  // Integration test - requires Qdrant
    async fn test_manager_creation() {
        let (manager, _temp) = create_test_manager().await;
        assert!(manager.db_path().exists());
    }

    #[tokio::test]
    #[ignore]  // Integration test - requires Qdrant
    async fn test_add_and_query() {
        let (manager, _temp) = create_test_manager().await;
        
        let embedding = vec![0.1; 768];
        let mut metadata = HashMap::new();
        metadata.insert("category".to_string(), JsonValue::String("test".to_string()));

        manager
            .add("knowledge", "test1", "Test document", &embedding, metadata)
            .await
            .unwrap();

        let results = manager
            .query("knowledge", &embedding, 5, 0.5)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test1");
        assert_eq!(results[0].document, "Test document");
    }

    #[tokio::test]
    #[ignore]  // Integration test - requires Qdrant
    async fn test_collection_stats() {
        let (manager, _temp) = create_test_manager().await;
        
        let count = manager.collection_stats("knowledge").await.unwrap();
        assert_eq!(count, 0);

        let embedding = vec![0.1; 768];
        manager
            .add("knowledge", "test1", "Test", &embedding, HashMap::new())
            .await
            .unwrap();

        let count = manager.collection_stats("knowledge").await.unwrap();
        assert_eq!(count, 1);
    }
}
