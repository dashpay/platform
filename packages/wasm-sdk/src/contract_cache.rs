//! Enhanced Contract Cache Module
//!
//! This module provides an optimized caching layer specifically for data contracts,
//! with support for versioning, lazy loading, and intelligent cache management.

use crate::cache::WasmCacheManager;
use crate::error::to_js_error;
use dpp::data_contract::DataContract;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::serialization::{
    PlatformLimitDeserializableFromVersionedStructure,
    PlatformSerializableWithPlatformVersion,
};
use js_sys::{Array, Date, Object, Reflect};
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::*;

/// Contract cache configuration
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct ContractCacheConfig {
    /// Maximum number of contracts to cache
    max_contracts: usize,
    /// TTL for contract cache entries in milliseconds
    ttl_ms: f64,
    /// Whether to cache contract history
    cache_history: bool,
    /// Maximum versions per contract to cache
    max_versions_per_contract: usize,
    /// Whether to enable automatic preloading of related contracts
    enable_preloading: bool,
}

#[wasm_bindgen]
impl ContractCacheConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            max_contracts: 100,
            ttl_ms: 3600000.0, // 1 hour default
            cache_history: true,
            max_versions_per_contract: 5,
            enable_preloading: true,
        }
    }

    #[wasm_bindgen(js_name = setMaxContracts)]
    pub fn set_max_contracts(&mut self, max: usize) {
        self.max_contracts = max;
    }

    #[wasm_bindgen(js_name = setTtl)]
    pub fn set_ttl(&mut self, ttl_ms: f64) {
        self.ttl_ms = ttl_ms;
    }

    #[wasm_bindgen(js_name = setCacheHistory)]
    pub fn set_cache_history(&mut self, enable: bool) {
        self.cache_history = enable;
    }

    #[wasm_bindgen(js_name = setMaxVersionsPerContract)]
    pub fn set_max_versions_per_contract(&mut self, max: usize) {
        self.max_versions_per_contract = max;
    }

    #[wasm_bindgen(js_name = setEnablePreloading)]
    pub fn set_enable_preloading(&mut self, enable: bool) {
        self.enable_preloading = enable;
    }
}

impl Default for ContractCacheConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Contract metadata for cache management
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ContractMetadata {
    id: String,
    version: u32,
    owner_id: String,
    schema_hash: String,
    document_types: Vec<String>,
    last_accessed: f64,
    access_count: u32,
    size_bytes: usize,
    dependencies: Vec<String>, // Other contract IDs this contract depends on
}

/// Cached contract entry
#[derive(Clone)]
struct CachedContract {
    _contract: DataContract,
    metadata: ContractMetadata,
    raw_bytes: Vec<u8>,
    cached_at: f64,
    ttl_ms: f64,
}

impl CachedContract {
    fn is_expired(&self) -> bool {
        Date::now() - self.cached_at > self.ttl_ms
    }

    fn update_access(&mut self) {
        self.metadata.last_accessed = Date::now();
        self.metadata.access_count += 1;
    }
}

/// Advanced contract cache with LRU eviction and smart preloading
#[wasm_bindgen]
pub struct ContractCache {
    config: ContractCacheConfig,
    contracts: Arc<RwLock<HashMap<String, CachedContract>>>,
    version_index: Arc<RwLock<HashMap<String, Vec<u32>>>>, // contract_id -> versions
    access_patterns: Arc<RwLock<HashMap<String, Vec<f64>>>>, // contract_id -> access timestamps
    preload_queue: Arc<RwLock<Vec<String>>>,
}

