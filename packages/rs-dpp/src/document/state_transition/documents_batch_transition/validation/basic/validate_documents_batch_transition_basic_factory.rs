use std::{
    collections::{hash_map::Entry, HashMap},
    convert::TryFrom,
};

use crate::{
    consensus::basic::BasicError,
    data_contract::{
        enrich_data_contract_with_base_schema::{
            enrich_data_contract_with_base_schema, PREFIX_BYTE_1, PREFIX_BYTE_2, PREFIX_BYTE_3,
        },
        DataContract,
    },
    document::{document_transition::Action, generate_document_id::generate_document_id},
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    util::json_value::{self, JsonValueExt},
    validation::{JsonSchemaValidator, ValidationResult},
    version::ProtocolVersionValidator,
    ProtocolError,
};
use anyhow::anyhow;
use lazy_static::lazy_static;
use serde_json::Value as JsonValue;

use super::{
    find_duplicates_by_indices::find_duplicates_by_indices,
    validate_partial_compound_indices::validate_partial_compound_indices,
};

lazy_static! {
    static ref BASE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "../../../../../schema/document/stateTransition/documentTransition/base.json"
    ))
    .unwrap();
    static ref CREATE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "../../../../../schema/document/stateTransition/documentTransition/create.json"
    ))
    .unwrap();
    static ref REPLACE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "../../../../../schema/document/stateTransition/documentTransition/replace.json"
    ))
    .unwrap();
    static ref DOCUMENTS_BATCH_TRANSITIONS_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "../../../../../schema/document/stateTransition/documentsBatch.json"
    ))
    .unwrap();
}

pub trait Validator {
    fn validate(&self, data: JsonValue) -> Result<ValidationResult, ProtocolError>;
}

pub async fn validate_documents_batch_transition_basic(
    protocol_version_validator: &ProtocolVersionValidator,
    raw_state_transition: &JsonValue,
    state_repository: &impl StateRepositoryLike,
) -> Result<ValidationResult, ProtocolError> {
    let mut result = ValidationResult::default();
    let validator =
        JsonSchemaValidator::new(DOCUMENTS_BATCH_TRANSITIONS_SCHEMA.clone()).map_err(|e| {
            anyhow!(
                "unable to compile the batch transitions schema: {}",
                e.to_string()
            )
        })?;

    let validation_result = validator.validate(raw_state_transition)?;
    result.merge(validation_result);
    if !result.is_valid() {
        return Ok(result);
    }

    let protocol_version = raw_state_transition.get_u64("protocolVersion")? as u32;
    let validation_result = protocol_version_validator.validate(protocol_version)?;
    result.merge(validation_result);
    if !result.is_valid() {
        return Ok(result);
    }

    let raw_document_transitions = raw_state_transition
        .get("transitions")
        .ok_or(anyhow!("transitions property doesn't exist"))?
        .as_array()
        .ok_or(anyhow!("transitions property isn't an array"))?;
    let mut document_transitions_by_contracts: HashMap<Identifier, Vec<&JsonValue>> =
        HashMap::new();

    for raw_document_transition in raw_document_transitions {
        let data_contract_id_bytes = match raw_document_transition.get_bytes("$dataContractId") {
            Err(_) => {
                result.add_error(BasicError::MissingDataContractIdError);
                continue;
            }
            Ok(id) => id,
        };

        let identifier = match Identifier::from_bytes(&data_contract_id_bytes) {
            Ok(identifier) => identifier,
            Err(e) => {
                result.add_error(BasicError::InvalidIdentifierError {
                    identifier_name: String::from("$dataContractId"),
                    error: e.to_string(),
                });
                continue;
            }
        };

        match document_transitions_by_contracts.entry(identifier) {
            Entry::Vacant(vacant) => {
                vacant.insert(vec![raw_document_transition]);
            }
            Entry::Occupied(mut identifiers) => {
                identifiers.get_mut().push(raw_document_transition);
            }
        };
    }

    for (data_contract_id, transitions) in document_transitions_by_contracts {
        let data_contract: DataContract = match state_repository
            .fetch_data_contract(&data_contract_id)
            .await
        {
            Err(_) => {
                result.add_error(BasicError::DataContractNotPresent {
                    data_contract_id: data_contract_id.clone(),
                });
                continue;
            }
            Ok(data_contract) => data_contract,
        };

        let owner_id = Identifier::from_bytes(&raw_state_transition.get_bytes("ownerId")?)?;

        let validation_result =
            validate_document_transitions(&data_contract, &owner_id, transitions)?;
        result.merge(validation_result);
    }

    Ok(result)
}

