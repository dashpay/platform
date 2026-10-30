use crate::consensus::basic::identity::{
    IdentityAssetLockProofLockedTransactionMismatchError,
    IdentityAssetLockTransactionTooManyInputsError,
};
use crate::identity::state_transition::asset_lock_proof::validate_asset_lock_transaction_structure::validate_asset_lock_transaction_structure;
use crate::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Validates instant asset lock proof structure.
///
/// Same as v0, except that the proof holds the transaction it embeds to
/// `max_asset_lock_transaction_inputs` itself, ahead of the transaction structure check, which
/// from `validate_asset_lock_transaction_structure` 1 no longer counts inputs. The cap keeps a
/// state transition that embeds the whole transaction under the maximum state transition size.
#[inline(always)]
pub(in crate::identity::state_transition::asset_lock_proof::instant) fn validate_instant_asset_lock_proof_structure_v1(
    proof: &InstantAssetLockProof,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut result = SimpleConsensusValidationResult::default();

    let transaction_id = proof.transaction().txid();
    if proof.instant_lock().txid != transaction_id {
        result.add_error(IdentityAssetLockProofLockedTransactionMismatchError::new(
            proof.instant_lock().txid,
            transaction_id,
        ));

        return Ok(result);
    }

    let max_inputs = platform_version
        .dpp
        .state_transitions
        .identities
        .asset_locks
        .max_asset_lock_transaction_inputs;
    let input_count = proof.transaction().input.len();
    if input_count > max_inputs as usize {
        result.add_error(IdentityAssetLockTransactionTooManyInputsError::new(
            u16::try_from(input_count).unwrap_or(u16::MAX),
            max_inputs,
        ));

        return Ok(result);
    }

    let validate_transaction_result = validate_asset_lock_transaction_structure(
        proof.transaction(),
        proof.output_index(),
        platform_version,
    )?;

    if !validate_transaction_result.is_valid() {
        result.merge(validate_transaction_result);
    }

    Ok(result)
}
