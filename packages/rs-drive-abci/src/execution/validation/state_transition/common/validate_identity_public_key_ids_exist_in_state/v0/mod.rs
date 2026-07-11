use crate::error::Error;

use dpp::consensus::state::identity::missing_identity_public_key_ids_error::MissingIdentityPublicKeyIdsError;

use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::platform_value::Identifier;
use dpp::prelude::ConsensusValidationResult;

use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::version::PlatformVersion;

/// This will validate that all keys are valid against the state
pub(super) fn validate_identity_public_key_ids_exist_in_state_v0(
    identity_id: Identifier,
    key_ids: &[KeyID],
    drive: &Drive,
    _execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Vec<IdentityPublicKey>>, Error> {
    let limit = key_ids.len() as u16;
    let identity_key_request = IdentityKeysRequest {
        identity_id: identity_id.to_buffer(),
        request_type: KeyRequestType::SpecificKeys(key_ids.to_vec()),
        limit: Some(limit),
        offset: None,
    };
    let to_remove_keys = drive.fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
        identity_key_request,
        transaction,
        platform_version,
    )?;
    if to_remove_keys.len() != key_ids.len() {
        let mut missing_keys = key_ids.to_vec();
        missing_keys.retain(|found_key| !to_remove_keys.contains_key(found_key));
        // keys should all exist
        Ok(ConsensusValidationResult::new_with_error(
            MissingIdentityPublicKeyIdsError::new(missing_keys).into(),
        ))
    } else {
        let values: Vec<_> = to_remove_keys.into_values().collect();
        Ok(ConsensusValidationResult::new_with_data(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::Identity;
    use dpp::version::DefaultForPlatformVersion;
    use rand::SeedableRng;

    #[test]
    fn should_fail_when_identity_has_no_keys_at_requested_ids() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity_id = Identifier::random();
        let missing_ids: Vec<KeyID> = vec![17, 42, 99];

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_exist_in_state_v0(
            identity_id,
            &missing_ids,
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(!result.is_valid(), "requested keys should be missing");
        assert_eq!(result.errors.len(), 1);
        match &result.errors[0] {
            ConsensusError::StateError(StateError::MissingIdentityPublicKeyIdsError(e)) => {
                let missing = e.ids();
                // All three should be reported as missing
                assert_eq!(missing.len(), 3);
                for id in &missing_ids {
                    assert!(missing.contains(id), "missing should contain {}", id);
                }
            }
            other => panic!("expected MissingIdentityPublicKeyIdsError, got {:?}", other),
        }
    }

    #[test]
    fn should_succeed_when_all_requested_keys_exist() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, keys_with_private): (
            Identity,
            Vec<(dpp::identity::IdentityPublicKey, [u8; 32])>,
        ) = Identity::random_identity_with_main_keys_with_private_key(
            2,
            &mut rand::rngs::StdRng::seed_from_u64(77),
            platform_version,
        )
        .expect("got identity");

        let identity_id = identity.id();
        let existing_ids: Vec<KeyID> = keys_with_private.iter().map(|(k, _)| k.id()).collect();

        platform
            .drive
            .add_new_identity(
                identity,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("should add identity");

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_exist_in_state_v0(
            identity_id,
            &existing_ids,
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(result.is_valid(), "existing keys should validate");
        let fetched = result.data.expect("should have data");
        assert_eq!(fetched.len(), existing_ids.len());
    }

    #[test]
    fn should_report_only_missing_ids_when_partial_overlap() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, keys_with_private): (
            Identity,
            Vec<(dpp::identity::IdentityPublicKey, [u8; 32])>,
        ) = Identity::random_identity_with_main_keys_with_private_key(
            2,
            &mut rand::rngs::StdRng::seed_from_u64(88),
            platform_version,
        )
        .expect("got identity");

        let identity_id = identity.id();
        let existing_id = keys_with_private[0].0.id();

        platform
            .drive
            .add_new_identity(
                identity,
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("should add identity");

        // Request one existing + one definitely-missing id.
        let missing_id: KeyID = 12345;
        let requested = vec![existing_id, missing_id];

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_exist_in_state_v0(
            identity_id,
            &requested,
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when a requested id is missing"
        );
        match &result.errors[0] {
            ConsensusError::StateError(StateError::MissingIdentityPublicKeyIdsError(e)) => {
                let missing = e.ids();
                // Only the missing id should be reported; existing id should be filtered out.
                assert_eq!(missing.len(), 1);
                assert!(missing.contains(&missing_id));
                assert!(!missing.contains(&existing_id));
            }
            other => panic!("expected MissingIdentityPublicKeyIdsError, got {:?}", other),
        }
    }
}
