use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::GroveDBToUse;
use dpp::prelude::BlockHeight;
use dpp::version::PlatformVersion;
use grovedb::PathBranchChunkQuery;

impl Drive {
    pub(super) fn prove_address_funds_branch_query_v0(
        &self,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_address_funds_branch_query_operations_v0(
            key,
            depth,
            checkpoint_height,
            &mut vec![],
            platform_version,
        )
    }

    pub(super) fn prove_address_funds_branch_query_operations_v0(
        &self,
        key: Vec<u8>,
        depth: u8,
        checkpoint_height: BlockHeight,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
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

        if depth < min_depth || depth > max_depth {
            return Err(Error::Drive(DriveError::InvalidInput(format!(
                "depth {} is outside the allowed range [{}, {}]",
                depth, min_depth, max_depth
            ))));
        }

        let path = Self::clear_addresses_path();
        let query = PathBranchChunkQuery { path, key, depth };

        self.grove_get_proved_branch_chunk_query(
            &query,
            GroveDBToUse::Checkpoint(checkpoint_height),
            drive_operations,
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    /// Depth below the allowed minimum must produce an InvalidInput error
    /// quoting both the received depth and the allowed range.
    #[test]
    fn depth_below_min_returns_invalid_input() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let min_depth = platform_version
            .drive
            .methods
            .address_funds
            .address_funds_query_min_depth;
        // min_depth could be 0 in some versions; use a depth we know is below.
        if min_depth == 0 {
            // If min is 0, there is no "below" branch to exercise here.
            return;
        }
        let too_shallow = min_depth - 1;

        let err = drive
            .prove_address_funds_branch_query_v0(vec![], too_shallow, 0, platform_version)
            .expect_err("depth below min should be rejected");
        match err {
            Error::Drive(DriveError::InvalidInput(msg)) => {
                assert!(msg.contains(&format!("{}", too_shallow)));
            }
            other => panic!("expected InvalidInput, got {:?}", other),
        }
    }

    /// Depth above the allowed maximum must produce an InvalidInput error.
    #[test]
    fn depth_above_max_returns_invalid_input() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let max_depth = platform_version
            .drive
            .methods
            .address_funds
            .address_funds_query_max_depth;
        let too_deep = max_depth.saturating_add(1);
        if too_deep == max_depth {
            // Saturated at u8::MAX — can't build a "too deep" case.
            return;
        }

        let err = drive
            .prove_address_funds_branch_query_v0(vec![], too_deep, 0, platform_version)
            .expect_err("depth above max should be rejected");
        assert!(matches!(err, Error::Drive(DriveError::InvalidInput(_))));
    }

    /// Using a checkpoint height that does not exist in the drive must
    /// propagate a GroveDB CheckpointNotFound (or equivalent) error.
    #[test]
    fn unknown_checkpoint_height_errors() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

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
        let depth = min_depth.max(1).min(max_depth);

        // Pick a checkpoint height that definitely does not exist.
        let unknown_height = 999_999_999u64;
        let err = drive
            .prove_address_funds_branch_query_v0(vec![], depth, unknown_height, platform_version)
            .expect_err("unknown checkpoint should error");
        // Must propagate as some error; the exact variant depends on GroveDB.
        let _ = err;
    }

    /// prove_address_funds_branch_query_operations_v0 short-circuits on a
    /// depth-below-min validation failure: the error is returned before any
    /// GroveDB work happens, so `drive_operations` must stay empty.
    #[test]
    fn branch_query_operations_invalid_input_does_not_populate_ops() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let min_depth = platform_version
            .drive
            .methods
            .address_funds
            .address_funds_query_min_depth;
        if min_depth == 0 {
            return;
        }

        let mut ops = vec![];
        let result = drive.prove_address_funds_branch_query_operations_v0(
            vec![],
            min_depth - 1,
            0,
            &mut ops,
            platform_version,
        );
        assert!(result.is_err());
        // Validation error short-circuits before any GroveDB work, so ops stays empty.
        assert!(ops.is_empty());
    }
}
