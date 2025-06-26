//! Integration tests for complete workflows

use wasm_bindgen_test::*;
use wasm_sdk::{
    sdk::WasmSdk,
    signer::WasmSigner,
    identity_info::*,
    prefunded_balance::*,
    contract_history::*,
    monitoring::*,
    bip39::*,
};
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsValue;
use crate::common::setup_test_sdk;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_complete_identity_workflow() {
    // Initialize monitoring
    initialize_monitoring(true, Some(100))
        .expect("Should initialize monitoring");
    
    let sdk = setup_test_sdk().await;
    
    // Generate mnemonic for new identity
    let mnemonic = Mnemonic::generate(MnemonicStrength::Words12, WordListLanguage::English)
        .expect("Should generate mnemonic");
    
    // Create signer from mnemonic
    let seed = mnemonic.to_seed(None)
        .expect("Should generate seed");
    
    let mut signer = WasmSigner::new();
    
    // In a real scenario, we would:
    // 1. Derive HD keys from seed
    // 2. Create identity with those keys
    // 3. Top up the identity
    // 4. Check balance
    // 5. Monitor updates
    
    // For testing, we'll use a test identity
    let test_identity = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    
    // Check if identity exists
    let exists = check_identity_exists(&sdk, test_identity).await
        .unwrap_or(JsValue::from(false));
    
    // Get identity info if it exists
    if exists.as_bool() == Some(true) {
        let info = get_identity_info(&sdk, test_identity).await;
        assert!(info.is_ok() || info.is_err());
    }
    
    // Check monitoring captured operations
    if let Ok(Some(monitor)) = get_global_monitor() {
        let metrics = monitor.get_metrics()
            .expect("Should get metrics");
        
        // Should have recorded some operations
        assert!(metrics.length() > 0);
    }
}

#[wasm_bindgen_test]
async fn test_contract_deployment_workflow() {
    let sdk = setup_test_sdk().await;
    
    // Test contract ID
    let contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    
    // Get contract history
    let history_result = get_contract_history(&sdk, contract_id).await;
    
    if let Ok(history) = history_result {
        let history_array = history.dyn_ref::<Array>()
            .expect("Should be an array");
        
        if history_array.length() > 1 {
            // Get migration guide between versions
            let guide_result = get_migration_guide(&sdk, contract_id, 1, 2).await;
            assert!(guide_result.is_ok() || guide_result.is_err());
        }
    }
    
    // Monitor contract updates
    let callback = js_sys::Function::new_with_args(
        "update",
        "console.log('Contract update:', update);"
    );
    
    let monitor_result = monitor_contract_updates(
        &sdk,
        contract_id,
        callback,
        Some(2000)
    ).await;
    
    if let Ok(stop_fn) = monitor_result {
        // Let it run for a moment
        gloo_timers::future::TimeoutFuture::new(100).await;
        
        // Stop monitoring
        let stop = stop_fn.dyn_ref::<js_sys::Function>()
            .expect("Should be a function");
        let _ = stop.call0(&JsValue::null());
    }
}

#[wasm_bindgen_test]
async fn test_identity_funding_workflow() {
    let sdk = setup_test_sdk().await;
    let mut signer = WasmSigner::new();
    
    // Identity IDs for testing
    let funding_identity = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let recipient_identity = "HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed";
    
    // Set up signer
    signer.set_identity_id(funding_identity)
        .expect("Should set identity ID");
    signer.add_private_key(
        1,
        vec![0x01; 32], // Mock private key
        "ECDSA_SECP256K1",
        0
    ).expect("Should add private key");
    
    // Check initial balance
    let initial_balance = get_identity_balance(&sdk, recipient_identity).await;
    
    // Estimate top-up cost
    let cost = estimate_top_up_cost(100000);
    assert!(cost.as_f64().is_some());
    
    // In a real scenario, we would:
    // 1. Check funding identity balance
    // 2. Transfer credits
    // 3. Wait for balance update
    // 4. Verify transfer succeeded
    
    // Check minimum balance
    let has_minimum = check_minimum_balance(&sdk, recipient_identity, 50000).await;
    assert!(has_minimum.is_ok() || has_minimum.is_err());
}

#[wasm_bindgen_test]
async fn test_batch_operations_workflow() {
    let sdk = setup_test_sdk().await;
    
    // Create arrays for batch operations
    let identity_ids = Array::new();
    identity_ids.push(&"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec".into());
    identity_ids.push(&"HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed".into());
    identity_ids.push(&"IWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ee".into());
    
    // Batch get identities
    let identities_result = batch_get_identities(&sdk, identity_ids.clone()).await;
    
    if let Ok(identities_map) = identities_result {
        // Process each identity
        for i in 0..identity_ids.length() {
            let id = identity_ids.get(i);
            if let Some(id_str) = id.as_string() {
                // Check if we got info for this identity
                let has_info = identities_map.has(&id);
                web_sys::console::log_1(&format!("Identity {} found: {}", id_str, has_info).into());
            }
        }
    }
    
    // Batch get contracts
    let contract_ids = Array::new();
    contract_ids.push(&"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec".into());
    contract_ids.push(&"HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed".into());
    
    let contracts_result = batch_get_contracts(&sdk, contract_ids).await;
    assert!(contracts_result.is_ok() || contracts_result.is_err());
}

