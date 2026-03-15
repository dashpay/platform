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
    use dpp::identity::Identity;
    use dpp::version::DefaultForPlatformVersion;

    #[test]
    fn should_return_keys_when_all_exist() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity =
            Identity::random_identity(3, Some(50), platform_version).expect("got an identity");
        let identity_id = identity.id();
        let key_ids: Vec<KeyID> = identity.public_keys().keys().copied().collect();

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
            &key_ids,
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(result.is_valid(), "should be valid when all keys exist");
        assert!(result.data.is_some(), "should have key data");
        assert_eq!(
            result.data.unwrap().len(),
            key_ids.len(),
            "should return all keys"
        );
    }

    #[test]
    fn should_return_error_when_keys_missing() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity =
            Identity::random_identity(3, Some(50), platform_version).expect("got an identity");
        let identity_id = identity.id();

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

        let non_existent_key_ids: Vec<KeyID> = vec![100, 101];

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_exist_in_state_v0(
            identity_id,
            &non_existent_key_ids,
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when keys are missing"
        );
        assert_eq!(result.errors.len(), 1);
        match &result.errors[0] {
            ConsensusError::StateError(StateError::MissingIdentityPublicKeyIdsError(e)) => {
                let missing = e.ids();
                assert_eq!(missing.len(), 2);
                assert!(missing.contains(&100));
                assert!(missing.contains(&101));
            }
            other => panic!("expected MissingIdentityPublicKeyIdsError, got {:?}", other),
        }
    }

    #[test]
    fn should_return_error_for_partially_missing_keys() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity =
            Identity::random_identity(3, Some(50), platform_version).expect("got an identity");
        let identity_id = identity.id();
        let existing_key_id = *identity.public_keys().keys().next().unwrap();

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

        // Mix of existing and non-existing key IDs
        let key_ids: Vec<KeyID> = vec![existing_key_id, 999];

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_exist_in_state_v0(
            identity_id,
            &key_ids,
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when some keys are missing"
        );
        assert_eq!(result.errors.len(), 1);
        match &result.errors[0] {
            ConsensusError::StateError(StateError::MissingIdentityPublicKeyIdsError(e)) => {
                let missing = e.ids();
                assert_eq!(missing.len(), 1);
                assert!(missing.contains(&999));
            }
            other => panic!("expected MissingIdentityPublicKeyIdsError, got {:?}", other),
        }
    }
}
