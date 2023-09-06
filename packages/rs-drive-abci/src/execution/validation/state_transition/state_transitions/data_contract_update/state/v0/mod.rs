use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use serde::Serialize;

use dpp::consensus::basic::data_contract::{
    DataContractInvalidIndexDefinitionUpdateError, IncompatibleDataContractSchemaError,
    InvalidDataContractVersionError,
};
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::data_contract::config::v0::DataContractConfigGettersV0;

use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::schema::{
    get_operation_and_property_name_json, validate_schema_compatibility, EMPTY_JSON,
};
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::JsonValue;
use dpp::platform_value::converter::serde_json::{
    BTreeValueJsonConverter, BTreeValueRefJsonConverter,
};
use dpp::platform_value::{Value, ValueMap};

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::ProtocolError;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::{PlatformVersion, TryIntoPlatformVersioned};

use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
        platform_version: &PlatformVersion
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionStateValidationV0 for DataContractUpdateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action = self.transform_into_action_v0(platform_version)?;

        if !action.is_valid_with_data() {
            return Ok(action);
        }

        let state_transition_action = action.data.as_ref().unwrap();

        let new_data_contract = match state_transition_action {
            StateTransitionAction::DataContractUpdateAction(action) => {
                Some(action.data_contract_ref())
            }
            _ => None,
        }
        .unwrap();

        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::default();
        // Data contract should exist
        let add_to_cache_if_pulled = tx.is_some();
        // Data contract should exist
        let Some(contract_fetch_info) = drive
            .get_contract_with_fetch_info_and_fee(
                new_data_contract.id().to_buffer(),
                None,
                add_to_cache_if_pulled,
                tx,
                platform_version,
            )?
            .1
        else {
            validation_result.add_error(BasicError::DataContractNotPresentError(
                DataContractNotPresentError::new(new_data_contract.id()),
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

        let config_validation_result = old_data_contract.config().validate_config_update(
            new_data_contract.config(),
            self.data_contract().id(),
            platform_version,
        )?;

        if !config_validation_result.is_valid() {
            validation_result.merge(config_validation_result);
            return Ok(validation_result);
        }

        // We should now validate that new indexes contains all old indexes
        // This is most easily done by using the index level construct

        for (new_contract_document_type_name, new_contract_document_type) in
            new_data_contract.document_types()
        {
            let Some(old_contract_document_type) =
                old_data_contract.document_type_optional_for_name(new_contract_document_type_name)
            else {
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

        // Schema defs should be compatible

        // TODO: WE need to combine defs with documents schema and and resolve all refs
        //  Having such full schema we can make sure that changes in defs are actually
        //  affect document schema. Current simplified solution just apply the same logic
        //  as for document schema
        if let Some(old_defs) = old_data_contract.schema_defs() {
            let Some(new_defs) = self.data_contract().schema_defs() else {
                validation_result.add_error(BasicError::IncompatibleDataContractSchemaError(
                    IncompatibleDataContractSchemaError::new(
                        self.data_contract().id(),
                        "remove".to_string(),
                        "$defs".to_string(),
                        old_defs.clone(),
                        Value::Null,
                    ),
                ));

                return Ok(validation_result);
            };

            let old_defs_json: JsonValue = old_defs
                .clone()
                .try_into_validating_json()
                .map_err(ProtocolError::ValueError)?;

            let new_defs_json: JsonValue = new_defs
                .clone()
                .try_into_validating_json()
                .map_err(ProtocolError::ValueError)?;

            let diffs =
                validate_schema_compatibility(&old_defs_json, &new_defs_json, platform_version)?;

            if diffs.len() > 0 {
                let (operation_name, property_name) =
                    get_operation_and_property_name_json(&diffs[0]);

                validation_result.add_error(BasicError::IncompatibleDataContractSchemaError(
                    IncompatibleDataContractSchemaError::new(
                        self.data_contract().id(),
                        operation_name.to_owned(),
                        property_name.to_owned(),
                        old_defs_json.into(),
                        new_defs_json.into(),
                    ),
                ));

                return Ok(validation_result);
            }
        }

        // Document schemas should be backward compatible
        let new_schema = new_data_contract
            .document_schemas()
            .to_json_value()
            .map_err(ProtocolError::ValueError)?;

        for (document_type_name, old_document_schema) in old_data_contract.document_schemas() {
            let old_document_schema_json: JsonValue = old_document_schema
                .clone()
                .try_into()
                .map_err(ProtocolError::ValueError)?;

            let new_document_schema = new_data_contract
                .document_type_optional_for_name(&document_type_name)
                .map(|document_type| document_type.schema().clone())
                .unwrap_or(ValueMap::new().into());

            let new_document_schema_json: JsonValue = new_document_schema
                .clone()
                .try_into()
                .map_err(ProtocolError::ValueError)?;

            let diffs = validate_schema_compatibility(
                &old_document_schema_json,
                &new_document_schema_json,
                platform_version,
            )?;

            if diffs.len() > 0 {
                let (operation_name, property_name) =
                    get_operation_and_property_name_json(&diffs[0]);

                validation_result.add_error(BasicError::IncompatibleDataContractSchemaError(
                    IncompatibleDataContractSchemaError::new(
                        self.data_contract().id(),
                        operation_name.to_owned(),
                        property_name.to_owned(),
                        old_document_schema.clone(),
                        new_document_schema,
                    ),
                ));

                return Ok(validation_result);
            }
        }

        Ok(action)
    }

    fn transform_into_action_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action: StateTransitionAction = TryIntoPlatformVersioned::<
            DataContractUpdateTransitionAction,
        >::try_into_platform_versioned(
            self, platform_version
        )?
        .into();
        Ok(action.into())
    }
}
