use std::collections::BTreeMap;
use std::{
    collections::{hash_map::Entry, HashMap},
    convert::{TryFrom, TryInto},
    sync::Arc,
};

use crate::consensus::basic::document::{
    DataContractNotPresentError, DuplicateDocumentTransitionsWithIdsError,
    DuplicateDocumentTransitionsWithIndicesError, InvalidDocumentTransitionActionError,
    InvalidDocumentTransitionIdError, InvalidDocumentTypeError, MissingDataContractIdBasicError,
    MissingDocumentTransitionActionError, MissingDocumentTransitionTypeError,
};
use crate::consensus::basic::value_error::ValueError;
use crate::validation::SimpleConsensusValidationResult;
use crate::{
    consensus::basic::BasicError,
    data_contract::{
        enrich_with_base_schema::{PREFIX_BYTE_1, PREFIX_BYTE_2, PREFIX_BYTE_3},
        DataContract,
    },
    prelude::Identifier,
    validation::JsonSchemaValidator,
    ProtocolError,
};
use anyhow::anyhow;
use lazy_static::lazy_static;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::converter::serde_json::BTreeValueRefJsonConverter;
use platform_value::Value;
use serde_json::Value as JsonValue;

use super::{
    find_duplicates_by_indices::find_duplicates_by_indices,
    validate_partial_compound_indices::validate_partial_compound_indices,
};

lazy_static! {
    // pub static ref BASE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(include_str!(
    //     "../../../../../../schema/document/v0/stateTransition/documentTransition/base.json"
    // ))
    // .unwrap();
    // pub static ref CREATE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(include_str!(
    //     "../../../../../../schema/document/v0/stateTransition/documentTransition/create.json"
    // ))
    // .unwrap();
    // pub static ref REPLACE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(include_str!(
    //     "../../../../../../schema/document/v0/stateTransition/documentTransition/replace.json"
    // ))
    // .unwrap();
    // pub static ref DOCUMENTS_BATCH_TRANSITIONS_SCHEMA: JsonValue = serde_json::from_str(
    //     include_str!("../../../../../../schema/document/v0/stateTransition/documentsBatch.json")
    // )
    // .unwrap();
    pub static ref DOCUMENTS_BATCH_TRANSITIONS_SCHEMA_VALIDATOR: JsonSchemaValidator =
        JsonSchemaValidator::new(DOCUMENTS_BATCH_TRANSITIONS_SCHEMA.clone())
            .expect("unable to compile jsonschema");
}

