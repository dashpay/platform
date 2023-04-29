use dpp::consensus::basic::data_contract::InvalidDataContractVersionError;
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::data_contract::state_transition::data_contract_update_transition::validation::basic::DATA_CONTRACT_UPDATE_SCHEMA_VALIDATOR;
use dpp::identity::PartialIdentity;
use dpp::{
    consensus::basic::{
        data_contract::{
            DataContractImmutablePropertiesUpdateError, IncompatibleDataContractSchemaError,
        },
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
                validate_indices_are_backward_compatible, EMPTY_JSON,
            },
            DataContractUpdateTransition,
        },
    },
    platform_value::{self, Value},
    state_transition::StateTransitionAction,
    Convertible, ProtocolError,
};
use dpp::{
    data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult},
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use serde_json::Value as JsonValue;

use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature;
use crate::{error::Error, validation::state_transition::common::validate_schema};

use super::StateTransitionValidation;

impl StateTransitionValidation for DataContractUpdateTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&DATA_CONTRACT_UPDATE_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        // Validate protocol version
        //todo: redo versioning
        // let protocol_version_validator = ProtocolVersionValidator::default();
        // let result = protocol_version_validator
        //     .validate(self.protocol_version)
        //     .expect("TODO: again, how this will ever fail, why do we even need a validator trait");
        // if !result.is_valid() {
        //     return Ok(result);
        // }

        self.data_contract
            .validate_structure()
            .map_err(Error::Protocol)
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        Ok(
            validate_state_transition_identity_signature(drive, self, false, transaction)?
                .map(Some),
        )
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::default();

        // Data contract should exist
        let add_to_cache_if_pulled = tx.is_some();
        // Data contract should exist
        let Some(contract_fetch_info) =
            drive
                .get_contract_with_fetch_info(self.data_contract.id.0 .0, None, add_to_cache_if_pulled, tx)?
                .1
            else {
                validation_result
                    .add_error(BasicError::DataContractNotPresentError(
                        DataContractNotPresentError::new(self.data_contract.id.0.0.into())
                    ));
                return Ok(validation_result);
            };

        let existing_data_contract = &contract_fetch_info.contract;

        let new_version = self.data_contract.version;
        let old_version = existing_data_contract.version;
        if new_version < old_version || new_version - old_version != 1 {
            validation_result.add_error(BasicError::InvalidDataContractVersionError(
                InvalidDataContractVersionError::new(old_version + 1, new_version),
            ))
        }

        let mut existing_data_contract_object = existing_data_contract.to_object()?;
        let new_data_contract_object = self.data_contract.to_object()?;

        existing_data_contract_object
            .remove_many(&vec![
                property_names::DEFINITIONS,
                property_names::DOCUMENTS,
                property_names::VERSION,
            ])
            .map_err(ProtocolError::ValueError)?;

        let mut new_base_data_contract = new_data_contract_object.clone();
        new_base_data_contract
            .remove_many(&vec![
                property_names::DEFINITIONS,
                property_names::DOCUMENTS,
                property_names::VERSION,
            ])
            .map_err(ProtocolError::ValueError)?;

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
            .try_into_validating_json() //maybe (not sure) / could be just try_into
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
        validation_result.merge(validate_indices_are_backward_compatible(
            existing_data_contract.documents.iter(),
            new_documents,
        )?);
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let action: StateTransitionAction =
            Into::<DataContractUpdateTransitionAction>::into(self).into();
        Ok(action.into())
    }
}
