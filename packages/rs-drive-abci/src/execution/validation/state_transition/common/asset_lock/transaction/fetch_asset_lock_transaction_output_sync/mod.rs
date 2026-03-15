use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::TxOut;
use dpp::prelude::{AssetLockProof, ConsensusValidationResult};
use dpp::version::PlatformVersion;

mod v0;

/// This fetches the asset lock transaction output from core
pub fn fetch_asset_lock_transaction_output_sync<C: CoreRPCLike>(
    core_rpc: &C,
    asset_lock_proof: &AssetLockProof,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<TxOut>, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .asset_locks
        .fetch_asset_lock_transaction_output_sync
    {
        0 => v0::fetch_asset_lock_transaction_output_sync_v0(
            core_rpc,
            asset_lock_proof,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "fetch_asset_lock_transaction_output_sync".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
