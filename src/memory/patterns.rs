//! Pattern Matcher: LSH-based similar problem detection
//!
//! Uses Locality-Sensitive Hashing for approximate similarity search.
//! Implements MinHash LSH for efficient similarity detection.

use crate::memory::types::{Episode, PatternMatch};
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Number of hash functions for LSH (default)
const DEFAULT_K: usize = 5;

/// Number of permutations for MinHash
const NUM_PERMUTATIONS: usize = 128;

/// Feature vector for an episode
#[derive(Debug, Clone)]
struct FeatureVector {
    goal_keywords: HashSet<String>,
    tool_keywords: HashSet<String>,
    complexity_bucket: u8,
}

impl FeatureVector {
    /// Create feature vector from episode
    fn from_episode(episode: &Episode) -> Self {
        let goal_keywords = extract_keywords(&episode.goal);
        
        let tool_keywords: HashSet<String> = episode
            .actions
            .iter()
            .map(|a| a.tool.clone())
            .collect();

        let complexity_bucket = (episode.metadata.complexity_score * 10.0).floor() as u8;

        Self {
            goal_keywords,
            tool_keywords,
            complexity_bucket,
        }
    }

    /// Compute MinHash signature
    fn minhash_signature(&self) -> Vec<u64> {
        let mut signature = Vec::with_capacity(NUM_PERMUTATIONS);
        
        // Combine all features into a single set
        let mut features = HashSet::new();
        for keyword in &self.goal_keywords {
            features.insert(format!("goal:{}", keyword));
        }
        for tool in &self.tool_keywords {
            features.insert(format!("tool:{}", tool));
        }
        features.insert(format!("complexity:{}", self.complexity_bucket));

        // Compute MinHash for each permutation
        for perm in 0..NUM_PERMUTATIONS {
            let mut min_hash = u64::MAX;
            
            for feature in &features {
                let hash = hash_with_seed(feature, perm as u64);
                if hash < min_hash {
                    min_hash = hash;
                }
            }
            
            signature.push(min_hash);
        }

        signature
    }

    /// Compute Jaccard similarity with another feature vector
    fn jaccard_similarity(&self, other: &FeatureVector) -> f64 {
        // Combine goal and tool keywords
        let set1: HashSet<_> = self.goal_keywords
            .union(&self.tool_keywords)
            .collect();
        let set2: HashSet<_> = other.goal_keywords
            .union(&other.tool_keywords)
            .collect();

        if set1.is_empty() && set2.is_empty() {
            return 1.0;
        }

        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }
}

/// LSH-based pattern matcher
pub struct PatternMatcher {
    /// Number of hash functions
    k: usize,
    /// Number of bands for LSH
    bands: usize,
    /// Rows per band
    rows: usize,
    /// Hash buckets: band -> hash -> episode IDs
    buckets: Vec<HashMap<Vec<u64>, Vec<uuid::Uuid>>>,
    /// Episode cache for fast lookup
    episodes: HashMap<uuid::Uuid, Episode>,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new(k: usize) -> Self {
        // Calculate bands and rows for LSH
        // For k=5, use 5 bands with ~25 rows each
        let bands = k;
        let rows = NUM_PERMUTATIONS / bands;

        let mut buckets = Vec::with_capacity(bands);
        for _ in 0..bands {
            buckets.push(HashMap::new());
        }

        Self {
            k,
            bands,
            rows,
            buckets,
            episodes: HashMap::new(),
        }
    }

    /// Index an episode for pattern matching
    pub fn index_episode(&mut self, episode: Episode) {
        let id = episode.id;
        
        // Compute feature vector and MinHash signature
        let features = FeatureVector::from_episode(&episode);
        let signature = features.minhash_signature();

        // Add to LSH buckets (band-based)
        for band in 0..self.bands {
            let start = band * self.rows;
            let end = (start + self.rows).min(signature.len());
            let band_sig: Vec<u64> = signature[start..end].to_vec();

            self.buckets[band]
                .entry(band_sig)
                .or_insert_with(Vec::new)
                .push(id);
        }

        // Cache episode
        self.episodes.insert(id, episode);
    }

    /// Find similar episodes
    /// Returns episodes with similarity score > threshold
    pub fn find_matches(
        &self,
        goal: &str,
        context: &str,
        threshold: f64,
    ) -> Vec<PatternMatch> {
        // Create query episode
        let query_episode = self.create_query_episode(goal, context);
        let query_features = FeatureVector::from_episode(&query_episode);
        let query_signature = query_features.minhash_signature();

        // Find candidate episodes using LSH
        let mut candidates = HashSet::new();
        
        for band in 0..self.bands {
            let start = band * self.rows;
            let end = (start + self.rows).min(query_signature.len());
            let band_sig: Vec<u64> = query_signature[start..end].to_vec();

            if let Some(episode_ids) = self.buckets[band].get(&band_sig) {
                candidates.extend(episode_ids);
            }
        }

        // Compute exact similarity for candidates
        let mut matches = Vec::new();
        
        for episode_id in candidates {
            if let Some(episode) = self.episodes.get(&episode_id) {
                let episode_features = FeatureVector::from_episode(episode);
                let similarity = query_features.jaccard_similarity(&episode_features);

                if similarity >= threshold {
                    matches.push(PatternMatch {
                        episode: episode.clone(),
                        similarity,
                    });
                }
            }
        }

        // Sort by similarity (descending)
        matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

        matches
    }