pub trait Validator {
    fn validate(&self, data: JsonValue) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

pub struct DocumentBatchTransitionBasicValidator<SR> {
    state_repository: Arc<SR>,
    protocol_version_validator: Arc<ProtocolVersionValidator>,
}

impl<SR> DocumentBatchTransitionBasicValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(
        state_repository: Arc<SR>,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Self {
        Self {
            state_repository,
            protocol_version_validator,
        }
    }

    pub async fn validate(
        &self,
        raw_state_transition: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // TODO: move validation code into function body to avoid cloning of state_repository
        validate_documents_batch_transition_basic(
            &self.protocol_version_validator,
            raw_state_transition,
            self.state_repository.clone(),
            execution_context,
        )
        .await
    }
}

pub async fn validate_documents_batch_transition_basic(
    protocol_version_validator: &ProtocolVersionValidator,
    raw_state_transition: &Value,
    state_repository: Arc<impl StateRepositoryLike>,
    execution_context: &StateTransitionExecutionContext,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut result = SimpleConsensusValidationResult::default();
    let validator =
        JsonSchemaValidator::new(DOCUMENTS_BATCH_TRANSITIONS_SCHEMA.clone()).map_err(|e| {
            anyhow!(
                "unable to compile the batch transitions schema: {}",
                e.to_string()
            )
        })?;

    let raw_state_transition_json = raw_state_transition
        .clone()
        .try_into_validating_json()
        .map_err(ProtocolError::ValueError)?;
    let validation_result = validator.validate(&raw_state_transition_json)?;
    result.merge(validation_result);
    if !result.is_valid() {
        return Ok(result);
    }

    let state_transition_map = raw_state_transition
        .to_btree_ref_string_map()
        .map_err(ProtocolError::ValueError)?;

    let owner_id = state_transition_map
        .get_identifier(property_names::OWNER_ID)
        .map_err(ProtocolError::ValueError)?;

    let protocol_version =
        state_transition_map.get_integer(property_names::STATE_TRANSITION_PROTOCOL_VERSION)?;
    let validation_result = protocol_version_validator.validate(protocol_version)?;
    result.merge(validation_result);
    if !result.is_valid() {
        return Ok(result);
    }

    let raw_document_transitions: Vec<BTreeMap<String, &Value>> = state_transition_map
        .get_inner_map_in_array(property_names::TRANSITIONS)
        .map_err(ProtocolError::ValueError)?;
    let mut document_transitions_by_contracts: HashMap<Identifier, Vec<BTreeMap<String, &Value>>> =
        HashMap::new();

    for raw_document_transition in raw_document_transitions {
        let contract_identifier = match raw_document_transition
            .get_optional_identifier(property_names::DATA_CONTRACT_ID)
        {
            Ok(None) => {
                result.add_error(BasicError::MissingDataContractIdBasicError(
                    MissingDataContractIdBasicError::new(),
                ));
                continue;
            }
            Ok(Some(id)) => id,
            Err(err) => {
                result.add_error(BasicError::ValueError(ValueError::new(err)));
                continue;
            }
        };

        match document_transitions_by_contracts.entry(contract_identifier) {
            Entry::Vacant(vacant) => {
                vacant.insert(vec![raw_document_transition]);
            }
            Entry::Occupied(mut identifiers) => {
                identifiers.get_mut().push(raw_document_transition);
            }
        };
    }

    for (data_contract_id, transitions) in document_transitions_by_contracts {
        let maybe_data_contract = state_repository
            .fetch_data_contract(&data_contract_id, Some(execution_context))
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)?;

        if execution_context.is_dry_run() {
            return Ok(result);
        }

        let data_contract = match maybe_data_contract {
            None => {
                result.add_error(DataContractNotPresentError::new(data_contract_id));
                continue;
            }
            Some(data_contract) => data_contract,
        };

        let validation_result =
            validate_document_transitions(&data_contract, owner_id, transitions)?;
        result.merge(validation_result);
    }

    Ok(result)
}

