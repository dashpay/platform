//! Stateful checks shared by the identity-less token pool transitions
//! (`TokenShieldedTransferWithShieldedFee`, `TokenUnshieldWithShieldedFee`,
//! `TokenPurchaseFromShieldedPool`): each carries a bundle in a token's pool and a fee bundle in
//! the credit pool. The processor has already verified both Orchard proofs and the fee floor;
//! these functions check the pools against state. Nothing is metered to an identity: the flat
//! fee carved from the credit bundle pays for everything, exactly as for the credit pool's own
//! pool-paid transitions.

use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::shielded_common::{
    read_pool_total_balance, validate_anchor_exists, validate_nullifiers,
};
use dpp::consensus::state::data_contract::data_contract_not_found_error::DataContractNotFoundError;
use dpp::consensus::state::shielded::invalid_anchor_error::InvalidAnchorError;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::shielded::nullifier_already_spent_error::NullifierAlreadySpentError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{TokenIsPausedError, TokenShieldedPoolNotEnabledError};
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::TokenContractPosition;
use dpp::fee::Credits;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::tokens::status::v0::TokenStatusV0Accessors;
use dpp::version::PlatformVersion;
use drive::drive::contract::DataContractFetchInfo;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use std::collections::HashSet;
use std::sync::Arc;

/// A resolved pooled token: its contract and its configuration.
pub(crate) type PooledToken = (Arc<DataContractFetchInfo>, TokenConfiguration);

/// The consensus rejection of an identity-less token pool transition.
pub(crate) type PoolPaidRejection = ConsensusValidationResult<StateTransitionAction>;

/// The contract and the token configuration of a token that must own a shielded pool and not
/// be paused. A missing contract or token is an invalid transition, not an execution error.
pub(crate) fn resolve_pooled_token(
    drive: &Drive,
    data_contract_id: Identifier,
    token_contract_position: TokenContractPosition,
    token_id: Identifier,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Result<PooledToken, PoolPaidRejection>, Error> {
    let Some(contract_fetch_info) = drive.get_contract_with_fetch_info(
        data_contract_id.to_buffer(),
        false,
        transaction,
        platform_version,
    )?
    else {
        return Ok(Err(ConsensusValidationResult::new_with_error(
            ConsensusError::StateError(StateError::DataContractNotFoundError(
                DataContractNotFoundError::new(data_contract_id),
            )),
        )));
    };
    let Some(configuration) = contract_fetch_info
        .contract
        .tokens()
        .get(&token_contract_position)
        .cloned()
    else {
        return Ok(Err(ConsensusValidationResult::new_with_error(
            ConsensusError::StateError(StateError::TokenShieldedPoolNotEnabledError(
                TokenShieldedPoolNotEnabledError::new(token_id),
            )),
        )));
    };
    if !configuration.has_shielded_pool() {
        return Ok(Err(ConsensusValidationResult::new_with_error(
            ConsensusError::StateError(StateError::TokenShieldedPoolNotEnabledError(
                TokenShieldedPoolNotEnabledError::new(token_id),
            )),
        )));
    }
    if let Some(status) =
        drive.fetch_token_status(token_id.to_buffer(), transaction, platform_version)?
    {
        if status.paused() {
            return Ok(Err(ConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::TokenIsPausedError(
                    TokenIsPausedError::new(token_id),
                )),
            )));
        }
    }
    Ok(Ok((contract_fetch_info, configuration)))
}

/// The token bundle's anchor must be recorded for the token pool and its nullifiers must be
/// unspent there (and unique within the bundle). Returns the rejection, if any.
pub(crate) fn validate_token_pool_spend(
    drive: &Drive,
    token_id: Identifier,
    anchor: &[u8; 32],
    nullifiers: &[[u8; 32]],
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusValidationResult<StateTransitionAction>>, Error> {
    let token_id_bytes = token_id.to_buffer();
    let mut drive_operations = vec![];
    if !drive.has_token_pool_anchor(
        &token_id_bytes,
        anchor,
        transaction,
        &mut drive_operations,
        platform_version,
    )? {
        return Ok(Some(ConsensusValidationResult::new_with_error(
            StateError::InvalidAnchorError(InvalidAnchorError::new(*anchor)).into(),
        )));
    }
    let mut seen = HashSet::new();
    for nullifier in nullifiers {
        if !seen.insert(nullifier)
            || drive.has_token_pool_nullifier(
                &token_id_bytes,
                nullifier,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
        {
            return Ok(Some(ConsensusValidationResult::new_with_error(
                StateError::NullifierAlreadySpentError(NullifierAlreadySpentError::new(*nullifier))
                    .into(),
            )));
        }
    }
    Ok(None)
}

/// The credit pool side of the fee bundle: anchor recorded, nullifiers unspent and the pool
/// holding `credits_leaving`. Returns the pool's current total on success.
pub(crate) fn validate_credit_pool_fee_spend(
    drive: &Drive,
    fee_anchor: &[u8; 32],
    fee_nullifiers: &[[u8; 32]],
    credits_leaving: Credits,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Result<Credits, ConsensusValidationResult<StateTransitionAction>>, Error> {
    let mut drive_operations = vec![];
    let current_total_balance =
        read_pool_total_balance(drive, transaction, &mut drive_operations, platform_version)?;
    if current_total_balance < credits_leaving {
        return Ok(Err(ConsensusValidationResult::new_with_error(
            StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(format!(
                "shielded pool has insufficient balance: pool has {} but the token pool transition requires {}",
                current_total_balance, credits_leaving
            )))
            .into(),
        )));
    }
    if let Some(consensus_error) = validate_anchor_exists(
        drive,
        fee_anchor,
        transaction,
        &mut drive_operations,
        platform_version,
    )? {
        return Ok(Err(consensus_error));
    }
    if let Some(consensus_error) = validate_nullifiers(
        drive,
        fee_nullifiers,
        transaction,
        &mut drive_operations,
        platform_version,
    )? {
        return Ok(Err(consensus_error));
    }
    Ok(Ok(current_total_balance))
}

/// The token pool must hold `amount` for it to leave.
pub(crate) fn validate_token_pool_holds(
    drive: &Drive,
    token_id: Identifier,
    amount: u64,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusValidationResult<StateTransitionAction>>, Error> {
    let mut drive_operations = vec![];
    let pool_balance = drive.read_token_shielded_pool_total_balance(
        &token_id.to_buffer(),
        transaction,
        &mut drive_operations,
        platform_version,
    )?;
    if pool_balance < amount {
        return Ok(Some(ConsensusValidationResult::new_with_error(
            StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(format!(
                "token shielded pool has insufficient balance: pool has {} but the transition requires {}",
                pool_balance, amount
            )))
            .into(),
        )));
    }
    Ok(None)
}