#[wasm_bindgen]
impl ContractCache {
    #[wasm_bindgen(constructor)]
    pub fn new(config: Option<ContractCacheConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            contracts: Arc::new(RwLock::new(HashMap::new())),
            version_index: Arc::new(RwLock::new(HashMap::new())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            preload_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Cache a contract
    #[wasm_bindgen(js_name = cacheContract)]
    pub fn cache_contract(&self, contract_bytes: &[u8]) -> Result<String, JsError> {
        use platform_version::version::LATEST_PLATFORM_VERSION;
        let platform_version = &LATEST_PLATFORM_VERSION;
        let contract = DataContract::versioned_limit_deserialize(contract_bytes, platform_version)
            .map_err(|e| JsError::new(&format!("Failed to deserialize contract: {}", e)))?;

        let contract_id = contract.id().to_string(platform_value::string_encoding::Encoding::Base58);
        let version = contract.version();
        
        // Create metadata
        let metadata = ContractMetadata {
            id: contract_id.clone(),
            version,
            owner_id: contract.owner_id().to_string(platform_value::string_encoding::Encoding::Base58),
            schema_hash: self.calculate_schema_hash(&contract)?,
            document_types: self.get_document_types(&contract),
            last_accessed: Date::now(),
            access_count: 0,
            size_bytes: contract_bytes.len(),
            dependencies: self.extract_dependencies(&contract),
        };

        // Create cache entry
        let entry = CachedContract {
            _contract: contract,
            metadata,
            raw_bytes: contract_bytes.to_vec(),
            cached_at: Date::now(),
            ttl_ms: self.config.ttl_ms,
        };

        // Check cache size and evict if necessary
        self.evict_if_necessary()?;

        // Store in cache
        if let Ok(mut cache) = self.contracts.write() {
            cache.insert(contract_id.clone(), entry);
        }

        // Update version index
        if self.config.cache_history {
            if let Ok(mut index) = self.version_index.write() {
                index.entry(contract_id.clone())
                    .or_insert_with(Vec::new)
                    .push(version);
            }
        }

        // Queue related contracts for preloading
        if self.config.enable_preloading {
            self.queue_dependencies_for_preload(&contract_id)?;
        }

        Ok(contract_id)
    }

    /// Get a cached contract
    #[wasm_bindgen(js_name = getCachedContract)]
    pub fn get_cached_contract(&self, contract_id: &str) -> Option<Vec<u8>> {
        if let Ok(mut cache) = self.contracts.write() {
            if let Some(entry) = cache.get_mut(contract_id) {
                if entry.is_expired() {
                    cache.remove(contract_id);
                    return None;
                }
                
                entry.update_access();
                self.record_access(contract_id);
                
                return Some(entry.raw_bytes.clone());
            }
        }
        None
    }

    /// Get contract metadata
    #[wasm_bindgen(js_name = getContractMetadata)]
    pub fn get_contract_metadata(&self, contract_id: &str) -> Result<JsValue, JsError> {
        if let Ok(cache) = self.contracts.read() {
            if let Some(entry) = cache.get(contract_id) {
                let obj = Object::new();
                Reflect::set(&obj, &"id".into(), &entry.metadata.id.clone().into())
                    .map_err(|_| JsError::new("Failed to set id"))?;
                Reflect::set(&obj, &"version".into(), &entry.metadata.version.into())
                    .map_err(|_| JsError::new("Failed to set version"))?;
                Reflect::set(&obj, &"ownerId".into(), &entry.metadata.owner_id.clone().into())
                    .map_err(|_| JsError::new("Failed to set ownerId"))?;
                Reflect::set(&obj, &"schemaHash".into(), &entry.metadata.schema_hash.clone().into())
                    .map_err(|_| JsError::new("Failed to set schemaHash"))?;
                
                let doc_types = Array::new();
                for doc_type in &entry.metadata.document_types {
                    doc_types.push(&doc_type.into());
                }
                Reflect::set(&obj, &"documentTypes".into(), &doc_types)
                    .map_err(|_| JsError::new("Failed to set documentTypes"))?;
                
                Reflect::set(&obj, &"lastAccessed".into(), &entry.metadata.last_accessed.into())
                    .map_err(|_| JsError::new("Failed to set lastAccessed"))?;
                Reflect::set(&obj, &"accessCount".into(), &entry.metadata.access_count.into())
                    .map_err(|_| JsError::new("Failed to set accessCount"))?;
                Reflect::set(&obj, &"sizeBytes".into(), &entry.metadata.size_bytes.into())
                    .map_err(|_| JsError::new("Failed to set sizeBytes"))?;
                
                let deps = Array::new();
                for dep in &entry.metadata.dependencies {
                    deps.push(&dep.into());
                }
                Reflect::set(&obj, &"dependencies".into(), &deps)
                    .map_err(|_| JsError::new("Failed to set dependencies"))?;
                
                return Ok(obj.into());
            }
        }
        Err(JsError::new("Contract not found in cache"))
    }

    /// Check if a contract is cached
    #[wasm_bindgen(js_name = isContractCached)]
    pub fn is_contract_cached(&self, contract_id: &str) -> bool {
        if let Ok(cache) = self.contracts.read() {
            if let Some(entry) = cache.get(contract_id) {
                return !entry.is_expired();
            }
        }
        false
    }

    /// Get all cached contract IDs
    #[wasm_bindgen(js_name = getCachedContractIds)]
    pub fn get_cached_contract_ids(&self) -> Array {
        let ids = Array::new();
        if let Ok(cache) = self.contracts.read() {
            for (id, entry) in cache.iter() {
                if !entry.is_expired() {
                    ids.push(&id.into());
                }
            }
        }
        ids
    }

    /// Get cache statistics
    #[wasm_bindgen(js_name = getCacheStats)]
    pub fn get_cache_stats(&self) -> Result<JsValue, JsError> {
        let stats = Object::new();
        
        if let Ok(cache) = self.contracts.read() {
            let total_contracts = cache.len();
            let total_size: usize = cache.values().map(|e| e.metadata.size_bytes).sum();
            let avg_access_count: f64 = if total_contracts > 0 {
                cache.values().map(|e| e.metadata.access_count as f64).sum::<f64>() / total_contracts as f64
            } else {
                0.0
            };
            
            Reflect::set(&stats, &"totalContracts".into(), &total_contracts.into())
                .map_err(|_| JsError::new("Failed to set totalContracts"))?;
            Reflect::set(&stats, &"totalSizeBytes".into(), &total_size.into())
                .map_err(|_| JsError::new("Failed to set totalSizeBytes"))?;
            Reflect::set(&stats, &"averageAccessCount".into(), &avg_access_count.into())
                .map_err(|_| JsError::new("Failed to set averageAccessCount"))?;
            Reflect::set(&stats, &"maxContracts".into(), &self.config.max_contracts.into())
                .map_err(|_| JsError::new("Failed to set maxContracts"))?;
            Reflect::set(&stats, &"ttlMs".into(), &self.config.ttl_ms.into())
                .map_err(|_| JsError::new("Failed to set ttlMs"))?;
            
            // Most accessed contracts
            let mut contracts: Vec<_> = cache.values().collect();
            contracts.sort_by(|a, b| b.metadata.access_count.cmp(&a.metadata.access_count));
            
            let most_accessed = Array::new();
            for entry in contracts.iter().take(5) {
                let obj = Object::new();
                Reflect::set(&obj, &"id".into(), &entry.metadata.id.clone().into())
                    .map_err(|_| JsError::new("Failed to set id in stats"))?;
                Reflect::set(&obj, &"accessCount".into(), &entry.metadata.access_count.into())
                    .map_err(|_| JsError::new("Failed to set accessCount in stats"))?;
                most_accessed.push(&obj);
            }
            Reflect::set(&stats, &"mostAccessed".into(), &most_accessed)
                .map_err(|_| JsError::new("Failed to set mostAccessed"))?;
        }
        
        Ok(stats.into())
    }

    /// Clear the cache
    #[wasm_bindgen(js_name = clearCache)]
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.contracts.write() {
            cache.clear();
        }
        if let Ok(mut index) = self.version_index.write() {
            index.clear();
        }
        if let Ok(mut patterns) = self.access_patterns.write() {
            patterns.clear();
        }
        if let Ok(mut queue) = self.preload_queue.write() {
            queue.clear();
        }
    }

    /// Remove expired entries
    #[wasm_bindgen(js_name = cleanupExpired)]
    pub fn cleanup_expired(&self) -> u32 {
        let mut removed = 0;
        if let Ok(mut cache) = self.contracts.write() {
            let expired_ids: Vec<String> = cache
                .iter()
                .filter(|(_, entry)| entry.is_expired())
                .map(|(id, _)| id.clone())
                .collect();
            
            for id in expired_ids {
                cache.remove(&id);
                removed += 1;
            }
        }
        removed
    }

    /// Preload contracts based on access patterns
    #[wasm_bindgen(js_name = getPreloadSuggestions)]
    pub fn get_preload_suggestions(&self) -> Array {
        let suggestions = Array::new();
        
        if let Ok(patterns) = self.access_patterns.read() {
            // Analyze access patterns to suggest contracts to preload
            let mut scores: HashMap<String, f64> = HashMap::new();
            
            for (contract_id, timestamps) in patterns.iter() {
                if timestamps.len() >= 2 {
                    // Calculate access frequency
                    let frequency = timestamps.len() as f64;
                    let recency = Date::now() - timestamps.last().copied().unwrap_or(0.0);
                    let score = frequency * 1000.0 / (recency + 1.0);
                    scores.insert(contract_id.clone(), score);
                }
            }
            
            // Sort by score and suggest top contracts
            let mut sorted_scores: Vec<_> = scores.into_iter().collect();
            sorted_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            for (contract_id, _score) in sorted_scores.iter().take(10) {
                if !self.is_contract_cached(contract_id) {
                    suggestions.push(&contract_id.into());
                }
            }
        }
        
        suggestions
    }

    // Private helper methods
    
    fn calculate_schema_hash(&self, contract: &DataContract) -> Result<String, JsError> {
        use sha2::{Sha256, Digest};
        use platform_version::version::LATEST_PLATFORM_VERSION;
        let platform_version = &LATEST_PLATFORM_VERSION;
        
        let schema_bytes = contract.serialize_to_bytes_with_platform_version(platform_version)
            .map_err(to_js_error)?;
        
        let mut hasher = Sha256::new();
        hasher.update(&schema_bytes);
        let result = hasher.finalize();
        
        Ok(hex::encode(result))
    }
    
    fn get_document_types(&self, contract: &DataContract) -> Vec<String> {
        match contract {
            DataContract::V0(v0) => v0.document_types.keys().cloned().collect(),
            DataContract::V1(v1) => v1.document_types.keys().cloned().collect(),
        }
    }
    
    fn extract_dependencies(&self, contract: &DataContract) -> Vec<String> {
        let mut dependencies = HashSet::new();
        
        // Get document types based on contract version
        let document_types = match contract {
            DataContract::V0(v0) => &v0.document_types,
            DataContract::V1(v1) => &v1.document_types,
        };
        
        // Analyze each document type for references
        for (_doc_name, doc_type) in document_types.iter() {
            // Convert document schema to Value for analysis
            let schema = doc_type.schema();
            self.find_contract_references(schema, &mut dependencies);
        }
        
        // Note: Schema definitions ($defs) are not directly accessible in the current API
        // They would be embedded within document type schemas
        
        dependencies.into_iter().collect()
    }
    
    /// Recursively find contract ID references in a schema
    fn find_contract_references(&self, value: &Value, dependencies: &mut HashSet<String>) {
        match value {
            Value::Map(map) => {
                for (key, val) in map.iter() {
                    // Check for $ref pattern pointing to other contracts
                    if let Value::Text(key_str) = key {
                        if key_str == "$ref" {
                            if let Value::Text(ref_str) = val {
                                // Parse reference format: "#/$defs/<contract_id>/<type>"
                                // or "<contract_id>#<path>"
                                if let Some(contract_id) = self.extract_contract_id_from_ref(ref_str) {
                                    dependencies.insert(contract_id);
                                }
                            }
                        }
                        
                        // Check for byteArray contentMediaType references
                        if key_str == "contentMediaType" {
                            if let Value::Text(media_type) = val {
                                if media_type.starts_with("application/x.dash.dpp.identifier") {
                                    // This field references another contract/document
                                    // Extract from pattern or sibling properties
                                    // Look for pattern field in the same map
                                    for (pattern_key, pattern_val) in map.iter() {
                                        if let Value::Text(pattern_key_str) = pattern_key {
                                            if pattern_key_str == "pattern" {
                                                if let Value::Text(pattern) = pattern_val {
                                                    if let Some(contract_id) = self.extract_contract_id_from_pattern(pattern) {
                                                        dependencies.insert(contract_id);
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Recurse into nested structures
                    self.find_contract_references(val, dependencies);
                }
            }
            Value::Array(array) => {
                for item in array {
                    self.find_contract_references(item, dependencies);
                }
            }
            _ => {}
        }
    }
    
    /// Extract contract ID from a $ref string
    fn extract_contract_id_from_ref(&self, ref_str: &str) -> Option<String> {
        // Handle external contract references
        // Format: "<contract_id>#<path>" or "#/$defs/<contract_id>/<type>"
        if ref_str.contains('#') && !ref_str.starts_with('#') {
            // External reference: contract_id#path
            ref_str.split('#').next().map(|s| s.to_string())
        } else if ref_str.starts_with("#/$defs/") {
            // Internal reference that might contain contract ID
            let parts: Vec<&str> = ref_str.trim_start_matches("#/$defs/").split('/').collect();
            if parts.len() >= 2 {
                // Check if first part looks like a contract ID (base58 string)
                let potential_id = parts[0];
                if potential_id.len() > 20 && potential_id.chars().all(|c| c.is_alphanumeric()) {
                    Some(potential_id.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Extract contract ID from a pattern constraint
    fn extract_contract_id_from_pattern(&self, pattern: &str) -> Option<String> {
        // Pattern might contain contract ID in format like "^[contract_id]:[document_type]$"
        if pattern.contains(':') {
            let parts: Vec<&str> = pattern.trim_start_matches('^').trim_end_matches('$').split(':').collect();
            if parts.len() >= 2 {
                let contract_id = parts[0].trim_matches(|c| c == '[' || c == ']');
                if contract_id.len() > 20 && contract_id.chars().all(|c| c.is_alphanumeric()) {
                    Some(contract_id.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    
    fn evict_if_necessary(&self) -> Result<(), JsError> {
        if let Ok(mut cache) = self.contracts.write() {
            if cache.len() >= self.config.max_contracts {
                // Find least recently used contract
                let lru_id = cache
                    .iter()
                    .min_by_key(|(_, entry)| entry.metadata.last_accessed as i64)
                    .map(|(id, _)| id.clone());
                
                if let Some(id) = lru_id {
                    cache.remove(&id);
                }
            }
        }
        Ok(())
    }
    
    fn record_access(&self, contract_id: &str) {
        if let Ok(mut patterns) = self.access_patterns.write() {
            patterns
                .entry(contract_id.to_string())
                .or_insert_with(Vec::new)
                .push(Date::now());
            
            // Keep only recent accesses (last 100)
            if let Some(timestamps) = patterns.get_mut(contract_id) {
                if timestamps.len() > 100 {
                    timestamps.drain(0..timestamps.len() - 100);
                }
            }
        }
    }
    
    fn queue_dependencies_for_preload(&self, contract_id: &str) -> Result<(), JsError> {
        if let Ok(cache) = self.contracts.read() {
            if let Some(entry) = cache.get(contract_id) {
                if let Ok(mut queue) = self.preload_queue.write() {
                    for dep in &entry.metadata.dependencies {
                        if !queue.contains(dep) {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Create a global contract cache instance
#[wasm_bindgen(js_name = createContractCache)]
pub fn create_contract_cache(config: Option<ContractCacheConfig>) -> ContractCache {
    ContractCache::new(config)
}

/// Integration with WasmCacheManager
#[wasm_bindgen(js_name = integrateContractCache)]
pub fn integrate_contract_cache(
    _cache_manager: &WasmCacheManager,
    _contract_cache: &ContractCache,
) -> Result<(), JsError> {
    // This function would integrate the specialized contract cache
    // with the general cache manager for unified cache management
    
    // For now, just return success
    Ok(())
}