use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::consensus::basic::data_contract::{
    DataContractImmutablePropertiesUpdateError, IncompatibleDataContractSchemaError,
    InvalidDataContractVersionError,
};
use crate::consensus::basic::decode::ProtocolVersionParsingError;
use crate::consensus::basic::document::DataContractNotPresentError;
use crate::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
use crate::data_contract::DocumentName;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::AsyncDataValidatorWithContext;
use crate::version::PlatformVersion;
use crate::{
    consensus::basic::BasicError,
    data_contract::{
        property_names as contract_property_names, state_transition::property_names, DataContract,
    },
    state_repository::StateRepositoryLike,
    util::json_value::JsonValueExt,
    validation::{JsonSchemaValidator, SimpleConsensusValidationResult},
    version::ProtocolVersionValidator,
     DashPlatformProtocolInitError, ProtocolError,
};
use anyhow::anyhow;
use anyhow::Context;
use async_trait::async_trait;
use lazy_static::lazy_static;
use platform_value::patch::PatchOperation;
use platform_value::Value;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

use super::schema_compatibility_validator::validate_schema_compatibility;
use super::schema_compatibility_validator::DiffVAlidatorError;
use super::validate_indices_are_backward_compatible;

lazy_static! {
    pub static ref DATA_CONTRACT_UPDATE_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "./../../../../../../schema/data_contract/stateTransition/dataContractUpdate.json"
    ))
    .expect("schema for Data Contract Update should be a valid json");
    pub static ref EMPTY_JSON: JsonValue = json!({});
    pub static ref DATA_CONTRACT_UPDATE_SCHEMA_VALIDATOR: JsonSchemaValidator =
        JsonSchemaValidator::new(DATA_CONTRACT_UPDATE_SCHEMA.clone())
            .expect("unable to compile jsonschema");
}

pub fn any_schema_changes(
    old_schema: &BTreeMap<DocumentName, JsonValue>,
    new_schema: &JsonValue,
) -> bool {
    let changes = old_schema
        .iter()
        .filter(|(document_type, original_schema)| {
            let new_document_schema = new_schema.get(document_type).unwrap_or(&EMPTY_JSON);
            let diff = json_patch::diff(original_schema, new_document_schema);
            !diff.0.is_empty()
        })
        .count();

    changes > 0
}

pub struct DataContractUpdateTransitionBasicValidator<SR> {
    json_schema_validator: JsonSchemaValidator,
    data_contract_validator: DataContractValidator,
    protocol_version_validator: Arc<ProtocolVersionValidator>,
    state_repository: Arc<SR>,
}

impl<SR> DataContractUpdateTransitionBasicValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(
        state_repository: Arc<SR>,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let json_schema_validator = JsonSchemaValidator::new(DATA_CONTRACT_UPDATE_SCHEMA.clone())?;
        let data_contract_validator =
            DataContractValidator::new(protocol_version_validator.clone());

        Ok(Self {
            state_repository,
            protocol_version_validator,
            data_contract_validator,
            json_schema_validator,
        })
    }
}

