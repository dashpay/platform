use chrono::Utc;
use serde_json::{json, Value as JsonValue};

use crate::{
    identity::{
        state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition,
        KeyType, Purpose, SecurityLevel,
    },
    prelude::IdentityPublicKey,
    state_transition::{
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionType,
    },
    tests::{
        fixtures::get_identity_update_transition_fixture, utils::generate_random_identifier_struct,
    },
    util::string_encoding::Encoding,
};

struct TestData {
    transition: IdentityUpdateTransition,
    raw_transition: JsonValue,
}

fn setup_test() -> TestData {
    let transition = get_identity_update_transition_fixture();
    let raw_transition = transition.to_object(false).unwrap();
    TestData {
        transition,
        raw_transition,
    }
}

#[test]
fn get_type() {
    let TestData { transition, .. } = setup_test();
    assert_eq!(StateTransitionType::IdentityUpdate, transition.get_type());
}

#[test]
fn set_identity_id() {
    let TestData { mut transition, .. } = setup_test();
    let id = generate_random_identifier_struct();
    transition.set_identity_id(id.clone());
    assert_eq!(&id, transition.get_identity_id());
}

#[test]
fn get_revision() {
    let TestData { transition, .. } = setup_test();
    assert_eq!(0, transition.get_revision());
}

#[test]
fn set_revision() {
    let TestData { mut transition, .. } = setup_test();
    transition.set_revision(42);
    assert_eq!(42, transition.get_revision());
}

#[test]
fn get_owner_id() {
    let TestData { transition, .. } = setup_test();
    assert_eq!(&transition.identity_id, transition.get_owner_id());
}

#[test]
fn get_public_keys_to_add() {
    let TestData { transition, .. } = setup_test();
    assert_eq!(
        &transition.add_public_keys,
        transition.get_public_keys_to_add()
    );
}

#[test]
fn set_public_keys_to_add() {
    let TestData { mut transition, .. } = setup_test();
    let id_public_key = IdentityPublicKey {
						id : 0,
            key_type: KeyType::BLS12_381,
            purpose: Purpose::AUTHENTICATION,
            security_level : SecurityLevel::CRITICAL,
            read_only: true,
            data: hex::decode("01fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694").unwrap(),
            disabled_at : None,
        };
    transition.set_public_keys_to_add(vec![id_public_key.clone()]);

    assert_eq!(vec![id_public_key], transition.get_public_keys_to_add());
}

#[test]
fn get_disable_public_keys() {
    let TestData { transition, .. } = setup_test();
    assert_eq!(
        transition.disable_public_keys,
        transition.get_public_key_ids_to_disable()
    );
}

#[test]
fn set_disable_public_keys() {
    let TestData { mut transition, .. } = setup_test();
    let id_to_disable = vec![1, 2];
    transition.set_public_key_ids_to_disable(id_to_disable.clone());

    assert_eq!(&id_to_disable, transition.get_public_key_ids_to_disable());
}

#[test]
fn get_public_key_disabled_at() {
    let TestData { transition, .. } = setup_test();
    assert_eq!(
        transition.public_keys_disabled_at,
        transition.get_public_keys_disabled_at()
    );
}

#[test]
fn set_public_key_disabled_at() {
    let TestData { mut transition, .. } = setup_test();
    let now = Utc::now().timestamp_millis() as u64;
    transition.set_public_keys_disabled_at(Some(now));

    assert_eq!(Some(now), transition.get_public_keys_disabled_at());
}

#[test]
fn to_object() {
    let TestData { transition, .. } = setup_test();
    let result = transition
        .to_object(false)
        .expect("conversion to object shouldn't fail");

    let expected_raw_state_transition = json!({
        "protocolVersion" : 1,
        "type" : 5,
        "signature" : [],
        "signaturePublicKeyId": 0,
        "identityId" : transition.identity_id.to_buffer(),
        "revision": 0,
        "disablePublicKeys" : [0],
        "publicKeysDisabledAt" : 1234567,
        "addPublicKeys" : [
            {

                "id" : 3,
                "purpose" : 0,
                "type": 0,
                "securityLevel" : 0,
                "data" :base64::decode("AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH").unwrap(),
                "readOnly" : false
            }
        ]
    });

    assert_eq!(expected_raw_state_transition, result);
}

#[test]
fn to_object_with_signature_skipped() {
    let TestData { transition, .. } = setup_test();
    let result = transition
        .to_object(true)
        .expect("conversion to object shouldn't fail");

    let expected_raw_state_transition = json!({
        "protocolVersion" : 1,
        "type" : 5,
        "signaturePublicKeyId": 0,
        "identityId" : transition.identity_id.to_buffer(),
        "revision": 0,
        "disablePublicKeys" : [0],
        "publicKeysDisabledAt" : 1234567,
        "addPublicKeys" : [
            {

                "id" : 3,
                "purpose" : 0,
                "type": 0,
                "securityLevel" : 0,
                "data" :base64::decode("AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH").unwrap(),
                "readOnly" : false
            }
        ]
    });

    assert_eq!(expected_raw_state_transition, result);
}

#[test]
fn to_json() {
    let TestData { transition, .. } = setup_test();
    let result = transition
        .to_json()
        .expect("conversion to json shouldn't fail");

    let expected_raw_state_transition = json!({
        "protocolVersion" : 1,
        "type" : 5,
        "signature" : "",
        "signaturePublicKeyId": 0,
        "identityId" : transition.identity_id.to_string(Encoding::Base58),
        "revision": 0,
        "disablePublicKeys" : [0],
        "publicKeysDisabledAt" : 1234567,
        "addPublicKeys" : [
            {

                "id" : 3,
                "purpose" : 0,
                "type": 0,
                "securityLevel" : 0,
                "data" : "AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH",
                "readOnly" : false
            }
        ]
    });

    assert_eq!(expected_raw_state_transition, result);
}
