//! # Cache Module
//!
//! This module provides an internal cache system for contracts, tokens, and quorum keys
//! to optimize performance and reduce network requests.

use js_sys::{Date, Object, Reflect};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::RwLock;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::window;

/// Cache entry with timestamp for TTL management
#[derive(Clone, Debug)]
struct CacheEntry<T> {
    data: T,
    timestamp: f64,
    ttl_ms: f64,
}

impl<T> CacheEntry<T> {
    fn new(data: T, ttl_ms: f64) -> Self {
        Self {
            data,
            timestamp: Date::now(),
            ttl_ms,
        }
    }

    fn is_expired(&self) -> bool {
        Date::now() - self.timestamp > self.ttl_ms
    }
}

/// Thread-safe LRU cache implementation with size limits
#[derive(Clone)]
pub struct Cache<T: Clone> {
    storage: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    lru_keys: Arc<RwLock<VecDeque<String>>>,
    default_ttl_ms: f64,
    max_size: usize,
}

impl<T: Clone> Cache<T> {
    pub fn new(default_ttl_ms: f64) -> Self {
        Self::with_size_limit(default_ttl_ms, 1000) // Default max size of 1000 entries
    }

    pub fn with_size_limit(default_ttl_ms: f64, max_size: usize) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            lru_keys: Arc::new(RwLock::new(VecDeque::new())),
            default_ttl_ms,
            max_size,
        }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        let storage = self.storage.read().ok()?;
        let entry = storage.get(key)?;

        if entry.is_expired() {
            drop(storage);
            self.remove(key);
            None
        } else {
            // Update LRU order
            if let Ok(mut lru) = self.lru_keys.write() {
                if let Some(pos) = lru.iter().position(|k| k == key) {
                    lru.remove(pos);
                }
                lru.push_back(key.to_string());
            }
            Some(entry.data.clone())
        }
    }

    pub fn set(&self, key: String, value: T) {
        self.set_with_ttl(key, value, self.default_ttl_ms);
    }

    pub fn set_with_ttl(&self, key: String, value: T, ttl_ms: f64) {
        if let (Ok(mut storage), Ok(mut lru)) = (self.storage.write(), self.lru_keys.write()) {
            // Check if we need to evict entries
            while storage.len() >= self.max_size {
                if let Some(oldest_key) = lru.pop_front() {
                    storage.remove(&oldest_key);
                } else {
                    break;
                }
            }

            // Remove key from LRU if it already exists
            if let Some(pos) = lru.iter().position(|k| k == &key) {
                lru.remove(pos);
            }

            // Insert new entry and update LRU
            storage.insert(key.clone(), CacheEntry::new(value, ttl_ms));
            lru.push_back(key);
        }
    }

    pub fn remove(&self, key: &str) -> Option<T> {
        if let (Ok(mut storage), Ok(mut lru)) = (self.storage.write(), self.lru_keys.write()) {
            // Remove from LRU
            if let Some(pos) = lru.iter().position(|k| k == key) {
                lru.remove(pos);
            }
            storage.remove(key).map(|entry| entry.data)
        } else {
            None
        }
    }

    pub fn clear(&self) {
        if let (Ok(mut storage), Ok(mut lru)) = (self.storage.write(), self.lru_keys.write()) {
            storage.clear();
            lru.clear();
        }
    }

    pub fn cleanup_expired(&self) {
        if let (Ok(mut storage), Ok(mut lru)) = (self.storage.write(), self.lru_keys.write()) {
            let mut expired_keys = Vec::new();
            storage.retain(|key, entry| {
                if entry.is_expired() {
                    expired_keys.push(key.clone());
                    false
                } else {
                    true
                }
            });

            // Remove expired keys from LRU
            for expired_key in expired_keys {
                if let Some(pos) = lru.iter().position(|k| k == &expired_key) {
                    lru.remove(pos);
                }
            }
        }
    }

    pub fn size(&self) -> usize {
        self.storage.read().map(|s| s.len()).unwrap_or(0)
    }
}

/// WASM-exposed cache manager for the SDK
#[wasm_bindgen]
pub struct WasmCacheManager {
    contracts: Cache<Vec<u8>>,
    identities: Cache<Vec<u8>>,
    documents: Cache<Vec<u8>>,
    tokens: Cache<Vec<u8>>,
    quorum_keys: Cache<Vec<u8>>,
    metadata: Cache<Vec<u8>>,
    max_sizes: CacheMaxSizes,
    cleanup_interval_handle: Option<i32>,
}