#[async_trait(?Send)]
impl<SR> AsyncDataValidatorWithContext for DataContractUpdateTransitionBasicValidator<SR>
where
    SR: StateRepositoryLike,
{
    type Item = Value;

    async fn validate(
        &self,
        raw_state_transition: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut validation_result = SimpleConsensusValidationResult::default();

        let result = self.json_schema_validator.validate(
            &raw_state_transition
                .try_to_validating_json()
                .map_err(ProtocolError::ValueError)?,
        )?;
        if !result.is_valid() {
            return Ok(result);
        }

        let state_transition_version = match raw_state_transition
            .get_integer(property_names::STATE_TRANSITION_PROTOCOL_VERSION)
        {
            Ok(v) => v,
            Err(parsing_error) => {
                return Ok(SimpleConsensusValidationResult::new_with_errors(vec![
                    ProtocolVersionParsingError::new(parsing_error.to_string()).into(),
                ]))
            }
        };

        let result = PlatformVersion::get(self.protocol_version_validator.protocol_version())?
            .validate_contract_update_state_transition_version(state_transition_version);
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate Data Contract
        let new_data_contract_object =
            raw_state_transition.get_value(property_names::DATA_CONTRACT)?;
        let result = self
            .data_contract_validator
            .validate(new_data_contract_object)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let data_contract_id = new_data_contract_object
            .get_identifier(contract_property_names::ID)
            .map_err(ProtocolError::ValueError)?;

        if execution_context.is_dry_run() {
            return Ok(result);
        }

        // Data Contract should exists
        let existing_data_contract: DataContract = match self
            .state_repository
            .fetch_data_contract(&data_contract_id, Some(execution_context))
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)?
        {
            Some(data_contract) => data_contract,
            None => {
                validation_result.add_error(DataContractNotPresentError::new(data_contract_id));
                return Ok(validation_result);
            }
        };

        let new_version: u32 =
            new_data_contract_object.get_integer(contract_property_names::VERSION)?;
        let old_version = existing_data_contract.version;

        let version_diff = new_version
            .checked_sub(old_version)
            .ok_or(ProtocolError::Overflow(
                "comparing protocol versions failed",
            ))?;
        if new_version < old_version || version_diff != 1 {
            validation_result.add_error(BasicError::InvalidDataContractVersionError(
                InvalidDataContractVersionError::new(old_version + 1, new_version),
            ))
        }
        let mut existing_data_contract_object = existing_data_contract.to_object()?;

        existing_data_contract_object
            .remove_many(&vec![
                contract_property_names::DEFINITIONS,
                contract_property_names::DOCUMENTS,
                contract_property_names::VERSION,
            ])
            .map_err(ProtocolError::ValueError)?;

        let mut new_base_data_contract = new_data_contract_object.clone();
        new_base_data_contract
            .remove(contract_property_names::DEFINITIONS)
            .ok();
        new_base_data_contract.remove(contract_property_names::DOCUMENTS)?;
        new_base_data_contract.remove(contract_property_names::VERSION)?;

        let base_data_contract_diff =
            platform_value::patch::diff(&existing_data_contract_object, &new_base_data_contract);

        for diff in base_data_contract_diff.0.iter() {
            let (operation, property_name) = get_operation_and_property_name(diff);
            validation_result.add_error(BasicError::DataContractImmutablePropertiesUpdateError(
                DataContractImmutablePropertiesUpdateError::new(
                    operation.to_owned(),
                    property_name.to_owned(),
                    existing_data_contract_object
                        .get(property_name.split_at(1).1)
                        .ok()
                        .flatten()
                        .cloned()
                        .unwrap_or(Value::Null),
                    new_base_data_contract
                        .get(property_name.split_at(1).1)
                        .ok()
                        .flatten()
                        .cloned()
                        .unwrap_or(Value::Null),
                ),
            ))
        }
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Schema should be backward compatible
        let old_schema = &existing_data_contract.documents;
        let new_schema: JsonValue = new_data_contract_object
            .get_value("documents")?
            .clone()
            .try_into()
            .map_err(ProtocolError::ValueError)?;

        if !existing_data_contract
            .config
            .documents_mutable_contract_default
        {
            // todo: figure out how to calculate diff for mutable contracts
            // There's no point in validating schema compatibility; However, we need to check
            // if there are any differences between schemas at all.
            //  can we add new documents? Does adding a new document counts as mutation?
            for (document_type, original_schema) in old_schema.iter() {
                let new_schema = new_schema.get(document_type).unwrap_or(&EMPTY_JSON);
                let _patch = json_patch::diff(original_schema, new_schema);
            }
        }

        for (document_type, document_schema) in old_schema.iter() {
            let new_document_schema = new_schema.get(document_type).unwrap_or(&EMPTY_JSON);
            let result = validate_schema_compatibility(document_schema, new_document_schema);
            match result {
                Ok(_) => {}
                Err(DiffVAlidatorError::SchemaCompatibilityError { diffs }) => {
                    let (operation_name, property_name) =
                        get_operation_and_property_name_json(&diffs[0]);
                    validation_result.add_error(BasicError::IncompatibleDataContractSchemaError(
                        IncompatibleDataContractSchemaError::new(
                            existing_data_contract.id(),
                            operation_name.to_owned(),
                            property_name.to_owned(),
                            document_schema.clone(),
                            new_document_schema.clone(),
                        ),
                    ));
                }
                Err(DiffVAlidatorError::DataStructureError(e)) => {
                    return Err(ProtocolError::ParsingError(e.to_string()))
                }
            }
        }

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // check indices are not changed
        let new_documents: JsonValue = new_data_contract_object
            .get_value("documents")
            .and_then(|a| a.clone().try_into())
            .map_err(ProtocolError::ValueError)?;

        if existing_data_contract.config.readonly
            && any_schema_changes(&existing_data_contract.documents, &new_documents)
        {
            validation_result.add_error(DataContractIsReadonlyError::new(data_contract_id));
        }

        let new_documents = new_documents
            .as_object()
            .ok_or_else(|| anyhow!("the 'documents' property is not an array"))?;

        let result = validate_indices_are_backward_compatible(
            existing_data_contract.documents.iter(),
            new_documents,
        )?;
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(validation_result)
    }
}

