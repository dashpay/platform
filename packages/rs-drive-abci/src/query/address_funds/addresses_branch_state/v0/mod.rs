use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_addresses_branch_state_request::GetAddressesBranchStateRequestV0;
use dapi_grpc::platform::v0::get_addresses_branch_state_response::GetAddressesBranchStateResponseV0;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_addresses_branch_state_v0(
        &self,
        GetAddressesBranchStateRequestV0 {
            key,
            depth,
            checkpoint_height,
        }: GetAddressesBranchStateRequestV0,
        _platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressesBranchStateResponseV0>, Error> {
        // Drive refuses a depth outside this range with an internal error, which clients retry
        // on every other node; checking the wire value also keeps `depth as u8` from wrapping.
        let address_funds = &platform_version.drive.methods.address_funds;
        let (min_depth, max_depth) = (
            address_funds.address_funds_query_min_depth,
            address_funds.address_funds_query_max_depth,
        );
        if depth < min_depth as u32 || depth > max_depth as u32 {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "depth {} out of bounds of [{}, {}]",
                    depth, min_depth, max_depth
                )),
            ));
        }

        // checkpoint_height is now required and must match the height from trunk response metadata
        let merk_proof = self.drive.prove_address_funds_branch_query(
            key,
            depth as u8,
            checkpoint_height,
            platform_version,
        )?;

        let response = GetAddressesBranchStateResponseV0 { merk_proof };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_argument_status, setup_platform};
    use dpp::dashcore::Network;
    use drive::drive::{Checkpoint, CheckpointInfo};
    use drive::grovedb::GroveDb;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    #[test]
    fn test_branch_state_no_checkpoint_returns_error() {
        // Without checkpoints, the branch query should return an error
        // because prove_address_funds_branch_query uses GroveDBToUse::Checkpoint(height)
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetAddressesBranchStateRequestV0 {
            key: vec![0; 1],
            depth: version
                .drive
                .methods
                .address_funds
                .address_funds_query_min_depth as u32,
            checkpoint_height: 0,
        };

        let result = platform.query_addresses_branch_state_v0(request, &state, version);

        // The branch query uses `?` so errors propagate as Err, not as validation errors
        assert!(
            result.is_err(),
            "expected error when no checkpoint is available"
        );
    }

    /// A depth outside the allowed range is the request's fault, refused before Drive sees it;
    /// a depth that `as u8` would wrap into the range (263 into 7) is refused as well.
    #[test]
    fn should_refuse_a_depth_out_of_range_as_invalid_argument() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        for depth in [0, 255, 263] {
            let request = GetAddressesBranchStateRequestV0 {
                key: vec![0; 1],
                depth,
                checkpoint_height: 0,
            };

            let status = assert_invalid_argument_status(
                platform.query_addresses_branch_state_v0(request, &state, version),
            );
            assert!(status.message().contains("depth"), "{}", status.message());
        }
    }

    #[test]
    fn test_branch_state_with_checkpoint_returns_proof() {
        use dpp::address_funds::PlatformAddress;
        use dpp::block::block_info::BlockInfo;
        use drive::util::batch::drive_op_batch::{AddressFundsOperationType, DriveOperation};

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Insert address balances so the addresses tree is not empty
        let operations = vec![
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: PlatformAddress::P2pkh([10; 20]),
                nonce: 1,
                balance: 1_000_000,
            }),
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address: PlatformAddress::P2sh([20; 20]),
                nonce: 2,
                balance: 2_000_000,
            }),
        ];
        platform
            .drive
            .apply_drive_operations(operations, true, &BlockInfo::default(), None, version, None)
            .expect("expected to apply operations");

        let checkpoint_height = 1u64;

        // Create a checkpoint from the current grovedb state (which now has addresses)
        let checkpoint_path = platform
            .config
            .db_path
            .join("checkpoints")
            .join(checkpoint_height.to_string());
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
            checkpoint_height,
            CheckpointInfo::new(1000, Arc::new(checkpoint)),
        );
        platform.drive.checkpoints.store(Arc::new(checkpoints_map));

        // The min_depth from the platform version
        let min_depth = version
            .drive
            .methods
            .address_funds
            .address_funds_query_min_depth;

        // Use a valid address key from the tree
        let addr_key = PlatformAddress::P2pkh([10; 20]).to_bytes();

        let request = GetAddressesBranchStateRequestV0 {
            key: addr_key,
            depth: min_depth as u32,
            checkpoint_height,
        };

        let result = platform
            .query_addresses_branch_state_v0(request, &state, version)
            .expect("expected query to succeed with checkpoint");

        assert!(
            result.errors.is_empty(),
            "expected no errors with checkpoint"
        );
        let response = result.data.expect("expected response data");
        // The merk_proof should be non-empty
        assert!(
            !response.merk_proof.is_empty(),
            "expected non-empty merk proof"
        );
    }
}
