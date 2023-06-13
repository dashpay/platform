use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::Transaction;
use dpp::prelude::AssetLockProof;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use drive::grovedb::TransactionArg;

mod chain;
mod instant;

pub fn validate_asset_lock_proof_structure(
    asset_lock_proof: &AssetLockProof,
) -> Result<SimpleConsensusValidationResult, Error> {
    match asset_lock_proof {
        AssetLockProof::Instant(proof) => instant::validate_structure(proof),
        AssetLockProof::Chain(proof) => chain::validate_structure(proof),
    }
}

pub fn validate_asset_lock_proof_state<C: CoreRPCLike>(
    asset_lock_proof: &AssetLockProof,
    platform_ref: &PlatformRef<C>,
    transaction: TransactionArg,
) -> Result<SimpleConsensusValidationResult, Error> {
    match asset_lock_proof {
        AssetLockProof::Instant(proof) => instant::validate_state(proof, platform_ref, transaction),
        AssetLockProof::Chain(proof) => chain::validate_state(proof, platform_ref),
    }
}