/// Maximum sizes for each cache type
struct CacheMaxSizes {
    contracts: usize,
    identities: usize,
    documents: usize,
    tokens: usize,
    quorum_keys: usize,
    metadata: usize,
}

impl Default for CacheMaxSizes {
    fn default() -> Self {
        Self {
            contracts: 100,  // Max 100 contracts
            identities: 500, // Max 500 identities
            documents: 1000, // Max 1000 documents
            tokens: 200,     // Max 200 token infos
            quorum_keys: 50, // Max 50 quorum key sets
            metadata: 100,   // Max 100 metadata entries
        }
    }
}

#[wasm_bindgen]
impl WasmCacheManager {
    /// Create a new cache manager with default TTLs and size limits
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmCacheManager {
        let max_sizes = CacheMaxSizes::default();
        let mut manager = WasmCacheManager {
            contracts: Cache::with_size_limit(3600000.0, max_sizes.contracts), // 1 hour
            identities: Cache::with_size_limit(300000.0, max_sizes.identities), // 5 minutes
            documents: Cache::with_size_limit(60000.0, max_sizes.documents),   // 1 minute
            tokens: Cache::with_size_limit(300000.0, max_sizes.tokens),        // 5 minutes
            quorum_keys: Cache::with_size_limit(3600000.0, max_sizes.quorum_keys), // 1 hour
            metadata: Cache::with_size_limit(30000.0, max_sizes.metadata),     // 30 seconds
            max_sizes,
            cleanup_interval_handle: None,
        };

        // Start automatic cleanup every 5 minutes
        manager.start_auto_cleanup(300000); // 5 minutes

        manager
    }

    /// Set custom TTLs for each cache type
    #[wasm_bindgen(js_name = setTTLs)]
    pub fn set_ttls(
        &mut self,
        contracts_ttl: f64,
        identities_ttl: f64,
        documents_ttl: f64,
        tokens_ttl: f64,
        quorum_keys_ttl: f64,
        metadata_ttl: f64,
    ) {
        self.contracts = Cache::with_size_limit(contracts_ttl, self.max_sizes.contracts);
        self.identities = Cache::with_size_limit(identities_ttl, self.max_sizes.identities);
        self.documents = Cache::with_size_limit(documents_ttl, self.max_sizes.documents);
        self.tokens = Cache::with_size_limit(tokens_ttl, self.max_sizes.tokens);
        self.quorum_keys = Cache::with_size_limit(quorum_keys_ttl, self.max_sizes.quorum_keys);
        self.metadata = Cache::with_size_limit(metadata_ttl, self.max_sizes.metadata);
    }

    /// Set custom size limits for each cache type
    #[wasm_bindgen(js_name = setMaxSizes)]
    pub fn set_max_sizes(
        &mut self,
        contracts_max: usize,
        identities_max: usize,
        documents_max: usize,
        tokens_max: usize,
        quorum_keys_max: usize,
        metadata_max: usize,
    ) {
        self.max_sizes = CacheMaxSizes {
            contracts: contracts_max,
            identities: identities_max,
            documents: documents_max,
            tokens: tokens_max,
            quorum_keys: quorum_keys_max,
            metadata: metadata_max,
        };

        // Recreate caches with new size limits
        let contracts_ttl = self.contracts.default_ttl_ms;
        let identities_ttl = self.identities.default_ttl_ms;
        let documents_ttl = self.documents.default_ttl_ms;
        let tokens_ttl = self.tokens.default_ttl_ms;
        let quorum_keys_ttl = self.quorum_keys.default_ttl_ms;
        let metadata_ttl = self.metadata.default_ttl_ms;

        self.contracts = Cache::with_size_limit(contracts_ttl, contracts_max);
        self.identities = Cache::with_size_limit(identities_ttl, identities_max);
        self.documents = Cache::with_size_limit(documents_ttl, documents_max);
        self.tokens = Cache::with_size_limit(tokens_ttl, tokens_max);
        self.quorum_keys = Cache::with_size_limit(quorum_keys_ttl, quorum_keys_max);
        self.metadata = Cache::with_size_limit(metadata_ttl, metadata_max);
    }

    /// Cache a data contract
    #[wasm_bindgen(js_name = cacheContract)]
    pub fn cache_contract(&self, contract_id: &str, contract_data: Vec<u8>) {
        self.contracts.set(contract_id.to_string(), contract_data);
    }

    /// Get a cached data contract
    #[wasm_bindgen(js_name = getCachedContract)]
    pub fn get_cached_contract(&self, contract_id: &str) -> Option<Vec<u8>> {
        self.contracts.get(contract_id)
    }