fn validate_document_transitions<'a>(
    data_contract: &DataContract,
    owner_id: &Identifier,
    raw_document_transitions: impl IntoIterator<Item = &'a JsonValue>,
) -> Result<ValidationResult, ProtocolError> {
    let mut result = ValidationResult::default();
    let enriched_contracts_by_action = get_enriched_contracts_by_action(data_contract)?;

    let validation_result = validate_raw_transitions(
        data_contract,
        raw_document_transitions,
        &enriched_contracts_by_action,
        owner_id,
    )?;
    result.merge(validation_result);

    Ok(result)
}

fn get_enriched_contracts_by_action(
    data_contract: &DataContract,
) -> Result<HashMap<Action, DataContract>, ProtocolError> {
    let enriched_base_contract = enrich_data_contract_with_base_schema(
        data_contract,
        &BASE_TRANSITION_SCHEMA,
        PREFIX_BYTE_1,
        &[],
    )?;
    let enriched_create_contract = enrich_data_contract_with_base_schema(
        &enriched_base_contract,
        &CREATE_TRANSITION_SCHEMA,
        PREFIX_BYTE_2,
        &[],
    )?;
    let enriched_replace_contract = enrich_data_contract_with_base_schema(
        &enriched_base_contract,
        &REPLACE_TRANSITION_SCHEMA,
        PREFIX_BYTE_3,
        &["$createdAt"],
    )?;
    let mut enriched_contracts_by_action: HashMap<Action, DataContract> = HashMap::new();
    enriched_contracts_by_action.insert(Action::Create, enriched_create_contract);
    enriched_contracts_by_action.insert(Action::Replace, enriched_replace_contract);

    Ok(enriched_contracts_by_action)
}

