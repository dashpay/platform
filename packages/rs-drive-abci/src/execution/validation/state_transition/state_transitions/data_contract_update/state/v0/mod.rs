use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

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
    get_operation_and_property_name_json, validate_schema_compatibility,
};
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::JsonValue;
use dpp::platform_value::converter::serde_json::BTreeValueJsonConverter;
use dpp::platform_value::{Value, ValueMap};

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::ProtocolError;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::{PlatformVersion, TryIntoPlatformVersioned};

use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::ValidationMode;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
        validation_mode: ValidationMode,
        platform_version: &PlatformVersion
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionStateValidationV0 for DataContractUpdateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action = self.transform_into_action_v0(validation_mode, platform_version)?;

        if !action.is_valid() {
            return Ok(action);
        }

        let state_transition_action = action.data.as_ref().ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "we should always have an action at this point in data contract update",
            ),
        ))?;

        let new_data_contract = match state_transition_action {
            StateTransitionAction::DataContractUpdateAction(action) => {
                Some(action.data_contract_ref())
            }
            _ => None,
        }
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "we should always have a data contract at this point in data contract update",
        )))?;

        let drive = platform.drive;
        let mut validation_result = ConsensusValidationResult::default();
        // Data contract should exist
        let add_to_cache_if_pulled = validation_mode.can_alter_cache();
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

            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                )?,
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        };

        let old_data_contract = &contract_fetch_info.contract;

        let new_version = new_data_contract.version();
        let old_version = old_data_contract.version();
        if new_version < old_version || new_version - old_version != 1 {
            validation_result.add_error(BasicError::InvalidDataContractVersionError(
                InvalidDataContractVersionError::new(old_version + 1, new_version),
            ));
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                )?,
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        let config_validation_result = old_data_contract.config().validate_config_update(
            new_data_contract.config(),
            self.data_contract().id(),
            platform_version,
        )?;

        if !config_validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                )?,
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                config_validation_result.errors,
            ));
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
                // then new indices on it are fine and we don't need to validate that configuration didn't change
                continue;
            };

            let validate_update_result = old_contract_document_type
                .validate_update(new_contract_document_type, platform_version)?;

            if !validate_update_result.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                        self,
                    )?,
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validate_update_result.errors,
                ));
            }

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

            if !validation_result.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                        self,
                    )?,
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result.errors,
                ));
            }
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
                        old_defs.into(),
                        Value::Null,
                    ),
                ));

                return Ok(validation_result);
            };

            let old_defs_json: JsonValue = old_defs
                .to_json_value()
                .map_err(ProtocolError::ValueError)?;

            let new_defs_json: JsonValue = new_defs
                .to_json_value()
                .map_err(ProtocolError::ValueError)?;

            let diffs =
                validate_schema_compatibility(&old_defs_json, &new_defs_json, platform_version)?;

            if !diffs.is_empty() {
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

                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                        self,
                    )?,
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result.errors,
                ));
            }
        }

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

            if !diffs.is_empty() {
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

                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                        self,
                    )?,
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validation_result.errors,
                ));
            }
        }

        Ok(action)
    }

    fn transform_into_action_v0(
        &self,
        validation_mode: ValidationMode,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let result = DataContractUpdateTransitionAction::try_from_borrowed_transition(
            self,
            validation_mode.should_validate_contract_on_transform_into_action(),
            platform_version,
        );

        // Return validation result if any consensus errors happened
        // during data contract validation
        match result {
            Err(ProtocolError::ConsensusError(consensus_error)) => {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self)?,
                );

                Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    vec![*consensus_error],
                ))
            }
            Err(protocol_error) => Err(protocol_error.into()),
            Ok(create_action) => {
                let action: StateTransitionAction = create_action.into();
                Ok(action.into())
            }
        }
    }
}
