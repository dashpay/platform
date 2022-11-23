use crate::{
    identity::state_transition::identity_update_transition::validate_public_keys::{
        validate_public_keys, IDENTITY_JSON_SCHEMA,
    },
    prelude::Identity,
    tests::{fixtures::identity_fixture, utils::get_state_error_from_result},
    StateError,
};
use serde_json::Value as JsonValue;

struct TestData {
    raw_public_keys: Vec<JsonValue>,
    identity: Identity,
}

fn setup_test() -> TestData {
    let identity = identity_fixture();
    let raw_public_keys: Vec<JsonValue> = identity
        .public_keys
        .iter()
        .map(|pk| pk.to_raw_json_object(false))
        .collect::<Result<_, _>>()
        .unwrap();

    TestData {
        identity,
        raw_public_keys,
    }
}

#[test]
fn should_return_invalid_result_if_there_are_duplicate_key_ids() {
    let TestData {
        mut raw_public_keys,
        ..
    } = setup_test();

    raw_public_keys[1]["id"] = raw_public_keys[0]["id"].clone();
    let result =
        validate_public_keys(&raw_public_keys).expect("the validation result should be returned");

    let state_error = get_state_error_from_result(&result, 0);

    assert!(matches!(
        state_error,
        StateError::DuplicatedIdentityPublicKeyIdError { duplicated_ids }
        if duplicated_ids == &vec![0]
    ));
    assert_eq!(4022, result.errors[0].code());
}

#[test]
fn should_return_invalid_result_if_there_are_duplicate_keys() {
    let TestData {
        mut raw_public_keys,
        ..
    } = setup_test();

    raw_public_keys[1]["data"] = raw_public_keys[0]["data"].clone();
    let result =
        validate_public_keys(&raw_public_keys).expect("the validation result should be returned");

    let state_error = get_state_error_from_result(&result, 0);

    assert!(matches!(
        state_error,
        StateError::DuplicatedIdentityPublicKeyError { duplicated_public_key_ids }
        if duplicated_public_key_ids == &vec![1]
    ));
    assert_eq!(4021, result.errors[0].code());
}

#[test]
fn should_pass_valid_public_keys() {
    let TestData {
        raw_public_keys, ..
    } = setup_test();

    let result =
        validate_public_keys(&raw_public_keys).expect("the validation result should be returned");
    assert!(result.is_valid());
}

#[test]
fn should_return_invalid_result_if_number_of_public_keys_is_bigger_than_32() {
    let TestData {
        mut raw_public_keys,
        ..
    } = setup_test();

    let max_items = IDENTITY_JSON_SCHEMA["properties"]["publicKeys"]["maxItems"]
        .as_u64()
        .unwrap() as usize;
    let num_to_add = max_items - raw_public_keys.len() + 1;

    for _ in 0..num_to_add {
        raw_public_keys.push(raw_public_keys[0].clone());
    }

    let result =
        validate_public_keys(&raw_public_keys).expect("the validation result should be returned");
    let state_error = get_state_error_from_result(&result, 0);

    assert!(matches!(
        state_error,
        StateError::MaxIdentityPublicKeyLimitReached { max_items }
        if max_items == &32
    ));
    assert_eq!(4020, result.errors[0].code());
}
