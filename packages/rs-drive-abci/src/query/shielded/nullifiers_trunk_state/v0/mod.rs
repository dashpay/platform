use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_nullifiers_trunk_state_request::GetNullifiersTrunkStateRequestV0;
use dapi_grpc::platform::v0::get_nullifiers_trunk_state_response::GetNullifiersTrunkStateResponseV0;
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_nullifiers_trunk_state_v0(
        &self,
        GetNullifiersTrunkStateRequestV0 {
            pool_type,
            pool_identifier,
        }: GetNullifiersTrunkStateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetNullifiersTrunkStateResponseV0>, Error> {
        let pool_id = if pool_identifier.is_empty() {
            None
        } else {
            Some(pool_identifier)
        };

        let proof = check_validation_result_with_data!(self.drive.prove_nullifiers_trunk_query(
            pool_type,
            pool_id,
            platform_version
        ));

        let (grovedb_used, proof) =
            self.response_proof_v0(platform_state, proof, GroveDBToUse::LatestCheckpoint)?;

        let response = GetNullifiersTrunkStateResponseV0 {
            proof: Some(proof),
            metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::query::QueryError;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;
    use drive::drive::{Checkpoint, CheckpointInfo};
    use drive::error::drive::DriveError as DriveInnerError;
    use drive::grovedb::GroveDb;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    #[test]
    fn no_checkpoint_returns_validation_errors() {
        // Without any registered checkpoint, prove_nullifiers_trunk_query returns
        // a NoCheckpointsAvailable drive error, which the check_validation_result_with_data
        // macro funnels into the validation errors list.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersTrunkStateRequestV0 {
            pool_type: 0,
            pool_identifier: vec![],
        };

        let result = platform
            .query_nullifiers_trunk_state_v0(request, &state, version)
            .expect("should return Ok with validation errors, not Err");

        assert!(
            !result.errors.is_empty(),
            "expected validation errors when no checkpoint is available"
        );
        assert!(matches!(
            result.errors[0],
            QueryError::Drive(drive::error::Error::Drive(
                DriveInnerError::NoCheckpointsAvailable
            ))
        ));
    }

    #[test]
    fn unsupported_token_pool_type_returns_validation_errors() {
        // pool_type=2 hits the NotSupported arm of nullifiers_path_for_pool.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersTrunkStateRequestV0 {
            pool_type: 2,
            pool_identifier: vec![7u8; 32],
        };

        let result = platform
            .query_nullifiers_trunk_state_v0(request, &state, version)
            .expect("should return Ok with validation errors, not Err");

        assert!(
            !result.errors.is_empty(),
            "expected validation errors for unsupported pool"
        );
        assert!(matches!(
            result.errors[0],
            QueryError::Drive(drive::error::Error::Drive(DriveInnerError::NotSupported(_)))
        ));
    }

    #[test]
    fn unknown_pool_type_returns_validation_errors() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetNullifiersTrunkStateRequestV0 {
            pool_type: 1234,
            pool_identifier: vec![],
        };

        let result = platform
            .query_nullifiers_trunk_state_v0(request, &state, version)
            .expect("should return Ok with validation errors, not Err");

        assert!(!result.errors.is_empty());
        assert!(matches!(
            result.errors[0],
            QueryError::Drive(drive::error::Error::Drive(DriveInnerError::InvalidInput(_)))
        ));
    }

    #[test]
    fn with_checkpoint_returns_proof_and_metadata() {
        // Create a checkpoint from the current (empty) state and wire it up so
        // both the drive-layer prove call and response_proof_v0 succeed.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

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

        let mut checkpoint_states = BTreeMap::new();
        checkpoint_states.insert(1u64, state.clone());
        platform
            .checkpoint_platform_states
            .store(Arc::new(checkpoint_states));

        let request = GetNullifiersTrunkStateRequestV0 {
            pool_type: 0,
            pool_identifier: vec![],
        };

        let result = platform
            .query_nullifiers_trunk_state_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let response = result.data.expect("expected response data");
        let proof = response.proof.expect("expected proof present");
        assert!(
            !proof.grovedb_proof.is_empty(),
            "expected non-empty trunk proof bytes"
        );
        assert!(response.metadata.is_some());
    }
}
