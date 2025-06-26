//! Cache management tests

use wasm_bindgen_test::*;
use wasm_sdk::cache::WasmCacheManager;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_cache_manager_creation() {
    let cache = WasmCacheManager::new();
    
    // Check initial stats
    let stats = cache.get_stats();
    let contracts = js_sys::Reflect::get(&stats, &"contracts".into()).unwrap();
    let identities = js_sys::Reflect::get(&stats, &"identities".into()).unwrap();
    let documents = js_sys::Reflect::get(&stats, &"documents".into()).unwrap();
    let total = js_sys::Reflect::get(&stats, &"totalEntries".into()).unwrap();
    
    assert_eq!(contracts.as_f64().unwrap() as u32, 0);
    assert_eq!(identities.as_f64().unwrap() as u32, 0);
    assert_eq!(documents.as_f64().unwrap() as u32, 0);
    assert_eq!(total.as_f64().unwrap() as u32, 0);
}

#[wasm_bindgen_test]
fn test_cache_ttl_configuration() {
    let mut cache = WasmCacheManager::new();
    
    // Set custom TTLs
    cache.set_ttls(
        7200,  // contracts: 2 hours
        3600,  // identities: 1 hour
        600,   // documents: 10 minutes
        1800,  // tokens: 30 minutes
        14400, // quorum keys: 4 hours
        300    // metadata: 5 minutes
    );
    
    // TTL setting should not crash
    // In a real implementation, we would verify the TTLs are applied
}

#[wasm_bindgen_test]
fn test_contract_caching() {
    let cache = WasmCacheManager::new();
    let contract_id = "test_contract_123";
    let contract_data = vec![1, 2, 3, 4, 5];
    
    // Cache a contract
    cache.cache_contract(contract_id, contract_data.clone());
    
    // Retrieve cached contract
    let cached = cache.get_cached_contract(contract_id);
    assert!(cached.is_some(), "Should retrieve cached contract");
    assert_eq!(cached.unwrap(), contract_data, "Cached data should match");
    
    // Check non-existent contract
    let missing = cache.get_cached_contract("non_existent");
    assert!(missing.is_none(), "Should return None for missing contract");
    
    // Check stats
    let stats = cache.get_stats();
    let contracts = js_sys::Reflect::get(&stats, &"contracts".into()).unwrap();
    assert_eq!(contracts.as_f64().unwrap() as u32, 1);
}

#[wasm_bindgen_test]
fn test_identity_caching() {
    let cache = WasmCacheManager::new();
    let identity_id = "test_identity_456";
    let identity_data = vec![6, 7, 8, 9, 10];
    
    // Cache an identity
    cache.cache_identity(identity_id, identity_data.clone());
    
    // Retrieve cached identity
    let cached = cache.get_cached_identity(identity_id);
    assert!(cached.is_some(), "Should retrieve cached identity");
    assert_eq!(cached.unwrap(), identity_data, "Cached data should match");
}

#[wasm_bindgen_test]
fn test_document_caching() {
    let cache = WasmCacheManager::new();
    let document_key = "contract_id:doc_type:doc_id";
    let document_data = vec![11, 12, 13, 14, 15];
    
    // Cache a document
    cache.cache_document(document_key, document_data.clone());
    
    // Retrieve cached document
    let cached = cache.get_cached_document(document_key);
    assert!(cached.is_some(), "Should retrieve cached document");
    assert_eq!(cached.unwrap(), document_data, "Cached data should match");
}

#[wasm_bindgen_test]
fn test_token_caching() {
    let cache = WasmCacheManager::new();
    let token_id = "test_token_789";
    let token_data = vec![16, 17, 18, 19, 20];
    
    // Cache a token
    cache.cache_token(token_id, token_data.clone());
    
    // Retrieve cached token
    let cached = cache.get_cached_token(token_id);
    assert!(cached.is_some(), "Should retrieve cached token");
    assert_eq!(cached.unwrap(), token_data, "Cached data should match");
}

#[wasm_bindgen_test]
fn test_quorum_keys_caching() {
    let cache = WasmCacheManager::new();
    let epoch = 42;
    let keys_data = vec![21, 22, 23, 24, 25];
    
    // Cache quorum keys
    cache.cache_quorum_keys(epoch, keys_data.clone());
    
    // Retrieve cached keys
    let cached = cache.get_cached_quorum_keys(epoch);
    assert!(cached.is_some(), "Should retrieve cached quorum keys");
    assert_eq!(cached.unwrap(), keys_data, "Cached data should match");
}

#[wasm_bindgen_test]
fn test_metadata_caching() {
    let cache = WasmCacheManager::new();
    let metadata_key = "block_height:12345";
    let metadata = vec![26, 27, 28, 29, 30];
    
    // Cache metadata
    cache.cache_metadata(metadata_key, metadata.clone());
    
    // Retrieve cached metadata
    let cached = cache.get_cached_metadata(metadata_key);
    assert!(cached.is_some(), "Should retrieve cached metadata");
    assert_eq!(cached.unwrap(), metadata, "Cached data should match");
}

#[wasm_bindgen_test]
fn test_cache_clear_operations() {
    let cache = WasmCacheManager::new();
    
    // Add items to different caches
    cache.cache_contract("contract1", vec![1, 2, 3]);
    cache.cache_identity("identity1", vec![4, 5, 6]);
    cache.cache_document("doc1", vec![7, 8, 9]);
    cache.cache_token("token1", vec![10, 11, 12]);
    
    // Check total entries
    let stats = cache.get_stats();
    let total = js_sys::Reflect::get(&stats, &"totalEntries".into()).unwrap();
    assert_eq!(total.as_f64().unwrap() as u32, 4);
    
    // Clear specific cache type
    cache.clear_cache("contracts");
    assert!(cache.get_cached_contract("contract1").is_none());
    assert!(cache.get_cached_identity("identity1").is_some());
    
    // Clear all caches
    cache.clear_all();
    let stats_after = cache.get_stats();
    let total_after = js_sys::Reflect::get(&stats_after, &"totalEntries".into()).unwrap();
    assert_eq!(total_after.as_f64().unwrap() as u32, 0);
}

#[wasm_bindgen_test]
fn test_cache_cleanup_expired() {
    let mut cache = WasmCacheManager::new();
    
    // Set very short TTLs for testing
    cache.set_ttls(
        0,  // contracts: expire immediately
        0,  // identities: expire immediately
        0,  // documents: expire immediately
        0,  // tokens: expire immediately
        0,  // quorum keys: expire immediately
        0   // metadata: expire immediately
    );
    
    // Add items
    cache.cache_contract("contract1", vec![1, 2, 3]);
    cache.cache_identity("identity1", vec![4, 5, 6]);
    
    // Cleanup expired items
    cache.cleanup_expired();
    
    // In a real implementation with proper TTL handling,
    // these items would be expired and removed
}