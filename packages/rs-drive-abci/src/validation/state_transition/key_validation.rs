use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError,
    InvalidIdentityPublicKeySecurityLevelError,
};
use dpp::consensus::signature::{
    IdentityNotFoundError, InvalidSignaturePublicKeySecurityLevelError,
};
use dpp::consensus::ConsensusError;
use dpp::identity::security_level::ALLOWED_SECURITY_LEVELS;
use dpp::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreationWithWitness;
use dpp::identity::state_transition::identity_update_transition::validate_public_keys::IDENTITY_PLATFORM_VALUE_SCHEMA;
use dpp::identity::validation::{duplicated_key_ids_witness, duplicated_keys_witness};
use dpp::identity::{KeyID, PartialIdentity};
use dpp::platform_value::Identifier;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::ProtocolError;
use dpp::StateError::MissingIdentityPublicKeyIdsError;
use dpp::{
    consensus::signature::{
        InvalidIdentityPublicKeyTypeError, MissingPublicKeyError, PublicKeyIsDisabledError,
        SignatureError,
    },
    state_transition::validation::validate_state_transition_identity_signature::convert_to_consensus_signature_error,
    NativeBlsModule, StateError,
};
use drive::dpp::identity::KeyType;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDVec, KeyRequestType};
use drive::drive::Drive;
use drive::grovedb::{Transaction, TransactionArg};
use lazy_static::lazy_static;
use std::collections::{BTreeSet, HashMap, HashSet};

lazy_static! {
    static ref SUPPORTED_KEY_TYPES: HashSet<KeyType> = {
        let mut keys = HashSet::new();
        keys.insert(KeyType::ECDSA_SECP256K1);
        keys.insert(KeyType::BLS12_381);
        keys.insert(KeyType::ECDSA_HASH160);
        keys
    };
}

pub fn validate_state_transition_identity_signature(
    drive: &Drive,
    state_transition: &impl StateTransitionIdentitySigned,
    request_revision: bool,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
    let mut validation_result = ConsensusValidationResult::<PartialIdentity>::default();

    let key_id = state_transition.get_signature_public_key_id().ok_or(
        ProtocolError::CorruptedCodeExecution(format!(
            "state_transition does not have a public key Id to verify"
        )),
    )?;

    let key_request = IdentityKeysRequest::new_specific_key_query(
        state_transition.get_owner_id().as_bytes(),
        key_id,
    );

    let maybe_partial_identity = if request_revision {
        drive.fetch_identity_balance_with_keys_and_revision(key_request, transaction)?
    } else {
        drive.fetch_identity_balance_with_keys(key_request, transaction)?
    };

    let partial_identity = match maybe_partial_identity {
        None => {
            // dbg!(bs58::encode(&state_transition.get_owner_id()).into_string());
            validation_result.add_error(SignatureError::IdentityNotFoundError(
                IdentityNotFoundError::new(*state_transition.get_owner_id()),
            ));
            return Ok(validation_result);
        }
        Some(pk) => pk,
    };

    if !partial_identity.not_found_public_keys.is_empty() {
        validation_result.add_error(SignatureError::MissingPublicKeyError(
            MissingPublicKeyError::new(key_id),
        ));
        return Ok(validation_result);
    }

    let Some(public_key) = partial_identity.loaded_public_keys.get(&key_id) else {
        validation_result.add_error(SignatureError::MissingPublicKeyError(
            MissingPublicKeyError::new(key_id),
        ));
        return Ok(validation_result);
    };

    if !SUPPORTED_KEY_TYPES.contains(&public_key.key_type) {
        validation_result.add_error(SignatureError::InvalidIdentityPublicKeyTypeError(
            InvalidIdentityPublicKeyTypeError::new(public_key.key_type),
        ));
        return Ok(validation_result);
    }

    let security_levels = state_transition.get_security_level_requirement();

    if !security_levels.contains(&public_key.security_level) {
        validation_result.add_error(SignatureError::InvalidSignaturePublicKeySecurityLevelError(
            InvalidSignaturePublicKeySecurityLevelError::new(
                public_key.security_level,
                security_levels,
            ),
        ));
        return Ok(validation_result);
    }

    if public_key.is_disabled() {
        validation_result.add_error(SignatureError::PublicKeyIsDisabledError(
            PublicKeyIsDisabledError::new(public_key.id),
        ));
        return Ok(validation_result);
    }

    // let operation = SignatureVerificationOperation::new(public_key.key_type);
    // execution_context.add_operation(Operation::SignatureVerification(operation));
    //
    // if execution_context.is_dry_run() {
    //     return Ok(validation_result);
    // }

    let signature_is_valid =
        state_transition.verify_signature(&public_key, &NativeBlsModule::default());

    if let Err(err) = signature_is_valid {
        let consensus_error = convert_to_consensus_signature_error(err)?;
        validation_result.add_error(consensus_error);
        return Ok(validation_result);
    }

    validation_result.set_data(partial_identity);

    Ok(validation_result)
}

