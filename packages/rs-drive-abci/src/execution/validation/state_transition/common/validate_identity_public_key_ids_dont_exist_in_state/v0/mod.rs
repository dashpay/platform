use crate::error::Error;
use dpp::consensus::basic::identity::DuplicatedIdentityPublicKeyIdBasicError;
use dpp::consensus::basic::BasicError;

use dpp::identity::KeyID;
use dpp::platform_value::Identifier;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDVec, KeyRequestType};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

/// This will validate that all keys are valid against the state
pub(super) fn validate_identity_public_key_ids_dont_exist_in_state_v0(
    identity_id: Identifier,
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    transaction: TransactionArg,
    _execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    // first let's check that the identity has no keys with the same id
    let key_ids = identity_public_keys_with_witness
        .iter()
        .map(|key| key.id())
        .collect::<Vec<KeyID>>();
    let limit = key_ids.len() as u16;
    let identity_key_request = IdentityKeysRequest {
        identity_id: identity_id.to_buffer(),
        request_type: KeyRequestType::SpecificKeys(key_ids),
        limit: Some(limit),
        offset: None,
    };
    let keys = drive.fetch_identity_keys::<KeyIDVec>(
        identity_key_request,
        transaction,
        platform_version,
    )?;
    if !keys.is_empty() {
        // keys should all be empty
        Ok(SimpleConsensusValidationResult::new_with_error(
            BasicError::DuplicatedIdentityPublicKeyIdBasicError(
                DuplicatedIdentityPublicKeyIdBasicError::new(keys),
            )
            .into(),
        ))
    } else {
        Ok(SimpleConsensusValidationResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::version::DefaultForPlatformVersion;

    #[test]
    fn should_pass_when_key_ids_dont_exist_in_state() {
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

        // Create keys with IDs that don't exist in the identity
        let keys_in_creation: Vec<IdentityPublicKeyInCreation> = vec![
            IdentityPublicKeyInCreationV0 {
                id: 100,
                ..Default::default()
            }
            .into(),
            IdentityPublicKeyInCreationV0 {
                id: 101,
                ..Default::default()
            }
            .into(),
        ];

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_dont_exist_in_state_v0(
            identity_id,
            &keys_in_creation,
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(result.is_valid(), "should be valid when keys don't exist");
    }

    #[test]
    fn should_fail_when_key_ids_already_exist_in_state() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity =
            Identity::random_identity(3, Some(50), platform_version).expect("got an identity");
        let identity_id = identity.id();

        // Get one of the existing key IDs
        let existing_key_id = identity.public_keys().keys().next().copied().unwrap();

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

        // Create a key with an ID that already exists
        let keys_in_creation: Vec<IdentityPublicKeyInCreation> =
            vec![IdentityPublicKeyInCreationV0 {
                id: existing_key_id,
                ..Default::default()
            }
            .into()];

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_dont_exist_in_state_v0(
            identity_id,
            &keys_in_creation,
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(!result.is_valid(), "should be invalid when key IDs exist");
        assert_eq!(result.errors.len(), 1);
    }
}