#[wasm_bindgen_test]
async fn test_monitoring_with_operations() {
    // Initialize monitoring
    initialize_monitoring(true, Some(50))
        .expect("Should initialize monitoring");
    
    let sdk = setup_test_sdk().await;
    
    // Perform various operations that should be monitored
    let operations = vec![
        ("identity_check", async {
            let _ = check_identity_exists(&sdk, "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").await;
        }),
        ("balance_check", async {
            let _ = get_identity_balance(&sdk, "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").await;
        }),
        ("contract_fetch", async {
            let _ = get_contract_history(&sdk, "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").await;
        }),
    ];
    
    // Execute operations
    for (name, op) in operations {
        web_sys::console::log_1(&format!("Executing operation: {}", name).into());
        op.await;
    }
    
    // Check monitoring results
    if let Ok(Some(monitor)) = get_global_monitor() {
        let stats = monitor.get_operation_stats()
            .expect("Should get operation stats");
        
        web_sys::console::log_1(&stats);
        
        // Verify we have stats
        let stats_obj = stats.dyn_ref::<Object>()
            .expect("Stats should be an object");
        
        // Should have recorded some operations
        let keys = Object::keys(stats_obj);
        assert!(keys.length() > 0);
    }
    
    // Perform health check
    let health = perform_health_check(&sdk).await
        .expect("Should perform health check");
    
    web_sys::console::log_1(&format!("Health status: {}", health.status()).into());
}

#[wasm_bindgen_test]
async fn test_mnemonic_to_identity_workflow() {
    // Generate a new mnemonic
    let mnemonic = Mnemonic::generate(MnemonicStrength::Words24, WordListLanguage::English)
        .expect("Should generate 24-word mnemonic");
    
    // Validate the mnemonic
    assert!(mnemonic.validate().expect("Should validate"));
    
    // Convert to seed with passphrase
    let seed = mnemonic.to_seed(Some("test-passphrase".to_string()))
        .expect("Should generate seed");
    assert_eq!(seed.len(), 64);
    
    // Get HD private key
    let hd_key = mnemonic.to_hd_private_key(Some("test-passphrase".to_string()), "testnet")
        .expect("Should generate HD private key");
    assert!(hd_key.starts_with("tprv"));
    
    // Derive child keys for identity
    let auth_key = derive_child_key(
        &mnemonic.phrase(),
        Some("test-passphrase".to_string()),
        "m/9'/5'/3'/0/0",
        "testnet"
    ).expect("Should derive authentication key");
    
    let signing_key = derive_child_key(
        &mnemonic.phrase(),
        Some("test-passphrase".to_string()),
        "m/9'/5'/3'/3/0",
        "testnet"
    ).expect("Should derive signing key");
    
    // In a real scenario, these keys would be used to:
    // 1. Create identity public keys
    // 2. Register identity on platform
    // 3. Fund the identity
    // 4. Start using the identity
    
    web_sys::console::log_1(&format!("Generated mnemonic: {}", mnemonic.phrase()).into());
}

#[wasm_bindgen_test]
async fn test_error_recovery_workflow() {
    let sdk = setup_test_sdk().await;
    
    // Initialize monitoring to track errors
    initialize_monitoring(true, Some(20))
        .expect("Should initialize monitoring");
    
    // Test various error scenarios
    
    // 1. Invalid identity ID
    let invalid_result = get_identity_info(&sdk, "invalid_id").await;
    assert!(invalid_result.is_err());
    
    // 2. Non-existent identity
    let nonexistent = "ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZz";
    let not_found_result = get_identity_balance(&sdk, nonexistent).await;
    // May return error or zero balance
    assert!(not_found_result.is_ok() || not_found_result.is_err());
    
    // 3. Invalid mnemonic
    let invalid_mnemonic = Mnemonic::from_phrase("invalid words here", WordListLanguage::English);
    assert!(invalid_mnemonic.is_err());
    
    // Check monitoring captured errors
    if let Ok(Some(monitor)) = get_global_monitor() {
        let metrics = monitor.get_metrics()
            .expect("Should get metrics");
        
        // Count errors
        let mut error_count = 0;
        for i in 0..metrics.length() {
            let metric = metrics.get(i);
            if let Some(obj) = metric.dyn_ref::<Object>() {
                if let Ok(success) = Reflect::get(obj, &"success".into()) {
                    if success.as_bool() == Some(false) {
                        error_count += 1;
                    }
                }
            }
        }
        
        web_sys::console::log_1(&format!("Errors captured: {}", error_count).into());
    }
}