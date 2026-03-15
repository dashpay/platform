mod v0;
mod v1;
mod v2;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::block_execution_context::BlockExecutionContext;

use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;

use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for validator set rotations and performs rotations if necessary.
    ///
    /// This function is a version handler that directs to specific version implementations
    /// of the validator_set_update function.
    ///
    /// # Arguments
    ///
    /// * `platform_state` - A `PlatformState` reference.
    /// * `block_execution_context` - A mutable `BlockExecutionContext` reference.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<Option<ValidatorSetUpdate>, Error>` - If the rotation is successful, it returns `Ok(Some(ValidatorSetUpdate))`
    ///   If there is no update, it returns `Ok(None)`. If there is an error, it returns `Error`.
    ///
    pub fn validator_set_update(
        &self,
        proposer_pro_tx_hash: [u8; 32],
        platform_state: &PlatformState,
        block_execution_context: &mut BlockExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ValidatorSetUpdate>, Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .validator_set_update
        {
            0 => self.validator_set_update_v0(
                proposer_pro_tx_hash,
                platform_state,
                block_execution_context,
            ),
            1 => self.validator_set_update_v1(
                proposer_pro_tx_hash,
                platform_state,
                block_execution_context,
            ),
            2 => self.validator_set_update_v2(
                proposer_pro_tx_hash,
                platform_state,
                block_execution_context,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "validator_set_update".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::execution::types::block_state_info::BlockStateInfo;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use std::collections::BTreeMap;

    fn make_block_execution_context(block_platform_state: PlatformState) -> BlockExecutionContext {
        BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height: 1,
                round: 0,
                block_time_ms: 1_000_000,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash: None,
                app_hash: None,
            }),
            epoch_info: EpochInfo::V0(EpochInfoV0 {
                current_epoch_index: 0,
                previous_epoch_index: None,
                is_epoch_change: false,
            }),
            unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
            block_address_balance_changes: BTreeMap::new(),
            block_platform_state,
            proposer_results: None,
        })
    }

    #[test]
    fn test_dispatcher_unknown_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut modified_version = platform_version.clone();
        modified_version
            .drive_abci
            .methods
            .block_end
            .validator_set_update = 255;

        let platform_state = platform.state.load();
        let block_platform_state = platform_state.as_ref().clone();

        let mut block_execution_context = make_block_execution_context(block_platform_state);

        let result = platform.validator_set_update(
            [0u8; 32],
            &platform_state,
            &mut block_execution_context,
            &modified_version,
        );

        assert!(result.is_err());
        match result {
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            })) => {
                assert_eq!(method, "validator_set_update");
                assert_eq!(known_versions, vec![0, 1, 2]);
                assert_eq!(received, 255);
            }
            _ => panic!("expected UnknownVersionMismatch error"),
        }
    }
}