    /// Cache an identity
    #[wasm_bindgen(js_name = cacheIdentity)]
    pub fn cache_identity(&self, identity_id: &str, identity_data: Vec<u8>) {
        self.identities.set(identity_id.to_string(), identity_data);
    }

    /// Get a cached identity
    #[wasm_bindgen(js_name = getCachedIdentity)]
    pub fn get_cached_identity(&self, identity_id: &str) -> Option<Vec<u8>> {
        self.identities.get(identity_id)
    }

    /// Cache a document
    #[wasm_bindgen(js_name = cacheDocument)]
    pub fn cache_document(&self, document_key: &str, document_data: Vec<u8>) {
        self.documents.set(document_key.to_string(), document_data);
    }

    /// Get a cached document
    #[wasm_bindgen(js_name = getCachedDocument)]
    pub fn get_cached_document(&self, document_key: &str) -> Option<Vec<u8>> {
        self.documents.get(document_key)
    }

    /// Cache token information
    #[wasm_bindgen(js_name = cacheToken)]
    pub fn cache_token(&self, token_id: &str, token_data: Vec<u8>) {
        self.tokens.set(token_id.to_string(), token_data);
    }

    /// Get cached token information
    #[wasm_bindgen(js_name = getCachedToken)]
    pub fn get_cached_token(&self, token_id: &str) -> Option<Vec<u8>> {
        self.tokens.get(token_id)
    }

    /// Cache quorum keys
    #[wasm_bindgen(js_name = cacheQuorumKeys)]
    pub fn cache_quorum_keys(&self, epoch: u32, keys_data: Vec<u8>) {
        let key = format!("quorum_keys_{}", epoch);
        self.quorum_keys.set(key, keys_data);
    }

    /// Get cached quorum keys
    #[wasm_bindgen(js_name = getCachedQuorumKeys)]
    pub fn get_cached_quorum_keys(&self, epoch: u32) -> Option<Vec<u8>> {
        let key = format!("quorum_keys_{}", epoch);
        self.quorum_keys.get(&key)
    }

    /// Cache metadata
    #[wasm_bindgen(js_name = cacheMetadata)]
    pub fn cache_metadata(&self, key: &str, metadata: Vec<u8>) {
        self.metadata.set(key.to_string(), metadata);
    }

    /// Get cached metadata
    #[wasm_bindgen(js_name = getCachedMetadata)]
    pub fn get_cached_metadata(&self, key: &str) -> Option<Vec<u8>> {
        self.metadata.get(key)
    }

    /// Clear all caches
    #[wasm_bindgen(js_name = clearAll)]
    pub fn clear_all(&self) {
        self.contracts.clear();
        self.identities.clear();
        self.documents.clear();
        self.tokens.clear();
        self.quorum_keys.clear();
        self.metadata.clear();
    }

    /// Clear a specific cache type
    #[wasm_bindgen(js_name = clearCache)]
    pub fn clear_cache(&self, cache_type: &str) {
        match cache_type {
            "contracts" => self.contracts.clear(),
            "identities" => self.identities.clear(),
            "documents" => self.documents.clear(),
            "tokens" => self.tokens.clear(),
            "quorum_keys" => self.quorum_keys.clear(),
            "metadata" => self.metadata.clear(),
            _ => {}
        }
    }

    /// Remove expired entries from all caches
    #[wasm_bindgen(js_name = cleanupExpired)]
    pub fn cleanup_expired(&self) {
        self.contracts.cleanup_expired();
        self.identities.cleanup_expired();
        self.documents.cleanup_expired();
        self.tokens.cleanup_expired();
        self.quorum_keys.cleanup_expired();
        self.metadata.cleanup_expired();
    }

