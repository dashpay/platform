use crate::error::Error;

use dpp::consensus::signature::{
    IdentityNotFoundError, InvalidSignaturePublicKeySecurityLevelError,
};

use dpp::identity::PartialIdentity;

use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::validation::ConsensusValidationResult;
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
use drive::drive::identity::key::fetch::IdentityKeysRequest;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use lazy_static::lazy_static;
use std::collections::HashSet;

lazy_static! {
    static ref SUPPORTED_KEY_TYPES: HashSet<KeyType> = {
        let mut keys = HashSet::new();
        keys.insert(KeyType::ECDSA_SECP256K1);
        keys.insert(KeyType::BLS12_381);
        keys.insert(KeyType::ECDSA_HASH160);
        keys
    };
}

pub(crate) fn validate_state_transition_identity_signature_v0(
    drive: &Drive,
    state_transition: &impl StateTransitionIdentitySigned,
    request_revision: bool,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
    let mut validation_result = ConsensusValidationResult::<PartialIdentity>::default();

    let key_id = state_transition.get_signature_public_key_id().ok_or(
        ProtocolError::CorruptedCodeExecution(
            "state_transition does not have a public key Id to verify".to_string(),
        ),
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
        Some(partial_identity) => partial_identity,
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
        state_transition.verify_signature(public_key, &NativeBlsModule::default());

    if let Err(err) = signature_is_valid {
        let consensus_error = convert_to_consensus_signature_error(err)?;
        validation_result.add_error(consensus_error);
        return Ok(validation_result);
    }

    validation_result.set_data(partial_identity);

    Ok(validation_result)
}
