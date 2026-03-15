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
