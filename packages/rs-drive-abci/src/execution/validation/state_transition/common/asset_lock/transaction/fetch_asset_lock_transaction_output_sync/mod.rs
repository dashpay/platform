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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::execution::ExecutionError;
    use crate::rpc::core::MockCoreRPCLike;

    #[test]
    fn should_return_unknown_version_error() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .common_validation_methods
            .asset_locks
            .fetch_asset_lock_transaction_output_sync = 99;

        let mock_rpc = MockCoreRPCLike::new();

        use dpp::dashcore::hashes::Hash;
        use dpp::dashcore::{OutPoint, Txid};
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        let asset_lock_proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 1,
            out_point: OutPoint {
                txid: Txid::from_byte_array([0u8; 32]),
                vout: 0,
            },
        });

        let result = fetch_asset_lock_transaction_output_sync(
            &mock_rpc,
            &asset_lock_proof,
            &platform_version,
        );

        match result {
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            })) => {
                assert_eq!(method, "fetch_asset_lock_transaction_output_sync");
                assert_eq!(known_versions, vec![0]);
                assert_eq!(received, 99);
            }
            other => panic!("expected UnknownVersionMismatch error, got {:?}", other),
        }
    }
}
