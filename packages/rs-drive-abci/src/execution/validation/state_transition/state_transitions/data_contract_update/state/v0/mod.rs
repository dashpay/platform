use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use std::collections::BTreeSet;

use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::data_contract::data_contract_not_found_error::DataContractNotFoundError;
use dpp::consensus::state::group::IdentityMemberOfGroupNotFoundError;
use dpp::consensus::state::identity::identity_for_token_configuration_not_found_error::{
    IdentityInTokenConfigurationNotFoundError, TokenConfigurationIdentityContext,
};
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::InvalidTokenPositionStateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dpp::data_contract::validate_update::DataContractUpdateValidationMethodsV0;

use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::ValidationMode;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use drive::state_transition_action::StateTransitionAction;
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::execution::validation::state_transition::common::validate_non_masternode_identity_exists::validate_non_masternode_identity_exists;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionStateValidationV0 for DataContractUpdateTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut action = self.transform_into_action_v0(
            block_info,
            validation_mode,
            execution_context,
            platform_version,
        )?;

        if !action.is_valid() {
            return Ok(action);
        }

        let state_transition_action = action.data.as_mut().ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "we should always have an action at this point in data contract update",
            ),
        ))?;

        let new_data_contract = match state_transition_action {
            StateTransitionAction::DataContractUpdateAction(action) => {
                Some(action.data_contract_mut())
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

        let data_contract_fetch_info = drive.get_contract_with_fetch_info_and_fee(
            new_data_contract.id().to_buffer(),
            Some(&block_info.epoch),
            add_to_cache_if_pulled,
            tx,
            platform_version,
        )?;

        let fee = data_contract_fetch_info.0.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "fee must exist in validate state for data contract update transition",
            ),
        ))?;

        // We add the cost for fetching the contract even if the contract doesn't exist or was in cache
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        // Data contract should exist
        let Some(contract_fetch_info) = data_contract_fetch_info.1 else {
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

        // Here we do validations that consider the old data contract
        let validation_result =
            old_data_contract.validate_update(new_data_contract, block_info, platform_version)?;

        if !validation_result.is_valid() {
            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                    self,
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ));
        }

        new_data_contract.set_created_at(old_data_contract.created_at());
        new_data_contract.set_created_at_block_height(old_data_contract.created_at_block_height());
        new_data_contract.set_created_at_epoch(old_data_contract.created_at_epoch());

        let mut validated_identities = BTreeSet::new();

        for (position, group) in self.data_contract().groups() {
            for member_identity_id in group.members().keys() {
                if !validated_identities.contains(member_identity_id) {
                    let identity_exists = validate_non_masternode_identity_exists(
                        platform.drive,
                        member_identity_id,
                        execution_context,
                        tx,
                        platform_version,
                    )?;

                    if !identity_exists {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                                self,
                            ),
                        );
                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            vec![StateError::IdentityMemberOfGroupNotFoundError(
                                IdentityMemberOfGroupNotFoundError::new(
                                    self.data_contract().id(),
                                    *position,
                                    *member_identity_id,
                                ),
                            )
                            .into()],
                        ));
                    } else {
                        validated_identities.insert(*member_identity_id);
                    }
                }
            }
        }

        // Validate any newly added tokens
        for (token_contract_position, token_configuration) in new_data_contract.tokens() {
            if !old_data_contract
                .tokens()
                .contains_key(token_contract_position)
            {
                for (name, change_control_rules) in token_configuration.all_change_control_rules() {
                    if let AuthorizedActionTakers::Identity(identity_id) =
                        change_control_rules.authorized_to_make_change_action_takers()
                    {
                        // we need to make sure this identity exists
                        if !validated_identities.contains(identity_id) {
                            let identity_exists = validate_non_masternode_identity_exists(
                                platform.drive,
                                identity_id,
                                execution_context,
                                tx,
                                platform_version,
                            )?;

                            if !identity_exists {
                                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                                        self,
                                    ),
                                );

                                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                    bump_action,
                                    vec![StateError::IdentityInTokenConfigurationNotFoundError(
                                        IdentityInTokenConfigurationNotFoundError::new(
                                            old_data_contract.id(),
                                            *token_contract_position,
                                            TokenConfigurationIdentityContext::ChangeControlRule(
                                                name.to_string(),
                                            ),
                                            *identity_id,
                                        ),
                                    )
                                    .into()],
                                ));
                            } else {
                                validated_identities.insert(*identity_id);
                            }
                        }
                    }
                }

                if let Some(distribution) = token_configuration
                    .distribution_rules()
                    .perpetual_distribution()
                {
                    if let TokenDistributionRecipient::Identity(identifier) =
                        distribution.distribution_recipient()
                    {
                        if !validated_identities.contains(&identifier) {
                            let identity_exists = validate_identity_exists(
                                platform.drive,
                                &identifier,
                                execution_context,
                                tx,
                                platform_version,
                            )?;

                            if !identity_exists {
                                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                                        self,
                                    ),
                                );
                                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                    bump_action,
                                    vec![StateError::IdentityInTokenConfigurationNotFoundError(
                                        IdentityInTokenConfigurationNotFoundError::new(
                                            old_data_contract.id(),
                                            *token_contract_position,
                                            TokenConfigurationIdentityContext::PerpetualDistributionRecipient,
                                            identifier,
                                        ),
                                    )
                                        .into()],
                                ));
                            } else {
                                validated_identities.insert(identifier);
                            }
                        }
                    }
                }

                if let Some(distributions) = token_configuration
                    .distribution_rules()
                    .pre_programmed_distribution()
                {
                    for distribution in distributions.distributions().values() {
                        for identifier in distribution.keys() {
                            if !validated_identities.contains(identifier) {
                                let identity_exists = validate_identity_exists(
                                    platform.drive,
                                    identifier,
                                    execution_context,
                                    tx,
                                    platform_version,
                                )?;

                                if !identity_exists {
                                    let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                        BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                                            self,
                                        ),
                                    );
                                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                        bump_action,
                                        vec![StateError::IdentityInTokenConfigurationNotFoundError(
                                            IdentityInTokenConfigurationNotFoundError::new(
                                                old_data_contract.id(),
                                                *token_contract_position,
                                                TokenConfigurationIdentityContext::PreProgrammedDistributionRecipient,
                                                *identifier,
                                            ),
                                        )
                                            .into()],
                                    ));
                                } else {
                                    validated_identities.insert(*identifier);
                                }
                            }
                        }
                    }
                }

                // We validate that if we set a minting distribution that this identity exists
                // It can be an evonode, so we just use the balance as a check

                if let Some(minting_recipient) = token_configuration
                    .distribution_rules()
                    .new_tokens_destination_identity()
                {
                    if !validated_identities.contains(minting_recipient) {
                        let identity_exists = validate_identity_exists(
                            platform.drive,
                            minting_recipient,
                            execution_context,
                            tx,
                            platform_version,
                        )?;

                        if !identity_exists {
                            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                                    self,
                                ),
                            );

                            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                bump_action,
                                vec![StateError::IdentityInTokenConfigurationNotFoundError(
                                    IdentityInTokenConfigurationNotFoundError::new(
                                        old_data_contract.id(),
                                        *token_contract_position,
                                        TokenConfigurationIdentityContext::DefaultMintingRecipient,
                                        *minting_recipient,
                                    ),
                                )
                                .into()],
                            ));
                        } else {
                            validated_identities.insert(*minting_recipient);
                        }
                    }
                }
            }
        }

        // now we need to validate that all documents with token costs using external tokens
        // point to tokens that actually exist
        if let StateTransitionAction::DataContractUpdateAction(update_action) =
            action.data_as_borrowed()?
        {
            // this should always be the case, except if we already have a bump action,
            // in which case we don't need to validate anymore
            for document_type in update_action.data_contract_ref().document_types().values() {
                for (contract_id, token_positions) in
                    document_type.all_external_token_costs_contract_tokens()
                {
                    let contract_fetch_info = platform.drive.get_contract_with_fetch_info_and_fee(
                        contract_id.to_buffer(),
                        Some(&block_info.epoch),
                        false,
                        tx,
                        platform_version,
                    )?;

                    let fee =
                        contract_fetch_info
                            .0
                            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "fee must exist in validate state for data contract update transition",
                        )))?;

                    // We add the cost for fetching the contract even if the contract doesn't exist or was in cache
                    execution_context
                        .add_operation(ValidationOperation::PrecalculatedOperation(fee));

                    // Data contract should exist
                    if let Some(fetch_info) = contract_fetch_info.1 {
                        let contract_tokens = fetch_info.contract.tokens();
                        for token_position in &token_positions {
                            if !contract_tokens.contains_key(token_position) {
                                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                                        self,
                                    ),
                                );
                                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                    bump_action,
                                    vec![StateError::InvalidTokenPositionStateError(
                                        InvalidTokenPositionStateError::new(
                                            contract_tokens.last_key_value().map(
                                                |(token_contract_position, _)| {
                                                    *token_contract_position
                                                },
                                            ),
                                            *token_position,
                                        ),
                                    )
                                    .into()],
                                ));
                            }
                        }
                    } else {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                                self,
                            ),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            vec![StateError::DataContractNotFoundError(
                                DataContractNotFoundError::new(contract_id),
                            )
                            .into()],
                        ));
                    }
                }
            }
        }

        Ok(action)
    }

    fn transform_into_action_v0(
        &self,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_operations = vec![];

        let result = DataContractUpdateTransitionAction::try_from_borrowed_transition(
            self,
            block_info,
            validation_mode.should_fully_validate_contract_on_transform_into_action(),
            &mut validation_operations,
            platform_version,
        );

        execution_context.add_dpp_operations(validation_operations);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::errors::DataContractError;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::platform_value::Value;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceActionAccessorsV0;
    use platform_version::{DefaultForPlatformVersion, TryIntoPlatformVersioned};

    mod validate_state_v0 {
        use super::*;

        #[test]
        fn should_return_invalid_result_when_transform_into_action_failed() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let identity_id = data_contract.owner_id();
            let data_contract_id = data_contract.id();

            let mut data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            // Make the contract invalid
            let DataContractInSerializationFormat::V1(ref mut contract) =
                data_contract_for_serialization
            else {
                panic!("expected serialization version 1")
            };

            contract
                .document_schemas
                .insert("invalidType".to_string(), Value::Null);

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::ContractError(
                        DataContractError::InvalidContractStructure(message)
                    )
                )] if message == "document schema must be an object: structure error: value is not a map"
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == identity_id && action.identity_contract_nonce() == identity_contract_nonce && action.data_contract_id() == data_contract_id
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_return_invalid_result_when_data_contract_does_not_exist() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let identity_id = data_contract.owner_id();
            let data_contract_id = data_contract.id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractNotPresentError(e)
                )] if e.data_contract_id() == data_contract_id
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == identity_id && action.identity_contract_nonce() == identity_contract_nonce && action.data_contract_id() == data_contract_id
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_return_invalid_result_when_new_data_contract_has_incompatible_changes() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");

            let identity_id = data_contract.owner_id();
            let data_contract_id = data_contract.id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidDataContractVersionError(e)
                )] if e.expected_version() == 2 && e.version() == 1
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == identity_id && action.identity_contract_nonce() == identity_contract_nonce && action.data_contract_id() == data_contract_id
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_pass_when_contract_exists_and_update_is_compatible() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");

            let data_contract_id = data_contract.id();

            data_contract.set_version(2);

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(result.errors.as_slice(), []);

            assert_matches!(
                result.data,
                Some(StateTransitionAction::DataContractUpdateAction(action))
                if action.data_contract_ref().id() == data_contract_id
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_return_invalid_result_when_group_member_identity_does_not_exist() {
            use dpp::data_contract::accessors::v1::DataContractV1Getters;
            use dpp::data_contract::group::v0::GroupV0;
            use dpp::data_contract::group::Group;
            use dpp::identifier::Identifier;
            use dpp::identity::accessors::IdentityGettersV0;
            use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
            use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
            use rand::prelude::StdRng;
            use rand::SeedableRng;
            use simple_signer::signer::SimpleSigner;
            use std::collections::BTreeMap;

            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create and register the contract owner identity
            let mut signer = SimpleSigner::default();
            let mut rng = StdRng::seed_from_u64(42);
            let (master_key, master_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("expected key pair");
            signer.add_identity_public_key(master_key.clone(), master_private_key);

            let owner_identity: Identity = IdentityV0 {
                id: Identifier::random_with_rng(&mut rng),
                public_keys: BTreeMap::from([(0, master_key.clone())]),
                balance: 1_000_000_000,
                revision: 0,
            }
            .into();

            platform
                .drive
                .add_new_identity(
                    owner_identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add identity");

            // Create initial contract without groups
            let mut data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            data_contract.set_owner_id(owner_identity.id());

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");

            // Update contract: add a group with a non-existent member identity
            let non_existent_id = Identifier::from([99u8; 32]);
            data_contract.set_version(2);
            {
                let groups = data_contract.groups_mut().expect("expected groups");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [
                            (owner_identity.id(), 1),
                            (non_existent_id, 1), // does not exist
                        ]
                        .into(),
                        required_power: 1,
                    }),
                );
            }

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("expected validation to succeed");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::IdentityMemberOfGroupNotFoundError(_)
                )]
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(
                    _
                ))
            );
        }

        #[test]
        fn should_return_invalid_result_when_token_perpetual_distribution_recipient_not_found() {
            use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
            use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
            use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
            use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
            use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
            use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
            use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
            use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
            use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
            use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
            use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
            use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
            use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
            use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
            use dpp::identifier::Identifier;
            use std::collections::BTreeMap;

            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create initial contract without tokens
            let mut data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");

            // Update contract: add a token with perpetual distribution to a non-existent identity
            data_contract.set_version(2);

            let non_existent_id = Identifier::from([77u8; 32]);

            let mut token_cfg =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            token_cfg.set_base_supply(1_000);

            token_cfg.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "coin".to_string(),
                            plural_form: "coins".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));

            // Set perpetual distribution to a non-existent identity
            token_cfg
                .distribution_rules_mut()
                .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                    TokenPerpetualDistributionV0 {
                        distribution_type: RewardDistributionType::BlockBasedDistribution {
                            interval: 200,
                            function: DistributionFunction::FixedAmount { amount: 100 },
                        },
                        distribution_recipient: TokenDistributionRecipient::Identity(
                            non_existent_id,
                        ),
                    },
                )));

            data_contract.add_token(0, token_cfg);

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("expected validation to succeed");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::IdentityInTokenConfigurationNotFoundError(_)
                )]
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(
                    _
                ))
            );
        }

        #[test]
        fn should_return_invalid_result_when_token_pre_programmed_distribution_recipient_not_found()
        {
            use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
            use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
            use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
            use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
            use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
            use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
            use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
            use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
            use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
            use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
            use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
            use dpp::identifier::Identifier;
            use std::collections::BTreeMap;

            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create initial contract without tokens
            let mut data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");

            // Update contract: add token with pre-programmed distribution to non-existent identity
            data_contract.set_version(2);

            let non_existent_id = Identifier::from([88u8; 32]);

            let mut token_cfg =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            token_cfg.set_base_supply(1_000);

            token_cfg.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "coin".to_string(),
                            plural_form: "coins".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));

            // Set pre-programmed distribution with non-existent identity
            let distributions = BTreeMap::from([(
                1_000_000u64, // timestamp
                BTreeMap::from([(non_existent_id, 500u64)]),
            )]);

            token_cfg
                .distribution_rules_mut()
                .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                    TokenPreProgrammedDistributionV0 { distributions },
                )));

            data_contract.add_token(0, token_cfg);

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("expected validation to succeed");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::IdentityInTokenConfigurationNotFoundError(_)
                )]
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(
                    _
                ))
            );
        }

        #[test]
        fn should_return_invalid_result_when_minting_destination_identity_not_found() {
            use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
            use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
            use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
            use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
            use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
            use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
            use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
            use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
            use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
            use dpp::identifier::Identifier;
            use std::collections::BTreeMap;

            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create initial contract without tokens
            let mut data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");

            // Update contract: add token with minting destination set to non-existent identity
            data_contract.set_version(2);

            let non_existent_id = Identifier::from([66u8; 32]);

            let mut token_cfg =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            token_cfg.set_base_supply(1_000);

            token_cfg.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "coin".to_string(),
                            plural_form: "coins".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));

            // Set the new_tokens_destination_identity to a non-existent identity
            token_cfg
                .distribution_rules_mut()
                .set_new_tokens_destination_identity(Some(non_existent_id));

            data_contract.add_token(0, token_cfg);

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("expected validation to succeed");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::IdentityInTokenConfigurationNotFoundError(_)
                )]
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(
                    _
                ))
            );
        }

        #[test]
        fn should_pass_when_group_members_and_token_identities_all_exist() {
            use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
            use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
            use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
            use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
            use dpp::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
            use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
            use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
            use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
            use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
            use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
            use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
            use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
            use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
            use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
            use dpp::data_contract::group::v0::GroupV0;
            use dpp::data_contract::group::Group;
            use dpp::identifier::Identifier;
            use dpp::identity::accessors::IdentityGettersV0;
            use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
            use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
            use rand::prelude::StdRng;
            use rand::SeedableRng;
            use simple_signer::signer::SimpleSigner;
            use std::collections::BTreeMap;

            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create owner identity
            let mut rng = StdRng::seed_from_u64(42);
            let owner_id = Identifier::random_with_rng(&mut rng);
            let owner_identity: Identity = IdentityV0 {
                id: owner_id,
                public_keys: BTreeMap::new(),
                balance: 1_000_000_000,
                revision: 0,
            }
            .into();
            platform
                .drive
                .add_new_identity(
                    owner_identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add identity");

            // Create a second identity (group member + distribution recipient)
            let member_id = Identifier::random_with_rng(&mut rng);
            let member_identity: Identity = IdentityV0 {
                id: member_id,
                public_keys: BTreeMap::new(),
                balance: 500_000_000,
                revision: 0,
            }
            .into();
            platform
                .drive
                .add_new_identity(
                    member_identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add identity");

            // Create initial contract
            let mut data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            data_contract.set_owner_id(owner_id);

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");

            // Update: add group + token with perpetual distribution to existing identity
            data_contract.set_version(2);
            {
                let groups = data_contract.groups_mut().expect("expected groups");
                groups.insert(
                    0,
                    Group::V0(GroupV0 {
                        members: [(owner_id, 1), (member_id, 1)].into(),
                        required_power: 1,
                    }),
                );
            }

            let mut token_cfg =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            token_cfg.set_base_supply(1_000);
            token_cfg.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: true,
                            singular_form: "coin".to_string(),
                            plural_form: "coins".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));

            // Set perpetual distribution to existing identity
            token_cfg
                .distribution_rules_mut()
                .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                    TokenPerpetualDistributionV0 {
                        distribution_type: RewardDistributionType::BlockBasedDistribution {
                            interval: 200,
                            function: DistributionFunction::FixedAmount { amount: 10 },
                        },
                        distribution_recipient: TokenDistributionRecipient::Identity(member_id),
                    },
                )));

            // Set minting destination to existing identity
            token_cfg
                .distribution_rules_mut()
                .set_new_tokens_destination_identity(Some(owner_id));

            data_contract.add_token(0, token_cfg);

            let data_contract_id = data_contract.id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let state = platform.state.load_full();

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .validate_state_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                    platform_version,
                )
                .expect("expected validation to succeed");

            assert_matches!(result.errors.as_slice(), []);

            assert_matches!(
                result.data,
                Some(StateTransitionAction::DataContractUpdateAction(action))
                if action.data_contract_ref().id() == data_contract_id
            );
        }
    }

    mod transform_into_action_v0 {
        use super::*;

        #[test]
        fn should_return_invalid_result_when_new_data_contract_is_not_valid() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let identity_id = data_contract.owner_id();
            let data_contract_id = data_contract.id();

            let mut data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            // Make the contract invalid
            let DataContractInSerializationFormat::V1(ref mut contract) =
                data_contract_for_serialization
            else {
                panic!("expected serialization version 1")
            };

            contract
                .document_schemas
                .insert("invalidType".to_string(), Value::Null);

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let result = transition
                .transform_into_action_v0(
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::ContractError(
                        DataContractError::InvalidContractStructure(message)
                    )
                )] if message == "document schema must be an object: structure error: value is not a map"
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityDataContractNonceAction(action))
                if action.identity_id() == identity_id && action.identity_contract_nonce() == identity_contract_nonce && action.data_contract_id() == data_contract_id
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_pass_when_new_data_contract_is_valid() {
            let platform_version = PlatformVersion::latest();
            let identity_contract_nonce = IdentityNonce::default();

            let data_contract = get_data_contract_fixture(
                None,
                identity_contract_nonce,
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let data_contract_id = data_contract.id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractUpdateTransition = DataContractUpdateTransitionV0 {
                identity_contract_nonce,
                data_contract: data_contract_for_serialization,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let result = transition
                .transform_into_action_v0(
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    &mut execution_context,
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(result.errors.as_slice(), []);

            assert_matches!(
                result.data,
                Some(StateTransitionAction::DataContractUpdateAction(action)) if action.data_contract_ref().id() == data_contract_id
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }
    }
}
