use chrono::Utc;
use platform_value::{platform_value, BinaryData, Value};

use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation;
use crate::prelude::Revision;
use crate::{
    identity::{
        state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition,
        KeyType, Purpose, SecurityLevel,
    },
    state_transition::{
        StateTransitionFieldTypes, StateTransitionIdentitySignedV0, StateTransitionType,
    },
    tests::{
        fixtures::get_identity_update_transition_fixture, utils::generate_random_identifier_struct,
    },
};

struct TestData {
    transition: IdentityUpdateTransition,
    raw_transition: Value,
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
    transition.set_identity_id(id);
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

    let id_public_key = IdentityPublicKeyInCreation {
        id: 0,
        key_type: KeyType::BLS12_381,
        purpose: Purpose::AUTHENTICATION,
        security_level : SecurityLevel::CRITICAL,
        read_only: true,
        data: BinaryData::new(hex::decode("01fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694").unwrap()),
        signature : Default::default(),
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

    let expected_raw_state_transition = platform_value!({
        "protocolVersion" : 1u32,
        "type" : 5u8,

        "identityId" : transition.identity_id,
        "revision": 0 as Revision,
        "addPublicKeys" : [
            {

                "id" : 3u32,
                "type": 0u8,
                "purpose" : 0u8,
                "securityLevel" : 0u8,
                "readOnly" : false,
                "data" :BinaryData::new(base64::decode("AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH").unwrap()),
                "signature" : BinaryData::new(vec![0u8;65])
            }
        ],
        "disablePublicKeys" : [0u32],
        "publicKeysDisabledAt" : 1234567u64,
        "signaturePublicKeyId": 0u32,
        "signature" : BinaryData::new(vec![0u8;65]),
    });

    assert_eq!(expected_raw_state_transition, result);
}

#[test]
fn to_object_with_signature_skipped() {
    let TestData { transition, .. } = setup_test();
    let result = transition
        .to_object(true)
        .expect("conversion to object shouldn't fail");

    let expected_raw_state_transition = platform_value!({
        "protocolVersion" : 1u32,
        "type" : 5u8,
        "identityId" : transition.identity_id,
        "revision": 0 as Revision,
        "addPublicKeys" : [
            {

                "id" : 3u32,
                "type": 0u8,
                "purpose" : 0u8,
                "securityLevel" : 0u8,
                "readOnly" : false,
                "data" :BinaryData::new(base64::decode("AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH").unwrap()),
            }
        ],
        "disablePublicKeys" : [0u32],
        "publicKeysDisabledAt" : 1234567u64,
    });

    assert_eq!(expected_raw_state_transition, result);
}

#[test]
fn to_json() {
    let TestData { transition, .. } = setup_test();
    let result = transition
        .to_object(false)
        .expect("conversion to platform value shouldn't fail");

    let expected_raw_state_transition = platform_value!({
        "protocolVersion" : 1u32,
        "type" : 5u8,
        "identityId" : transition.identity_id,
        "revision": 0 as Revision,
        "addPublicKeys" : [
            {

                "id" : 3u32,
                "type": 0u8,
                "purpose" : 0u8,
                "securityLevel" : 0u8,
                "readOnly" : false,
                "data" : BinaryData::new(base64::decode("AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH").unwrap()),
                "signature" : BinaryData::new(vec![0;65]),
            }
        ],
        "disablePublicKeys" : [0u32],
        "publicKeysDisabledAt" : 1234567u64,
        "signaturePublicKeyId": 0u32,
        "signature" : BinaryData::new(vec![0u8;65]),
    });

    assert_eq!(expected_raw_state_transition, result);
}
