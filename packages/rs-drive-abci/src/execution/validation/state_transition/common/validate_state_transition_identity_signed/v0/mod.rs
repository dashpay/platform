use crate::error::Error;

use dpp::consensus::signature::{
    IdentityNotFoundError, InvalidSignaturePublicKeyPurposeError,
    InvalidSignaturePublicKeySecurityLevelError, InvalidStateTransitionSignatureError,
    PublicKeySecurityLevelNotMetError,
};

use dpp::identity::PartialIdentity;

use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::consensus::ConsensusError;

use crate::error::execution::ExecutionError;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::state_transition::StateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use dpp::{
    consensus::signature::{
        InvalidIdentityPublicKeyTypeError, MissingPublicKeyError, PublicKeyIsDisabledError,
        SignatureError,
    },
    NativeBlsModule,
};
use drive::dpp::identity::KeyType;
use drive::drive::identity::key::fetch::IdentityKeysRequest;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
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

pub(super) trait ValidateStateTransitionIdentitySignatureV0<'a> {
    fn validate_state_transition_identity_signed_v0(
        &self,
        drive: &Drive,
        action: Option<&StateTransitionAction>,
        request_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;
}

impl<'a> ValidateStateTransitionIdentitySignatureV0<'a> for StateTransition {
    fn validate_state_transition_identity_signed_v0(
        &self,
        drive: &Drive,
        action: Option<&StateTransitionAction>,
        request_identity_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error> {
        let mut validation_result = ConsensusValidationResult::<PartialIdentity>::default();

        let key_id =
            self.signature_public_key_id()
                .ok_or(ProtocolError::CorruptedCodeExecution(
                    "state_transition does not have a public key Id to verify".to_string(),
                ))?;

        let owner_id = self.owner_id();

        let security_levels = match self {
            StateTransition::DocumentsBatch(_) => {
                // We will have an action during consensus validation, but not on mempool check_tx
                if let Some(action) = action {
                    let StateTransitionAction::DocumentsBatchAction(documents_batch_action) =
                        action
                    else {
                        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                                "we expect a documents batch state transition action when validating the signature of the documents batch transition",
                            )));
                    };
                    documents_batch_action.contract_based_security_level_requirement()
                } else {
                    // In this case we just validate the transition
                    // If it is not valid during consensus validation because the key wasn't the
                    // right level then we will disable the key as it most likely is an attack
                    self.security_level_requirement()
                        .ok_or(ProtocolError::CorruptedCodeExecution(
                            "state_transition does not have security level".to_string(),
                        ))
                }
            }
            _ => self
                .security_level_requirement()
                .ok_or(ProtocolError::CorruptedCodeExecution(
                    "state_transition does not have security level".to_string(),
                )),
        }?;

        let purpose = self
            .purpose_requirement()
            .ok_or(ProtocolError::CorruptedCodeExecution(
                "state_transition does not have a key purpose requirement".to_string(),
            ))?;

        let key_request = IdentityKeysRequest::new_specific_key_query(owner_id.as_bytes(), key_id);

        let maybe_partial_identity = if request_identity_revision {
            drive.fetch_identity_balance_with_keys_and_revision(
                key_request,
                transaction,
                platform_version,
            )?
        } else {
            drive.fetch_identity_balance_with_keys(key_request, transaction, platform_version)?
        };

        let partial_identity = match maybe_partial_identity {
            None => {
                // dbg!(bs58::encode(&state_transition.get_owner_id()).into_string());
                validation_result.add_error(SignatureError::IdentityNotFoundError(
                    IdentityNotFoundError::new(owner_id),
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

        if !SUPPORTED_KEY_TYPES.contains(&public_key.key_type()) {
            validation_result.add_error(SignatureError::InvalidIdentityPublicKeyTypeError(
                InvalidIdentityPublicKeyTypeError::new(public_key.key_type()),
            ));
            return Ok(validation_result);
        }

        if purpose != public_key.purpose() {
            validation_result.add_error(SignatureError::InvalidSignaturePublicKeyPurposeError(
                InvalidSignaturePublicKeyPurposeError::new(public_key.purpose(), purpose),
            ));
            return Ok(validation_result);
        }

        if !security_levels.contains(&public_key.security_level()) {
            validation_result.add_error(
                SignatureError::InvalidSignaturePublicKeySecurityLevelError(
                    InvalidSignaturePublicKeySecurityLevelError::new(
                        public_key.security_level(),
                        security_levels,
                    ),
                ),
            );
            return Ok(validation_result);
        }

        if public_key.is_disabled() {
            validation_result.add_error(SignatureError::PublicKeyIsDisabledError(
                PublicKeyIsDisabledError::new(public_key.id()),
            ));
            return Ok(validation_result);
        }

        let operation = SignatureVerificationOperation::new(public_key.key_type());
        execution_context.add_operation(ValidationOperation::SignatureVerification(operation));

        let signature_is_valid = self.verify_signature(public_key, &NativeBlsModule);

        if let Err(err) = signature_is_valid {
            let consensus_error = convert_to_consensus_signature_error(err)?;
            validation_result.add_error(consensus_error);
            return Ok(validation_result);
        }

        validation_result.set_data(partial_identity);

        Ok(validation_result)
    }
}

pub fn convert_to_consensus_signature_error(
    error: ProtocolError,
) -> Result<ConsensusError, ProtocolError> {
    match error {
        ProtocolError::InvalidSignaturePublicKeySecurityLevelError(err) => {
            Ok(ConsensusError::SignatureError(
                SignatureError::InvalidSignaturePublicKeySecurityLevelError(err),
            ))
        }
        ProtocolError::PublicKeySecurityLevelNotMetError(err) => Ok(
            ConsensusError::SignatureError(SignatureError::PublicKeySecurityLevelNotMetError(
                PublicKeySecurityLevelNotMetError::new(
                    err.public_key_security_level(),
                    err.required_security_level(),
                ),
            )),
        ),
        ProtocolError::PublicKeyIsDisabledError(err) => Ok(ConsensusError::SignatureError(
            SignatureError::PublicKeyIsDisabledError(PublicKeyIsDisabledError::new(
                err.public_key_id(),
            )),
        )),
        ProtocolError::InvalidIdentityPublicKeyTypeError(err) => Ok(
            ConsensusError::SignatureError(SignatureError::InvalidIdentityPublicKeyTypeError(
                InvalidIdentityPublicKeyTypeError::new(err.public_key_type()),
            )),
        ),
        ProtocolError::WrongPublicKeyPurposeError(err) => Ok(err.into()),
        ProtocolError::Error(_) => Err(error),
        _ => Ok(ConsensusError::SignatureError(
            SignatureError::InvalidStateTransitionSignatureError(
                InvalidStateTransitionSignatureError::new(),
            ),
        )),
    }
}
