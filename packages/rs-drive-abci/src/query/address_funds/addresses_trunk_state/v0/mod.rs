use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_addresses_trunk_state_request::GetAddressesTrunkStateRequestV0;
use dapi_grpc::platform::v0::get_addresses_trunk_state_response::GetAddressesTrunkStateResponseV0;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_addresses_trunk_state_v0(
        &self,
        GetAddressesTrunkStateRequestV0 {}: GetAddressesTrunkStateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAddressesTrunkStateResponseV0>, Error> {
        let proof = check_validation_result_with_data!(self
            .drive
            .prove_address_funds_trunk_query(platform_version));

        let (grovedb_used, proof) =
            self.response_proof_v0(platform_state, proof, GroveDBToUse::LatestCheckpoint)?;

        let response = GetAddressesTrunkStateResponseV0 {
            proof: Some(proof),
            metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;
    use drive::drive::{Checkpoint, CheckpointInfo};
    use drive::grovedb::GroveDb;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    #[test]
    fn test_trunk_state_no_checkpoint_returns_validation_errors() {
        // Without checkpoints, prove_address_funds_trunk_query returns an error.
        // The check_validation_result_with_data! macro converts this to a
        // validation result with errors (not a function-level Err).
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetAddressesTrunkStateRequestV0 {};

        let result = platform
            .query_addresses_trunk_state_v0(request, &state, version)
            .expect("should return Ok with validation errors, not Err");

        assert!(
            !result.errors.is_empty(),
            "expected validation errors when no checkpoint is available"
        );
    }

    #[test]
    fn test_trunk_state_no_checkpoint_with_initial_state() {
        // Even with genesis state set up, there are still no checkpoints,
        // so the trunk query should still return validation errors.
        let (platform, state, version) = setup_platform(Some((1, 1500)), Network::Testnet, None);

        let request = GetAddressesTrunkStateRequestV0 {};

        let result = platform
            .query_addresses_trunk_state_v0(request, &state, version)
            .expect("should return Ok with validation errors, not Err");

        assert!(
            !result.errors.is_empty(),
            "expected validation errors when no checkpoint is available"
        );
    }

    #[test]
    fn test_trunk_state_with_checkpoint_returns_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Create a checkpoint from the current grovedb state
        let checkpoint_path = platform.config.db_path.join("checkpoints").join("1");
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
        checkpoints_map.insert(1u64, CheckpointInfo::new(1000, Arc::new(checkpoint)));
        platform.drive.checkpoints.store(Arc::new(checkpoints_map));

        // Also set up checkpoint_platform_states
        let mut checkpoint_states = BTreeMap::new();
        checkpoint_states.insert(1u64, state.clone());
        platform
            .checkpoint_platform_states
            .store(Arc::new(checkpoint_states));

        let request = GetAddressesTrunkStateRequestV0 {};

        let result = platform
            .query_addresses_trunk_state_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            result.errors.is_empty(),
            "expected no errors with checkpoint available"
        );
        let response = result.data.expect("expected response data");
        assert!(response.proof.is_some(), "expected proof in response");
        assert!(response.metadata.is_some(), "expected metadata in response");
    }
}
