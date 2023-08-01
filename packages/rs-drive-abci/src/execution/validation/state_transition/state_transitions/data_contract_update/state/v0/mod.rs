use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use serde_json::Value as JsonValue;

use dpp::consensus::basic::data_contract::{
    DataContractInvalidIndexDefinitionUpdateError, InvalidDataContractVersionError,
};
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::base::DataContractBaseMethodsV0;
use dpp::data_contract::conversion::platform_value_conversion::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::data_contract_config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use dpp::state_transition_action::StateTransitionAction;
use dpp::version::{PlatformVersion, TryIntoPlatformVersioned};
use dpp::{
    consensus::basic::data_contract::{
        DataContractImmutablePropertiesUpdateError, IncompatibleDataContractSchemaError,
    },
    data_contract::property_names,
    platform_value::{self, Value},
    Convertible, ProtocolError,
};
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionStateValidationV0 for DataContractUpdateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::default();

        let new_data_contract: DataContract = self
            .data_contract()
            .try_into_platform_versioned(platform_version)?;

        // Data contract should exist
        let add_to_cache_if_pulled = tx.is_some();
        // Data contract should exist
        let Some(contract_fetch_info) =
            drive
                .get_contract_with_fetch_info_and_fee(
                    new_data_contract.id().to_buffer(),
                    None,
                    add_to_cache_if_pulled,
                    tx,
                    platform_version,
                )?
                .1
            else {
                validation_result
                    .add_error(BasicError::DataContractNotPresentError(
                        DataContractNotPresentError::new(new_data_contract.id())
                    ));
                return Ok(validation_result);
            };

        let old_data_contract = &contract_fetch_info.contract;

        let new_version = new_data_contract.version();
        let old_version = old_data_contract.version();
        if new_version < old_version || new_version - old_version != 1 {
            validation_result.add_error(BasicError::InvalidDataContractVersionError(
                InvalidDataContractVersionError::new(old_version + 1, new_version),
            ))
        }

        if old_data_contract.config().readonly() {
            validation_result.add_error(DataContractIsReadonlyError::new(new_data_contract.id()));
            return Ok(validation_result);
        }

        // We should now validate that new indexes contains all old indexes
        // This is most easily done by using the index level construct

        for (new_contract_document_type_name, new_contract_document_type) in
            new_data_contract.document_types()
        {
            let Some(old_contract_document_type) = old_data_contract.optional_document_type_for_name(new_contract_document_type_name) else {
                // if it's a new document type (ie the old data contract didn't have it)
                // then new indices on it are fine
                continue;
            };
            // If the new contract document type doesn't contain all previous indexes then
            // there is a problem
            if let Some(non_subset_path) = new_contract_document_type
                .index_structure()
                .contains_subset_first_non_subset_path(old_contract_document_type.index_structure())
            {
                validation_result.add_error(
                    BasicError::DataContractInvalidIndexDefinitionUpdateError(
                        DataContractInvalidIndexDefinitionUpdateError::new(
                            new_contract_document_type_name.clone(),
                            non_subset_path,
                        ),
                    ),
                )
            }
        }

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        //todo: reenable checks

        //
        //
        // let mut existing_data_contract_object = existing_data_contract.to_object()?;
        // let new_data_contract_object = new_data_contract.to_object()?;
        //
        // existing_data_contract_object
        //     .remove_many(&vec![
        //         property_names::DEFINITIONS,
        //         property_names::DOCUMENTS,
        //         property_names::VERSION,
        //     ])
        //     .map_err(ProtocolError::ValueError)?;
        //
        // let mut new_base_data_contract = new_data_contract_object.clone();
        // new_base_data_contract
        //     .remove_many(&vec![
        //         property_names::DEFINITIONS,
        //         property_names::DOCUMENTS,
        //         property_names::VERSION,
        //     ])
        //     .map_err(ProtocolError::ValueError)?;
        //
        // let base_data_contract_diff =
        //     platform_value::patch::diff(&existing_data_contract_object, &new_base_data_contract);
        //
        // for diff in base_data_contract_diff.0.iter() {
        //     let (operation, property_name) = get_operation_and_property_name(diff);
        //     validation_result.add_error(BasicError::DataContractImmutablePropertiesUpdateError(
        //         DataContractImmutablePropertiesUpdateError::new(
        //             operation.to_owned(),
        //             property_name.to_owned(),
        //             existing_data_contract_object
        //                 .get(property_name.split_at(1).1)
        //                 .ok()
        //                 .flatten()
        //                 .cloned()
        //                 .unwrap_or(Value::Null),
        //             new_base_data_contract
        //                 .get(property_name.split_at(1).1)
        //                 .ok()
        //                 .flatten()
        //                 .cloned()
        //                 .unwrap_or(Value::Null),
        //         ),
        //     ))
        // }
        // if !validation_result.is_valid() {
        //     return Ok(validation_result);
        // }
        //
        // // Schema should be backward compatible
        // let old_schema = &existing_data_contract.documents()?;
        // let new_schema: JsonValue = new_data_contract_object
        //     .get_value("documents")
        //     .map_err(ProtocolError::ValueError)?
        //     .clone()
        //     .try_into_validating_json() //maybe (not sure) / could be just try_into
        //     .map_err(ProtocolError::ValueError)?;
        //
        // for (document_type, document_schema) in old_schema.iter() {
        //     let new_document_schema = new_schema.get(document_type).unwrap_or(&EMPTY_JSON);
        //     let result = validate_schema_compatibility(document_schema, new_document_schema);
        //     match result {
        //         Ok(_) => {}
        //         Err(DiffValidatorError::SchemaCompatibilityError { diffs }) => {
        //             let (operation_name, property_name) =
        //                 get_operation_and_property_name_json(&diffs[0]);
        //             validation_result.add_error(BasicError::IncompatibleDataContractSchemaError(
        //                 IncompatibleDataContractSchemaError::new(
        //                     existing_data_contract.id(),
        //                     operation_name.to_owned(),
        //                     property_name.to_owned(),
        //                     document_schema.clone(),
        //                     new_document_schema.clone(),
        //                 ),
        //             ));
        //         }
        //         Err(DiffValidatorError::DataStructureError(e)) => {
        //             return Err(ProtocolError::ParsingError(e.to_string()).into())
        //         }
        //     }
        // }
        //
        // if !validation_result.is_valid() {
        //     return Ok(validation_result);
        // }

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
