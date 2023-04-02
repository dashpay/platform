use std::sync::Arc;

use dpp::{
    consensus::basic::{
        data_contract::{
            DataContractImmutablePropertiesUpdateError, IncompatibleDataContractSchemaError,
        },
        invalid_data_contract_version_error::InvalidDataContractVersionError,
        BasicError,
    },
    data_contract::{
        property_names,
        state_transition::data_contract_update_transition::{
            validation::basic::{
                get_operation_and_property_name, get_operation_and_property_name_json,
                schema_compatibility_validator::{
                    validate_schema_compatibility, DiffVAlidatorError,
                },
                validate_indices_are_backward_compatible, DATA_CONTRACT_UPDATE_SCHEMA, EMPTY_JSON,
            },
            DataContractUpdateTransition,
        },
        validation::data_contract_validator::DataContractValidator,
    },
    platform_value::{self, Value},
    state_transition::{StateTransitionAction, StateTransitionConvert},
    validation::{JsonSchemaValidator, SimpleValidationResult, ValidationResult},
    version::ProtocolVersionValidator,
    Convertible, ProtocolError,
};
use drive::{drive::Drive, grovedb::Transaction};
use serde_json::Value as JsonValue;

use crate::error::Error;

use super::StateTransitionValidation;

impl StateTransitionValidation for DataContractUpdateTransition {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<SimpleValidationResult, Error> {
        // Reuse jsonschema validation on a whole state transition
        let json_schema_validator = JsonSchemaValidator::new(DATA_CONTRACT_UPDATE_SCHEMA.clone())
            .expect("unable to compile jsonschema");
        let result = json_schema_validator
            .validate(
                &(self
                    .to_object(true)
                    .expect("data contract is serializable")
                    .try_into_validating_json()
                    .expect("TODO")),
            )
            .expect("TODO: how jsonschema validation will ever fail?");
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate protocol version
        let protocol_version_validator = ProtocolVersionValidator::default();
        let result = protocol_version_validator
            .validate(self.protocol_version)
            .expect("TODO: again, how this will ever fail, why do we even need a validator trait");
        if !result.is_valid() {
            return Ok(result);
        }

        let new_data_contract_object = self
            .data_contract
            .clone()
            .into_object()
            .expect("TODO: why would we fail to serialize it?");

        // Validate data contract
        let data_contract_validator =
            DataContractValidator::new(Arc::new(protocol_version_validator));
        let result = data_contract_validator.validate(&new_data_contract_object)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let mut validation_result = ValidationResult::default();

        // Data contract should exist
        let Some(contract_fetch_info) =
            drive
            .get_contract_with_fetch_info(self.data_contract.id.0 .0, None, Some(tx))?
            .1
        else {
            validation_result
                .add_error(BasicError::DataContractNotPresent {
                    data_contract_id: self.data_contract.id.0.0.into()
                });
            return Ok(validation_result);
        };

        let existing_data_contract = &contract_fetch_info.contract;

        let new_version = self.data_contract.version;
        let old_version = existing_data_contract.version;
        if (new_version - old_version) != 1 {
            validation_result.add_error(BasicError::InvalidDataContractVersionError(
                InvalidDataContractVersionError::new(old_version + 1, new_version),
            ))
        }

        let mut existing_data_contract_object = existing_data_contract.to_object()?;

        existing_data_contract_object
            .remove_many(&vec![
                property_names::DEFINITIONS,
                property_names::DOCUMENTS,
                property_names::VERSION,
            ])
            .map_err(ProtocolError::ValueError)?;

        let mut new_base_data_contract = new_data_contract_object.clone();
        new_base_data_contract
            .remove(property_names::DEFINITIONS)
            .ok();
        new_base_data_contract
            .remove(property_names::DOCUMENTS)
            .ok();
        new_base_data_contract.remove(property_names::VERSION).ok();

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
            .get_value("documents")
            .map_err(ProtocolError::ValueError)?
            .clone()
            .try_into()
            .map_err(ProtocolError::ValueError)?;

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
                            existing_data_contract.id,
                            operation_name.to_owned(),
                            property_name.to_owned(),
                            document_schema.clone(),
                            new_document_schema.clone(),
                        ),
                    ));
                }
                Err(DiffVAlidatorError::DataStructureError(e)) => {
                    return Err(ProtocolError::ParsingError(e.to_string()).into())
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
        let new_documents = new_documents.as_object().ok_or_else(|| {
            ProtocolError::ParsingError("new documents is not a json object".to_owned())
        })?;
        let result = validate_indices_are_backward_compatible(
            existing_data_contract.documents.iter(),
            new_documents,
        )?;
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(validation_result)
    }

    fn validate_signature(&self, drive: &Drive) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_key_signature(&self) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_state(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<ValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