fn validate_raw_transitions<'a>(
    // json_schema_validator : JsonSchemaValidator,
    data_contract: &DataContract,
    raw_document_transitions: impl IntoIterator<Item = &'a JsonValue>,
    enriched_contracts_by_action: &HashMap<Action, DataContract>,
    owner_id: &Identifier,
) -> Result<ValidationResult, ProtocolError> {
    let mut result = ValidationResult::default();
    let raw_document_transitions: Vec<&JsonValue> = raw_document_transitions.into_iter().collect();

    for raw_document_transition in raw_document_transitions.iter() {
        let document_type = match raw_document_transition.get_string("$type") {
            Err(_) => {
                result.add_error(BasicError::MissingDocumentTypeError);
                return Ok(result);
            }

            Ok(document_type) => {
                if !data_contract.is_document_defined(document_type) {
                    result.add_error(BasicError::InvalidDocumentTypeError {
                        document_type: document_type.to_string(),
                        data_contract_id: data_contract.id().clone(),
                    });
                    return Ok(result);
                }
                document_type
            }
        };

        let document_action = match raw_document_transition.get_u64("$action") {
            Ok(action) => action,
            Err(_) => {
                result.add_error(BasicError::MissingDocumentTransitionActionError);
                return Ok(result);
            }
        };

        let action = match Action::try_from(document_action as u8) {
            Ok(action) => action,
            Err(_) => {
                result.add_error(BasicError::InvalidDocumentTransitionActionError {
                    action: document_action.to_string(),
                });
                return Ok(result);
            }
        };

        match action {
            Action::Create | Action::Replace => {
                let enriched_data_contract = &enriched_contracts_by_action[&action];
                // let schema = enriched_data_contract.to_json()?;

                let document_schema =
                    enriched_data_contract.get_full_schema_with_defs_for_document(document_type)?;
                let schema_validator = JsonSchemaValidator::new(document_schema)
                    .map_err(|e| anyhow!("unable to compile enriched schema: {}", e))?;

                let schema_result = schema_validator.validate(raw_document_transition)?;
                if !schema_result.is_valid() {
                    result.merge(schema_result);
                    return Ok(result);
                }

                if action == Action::Create {
                    let document_id =
                        Identifier::from_bytes(&raw_document_transition.get_bytes("$id")?)?;
                    let entropy = raw_document_transition.get_bytes("$entropy")?;
                    // validate the id  generation
                    let generated_document_id =
                        generate_document_id(data_contract.id(), owner_id, document_type, &entropy);

                    if generated_document_id != document_id {
                        result.add_error(BasicError::InvalidDocumentTransitionIdError {
                            expected_id: generated_document_id,
                            invalid_id: document_id,
                        })
                    }
                }
            }

            Action::Delete => {
                let validator = JsonSchemaValidator::new(BASE_TRANSITION_SCHEMA.clone())
                    .map_err(|e| anyhow!("unable to compile base transition schema: {}", e))?;
                let validation_result = validator.validate(raw_document_transition)?;
                if !validation_result.is_valid() {
                    result.merge(validation_result);
                    return Ok(result);
                }
            }
        }
    }

    let dtr = raw_document_transitions.into_iter();

    let duplicate_transitions = find_duplicates_by_indices(dtr.clone(), data_contract)?;
    if !duplicate_transitions.is_empty() {
        let references: Vec<(String, Vec<u8>)> = duplicate_transitions
            .iter()
            .map(|t| {
                let doc_type = t.get_string("$type")?.to_string();
                let id = t.get_bytes("$id")?;
                Ok((doc_type, id))
            })
            .collect::<Result<Vec<(String, Vec<u8>)>, anyhow::Error>>()?;
        result.add_error(BasicError::DuplicateDocumentTransitionsWithIdsError { references });
    }

    let validation_result = validate_partial_compound_indices(
        dtr.clone()
            .filter(|t| action_is_not_delete(t.get_string("$action").unwrap_or_default())),
        data_contract,
    )?;
    result.merge(validation_result);

    Ok(result)
}

fn action_is_not_delete(action: &str) -> bool {
    match Action::try_from(action) {
        Err(_) => false,
        Ok(Action::Delete) => false,
        Ok(Action::Create) | Ok(Action::Replace) => true,
    }
}

#[cfg(test)]
mod test {
    use crate::{
        data_contract::{get_binary_properties_from_schema, DataContract},
        document::{
            document_transition::{Action, DocumentTransitionObjectLike},
            DocumentsBatchTransition,
        },
        state_repository::{MockStateRepositoryLike, StateRepositoryLike},
        state_transition::StateTransitionConvert,
        tests::{
            fixtures::{
                get_data_contract_fixture, get_document_transitions_fixture,
                get_documents_fixture_with_owner_id_from_contract,
                get_protocol_version_validator_fixture,
            },
            utils::{
                generate_random_identifier, get_basic_error, get_basic_error_from_result,
                get_schema_error,
            },
        },
        util::json_value::JsonValueExt,
        version::{ProtocolVersionValidator, LATEST_VERSION},
        ProtocolError,
    };
    use anyhow::anyhow;
    use jsonschema::error::ValidationErrorKind;
    use serde_json::{json, Value as JsonValue};
    use test_case::test_case;

    use super::validate_documents_batch_transition_basic;

    struct TestData {
        data_contract: DataContract,
        state_transition: DocumentsBatchTransition,
        raw_state_transition: JsonValue,
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

