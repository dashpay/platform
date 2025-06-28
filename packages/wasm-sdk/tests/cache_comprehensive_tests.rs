//! Comprehensive cache module tests

use js_sys::Date;
use wasm_bindgen_test::*;
use wasm_sdk::{
    cache::{create_document_cache_key, create_token_balance_cache_key, WasmCacheManager},
    start,
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_cache_size_limits() {
    start().await.expect("Failed to start WASM");

    let mut cache = WasmCacheManager::new();

    // Set small size limits for testing
    cache.set_max_sizes(5, 5, 5, 5, 5, 5);

    // Add more items than the limit
    for i in 0..10 {
        let data = vec![i as u8; 100];
        cache.cache_contract(&format!("contract_{}", i), data);
    }

    // Check that cache respects size limit
    let stats = cache.get_stats().expect("Failed to get stats");
    let contracts_size = js_sys::Reflect::get(&stats, &"contracts".into())
        .expect("Failed to get contracts size")
        .as_f64()
        .expect("Not a number");

    assert!(contracts_size <= 5.0, "Cache should not exceed size limit");
}

#[wasm_bindgen_test]
async fn test_cache_ttl_expiration() {
    start().await.expect("Failed to start WASM");

    let mut cache = WasmCacheManager::new();

    // Set very short TTL (1ms)
    cache.set_ttls(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);

    // Add items
    cache.cache_contract("test_contract", vec![1, 2, 3]);
    cache.cache_identity("test_identity", vec![4, 5, 6]);

    // Wait for expiration
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 10)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();

    // Cleanup and check items are gone
    cache.cleanup_expired();

    assert!(cache.get_cached_contract("test_contract").is_none());
    assert!(cache.get_cached_identity("test_identity").is_none());
}

#[wasm_bindgen_test]
async fn test_cache_lru_eviction() {
    start().await.expect("Failed to start WASM");

    let mut cache = WasmCacheManager::new();
    cache.set_max_sizes(3, 3, 3, 3, 3, 3);

    // Add items in order
    cache.cache_contract("contract_1", vec![1]);
    cache.cache_contract("contract_2", vec![2]);
    cache.cache_contract("contract_3", vec![3]);

    // Access contract_1 to make it recently used
    let _ = cache.get_cached_contract("contract_1");

    // Add one more item, should evict contract_2 (oldest non-accessed)
    cache.cache_contract("contract_4", vec![4]);

    assert!(
        cache.get_cached_contract("contract_1").is_some(),
        "Recently accessed item should remain"
    );
    assert!(
        cache.get_cached_contract("contract_3").is_some(),
        "Recent item should remain"
    );
    assert!(
        cache.get_cached_contract("contract_4").is_some(),
        "New item should be added"
    );
}

#[wasm_bindgen_test]
async fn test_cache_clear_operations() {
    start().await.expect("Failed to start WASM");

    let cache = WasmCacheManager::new();

    // Add items to different caches
    cache.cache_contract("contract", vec![1]);
    cache.cache_identity("identity", vec![2]);
    cache.cache_document("doc", vec![3]);
    cache.cache_token("token", vec![4]);

    // Clear specific cache
    cache.clear_cache("contracts");

    assert!(cache.get_cached_contract("contract").is_none());
    assert!(cache.get_cached_identity("identity").is_some());
    assert!(cache.get_cached_document("doc").is_some());
    assert!(cache.get_cached_token("token").is_some());

    // Clear all
    cache.clear_all();

    assert!(cache.get_cached_identity("identity").is_none());
    assert!(cache.get_cached_document("doc").is_none());
    assert!(cache.get_cached_token("token").is_none());
}

#[wasm_bindgen_test]
async fn test_cache_key_generation() {
    // Test document cache key
    let doc_key = create_document_cache_key("contract123", "user", "doc456");
    assert_eq!(doc_key, "contract123_user_doc456");

    // Test token balance cache key
    let token_key = create_token_balance_cache_key("token789", "identity123");
    assert_eq!(token_key, "token_balance_token789_identity123");
}

#[wasm_bindgen_test]
async fn test_cache_stats() {
    start().await.expect("Failed to start WASM");

    let cache = WasmCacheManager::new();

    // Add items
    cache.cache_contract("c1", vec![1]);
    cache.cache_contract("c2", vec![2]);
    cache.cache_identity("i1", vec![3]);
    cache.cache_document("d1", vec![4]);

    let stats = cache.get_stats().expect("Failed to get stats");

    // Check individual counts
    let contracts = js_sys::Reflect::get(&stats, &"contracts".into())
        .unwrap()
        .as_f64()
        .unwrap();
    let identities = js_sys::Reflect::get(&stats, &"identities".into())
        .unwrap()
        .as_f64()
        .unwrap();
    let documents = js_sys::Reflect::get(&stats, &"documents".into())
        .unwrap()
        .as_f64()
        .unwrap();
    let total = js_sys::Reflect::get(&stats, &"totalEntries".into())
        .unwrap()
        .as_f64()
        .unwrap();

    assert_eq!(contracts, 2.0);
    assert_eq!(identities, 1.0);
    assert_eq!(documents, 1.0);
    assert_eq!(total, 4.0);

    // Check max sizes are included
    assert!(js_sys::Reflect::has(&stats, &"maxContracts".into()).unwrap());
    assert!(js_sys::Reflect::has(&stats, &"maxIdentities".into()).unwrap());
}

#[wasm_bindgen_test]
async fn test_auto_cleanup() {
    start().await.expect("Failed to start WASM");

    let mut cache = WasmCacheManager::new();

    // Set very short TTL
    cache.set_ttls(5.0, 5.0, 5.0, 5.0, 5.0, 5.0);

    // Add item
    cache.cache_contract("test", vec![1, 2, 3]);

    // Start auto cleanup with 10ms interval
    cache.start_auto_cleanup(10);

    // Wait for auto cleanup to run
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 20)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();

    // Item should be gone due to auto cleanup
    assert!(cache.get_cached_contract("test").is_none());

    // Stop auto cleanup
    cache.stop_auto_cleanup();
}

#[wasm_bindgen_test]
async fn test_concurrent_access() {
    start().await.expect("Failed to start WASM");

    let cache = WasmCacheManager::new();

    // Simulate concurrent access by rapidly adding and reading
    for i in 0..100 {
        let key = format!("item_{}", i);
        cache.cache_contract(&key, vec![i as u8]);

        // Immediately read back
        let result = cache.get_cached_contract(&key);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![i as u8]);
    }

    // Check final count
    let stats = cache.get_stats().unwrap();
    let contracts = js_sys::Reflect::get(&stats, &"contracts".into())
        .unwrap()
        .as_f64()
        .unwrap();

    // Should be limited by max size (100 default)
    assert_eq!(contracts, 100.0);
}
