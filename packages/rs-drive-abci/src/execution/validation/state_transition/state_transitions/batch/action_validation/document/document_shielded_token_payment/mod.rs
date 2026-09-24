//! State validation of a document action's token cost paid out of the token's shielded pool
//! (`TokenPaymentInfo::V1`).
//!
//! The document action itself is validated first by its own validator; this runs afterwards,
//! from the batch state validation, and mirrors the pool side of `TokenUnshield` (the notes
//! leave the pool; the cost lands in the contract owner's balance or leaves the supply): the
//! pool must exist, the token must not be paused, the pool must hold the notes the token's
//! configuration requires before tokens leave it, the anchor must be recorded, the nullifiers
//! unspent, the pool must hold the amount, and the spend bundle must verify with the token id,
//! the batch owner, the document's contract and id and the amount bound into its sighash.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::{
    charge_drive_operations, validate_minimum_token_pool_notes, validate_token_not_paused,
    validate_token_pool_anchor_exists, validate_token_pool_nullifiers, verify_token_pool_bundle,
};
use crate::execution::validation::state_transition::state_transitions::shielded_common::FLAGS_SPENDS_AND_OUTPUTS;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::TokenShieldedPoolNotEnabledError;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::prelude::Identifier;
use dpp::shielded::document_token_payment_extra_sighash_data;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::error::drive::DriveError;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{
    DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0,
    DocumentShieldedTokenPayment,
};

/// Validates the pool side of a document action's shielded token payment and verifies its
/// bundle (in block processing; CheckTx verifies it under the nonce-aware admission instead).
#[allow(clippy::too_many_arguments)]
pub(in crate::execution::validation::state_transition::state_transitions::batch) fn validate_document_shielded_token_payment(
    base: &DocumentBaseTransitionAction,
    shielded_payment: &DocumentShieldedTokenPayment,
    platform: &PlatformStateRef,
    owner_id: Identifier,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    validation_mode: ValidationMode,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    // The transformer only keeps a shielded payment when the document type has a token cost
    // and the payment's amount equals it.
    let Some((token_id, _effect, cost)) = base.token_cost() else {
        return Ok(SimpleConsensusValidationResult::new());
    };
    let token_id_bytes = token_id.to_buffer();
    let DocumentShieldedTokenPayment {
        payment,
        token_contract_id,
        token_contract_position,
    } = shielded_payment;

    let mut drive_operations = vec![];

    // The pool trees exist exactly for the tokens whose configuration opted in.
    let pool_exists = platform.drive.has_token_shielded_pool(
        token_id_bytes,
        transaction,
        &mut drive_operations,
        platform_version,
    )?;
    if !pool_exists {
        charge_drive_operations(
            platform.drive,
            block_info,
            execution_context,
            drive_operations,
            platform_version,
        )?;
        return Ok(SimpleConsensusValidationResult::new_with_error(
            TokenShieldedPoolNotEnabledError::new(token_id).into(),
        ));
    }

    let validation_result = validate_token_not_paused(
        platform,
        token_id,
        block_info,
        execution_context,
        transaction,
        platform_version,
    )?;
    if !validation_result.is_valid() {
        charge_drive_operations(
            platform.drive,
            block_info,
            execution_context,
            drive_operations,
            platform_version,
        )?;
        return Ok(validation_result);
    }

    // The token's configuration holds its pool's outgoing notes threshold. Its contract is the
    // document's own, already loaded, or the one the document type's token cost names, fetched
    // and billed here. The pool exists, so its token's contract does too.
    let token_contract = if *token_contract_id == base.data_contract_id() {
        base.data_contract_fetch_info()
    } else {
        let (fee, token_contract) = platform.drive.get_contract_with_fetch_info_and_fee(
            token_contract_id.to_buffer(),
            Some(&block_info.epoch),
            false,
            transaction,
            platform_version,
        )?;
        let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "fee must exist when fetching a token's contract with an epoch",
        )))?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
        token_contract.ok_or_else(|| {
            Error::Drive(drive::error::Error::Drive(DriveError::CorruptedDriveState(
                format!(
                    "contract {} of token {} is missing although the token has a shielded pool",
                    token_contract_id, token_id
                ),
            )))
        })?
    };
    let token_configuration = token_contract
        .contract
        .expected_token_configuration(*token_contract_position)?;

    let validation_result = validate_minimum_token_pool_notes(
        platform.drive,
        &token_id_bytes,
        token_configuration,
        transaction,
        &mut drive_operations,
        platform_version,
    )?;
    if !validation_result.is_valid() {
        charge_drive_operations(
            platform.drive,
            block_info,
            execution_context,
            drive_operations,
            platform_version,
        )?;
        return Ok(validation_result);
    }

    let validation_result = validate_token_pool_anchor_exists(
        platform.drive,
        &token_id_bytes,
        &payment.anchor,
        transaction,
        &mut drive_operations,
        platform_version,
    )?;
    if !validation_result.is_valid() {
        charge_drive_operations(
            platform.drive,
            block_info,
            execution_context,
            drive_operations,
            platform_version,
        )?;
        return Ok(validation_result);
    }

    let nullifiers: Vec<[u8; 32]> = payment
        .actions
        .iter()
        .map(|action| action.nullifier)
        .collect();
    let validation_result = validate_token_pool_nullifiers(
        platform.drive,
        &token_id_bytes,
        &nullifiers,
        transaction,
        &mut drive_operations,
        platform_version,
    )?;
    if !validation_result.is_valid() {
        charge_drive_operations(
            platform.drive,
            block_info,
            execution_context,
            drive_operations,
            platform_version,
        )?;
        return Ok(validation_result);
    }

    let pool_balance = platform.drive.read_token_shielded_pool_total_balance(
        &token_id_bytes,
        transaction,
        &mut drive_operations,
        platform_version,
    )?;

    charge_drive_operations(
        platform.drive,
        block_info,
        execution_context,
        drive_operations,
        platform_version,
    )?;

    if pool_balance < cost {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            StateError::InvalidShieldedProofError(InvalidShieldedProofError::new(format!(
                "token shielded pool has insufficient balance: pool has {} but the document payment requires {}",
                pool_balance, cost
            )))
            .into(),
        ));
    }

    let extra_sighash_data = document_token_payment_extra_sighash_data(
        &token_id_bytes,
        &owner_id.to_buffer(),
        &base.data_contract_id().to_buffer(),
        &base.id().to_buffer(),
        cost,
        platform_version,
    )?;

    verify_token_pool_bundle(
        validation_mode,
        &payment.actions,
        FLAGS_SPENDS_AND_OUTPUTS,
        cost as i64,
        &payment.anchor,
        &payment.proof,
        &payment.binding_signature,
        &extra_sighash_data,
    )
}
