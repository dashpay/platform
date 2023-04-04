use crate::error::Error;
use dpp::state_transition::fee::operations::{Operation, SignatureVerificationOperation};
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::{
    consensus::signature::{
        InvalidIdentityPublicKeyTypeError, MissingPublicKeyError, PublicKeyIsDisabledError,
        SignatureError,
    },
    state_transition::validation::validate_state_transition_identity_signature::convert_to_consensus_signature_error,
};
use dpp::{BlsModule, ProtocolError};
use drive::dpp::identity::KeyType;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, OptionalSingleIdentityPublicKeyOutcome,
};
use drive::drive::Drive;
use drive::grovedb::Transaction;
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

pub fn validate_state_transition_identity_signature(
    drive: &Drive,
    state_transition: &impl StateTransitionIdentitySigned,
    transaction: &Transaction,
    bls: &impl BlsModule,
    execution_context: &StateTransitionExecutionContext,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut validation_result = SimpleConsensusValidationResult::default();

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
        drive.fetch_identity_keys(key_request, Some(transaction))?;

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

    let operation = SignatureVerificationOperation::new(public_key.key_type);
    execution_context.add_operation(Operation::SignatureVerification(operation));

    if execution_context.is_dry_run() {
        return Ok(validation_result);
    }

    let signature_is_valid = state_transition.verify_signature(&public_key, bls);

    if let Err(err) = signature_is_valid {
        let consensus_error = convert_to_consensus_signature_error(err)?;
        validation_result.add_error(consensus_error);
        return Ok(validation_result);
    }

    Ok(validation_result)
}
