mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Sets current protocol version and next epoch protocol version to block platform state
    ///
    /// This function should be called on very top of bock production before we add new proposed version for the next epoch
    ///
    /// It takes five parameters:
    /// * `block_info`: Information about the current block.
    /// * `epoch_info`: Information about the current epoch.
    /// * `last_committed_platform_state`: The last committed state of the platform.
    /// * `block_platform_state`: The current state of the platform.
    /// * `transaction`: The current transaction.
    /// * `platform_version`: The current version of the platform.
    ///
    /// # Errors
    ///
    /// This function will return an error if the previous block protocol version does not match the current block protocol version not on epoch change
    pub fn upgrade_protocol_version_on_epoch_change(
        &self,
        block_info: &BlockInfo,
        epoch_info: &EpochInfo,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &mut PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .upgrade_protocol_version_on_epoch_change
        {
            0 => self.upgrade_protocol_version_on_epoch_change_v0(
                block_info,
                epoch_info,
                last_committed_platform_state,
                block_platform_state,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "upgrade_protocol_version_on_epoch_change".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::epoch::Epoch;

    #[test]
    fn test_dispatcher_unknown_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let mut modified_version = platform_version.clone();
        modified_version
            .drive_abci
            .methods
            .protocol_upgrade
            .upgrade_protocol_version_on_epoch_change = 255;

        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 0,
            previous_epoch_index: None,
            is_epoch_change: false,
        });

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 2,
            core_height: 100,
            epoch: Epoch::default(),
        };

        let last_committed_state = platform.state.load();
        let mut block_platform_state = last_committed_state.as_ref().clone();

        let result = platform.upgrade_protocol_version_on_epoch_change(
            &block_info,
            &epoch_info,
            &last_committed_state,
            &mut block_platform_state,
            &transaction,
            &modified_version,
        );

        assert!(result.is_err());
        match result {
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            })) => {
                assert_eq!(method, "upgrade_protocol_version_on_epoch_change");
                assert_eq!(known_versions, vec![0]);
                assert_eq!(received, 255);
            }
            _ => panic!("expected UnknownVersionMismatch error"),
        }
    }
}
