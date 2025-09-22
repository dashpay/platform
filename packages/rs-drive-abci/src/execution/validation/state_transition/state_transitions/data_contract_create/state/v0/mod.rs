use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use std::collections::BTreeSet;

use dpp::consensus::state::data_contract::data_contract_already_present_error::DataContractAlreadyPresentError;
use dpp::consensus::state::data_contract::data_contract_not_found_error::DataContractNotFoundError;
use dpp::consensus::state::group::IdentityMemberOfGroupNotFoundError;
use dpp::consensus::state::identity::identity_for_token_configuration_not_found_error::{
    IdentityInTokenConfigurationNotFoundError, TokenConfigurationIdentityContext,
};
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{
    InvalidTokenPositionStateError, PreProgrammedDistributionTimestampInPastError,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::document_type::accessors::DocumentTypeV1Getters;
use dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::ProtocolError;

use crate::error::execution::ExecutionError;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::ValidationMode;
use dpp::version::PlatformVersion;
use drive::drive::subscriptions::{DriveSubscriptionFilter, HitFiltersType};
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::transform_to_state_transition_action_result::TransformToStateTransitionActionResult;
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::execution::validation::state_transition::common::validate_non_masternode_identity_exists::validate_non_masternode_identity_exists;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionStateValidationV0 {
    fn validate_state_v0<'a, C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error>;

    fn transform_into_action_v0<'a, C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error>;
}

impl DataContractCreateStateTransitionStateValidationV0 for DataContractCreateTransition {
    fn validate_state_v0<'a, C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        // These are the filters that have already shown that this transition is a match
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error> {
        let action = self.transform_into_action_v0::<C>(
            block_info,
            platform,
            validation_mode,
            tx,
            execution_context,
            passing_filters_for_transition,
            platform_version,
        )?;

        if !action.is_valid() {
            return Ok(action);
        }

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
                        return Ok(ConsensusValidationResult::new_with_data_into_and_errors(
                            StateTransitionAction::BumpIdentityNonceAction(
                                BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
                            ),
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

        // Validate token distribution rules
        for (position, config) in self.data_contract().tokens() {
            // We validate that if for any change control rule set to an identity that that identity exists
            // and can sign state transition (not an evonode identity)
            for (name, change_control_rules) in config.all_change_control_rules() {
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
                            return Ok(ConsensusValidationResult::new_with_data_into_and_errors(
                                StateTransitionAction::BumpIdentityNonceAction(
                                    BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
                                ),
                                vec![StateError::IdentityInTokenConfigurationNotFoundError(
                                    IdentityInTokenConfigurationNotFoundError::new(
                                        self.data_contract().id(),
                                        *position,
                                        TokenConfigurationIdentityContext::ChangeControlRule(name.to_string()),
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
            // We validate that if we set a minting distribution that this identity exists
            // It can be an evonode, so we just use the balance as a check

            if let Some(minting_recipient) = config
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
                        return Ok(ConsensusValidationResult::new_with_data_into_and_errors(
                            StateTransitionAction::BumpIdentityNonceAction(
                                BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(
                                    self,
                                ),
                            ),
                            vec![StateError::IdentityInTokenConfigurationNotFoundError(
                                IdentityInTokenConfigurationNotFoundError::new(
                                    self.data_contract().id(),
                                    *position,
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

            if let Some(distribution) = config.distribution_rules().perpetual_distribution() {
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
                            return Ok(ConsensusValidationResult::new_with_data_into_and_errors(
                                StateTransitionAction::BumpIdentityNonceAction(
                                    BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(
                                        self,
                                    ),
                                ),
                                vec![StateError::IdentityInTokenConfigurationNotFoundError(
                                    IdentityInTokenConfigurationNotFoundError::new(
                                        self.data_contract().id(),
                                        *position,
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

            if let Some(distribution) = config.distribution_rules().pre_programmed_distribution() {
                if let Some((timestamp, _)) = distribution.distributions().iter().next() {
                    if timestamp < &block_info.time_ms {
                        return Ok(ConsensusValidationResult::new_with_data_into_and_errors(
                            StateTransitionAction::BumpIdentityNonceAction(
                                BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
                            ),
                            vec![StateError::PreProgrammedDistributionTimestampInPastError(
                                PreProgrammedDistributionTimestampInPastError::new(self.data_contract().id(), *position, *timestamp, block_info.time_ms),
                            )
                                .into()],
                        ));
                    }
                }
                for distribution in distribution.distributions().values() {
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
                                return Ok(ConsensusValidationResult::new_with_data_into_and_errors(
                                    StateTransitionAction::BumpIdentityNonceAction(
                                        BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(
                                            self,
                                        ),
                                    ),
                                    vec![StateError::IdentityInTokenConfigurationNotFoundError(
                                        IdentityInTokenConfigurationNotFoundError::new(
                                            self.data_contract().id(),
                                            *position,
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
        }

        let contract_fetch_info = platform.drive.get_contract_with_fetch_info_and_fee(
            self.data_contract().id().to_buffer(),
            Some(&block_info.epoch),
            false,
            tx,
            platform_version,
        )?;

        let fee = contract_fetch_info.0.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "fee must exist in validate state for data contract create transition",
            ),
        ))?;

        // We add the cost for fetching the contract even if the contract doesn't exist or was in cache
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        // Data contract shouldn't exist
        if contract_fetch_info.1.is_some() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_into_and_errors(
                bump_action,
                vec![StateError::DataContractAlreadyPresentError(
                    DataContractAlreadyPresentError::new(self.data_contract().id().to_owned()),
                )
                .into()],
            ));
        }

        // now we need to validate that all documents with token costs using external tokens
        // point to tokens that actually exist
        if let StateTransitionAction::DataContractCreateAction(create_action) =
            action.data_as_borrowed()?
        {
            // this should always be the case, except if we already have a bump action,
            // in which case we don't need to validate anymore
            for document_type in create_action.data_contract_ref().document_types().values() {
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
                            "fee must exist in validate state for data contract create transition",
                        )))?;

                    // We add the cost for fetching the contract even if the contract doesn't exist or was in cache
                    execution_context
                        .add_operation(ValidationOperation::PrecalculatedOperation(fee));

                    // Data contract should exist
                    if let Some(fetch_info) = contract_fetch_info.1 {
                        let contract_tokens = fetch_info.contract.tokens();
                        for token_position in &token_positions {
                            if !contract_tokens.contains_key(token_position) {
                                return Ok(ConsensusValidationResult::new_with_data_into_and_errors(
                                    StateTransitionAction::BumpIdentityNonceAction(
                                        BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
                                    ),
                                    vec![StateError::InvalidTokenPositionStateError(
                                        InvalidTokenPositionStateError::new(
                                            contract_tokens.last_key_value().map(|(token_contract_position,_)| *token_contract_position),
                                            *token_position,
                                        ),
                                    )
                                        .into()],
                                ));
                            }
                        }
                    } else {
                        let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                            BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(
                                self,
                            ),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_into_and_errors(
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

    fn transform_into_action_v0<'a, C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        // For data contract create we can only have passing filters, as we would never need the original
        passing_filters_for_transition: &[&'a DriveSubscriptionFilter],
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<TransformToStateTransitionActionResult<'a>>, Error> {
        let mut validation_operations = vec![];

        // The transformation of the state transition into the state transition action will transform
        // The contract in serialized form into it's execution form
        let result = DataContractCreateTransitionAction::try_from_borrowed_transition(
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
                let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                    BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
                );

                Ok(ConsensusValidationResult::new_with_data_into_and_errors(
                    bump_action,
                    vec![*consensus_error],
                ))
            }
            Err(protocol_error) => Err(protocol_error.into()),
            Ok(create_action) => {
                let action: StateTransitionAction = create_action.into();
                if !passing_filters_for_transition.is_empty() {
                    // We have filters on this data contract create, we should get the grovedb proof that
                    // nothing existed before
                    if let Ok(original_grovedb_proof) = platform.drive.prove_contract(
                        self.data_contract().id().to_buffer(),
                        tx,
                        platform_version,
                    ) {
                        let action_result = TransformToStateTransitionActionResult {
                            action,
                            filters_hit: HitFiltersType::DidHitFilters {
                                original_grovedb_proof,
                                filters_hit: passing_filters_for_transition.to_vec(),
                            },
                        };
                        return Ok(action_result.into());
                    }
                    // if the contract proving fails just say we didn't get any action results
                }
                let action_result: TransformToStateTransitionActionResult = action.into();
                Ok(action_result.into())
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
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::errors::DataContractError;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::platform_value::Value;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceActionAccessorsV0;
    use platform_version::{DefaultForPlatformVersion, TryIntoPlatformVersioned};

    mod validate_state_v0 {
        use super::*;

        #[test]
        fn should_return_invalid_result_when_transform_into_action_failed_v7() {
            let platform_version = PlatformVersion::get(7).expect("expected version 7");
            let identity_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            let identity_id = data_contract.owner_id();

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

            let mut data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            // Make the contract invalid
            let DataContractInSerializationFormat::V0(ref mut contract) =
                data_contract_for_serialization
            else {
                panic!("expected serialization version 0")
            };

            contract
                .document_schemas
                .insert("invalidType".to_string(), Value::Null);

            let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
                data_contract: data_contract_for_serialization,
                identity_nonce,
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
                    None,
                    &mut execution_context,
                    &vec![],
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
                Some(StateTransitionAction::BumpIdentityNonceAction(action)) if action.identity_id() == identity_id && action.identity_nonce() == identity_nonce
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_return_invalid_result_when_transform_into_action_failed_latest() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            let identity_id = data_contract.owner_id();

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

            let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
                data_contract: data_contract_for_serialization,
                identity_nonce,
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
                    None,
                    &mut execution_context,
                    &vec![],
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
                Some(StateTransitionAction::BumpIdentityNonceAction(action)) if action.identity_id() == identity_id && action.identity_nonce() == identity_nonce
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_return_invalid_result_when_contract_with_specified_id_already_exists() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            let identity_id = data_contract.owner_id();
            let data_contract_id = data_contract.id();

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

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
                data_contract: data_contract_for_serialization,
                identity_nonce,
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
                    None,
                    &mut execution_context,
                    &vec![],
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DataContractAlreadyPresentError(e))] if *e.data_contract_id() == data_contract_id
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityNonceAction(action))
                if action.identity_id() == identity_id && action.identity_nonce() == identity_nonce
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_pass_when_data_contract_valid_and_does_not_exist() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            let data_contract_id = data_contract.id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
                data_contract: data_contract_for_serialization,
                identity_nonce,
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
                    None,
                    &mut execution_context,
                    &vec![],
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(result.errors.as_slice(), []);

            assert_matches!(
                result.data,
                Some(StateTransitionAction::DataContractCreateAction(action)) if action.data_contract_ref().id() == data_contract_id
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }
    }

    mod transform_into_action_v0 {
        use super::*;

        #[test]
        fn should_return_invalid_result_if_data_contract_is_not_valid() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = IdentityNonce::default();

            let data_contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            let identity_id = data_contract.owner_id();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

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

            let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
                data_contract: data_contract_for_serialization,
                identity_nonce,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .transform_into_action_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    None,
                    &mut execution_context,
                    &vec![],
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
                Some(StateTransitionAction::BumpIdentityNonceAction(action)) if action.identity_id() == identity_id && action.identity_nonce() == identity_nonce
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }

        #[test]
        fn should_pass_when_data_contract_is_valid() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = IdentityNonce::default();

            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

            let data_contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            let data_contract_id = data_contract.id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
                data_contract: data_contract_for_serialization,
                identity_nonce,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let result = transition
                .transform_into_action_v0::<MockCoreRPCLike>(
                    &platform_ref,
                    &BlockInfo::default(),
                    ValidationMode::Validator,
                    None,
                    &mut execution_context,
                    &vec![],
                    platform_version,
                )
                .expect("failed to validate advanced structure");

            assert_matches!(result.errors.as_slice(), []);

            assert_matches!(
                result.data,
                Some(StateTransitionAction::DataContractCreateAction(action)) if action.data_contract_ref().id() == data_contract_id
            );

            // We have tons of operations here so not sure if we want to assert all of them
            assert!(!execution_context.operations_slice().is_empty());
        }
    }
}
