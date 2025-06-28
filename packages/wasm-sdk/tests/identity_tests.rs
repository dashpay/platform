//! Identity management tests

mod common;
use common::*;
use wasm_bindgen_test::*;
use wasm_sdk::{
    asset_lock::{create_identity_with_asset_lock, validate_asset_lock_proof, AssetLockProof},
    fetch::{fetch_identity, FetchOptions},
    fetch_unproved::fetch_identity_unproved,
    identity_info::{
        check_identity_balance, estimate_credits_needed, fetch_identity_balance,
        fetch_identity_info,
    },
    nonce::{get_identity_nonce, increment_identity_nonce},
    state_transitions::identity::{create_identity, topup_identity, update_identity},
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_asset_lock_proof_creation() {
    let transaction = test_transaction_bytes();
    let instant_lock = test_instant_lock_bytes();

    // Test instant asset lock proof
    let instant_proof =
        AssetLockProof::create_instant(transaction.clone(), 0, instant_lock.clone());
    assert!(
        instant_proof.is_ok(),
        "Should create instant asset lock proof"
    );

    let proof = instant_proof.unwrap();
    assert_eq!(proof.proof_type(), "instant");
    assert_eq!(proof.transaction(), transaction);
    assert_eq!(proof.output_index(), 0);
    assert_eq!(proof.instant_lock(), Some(instant_lock));

    // Test chain asset lock proof
    let chain_proof = AssetLockProof::create_chain(transaction.clone(), 1);
    assert!(chain_proof.is_ok(), "Should create chain asset lock proof");

    let proof = chain_proof.unwrap();
    assert_eq!(proof.proof_type(), "chain");
    assert_eq!(proof.output_index(), 1);
    assert!(proof.instant_lock().is_none());
}

#[wasm_bindgen_test]
async fn test_asset_lock_proof_serialization() {
    let transaction = test_transaction_bytes();
    let instant_lock = test_instant_lock_bytes();

    let proof = AssetLockProof::create_instant(transaction, 0, instant_lock)
        .expect("Failed to create proof");

    // Test serialization
    let bytes = proof.to_bytes();
    assert!(bytes.is_ok(), "Should serialize proof");

    // Test deserialization
    let deserialized = AssetLockProof::from_bytes(&bytes.unwrap());
    assert!(deserialized.is_ok(), "Should deserialize proof");

    let proof2 = deserialized.unwrap();
    assert_eq!(proof.proof_type(), proof2.proof_type());
    assert_eq!(proof.transaction(), proof2.transaction());
    assert_eq!(proof.output_index(), proof2.output_index());
}

#[wasm_bindgen_test]
async fn test_validate_asset_lock_proof() {
    let transaction = test_transaction_bytes();
    let instant_lock = test_instant_lock_bytes();

    let proof = AssetLockProof::create_instant(transaction, 0, instant_lock)
        .expect("Failed to create proof");

    // Test validation without identity ID
    let valid = validate_asset_lock_proof(&proof, None);
    assert!(valid.is_ok(), "Should validate proof");
    assert!(valid.unwrap(), "Proof should be valid");

    // Test validation with identity ID
    let valid_with_id = validate_asset_lock_proof(&proof, Some(test_identity_id()));
    assert!(valid_with_id.is_ok(), "Should validate proof with ID");
}

#[wasm_bindgen_test]
async fn test_create_identity_state_transition() {
    let asset_lock_proof = vec![1, 2, 3, 4, 5];
    let public_keys = js_sys::Array::new();

    // Create a public key object
    let key_obj = js_sys::Object::new();
    js_sys::Reflect::set(&key_obj, &"id".into(), &0.into()).unwrap();
    js_sys::Reflect::set(&key_obj, &"type".into(), &0.into()).unwrap();
    js_sys::Reflect::set(&key_obj, &"purpose".into(), &0.into()).unwrap();
    js_sys::Reflect::set(&key_obj, &"securityLevel".into(), &0.into()).unwrap();
    js_sys::Reflect::set(
        &key_obj,
        &"data".into(),
        &js_sys::Uint8Array::from(&test_public_key()[..]),
    )
    .unwrap();
    js_sys::Reflect::set(&key_obj, &"readOnly".into(), &false.into()).unwrap();

    public_keys.push(&key_obj);

    let result = create_identity(asset_lock_proof, public_keys.into());
    assert!(result.is_ok(), "Should create identity state transition");
    assert!(
        !result.unwrap().is_empty(),
        "State transition should not be empty"
    );
}

#[wasm_bindgen_test]
async fn test_update_identity_state_transition() {
    let identity_id = test_identity_id();
    let revision = 2u64;
    let add_keys = js_sys::Array::new();
    let disable_keys = js_sys::Array::new();
    disable_keys.push(&1.into());
    disable_keys.push(&2.into());

    let result = update_identity(
        &identity_id,
        revision,
        add_keys.into(),
        disable_keys.into(),
        None,
        0,
    );
    assert!(
        result.is_ok(),
        "Should create update identity state transition"
    );
}

#[wasm_bindgen_test]
async fn test_topup_identity_state_transition() {
    let identity_id = test_identity_id();
    let asset_lock_proof = vec![1, 2, 3, 4, 5];

    let result = topup_identity(&identity_id, asset_lock_proof);
    assert!(
        result.is_ok(),
        "Should create topup identity state transition"
    );
}

#[wasm_bindgen_test]
async fn test_fetch_identity() {
    let sdk = setup_test_sdk().await;
    let identity_id = test_identity_id();

    // Test basic fetch
    let result = fetch_identity(&sdk, &identity_id, None).await;
    assert!(result.is_ok(), "Should fetch identity");

    // Test fetch with options
    let options = FetchOptions::new();
    let result_with_options = fetch_identity(&sdk, &identity_id, Some(options)).await;
    assert!(
        result_with_options.is_ok(),
        "Should fetch identity with options"
    );
}

#[wasm_bindgen_test]
async fn test_fetch_identity_unproved() {
    let sdk = setup_test_sdk().await;
    let identity_id = test_identity_id();

    let result = fetch_identity_unproved(&sdk, &identity_id, None).await;
    assert!(result.is_ok(), "Should fetch identity without proof");
}

#[wasm_bindgen_test]
async fn test_identity_balance() {
    let sdk = setup_test_sdk().await;
    let identity_id = test_identity_id();

    // Test fetch balance
    let balance = fetch_identity_balance(&sdk, &identity_id).await;
    assert!(balance.is_ok(), "Should fetch identity balance");

    let bal = balance.unwrap();
    assert!(bal.confirmed() >= 0);
    assert!(bal.unconfirmed() >= 0);
    assert_eq!(bal.total(), bal.confirmed() + bal.unconfirmed());

    // Test check balance
    let has_balance = check_identity_balance(&sdk, &identity_id, 100, false).await;
    assert!(has_balance.is_ok(), "Should check identity balance");
}

#[wasm_bindgen_test]
async fn test_identity_info() {
    let sdk = setup_test_sdk().await;
    let identity_id = test_identity_id();

    let info = fetch_identity_info(&sdk, &identity_id).await;
    assert!(info.is_ok(), "Should fetch identity info");

    let identity_info = info.unwrap();
    assert_eq!(identity_info.id(), identity_id);
    assert!(identity_info.balance().confirmed() >= 0);
    assert!(identity_info.revision().revision() >= 0);
}

#[wasm_bindgen_test]
async fn test_estimate_credits() {
    // Test various operation types
    let operations = vec![
        ("document_create", Some(1024), 1000),
        ("document_update", Some(512), 500),
        ("document_delete", None, 200),
        ("identity_update", None, 2000),
        ("identity_topup", None, 100),
        ("contract_create", Some(2048), 5000),
        ("contract_update", Some(1024), 3000),
    ];

    for (op_type, data_size, expected_base) in operations {
        let credits = estimate_credits_needed(op_type, data_size.map(|s| s as u32));
        assert!(credits.is_ok(), "Should estimate credits for {}", op_type);
        assert!(
            credits.unwrap() >= expected_base,
            "Credits should be at least base cost"
        );
    }
}

#[wasm_bindgen_test]
async fn test_identity_nonce() {
    let sdk = setup_test_sdk().await;
    let identity_id = test_identity_id();

    // Test get nonce
    let nonce = get_identity_nonce(&sdk, &identity_id, false).await;
    assert!(nonce.is_ok(), "Should get identity nonce");

    // Test increment nonce
    let incremented = increment_identity_nonce(&sdk, &identity_id, Some(1)).await;
    assert!(incremented.is_ok(), "Should increment identity nonce");
}

#[wasm_bindgen_test]
async fn test_create_identity_with_asset_lock() {
    let transaction = test_transaction_bytes();
    let instant_lock = test_instant_lock_bytes();

    let asset_lock_proof = AssetLockProof::create_instant(transaction, 0, instant_lock)
        .expect("Failed to create proof");

    let public_keys = js_sys::Array::new();

    let result = create_identity_with_asset_lock(&asset_lock_proof, public_keys.into()).await;
    assert!(result.is_ok(), "Should create identity with asset lock");
}