fn replace_bytes_with_hex_string(
    property_names: &[impl AsRef<str>],
    data: &mut JsonValue,
) -> Result<(), anyhow::Error> {
    for property_name in property_names {
        let bytes = data
            .get_bytes(property_name.as_ref())
            .with_context(|| "replacing bytes with hex string failed")?;
        let hex_string = hex::encode(bytes).to_string();
        data[property_name.as_ref()] = JsonValue::String(hex_string);
    }
    Ok(())
}

pub fn get_operation_and_property_name(p: &PatchOperation) -> (&'static str, &str) {
    match &p {
        PatchOperation::Add(ref o) => ("add", o.path.as_str()),
        PatchOperation::Copy(ref o) => ("copy", o.path.as_str()),
        PatchOperation::Remove(ref o) => ("remove", o.path.as_str()),
        PatchOperation::Replace(ref o) => ("replace", o.path.as_str()),
        PatchOperation::Move(ref o) => ("move", o.path.as_str()),
        PatchOperation::Test(ref o) => ("test", o.path.as_str()),
    }
}

pub fn get_operation_and_property_name_json(
    p: &json_patch::PatchOperation,
) -> (&'static str, &str) {
    match &p {
        json_patch::PatchOperation::Add(ref o) => ("add json", o.path.as_str()),
        json_patch::PatchOperation::Copy(ref o) => ("copy json", o.path.as_str()),
        json_patch::PatchOperation::Remove(ref o) => ("remove json", o.path.as_str()),
        json_patch::PatchOperation::Replace(ref o) => ("replace json", o.path.as_str()),
        json_patch::PatchOperation::Move(ref o) => ("move json", o.path.as_str()),
        json_patch::PatchOperation::Test(ref o) => ("test json", o.path.as_str()),
    }
}

#[cfg(test)]
mod test {
    use super::replace_bytes_with_hex_string;
    use serde_json::json;

    #[test]
    fn test_replacing_bytes_with_hex_string() {
        let mut example_json = json!({
            "alpha_bytes" : "123".as_bytes()
        });

        let result = replace_bytes_with_hex_string(&["alpha_bytes"], &mut example_json);
        assert!(result.is_ok());
        assert!(example_json["alpha_bytes"].is_string());
    }

    #[test]
    fn test_replacing_bytes_with_hex_string_property_not_exist() {
        let mut example_json = json!({
            "alpha_bytes" : "123".as_bytes()
        });

        let _ = replace_bytes_with_hex_string(&["bravo_bytes"], &mut example_json)
            .expect_err("error expected");
    }
}
