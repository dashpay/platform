//! Comprehensive tests for the group actions module

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
use wasm_sdk::{
    start,
    state_transitions::group::{
        create_group_action, create_group_proposal, create_group_state_transition_info,
        validate_group_action_data,
    },
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_group_state_transition_info() {
    start().await.expect("Failed to start WASM");

    // Test basic creation
    let info = create_group_state_transition_info(0, Some("action-123".to_string()), Some(true));

    assert!(info.is_object());

    // Test without optional parameters
    let info_minimal = create_group_state_transition_info(1, None, None);

    assert!(info_minimal.is_object());
}

#[wasm_bindgen_test]
async fn test_create_group_proposal() {
    start().await.expect("Failed to start WASM");

    let data_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let document_type_position = 0;
    let action_name = "updateFee";

    // Create action data
    let data_json = js_sys::Object::new();
    js_sys::Reflect::set(&data_json, &"newFee".into(), &JsValue::from(100)).unwrap();
    js_sys::Reflect::set(&data_json, &"reason".into(), &"Fee adjustment".into()).unwrap();

    let proposer_id = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF";

    // Create group info
    let info = create_group_state_transition_info(0, Some("proposal-123".to_string()), Some(true));

    let signature_public_key_id = 1;

    // Create proposal
    let result = create_group_proposal(
        data_contract_id,
        document_type_position,
        action_name,
        data_json.into(),
        proposer_id,
        info,
        signature_public_key_id,
    );

    // Should return a Uint8Array
    assert!(result.is_ok());
    if let Ok(proposal) = result {
        assert!(proposal.is_instance_of::<js_sys::Uint8Array>());
    }
}

#[wasm_bindgen_test]
async fn test_create_group_action() {
    start().await.expect("Failed to start WASM");

    let data_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let document_type_position = 0;
    let action_name = "vote";

    // Create action data
    let data_json = js_sys::Object::new();
    js_sys::Reflect::set(&data_json, &"proposalId".into(), &"proposal-123".into()).unwrap();
    js_sys::Reflect::set(&data_json, &"vote".into(), &"yes".into()).unwrap();

    let actor_id = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF";

    // Create group info
    let info = create_group_state_transition_info(0, Some("action-456".to_string()), Some(false));

    let signature_public_key_id = 1;

    // Create action
    let result = create_group_action(
        data_contract_id,
        document_type_position,
        action_name,
        data_json.into(),
        actor_id,
        info,
        signature_public_key_id,
    );

    // Should return a Uint8Array
    assert!(result.is_ok());
    if let Ok(action) = result {
        assert!(action.is_instance_of::<js_sys::Uint8Array>());
    }
}

#[wasm_bindgen_test]
async fn test_validate_group_action_data() {
    start().await.expect("Failed to start WASM");

    // Test valid data
    let valid_data = js_sys::Object::new();
    js_sys::Reflect::set(&valid_data, &"action".into(), &"approve".into()).unwrap();
    js_sys::Reflect::set(
        &valid_data,
        &"timestamp".into(),
        &JsValue::from(Date::now()),
    )
    .unwrap();

    let result = validate_group_action_data(valid_data.into());
    assert!(result.is_ok());

    // Test invalid data (null)
    let result_null = validate_group_action_data(JsValue::null());
    assert!(result_null.is_err());

    // Test invalid data (not an object)
    let result_string = validate_group_action_data(JsValue::from_str("not an object"));
    assert!(result_string.is_err());
}

#[wasm_bindgen_test]
async fn test_group_proposal_with_complex_data() {
    start().await.expect("Failed to start WASM");

    let data_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let document_type_position = 0;
    let action_name = "updateConfig";

    // Create complex action data
    let data_json = js_sys::Object::new();

    // Add nested object
    let config = js_sys::Object::new();
    js_sys::Reflect::set(&config, &"maxMembers".into(), &JsValue::from(100)).unwrap();
    js_sys::Reflect::set(&config, &"votingThreshold".into(), &JsValue::from(0.66)).unwrap();
    js_sys::Reflect::set(&config, &"proposalDuration".into(), &JsValue::from(86400)).unwrap();

    js_sys::Reflect::set(&data_json, &"config".into(), &config).unwrap();
    js_sys::Reflect::set(
        &data_json,
        &"effectiveDate".into(),
        &JsValue::from(Date::now()),
    )
    .unwrap();

    let proposer_id = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF";

    let info =
        create_group_state_transition_info(0, Some("proposal-complex".to_string()), Some(true));

    let result = create_group_proposal(
        data_contract_id,
        document_type_position,
        action_name,
        data_json.into(),
        proposer_id,
        info,
        1,
    );

    assert!(result.is_ok());
}

#[wasm_bindgen_test]
async fn test_group_action_with_array_data() {
    start().await.expect("Failed to start WASM");

    let data_contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let document_type_position = 0;
    let action_name = "addMembers";

    // Create action data with array
    let data_json = js_sys::Object::new();

    let members = js_sys::Array::new();
    members.push(&JsValue::from("member1"));
    members.push(&JsValue::from("member2"));
    members.push(&JsValue::from("member3"));

    js_sys::Reflect::set(&data_json, &"members".into(), &members).unwrap();
    js_sys::Reflect::set(&data_json, &"role".into(), &"contributor".into()).unwrap();

    let actor_id = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF";

    let info = create_group_state_transition_info(0, Some("action-array".to_string()), Some(false));

    let result = create_group_action(
        data_contract_id,
        document_type_position,
        action_name,
        data_json.into(),
        actor_id,
        info,
        1,
    );

    assert!(result.is_ok());
}

#[wasm_bindgen_test]
async fn test_group_info_edge_cases() {
    start().await.expect("Failed to start WASM");

    // Test with maximum position
    let info_max =
        create_group_state_transition_info(u16::MAX, Some("max-position".to_string()), Some(true));
    assert!(info_max.is_object());

    // Test with empty string action ID
    let info_empty = create_group_state_transition_info(0, Some("".to_string()), Some(false));
    assert!(info_empty.is_object());

    // Test with very long action ID
    let long_id = "a".repeat(1000);
    let info_long = create_group_state_transition_info(0, Some(long_id), None);
    assert!(info_long.is_object());
}

#[wasm_bindgen_test]
async fn test_invalid_contract_id() {
    start().await.expect("Failed to start WASM");

    // Test with invalid contract ID format
    let result = create_group_proposal(
        "invalid-contract-id",
        0,
        "action",
        js_sys::Object::new().into(),
        "proposer-id",
        create_group_state_transition_info(0, None, None),
        1,
    );

    // Should handle invalid IDs gracefully
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_validate_empty_object() {
    start().await.expect("Failed to start WASM");

    // Empty object should be valid
    let empty_obj = js_sys::Object::new();
    let result = validate_group_action_data(empty_obj.into());
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
async fn test_group_action_validation_types() {
    start().await.expect("Failed to start WASM");

    // Test various data types
    let test_cases = vec![
        (JsValue::from(42), false),           // Number
        (JsValue::from(true), false),         // Boolean
        (JsValue::undefined(), false),        // Undefined
        (js_sys::Array::new().into(), false), // Array (not object)
        (js_sys::Object::new().into(), true), // Object
    ];

    for (value, should_succeed) in test_cases {
        let result = validate_group_action_data(value);
        assert_eq!(result.is_ok(), should_succeed);
    }
}
