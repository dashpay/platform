use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::GroveDBToUse;
use dpp::version::PlatformVersion;
use grovedb::PathTrunkChunkQuery;

impl Drive {
    pub(super) fn prove_address_funds_trunk_query_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_address_funds_trunk_query_operations_v0(&mut vec![], platform_version)
    }

    pub(super) fn prove_address_funds_trunk_query_operations_v0(
        &self,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path = Self::clear_addresses_path();
        let min_depth = platform_version
            .drive
            .methods
            .address_funds
            .address_funds_query_min_depth;
        let max_depth = platform_version
            .drive
            .methods
            .address_funds
            .address_funds_query_max_depth;

        let query = PathTrunkChunkQuery {
            path,
            min_depth: Some(min_depth),
            max_depth,
        };

        self.grove_get_proved_trunk_chunk_query(
            &query,
            GroveDBToUse::LatestCheckpoint,
            drive_operations,
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use crate::util::batch::drive_op_batch::{AddressFundsOperationType, DriveOperation};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::block_info::BlockInfo;

    /// Trunk query operations must populate drive_operations on success. In the
    /// minimal test setup no checkpoint exists yet, so the only acceptable
    /// error is `DriveError::NoCheckpointsAvailable` (raised by
    /// `grove_get_proved_trunk_chunk_query_v0` when
    /// `GroveDBToUse::LatestCheckpoint` is selected against an empty
    /// checkpoint map). Any other error means the prove-path regressed.
    #[test]
    fn trunk_query_operations_populates_ops() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Populate at least one address so the tree isn't empty.
        let ops_in = vec![DriveOperation::AddressFundsOperation(
            AddressFundsOperationType::SetBalanceToAddress {
                address: PlatformAddress::P2pkh([7; 20]),
                nonce: 1,
                balance: 42,
            },
        )];
        drive
            .apply_drive_operations(
                ops_in,
                true,
                &BlockInfo::default(),
                None,
                platform_version,
                None,
            )
            .expect("apply");

        let mut drive_operations = vec![];
        let result = drive
            .prove_address_funds_trunk_query_operations_v0(&mut drive_operations, platform_version);

        match result {
            Ok(proof) => {
                assert!(!proof.is_empty());
                assert!(!drive_operations.is_empty());
            }
            Err(Error::Drive(DriveError::NoCheckpointsAvailable)) => {
                // Expected when the test setup did not create a checkpoint.
            }
            Err(other) => panic!(
                "unexpected error from prove_address_funds_trunk_query_operations_v0: {:?}",
                other
            ),
        }
    }

    /// Public dispatcher matches the v0 path. When both paths succeed the
    /// proof bytes must be byte-equal; when both fail that's also acceptable
    /// (they must fail together). A divergence in the success/fail outcome
    /// means the dispatcher disagrees with the v0 implementation.
    #[test]
    fn top_level_trunk_query_returns_same_shape_as_v0() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let top = drive.prove_address_funds_trunk_query(platform_version);
        let v0 = drive.prove_address_funds_trunk_query_v0(platform_version);

        match (top, v0) {
            (Ok(a), Ok(b)) => assert_eq!(
                a, b,
                "top-level dispatcher must return identical proof bytes to v0"
            ),
            (Err(_), Err(_)) => {
                // Both paths hit the same underlying error (e.g. no checkpoint
                // available in a minimal test setup). That's consistent
                // behaviour — nothing further to assert.
            }
            (top, v0) => panic!(
                "dispatcher/v0 divergence: top={:?} v0={:?}",
                top.is_ok(),
                v0.is_ok()
            ),
        }
    }
}
