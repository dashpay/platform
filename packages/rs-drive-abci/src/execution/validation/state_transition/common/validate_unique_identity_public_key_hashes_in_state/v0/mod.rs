use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::consensus::basic::identity::DuplicatedIdentityPublicKeyIdBasicError;
use dpp::consensus::basic::BasicError;

use dpp::identity::KeyID;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::ProtocolError;

use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::version::PlatformVersion;
use std::collections::HashMap;

/// This will validate that all keys are valid against the state
pub(crate) fn validate_unique_identity_public_key_hashes_in_state_v0(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    // we should check that the public key is unique among all unique public keys

    let key_ids_map = identity_public_keys_with_witness
        .iter()
        .map(|key| Ok((key.hash(platform_version)?, key.id())))
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
        Ok(SimpleConsensusValidationResult::new_with_error(
            BasicError::DuplicatedIdentityPublicKeyIdBasicError(
                DuplicatedIdentityPublicKeyIdBasicError::new(duplicate_ids),
            )
            .into(),
        ))
    } else {
        Ok(SimpleConsensusValidationResult::default())
    }
}