/// This validation will validate the count of new keys, that there are no duplicates either by
/// id or by data. This is done before signature and state validation to remove potential
/// attack vectors.
pub fn validate_identity_public_keys_structure(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreationWithWitness],
) -> Result<SimpleConsensusValidationResult, Error> {
    let max_items: usize = IDENTITY_PLATFORM_VALUE_SCHEMA
        .get_integer_at_path("properties.publicKeys.maxItems")
        .map_err(ProtocolError::ValueError)?;

    if identity_public_keys_with_witness.len() > max_items {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            StateError::MaxIdentityPublicKeyLimitReachedError { max_items }.into(),
        ));
    }

    // Check that there's not duplicates key ids in the state transition
    let duplicated_ids = duplicated_key_ids_witness(&identity_public_keys_with_witness);
    if !duplicated_ids.is_empty() {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            StateError::DuplicatedIdentityPublicKeyIdError { duplicated_ids }.into(),
        ));
    }

    // Check that there's no duplicated keys
    let duplicated_key_ids = duplicated_keys_witness(&identity_public_keys_with_witness);
    if !duplicated_key_ids.is_empty() {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            StateError::DuplicatedIdentityPublicKeyError {
                duplicated_public_key_ids: duplicated_key_ids,
            }
            .into(),
        ));
    }

    // We should check all the security levels
    let validation_errors = identity_public_keys_with_witness
        .into_iter()
        .filter_map(|identity_public_key| {
            let allowed_security_levels = ALLOWED_SECURITY_LEVELS.get(&identity_public_key.purpose);
            if let Some(levels) = allowed_security_levels {
                if !levels.contains(&identity_public_key.security_level) {
                    Some(
                        InvalidIdentityPublicKeySecurityLevelError::new(
                            identity_public_key.id,
                            identity_public_key.purpose,
                            identity_public_key.security_level,
                            Some(levels.clone()),
                        )
                        .into(),
                    )
                } else {
                    None //No error
                }
            } else {
                Some(
                    InvalidIdentityPublicKeySecurityLevelError::new(
                        identity_public_key.id,
                        identity_public_key.purpose,
                        identity_public_key.security_level,
                        None,
                    )
                    .into(),
                )
            }
        })
        .collect();
    Ok(SimpleConsensusValidationResult::new_with_errors(
        validation_errors,
    ))
}

/// This validation will validate that all keys are valid for their type and that all signatures
/// are also valid.
pub fn validate_identity_public_keys_signatures(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreationWithWitness],
) -> Result<SimpleConsensusValidationResult, Error> {
    let validation_errors = identity_public_keys_with_witness
        .into_iter()
        .map(|identity_public_key| {
            identity_public_key
                .verify_signature()
                .map_err(Error::Protocol)
        })
        .collect::<Result<Vec<SimpleConsensusValidationResult>, Error>>()?;

    Ok(SimpleConsensusValidationResult::merge_many_errors(
        validation_errors,
    ))
}

/// This will validate that all keys are valid against the state
pub fn validate_identity_public_key_ids_dont_exist_in_state(
    identity_id: Identifier,
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreationWithWitness],
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
            ConsensusError::DuplicatedIdentityPublicKeyBasicIdError(
                DuplicatedIdentityPublicKeyIdError::new(keys),
            ),
        ))
    } else {
        Ok(SimpleConsensusValidationResult::default())
    }
}

/// This will validate that all keys are valid against the state
pub fn validate_identity_public_key_ids_exist_in_state(
    identity_id: Identifier,
    mut key_ids: Vec<KeyID>,
    drive: &Drive,
    transaction: TransactionArg,
) -> Result<SimpleConsensusValidationResult, Error> {
    let identity_key_request = IdentityKeysRequest {
        identity_id: identity_id.to_buffer(),
        request_type: KeyRequestType::SpecificKeys(key_ids.clone()),
        limit: None,
        offset: None,
    };
    let keys = drive.fetch_identity_keys::<KeyIDVec>(identity_key_request, transaction)?;
    if keys.len() != key_ids.len() {
        let to_remove = BTreeSet::from_iter(keys);
        key_ids.retain(|found_key| !to_remove.contains(found_key));
        // keys should all exist
        Ok(SimpleConsensusValidationResult::new_with_error(
            MissingIdentityPublicKeyIdsError { ids: key_ids }.into(),
        ))
    } else {
        Ok(SimpleConsensusValidationResult::default())
    }
}

/// This will validate that all keys are valid against the state
pub fn validate_unique_identity_public_key_hashes_state(
    identity_public_keys_with_witness: &[IdentityPublicKeyInCreationWithWitness],
    drive: &Drive,
    transaction: TransactionArg,
) -> Result<SimpleConsensusValidationResult, Error> {
    // we should check that the public key is unique among all unique public keys

    let key_ids_map = identity_public_keys_with_witness
        .iter()
        .map(|key| Ok((key.hash()?, key.id)))
        .collect::<Result<HashMap<[u8; 20], KeyID>, ProtocolError>>()?;

    let duplicates = drive
        .has_any_of_unique_public_key_hashes(key_ids_map.keys().copied().collect(), transaction)?;

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
            ConsensusError::DuplicatedIdentityPublicKeyBasicError(
                DuplicatedIdentityPublicKeyError::new(duplicate_ids),
            ),
        ))
    } else {
        Ok(SimpleConsensusValidationResult::default())
    }
}
