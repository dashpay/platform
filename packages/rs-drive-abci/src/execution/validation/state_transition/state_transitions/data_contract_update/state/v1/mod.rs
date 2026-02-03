use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use std::collections::BTreeSet;

use dpp::consensus::state::data_contract::data_contract_not_found_error::DataContractNotFoundError;
use dpp::consensus::state::group::IdentityMemberOfGroupNotFoundError;
use dpp::consensus::state::identity::identity_for_token_configuration_not_found_error::{
    IdentityInTokenConfigurationNotFoundError, TokenConfigurationIdentityContext,
};
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::InvalidTokenPositionStateError;
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
use dpp::data_contract::validate_update::DataContractUpdateValidationMethodsV0;

use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::ValidationMode;
use dpp::prelude::ConsensusValidationResult;
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

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionStateValidationV1 {
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractUpdateStateTransitionStateValidationV1 for DataContractUpdateTransition {
    fn validate_state_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let action = self.transform_into_action_v1(
            platform,
            block_info,
            validation_mode,
            execution_context,
            tx,
            platform_version,
        )?;

        if !action.is_valid() {
            return Ok(action);
        }

        // For V1 transitions, the old contract was already fetched in transform_into_action_v1
        // and the new contract already has created_at fields copied from old contract.
        // Get references to both for validation.
        let state_transition_action = action.data.as_ref().ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "we should always have an action at this point in data contract update",
            ),
        ))?;

        let (new_data_contract, old_data_contract_ref) = match state_transition_action {
            StateTransitionAction::DataContractUpdateAction(update_action) => {
                let old = update_action
                    .old_data_contract_ref()
                    .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "V1 update action should have old_data_contract",
                    )))?;
                (update_action.data_contract_ref(), old)
            }
            _ => {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "we should always have an update action at this point in data contract update",
                )));
            }
        };

        // Validate the update against the old data contract
        let validation_result = old_data_contract_ref.validate_update(
            new_data_contract,
            block_info,
            platform_version,
        )?;

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

        let mut validated_identities = BTreeSet::new();

        // Get groups from the transition - V0 has embedded contract, V1 has new_groups field
        let groups_to_validate = match self {
            DataContractUpdateTransition::V0(v0) => v0.data_contract.groups(),
            DataContractUpdateTransition::V1(v1) => &v1.new_groups,
        };

        let contract_id = new_data_contract.id();

        for (position, group) in groups_to_validate {
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
                                    contract_id,
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
            if !old_data_contract_ref
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
                                            old_data_contract_ref.id(),
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
                                            old_data_contract_ref.id(),
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
                                                old_data_contract_ref.id(),
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
                                        old_data_contract_ref.id(),
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

    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let mut validation_operations = vec![];

        let result = DataContractUpdateTransitionAction::try_from_borrowed_transition(
            self,
            platform.drive,
            tx,
            block_info,
            validation_mode.should_fully_validate_contract_on_transform_into_action(),
            &mut validation_operations,
            platform_version,
        );

        execution_context.add_dpp_operations(validation_operations);

        // Return validation result if any consensus errors happened
        // during data contract validation
        match result {
            Err(drive::error::Error::Protocol(protocol_error)) => {
                if let ProtocolError::ConsensusError(consensus_error) = *protocol_error {
                    let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                        BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self),
                    );

                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        bump_action,
                        vec![*consensus_error],
                    ))
                } else {
                    Err(Error::Protocol(*protocol_error))
                }
            }
            Err(drive_error) => Err(drive_error.into()),
            Ok(validation_result) => {
                if !validation_result.is_valid() {
                    let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                        BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(self),
                    );
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        bump_action,
                        validation_result.errors,
                    ))
                } else {
                    Ok(validation_result.map(|update_action| update_action.into()))
                }
            }
        }
    }
}
