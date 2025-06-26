//! # Cache Module
//!
//! This module provides an internal cache system for contracts, tokens, and quorum keys
//! to optimize performance and reduce network requests.

use dpp::prelude::Identifier;
use js_sys::{Date, Object, Reflect};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use wasm_bindgen::prelude::*;

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

/// Thread-safe cache implementation
#[derive(Clone)]
pub struct Cache<T: Clone> {
    storage: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    default_ttl_ms: f64,
}

impl<T: Clone> Cache<T> {
    pub fn new(default_ttl_ms: f64) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            default_ttl_ms,
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
            Some(entry.data.clone())
        }
    }

    pub fn set(&self, key: String, value: T) {
        self.set_with_ttl(key, value, self.default_ttl_ms);
    }

    pub fn set_with_ttl(&self, key: String, value: T, ttl_ms: f64) {
        if let Ok(mut storage) = self.storage.write() {
            storage.insert(key, CacheEntry::new(value, ttl_ms));
        }
    }

    pub fn remove(&self, key: &str) -> Option<T> {
        if let Ok(mut storage) = self.storage.write() {
            storage.remove(key).map(|entry| entry.data)
        } else {
            None
        }
    }

    pub fn clear(&self) {
        if let Ok(mut storage) = self.storage.write() {
            storage.clear();
        }
    }

    pub fn cleanup_expired(&self) {
        if let Ok(mut storage) = self.storage.write() {
            storage.retain(|_, entry| !entry.is_expired());
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
}

#[wasm_bindgen]
impl WasmCacheManager {
    /// Create a new cache manager with default TTLs
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmCacheManager {
        WasmCacheManager {
            contracts: Cache::new(3600000.0),      // 1 hour
            identities: Cache::new(300000.0),      // 5 minutes
            documents: Cache::new(60000.0),        // 1 minute
            tokens: Cache::new(300000.0),          // 5 minutes
            quorum_keys: Cache::new(3600000.0),    // 1 hour
            metadata: Cache::new(30000.0),         // 30 seconds
        }
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
        self.contracts = Cache::new(contracts_ttl);
        self.identities = Cache::new(identities_ttl);
        self.documents = Cache::new(documents_ttl);
        self.tokens = Cache::new(tokens_ttl);
        self.quorum_keys = Cache::new(quorum_keys_ttl);
        self.metadata = Cache::new(metadata_ttl);
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
        Reflect::set(&stats, &"quorumKeys".into(), &self.quorum_keys.size().into())
            .map_err(|_| JsError::new("Failed to set quorum keys size"))?;
        Reflect::set(&stats, &"metadata".into(), &self.metadata.size().into())
            .map_err(|_| JsError::new("Failed to set metadata size"))?;
        
        let total_size = self.contracts.size() + 
                        self.identities.size() + 
                        self.documents.size() + 
                        self.tokens.size() + 
                        self.quorum_keys.size() + 
                        self.metadata.size();
        
        Reflect::set(&stats, &"totalEntries".into(), &total_size.into())
            .map_err(|_| JsError::new("Failed to set total entries"))?;
        
        Ok(stats.into())
    }
}

impl Default for WasmCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a cache key for documents
pub fn create_document_cache_key(contract_id: &str, document_type: &str, document_id: &str) -> String {
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