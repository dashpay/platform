use crate::{
    data_contract::DataContract,
    document::{
        document_transition::{Action, DocumentTransitionObjectLike},
        validation::basic::validate_documents_batch_transition_basic::DocumentBatchTransitionBasicValidator,
        DocumentsBatchTransition,
    },
    state_repository::MockStateRepositoryLike,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext, StateTransitionConvert,
    },
    tests::{
        fixtures::{
            get_data_contract_fixture, get_document_transitions_fixture,
            get_documents_fixture_with_owner_id_from_contract,
            get_protocol_version_validator_fixture,
        },
        utils::get_schema_error,
    },
    version::{ProtocolVersionValidator, LATEST_VERSION},
};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::document::document_transition::document_base_transition::JsonValue;
use crate::tests::utils::generate_random_identifier_struct;
use jsonschema::error::ValidationErrorKind;
use platform_value::{platform_value, Value};
use test_case::test_case;

struct TestData {
    data_contract: DataContract,
    state_transition: DocumentsBatchTransition,
    raw_state_transition: Value,
    protocol_version_validator: ProtocolVersionValidator,
    state_repository_mock: MockStateRepositoryLike,
}

fn setup_test(action: Action) -> TestData {
    let data_contract = get_data_contract_fixture(None);
    let documents =
        get_documents_fixture_with_owner_id_from_contract(data_contract.clone()).unwrap();
    let transitions = match action {
        Action::Create => get_document_transitions_fixture([(Action::Create, documents)]),
        Action::Replace => get_document_transitions_fixture([
            (Action::Replace, documents),
            (Action::Create, vec![]),
        ]),
        Action::Delete => get_document_transitions_fixture([
            (Action::Delete, documents),
            (Action::Replace, vec![]),
            (Action::Create, vec![]),
        ]),
    };

    let owner_id = data_contract.owner_id;
    let raw_transitions: Vec<Value> = transitions.iter().map(|d| d.to_object().unwrap()).collect();
    let signature = [0_u8; 65].to_vec();
    let mut map = BTreeMap::new();
    map.insert("protocolVersion".to_string(), Value::U32(LATEST_VERSION));
    map.insert(
        "ownerId".to_string(),
        Value::Identifier(owner_id.to_buffer()),
    );
    map.insert(
        "contractId".to_string(),
        Value::Identifier(data_contract.id.to_buffer()),
    );
    map.insert("signature".to_string(), Value::Bytes(signature));
    map.insert("signaturePublicKeyId".to_string(), Value::U32(0));

    map.insert("transitions".to_string(), Value::Array(raw_transitions));

    let state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("crating state transition shouldn't fail");

    let raw_state_transition = state_transition
        .to_object(false)
        .expect("conversion to the object shouldn't fail");

    let protocol_version_validator = get_protocol_version_validator_fixture();

    let mut state_repository_mock = MockStateRepositoryLike::new();
    let contract_to_return = data_contract.clone();
    state_repository_mock
        .expect_fetch_data_contract()
        .returning(move |_, _| Ok(Some(contract_to_return.clone())));

    TestData {
        data_contract,
        state_transition,
        raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
    }
}

#[test_case("protocolVersion")]
#[test_case("type")]
#[test_case("ownerId")]
#[test_case("transitions")]
#[test_case("signature")]
#[tokio::test]
async fn property_should_be_present(property: &str) {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition.remove(property).unwrap();

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

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

#[tokio::test]
async fn protocol_version_should_be_integer() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["protocolVersion"] = platform_value!("1");

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/protocolVersion", schema_error.instance_path().to_string());
    assert_eq!(Some("type"), schema_error.keyword(),);
}

#[tokio::test]
async fn protocol_version_should_be_valid() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["protocolVersion"] = platform_value!("-1");

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/protocolVersion", schema_error.instance_path().to_string());
    assert_eq!(Some("type"), schema_error.keyword(),);
}

#[tokio::test]
async fn type_should_be_equal_1() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["type"] = platform_value!(666);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/type", schema_error.instance_path().to_string());
    assert_eq!(Some("const"), schema_error.keyword(),);
}

