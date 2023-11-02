use platform_version::version::PlatformVersion;
use crate::consensus::basic::identity::IdentityAssetLockProofLockedTransactionMismatchError;
use crate::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::validate_asset_lock_transaction_structure::validate_asset_lock_transaction_structure;
use crate::ProtocolError;
use crate::validation::SimpleConsensusValidationResult;

pub(in crate::identity::state_transition::asset_lock_proof::instant) fn validate_instant_asset_lock_proof_structure_v0(
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
