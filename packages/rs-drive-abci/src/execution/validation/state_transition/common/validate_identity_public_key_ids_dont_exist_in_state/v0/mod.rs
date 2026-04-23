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
    use dpp::consensus::ConsensusError;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::version::DefaultForPlatformVersion;
    use rand::SeedableRng;

    #[test]
    fn should_pass_when_identity_has_no_existing_keys_with_those_ids() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        // Use an identity id that doesn't exist in state so no key ids will be found.
        let identity_id = Identifier::random();

        let key: IdentityPublicKeyInCreation = IdentityPublicKeyInCreationV0 {
            id: 9999,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![1u8; 20]),
            signature: BinaryData::default(),
        }
        .into();

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_dont_exist_in_state_v0(
            identity_id,
            &[key],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(result.is_valid(), "should be valid with fresh key ids");
    }

    #[test]
    fn should_fail_when_key_id_already_exists_for_identity() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, keys_with_private): (Identity, Vec<(IdentityPublicKey, [u8; 32])>) =
            Identity::random_identity_with_main_keys_with_private_key(
                2,
                &mut rand::rngs::StdRng::seed_from_u64(55),
                platform_version,
            )
            .expect("got an identity");

        let identity_id = identity.id();

        // Pick an existing key id from this identity.
        let existing_key_id = keys_with_private[0].0.id();

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

        // Build a creation-key with the same id — this should be flagged as duplicate.
        let key: IdentityPublicKeyInCreation = IdentityPublicKeyInCreationV0 {
            id: existing_key_id,
            key_type: KeyType::ECDSA_HASH160,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![9u8; 20]),
            signature: BinaryData::default(),
        }
        .into();

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_dont_exist_in_state_v0(
            identity_id,
            &[key],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when key id already exists for the identity"
        );
        assert_eq!(result.errors.len(), 1);
        match &result.errors[0] {
            ConsensusError::BasicError(BasicError::DuplicatedIdentityPublicKeyIdBasicError(e)) => {
                assert!(e.duplicated_ids().contains(&existing_key_id));
            }
            other => panic!(
                "expected DuplicatedIdentityPublicKeyIdBasicError, got {:?}",
                other
            ),
        }
    }

    /// Empty input `&[]` → `key_ids = vec![]`, `limit = Some(0)`, and the
    /// built request is `SpecificKeys(vec![])`. The production path issues
    /// `drive.fetch_identity_keys` which must return an empty `KeyIDVec`, so
    /// the validator's branch `if !keys.is_empty()` is skipped and the
    /// function yields a valid (empty) `SimpleConsensusValidationResult`.
    /// This pins the "no keys to check" short-circuit behaviour: callers that
    /// pass an empty slice must not trigger a duplicated-id error.
    #[test]
    fn should_pass_when_empty_key_list() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let identity_id = Identifier::random();

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_identity_public_key_ids_dont_exist_in_state_v0(
            identity_id,
            &[],
            &platform.drive,
            None,
            &mut execution_context,
            platform_version,
        )
        .expect("should succeed");

        assert!(result.is_valid(), "empty list should be trivially valid");
        assert!(
            result.errors.is_empty(),
            "no consensus errors expected for an empty key list, got {:?}",
            result.errors
        );
    }
}