pub fn validate_document_transitions<'a>(
    data_contract: &DataContract,
    owner_id: Identifier,
    raw_document_transitions: impl IntoIterator<Item = BTreeMap<String, &'a Value>>,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut result = SimpleConsensusValidationResult::default();
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
    let enriched_base_contract =
        data_contract.enrich_with_base_schema(&BASE_TRANSITION_SCHEMA, PREFIX_BYTE_1, &[])?;
    let enriched_create_contract = enriched_base_contract.enrich_with_base_schema(
        &CREATE_TRANSITION_SCHEMA,
        PREFIX_BYTE_2,
        &[],
    )?;
    let enriched_replace_contract = enriched_base_contract.enrich_with_base_schema(
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
    data_contract: &DataContract,
    raw_document_transitions: impl IntoIterator<Item = BTreeMap<String, &'a Value>>,
    enriched_contracts_by_action: &HashMap<Action, DataContract>,
    owner_id: Identifier,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut result = SimpleConsensusValidationResult::default();
    let mut raw_document_transitions_as_value: Vec<Value> = vec![];
    let owner_id_value: Value = owner_id.into();
    for mut raw_document_transition in raw_document_transitions {
        let Some(document_type) = raw_document_transition.get_optional_str("$type").map_err(ProtocolError::ValueError)? else {
                result.add_error(BasicError::MissingDocumentTransitionTypeError(MissingDocumentTransitionTypeError::new()));
                return Ok(result);
        };

        if !data_contract.is_document_defined(document_type) {
            result.add_error(BasicError::InvalidDocumentTypeError(
                InvalidDocumentTypeError::new(document_type.to_string(), data_contract.id),
            ));
            return Ok(result);
        }

        let Some(document_action) = raw_document_transition.get_optional_integer::<u8>("$action").map_err(ProtocolError::ValueError)? else {
            result.add_error(BasicError::MissingDocumentTransitionActionError(MissingDocumentTransitionActionError::new()));
            return Ok(result);
        };

        let action = match Action::try_from(document_action) {
            Ok(action) => action,
            Err(_) => {
                result.add_error(BasicError::InvalidDocumentTransitionActionError(
                    InvalidDocumentTransitionActionError::new(document_action.to_string()),
                ));
                return Ok(result);
            }
        };

        match action {
            Action::Create | Action::Replace => {
                let enriched_data_contract = &enriched_contracts_by_action[&action];
                let document_schema = enriched_data_contract.get_document_schema(document_type)?;

                let schema_validator = if let Some(defs) = &enriched_data_contract.defs {
                    JsonSchemaValidator::new_with_definitions(document_schema.clone(), defs.iter())
                } else {
                    JsonSchemaValidator::new(document_schema.clone())
                }
                .map_err(|e| anyhow!("unable to compile enriched schema: {}", e))?;

                let schema_result = schema_validator.validate(
                    &raw_document_transition
                        .to_validating_json_value()
                        .map_err(ProtocolError::ValueError)?,
                )?;
                if !schema_result.is_valid() {
                    result.merge(schema_result);
                    return Ok(result);
                }

                if action == Action::Create {
                    let document_id = raw_document_transition.get_identifier("$id")?;
                    let entropy = raw_document_transition.get_bytes("$entropy")?;
                    // validate the id  generation
                    let generated_document_id = generate_document_id_v0(
                        &data_contract.id,
                        &owner_id,
                        document_type,
                        &entropy,
                    );

                    if generated_document_id != document_id {
                        // dbg!(
                        //     "g {} d {} c id {} owner {} dt {} e {}",
                        //     hex::encode(generated_document_id),
                        //     hex::encode(document_id),
                        //     hex::encode(data_contract.id),
                        //     hex::encode(owner_id),
                        //     document_type,
                        //     hex::encode(entropy)
                        // );
                        result.add_error(BasicError::InvalidDocumentTransitionIdError(
                            InvalidDocumentTransitionIdError::new(
                                generated_document_id,
                                document_id,
                            ),
                        ))
                    }
                }
            }

            Action::Delete => {
                let validator = JsonSchemaValidator::new(BASE_TRANSITION_SCHEMA.clone())
                    .map_err(|e| anyhow!("unable to compile base transition schema: {}", e))?;
                let validation_result = validator.validate(
                    &raw_document_transition
                        .to_validating_json_value()
                        .map_err(ProtocolError::ValueError)?,
                )?;
                if !validation_result.is_valid() {
                    result.merge(validation_result);
                    return Ok(result);
                }
            }
        }
        // we passed validation, let's add the owner_id now so we can validate indices (that might
        // use the ownerId)
        raw_document_transition.insert("$ownerId".to_string(), &owner_id_value);
        raw_document_transitions_as_value.push(raw_document_transition.into())
    }
    let raw_document_transitions_as_value_iter = raw_document_transitions_as_value.iter();
    let duplicate_transitions =
        find_duplicates_by_id(raw_document_transitions_as_value_iter.clone())?;
    if !duplicate_transitions.is_empty() {
        let references: Vec<(String, [u8; 32])> = duplicate_transitions
            .iter()
            .map(|transition_value| {
                let map = transition_value
                    .to_map()
                    .map_err(ProtocolError::ValueError)?;
                let doc_type = Value::inner_text_value(map, "$type")?;
                let id = Value::inner_hash256_value(map, "$id")?;
                Ok((doc_type.to_string(), id))
            })
            .collect::<Result<Vec<(String, [u8; 32])>, anyhow::Error>>()?;
        result.add_error(BasicError::DuplicateDocumentTransitionsWithIdsError(
            DuplicateDocumentTransitionsWithIdsError::new(references),
        ));
    }

    let duplicate_transitions_by_indices = find_duplicates_by_indices(
        raw_document_transitions_as_value_iter.clone(),
        data_contract,
    )?;
    if !duplicate_transitions_by_indices.is_empty() {
        let references: Vec<(String, [u8; 32])> = duplicate_transitions_by_indices
            .iter()
            .map(|transition_value| {
                let map = transition_value
                    .to_map()
                    .map_err(ProtocolError::ValueError)?;
                let doc_type = Value::inner_text_value(map, "$type")?;
                let id = Value::inner_hash256_value(map, "$id")?;
                Ok((doc_type.to_string(), id))
            })
            .collect::<Result<Vec<(String, [u8; 32])>, anyhow::Error>>()?;
        result.add_error(BasicError::DuplicateDocumentTransitionsWithIndicesError(
            DuplicateDocumentTransitionsWithIndicesError::new(references),
        ));
    }

    let validation_result = validate_partial_compound_indices(
        raw_document_transitions_as_value_iter
            .clone()
            .filter(|t| action_is_not_delete(t.get_str("$action").unwrap_or_default())),
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
