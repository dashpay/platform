use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyIdBasicError, InvalidIdentityPublicKeySecurityLevelError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::signature::{
    IdentityNotFoundError, InvalidSignaturePublicKeySecurityLevelError,
};
use dpp::consensus::state::identity::duplicated_identity_public_key_state_error::DuplicatedIdentityPublicKeyStateError;
use dpp::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use dpp::consensus::state::identity::missing_identity_public_key_ids_error::MissingIdentityPublicKeyIdsError;
use dpp::consensus::state::state_error::StateError;

use dpp::identity::security_level::ALLOWED_SECURITY_LEVELS;
use dpp::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation;
use dpp::identity::state_transition::identity_update_transition::validate_public_keys::IDENTITY_PLATFORM_VALUE_SCHEMA;
use dpp::identity::validation::{duplicated_key_ids_witness, duplicated_keys_witness};
use dpp::identity::{KeyID, PartialIdentity};
use dpp::platform_value::Identifier;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::ProtocolError;
use dpp::{
    consensus::signature::{
        InvalidIdentityPublicKeyTypeError, MissingPublicKeyError, PublicKeyIsDisabledError,
        SignatureError,
    },
    state_transition::validation::validate_state_transition_identity_signature::convert_to_consensus_signature_error,
    NativeBlsModule,
};
use drive::dpp::identity::KeyType;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDVec, KeyRequestType};
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use lazy_static::lazy_static;
use std::collections::{BTreeSet, HashMap, HashSet};

/// This will validate that all keys are valid against the state
pub(in crate::validation) fn validate_identity_public_key_ids_dont_exist_in_state_v0(
    identity_id: Identifier,
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreation],
    drive: &Drive,
    transaction: TransactionArg,
) -> Result<SimpleConsensusValidationResult, Error> {
    // first let's check that the identity has no keys with the same id
    let key_ids = identity_public_keys_with_witness
        .iter()
        .map(|key| key.id)
        .collect::<Vec<KeyID>>();
    let limit = key_ids.len() as u16;
    let identity_key_request = IdentityKeysRequest {
        identity_id: identity_id.to_buffer(),
        request_type: KeyRequestType::SpecificKeys(key_ids),
        limit: Some(limit),
        offset: None,
    };
    let keys = drive.fetch_identity_keys::<KeyIDVec>(identity_key_request, transaction)?;
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
