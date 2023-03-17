use anyhow::anyhow;
use std::sync::Arc;
use test_case::test_case;

use crate::{
    consensus::{basic::BasicError, ConsensusError},
    data_contract::state_transition::{
        data_contract_update_transition::validation::basic::DataContractUpdateTransitionBasicValidator,
        property_names, DataContractUpdateTransition,
    },
    state_repository::MockStateRepositoryLike,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionConvert, StateTransitionType,
    },
    tests::{
        fixtures::{get_data_contract_fixture, get_protocol_version_validator_fixture},
        utils::{get_basic_error_from_result, get_schema_error},
    },
    version::{ProtocolVersionValidator, LATEST_VERSION},
};

use jsonschema::error::ValidationErrorKind;
use platform_value::{platform_value, BinaryData, Value};
use serde_json::Value as JsonValue;

struct TestData {
    version_validator: ProtocolVersionValidator,
    state_repository_mock: MockStateRepositoryLike,
    raw_state_transition: Value,
}

fn setup_test() -> TestData {
    let data_contract = get_data_contract_fixture(None);
    let mut updated_data_contract = data_contract.clone();
    updated_data_contract.increment_version();

    let state_transition = DataContractUpdateTransition {
        protocol_version: LATEST_VERSION,
        data_contract: updated_data_contract,
        signature: BinaryData::new(vec![0; 65]),
        signature_public_key_id: 0,
        transition_type: StateTransitionType::DataContractUpdate,
        execution_context: Default::default(),
    };

    let raw_state_transition = state_transition.to_object(false).unwrap();
    let version_validator = get_protocol_version_validator_fixture();

    let mut state_repository_mock = MockStateRepositoryLike::new();
    state_repository_mock
        .expect_fetch_data_contract()
        .returning(move |_, _| Ok(Some(data_contract.clone())));

    TestData {
        version_validator,
        state_repository_mock,
        raw_state_transition,
    }
}

#[test_case(property_names::PROTOCOL_VERSION)]
#[test_case(property_names::DATA_CONTRACT)]
#[test_case(property_names::SIGNATURE)]
#[tokio::test]
async fn should_be_present(property: &str) {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    raw_state_transition.remove(property).unwrap();

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);

    assert!(matches!(
        schema_error.kind(),
        ValidationErrorKind::Required {
            property: JsonValue::String(missing_property)
        } if missing_property == property
    ));
}

#[test_case(property_names::PROTOCOL_VERSION)]
#[test_case(property_names::SIGNATURE_PUBLIC_KEY_ID)]
#[tokio::test]
async fn should_be_integer(property: &str) {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    raw_state_transition[property] = platform_value!("1");

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property),
        schema_error.instance_path().to_string()
    );
    assert_eq!(Some("type"), schema_error.keyword(),);
}

#[tokio::test]
async fn protocol_version_should_be_valid() {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    raw_state_transition[property_names::PROTOCOL_VERSION] = platform_value!(-1);

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    assert!(matches!(
        result.errors.first(),
        Some(ConsensusError::ProtocolVersionParsingError { .. })
    ));
}

#[tokio::test]
async fn type_should_be_equal_4() {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    raw_state_transition[property_names::TRANSITION_TYPE] = platform_value!(666);

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::TRANSITION_TYPE),
        schema_error.instance_path().to_string()
    );
    assert_eq!(Some("const"), schema_error.keyword());
}

#[test_case(property_names::SIGNATURE)]
#[tokio::test]
async fn property_should_be_byte_array(property_name: &str) {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    let array = ["string"; 32];
    raw_state_transition[property_name] = platform_value!(array);

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    let byte_array_schema_error = get_schema_error(&result, 1);
    assert_eq!(
        format!("/{}/0", property_name),
        schema_error.instance_path().to_string()
    );
    assert_eq!(Some("type"), schema_error.keyword(),);
    assert_eq!(
        format!("/properties/{}/byteArray/items/type", property_name),
        byte_array_schema_error.schema_path().to_string()
    );
}

#[test_case(property_names::SIGNATURE, 65)]
#[tokio::test]
async fn should_be_not_less_than_n_bytes(property_name: &str, n_bytes: usize) {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    let array = vec![0u8; n_bytes - 1];
    raw_state_transition[property_name] = platform_value!(array);

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_name),
        schema_error.instance_path().to_string()
    );
    assert_eq!(Some("minItems"), schema_error.keyword(),);
}

#[test_case(property_names::SIGNATURE, 96)]
#[tokio::test]
async fn should_be_not_longer_than_n_bytes(property_name: &str, n_bytes: usize) {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    let array = vec![0u8; n_bytes + 1];
    raw_state_transition[property_name] = platform_value!(array);

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_name),
        schema_error.instance_path().to_string()
    );
    assert_eq!(Some("maxItems"), schema_error.keyword(),);
}

#[tokio::test]
async fn signature_public_key_id_should_be_valid() {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    raw_state_transition[property_names::SIGNATURE_PUBLIC_KEY_ID] = platform_value!(-1);

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let schema_error = get_schema_error(&result, 0);
    assert_eq!(
        format!("/{}", property_names::SIGNATURE_PUBLIC_KEY_ID),
        schema_error.instance_path().to_string()
    );
}

#[tokio::test]
async fn should_allow_making_backward_compatible_changes() {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    raw_state_transition[property_names::DATA_CONTRACT]["documents"]["indexedDocument"]
        ["properties"]["newProp"] = platform_value!({
        "type" : "integer",
        "minimum" : 0,

    });

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    assert!(result.is_valid());
}

#[tokio::test]
async fn should_have_existing_documents_schema_backward_compatible() {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    raw_state_transition[property_names::DATA_CONTRACT]["documents"]["niceDocument"]["required"]
        .push(platform_value!("name"))
        .unwrap();

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let basic_error = get_basic_error_from_result(&result, 0);

    match basic_error {
        BasicError::IncompatibleDataContractSchemaError(err) => {
            assert_eq!(err.operation(), "add".to_string());
            assert_eq!(err.field_path(), "/required/1".to_string());
        }
        _ => panic!(
            "Expected IncompatibleDataContractSchemaError, got {}",
            basic_error
        ),
    }
}

#[tokio::test]
async fn should_allow_defining_new_document() {
    let TestData {
        version_validator,
        state_repository_mock,
        mut raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    let new_document =
        raw_state_transition[property_names::DATA_CONTRACT]["documents"]["niceDocument"].clone();
    raw_state_transition[property_names::DATA_CONTRACT]["documents"]["new_doc"] = new_document;

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    assert!(result.is_valid());
}

#[tokio::test]
async fn should_return_valid_result() {
    let TestData {
        version_validator,
        state_repository_mock,
        raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    assert!(result.is_valid());
}

#[tokio::test]
async fn should_not_check_data_contract_on_dry_run() {
    let TestData {
        version_validator,
        state_repository_mock,
        raw_state_transition,
    } = setup_test();

    let validator = DataContractUpdateTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(version_validator),
    )
    .expect("validator should be created");

    let mut state_repository_mock = MockStateRepositoryLike::new();
    state_repository_mock
        .expect_fetch_data_contract()
        .returning(|_, _| Err(anyhow!("some error")));

    let execution_context = StateTransitionExecutionContext::default();
    execution_context.enable_dry_run();

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    assert!(result.is_valid());
}
