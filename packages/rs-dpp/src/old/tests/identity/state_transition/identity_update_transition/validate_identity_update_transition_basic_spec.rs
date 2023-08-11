use crate::consensus::test_consensus_error::TestConsensusError;
use crate::identity::state_transition::validate_public_key_signatures::PublicKeysSignaturesValidator;
use crate::{
    consensus::ConsensusError,
    identity::{
        state_transition::{
            identity_update_transition::{
                identity_update_transition::{property_names, IdentityUpdateTransition},
                validate_identity_update_transition_basic::ValidateIdentityUpdateTransitionBasic,
            },
            validate_public_key_signatures::TPublicKeysSignaturesValidator,
        },
        validation::MockTPublicKeysValidator,
        KeyType, Purpose, SecurityLevel,
    },
    prelude::IdentityPublicKey,
    state_transition::{StateTransitionFieldTypes, StateTransitionIdentitySignedV0},
    tests::{
        fixtures::{
            get_identity_update_transition_fixture, get_protocol_version_validator_fixture,
        },
        utils::get_schema_error,
    },
    validation::SimpleConsensusValidationResult,
    version::ProtocolVersionValidator,
    NativeBlsModule, NonConsensusError,
};
use platform_value::{platform_value, BinaryData, Value};
use std::{convert::TryInto, sync::Arc};
use test_case::test_case;

struct TestData {
    protocol_version_validator: ProtocolVersionValidator,
    validate_public_keys_mock: MockTPublicKeysValidator,
    ec_public_key: [u8; 33],
    ec_private_key: [u8; 32],
    identity_public_key: IdentityPublicKey,
    state_transition: IdentityUpdateTransition,
    raw_state_transition: Value,
    raw_public_key_to_add: Value,
    public_keys_signatures_validator: PublicKeysSignaturesValidator<NativeBlsModule>,
}

#[derive(Default)]
pub struct SignaturesValidatorMock {}

impl TPublicKeysSignaturesValidator for SignaturesValidatorMock {
    fn validate_public_key_signatures<'a>(
        &self,
        _raw_state_transition: &Value,
        _raw_public_keys: impl IntoIterator<Item = &'a Value>,
    ) -> Result<SimpleConsensusValidationResult, NonConsensusError> {
        Ok(SimpleConsensusValidationResult::default())
    }
}

fn setup_test() -> TestData {
    let bls = NativeBlsModule::default();
    let protocol_version_validator = get_protocol_version_validator_fixture();
    let validate_public_keys_mock = MockTPublicKeysValidator::new();
    let mut state_transition = get_identity_update_transition_fixture();

    let secp = dashcore::secp256k1::Secp256k1::new();
    let mut rng = dashcore::secp256k1::rand::thread_rng();
    let (private_key, public_key) = secp.generate_keypair(&mut rng);
    let ec_private_key = private_key.secret_bytes();
    let ec_public_key = public_key.serialize();

    let identity_public_key = IdentityPublicKey {
        id: 1,
        key_type: KeyType::ECDSA_SECP256K1,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::MASTER,
        data: BinaryData::new(ec_public_key.try_into().unwrap()),
        read_only: false,
        disabled_at: None,
    };

    state_transition
        .sign(&identity_public_key, &ec_private_key, &bls)
        .expect("transition should be singed");
    let raw_state_transition = state_transition.to_object(false).unwrap();

    let raw_public_key_to_add = platform_value!({
        "id": 0u32,
        "type": KeyType::ECDSA_SECP256K1 as u8,
        "data":  base64::decode("AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di").unwrap(),
        "purpose": Purpose::AUTHENTICATION as u8,
        "securityLevel": SecurityLevel::MASTER as u8,
        "readOnly": false,
    });

    TestData {
        protocol_version_validator,
        validate_public_keys_mock,
        ec_public_key,
        ec_private_key,
        identity_public_key,
        state_transition,
        raw_state_transition,
        raw_public_key_to_add,
        public_keys_signatures_validator: PublicKeysSignaturesValidator::new(
            NativeBlsModule::default(),
        ),
    }
}

