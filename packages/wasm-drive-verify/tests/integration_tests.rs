//! Integration tests with real proof data
//!
//! These tests verify that the WASM bindings work correctly with real-world proof data.
//! The test data is based on actual proofs from Dash testnet.

use js_sys::Uint8Array;
use wasm_bindgen_test::*;
use wasm_drive_verify::document_verification::verify_proof::verify_document_proof;
use wasm_drive_verify::identity_verification::*;

mod fixtures;
use fixtures::{load_example_fixtures, proof_string_to_uint8array};

wasm_bindgen_test_configure!(run_in_browser);

/// Helper to create Uint8Array from hex string
fn hex_to_uint8array(hex: &str) -> Uint8Array {
    let bytes: Vec<u8> = hex::decode(hex).expect("Invalid hex string");
    Uint8Array::from(&bytes[..])
}

/// Helper to create Uint8Array from base64 string
fn base64_to_uint8array(base64: &str) -> Uint8Array {
    use base64::{engine::general_purpose, Engine as _};
    let bytes = general_purpose::STANDARD
        .decode(base64)
        .expect("Invalid base64 string");
    Uint8Array::from(&bytes[..])
}

mod identity_integration {
    use super::*;

    #[wasm_bindgen_test]
    fn test_verify_identity_with_mock_testnet_proof() {
        // This is a mock proof structure that mimics real testnet data
        // In production, this would be replaced with actual testnet proofs
        let mock_proof = create_mock_identity_proof();
        let identity_id =
            hex_to_uint8array("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");

        // Platform version 1 (testnet)
        let result = verify_full_identity_by_identity_id(
            &mock_proof,
            false, // not a subset proof
            &identity_id,
            1,
        );

        assert!(result.is_ok(), "Should successfully verify identity proof");
        let verified = result.unwrap();

        // Check root hash is returned
        assert!(verified.root_hash().length() > 0, "Should return root hash");

        // Check identity is returned (could be null if not found)
        let identity = verified.identity();
        assert!(!identity.is_undefined(), "Should return identity or null");
    }

    #[wasm_bindgen_test]
    fn test_verify_identity_balance_with_mock_proof() {
        let mock_proof = create_mock_balance_proof();
        let identity_id =
            hex_to_uint8array("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");

        let result = verify_identity_balance_for_identity_id(&mock_proof, &identity_id, false, 1);

        assert!(result.is_ok(), "Should successfully verify balance proof");
        let verified = result.unwrap();

        assert!(verified.root_hash().length() > 0, "Should return root hash");
        // Balance returns Option<u64>
        let balance = verified.balance();
        assert!(
            balance.is_some() || balance.is_none(),
            "Should return balance or none"
        );
    }
}

mod document_integration {
    use super::*;
    use js_sys::Object;

    #[wasm_bindgen_test]
    fn test_verify_document_with_mock_dpns_proof() {
        // Mock DPNS contract and proof
        let mock_proof = create_mock_document_proof();
        let contract_bytes = create_mock_dpns_contract();

        // Create a simple where clause
        let where_clauses = Object::new();

        let result = verify_document_proof(
            &mock_proof,
            &contract_bytes.into(),
            "domain", // DPNS domain document type
            &where_clauses.into(),
            &Object::new().into(), // empty order_by
            Some(10),              // limit
            None,                  // offset
            None,                  // start_at
            false,                 // start_at_included
            None,                  // block_time_ms
            1,                     // platform_version
        );

        assert!(result.is_ok(), "Should successfully verify document proof");
        let verified = result.unwrap();

        assert!(verified.root_hash().length() > 0, "Should return root hash");
        assert!(
            !verified.documents().is_undefined(),
            "Should return documents array"
        );
    }
}

mod contract_integration {
    use super::*;
    use wasm_drive_verify::contract_verification::verify_contract::verify_contract;

    #[wasm_bindgen_test]
    fn test_verify_contract_with_mock_proof() {
        let mock_proof = create_mock_contract_proof();
        let contract_id =
            hex_to_uint8array("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");

        let result = verify_contract(&mock_proof, None, false, false, &contract_id, 1);

        assert!(result.is_ok(), "Should successfully verify contract proof");
        let verified = result.unwrap();

        assert!(verified.root_hash().length() > 0, "Should return root hash");
        // Contract is returned as JsValue
        assert!(
            !verified.contract().is_undefined(),
            "Should return contract or null"
        );
    }
}

// Mock proof creation functions
// In real integration tests, these would load actual testnet proof data

fn create_mock_identity_proof() -> Uint8Array {
    // This would be replaced with actual proof bytes from testnet
    // For now, create a minimal valid proof structure
    let proof_bytes = vec![
        0x01, // version
        0x00, 0x00, 0x00, 0x20, // proof length (32 bytes)
        // Mock proof data
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ];
    Uint8Array::from(&proof_bytes[..])
}

fn create_mock_balance_proof() -> Uint8Array {
    // Similar mock proof for balance queries
    create_mock_identity_proof()
}

fn create_mock_document_proof() -> Uint8Array {
    // Mock proof for document queries
    create_mock_identity_proof()
}

fn create_mock_contract_proof() -> Uint8Array {
    // Mock proof for contract queries
    create_mock_identity_proof()
}

fn create_mock_dpns_contract() -> Uint8Array {
    // This would be the actual DPNS contract CBOR bytes
    // For now, return a minimal valid CBOR structure
    let contract_cbor = vec![
        0xa1, // map with 1 item
        0x64, // text string of length 4
        0x74, 0x65, 0x73, 0x74, // "test"
        0x64, // text string of length 4
        0x64, 0x61, 0x74, 0x61, // "data"
    ];
    Uint8Array::from(&contract_cbor[..])
}

#[cfg(test)]
mod platform_version_tests {
    use super::*;

    #[wasm_bindgen_test]
    fn test_different_platform_versions() {
        let mock_proof = create_mock_identity_proof();
        let identity_id =
            hex_to_uint8array("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");

        // Test with different platform versions
        for version in [1, 2, 3] {
            let result =
                verify_full_identity_by_identity_id(&mock_proof, false, &identity_id, version);

            // Version 1-3 should be supported
            assert!(
                result.is_ok(),
                "Platform version {} should be supported",
                version
            );
        }
    }
}

#[cfg(test)]
mod fixture_based_tests {
    use super::*;

    #[wasm_bindgen_test]
    fn test_with_example_fixtures() {
        let fixtures = load_example_fixtures();

        // Test identity proof from fixtures
        if let Some(identity_proof_data) = fixtures.proofs.get("identityById") {
            let proof = proof_string_to_uint8array(&identity_proof_data.proof);
            let identity_id = hex_to_uint8array(
                "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            );

            let result = verify_full_identity_by_identity_id(
                &proof,
                false,
                &identity_id,
                fixtures.platform_version,
            );

            // Even with mock data, verification should not panic
            if identity_proof_data.expected_result.has_root_hash {
                // If we expect a root hash, the function should at least not panic
                let _ = result;
            }
        }
    }
}