#[test_case("ownerId")]
#[test_case("signature")]
#[tokio::test]
async fn property_in_state_transition_should_be_byte_array(property_name: &str) {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let array = ["string"; 32];
    raw_state_transition[property_name] = platform_value!(array);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

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

#[tokio::test]
async fn owner_id_should_be_no_less_than_32_bytes() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let array = [0u8; 31];
    raw_state_transition["ownerId"] = platform_value!(array);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/ownerId", schema_error.instance_path().to_string());
    assert_eq!(Some("minItems"), schema_error.keyword(),);
}

#[tokio::test]
async fn owner_id_should_be_no_longer_than_32_bytes() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let mut array = Vec::new();
    array.resize(33, 0u8);
    raw_state_transition["ownerId"] = platform_value!(array);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/ownerId", schema_error.instance_path().to_string());
    assert_eq!(Some("maxItems"), schema_error.keyword(),);
}

#[tokio::test]
async fn transitions_should_be_an_array() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["transitions"] = platform_value!("not an array");

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/transitions", schema_error.instance_path().to_string());
    assert_eq!(Some("type"), schema_error.keyword(),);
}

#[tokio::test]
async fn transitions_should_have_at_least_one_element() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["transitions"] = platform_value!([]);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/transitions", schema_error.instance_path().to_string());
    assert_eq!(Some("minItems"), schema_error.keyword(),);
}

#[tokio::test]
async fn transitions_should_have_no_more_than_10_elements() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let mut elements = vec![];
    for _ in 0..11 {
        elements.push(platform_value!({}))
    }
    raw_state_transition["transitions"] = Value::Array(elements);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/transitions", schema_error.instance_path().to_string());
    assert_eq!(Some("maxItems"), schema_error.keyword(),);
}

#[tokio::test]
async fn transitions_should_have_an_object_as_elements() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let elements = vec![platform_value!(1)];
    raw_state_transition["transitions"] = Value::Array(elements);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/transitions/0", schema_error.instance_path().to_string());
    assert_eq!(Some("type"), schema_error.keyword(),);
}

// document transitions

#[test_case("$id")]
#[test_case("$entropy")]
#[tokio::test]
async fn property_in_document_transition_should_be_present(property: &str) {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["transitions"][0]
        .remove(property)
        .expect("the property should exist and be removed");

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

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

#[test_case("$action", 1026)]
#[test_case("$type", 1027)]
#[test_case("$dataContractId", 1025)]
#[tokio::test]
async fn property_should_should_exist_with_code(property_name: &str, error_code: u32) {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["transitions"][0]
        .remove(property_name)
        .expect("the property should exist and be removed");

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let error = &result.errors[0];
    assert_eq!(error_code, error.code());
}

#[test_case("$id")]
#[test_case("$entropy")]
#[tokio::test]
async fn property_should_be_byte_array(property_name: &str) {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let array = ["string"; 32];
    raw_state_transition["transitions"][0][property_name] = platform_value!(array);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

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

#[tokio::test]
async fn data_contract_id_should_be_byte_array() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["transitions"][0]["$dataContractId"] = platform_value!("something");

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let error = &result.errors[0];
    assert_eq!(5000, error.code());
}

#[test_case("$id")]
#[test_case("$entropy")]
#[tokio::test]
async fn property_should_be_no_less_than_32_bytes(property_name: &str) {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let array = [0u8; 31];
    raw_state_transition["transitions"][0][property_name] = platform_value!(array);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

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

#[test_case("$id")]
#[test_case("$entropy")]
#[tokio::test]
async fn id_should_be_no_longer_than_32_bytes(property_name: &str) {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let mut array = Vec::new();
    array.resize(33, 0u8);
    raw_state_transition["transitions"][0][property_name] = platform_value!(array);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

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
#[ignore = "unable to mock"]
async fn should_have_no_duplicate_id_in_state_transition() {}

#[tokio::test]
async fn data_contract_should_exist_in_the_state() {
    let TestData {
        raw_state_transition,
        protocol_version_validator,
        ..
    } = setup_test(Action::Create);
    let mut state_repository_mock = MockStateRepositoryLike::new();
    state_repository_mock
        .expect_fetch_data_contract()
        .returning(|_, _| Ok(None));

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let error = &result.errors[0];
    assert_eq!(1018, error.code());
}

#[tokio::test]
async fn type_should_be_defined_in_data_contract() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["transitions"][0]["$type"] = platform_value!("wrong");

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let error = &result.errors[0];
    assert_eq!(1024, error.code());
}

#[tokio::test]
async fn should_throw_invalid_document_transaction_action_error_if_action_is_not_valid() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["transitions"][0]["$action"] = platform_value!(4);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let error = &result.errors[0];
    assert_eq!(1022, error.code());
}

#[tokio::test]
async fn id_should_be_valid_generated_id() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    raw_state_transition["transitions"][0]["$id"] =
        platform_value!(generate_random_identifier_struct());

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    let error = &result.errors[0];
    assert_eq!(1023, error.code());
}

