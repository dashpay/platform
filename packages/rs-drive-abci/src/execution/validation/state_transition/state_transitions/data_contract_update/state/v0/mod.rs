use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::basic::data_contract::InvalidDataContractVersionError;
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use dpp::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::contract_config::ContractConfig;
use dpp::data_contract::state_transition::data_contract_update_transition::validation::basic::any_schema_changes;
use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransitionAction;
use dpp::document::document_transition::document_base_transition::JsonValue;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::{
    consensus::basic::data_contract::{
        DataContractImmutablePropertiesUpdateError, IncompatibleDataContractSchemaError,
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
use drive::grovedb::TransactionArg;

pub(crate) trait StateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

fn validate_config_update(
    old_config: &ContractConfig,
    new_config: &ContractConfig,
    contract_id: Identifier,
) -> Result<(), ConsensusError> {
    if old_config.readonly {
        return Err(DataContractIsReadonlyError::new(contract_id).into());
    }

    if new_config.readonly {
        return Err(DataContractConfigUpdateError::new(
            contract_id,
            "contract can not be changed to readonly",
        )
        .into());
    }

    if new_config.keeps_history != old_config.keeps_history {
        return Err(DataContractConfigUpdateError::new(
            contract_id,
            format!(
                "contract can not change whether it keeps history: changing from {} to {}",
                old_config.keeps_history, new_config.keeps_history
            ),
        )
        .into());
    }

    if new_config.documents_keep_history_contract_default
        != old_config.documents_keep_history_contract_default
    {
        return Err(DataContractConfigUpdateError::new(
            contract_id,
            "contract can not change the default of whether documents keeps history",
        )
        .into());
    }

    if new_config.documents_mutable_contract_default
        != old_config.documents_mutable_contract_default
    {
        return Err(DataContractConfigUpdateError::new(
            contract_id,
            "contract can not change the default of whether documents are mutable",
        )
        .into());
    }

    Ok(())
}

impl StateTransitionStateValidationV0 for DataContractUpdateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
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
                .get_contract_with_fetch_info_and_fee(self.data_contract.id.0 .0, None, add_to_cache_if_pulled, tx)?
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

        if let Err(e) = validate_config_update(
            &existing_data_contract.config,
            &self.data_contract.config,
            self.data_contract.id.0 .0.into(),
        ) {
            validation_result.add_error(e);
            return Ok(validation_result);
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

        if existing_data_contract.config.readonly && any_schema_changes(old_schema, &new_schema) {
            validation_result.add_error(DataContractIsReadonlyError::new(
                self.data_contract.id.0 .0.into(),
            ));
            return Ok(validation_result);
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

        self.transform_into_action_v0()
    }

    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action: StateTransitionAction =
            Into::<DataContractUpdateTransitionAction>::into(self).into();
        Ok(action.into())
    }
}
