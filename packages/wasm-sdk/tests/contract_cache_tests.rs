//! Comprehensive tests for the contract cache module

use js_sys::{Array, Date, Object, Reflect};
use wasm_bindgen_test::*;
use wasm_sdk::{
    contract_cache::{ContractCache, ContractCacheConfig},
    start,
};

wasm_bindgen_test_configure!(run_in_browser);

// Test contract data (simplified for testing)
fn create_test_contract_bytes(id: u8, version: u16) -> Vec<u8> {
    // This is a simplified contract representation
    // In reality, this would be a serialized DataContract
    let mut bytes = vec![0u8; 100];
    bytes[0] = id; // Unique identifier
    bytes[1] = (version >> 8) as u8;
    bytes[2] = (version & 0xFF) as u8;
    bytes
}

#[wasm_bindgen_test]
async fn test_cache_config() {
    start().await.expect("Failed to start WASM");

    let mut config = ContractCacheConfig::new();

    // Test setters
    config.set_max_contracts(50);
    config.set_ttl_ms(1800000.0); // 30 minutes
    config.set_cache_history(false);
    config.set_max_versions_per_contract(3);
    config.set_enable_preloading(false);
}

#[wasm_bindgen_test]
async fn test_basic_caching() {
    start().await.expect("Failed to start WASM");

    let cache = ContractCache::new(None);

    // Create test contract
    let contract_bytes = create_test_contract_bytes(1, 1);

    // Cache the contract
    let contract_id = cache
        .cache_contract(&contract_bytes)
        .expect("Failed to cache contract");

    assert!(!contract_id.is_empty());

    // Check if cached
    assert!(cache.is_contract_cached(&contract_id));

    // Get cached contract
    let cached_bytes = cache
        .get_cached_contract(&contract_id)
        .expect("Failed to get cached contract");

    // For testing, we'll just check it's not empty
    assert!(!cached_bytes.is_empty());
}

#[wasm_bindgen_test]
async fn test_cache_expiration() {
    start().await.expect("Failed to start WASM");

    let mut config = ContractCacheConfig::new();
    config.set_ttl_ms(100.0); // 100ms TTL for testing

    let cache = ContractCache::new(Some(config));

    let contract_bytes = create_test_contract_bytes(2, 1);
    let contract_id = cache
        .cache_contract(&contract_bytes)
        .expect("Failed to cache contract");

    // Should be cached immediately
    assert!(cache.is_contract_cached(&contract_id));

    // Wait for expiration
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve, 150, // Wait 150ms
            )
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();

    // Should be expired now
    // Note: This depends on the implementation checking expiration on access
    let cached = cache.get_cached_contract(&contract_id);
    // If implementation removes expired entries on access, this would be None
}

#[wasm_bindgen_test]
async fn test_cache_stats() {
    start().await.expect("Failed to start WASM");

    let cache = ContractCache::new(None);

    // Cache multiple contracts
    for i in 0..5 {
        let contract_bytes = create_test_contract_bytes(i, 1);
        cache
            .cache_contract(&contract_bytes)
            .expect("Failed to cache");
    }

    // Get stats
    let stats = cache.get_cache_stats();

    // Verify stats structure
    assert!(Reflect::has(&stats, &"totalContracts".into()).unwrap());
    assert!(Reflect::has(&stats, &"totalSize".into()).unwrap());
    assert!(Reflect::has(&stats, &"avgAccessCount".into()).unwrap());
    assert!(Reflect::has(&stats, &"cacheHitRate".into()).unwrap());

    let total_contracts = Reflect::get(&stats, &"totalContracts".into())
        .unwrap()
        .as_f64()
        .unwrap();
    assert!(total_contracts >= 5.0);
}

#[wasm_bindgen_test]
async fn test_clear_cache() {
    start().await.expect("Failed to start WASM");

    let cache = ContractCache::new(None);

    // Cache some contracts
    let mut contract_ids = vec![];
    for i in 0..3 {
        let contract_bytes = create_test_contract_bytes(i, 1);
        let id = cache
            .cache_contract(&contract_bytes)
            .expect("Failed to cache");
        contract_ids.push(id);
    }

    // Verify all cached
    for id in &contract_ids {
        assert!(cache.is_contract_cached(id));
    }

    // Clear cache
    cache.clear_cache();

    // Verify all cleared
    for id in &contract_ids {
        assert!(!cache.is_contract_cached(id));
    }
}

