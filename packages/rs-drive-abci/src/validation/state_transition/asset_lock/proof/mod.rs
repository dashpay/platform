use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::asset_lock::fetch_asset_lock_transaction_output_sync;
use dpp::dashcore::TxOut;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProved;
use dpp::identity::KeyType;
use dpp::prelude::AssetLockProof;
use dpp::state_transition::validation::validate_state_transition_identity_signature::convert_to_consensus_signature_error;
use dpp::state_transition::StateTransitionLike;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::NativeBlsModule;
use drive::grovedb::TransactionArg;

mod chain;
mod instant;

/// Validate the structure of the asset lock proof
pub fn validate_structure(
    asset_lock_proof: &AssetLockProof,
) -> Result<SimpleConsensusValidationResult, Error> {
    match asset_lock_proof {
        AssetLockProof::Instant(proof) => instant::validate_structure(proof),
        AssetLockProof::Chain(proof) => chain::validate_structure(proof),
    }
}

/// Validate the state of the asset lock proof
pub fn validate_state<C: CoreRPCLike>(
    asset_lock_proof: &AssetLockProof,
    platform_ref: &PlatformRef<C>,
    transaction: TransactionArg,
) -> Result<SimpleConsensusValidationResult, Error> {
    match asset_lock_proof {
        AssetLockProof::Instant(proof) => instant::validate_state(proof, platform_ref, transaction),
        AssetLockProof::Chain(proof) => chain::validate_state(proof, platform_ref, transaction),
    }
}

/// Validate the signature of the state transition with it's asset lock proof
pub fn validate_signature<C, ST>(
    state_transition: &ST,
    platform_ref: &PlatformRef<C>,
) -> Result<ConsensusValidationResult<TxOut>, Error>
where
    C: CoreRPCLike,
    ST: StateTransitionLike + AssetLockProved,
{
    let asset_lock_validation = fetch_asset_lock_transaction_output_sync(
        platform_ref.core_rpc,
        state_transition.get_asset_lock_proof(),
    )?;

    if !asset_lock_validation.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(
            asset_lock_validation.errors,
        ));
    }

    let asset_lock_output = asset_lock_validation.into_data()?;

    let public_key_hash = &asset_lock_output.script_pubkey.as_bytes()[2..];

    match state_transition.verify_by_public_key(
        public_key_hash,
        KeyType::ECDSA_HASH160,
        &NativeBlsModule::default(),
    ) {
        Ok(_) => Ok(ConsensusValidationResult::new_with_data(asset_lock_output)),
        Err(err) => {
            let consensus_error = convert_to_consensus_signature_error(err)?;

            Ok(ConsensusValidationResult::new_with_error(consensus_error))
        }
    }
}
