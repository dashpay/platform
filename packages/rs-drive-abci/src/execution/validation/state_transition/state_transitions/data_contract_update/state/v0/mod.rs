use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::basic::data_contract::{
    DataContractInvalidIndexDefinitionUpdateError, IncompatibleDataContractSchemaError,
    InvalidDataContractVersionError,
};
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;

use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::schema::{
    get_operation_and_property_name_json, validate_schema_compatibility,
};
use dpp::data_contract::errors::DataContractError;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::JsonValue;
use dpp::platform_value::converter::serde_json::BTreeValueJsonConverter;
use dpp::platform_value::{Value, ValueMap};

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::ProtocolError;

use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::PlatformVersion;

use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::ValidationMode;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

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

        // Check previous data contract already exists in the state
        // Failure (contract does not exist): Keep ST and transform it to a nonce bump action.
        // How: A user pushed an update for a data contract that didn’t exist.
        // Note: Existing in the state can also mean that it exists in the current block state, meaning that the contract was inserted in the same block with a previous transition.

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
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![
                    BasicError::DataContractNotPresentError(DataContractNotPresentError::new(
                        new_data_contract.id(),
                    ))
                    .into(),
                ],
            ));
        };

        let old_data_contract = &contract_fetch_info.contract;

        // Check version is bumped
        // Failure (version != previous version + 1): Keep ST and transform it to a nonce bump action.
        // How: A user pushed an update that was not the next version.

        let new_version = new_data_contract.version();
        let old_version = old_data_contract.version();
        if new_version < old_version || new_version - old_version != 1 {
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![BasicError::InvalidDataContractVersionError(
                    InvalidDataContractVersionError::new(old_version + 1, new_version),
                )
                .into()],
            ));
        }

        // Validate that the config was not updated
        // * Includes verifications that:
        //     - Old contract is not read_only
        //     - New contract is not read_only
        //     - Keeps history did not change
        //     - Can be deleted did not change
        //     - Documents keep history did not change
        //     - Documents mutable contract default did not change
        //     - Requires identity encryption bounded key did not change
        //     - Requires identity decryption bounded key did not change
        // * Failure (contract does not exist): Keep ST and transform it to a nonce bump action.
        // * How: A user pushed an update to a contract that changed its configuration.

        let config_validation_result = old_data_contract.config().validate_config_update(
            new_data_contract.config(),
            self.data_contract().id(),
            platform_version,
        )?;

        if !config_validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
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
                ),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    validate_update_result.errors,
                ));
            }

            // We currently don't allow indexes to change
            if new_contract_document_type.index_structure()
                != old_contract_document_type.index_structure()
            {
                // We want to figure out what changed, so we compare one way then the other

                // If the new contract document type doesn't contain all previous indexes
                if let Some(non_subset_path) = new_contract_document_type
                    .index_structure()
                    .contains_subset_first_non_subset_path(
                        old_contract_document_type.index_structure(),
                    )
                {
                    let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                        BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                            self,
                        ),
                    );

                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        bump_action,
                        vec![BasicError::DataContractInvalidIndexDefinitionUpdateError(
                            DataContractInvalidIndexDefinitionUpdateError::new(
                                new_contract_document_type_name.clone(),
                                non_subset_path,
                            ),
                        )
                        .into()],
                    ));
                }

                // If the old contract document type doesn't contain all new indexes
                if let Some(non_subset_path) = old_contract_document_type
                    .index_structure()
                    .contains_subset_first_non_subset_path(
                        new_contract_document_type.index_structure(),
                    )
                {
                    let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                        BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                            self,
                        ),
                    );

                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        bump_action,
                        vec![BasicError::DataContractInvalidIndexDefinitionUpdateError(
                            DataContractInvalidIndexDefinitionUpdateError::new(
                                new_contract_document_type_name.clone(),
                                non_subset_path,
                            ),
                        )
                        .into()],
                    ));
                }
            }
        }

        // Schema defs should be compatible

        // TODO: WE need to combine defs with documents schema and and resolve all refs
        //  Having such full schema we can make sure that changes in defs are actually
        //  affect document schema. Current simplified solution just apply the same logic
        //  as for document schema
        if let Some(old_defs) = old_data_contract.schema_defs() {
            let Some(new_defs) = self.data_contract().schema_defs() else {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    vec![BasicError::IncompatibleDataContractSchemaError(
                        IncompatibleDataContractSchemaError::new(
                            self.data_contract().id(),
                            "remove".to_string(),
                            "$defs".to_string(),
                            old_defs.into(),
                            Value::Null,
                        ),
                    )
                    .into()],
                ));
            };

            // Old defs are in state so should be valid
            let old_defs_json: JsonValue = old_defs
                .to_json_value()
                .map_err(ProtocolError::ValueError)?;

            let new_defs_json: JsonValue = match new_defs.to_json_value() {
                Ok(json_value) => json_value,
                Err(e) => {
                    let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                        BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                            self,
                        ),
                    );

                    let data_contract_error: DataContractError =
                        (e, "json schema new defs invalid").into();

                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        bump_action,
                        vec![ConsensusError::BasicError(BasicError::ContractError(
                            data_contract_error,
                        ))],
                    ));
                }
            };

            let diffs =
                validate_schema_compatibility(&old_defs_json, &new_defs_json, platform_version)?;

            if !diffs.is_empty() {
                let (operation_name, property_name) =
                    get_operation_and_property_name_json(&diffs[0]);

                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    vec![BasicError::IncompatibleDataContractSchemaError(
                        IncompatibleDataContractSchemaError::new(
                            self.data_contract().id(),
                            operation_name.to_owned(),
                            property_name.to_owned(),
                            old_defs_json.into(),
                            new_defs_json.into(),
                        ),
                    )
                    .into()],
                ));
            }
        }

        for (document_type_name, old_document_schema) in old_data_contract.document_schemas() {
            // The old document schema is in the state already so we are guaranteed that it can be transformed into a JSON value
            let old_document_schema_json: JsonValue = old_document_schema
                .clone()
                .try_into()
                .map_err(ProtocolError::ValueError)?;

            let new_document_schema = new_data_contract
                .document_type_optional_for_name(&document_type_name)
                .map(|document_type| document_type.schema().clone())
                .unwrap_or(ValueMap::new().into());

            // The new document schema is not the state already so we are not guaranteed that it can be transformed into a JSON value
            // If it can not we should throw a consensus validation error
            let new_document_schema_json: JsonValue = match new_document_schema.clone().try_into() {
                Ok(json_value) => json_value,
                Err(e) => {
                    let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                        BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                            self,
                        ),
                    );

                    let data_contract_error: DataContractError =
                        (e, "json schema new schema invalid").into();

                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        bump_action,
                        vec![ConsensusError::BasicError(BasicError::ContractError(
                            data_contract_error,
                        ))],
                    ));
                }
            };

            let diffs = validate_schema_compatibility(
                &old_document_schema_json,
                &new_document_schema_json,
                platform_version,
            )?;

            if !diffs.is_empty() {
                let (operation_name, property_name) =
                    get_operation_and_property_name_json(&diffs[0]);

                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
                );

                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    vec![BasicError::IncompatibleDataContractSchemaError(
                        IncompatibleDataContractSchemaError::new(
                            self.data_contract().id(),
                            operation_name.to_owned(),
                            property_name.to_owned(),
                            old_document_schema.clone(),
                            new_document_schema,
                        ),
                    )
                    .into()],
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
                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self),
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
