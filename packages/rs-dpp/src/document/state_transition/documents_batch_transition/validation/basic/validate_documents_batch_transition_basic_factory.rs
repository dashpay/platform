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
mod test {}
