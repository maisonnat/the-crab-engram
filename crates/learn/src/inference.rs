//! Local inference engine — lazy-loaded LLM for memory curation.
//!
//! Behind the `inference` feature flag. Loads a GGUF model on first use,
//! releases memory after inference. Supports prompt prefix caching for
//! reduced TTFT (Time To First Token).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use tracing::info;

/// Opaque cache key for a pre-processed system prompt prefix.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    pub hash: String,
}

/// Host-memory cache for prompt prefixes (system prompt + JSON schema).
/// Stores the tokenized prefix so it doesn't need reprocessing per request.
struct PromptCache {
    entries: std::collections::HashMap<String, Vec<u8>>,
}

impl PromptCache {
    fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    fn insert(&mut self, key: &str, tokens: Vec<u8>) {
        self.entries.insert(key.to_string(), tokens);
    }

    fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.entries.get(key)
    }
}

/// Inference engine with lazy model loading.
///
/// The model is loaded on the first call to `infer()` and kept in memory
/// until `unload()` is called. This allows on-demand GPU/RAM usage.
pub struct InferenceEngine {
    model_path: PathBuf,
    /// Lazy-loaded model state. None = not loaded yet.
    model: Arc<Mutex<Option<LoadedModel>>>,
    cache: Arc<Mutex<PromptCache>>,
}

/// Represents a loaded GGUF model.
/// When the `inference` feature is active, this wraps llama_cpp_rs.
/// Without the feature, this is a placeholder for testing.
#[cfg(feature = "inference")]
struct LoadedModel {
    _model: llama_cpp_rs::Model,
    _context: llama_cpp_rs::ModelContext,
}

#[cfg(not(feature = "inference"))]
struct LoadedModel {
    /// Placeholder for non-inference builds — stores the path for diagnostics.
    _path: PathBuf,
}

impl InferenceEngine {
    /// Create a new inference engine pointing to a GGUF model file.
    /// The model is NOT loaded until `infer()` or `load()` is called.
    pub fn new(model_path: PathBuf) -> Self {
        info!("InferenceEngine created for {:?}", model_path);
        Self {
            model_path,
            model: Arc::new(Mutex::new(None)),
            cache: Arc::new(Mutex::new(PromptCache::new())),
        }
    }

    /// Check if the model is currently loaded in memory.
    pub fn is_loaded(&self) -> bool {
        self.model.lock().map(|m| m.is_some()).unwrap_or(false)
    }

    /// Explicitly load the model into memory.
    /// Called automatically by `infer()` if not yet loaded.
    #[cfg(feature = "inference")]
    pub fn load(&self) -> Result<()> {
        let mut guard = self
            .model
            .lock()
            .map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        if guard.is_some() {
            return Ok(());
        }
        info!("Loading GGUF model from {:?}", self.model_path);
        // TODO: Initialize llama_cpp_rs::Model and ModelContext here
        // let model = llama_cpp_rs::Model::load_from_file(&self.model_path, ...)?;
        // let ctx = model.create_context(...)?;
        // *guard = Some(LoadedModel { _model: model, _context: ctx });
        anyhow::bail!("llama_cpp_rs integration not yet wired up — use stub mode");
    }

    #[cfg(not(feature = "inference"))]
    pub fn load(&self) -> Result<()> {
        let mut guard = self
            .model
            .lock()
            .map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        if guard.is_some() {
            return Ok(());
        }
        info!("Loading model (stub mode) from {:?}", self.model_path);
        *guard = Some(LoadedModel {
            _path: self.model_path.clone(),
        });
        Ok(())
    }

    /// Unload the model from memory, freeing resources.
    pub fn unload(&self) {
        if let Ok(mut guard) = self.model.lock() {
            if guard.take().is_some() {
                info!("Model unloaded, memory released");
            }
        }
    }

    /// Run inference: generate a response from a prompt.
    /// Loads the model on first call (lazy initialization).
    #[cfg(feature = "inference")]
    pub fn infer(&self, prompt: &str) -> Result<String> {
        if !self.is_loaded() {
            self.load()?;
        }
        // TODO: Use llama_cpp_rs to run inference
        // let guard = self.model.lock().unwrap();
        // let loaded = guard.as_ref().ok_or_else(|| anyhow::anyhow!("model not loaded"))?;
        // loaded._context.generate(prompt, ...)
        Ok(format!(
            "[stub inference for: {}]",
            &prompt[..prompt.len().min(50)]
        ))
    }

    #[cfg(not(feature = "inference"))]
    pub fn infer(&self, prompt: &str) -> Result<String> {
        if !self.is_loaded() {
            self.load()?;
        }
        // Stub mode: return a placeholder
        Ok(format!(
            "[stub inference for: {}]",
            &prompt[..prompt.len().min(50)]
        ))
    }

    /// Cache a system prompt + schema prefix for reuse across inferences.
    pub fn cache_system_prompt(&self, system: &str, schema: &str) -> CacheKey {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let key_str = format!("{}|||{}", system, schema);
        let mut hasher = DefaultHasher::new();
        key_str.hash(&mut hasher);
        let hash = format!("{:016x}", hasher.finish());

        // In stub mode, store the raw concatenated prompt for reuse.
        // With inference enabled, this stores the tokenized prefix instead.
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(&hash, key_str.as_bytes().to_vec());
        }

        CacheKey { hash }
    }

    /// Run inference with a cached system prompt prefix.
    /// Uses the pre-cached prefix to reduce TTFT.
    pub fn infer_with_cache(&self, cache_key: &CacheKey, user_prompt: &str) -> Result<String> {
        let full_prompt = if let Ok(cache) = self.cache.lock() {
            if cache.get(&cache_key.hash).is_some() {
                // Prefix found in cache — in real impl, we'd skip tokenizing it
                format!("[cached-prefix] {}", user_prompt)
            } else {
                user_prompt.to_string()
            }
        } else {
            user_prompt.to_string()
        };
        self.infer(&full_prompt)
    }
}

impl Drop for InferenceEngine {
    fn drop(&mut self) {
        self.unload();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_new_not_loaded() {
        let engine = InferenceEngine::new(PathBuf::from("/tmp/test.gguf"));
        assert!(!engine.is_loaded());
    }

    #[test]
    fn engine_load_unload_cycle() {
        let engine = InferenceEngine::new(PathBuf::from("/tmp/test.gguf"));
        assert!(!engine.is_loaded());
        engine.load().unwrap();
        assert!(engine.is_loaded());
        engine.unload();
        assert!(!engine.is_loaded());
    }

    #[test]
    fn cache_system_prompt_returns_key() {
        let engine = InferenceEngine::new(PathBuf::from("/tmp/test.gguf"));
        let key = engine.cache_system_prompt("You are a helpful assistant", "schema.json");
        assert!(!key.hash.is_empty());
        // Same input → same key
        let key2 = engine.cache_system_prompt("You are a helpful assistant", "schema.json");
        assert_eq!(key.hash, key2.hash);
    }

    #[test]
    fn infer_auto_loads_model() {
        let engine = InferenceEngine::new(PathBuf::from("/tmp/test.gguf"));
        assert!(!engine.is_loaded());
        let result = engine.infer("hello").unwrap();
        assert!(result.contains("stub inference"));
        assert!(engine.is_loaded());
    }

    #[test]
    fn infer_with_cache_works() {
        let engine = InferenceEngine::new(PathBuf::from("/tmp/test.gguf"));
        let key = engine.cache_system_prompt("system", "schema");
        let result = engine.infer_with_cache(&key, "user prompt").unwrap();
        assert!(result.contains("stub inference"));
    }
}
