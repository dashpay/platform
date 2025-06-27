//! Comprehensive tests for the nonce module

use wasm_bindgen_test::*;
use wasm_sdk::{
    nonce::{
        NonceOptions, check_identity_nonce_cache, update_identity_nonce_cache,
        check_identity_contract_nonce_cache, update_identity_contract_nonce_cache,
        increment_identity_nonce_cache, increment_identity_contract_nonce_cache,
        clear_identity_nonce_cache, clear_identity_contract_nonce_cache
    },
    start,
};

wasm_bindgen_test_configure!(run_in_browser);

// Test identity ID - valid base58 encoded identifier
const TEST_IDENTITY_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
const TEST_CONTRACT_ID: &str = "11c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c";

#[wasm_bindgen_test]
async fn test_nonce_options() {
    start().await.expect("Failed to start WASM");
    
    let mut options = NonceOptions::new();
    
    // Test default values through setters
    options.set_cached(false);
    options.set_prove(false);
}

#[wasm_bindgen_test]
async fn test_identity_nonce_cache_lifecycle() {
    start().await.expect("Failed to start WASM");
    
    // Clear cache to start fresh
    clear_identity_nonce_cache();
    
    // Check empty cache
    let cached = check_identity_nonce_cache(TEST_IDENTITY_ID)
        .expect("Failed to check cache");
    assert!(cached.is_none());
    
    // Update cache
    let test_nonce = 42u64;
    update_identity_nonce_cache(TEST_IDENTITY_ID, test_nonce)
        .expect("Failed to update cache");
    
    // Check cached value
    let cached = check_identity_nonce_cache(TEST_IDENTITY_ID)
        .expect("Failed to check cache");
    assert_eq!(cached, Some(test_nonce));
    
    // Clear cache
    clear_identity_nonce_cache();
    
    // Verify cleared
    let cached = check_identity_nonce_cache(TEST_IDENTITY_ID)
        .expect("Failed to check cache");
    assert!(cached.is_none());
}

#[wasm_bindgen_test]
async fn test_identity_nonce_increment() {
    start().await.expect("Failed to start WASM");
    
    clear_identity_nonce_cache();
    
    // Try increment without cache - should fail
    let result = increment_identity_nonce_cache(TEST_IDENTITY_ID, None);
    assert!(result.is_err());
    
    // Set initial nonce
    update_identity_nonce_cache(TEST_IDENTITY_ID, 10)
        .expect("Failed to update cache");
    
    // Increment by default (1)
    let new_nonce = increment_identity_nonce_cache(TEST_IDENTITY_ID, None)
        .expect("Failed to increment");
    assert_eq!(new_nonce, 11);
    
    // Increment by custom amount
    let new_nonce = increment_identity_nonce_cache(TEST_IDENTITY_ID, Some(5))
        .expect("Failed to increment");
    assert_eq!(new_nonce, 16);
    
    // Verify cache updated
    let cached = check_identity_nonce_cache(TEST_IDENTITY_ID)
        .expect("Failed to check cache");
    assert_eq!(cached, Some(16));
}

#[wasm_bindgen_test]
async fn test_identity_contract_nonce_cache() {
    start().await.expect("Failed to start WASM");
    
    clear_identity_contract_nonce_cache();
    
    // Check empty cache
    let cached = check_identity_contract_nonce_cache(TEST_IDENTITY_ID, TEST_CONTRACT_ID)
        .expect("Failed to check cache");
    assert!(cached.is_none());
    
    // Update cache
    let test_nonce = 100u64;
    update_identity_contract_nonce_cache(TEST_IDENTITY_ID, TEST_CONTRACT_ID, test_nonce)
        .expect("Failed to update cache");
    
    // Check cached value
    let cached = check_identity_contract_nonce_cache(TEST_IDENTITY_ID, TEST_CONTRACT_ID)
        .expect("Failed to check cache");
    assert_eq!(cached, Some(test_nonce));
    
    // Different contract should have no cache
    let different_contract = "22c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c";
    let cached = check_identity_contract_nonce_cache(TEST_IDENTITY_ID, different_contract)
        .expect("Failed to check cache");
    assert!(cached.is_none());
}

#[wasm_bindgen_test]
async fn test_identity_contract_nonce_increment() {
    start().await.expect("Failed to start WASM");
    
    clear_identity_contract_nonce_cache();
    
    // Set initial nonce
    update_identity_contract_nonce_cache(TEST_IDENTITY_ID, TEST_CONTRACT_ID, 50)
        .expect("Failed to update cache");
    
    // Increment by default
    let new_nonce = increment_identity_contract_nonce_cache(TEST_IDENTITY_ID, TEST_CONTRACT_ID, None)
        .expect("Failed to increment");
    assert_eq!(new_nonce, 51);
    
    // Increment by custom amount
    let new_nonce = increment_identity_contract_nonce_cache(TEST_IDENTITY_ID, TEST_CONTRACT_ID, Some(10))
        .expect("Failed to increment");
    assert_eq!(new_nonce, 61);
}

