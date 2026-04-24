use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_nullifiers_branch_state_request::GetNullifiersBranchStateRequestV0;
use dapi_grpc::platform::v0::get_nullifiers_branch_state_response::GetNullifiersBranchStateResponseV0;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_nullifiers_branch_state_v0(
        &self,
        GetNullifiersBranchStateRequestV0 {
            pool_type,
            pool_identifier,
            key,
            depth,
            checkpoint_height,
        }: GetNullifiersBranchStateRequestV0,
        _platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetNullifiersBranchStateResponseV0>, Error> {
        let pool_id = if pool_identifier.is_empty() {
            None
        } else {
            Some(pool_identifier)
        };

        let merk_proof = self.drive.prove_nullifiers_branch_query(
            pool_type,
            pool_id,
            key,
            depth as u8,
            checkpoint_height,
            platform_version,
        )?;

        let response = GetNullifiersBranchStateResponseV0 { merk_proof };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;
    use drive::drive::{Checkpoint, CheckpointInfo};
    use drive::error::drive::DriveError as DriveInnerError;
    use drive::grovedb::GroveDb;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn install_checkpoint(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        height: u64,
    ) {
        let checkpoint_path = platform
            .config
            .db_path
            .join("checkpoints")
            .join(height.to_string());
        std::fs::create_dir_all(checkpoint_path.parent().unwrap())
            .expect("expected to create checkpoints dir");
        platform
            .drive
            .grove
            .create_checkpoint(&checkpoint_path)
            .expect("expected to create checkpoint");
        let checkpoint_db =
            GroveDb::open(&checkpoint_path).expect("expected to open checkpoint db");
        let checkpoint = Checkpoint::new(checkpoint_db, checkpoint_path);

        let mut checkpoints_map = BTreeMap::new();
        checkpoints_map.insert(
            height,
            CheckpointInfo::new(height * 10, Arc::new(checkpoint)),
        );
        platform.drive.checkpoints.store(Arc::new(checkpoints_map));
    }

    #[test]
    fn depth_below_min_returns_invalid_input_error() {
        // nullifiers_query_min_depth is 6, so depth=3 is out of range and must
        // bubble up as a Drive InvalidInput error.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersBranchStateRequestV0 {
            pool_type: 0,
            pool_identifier: vec![],
            key: vec![0u8; 32],
            depth: 3,
            checkpoint_height: 0,
        };

        let err = platform
            .query_nullifiers_branch_state_v0(request, &state, version)
            .expect_err("expected depth-range error");

        let is_invalid_input = matches!(
            err,
            Error::Drive(drive::error::Error::Drive(DriveInnerError::InvalidInput(
                ref msg
            ))) if msg.contains("depth")
        );
        assert!(is_invalid_input, "unexpected error: {err:?}");
    }

    #[test]
    fn depth_above_max_returns_invalid_input_error() {
        // nullifiers_query_max_depth is 10, so depth=11 is out of range.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersBranchStateRequestV0 {
            pool_type: 0,
            pool_identifier: vec![],
            key: vec![0u8; 32],
            depth: 11,
            checkpoint_height: 0,
        };

        let err = platform
            .query_nullifiers_branch_state_v0(request, &state, version)
            .expect_err("expected depth-range error");

        assert!(matches!(
            err,
            Error::Drive(drive::error::Error::Drive(DriveInnerError::InvalidInput(_)))
        ));
    }

    #[test]
    fn unsupported_token_pool_type_returns_not_supported_error() {
        // Pool types 1 and 2 are reserved for token shielded pools and must error
        // out with NotSupported, exposing the nullifiers_path_for_pool match arm.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersBranchStateRequestV0 {
            pool_type: 1,
            pool_identifier: vec![],
            key: vec![0u8; 32],
            depth: 6,
            checkpoint_height: 0,
        };

        let err = platform
            .query_nullifiers_branch_state_v0(request, &state, version)
            .expect_err("expected unsupported pool type error");

        assert!(matches!(
            err,
            Error::Drive(drive::error::Error::Drive(DriveInnerError::NotSupported(_)))
        ));
    }

    #[test]
    fn unknown_pool_type_returns_invalid_input_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersBranchStateRequestV0 {
            pool_type: 999,
            pool_identifier: vec![],
            key: vec![0u8; 32],
            depth: 6,
            checkpoint_height: 0,
        };

        let err = platform
            .query_nullifiers_branch_state_v0(request, &state, version)
            .expect_err("expected unknown pool type error");

        assert!(matches!(
            err,
            Error::Drive(drive::error::Error::Drive(DriveInnerError::InvalidInput(
                ref msg
            ))) if msg.contains("Unknown pool type")
        ));
    }

    #[test]
    fn missing_checkpoint_returns_checkpoint_not_found_error() {
        // No checkpoints configured, so the requested checkpoint_height cannot be
        // resolved and the Drive layer must bubble up CheckpointNotFound.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersBranchStateRequestV0 {
            pool_type: 0,
            pool_identifier: vec![],
            key: vec![0u8; 32],
            depth: 6,
            checkpoint_height: 42,
        };

        let err = platform
            .query_nullifiers_branch_state_v0(request, &state, version)
            .expect_err("expected checkpoint missing error");

        assert!(matches!(
            err,
            Error::Drive(drive::error::Error::Drive(DriveInnerError::CheckpointNotFound(h))) if h == 42
        ));
    }

    #[test]
    fn pool_identifier_empty_is_normalized_to_none() {
        // When pool_identifier is empty and no checkpoint exists, the handler
        // must still translate it to `None` before reaching the drive layer,
        // which then errors with CheckpointNotFound rather than a path error.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersBranchStateRequestV0 {
            pool_type: 0,
            // Explicitly empty – exercises the `is_empty() -> None` branch.
            pool_identifier: vec![],
            key: vec![0u8; 32],
            depth: 6,
            checkpoint_height: 7,
        };

        let err = platform
            .query_nullifiers_branch_state_v0(request, &state, version)
            .expect_err("expected checkpoint missing error");

        assert!(matches!(
            err,
            Error::Drive(drive::error::Error::Drive(
                DriveInnerError::CheckpointNotFound(_)
            ))
        ));
    }

    #[test]
    fn branch_query_on_empty_tree_surfaces_merk_error() {
        // When a checkpoint exists but the nullifiers tree is still empty, the
        // underlying grovedb merk returns "branch_query cannot be performed on
        // an empty tree". This documents that the drive-abci layer surfaces it
        // verbatim as a Drive error rather than silently producing an empty proof.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        install_checkpoint(&platform, 1);

        let request = GetNullifiersBranchStateRequestV0 {
            pool_type: 0,
            pool_identifier: vec![],
            key: vec![0u8; 32],
            depth: 6,
            checkpoint_height: 1,
        };

        let err = platform
            .query_nullifiers_branch_state_v0(request, &state, version)
            .expect_err("expected merk error on empty tree");

        // The precise inner variant is a grovedb/merk implementation detail; we
        // just assert it bubbles up as Error::Drive rather than being swallowed.
        assert!(matches!(err, Error::Drive(_)), "unexpected error: {err:?}");
    }
}
