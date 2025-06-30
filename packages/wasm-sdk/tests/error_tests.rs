//! Error handling tests

use wasm_bindgen_test::*;
use wasm_sdk::error::{ErrorCategory, WasmError};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_error_creation() {
    // Test creating errors with different categories
    let network_error = WasmError::new(ErrorCategory::Network, "Network connection failed");
    assert_eq!(network_error.category(), "Network");
    assert_eq!(network_error.message(), "Network connection failed");

    let validation_error = WasmError::new(ErrorCategory::Validation, "Invalid input");
    assert_eq!(validation_error.category(), "Validation");
    assert_eq!(validation_error.message(), "Invalid input");

    let proof_error = WasmError::new(
        ErrorCategory::ProofVerification,
        "Proof verification failed",
    );
    assert_eq!(proof_error.category(), "ProofVerification");
    assert_eq!(proof_error.message(), "Proof verification failed");
}

#[wasm_bindgen_test]
fn test_error_from_string() {
    let error = WasmError::from_string("Test error message");
    assert_eq!(error.category(), "Unknown");
    assert_eq!(error.message(), "Test error message");
}

#[wasm_bindgen_test]
fn test_all_error_categories() {
    let categories = vec![
        (ErrorCategory::Network, "Network"),
        (ErrorCategory::Serialization, "Serialization"),
        (ErrorCategory::Validation, "Validation"),
        (ErrorCategory::Platform, "Platform"),
        (ErrorCategory::ProofVerification, "ProofVerification"),
        (ErrorCategory::StateTransition, "StateTransition"),
        (ErrorCategory::Identity, "Identity"),
        (ErrorCategory::Document, "Document"),
        (ErrorCategory::Contract, "Contract"),
        (ErrorCategory::Unknown, "Unknown"),
    ];

    for (category, expected_str) in categories {
        let error = WasmError::new(category, "Test message");
        assert_eq!(error.category(), expected_str);
    }
}

#[wasm_bindgen_test]
fn test_error_display() {
    let error = WasmError::new(ErrorCategory::Network, "Connection timeout");
    let display_string = error.to_string();
    assert!(display_string.contains("Network"));
    assert!(display_string.contains("Connection timeout"));
}