#[test_case(property_names::PROTOCOL_VERSION)]
#[test_case(property_names::TYPE)]
#[test_case(property_names::SIGNATURE)]
#[test_case(property_names::REVISION)]
#[test_case(property_names::IDENTITY_ID)]
fn property_should_be_present(property: &str) {
    let TestData {
        protocol_version_validator,
        validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    raw_state_transition.remove(property).unwrap();

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!(schema_error.keyword(), "required");
    assert_eq!(schema_error.property_name(), property);
}

#[test_case(property_names::IDENTITY_ID)]
#[test_case(property_names::SIGNATURE)]
fn property_should_be_byte_array(property_name: &str) {
    let TestData {
        protocol_version_validator,
        validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    let array = ["string"; 32];
    raw_state_transition[property_name] = platform_value!(array);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    let byte_array_schema_error = get_schema_error(&result, 1);
    assert_eq!(
        format!("/{}/0", property_name),
        schema_error.instance_path()
    );
    assert_eq!("type", schema_error.keyword());
    assert_eq!(
        format!("/properties/{}/byteArray/items/type", property_name),
        byte_array_schema_error.schema_path().to_string()
    );
}

#[test_case(property_names::PROTOCOL_VERSION)]
#[test_case(property_names::REVISION)]
#[test_case(property_names::PUBLIC_KEYS_DISABLED_AT)]
fn property_should_be_integer(property_name: &str) {
    let TestData {
        protocol_version_validator,
        validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    raw_state_transition[property_name] = platform_value!("1");

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(format!("/{}", property_name), schema_error.instance_path());
    assert_eq!("type", schema_error.keyword(),);
}

#[test_case(property_names::IDENTITY_ID, 32)]
#[test_case(property_names::SIGNATURE, 65)]
fn signature_should_be_not_less_than_n_bytes(property_name: &str, n_bytes: usize) {
    let TestData {
        protocol_version_validator,
        validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    let array = vec![0u8; n_bytes - 1];
    raw_state_transition[property_name] = platform_value!(array);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(format!("/{}", property_name), schema_error.instance_path());
    assert_eq!("minItems", schema_error.keyword(),);
}

#[test_case(property_names::IDENTITY_ID, 32)]
#[test_case(property_names::SIGNATURE, 96)]
fn signature_should_be_not_longer_than_n_bytes(property_name: &str, n_bytes: usize) {
    let TestData {
        protocol_version_validator,
        validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    let array = vec![0u8; n_bytes + 1];
    raw_state_transition[property_name] = platform_value!(array);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(format!("/{}", property_name), schema_error.instance_path());
    assert_eq!("maxItems", schema_error.keyword(),);
}

#[test]
fn protocol_version_should_be_valid() {
    let TestData {
        protocol_version_validator,
        validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    raw_state_transition[property_names::PROTOCOL_VERSION] = platform_value!(-1);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect_err("error should be returned");

    assert!(matches!(result, NonConsensusError::ValueError(_)));
}

#[test]
fn raw_state_transition_type_should_be_valid() {
    let TestData {
        protocol_version_validator,
        validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    raw_state_transition[property_names::TYPE] = platform_value!(666);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::TYPE),
        schema_error.instance_path()
    );
    assert_eq!("const", schema_error.keyword());
}

#[test]
fn revision_should_be_greater_or_equal_0() {
    let TestData {
        protocol_version_validator,
        validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    raw_state_transition[property_names::REVISION] = platform_value!(-1);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::REVISION),
        schema_error.instance_path()
    );
    assert_eq!("minimum", schema_error.keyword());
}

#[test]
fn add_public_keys_should_return_valid_result() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        raw_public_key_to_add,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
    let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
    raw_state_transition[property_names::ADD_PUBLIC_KEYS] =
        platform_value!(vec![raw_public_key_to_add]);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    assert!(result.is_valid());
}

#[test]
fn add_public_keys_should_not_be_empty() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
    let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
    raw_state_transition[property_names::ADD_PUBLIC_KEYS] = platform_value!([]);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::ADD_PUBLIC_KEYS),
        schema_error.instance_path()
    );
    assert_eq!("minItems", schema_error.keyword(),);
}

#[test]
fn add_public_keys_should_not_have_more_than_10_items() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        raw_public_key_to_add,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
    let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
    let public_keys_to_add: Vec<Value> = (0..11).map(|_| raw_public_key_to_add.clone()).collect();
    raw_state_transition[property_names::ADD_PUBLIC_KEYS] = platform_value!(public_keys_to_add);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::ADD_PUBLIC_KEYS),
        schema_error.instance_path()
    );
    assert_eq!("maxItems", schema_error.keyword(),);
}

#[test]
fn add_public_keys_should_be_unique() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        raw_public_key_to_add,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
    let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
    let public_keys_to_add: Vec<Value> = (0..2).map(|_| raw_public_key_to_add.clone()).collect();
    raw_state_transition[property_names::ADD_PUBLIC_KEYS] = platform_value!(public_keys_to_add);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::ADD_PUBLIC_KEYS),
        schema_error.instance_path()
    );
    assert_eq!("uniqueItems", schema_error.keyword(),);
}

