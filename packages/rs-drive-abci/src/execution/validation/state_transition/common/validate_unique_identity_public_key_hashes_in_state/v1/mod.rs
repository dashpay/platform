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
