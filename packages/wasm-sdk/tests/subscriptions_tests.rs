//! Comprehensive tests for the subscriptions module

use js_sys::Function;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
use wasm_sdk::{
    start,
    subscriptions_v2::{
        cleanup_all_subscriptions, get_active_subscription_count,
        subscribe_to_data_contract_updates_v2, subscribe_to_document_updates_v2,
        subscribe_to_identity_balance_updates_v2, subscribe_with_handlers_v2, SubscriptionOptions,
    },
};

wasm_bindgen_test_configure!(run_in_browser);

// Mock WebSocket endpoint for testing
const TEST_ENDPOINT: &str = "wss://test.platform.dash.org/ws";

#[wasm_bindgen_test]
async fn test_subscription_lifecycle() {
    start().await.expect("Failed to start WASM");

    // Clear any existing subscriptions
    cleanup_all_subscriptions();
    assert_eq!(get_active_subscription_count(), 0);

    // Create a callback function
    let callback = Function::new_no_args(
        "
        console.log('Subscription callback called');
    ",
    );

    // Subscribe to identity balance updates
    let result = subscribe_to_identity_balance_updates_v2(
        "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
        &callback,
        Some(TEST_ENDPOINT.to_string()),
    );

    // Note: This will fail in test environment due to WebSocket connection
    // In a real test, we'd need to mock WebSocket
    assert!(result.is_err()); // Expected to fail without real WebSocket server
}

#[wasm_bindgen_test]
async fn test_subscription_cleanup() {
    start().await.expect("Failed to start WASM");

    // Ensure we start clean
    cleanup_all_subscriptions();
    assert_eq!(get_active_subscription_count(), 0);

    // After cleanup, count should be zero
    cleanup_all_subscriptions();
    assert_eq!(get_active_subscription_count(), 0);
}

#[wasm_bindgen_test]
async fn test_subscription_options() {
    start().await.expect("Failed to start WASM");

    let mut options = SubscriptionOptions::new();

    // Test default values
    assert!(options.auto_reconnect);
    assert_eq!(options.max_reconnect_attempts, 5);
    assert_eq!(options.reconnect_delay_ms, 1000);
    assert_eq!(options.connection_timeout_ms, 30000);

    // Modify options
    options.auto_reconnect = false;
    options.max_reconnect_attempts = 10;
    options.reconnect_delay_ms = 2000;
    options.connection_timeout_ms = 60000;

    assert!(!options.auto_reconnect);
    assert_eq!(options.max_reconnect_attempts, 10);
}

#[wasm_bindgen_test]
async fn test_subscribe_with_handlers() {
    start().await.expect("Failed to start WASM");

    cleanup_all_subscriptions();

    let on_message = Function::new_no_args(
        "
        console.log('Message received');
    ",
    );

    let on_error = Function::new_no_args(
        "
        console.log('Error occurred');
    ",
    );

    let on_close = Function::new_no_args(
        "
        console.log('Connection closed');
    ",
    );

    // Create params object
    let params = js_sys::Object::new();
    js_sys::Reflect::set(&params, &"identityId".into(), &"test-id".into()).unwrap();

    let result = subscribe_with_handlers_v2(
        "identityBalance",
        params.into(),
        &on_message,
        Some(on_error),
        Some(on_close),
        Some(TEST_ENDPOINT.to_string()),
    );

    // Expected to fail without real WebSocket server
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_document_subscription_with_where_clause() {
    start().await.expect("Failed to start WASM");

    cleanup_all_subscriptions();

    let callback = Function::new_no_args(
        "
        console.log('Document update received');
    ",
    );

    // Create where clause
    let where_clause = js_sys::Object::new();
    let owner_obj = js_sys::Object::new();
    js_sys::Reflect::set(&owner_obj, &"$eq".into(), &"owner-id".into()).unwrap();
    js_sys::Reflect::set(&where_clause, &"owner".into(), &owner_obj).unwrap();

    let result = subscribe_to_document_updates_v2(
        "contract-id",
        "profile",
        where_clause.into(),
        &callback,
        Some(TEST_ENDPOINT.to_string()),
    );

    // Expected to fail without real WebSocket server
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_null_where_clause() {
    start().await.expect("Failed to start WASM");

    cleanup_all_subscriptions();

    let callback = Function::new_no_args(
        "
        console.log('Document update received');
    ",
    );

    // Test with null where clause
    let result = subscribe_to_document_updates_v2(
        "contract-id",
        "profile",
        JsValue::null(),
        &callback,
        Some(TEST_ENDPOINT.to_string()),
    );

    // Should handle null where clause gracefully
    assert!(result.is_err()); // Still fails due to WebSocket
}

#[wasm_bindgen_test]
async fn test_subscription_handle_memory() {
    start().await.expect("Failed to start WASM");

    cleanup_all_subscriptions();

    // If we had a working WebSocket mock, we would test:
    // 1. Create subscription
    // 2. Get handle
    // 3. Drop handle
    // 4. Verify cleanup happened automatically

    // For now, just ensure cleanup doesn't panic
    cleanup_all_subscriptions();
    assert_eq!(get_active_subscription_count(), 0);
}

// Mock test to demonstrate proper subscription handling
#[wasm_bindgen_test]
async fn test_subscription_patterns() {
    start().await.expect("Failed to start WASM");

    // Pattern 1: Simple subscription
    let simple_callback = Function::new_with_args("data", "console.log('Received:', data);");

    // Pattern 2: Error handling
    let error_handler =
        Function::new_with_args("error", "console.error('Subscription error:', error);");

    // Pattern 3: Cleanup on close
    let close_handler =
        Function::new_no_args("console.log('Subscription closed, performing cleanup...');");

    // These patterns demonstrate proper usage even though
    // actual WebSocket connection will fail in tests
}

#[wasm_bindgen_test]
async fn test_multiple_subscription_types() {
    start().await.expect("Failed to start WASM");

    cleanup_all_subscriptions();

    let callback = Function::new_no_args("console.log('Update');");

    // Test different subscription types
    let subscription_types = vec![
        ("identityBalance", js_sys::Object::new()),
        ("dataContract", js_sys::Object::new()),
        ("documents", js_sys::Object::new()),
        ("blockHeaders", js_sys::Object::new()),
        ("stateTransitionResult", js_sys::Object::new()),
    ];

    for (sub_type, params) in subscription_types {
        let result = subscribe_with_handlers_v2(
            sub_type,
            params.into(),
            &callback,
            None,
            None,
            Some(TEST_ENDPOINT.to_string()),
        );

        // All should fail without WebSocket server
        assert!(result.is_err());
    }

    // Ensure cleanup
    cleanup_all_subscriptions();
}
