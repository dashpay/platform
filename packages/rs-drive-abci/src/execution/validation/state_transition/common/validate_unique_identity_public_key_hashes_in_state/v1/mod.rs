use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::consensus::state::identity::duplicated_identity_public_key_id_state_error::DuplicatedIdentityPublicKeyIdStateError;
use dpp::consensus::state::state_error::StateError;

use dpp::identity::KeyID;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::ProtocolError;

use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::version::PlatformVersion;
use std::collections::HashMap;

/// This will validate that all keys are valid against the state
/// v1: Returns StateError::DuplicatedIdentityPublicKeyIdStateError instead of BasicError
pub(super) fn validate_unique_identity_public_key_hashes_not_in_state_v1(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    _execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    // we should check that the public key is unique among all unique public keys

    let key_ids_map = identity_public_keys_with_witness
        .iter()
        .map(|key| Ok((key.hash()?, key.id())))
        .collect::<Result<HashMap<[u8; 20], KeyID>, ProtocolError>>()?;

    let duplicates = drive.has_any_of_unique_public_key_hashes(
        key_ids_map.keys().copied().collect(),
        transaction,
        platform_version,
    )?;

    let duplicate_ids = duplicates
        .into_iter()
        .map(|duplicate_key_hash| {
            key_ids_map
                .get(duplicate_key_hash.as_slice())
                .copied()
                .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                    "we should always have a value".to_string(),
                )))
        })
        .collect::<Result<Vec<KeyID>, Error>>()?;
    if !duplicate_ids.is_empty() {
        // Return StateError since we found duplicates in state
        Ok(SimpleConsensusValidationResult::new_with_error(
            StateError::DuplicatedIdentityPublicKeyIdStateError(
                DuplicatedIdentityPublicKeyIdStateError::new(duplicate_ids),
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
    fn should_pass_when_key_hashes_are_unique() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        // Use ECDSA_HASH160 which accepts 20-byte data without validation
        let keys_in_creation: Vec<IdentityPublicKeyInCreation> =
            vec![IdentityPublicKeyInCreationV0 {
                id: 100,
                key_type: KeyType::ECDSA_HASH160,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                contract_bounds: None,
                read_only: false,
                data: BinaryData::new(vec![
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
                ]),
                signature: BinaryData::default(),
            }
            .into()];

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_unique_identity_public_key_hashes_not_in_state_v1(
            &keys_in_creation,
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            result.is_valid(),
            "should be valid when no duplicate hashes in state"
        );
    }

    #[test]
    fn should_fail_with_state_error_when_key_hash_exists() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, keys_with_private): (Identity, Vec<(IdentityPublicKey, [u8; 32])>) =
            Identity::random_identity_with_main_keys_with_private_key(
                3,
                &mut rand::rngs::StdRng::seed_from_u64(50),
                platform_version,
            )
            .expect("got an identity");

        let existing_key = keys_with_private
            .iter()
            .find(|(k, _)| k.key_type() == KeyType::ECDSA_SECP256K1)
            .map(|(k, _)| k.clone())
            .unwrap();

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

        let key_in_creation: IdentityPublicKeyInCreation = IdentityPublicKeyInCreationV0 {
            id: 200,
            key_type: existing_key.key_type(),
            purpose: existing_key.purpose(),
            security_level: existing_key.security_level(),
            contract_bounds: None,
            read_only: false,
            data: existing_key.data().clone(),
            signature: BinaryData::default(),
        }
        .into();

        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("should create execution context");

        let result = validate_unique_identity_public_key_hashes_not_in_state_v1(
            &[key_in_creation],
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("should succeed");

        assert!(
            !result.is_valid(),
            "should be invalid when key hash exists in state"
        );
        assert_eq!(result.errors.len(), 1);
        // v1 returns StateError instead of BasicError
        match &result.errors[0] {
            ConsensusError::StateError(StateError::DuplicatedIdentityPublicKeyIdStateError(_)) => {}
            other => panic!(
                "expected DuplicatedIdentityPublicKeyIdStateError, got {:?}",
                other
            ),
        }
    }
}
