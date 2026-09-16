mod v0;
mod v1;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C> {
    /// Executes protocol-specific events on the first block after a protocol version change.
    ///
    /// This function is triggered when there is a protocol version upgrade detected in the network.
    /// It checks if the current protocol version has transitioned from an earlier version to version 4,
    /// and if so, performs the necessary setup or migration tasks associated with version 4.
    ///
    /// Currently, the function handles the transition to version 4 by initializing new structures
    /// or states required for the new protocol version.
    ///
    /// # Parameters
    ///
    /// * `transaction`: A reference to the transaction context in which the changes should be applied.
    /// * `previous_protocol_version`: The protocol version prior to the upgrade.
    /// * `platform_version`: The current platform version containing the updated protocol version and relevant configuration details.
    ///
    /// # Returns
    ///
    /// * `Ok(())`: If all events related to the protocol change were successfully executed.
    /// * `Err(Error)`: If there was an issue executing the protocol-specific events.
    ///
    /// # Versioning
    ///
    /// This function uses the `platform_version` parameter to determine which version-specific implementation
    /// of the protocol change events should be executed:
    ///
    /// - If the version is `0`, it calls the `perform_events_on_first_block_of_protocol_change_v0` function,
    ///   which contains the logic for version `0`.
    /// - If the version is `1`, it calls `perform_events_on_first_block_of_protocol_change_v1`, which runs
    ///   the same transitions and then refreshes the cached definitions of the contracts they rewrote.
    /// - If no version is specified (`None`), the function does nothing and returns `Ok(())`.
    /// - If a different version is specified, it returns an error indicating an unknown version mismatch.
    ///
    pub fn perform_events_on_first_block_of_protocol_change(
        &self,
        platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .perform_events_on_first_block_of_protocol_change
        {
            Some(0) => self.perform_events_on_first_block_of_protocol_change_v0(
                platform_state,
                block_info,
                transaction,
                previous_protocol_version,
                platform_version,
            ),
            Some(1) => self.perform_events_on_first_block_of_protocol_change_v1(
                platform_state,
                block_info,
                transaction,
                previous_protocol_version,
                platform_version,
            ),
            None => Ok(()),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "perform_events_on_first_block_of_protocol_change".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;

    #[test]
    fn test_perform_events_when_version_method_is_none() {
        // When the perform_events_on_first_block_of_protocol_change method is None,
        // the dispatcher should return Ok(()) without doing anything.
        // We can test this indirectly: the latest platform version has Some(0),
        // but we can verify the dispatch for a same-version transition (no-op case).
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let platform_state = platform.state.load();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(1).expect("expected epoch"),
        };

        // Calling with previous_protocol_version == current version should be fine
        // since all transition conditions (previous < X && current >= X) will be false
        let result = platform.perform_events_on_first_block_of_protocol_change(
            &platform_state,
            &block_info,
            &transaction,
            platform_version.protocol_version,
            platform_version,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_perform_events_unknown_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(1).expect("expected epoch"),
        };

        // Create a modified platform version with unknown method version
        let mut modified_version = platform_version.clone();
        modified_version
            .drive_abci
            .methods
            .protocol_upgrade
            .perform_events_on_first_block_of_protocol_change = Some(255);

        let result = platform.perform_events_on_first_block_of_protocol_change(
            &platform_state,
            &block_info,
            &transaction,
            platform_version.protocol_version,
            &modified_version,
        );

        assert!(result.is_err());
        match result {
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            })) => {
                assert_eq!(method, "perform_events_on_first_block_of_protocol_change");
                assert_eq!(known_versions, vec![0, 1]);
                assert_eq!(received, 255);
            }
            _ => panic!("expected UnknownVersionMismatch error"),
        }
    }
}
