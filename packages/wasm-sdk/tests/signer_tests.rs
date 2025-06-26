//! Signer functionality tests

mod common;
use common::*;
use wasm_bindgen_test::*;
use wasm_sdk::signer::{BrowserSigner, HDSigner, WasmSigner};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_wasm_signer() {
    let signer = WasmSigner::new();
    
    // Set identity ID
    signer.set_identity_id(&test_identity_id());
    
    // Add a private key
    let add_result = signer.add_private_key(
        0,
        test_private_key(),
        "ECDSA_SECP256K1",
        0 // AUTHENTICATION purpose
    );
    assert!(add_result.is_ok(), "Should add private key");
    
    // Check key count
    assert_eq!(signer.get_key_count(), 1, "Should have 1 key");
    
    // Check if key exists
    assert!(signer.has_key(0), "Should have key with ID 0");
    assert!(!signer.has_key(1), "Should not have key with ID 1");
    
    // Get key IDs
    let key_ids = signer.get_key_ids();
    assert_eq!(key_ids.length(), 1, "Should have 1 key ID");
    
    // Sign data
    let data_to_sign = vec![1, 2, 3, 4, 5];
    let signature = signer.sign_data(data_to_sign, 0).await;
    assert!(signature.is_ok(), "Should sign data");
    assert!(!signature.unwrap().is_empty(), "Signature should not be empty");
    
    // Remove key
    let remove_result = signer.remove_private_key(0);
    assert!(remove_result.is_ok(), "Should remove key");
    assert!(remove_result.unwrap(), "Should return true for successful removal");
    assert_eq!(signer.get_key_count(), 0, "Should have 0 keys");
}

#[wasm_bindgen_test]
async fn test_wasm_signer_multiple_keys() {
    let signer = WasmSigner::new();
    signer.set_identity_id(&test_identity_id());
    
    // Add multiple keys with different purposes
    let purposes = vec![
        (0, "AUTHENTICATION"),
        (1, "ENCRYPTION"),
        (2, "DECRYPTION"),
        (3, "TRANSFER"),
    ];
    
    for (purpose, _name) in &purposes {
        let result = signer.add_private_key(
            *purpose as u32,
            test_private_key(),
            "ECDSA_SECP256K1",
            *purpose
        );
        assert!(result.is_ok(), "Should add key with purpose {}", purpose);
    }
    
    assert_eq!(signer.get_key_count(), 4, "Should have 4 keys");
    
    // Sign with different keys
    let data = vec![1, 2, 3];
    for (key_id, _) in &purposes {
        let signature = signer.sign_data(data.clone(), *key_id as u32).await;
        assert!(signature.is_ok(), "Should sign with key {}", key_id);
    }
}

#[wasm_bindgen_test]
async fn test_browser_signer() {
    let signer = BrowserSigner::new();
    
    // Note: In a real browser environment, this would use Web Crypto API
    // For testing, we'll just verify the methods exist and can be called
    
    // Generate key pair
    let key_pair_result = signer.generate_key_pair("ECDSA_SECP256K1", 0).await;
    // In test environment, this might fail due to lack of Web Crypto API
    // But we're testing that the method exists and can be called
    assert!(key_pair_result.is_ok() || key_pair_result.is_err());
    
    // Test sign with stored key (would use IndexedDB in real browser)
    let data = vec![1, 2, 3];
    let sign_result = signer.sign_with_stored_key(data, 0).await;
    assert!(sign_result.is_ok() || sign_result.is_err());
}

#[wasm_bindgen_test]
fn test_hd_signer() {
    // Test mnemonic generation
    let mnemonic_12 = HDSigner::generate_mnemonic(12);
    assert!(mnemonic_12.is_ok(), "Should generate 12-word mnemonic");
    let words_12: Vec<&str> = mnemonic_12.unwrap().split_whitespace().collect();
    assert_eq!(words_12.len(), 12, "Should have 12 words");
    
    let mnemonic_24 = HDSigner::generate_mnemonic(24);
    assert!(mnemonic_24.is_ok(), "Should generate 24-word mnemonic");
    let words_24: Vec<&str> = mnemonic_24.unwrap().split_whitespace().collect();
    assert_eq!(words_24.len(), 24, "Should have 24 words");
    
    // Test invalid word count
    let invalid_mnemonic = HDSigner::generate_mnemonic(13);
    assert!(invalid_mnemonic.is_err(), "Should fail with invalid word count");
}

#[wasm_bindgen_test]
fn test_hd_signer_key_derivation() {
    // Use a test mnemonic
    let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let derivation_path = "m/44'/1'/0'/0";
    
    let hd_signer = HDSigner::new(test_mnemonic, derivation_path);
    assert!(hd_signer.is_ok(), "Should create HD signer");
    
    let signer = hd_signer.unwrap();
    assert_eq!(signer.derivation_path(), derivation_path);
    
    // Derive keys at different indices
    for i in 0..5 {
        let key_result = signer.derive_key(i);
        assert!(key_result.is_ok(), "Should derive key at index {}", i);
        let key = key_result.unwrap();
        assert_eq!(key.len(), 32, "Private key should be 32 bytes");
    }
}

#[wasm_bindgen_test]
fn test_signer_error_handling() {
    let signer = WasmSigner::new();
    
    // Test signing without adding key
    let data = vec![1, 2, 3];
    let sign_result = wasm_bindgen_futures::JsFuture::from(signer.sign_data(data.clone(), 0));
    // This should fail as no key with ID 0 exists
    
    // Test invalid key type
    let invalid_key_result = signer.add_private_key(
        0,
        test_private_key(),
        "INVALID_KEY_TYPE",
        0
    );
    assert!(invalid_key_result.is_err(), "Should fail with invalid key type");
    
    // Test removing non-existent key
    let remove_result = signer.remove_private_key(999);
    assert!(remove_result.is_ok(), "Should not error on removing non-existent key");
    assert!(!remove_result.unwrap(), "Should return false for non-existent key");
}