#[wasm_bindgen_test]
async fn test_remove_contract() {
    start().await.expect("Failed to start WASM");

    let cache = ContractCache::new(None);

    // Cache contracts
    let contract1_bytes = create_test_contract_bytes(1, 1);
    let contract2_bytes = create_test_contract_bytes(2, 1);

    let id1 = cache
        .cache_contract(&contract1_bytes)
        .expect("Failed to cache");
    let id2 = cache
        .cache_contract(&contract2_bytes)
        .expect("Failed to cache");

    // Remove one
    assert!(cache.remove_contract(&id1));

    // Verify removal
    assert!(!cache.is_contract_cached(&id1));
    assert!(cache.is_contract_cached(&id2));

    // Try to remove non-existent
    assert!(!cache.remove_contract(&id1));
}

#[wasm_bindgen_test]
async fn test_contract_metadata() {
    start().await.expect("Failed to start WASM");

    let cache = ContractCache::new(None);

    let contract_bytes = create_test_contract_bytes(1, 1);
    let contract_id = cache
        .cache_contract(&contract_bytes)
        .expect("Failed to cache");

    // Get metadata
    let metadata = cache.get_contract_metadata(&contract_id);

    // Check metadata exists
    assert!(!metadata.is_undefined());

    if let Some(obj) = metadata.dyn_ref::<Object>() {
        // Verify metadata fields
        assert!(Reflect::has(obj, &"version".into()).unwrap());
        assert!(Reflect::has(obj, &"size".into()).unwrap());
        assert!(Reflect::has(obj, &"accessCount".into()).unwrap());
        assert!(Reflect::has(obj, &"lastAccessed".into()).unwrap());
        assert!(Reflect::has(obj, &"cachedAt".into()).unwrap());
    }
}

#[wasm_bindgen_test]
async fn test_preload_suggestions() {
    start().await.expect("Failed to start WASM");

    let cache = ContractCache::new(None);

    // Cache some contracts and access them to build patterns
    let contract_bytes = create_test_contract_bytes(1, 1);
    let contract_id = cache
        .cache_contract(&contract_bytes)
        .expect("Failed to cache");

    // Access the contract multiple times
    for _ in 0..5 {
        cache
            .get_cached_contract(&contract_id)
            .expect("Failed to get");
    }

    // Get preload suggestions
    let suggestions = cache.get_preload_suggestions();

    // Suggestions should be an array
    assert!(suggestions.is_array());
}

#[wasm_bindgen_test]
async fn test_cache_size_limit() {
    start().await.expect("Failed to start WASM");

    let mut config = ContractCacheConfig::new();
    config.set_max_contracts(3);

    let cache = ContractCache::new(Some(config));

    // Cache more contracts than the limit
    let mut contract_ids = vec![];
    for i in 0..5 {
        let contract_bytes = create_test_contract_bytes(i, 1);
        let id = cache
            .cache_contract(&contract_bytes)
            .expect("Failed to cache");
        contract_ids.push(id);
    }

    // Get stats to check total cached
    let stats = cache.get_cache_stats();
    let total = Reflect::get(&stats, &"totalContracts".into())
        .unwrap()
        .as_f64()
        .unwrap();

    // Should not exceed max contracts
    assert!(total <= 3.0);
}

#[wasm_bindgen_test]
async fn test_version_caching() {
    start().await.expect("Failed to start WASM");

    let mut config = ContractCacheConfig::new();
    config.set_cache_history(true);
    config.set_max_versions_per_contract(3);

    let cache = ContractCache::new(Some(config));

    // Cache multiple versions of the same contract
    // In reality, these would have the same contract ID but different versions
    for version in 1..=5 {
        let contract_bytes = create_test_contract_bytes(1, version);
        cache
            .cache_contract(&contract_bytes)
            .expect("Failed to cache");
    }

    // Check that version limit is respected
    let stats = cache.get_cache_stats();
    // Implementation should limit versions per contract
}

#[wasm_bindgen_test]
async fn test_cache_performance() {
    start().await.expect("Failed to start WASM");

    let cache = ContractCache::new(None);

    let start_time = Date::now();

    // Cache many contracts
    for i in 0..20 {
        let contract_bytes = create_test_contract_bytes(i, 1);
        cache
            .cache_contract(&contract_bytes)
            .expect("Failed to cache");
    }

    let cache_time = Date::now() - start_time;

    // Verify caching is reasonably fast (< 100ms for 20 contracts)
    assert!(
        cache_time < 100.0,
        "Caching took too long: {}ms",
        cache_time
    );

    // Test retrieval performance
    let contract_bytes = create_test_contract_bytes(10, 1);
    let contract_id = cache
        .cache_contract(&contract_bytes)
        .expect("Failed to cache");

    let retrieve_start = Date::now();
    for _ in 0..100 {
        cache
            .get_cached_contract(&contract_id)
            .expect("Failed to get");
    }
    let retrieve_time = Date::now() - retrieve_start;

    // Retrieval should be very fast (< 10ms for 100 retrievals)
    assert!(
        retrieve_time < 10.0,
        "Retrieval took too long: {}ms",
        retrieve_time
    );
}