    /// Get cache statistics
    #[wasm_bindgen(js_name = getStats)]
    pub fn get_stats(&self) -> Result<JsValue, JsError> {
        let stats = Object::new();

        Reflect::set(&stats, &"contracts".into(), &self.contracts.size().into())
            .map_err(|_| JsError::new("Failed to set contracts size"))?;
        Reflect::set(&stats, &"identities".into(), &self.identities.size().into())
            .map_err(|_| JsError::new("Failed to set identities size"))?;
        Reflect::set(&stats, &"documents".into(), &self.documents.size().into())
            .map_err(|_| JsError::new("Failed to set documents size"))?;
        Reflect::set(&stats, &"tokens".into(), &self.tokens.size().into())
            .map_err(|_| JsError::new("Failed to set tokens size"))?;
        Reflect::set(
            &stats,
            &"quorumKeys".into(),
            &self.quorum_keys.size().into(),
        )
        .map_err(|_| JsError::new("Failed to set quorum keys size"))?;
        Reflect::set(&stats, &"metadata".into(), &self.metadata.size().into())
            .map_err(|_| JsError::new("Failed to set metadata size"))?;

        let total_size = self.contracts.size()
            + self.identities.size()
            + self.documents.size()
            + self.tokens.size()
            + self.quorum_keys.size()
            + self.metadata.size();

        Reflect::set(&stats, &"totalEntries".into(), &total_size.into())
            .map_err(|_| JsError::new("Failed to set total entries"))?;

        // Add max sizes
        Reflect::set(
            &stats,
            &"maxContracts".into(),
            &(self.max_sizes.contracts as u32).into(),
        )
        .map_err(|_| JsError::new("Failed to set max contracts"))?;
        Reflect::set(
            &stats,
            &"maxIdentities".into(),
            &(self.max_sizes.identities as u32).into(),
        )
        .map_err(|_| JsError::new("Failed to set max identities"))?;
        Reflect::set(
            &stats,
            &"maxDocuments".into(),
            &(self.max_sizes.documents as u32).into(),
        )
        .map_err(|_| JsError::new("Failed to set max documents"))?;
        Reflect::set(
            &stats,
            &"maxTokens".into(),
            &(self.max_sizes.tokens as u32).into(),
        )
        .map_err(|_| JsError::new("Failed to set max tokens"))?;
        Reflect::set(
            &stats,
            &"maxQuorumKeys".into(),
            &(self.max_sizes.quorum_keys as u32).into(),
        )
        .map_err(|_| JsError::new("Failed to set max quorum keys"))?;
        Reflect::set(
            &stats,
            &"maxMetadata".into(),
            &(self.max_sizes.metadata as u32).into(),
        )
        .map_err(|_| JsError::new("Failed to set max metadata"))?;

        Ok(stats.into())
    }

    /// Start automatic cleanup with specified interval in milliseconds
    #[wasm_bindgen(js_name = startAutoCleanup)]
    pub fn start_auto_cleanup(&mut self, interval_ms: u32) {
        // Stop existing cleanup if any
        self.stop_auto_cleanup();

        // Create a closure that can be called repeatedly
        let cleanup_fn = {
            let contracts = self.contracts.clone();
            let identities = self.identities.clone();
            let documents = self.documents.clone();
            let tokens = self.tokens.clone();
            let quorum_keys = self.quorum_keys.clone();
            let metadata = self.metadata.clone();

            Closure::<dyn Fn()>::new(move || {
                contracts.cleanup_expired();
                identities.cleanup_expired();
                documents.cleanup_expired();
                tokens.cleanup_expired();
                quorum_keys.cleanup_expired();
                metadata.cleanup_expired();
            })
        };

        // Set up interval
        if let Some(window) = window() {
            if let Ok(handle) = window.set_interval_with_callback_and_timeout_and_arguments_0(
                cleanup_fn.as_ref().unchecked_ref(),
                interval_ms as i32,
            ) {
                self.cleanup_interval_handle = Some(handle);
            }
        }

        // Keep the closure alive
        cleanup_fn.forget();
    }

    /// Stop automatic cleanup
    #[wasm_bindgen(js_name = stopAutoCleanup)]
    pub fn stop_auto_cleanup(&mut self) {
        if let Some(handle) = self.cleanup_interval_handle {
            if let Some(window) = window() {
                window.clear_interval_with_handle(handle);
            }
            self.cleanup_interval_handle = None;
        }
    }
}

impl Default for WasmCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WasmCacheManager {
    fn drop(&mut self) {
        // Clean up interval when cache manager is dropped
        self.stop_auto_cleanup();
    }
}

/// Create a cache key for documents
pub fn create_document_cache_key(
    contract_id: &str,
    document_type: &str,
    document_id: &str,
) -> String {
    format!("{}_{}_{}", contract_id, document_type, document_id)
}

/// Create a cache key for document queries
pub fn create_document_query_cache_key(
    contract_id: &str,
    document_type: &str,
    where_clause: &str,
    order_by: &str,
    limit: u32,
    offset: u32,
) -> String {
    format!(
        "query_{}_{}_{}_{}_{}_{}",
        contract_id, document_type, where_clause, order_by, limit, offset
    )
}

/// Create a cache key for identity by public key hash
pub fn create_identity_by_key_cache_key(public_key_hash: &[u8]) -> String {
    format!("identity_by_key_{}", hex::encode(public_key_hash))
}

/// Create a cache key for token balances
pub fn create_token_balance_cache_key(token_id: &str, identity_id: &str) -> String {
    format!("token_balance_{}_{}", token_id, identity_id)
}
