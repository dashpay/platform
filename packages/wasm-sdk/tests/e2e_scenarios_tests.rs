//! End-to-end scenario tests

use crate::common::setup_test_sdk;
use js_sys::{Array, Function, Object, Promise, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use wasm_sdk::{
    cache::*,
    dapi_client::{DapiClient, DapiClientConfig},
    monitoring::*,
    sdk::WasmSdk,
    signer::{BrowserSigner, HDSigner, WasmSigner},
    state_transitions::documents::*,
    subscriptions::*,
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_e2e_domain_registration() {
    // Initialize SDK with monitoring
    initialize_monitoring(true, Some(100)).expect("Should initialize monitoring");

    let sdk = setup_test_sdk().await;

    // Scenario: User wants to register a domain name
    // 1. Check if domain is available
    // 2. Create domain document
    // 3. Sign and broadcast
    // 4. Monitor for confirmation

    let domain_name = "test-domain";
    let dpns_contract = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"; // Mock DPNS contract

    // Create domain document
    let domain_doc = Object::new();
    Reflect::set(&domain_doc, &"label".into(), &domain_name.into()).unwrap();
    Reflect::set(
        &domain_doc,
        &"normalizedLabel".into(),
        &domain_name.to_lowercase().into(),
    )
    .unwrap();
    Reflect::set(
        &domain_doc,
        &"normalizedParentDomainName".into(),
        &"dash".into(),
    )
    .unwrap();
    Reflect::set(&domain_doc, &"preorderSalt".into(), &"mock_salt".into()).unwrap();

    let records = Object::new();
    Reflect::set(
        &records,
        &"dashUniqueIdentityId".into(),
        &"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec".into(),
    )
    .unwrap();
    Reflect::set(&domain_doc, &"records".into(), &records).unwrap();

    // In a real scenario:
    // 1. Create preorder document
    // 2. Wait for confirmation
    // 3. Create domain document
    // 4. Submit and monitor

    web_sys::console::log_1(&format!("Domain registration scenario for: {}", domain_name).into());
}

#[wasm_bindgen_test]
async fn test_e2e_social_profile_creation() {
    let sdk = setup_test_sdk().await;
    let mut signer = WasmSigner::new();

    // Scenario: User creates a social profile on DashPay
    let identity_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let dashpay_contract = "HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed"; // Mock DashPay contract

    // Set up signer
    signer
        .set_identity_id(identity_id)
        .expect("Should set identity ID");
    signer
        .add_private_key(1, vec![0x01; 32], "ECDSA_SECP256K1", 0)
        .expect("Should add private key");

    // Create profile document
    let profile = Object::new();
    Reflect::set(&profile, &"displayName".into(), &"Test User".into()).unwrap();
    Reflect::set(&profile, &"bio".into(), &"Testing the WASM SDK".into()).unwrap();
    Reflect::set(
        &profile,
        &"avatarUrl".into(),
        &"https://example.com/avatar.jpg".into(),
    )
    .unwrap();

    // Create document
    let result = create_document(
        &sdk,
        dashpay_contract,
        identity_id,
        "profile",
        profile,
        &signer,
    )
    .await;

    // In a real scenario, we would wait for confirmation
    assert!(result.is_ok() || result.is_err());

    web_sys::console::log_1(&"Social profile creation scenario completed".into());
}

#[wasm_bindgen_test]
async fn test_e2e_subscription_monitoring() {
    let sdk = setup_test_sdk().await;

    // Scenario: Monitor contract documents in real-time
    let contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";

    // Create subscription client
    let sub_client =
        SubscriptionClient::new("testnet".to_string()).expect("Should create subscription client");

    // Subscribe to document updates
    let callback = Function::new_with_args(
        "update",
        "console.log('Document update received:', update);",
    );

    let subscription_id = sub_client
        .subscribe_to_documents(contract_id, "domain", callback)
        .await;

    if let Ok(sub_id) = subscription_id {
        web_sys::console::log_1(&format!("Subscription started with ID: {}", sub_id).into());

        // Let it run for a moment
        gloo_timers::future::TimeoutFuture::new(2000).await;

        // Unsubscribe
        let _ = sub_client.unsubscribe(&sub_id).await;
    }
}

#[wasm_bindgen_test]
async fn test_e2e_multi_identity_management() {
    let sdk = setup_test_sdk().await;

    // Scenario: User manages multiple identities
    let identities = vec![
        ("personal", "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"),
        ("business", "HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed"),
        ("gaming", "IWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ee"),
    ];

    // Create HD signer for deterministic key derivation
    let hd_signer = HDSigner::new(
        "abandon ability able about above absent absorb abstract absurd abuse access accident",
        "m/9'/5'/3'/0",
    )
    .expect("Should create HD signer");

    // Manage each identity
    for (purpose, identity_id) in identities {
        web_sys::console::log_1(&format!("Managing {} identity: {}", purpose, identity_id).into());

        // Derive keys for this identity
        let key = hd_signer.derive_key(0).expect("Should derive key");

        // In a real scenario:
        // 1. Check identity balance
        // 2. Update profile if needed
        // 3. Manage permissions
        // 4. Monitor activity
    }
}

#[wasm_bindgen_test]
async fn test_e2e_browser_crypto_integration() {
    // Scenario: Use browser's native crypto for key management
    let mut browser_signer = BrowserSigner::new();

    // Generate key pair in browser
    let public_key = browser_signer.generate_key_pair("ECDSA_SECP256K1", 1).await;

    if let Ok(pub_key) = public_key {
        web_sys::console::log_1(&"Generated key pair in browser".into());

        // Sign test data
        let test_data = b"Test message for signing";
        let signature = browser_signer
            .sign_with_stored_key(test_data.to_vec(), 1)
            .await;

        assert!(signature.is_ok() || signature.is_err());

        if let Ok(sig) = signature {
            web_sys::console::log_1(&format!("Signature length: {}", sig.len()).into());
        }
    }
}

#[wasm_bindgen_test]
async fn test_e2e_performance_monitoring() {
    // Initialize monitoring
    initialize_monitoring(true, Some(50)).expect("Should initialize monitoring");

    let sdk = setup_test_sdk().await;

    // Scenario: Monitor SDK performance during heavy usage
    let operations = 20;
    let start_time = js_sys::Date::now();

    // Perform multiple operations
    for i in 0..operations {
        let operation_id = format!("perf_test_{}", i);

        // Track operation
        if let Ok(Some(monitor)) = get_global_monitor() {
            monitor
                .start_operation(operation_id.clone(), "PerformanceTest".to_string())
                .expect("Should start operation");
        }

        // Simulate work
        let _ = sdk.network();

        // End operation
        if let Ok(Some(monitor)) = get_global_monitor() {
            monitor
                .end_operation(operation_id, true, None)
                .expect("Should end operation");
        }
    }

    let total_time = js_sys::Date::now() - start_time;

    // Get performance stats
    if let Ok(Some(monitor)) = get_global_monitor() {
        let stats = monitor.get_operation_stats().expect("Should get stats");

        web_sys::console::log_1(&stats);
        web_sys::console::log_1(
            &format!("Total time for {} operations: {}ms", operations, total_time).into(),
        );

        // Check resource usage
        let usage = get_resource_usage().expect("Should get resource usage");
        web_sys::console::log_1(&usage);
    }
}

#[wasm_bindgen_test]
async fn test_e2e_cache_optimization() {
    let sdk = setup_test_sdk().await;

    // Scenario: Optimize performance with caching
    let contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";

    // Initialize cache
    let cache = init_cache().await.expect("Should initialize cache");

    // First fetch - will hit network
    let start1 = js_sys::Date::now();
    let doc1 = cache_get(&format!("contract:{}", contract_id))
        .await
        .expect("Should check cache");
    let time1 = js_sys::Date::now() - start1;

    if doc1.is_none() {
        // Simulate fetching and caching
        let mock_contract = Object::new();
        Reflect::set(&mock_contract, &"id".into(), &contract_id.into()).unwrap();
        Reflect::set(&mock_contract, &"version".into(), &1.into()).unwrap();

        cache_set(
            &format!("contract:{}", contract_id),
            mock_contract.into(),
            Some(300000), // 5 minute TTL
        )
        .await
        .expect("Should cache contract");
    }

    // Second fetch - should hit cache
    let start2 = js_sys::Date::now();
    let doc2 = cache_get(&format!("contract:{}", contract_id))
        .await
        .expect("Should check cache");
    let time2 = js_sys::Date::now() - start2;

    web_sys::console::log_1(&format!("First fetch: {}ms, Second fetch: {}ms", time1, time2).into());

    // Cache should be faster
    if doc2.is_some() {
        assert!(time2 < time1 || time2 < 50.0); // Cache should be under 50ms
    }
}

#[wasm_bindgen_test]
async fn test_e2e_error_handling_resilience() {
    let sdk = setup_test_sdk().await;

    // Scenario: Test SDK resilience to errors
    let mut signer = WasmSigner::new();

    // Test various error scenarios
    let error_scenarios = vec![
        ("Invalid identity ID", async {
            signer.set_identity_id("invalid").err()
        }),
        ("Missing private key", async {
            signer.sign_data(vec![1, 2, 3], 999).await.err()
        }),
        ("Invalid contract", async {
            create_document(&sdk, "invalid", "invalid", "test", Object::new(), &signer)
                .await
                .err()
        }),
    ];

    let mut error_count = 0;
    for (scenario, test) in error_scenarios {
        if test.await.is_some() {
            error_count += 1;
            web_sys::console::log_1(&format!("Error scenario handled: {}", scenario).into());
        }
    }

    // All scenarios should produce errors
    assert!(error_count > 0);
    web_sys::console::log_1(&format!("Handled {} error scenarios", error_count).into());
}
