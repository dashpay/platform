use anyhow::anyhow;
use anyhow::Context;
use json_patch::PatchOperation;
use lazy_static::lazy_static;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

use crate::{
    consensus::basic::BasicError,
    data_contract::{
        property_names as contract_property_names, state_transition::property_names,
        validation::data_contract_validator::DataContractValidator, DataContract,
    },
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    util::json_value::JsonValueExt,
    validation::{JsonSchemaValidator, SimpleValidationResult},
    version::ProtocolVersionValidator,
    DashPlatformProtocolInitError, ProtocolError,
};

use super::schema_compatibility_validator::validate_schema_compatibility;
use super::schema_compatibility_validator::DiffVAlidatorError;
use super::validate_indices_are_backward_compatible;

lazy_static! {
    static ref DATA_CONTRACT_UPDATE_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "./../../../../../schema/data_contract/stateTransition/dataContractUpdate.json"
    ))
    .expect("schema for Data Contract Update should be a valid json");
    static ref EMPTY_JSON: JsonValue = json!({});
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

    pub async fn validate(
        &self,
        raw_state_transition: &JsonValue,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        let mut validation_result = SimpleValidationResult::default();

        let result = self.json_schema_validator.validate(raw_state_transition)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let protocol_version = raw_state_transition
            .get_u64(property_names::PROTOCOL_VERSION)
            .with_context(|| "invalid protocol version")? as u32;
        let result = self.protocol_version_validator.validate(protocol_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate Data Contract
        let raw_data_contract = raw_state_transition.get_value(property_names::DATA_CONTRACT)?;
        let result = self.data_contract_validator.validate(raw_data_contract)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let raw_data_contract_id = raw_data_contract.get_bytes(contract_property_names::ID)?;
        let data_contract_id = Identifier::from_bytes(&raw_data_contract_id)?;

        // Data Contract should exists
        let existing_data_contract = match self
            .state_repository
            .fetch_data_contract::<DataContract>(&data_contract_id)
            .await
        {
            Ok(data_contract) => data_contract,
            Err(_) => {
                validation_result.add_error(BasicError::DataContractNotPresent {
                    data_contract_id: data_contract_id.clone(),
                });
                return Ok(validation_result);
            }
        };

        let new_version = raw_data_contract.get_u64(contract_property_names::VERSION)? as u32;
        let old_version = existing_data_contract.version;
        if (new_version - old_version) != 1 {
            validation_result.add_error(BasicError::InvalidDataContractVersionError {
                expected_version: old_version + 1,
                version: new_version,
            })
        }
        let raw_existing_data_contract = existing_data_contract.to_object(false)?;

        let mut old_base_data_contract = raw_existing_data_contract;
        old_base_data_contract.remove(contract_property_names::DEFINITIONS)?;
        old_base_data_contract.remove(contract_property_names::DOCUMENTS)?;
        old_base_data_contract.remove(contract_property_names::VERSION)?;

        replace_bytes_with_hex_string(
            &[
                contract_property_names::ID,
                contract_property_names::OWNER_ID,
            ],
            &mut old_base_data_contract,
        )?;

        let mut new_base_data_contract = raw_data_contract.clone();
        new_base_data_contract.remove(contract_property_names::DEFINITIONS)?;
        new_base_data_contract.remove(contract_property_names::DOCUMENTS)?;
        new_base_data_contract.remove(contract_property_names::VERSION)?;

        replace_bytes_with_hex_string(
            &[
                contract_property_names::ID,
                contract_property_names::OWNER_ID,
            ],
            &mut new_base_data_contract,
        )?;

        let base_data_contract_diff =
            json_patch::diff(&old_base_data_contract, &new_base_data_contract);

        for diff in base_data_contract_diff.0.iter() {
            let (operation, property_name) = get_operation_and_property_name(diff);
            validation_result.add_error(BasicError::DataContractImmutablePropertiesUpdateError {
                operation: operation.to_owned(),
                field_path: property_name.to_owned(),
            })
        }
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // check indices are not changes
        let new_documents = raw_data_contract
            .get_value("documents")?
            .as_object()
            .ok_or_else(|| anyhow!("the 'documents' property is not an array"))?;
        let result = validate_indices_are_backward_compatible(
            existing_data_contract.documents(),
            new_documents,
        )?;
        if !result.is_valid() {
            return Ok(result);
        }

        // Schema should be backward compatible
        let old_schema = existing_data_contract.documents();
        let new_schema = raw_data_contract.get_value("documents")?;

        for (document_type, document_schema) in old_schema.iter() {
            let new_document_schema = new_schema.get(document_type).unwrap_or(&EMPTY_JSON);
            let result = validate_schema_compatibility(document_schema, new_document_schema);
            match result {
                Ok(_) => {}
                Err(DiffVAlidatorError::SchemaCompatibilityError { diffs }) => {
                    let (operation_name, property_name) =
                        get_operation_and_property_name(&diffs[0]);
                    validation_result.add_error(BasicError::IncompatibleDataContractSchemaError {
                        data_contract_id: existing_data_contract.id.clone(),
                        operation: operation_name.to_owned(),
                        field_path: property_name.to_owned(),
                        old_schema: document_schema.clone(),
                        new_schema: new_document_schema.clone(),
                    });
                }
                Err(DiffVAlidatorError::DataStructureError(e)) => {
                    return Err(ProtocolError::ParsingError(e.to_string()))
                }
            }
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
        let hex_string = hex::encode(&bytes).to_string();
        data[property_name.as_ref()] = JsonValue::String(hex_string);
    }
    Ok(())
}

fn get_operation_and_property_name(p: &PatchOperation) -> (&'static str, &str) {
    match &p {
        PatchOperation::Add(ref o) => ("add", o.path.as_str()),
        PatchOperation::Copy(ref o) => ("copy", o.path.as_str()),
        PatchOperation::Remove(ref o) => ("remove", o.path.as_str()),
        PatchOperation::Replace(ref o) => ("replace", o.path.as_str()),
        PatchOperation::Move(ref o) => ("move", o.path.as_str()),
        PatchOperation::Test(ref o) => ("test", o.path.as_str()),
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