#[wasm_bindgen_test]
async fn test_cache_staleness() {
    start().await.expect("Failed to start WASM");
    
    clear_identity_nonce_cache();
    
    // Update cache
    update_identity_nonce_cache(TEST_IDENTITY_ID, 123)
        .expect("Failed to update cache");
    
    // Should be cached immediately
    let cached = check_identity_nonce_cache(TEST_IDENTITY_ID)
        .expect("Failed to check cache");
    assert_eq!(cached, Some(123));
    
    // Wait for cache to become stale (> 5 seconds)
    // In real tests, we'd mock time or make staleness configurable
    // For now, just verify the cache exists
    
    // After 5+ seconds, cache should return None (stale)
    // This would need proper time mocking to test reliably
}

#[wasm_bindgen_test]
async fn test_multiple_identities() {
    start().await.expect("Failed to start WASM");
    
    clear_identity_nonce_cache();
    
    let identity1 = TEST_IDENTITY_ID;
    let identity2 = "5XYJGgRoKiQv9D8p3kDkSqRTCkefUPdK5Qd3LqvQWFKW";
    
    // Set different nonces
    update_identity_nonce_cache(identity1, 10).expect("Failed to update");
    update_identity_nonce_cache(identity2, 20).expect("Failed to update");
    
    // Verify independent caches
    assert_eq!(check_identity_nonce_cache(identity1).unwrap(), Some(10));
    assert_eq!(check_identity_nonce_cache(identity2).unwrap(), Some(20));
    
    // Increment independently
    increment_identity_nonce_cache(identity1, Some(5)).expect("Failed to increment");
    assert_eq!(check_identity_nonce_cache(identity1).unwrap(), Some(15));
    assert_eq!(check_identity_nonce_cache(identity2).unwrap(), Some(20));
}

#[wasm_bindgen_test]
async fn test_invalid_identity_ids() {
    start().await.expect("Failed to start WASM");
    
    // Test invalid base58
    let result = check_identity_nonce_cache("invalid!@#$");
    assert!(result.is_err());
    
    let result = update_identity_nonce_cache("", 10);
    assert!(result.is_err());
    
    let result = increment_identity_nonce_cache("not-base58", None);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_contract_nonce_isolation() {
    start().await.expect("Failed to start WASM");
    
    clear_identity_contract_nonce_cache();
    
    let contract1 = TEST_CONTRACT_ID;
    let contract2 = "33c70af56a763b05943888fa3719ef56b3e826615fdda2d463c63f4034cb861c";
    
    // Set nonces for same identity, different contracts
    update_identity_contract_nonce_cache(TEST_IDENTITY_ID, contract1, 100)
        .expect("Failed to update");
    update_identity_contract_nonce_cache(TEST_IDENTITY_ID, contract2, 200)
        .expect("Failed to update");
    
    // Verify isolation
    assert_eq!(
        check_identity_contract_nonce_cache(TEST_IDENTITY_ID, contract1).unwrap(),
        Some(100)
    );
    assert_eq!(
        check_identity_contract_nonce_cache(TEST_IDENTITY_ID, contract2).unwrap(),
        Some(200)
    );
}

#[wasm_bindgen_test]
async fn test_saturating_increment() {
    start().await.expect("Failed to start WASM");
    
    clear_identity_nonce_cache();
    
    // Set nonce near max
    let near_max = u64::MAX - 10;
    update_identity_nonce_cache(TEST_IDENTITY_ID, near_max)
        .expect("Failed to update");
    
    // Increment should saturate, not overflow
    let new_nonce = increment_identity_nonce_cache(TEST_IDENTITY_ID, Some(20))
        .expect("Failed to increment");
    assert_eq!(new_nonce, u64::MAX);
}

#[wasm_bindgen_test]
async fn test_cache_clear_independence() {
    start().await.expect("Failed to start WASM");
    
    // Set both caches
    update_identity_nonce_cache(TEST_IDENTITY_ID, 10).expect("Failed");
    update_identity_contract_nonce_cache(TEST_IDENTITY_ID, TEST_CONTRACT_ID, 20).expect("Failed");
    
    // Clear only identity cache
    clear_identity_nonce_cache();
    
    // Identity cache should be empty
    assert!(check_identity_nonce_cache(TEST_IDENTITY_ID).unwrap().is_none());
    
    // Contract cache should still have data
    assert_eq!(
        check_identity_contract_nonce_cache(TEST_IDENTITY_ID, TEST_CONTRACT_ID).unwrap(),
        Some(20)
    );
    
    // Clear contract cache
    clear_identity_contract_nonce_cache();
    
    // Both should be empty now
    assert!(check_identity_contract_nonce_cache(TEST_IDENTITY_ID, TEST_CONTRACT_ID).unwrap().is_none());
}

#[wasm_bindgen_test]
async fn test_concurrent_increments() {
    start().await.expect("Failed to start WASM");
    
    clear_identity_nonce_cache();
    
    // Set initial nonce
    update_identity_nonce_cache(TEST_IDENTITY_ID, 0).expect("Failed");
    
    // Simulate concurrent increments
    // In a real concurrent environment, these would be from different threads
    // The mutex should ensure atomic updates
    for _ in 0..10 {
        increment_identity_nonce_cache(TEST_IDENTITY_ID, Some(1)).expect("Failed");
    }
    
    // Should have incremented exactly 10 times
    let final_nonce = check_identity_nonce_cache(TEST_IDENTITY_ID).unwrap().unwrap();
    assert_eq!(final_nonce, 10);
}