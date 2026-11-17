use crate::consensus::basic::identity::IdentityAssetLockProofLockedTransactionMismatchError;
use crate::identity::state_transition::asset_lock_proof::validate_asset_lock_transaction_inputs::validate_asset_lock_transaction_inputs;
use crate::identity::state_transition::asset_lock_proof::validate_asset_lock_transaction_structure::validate_asset_lock_transaction_structure;
use crate::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Validates an instant asset lock proof's structure.
///
/// Unlike v0, it checks the input cap of the embedded transaction here instead of relying on
/// `validate_asset_lock_transaction_structure`, whose version 1 no longer caps inputs: the cap
/// keeps the state transition that carries the transaction small, and a chain proof's
/// transaction, which that check also sees, is not carried. The checks run in v0's order, each
/// returning its single error, so a proof gets the verdict and the error it got from v0.
pub fn validate_instant_asset_lock_proof_structure_v1(
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

    let validate_inputs_result =
        validate_asset_lock_transaction_inputs(proof.transaction(), platform_version);

    if !validate_inputs_result.is_valid() {
        return Ok(validate_inputs_result);
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
