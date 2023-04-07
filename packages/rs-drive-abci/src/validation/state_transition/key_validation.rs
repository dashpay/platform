use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::bls_signatures::Serialize;
use dpp::consensus::basic::identity::{
    DuplicatedIdentityPublicKeyError, DuplicatedIdentityPublicKeyIdError,
    InvalidIdentityPublicKeySecurityLevelError,
};
use dpp::consensus::ConsensusError;
use dpp::identity::security_level::ALLOWED_SECURITY_LEVELS;
use dpp::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyWithWitness;
use dpp::identity::state_transition::identity_update_transition::validate_public_keys::{
    IDENTITY_JSON_SCHEMA, IDENTITY_PLATFORM_VALUE_SCHEMA,
};
use dpp::identity::validation::{duplicated_key_ids_witness, duplicated_keys_witness};
use dpp::identity::{KeyID, PartialIdentity};
use dpp::platform_value::Identifier;
use dpp::prelude::IdentityPublicKey;
use dpp::state_transition::fee::operations::{Operation, SignatureVerificationOperation};
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::{
    consensus::signature::{
        InvalidIdentityPublicKeyTypeError, MissingPublicKeyError, PublicKeyIsDisabledError,
        SignatureError,
    },
    state_transition::validation::validate_state_transition_identity_signature::convert_to_consensus_signature_error,
    NativeBlsModule, PublicKeyValidationError, StateError,
};
use dpp::{BlsModule, ProtocolError};
use drive::dpp::identity::KeyType;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDHashSet, KeyIDVec, KeyRequestType,
    OptionalSingleIdentityPublicKeyOutcome,
};
use drive::drive::Drive;
use drive::grovedb::{Transaction, TransactionArg};
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};

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

    let maybe_public_key: OptionalSingleIdentityPublicKeyOutcome =
        drive.fetch_identity_keys(key_request, transaction)?;

    let public_key = match maybe_public_key {
        None => {
            validation_result.add_error(SignatureError::MissingPublicKeyError(
                MissingPublicKeyError::new(key_id),
            ));
            return Ok(validation_result);
        }
        Some(pk) => pk,
    };

    if !SUPPORTED_KEY_TYPES.contains(&public_key.key_type) {
        validation_result.add_error(SignatureError::InvalidIdentityPublicKeyTypeError(
            InvalidIdentityPublicKeyTypeError::new(public_key.key_type),
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

    Ok(validation_result)
}

/// This validation will validate the count of new keys, that there are no duplicates either by
/// id or by data. This is done before signature and state validation to remove potential
/// attack vectors.
pub fn validate_identity_public_keys_structure(
    identity_public_keys_with_witness: &[IdentityPublicKeyWithWitness],
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
fn validate_identity_public_keys_signatures(
    identity_public_keys_with_witness: &[IdentityPublicKeyWithWitness],
) -> Result<SimpleConsensusValidationResult, Error> {
    let validation_errors = identity_public_keys_with_witness
        .into_iter()
        .map(|identity_public_key| {
            identity_public_key
                .verify_signature()
                .map_err(Error::Protocol)
        })
        .collect::<Result<Vec<SimpleConsensusValidationResult>, Error>>()?;

    Ok(SimpleConsensusValidationResult::merge_many(
        validation_errors,
    ))
}

/// This will validate that all keys are valid against the state
fn validate_add_identity_public_keys_state(
    identity_id: Identifier,
    identity_public_keys_with_witness: &[IdentityPublicKeyWithWitness],
    drive: &Drive,
    transaction: TransactionArg,
) -> Result<SimpleConsensusValidationResult, Error> {
    // first let's check that the identity has no keys with the same id
    let key_ids = identity_public_keys_with_witness
        .iter()
        .map(|key| key.id)
        .collect();
    let identity_key_request = IdentityKeysRequest {
        identity_id: identity_id.to_buffer(),
        request_type: KeyRequestType::SpecificKeys(key_ids),
        limit: None,
        offset: None,
    };
    let keys = drive.fetch_identity_keys::<KeyIDVec>(identity_key_request, transaction)?;
    if !keys.is_empty() {
        // keys should all be empty
        return Ok(SimpleConsensusValidationResult::new_with_error(
            ConsensusError::DuplicatedIdentityPublicKeyBasicIdError(
                DuplicatedIdentityPublicKeyIdError::new(keys),
            ),
        ));
    }

    // next we should check that the public key is unique among all unique public keys

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
