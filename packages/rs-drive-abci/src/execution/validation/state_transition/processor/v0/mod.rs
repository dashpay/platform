use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::processor::address_balances_and_nonces::StateTransitionAddressBalancesAndNoncesValidation;
use crate::execution::validation::state_transition::processor::address_witnesses::{
    StateTransitionAddressWitnessValidationV0, StateTransitionHasAddressWitnessValidationV0,
};
use crate::execution::validation::state_transition::processor::addresses_minimum_balance::StateTransitionAddressesMinimumBalanceValidationV0;
use crate::execution::validation::state_transition::processor::advanced_structure_with_state::StateTransitionStructureKnownInStateValidationV0;
use crate::execution::validation::state_transition::processor::advanced_structure_without_state::StateTransitionAdvancedStructureValidationV0;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::processor::identity_balance::StateTransitionIdentityBalanceValidationV0;
use crate::execution::validation::state_transition::processor::identity_based_signature::StateTransitionIdentityBasedSignatureValidationV0;
use crate::execution::validation::state_transition::processor::identity_nonces::{
    StateTransitionHasIdentityNonceValidationV0, StateTransitionIdentityNonceValidationV0,
};
use crate::execution::validation::state_transition::processor::is_allowed::StateTransitionIsAllowedValidationV0;
use crate::execution::validation::state_transition::processor::prefunded_specialized_balance::StateTransitionPrefundedSpecializedBalanceValidationV0;
use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
use crate::execution::validation::state_transition::processor::traits::shielded_proof::{
    StateTransitionHasShieldedProofValidationV0, StateTransitionShieldedMinimumFeeValidationV0,
    StateTransitionShieldedProofValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use dpp::ProtocolError;
use drive::grovedb::TransactionArg;

pub(super) fn process_state_transition_v0<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    block_info: &BlockInfo,
    state_transition: StateTransition,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {
    let mut state_transition_execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)?;

    if state_transition.has_is_allowed_validation()? {
        let result = state_transition.validate_is_allowed(platform, platform_version)?;

        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Only identity create does not use identity in state validation, because it doesn't yet have the identity in state
    let mut maybe_identity = if state_transition.uses_identity_in_state() {
        // Validating signature for identity based state transitions (all those except identity create and identity top up)
        // As we already have removed identity create above, it just splits between identity top up (below - false) and
        // all other state transitions (above - true)
        let result = if state_transition.validates_signature_based_on_identity_info() {
            state_transition.validate_identity_signed_state_transition(
                platform.drive,
                transaction,
                &mut state_transition_execution_context,
                platform_version,
            )
        } else {
            // Currently only identity top up and identity top up from addresses uses this,
            // We will add the cost for a balance retrieval
            state_transition.retrieve_identity_info(
                platform.drive,
                transaction,
                &mut state_transition_execution_context,
                platform_version,
            )
        }?;
        if !result.is_valid() {
            // If the signature is not valid or if we could not retrieve identity info
            // we do not have the user pay for the state transition.
            // Since it is most likely not from them
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
        Some(result.into_data()?)
    } else {
        // Currently only identity create
        None
    };

    if state_transition.has_address_witness_validation(platform_version)? {
        let result = state_transition.validate_address_witnesses(
            &mut state_transition_execution_context,
            platform_version,
        )?;
        if !result.is_valid() {
            // If the witnesses are not valid
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Start by validating addresses if the transition has input addresses
    let remaining_address_balances = if state_transition
        .has_addresses_balances_and_nonces_validation()
    {
        // Here we validate that all input addresses have enough balance
        // We also validate that nonces are bumped
        let result = state_transition.validate_address_balances_and_nonces(
            platform.drive,
            &mut state_transition_execution_context,
            transaction,
            platform_version,
        )?;
        if !result.is_valid() {
            // The nonces are not valid or there is not enough balance. The transaction is each replaying an input or there
            // isn't enough balance, either way the transaction should be rejected.
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
        Some(result.into_data()?)
    } else {
        None
    };

    // Only identity top up and identity create do not have nonces validation
    if state_transition.has_identity_nonce_validation(platform_version)? {
        // Validating identity contract nonce, this must happen after validating the signature
        let result = state_transition.validate_identity_nonces(
            &platform.into(),
            platform.state.last_block_info(),
            transaction,
            &mut state_transition_execution_context,
            platform_version,
        )?;

        if !result.is_valid() {
            // If the nonce is not valid the state transition is not paid for, most likely because
            // this is just a replayed block
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Only Data contract state transitions and Masternode vote do not have basic structure validation
    if state_transition.has_basic_structure_validation(platform_version) {
        // We validate basic structure validation after verifying the identity,
        // this is structure validation that does not require state and is already checked on check_tx
        let consensus_result =
            state_transition.validate_basic_structure(platform.config.network, platform_version)?;

        if !consensus_result.is_valid() {
            // Basic structure validation is extremely cheap to process, because of this attacks are
            // not likely.
            // Often the basic structure validation is necessary for estimated costs
            // Proposers should remove such transactions from the block
            // Other validators should reject blocks with such transactions
            return Ok(
                ConsensusValidationResult::<ExecutionEvent>::new_with_errors(
                    consensus_result.errors,
                ),
            );
        }
    }

    // For identity credit withdrawal and identity credit transfers we have a balance pre-check that includes a
    // processing amount and the transfer amount.
    // For other state transitions we only check a min balance for an amount set per version.
    // This is not done for identity create and identity top up who don't have this check here
    if state_transition.has_identity_minimum_balance_pre_check_validation() {
        // Validating that we have sufficient balance for a transfer or withdrawal,
        // this must happen after validating the signature

        let identity = maybe_identity
            .as_mut()
            .ok_or(ProtocolError::CorruptedCodeExecution(
                "identity must be known to validate the balance".to_string(),
            ))?;
        let result = state_transition
            .validate_identity_minimum_balance_pre_check(identity, platform_version)?;

        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // For address-based state transitions that transfer or withdraw, we have a balance pre-check
    // that validates addresses have enough remaining balance after the input amounts to cover fees.
    if state_transition.has_addresses_minimum_balance_pre_check_validation() {
        // Validating that addresses have sufficient remaining balance for fees,
        // this must happen after validating the address balances and nonces

        let address_balances =
            remaining_address_balances
                .as_ref()
                .ok_or(ProtocolError::CorruptedCodeExecution(
                    "address balances must be known to validate the minimum balance".to_string(),
                ))?;
        let result = state_transition
            .validate_addresses_minimum_balance_pre_check(address_balances, platform_version)?;

        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // The prefunded_balances are currently not used as we would only use them for a masternode vote
    // however the masternode vote acts as a free operation, as it is paid for
    let _prefunded_balances = if state_transition.uses_prefunded_specialized_balance_for_payment() {
        Some(
            state_transition.validate_minimum_prefunded_specialized_balance_pre_check(
                platform.drive,
                transaction,
                &mut state_transition_execution_context,
                platform_version,
            )?,
        )
    } else {
        None
    };

    // Validate minimum fee for shielded spending transitions (stateless, uses public value_balance).
    // This is cheaper than proof verification so we check it first.
    // Only applies to ShieldedTransfer/Unshield/ShieldedWithdrawal — Shield pays from address
    // inputs and ShieldFromAssetLock pays from the asset lock.
    if state_transition.has_shielded_minimum_fee_validation() {
        let result = state_transition.validate_minimum_shielded_fee(platform_version)?;
        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Verify ZK proof for shielded transitions (stateless, like signature verification).
    if state_transition.has_shielded_proof_validation() {
        let result = state_transition.validate_shielded_proof(platform_version)?;
        if !result.is_valid() {
            return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
        }
    }

    // Only identity update and data contract create have advanced structure validation without state
    if state_transition.has_advanced_structure_validation_without_state() {
        // Currently only used for Identity Update, Data Contract Create and Identity Create From Addresses
        // Next we have advanced structure validation, this is structure validation that does not require
        // state but isn't checked on check_tx. If advanced structure fails identity nonces or identity
        // contract nonces will be bumped
        let identity = maybe_identity
            .as_ref()
            .ok_or(ProtocolError::CorruptedCodeExecution(
                "the identity should always be known on advanced structure validation".to_string(),
            ))?;
        let consensus_result = state_transition.validate_advanced_structure(
            identity,
            &mut state_transition_execution_context,
            platform_version,
        )?;

        if !consensus_result.is_valid() {
            return consensus_result.map_result(|action| {
                ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )
            });
        }
    }

    // Identity create, documents batch and masternode vote all have advanced structure validation with state
    let action = if state_transition.has_advanced_structure_validation_with_state() {
        // Currently used for identity create and documents batch
        let state_transition_action_result = state_transition.transform_into_action(
            platform,
            block_info,
            &remaining_address_balances,
            ValidationMode::Validator,
            &mut state_transition_execution_context,
            transaction,
        )?;
        if !state_transition_action_result.is_valid_with_data() {
            return state_transition_action_result.map_result(|action| {
                ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )
            });
        }
        let action = state_transition_action_result.into_data()?;

        // Validating structure
        let result = state_transition.validate_advanced_structure_from_state(
            block_info,
            platform.config.network,
            &action,
            maybe_identity.as_ref(),
            &mut state_transition_execution_context,
            platform_version,
        )?;
        if !result.is_valid() {
            return result.map_result(|action| {
                ExecutionEvent::create_from_state_transition_action(
                    action,
                    maybe_identity,
                    platform.state.last_committed_block_epoch_ref(),
                    state_transition_execution_context,
                    platform_version,
                )
            });
        }

        Some(action)
    } else {
        None
    };

    // Validating state
    // Only identity Top up does not validate state and instead just returns the action for topping up
    let result = if state_transition.has_state_validation() {
        state_transition.validate_state(
            action,
            platform,
            ValidationMode::Validator,
            block_info,
            &mut state_transition_execution_context,
            transaction,
        )?
    } else if let Some(action) = action {
        ConsensusValidationResult::new_with_data(action)
    } else {
        state_transition.transform_into_action(
            platform,
            block_info,
            &remaining_address_balances,
            ValidationMode::Validator,
            &mut state_transition_execution_context,
            transaction,
        )?
    };

    result.map_result(|action| {
        ExecutionEvent::create_from_state_transition_action(
            action,
            maybe_identity,
            platform.state.last_committed_block_epoch_ref(),
            state_transition_execution_context,
            platform_version,
        )
    })
}