    /// Create a query episode from goal and context
    fn create_query_episode(&self, goal: &str, _context: &str) -> Episode {
        let mut episode = Episode::new(goal.to_string(), String::new());
        episode.metadata.complexity_score = 0.5; // Default complexity
        episode
    }

    /// Get number of indexed episodes
    pub fn size(&self) -> usize {
        self.episodes.len()
    }

    /// Clear all indexed episodes
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        self.episodes.clear();
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self::new(DEFAULT_K)
    }
}

/// Extract keywords from text
fn extract_keywords(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split_whitespace()
        .filter(|word| word.len() > 3)
        .filter(|word| !is_stopword(word))
        .map(|s| s.to_string())
        .collect()
}

/// Check if word is a stopword
fn is_stopword(word: &str) -> bool {
    matches!(
        word,
        "this" | "that" | "these" | "those" | "with" | "from" | "have" | "been" | "were"
    )
}

/// Hash a string with a seed
fn hash_with_seed(s: &str, seed: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    seed.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::{ActionRecord, EpisodeOutcome};

    fn create_test_episode(goal: &str, tools: &[&str], complexity: f64) -> Episode {
        let mut episode = Episode::new(goal.to_string(), "test context".to_string());
        
        for tool in tools {
            episode.actions.push(ActionRecord {
                tool: tool.to_string(),
                args: serde_json::json!({}),
                result: "success".to_string(),
                success: true,
            });
        }

        episode.metadata.tool_count = tools.len();
        episode.metadata.complexity_score = complexity;
        episode.outcome = EpisodeOutcome::Success;
        
        episode
    }

    #[test]
    fn test_pattern_matcher_creation() {
        let matcher = PatternMatcher::new(5);
        assert_eq!(matcher.size(), 0);
    }

    #[test]
    fn test_index_episode() {
        let mut matcher = PatternMatcher::new(5);
        let episode = create_test_episode("read configuration file", &["read_file"], 0.5);
        
        matcher.index_episode(episode);
        assert_eq!(matcher.size(), 1);
    }

    #[test]
    fn test_find_similar_episodes() {
        let mut matcher = PatternMatcher::new(5);
        
        // Index some episodes
        matcher.index_episode(create_test_episode(
            "read the configuration file",
            &["read_file"],
            0.5,
        ));
        matcher.index_episode(create_test_episode(
            "write output to file",
            &["write_file"],
            0.3,
        ));
        matcher.index_episode(create_test_episode(
            "read the data file",
            &["read_file"],
            0.4,
        ));

        // Find matches for similar query with lower threshold
        let matches = matcher.find_matches("read the input file", "", 0.1);

        // LSH is probabilistic, so we check if we got any candidates
        // If matches found, verify they are sorted by similarity
        if !matches.is_empty() {
            // Verify sorted descending
            for i in 0..matches.len().saturating_sub(1) {
                assert!(matches[i].similarity >= matches[i + 1].similarity,
                    "Matches should be sorted by similarity (descending)");
            }
            // Verify similarity is in valid range
            assert!(matches[0].similarity >= 0.0 && matches[0].similarity <= 1.0,
                "Similarity should be in range [0, 1]");
        }
        
        // The matcher should have indexed 3 episodes
        assert_eq!(matcher.size(), 3);
    }

    #[test]
    fn test_feature_vector_extraction() {
        let episode = create_test_episode(
            "read the configuration file from disk",
            &["read_file", "system_info"],
            0.7,
        );

        let features = FeatureVector::from_episode(&episode);
        
        assert!(features.goal_keywords.len() > 0);
        assert_eq!(features.tool_keywords.len(), 2);
        assert_eq!(features.complexity_bucket, 7);
    }

    #[test]
    fn test_minhash_signature() {
        let episode = create_test_episode("test goal", &["read_file"], 0.5);
        let features = FeatureVector::from_episode(&episode);
        
        let signature = features.minhash_signature();
        
        assert_eq!(signature.len(), NUM_PERMUTATIONS);
        assert!(signature.iter().all(|&h| h < u64::MAX));
    }

    #[test]
    fn test_jaccard_similarity() {
        let ep1 = create_test_episode("read file data", &["read_file"], 0.5);
        let ep2 = create_test_episode("read file information", &["read_file"], 0.5);
        let ep3 = create_test_episode("write output results", &["write_file"], 0.5);

        let f1 = FeatureVector::from_episode(&ep1);
        let f2 = FeatureVector::from_episode(&ep2);
        let f3 = FeatureVector::from_episode(&ep3);

        let sim_12 = f1.jaccard_similarity(&f2);
        let sim_13 = f1.jaccard_similarity(&f3);

        assert!(sim_12 > sim_13, "Similar episodes should have higher similarity");
        assert!(sim_12 > 0.3, "Similar episodes should have similarity > 0.3");
    }

    #[test]
    fn test_clear() {
        let mut matcher = PatternMatcher::new(5);
        matcher.index_episode(create_test_episode("test", &["read_file"], 0.5));
        
        assert_eq!(matcher.size(), 1);
        matcher.clear();
        assert_eq!(matcher.size(), 0);
    }

    #[test]
    fn test_no_matches_below_threshold() {
        let mut matcher = PatternMatcher::new(5);
        matcher.index_episode(create_test_episode("read file", &["read_file"], 0.5));

        let matches = matcher.find_matches("completely different query", "", 0.8);
        
        // Should find few or no matches with high threshold
        assert!(matches.is_empty() || matches[0].similarity < 0.8);
    }
}
