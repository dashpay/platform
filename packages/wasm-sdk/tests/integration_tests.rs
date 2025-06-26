//! Integration tests for WASM SDK
//!
//! These tests verify the integration of multiple components working together
//! in a WASM environment.

mod common;
use common::*;
use wasm_bindgen_test::*;
use wasm_sdk::{
    cache::WasmCacheManager,
    context_provider::ContextProvider,
    fetch::{fetch_identity, fetch_data_contract, fetch_documents, FetchOptions},
    optimize::{FeatureFlags, PerformanceMonitor},
    query::DocumentQuery,
    request_settings::RequestSettings,
    sdk::WasmSdk,
    signer::WasmSigner,
    state_transitions::{
        broadcast::broadcast_state_transition,
        document::DocumentBatchBuilder,
        identity::put_identity,
    },
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_full_identity_workflow() {
    let sdk = setup_test_sdk().await;
    let monitor = PerformanceMonitor::new();
    monitor.mark("start");
    
    // Create and configure signer
    let signer = WasmSigner::new();
    let identity_id = test_identity_id();
    signer.set_identity_id(&identity_id);
    signer.add_private_key(0, test_private_key(), "ECDSA_SECP256K1", 0).unwrap();
    
    monitor.mark("signer_setup");
    
    // Create identity state transition
    let public_keys = vec![test_public_key()];
    let asset_lock_proof = test_asset_lock_proof();
    
    let transition = put_identity(
        asset_lock_proof,
        public_keys,
        None,
        0
    );
    
    monitor.mark("transition_created");
    
    // Sign the transition
    let signed = signer.sign_data(transition.unwrap(), 0).await;
    assert!(signed.is_ok(), "Should sign identity transition");
    
    monitor.mark("transition_signed");
    
    // Broadcast (would actually submit to network)
    let request_settings = RequestSettings::new();
    let broadcast_result = broadcast_state_transition(
        &sdk,
        signed.unwrap(),
        Some(request_settings)
    ).await;
    
    monitor.mark("broadcast_complete");
    
    // Fetch the identity back
    let fetch_result = fetch_identity(&sdk, &identity_id, None).await;
    assert!(fetch_result.is_ok(), "Should fetch identity");
    
    monitor.mark("identity_fetched");
    
    // Check performance
    let report = monitor.get_report();
    console_log!("{}", report);
}

#[wasm_bindgen_test]
async fn test_document_management_workflow() {
    let sdk = setup_test_sdk().await;
    let cache = WasmCacheManager::new();
    
    // Setup identity and contract
    let owner_id = test_identity_id();
    let contract_id = test_contract_id();
    
    // Cache the contract for faster access
    cache.cache_contract(&contract_id, vec![1, 2, 3, 4, 5]);
    
    // Create document batch
    let batch_builder = DocumentBatchBuilder::new(&owner_id).unwrap();
    let mut batch = batch_builder;
    
    // Create multiple documents
    for i in 0..5 {
        let data = js_sys::Object::new();
        js_sys::Reflect::set(&data, &"index".into(), &i.into()).unwrap();
        js_sys::Reflect::set(&data, &"title".into(), &format!("Document {}", i).into()).unwrap();
        js_sys::Reflect::set(&data, &"content".into(), &"Test content".into()).unwrap();
        
        batch.add_create_document(
            &contract_id,
            "post",
            &format!("doc{}", i),
            data.into()
        ).unwrap();
    }
    
    // Build and sign the batch
    let transition = batch.build(0).unwrap();
    
    // Create query to fetch documents
    let mut query = DocumentQuery::new(&contract_id, "post").unwrap();
    query.add_order_by("index", true);
    query.set_limit(10);
    
    // Fetch documents with caching
    let where_clause = js_sys::Object::new();
    let fetch_options = FetchOptions::new();
    
    let documents = fetch_documents(
        &sdk,
        &contract_id,
        "post",
        where_clause.into(),
        Some(fetch_options)
    ).await;
    
    assert!(documents.is_ok(), "Should fetch documents");
    
    // Check cache stats
    let stats = cache.get_stats();
    let contracts = js_sys::Reflect::get(&stats, &"contracts".into()).unwrap();
    assert_eq!(contracts.as_f64().unwrap() as u32, 1, "Should have cached contract");
}

#[wasm_bindgen_test]
async fn test_optimized_sdk_with_minimal_features() {
    // Create SDK with minimal features for smaller bundle size
    let mut feature_flags = FeatureFlags::minimal();
    feature_flags.set_enable_identities(true);
    feature_flags.set_enable_documents(true);
    
    let sdk = WasmSdk::new_with_features("testnet".to_string(), None, feature_flags);
    assert!(sdk.is_ok(), "Should create SDK with minimal features");
    
    let minimal_sdk = sdk.unwrap();
    
    // Verify disabled features return appropriate errors
    let token_result = wasm_sdk::token::mint_token(
        &minimal_sdk,
        &test_identity_id(),
        &test_contract_id(),
        1000,
        &test_identity_id(),
        0,
        0
    ).await;
    
    // This should fail as tokens are disabled
    assert!(token_result.is_err(), "Token operations should fail with minimal features");
}

#[wasm_bindgen_test]
async fn test_context_provider_integration() {
    let sdk = setup_test_sdk().await;
    let provider = ContextProvider::new(&sdk);
    
    // Set some context data
    let context_data = js_sys::Object::new();
    js_sys::Reflect::set(&context_data, &"user_id".into(), &test_identity_id().into()).unwrap();
    js_sys::Reflect::set(&context_data, &"network".into(), &"testnet".into()).unwrap();
    js_sys::Reflect::set(&context_data, &"timestamp".into(), &js_sys::Date::now().into()).unwrap();
    
    provider.set_context("test_context", context_data.into());
    
    // Retrieve context
    let retrieved = provider.get_context("test_context");
    assert!(retrieved.is_some(), "Should retrieve context");
    
    let ctx = retrieved.unwrap();
    let user_id = js_sys::Reflect::get(&ctx, &"user_id".into()).unwrap();
    assert_eq!(user_id.as_string().unwrap(), test_identity_id());
}

#[wasm_bindgen_test]
async fn test_retry_logic_with_request_settings() {
    let sdk = setup_test_sdk().await;
    
    // Configure aggressive retry settings
    let mut settings = RequestSettings::new();
    settings.set_timeout(1000); // 1 second timeout
    settings.set_retries(3);
    settings.set_retry_delay(100); // 100ms between retries
    
    // Attempt to fetch non-existent identity (should retry and fail)
    let start = js_sys::Date::now();
    let result = fetch_identity(&sdk, "non_existent_id", Some(settings)).await;
    let duration = js_sys::Date::now() - start;
    
    assert!(result.is_err(), "Should fail to fetch non-existent identity");
    // With 3 retries and 100ms delay, should take at least 200ms
    assert!(duration >= 200.0, "Should respect retry delays");
}

#[wasm_bindgen_test]
async fn test_concurrent_operations() {
    let sdk = setup_test_sdk().await;
    let cache = WasmCacheManager::new();
    
    // Create multiple async operations
    let identity_ids = vec![
        test_identity_id(),
        "identity2",
        "identity3",
    ];
    
    let contract_ids = vec![
        test_contract_id(),
        "contract2",
        "contract3",
    ];
    
    // Cache some data
    for (i, id) in identity_ids.iter().enumerate() {
        cache.cache_identity(id, vec![i as u8; 32]);
    }
    
    for (i, id) in contract_ids.iter().enumerate() {
        cache.cache_contract(id, vec![(i + 10) as u8; 32]);
    }
    
    // Verify all cached correctly
    let stats = cache.get_stats();
    let identities = js_sys::Reflect::get(&stats, &"identities".into()).unwrap();
    let contracts = js_sys::Reflect::get(&stats, &"contracts".into()).unwrap();
    
    assert_eq!(identities.as_f64().unwrap() as u32, 3);
    assert_eq!(contracts.as_f64().unwrap() as u32, 3);
}

#[wasm_bindgen_test]
async fn test_error_propagation_across_layers() {
    let sdk = setup_test_sdk().await;
    
    // Test invalid contract ID format
    let invalid_query = DocumentQuery::new("invalid_contract_id", "doc_type");
    assert!(invalid_query.is_err(), "Should fail with invalid contract ID");
    
    // Test invalid identity transition
    let invalid_transition = put_identity(
        vec![], // Empty asset lock proof
        vec![], // No public keys
        None,
        0
    );
    assert!(invalid_transition.is_err(), "Should fail with invalid parameters");
    
    // Test invalid broadcast
    let broadcast_result = broadcast_state_transition(
        &sdk,
        vec![], // Empty transition
        None
    ).await;
    assert!(broadcast_result.is_err(), "Should fail to broadcast empty transition");
}

#[wasm_bindgen_test]
async fn test_memory_optimization() {
    use wasm_sdk::optimize::{MemoryOptimizer, optimize_uint8_array};
    
    let mut optimizer = MemoryOptimizer::new();
    
    // Create large data arrays
    let large_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
    
    // Track allocation
    optimizer.track_allocation(large_data.len());
    
    // Optimize the array
    let optimized = optimize_uint8_array(&large_data);
    
    // Verify optimization
    assert_eq!(optimized.length(), large_data.len() as u32);
    
    let stats = optimizer.get_stats();
    assert!(stats.contains("10000"), "Should track large allocation");
}

#[wasm_bindgen_test]
async fn test_complete_application_flow() {
    // This test simulates a complete application flow
    let monitor = PerformanceMonitor::new();
    monitor.mark("app_start");
    
    // 1. Initialize SDK with optimized features
    let mut features = FeatureFlags::new();
    features.set_enable_groups(false); // Disable unused features
    features.set_enable_voting(false);
    
    let sdk = WasmSdk::new_with_features("testnet".to_string(), None, features).unwrap();
    monitor.mark("sdk_initialized");
    
    // 2. Setup caching
    let cache = WasmCacheManager::new();
    cache.set_ttls(
        3600,  // contracts: 1 hour
        1800,  // identities: 30 minutes
        300,   // documents: 5 minutes
        3600,  // tokens: 1 hour
        7200,  // quorum keys: 2 hours
        60     // metadata: 1 minute
    );
    monitor.mark("cache_configured");
    
    // 3. Create and setup identity
    let signer = WasmSigner::new();
    let identity_id = test_identity_id();
    signer.set_identity_id(&identity_id);
    signer.add_private_key(0, test_private_key(), "ECDSA_SECP256K1", 0).unwrap();
    monitor.mark("identity_setup");
    
    // 4. Create data contract
    let contract_id = test_contract_id();
    cache.cache_contract(&contract_id, vec![1, 2, 3, 4, 5]);
    monitor.mark("contract_cached");
    
    // 5. Create and query documents
    let mut batch = DocumentBatchBuilder::new(&identity_id).unwrap();
    
    // Add sample documents
    for i in 0..3 {
        let doc_data = js_sys::Object::new();
        js_sys::Reflect::set(&doc_data, &"id".into(), &i.into()).unwrap();
        js_sys::Reflect::set(&doc_data, &"type".into(), &"message".into()).unwrap();
        js_sys::Reflect::set(&doc_data, &"content".into(), &format!("Message {}", i).into()).unwrap();
        js_sys::Reflect::set(&doc_data, &"timestamp".into(), &js_sys::Date::now().into()).unwrap();
        
        batch.add_create_document(
            &contract_id,
            "message",
            &format!("msg_{}", i),
            doc_data.into()
        ).unwrap();
    }
    monitor.mark("documents_prepared");
    
    // 6. Build state transition
    let transition = batch.build(0).unwrap();
    monitor.mark("transition_built");
    
    // 7. Sign transition
    let signed = signer.sign_data(transition, 0).await.unwrap();
    monitor.mark("transition_signed");
    
    // 8. Prepare for broadcast with retry settings
    let mut settings = RequestSettings::new();
    settings.set_timeout(5000);
    settings.set_retries(2);
    monitor.mark("broadcast_configured");
    
    // 9. Generate performance report
    let report = monitor.get_report();
    console_log!("Application Flow Performance:\n{}", report);
    
    // 10. Verify cache effectiveness
    let cache_stats = cache.get_stats();
    let total_entries = js_sys::Reflect::get(&cache_stats, &"totalEntries".into()).unwrap();
    assert!(total_entries.as_f64().unwrap() > 0.0, "Cache should contain entries");
    
    // 11. Get optimization recommendations
    let recommendations = wasm_sdk::optimize::get_optimization_recommendations();
    assert!(recommendations.length() > 0, "Should provide optimization recommendations");
    
    console_log!("Test completed successfully!");
}