        let owner_id = data_contract.owner_id.clone();
        let raw_transitions: Vec<JsonValue> =
            transitions.iter().map(|d| d.to_object().unwrap()).collect();
        let signature = [0_u8; 65].to_vec();
        let state_transition = DocumentsBatchTransition::from_raw_object(
            json!({
                "protocolVersion":  LATEST_VERSION,
                "ownerId" : owner_id.as_bytes(),
                "contractId" : data_contract.id().as_bytes(),
                "transitions" : raw_transitions,
                "signature": signature,
                "signaturePublicKeyId": 0,
            }),
            vec![data_contract.clone()],
        )
        .expect("crating state transition shouldn't fail");

        let raw_state_transition = state_transition
            .to_object(false)
            .expect("conversion to the object shouldn't fail");

        let protocol_version_validator = get_protocol_version_validator_fixture();

        let mut state_repository_mock = MockStateRepositoryLike::new();
        let contract_to_return = data_contract.clone();
        state_repository_mock
            .expect_fetch_data_contract()
            .returning(move |_| Ok(contract_to_return.clone()));

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

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        raw_state_transition["protocolVersion"] = json!("1");

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        raw_state_transition["protocolVersion"] = json!("-1");

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
        .await
        .expect("protocol error should be returned");

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

        raw_state_transition["type"] = json!(666);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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
        raw_state_transition[property_name] = json!(array);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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
        raw_state_transition["ownerId"] = json!(array);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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
        raw_state_transition["ownerId"] = json!(array);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        raw_state_transition["transitions"] = json!("not an array");

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        raw_state_transition["transitions"] = json!([]);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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
            elements.push(json!({}))
        }
        raw_state_transition["transitions"] = JsonValue::Array(elements);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        let elements = vec![json!(1)];
        raw_state_transition["transitions"] = JsonValue::Array(elements);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
        .await
        .expect("validation result should be returned");

        let error = &result.errors()[0];
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
        raw_state_transition["transitions"][0][property_name] = json!(array);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        raw_state_transition["transitions"][0]["$dataContractId"] = json!("something");

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
        .await
        .expect("validation result should be returned");

        let error = &result.errors()[0];
        assert_eq!(1025, error.code());
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
        raw_state_transition["transitions"][0][property_name] = json!(array);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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
        raw_state_transition["transitions"][0][property_name] = json!(array);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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
            .expect_fetch_data_contract::<DataContract>()
            .returning(|_| Err(anyhow!("no contract found")));

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
        .await
        .expect("validation result should be returned");

        let error = &result.errors()[0];
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

        raw_state_transition["transitions"][0]["$type"] = json!("wrong");

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
        .await
        .expect("validation result should be returned");

        let error = &result.errors()[0];
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

        raw_state_transition["transitions"][0]["$action"] = json!(4);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
        .await
        .expect("validation result should be returned");

        let error = &result.errors()[0];
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

        raw_state_transition["transitions"][0]["$id"] = json!(generate_random_identifier());

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
        .await
        .expect("validation result should be returned");

        let error = &result.errors()[0];
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

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        raw_state_transition["transitions"][0]["$revision"] = json!("1");

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        raw_state_transition["transitions"][0]["$revision"] = json!(1.2);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        raw_state_transition["transitions"][0]["$revision"] = json!(0);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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
        raw_state_transition["signature"] = json!(array);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
        .await
        .expect("validation result should be returned");

        let schema_error = get_schema_error(&result, 0);
        assert_eq!("/signature", schema_error.instance_path().to_string());
        assert_eq!(Some("minItems"), schema_error.keyword(),);
    }

    #[tokio::test]
    async fn signature_should_be_not_longer_than_65_bytes() {
        let TestData {
            mut raw_state_transition,
            protocol_version_validator,
            state_repository_mock,
            ..
        } = setup_test(Action::Create);

        let array = [0u8; 66].to_vec();
        raw_state_transition["signature"] = json!(array);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        raw_state_transition["signaturePublicKeyId"] = json!(1.4);

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
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

        let result = validate_documents_batch_transition_basic(
            &protocol_version_validator,
            &raw_state_transition,
            &state_repository_mock,
        )
        .await
        .expect("validation result should be returned");

        assert!(result.is_valid());
    }
}