#[test]
fn add_public_keys_should_be_valid() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        raw_public_key_to_add,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .return_once(|_| {
            Ok(SimpleConsensusValidationResult::new_with_errors(vec![
                ConsensusError::TestConsensusError(TestConsensusError::new("test")),
            ]))
        });

    let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
    let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);
    raw_state_transition[property_names::ADD_PUBLIC_KEYS] =
        platform_value!([raw_public_key_to_add]);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    assert!(matches!(
        result.errors[0],
        ConsensusError::TestConsensusError(_)
    ))
}

#[test]
fn disable_public_keys_should_be_used_only_with_public_keys_disabled_at() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
    let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);

    assert_eq!(schema_error.keyword(), "required");
    assert_eq!(
        schema_error.property_name(),
        property_names::PUBLIC_KEYS_DISABLED_AT
    );
}

#[test]
fn disable_public_keys_should_be_valid() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
    raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = platform_value!(vec![0]);
    raw_state_transition[property_names::PUBLIC_KEYS_DISABLED_AT] = platform_value!(0);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");
    assert!(result.is_valid());
}

#[test]
fn disable_public_keys_should_contain_number_greater_or_equal_0() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
    raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = platform_value!(vec![-1, 0]);
    raw_state_transition[property_names::PUBLIC_KEYS_DISABLED_AT] = platform_value!(0);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}/0", property_names::DISABLE_PUBLIC_KEYS),
        schema_error.instance_path()
    );
    assert_eq!("minimum", schema_error.keyword(),);
}

#[test]
fn disable_public_keys_should_contain_integers() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
    raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = platform_value!(vec![1.1]);
    raw_state_transition[property_names::PUBLIC_KEYS_DISABLED_AT] = platform_value!(0);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}/0", property_names::DISABLE_PUBLIC_KEYS),
        schema_error.instance_path()
    );
    assert_eq!("type", schema_error.keyword(),);
}

#[test]
fn disable_public_keys_should_not_have_more_than_10_items() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
    let key_ids_to_disable: Vec<usize> = (0..11).collect();
    raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = platform_value!(key_ids_to_disable);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::DISABLE_PUBLIC_KEYS),
        schema_error.instance_path()
    );
    assert_eq!("maxItems", schema_error.keyword(),);
}

#[test]
fn disable_public_keys_should_be_unique() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
    let key_ids_to_disable: Vec<usize> = vec![0, 0];
    raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = platform_value!(key_ids_to_disable);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::DISABLE_PUBLIC_KEYS),
        schema_error.instance_path()
    );
    assert_eq!("uniqueItems", schema_error.keyword(),);
}

#[test]
fn public_keys_disabled_at_should_be_used_only_with_disable_public_keys() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
    let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);

    assert_eq!(
        schema_error.property_name(),
        property_names::DISABLE_PUBLIC_KEYS
    );

    assert_eq!(schema_error.keyword(), "required");
}

#[test]
fn public_keys_disabled_at_should_be_greater_or_equal_0() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
    raw_state_transition[property_names::DISABLE_PUBLIC_KEYS] = platform_value!(vec![0]);
    raw_state_transition[property_names::PUBLIC_KEYS_DISABLED_AT] = platform_value!(-1);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::PUBLIC_KEYS_DISABLED_AT),
        schema_error.instance_path()
    );
    assert_eq!("minimum", schema_error.keyword(),);
}

#[test]
fn public_keys_disabled_at_should_return_valid_result() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");
    assert!(result.is_valid());
}

#[test]
fn should_return_valid_result() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");
    println!("result is {:#?}", result);
    assert!(result.is_valid());
}

#[test]
fn should_have_either_add_public_keys_or_disable_public_keys() {
    let TestData {
        protocol_version_validator,
        mut validate_public_keys_mock,
        mut raw_state_transition,
        ..
    } = setup_test();

    validate_public_keys_mock
        .expect_validate_keys()
        .returning(|_| Ok(Default::default()));

    let _ = raw_state_transition.remove(property_names::ADD_PUBLIC_KEYS);
    let _ = raw_state_transition.remove(property_names::DISABLE_PUBLIC_KEYS);
    let _ = raw_state_transition.remove(property_names::PUBLIC_KEYS_DISABLED_AT);

    let validator: ValidateIdentityUpdateTransitionBasic<_, SignaturesValidatorMock> =
        ValidateIdentityUpdateTransitionBasic::new(
            protocol_version_validator,
            Arc::new(validate_public_keys_mock),
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap();

    let result = validator
        .validate(&raw_state_transition)
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!("", schema_error.instance_path().to_string());
    assert_eq!("anyOf", schema_error.keyword(),);
}