#[test_case("$revision")]
#[tokio::test]
async fn property_in_replace_transition_should_be_present(property: &str) {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Replace);

    raw_state_transition["transitions"][0]
        .remove(property)
        .expect("the property should exist and be removed");

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

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

#[tokio::test]
async fn revision_should_be_number() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Replace);

    raw_state_transition["transitions"][0]["$revision"] = platform_value!("1");

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/$revision", schema_error.instance_path().to_string());
    assert_eq!(Some("type"), schema_error.keyword(),);
}

#[tokio::test]
async fn revision_should_not_be_fractional() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Replace);

    raw_state_transition["transitions"][0]["$revision"] = platform_value!(1.2);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/$revision", schema_error.instance_path().to_string());
    assert_eq!(Some("type"), schema_error.keyword(),);
}

#[tokio::test]
async fn revision_should_be_at_least_1() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Replace);

    raw_state_transition["transitions"][0]["$revision"] = platform_value!(0);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/$revision", schema_error.instance_path().to_string());
    assert_eq!(Some("minimum"), schema_error.keyword(),);
}

#[tokio::test]
async fn id_should_be_present_in_delete_transition() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Delete);

    raw_state_transition["transitions"][0]
        .remove("$id")
        .unwrap();

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert!(matches!(
        schema_error.kind(),
        ValidationErrorKind::Required {
            property: JsonValue::String(missing_property)
        } if missing_property == "$id"
    ));
}

#[tokio::test]
#[ignore]
async fn should_return_invalid_result_if_there_are_duplicate_unique_index_values() {
    unimplemented!("unable to mock unique indices validation")
}

#[tokio::test]
#[ignore]
async fn should_return_invalid_result_if_compound_index_does_not_contain_all_fields() {
    unimplemented!("unable to mock compound indices validation")
}

#[tokio::test]
async fn signature_should_be_not_less_than_65_bytes() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let array = [0u8; 64].to_vec();
    raw_state_transition["signature"] = platform_value!(array);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/signature", schema_error.instance_path().to_string());
    assert_eq!(Some("minItems"), schema_error.keyword(),);
}

#[tokio::test]
async fn signature_should_be_not_longer_than_96_bytes() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let array = [0u8; 97].to_vec();
    raw_state_transition["signature"] = platform_value!(array);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!("/signature", schema_error.instance_path().to_string());
    assert_eq!(Some("maxItems"), schema_error.keyword(),);
}

#[tokio::test]
async fn signature_public_key_should_be_an_integer() {
    let TestData {
        mut raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Delete);

    raw_state_transition["signaturePublicKeyId"] = platform_value!(1.4);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");
    let schema_error = get_schema_error(&result, 0);

    assert_eq!(
        "/signaturePublicKeyId",
        schema_error.instance_path().to_string()
    );
    assert_eq!(Some("type"), schema_error.keyword(),);
}

#[tokio::test]
async fn validation_should_be_successful() {
    let TestData {
        raw_state_transition,
        protocol_version_validator,
        state_repository_mock,
        ..
    } = setup_test(Action::Create);

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &Default::default())
        .await
        .expect("validation result should be returned");

    assert!(result.is_valid(), "{:?}", result.errors);
}

#[tokio::test]
async fn should_not_validate_document_transitions_on_dry_run() {
    let TestData {
        raw_state_transition,
        protocol_version_validator,
        ..
    } = setup_test(Action::Create);

    let execution_context = StateTransitionExecutionContext::default();
    execution_context.enable_dry_run();

    let mut state_repository_mock = MockStateRepositoryLike::new();
    state_repository_mock
        .expect_fetch_data_contract()
        .return_once(|_, _| Ok(None));

    let validator = DocumentBatchTransitionBasicValidator::new(
        Arc::new(state_repository_mock),
        Arc::new(protocol_version_validator),
    );

    let result = validator
        .validate(&raw_state_transition, &execution_context)
        .await
        .expect("validation result should be returned");

    assert!(result.is_valid());
}
