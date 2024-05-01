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
