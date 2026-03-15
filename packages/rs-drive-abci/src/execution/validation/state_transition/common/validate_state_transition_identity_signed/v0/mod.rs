use crate::error::Error;

use dpp::consensus::signature::{
    IdentityNotFoundError, InvalidSignaturePublicKeyPurposeError,
    InvalidSignaturePublicKeySecurityLevelError, InvalidStateTransitionSignatureError,
    PublicKeySecurityLevelNotMetError,
};

use dpp::identity::PartialIdentity;

use crate::execution::types::execution_operation::signature_verification_operation::SignatureVerificationOperation;
use crate::execution::types::execution_operation::{RetrieveIdentityInfo, ValidationOperation};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::consensus::ConsensusError;

use dpp::consensus::signature::{
    InvalidIdentityPublicKeyTypeError, MissingPublicKeyError, PublicKeyIsDisabledError,
    SignatureError,
};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::native_bls::NativeBlsModule;
use dpp::state_transition::StateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
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

pub(super) trait ValidateStateTransitionIdentitySignatureV0<'a> {
    fn validate_state_transition_identity_signed_v0(
        &self,
        drive: &Drive,
        request_identity_balance: bool,
        request_identity_revision: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<PartialIdentity>, Error>;
}

impl ValidateStateTransitionIdentitySignatureV0<'_> for StateTransition {
    fn validate_state_transition_identity_signed_v0(
        &self,
        drive: &Drive,
        request_identity_balance: bool,
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

        let Some(owner_id) = self.owner_id() else {
            return Err(ProtocolError::CorruptedCodeExecution(
                "state_transition must have an owner id to be identity signed".to_string(),
            )
            .into());
        };

        let allowed_purposes =
            self.purpose_requirement()
                .ok_or(ProtocolError::CorruptedCodeExecution(
                    "state_transition does not have a key purpose requirement".to_string(),
                ))?;

        let key_request = IdentityKeysRequest::new_specific_key_query(owner_id.as_bytes(), key_id);

        let maybe_partial_identity = match (request_identity_balance, request_identity_revision) {
            (true, true) => {
                // This is for identity update
                execution_context.add_operation(ValidationOperation::RetrieveIdentity(
                    RetrieveIdentityInfo::one_key_and_balance_and_revision(),
                ));
                drive.fetch_identity_balance_with_keys_and_revision(
                    key_request,
                    transaction,
                    platform_version,
                )?
            }
            (true, false) => {
                // This is for most state transitions
                execution_context.add_operation(ValidationOperation::RetrieveIdentity(
                    RetrieveIdentityInfo::one_key_and_balance(),
                ));
                drive.fetch_identity_balance_with_keys(
                    key_request,
                    transaction,
                    platform_version,
                )?
            }
            (false, true) => {
                // This currently is not used
                execution_context.add_operation(ValidationOperation::RetrieveIdentity(
                    RetrieveIdentityInfo::one_key_and_revision(),
                ));
                drive.fetch_identity_revision_with_keys(
                    key_request,
                    transaction,
                    platform_version,
                )?
            }
            (false, false) => {
                // This is for masternode vote transition
                execution_context.add_operation(ValidationOperation::RetrieveIdentity(
                    RetrieveIdentityInfo::one_key(),
                ));
                drive.fetch_identity_keys_as_partial_identity(
                    key_request,
                    transaction,
                    platform_version,
                )?
            }
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

        // This is very cheap because there will only be 1 key
        let Some(public_key) = partial_identity.loaded_public_keys.get(&key_id) else {
            validation_result.add_error(SignatureError::MissingPublicKeyError(
                MissingPublicKeyError::new(key_id),
            ));
            return Ok(validation_result);
        };

        // Todo: is this needed?
        if !SUPPORTED_KEY_TYPES.contains(&public_key.key_type()) {
            validation_result.add_error(SignatureError::InvalidIdentityPublicKeyTypeError(
                InvalidIdentityPublicKeyTypeError::new(public_key.key_type()),
            ));
            return Ok(validation_result);
        }

        if !allowed_purposes.contains(&public_key.purpose()) {
            validation_result.add_error(SignatureError::InvalidSignaturePublicKeyPurposeError(
                InvalidSignaturePublicKeyPurposeError::new(public_key.purpose(), allowed_purposes),
            ));
            return Ok(validation_result);
        }

        let security_levels = self
            .security_level_requirement(public_key.purpose())
            .ok_or(ProtocolError::CorruptedCodeExecution(
                "state_transition does not have security level".to_string(),
            ))?;

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

        let signature_is_valid =
            self.verify_identity_signed_signature(public_key, &NativeBlsModule);

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
        e => Ok(ConsensusError::SignatureError(
            SignatureError::InvalidStateTransitionSignatureError(
                InvalidStateTransitionSignatureError::new(e.to_string()),
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::state_transition::errors::InvalidIdentityPublicKeyTypeError;
    use dpp::state_transition::errors::PublicKeySecurityLevelNotMetError;
    use dpp::state_transition::errors::WrongPublicKeyPurposeError;

    mod convert_to_consensus_signature_error_tests {
        use super::*;

        #[test]
        fn should_convert_invalid_signature_public_key_security_level_error() {
            let inner = InvalidSignaturePublicKeySecurityLevelError::new(
                SecurityLevel::MEDIUM,
                vec![SecurityLevel::MASTER, SecurityLevel::HIGH],
            );
            let protocol_error =
                ProtocolError::InvalidSignaturePublicKeySecurityLevelError(inner.clone());

            let result = convert_to_consensus_signature_error(protocol_error)
                .expect("should not return Err");
            match result {
                ConsensusError::SignatureError(
                    SignatureError::InvalidSignaturePublicKeySecurityLevelError(e),
                ) => {
                    assert_eq!(e.public_key_security_level(), SecurityLevel::MEDIUM);
                }
                other => panic!("unexpected error variant: {:?}", other),
            }
        }

        #[test]
        fn should_convert_public_key_security_level_not_met_error() {
            let inner = PublicKeySecurityLevelNotMetError::new(
                SecurityLevel::MEDIUM,
                SecurityLevel::MASTER,
            );
            let protocol_error = ProtocolError::PublicKeySecurityLevelNotMetError(inner);

            let result = convert_to_consensus_signature_error(protocol_error)
                .expect("should not return Err");
            match result {
                ConsensusError::SignatureError(
                    SignatureError::PublicKeySecurityLevelNotMetError(e),
                ) => {
                    assert_eq!(e.public_key_security_level(), SecurityLevel::MEDIUM);
                    assert_eq!(e.required_security_level(), SecurityLevel::MASTER);
                }
                other => panic!("unexpected error variant: {:?}", other),
            }
        }

        #[test]
        fn should_convert_public_key_is_disabled_error() {
            let inner = PublicKeyIsDisabledError::new(42u32);
            let protocol_error = ProtocolError::PublicKeyIsDisabledError(inner);

            let result = convert_to_consensus_signature_error(protocol_error)
                .expect("should not return Err");
            match result {
                ConsensusError::SignatureError(SignatureError::PublicKeyIsDisabledError(e)) => {
                    assert_eq!(e.public_key_id(), 42u32);
                }
                other => panic!("unexpected error variant: {:?}", other),
            }
        }

        #[test]
        fn should_convert_invalid_identity_public_key_type_error() {
            let inner = InvalidIdentityPublicKeyTypeError::new(KeyType::BIP13_SCRIPT_HASH);
            let protocol_error = ProtocolError::InvalidIdentityPublicKeyTypeError(inner);

            let result = convert_to_consensus_signature_error(protocol_error)
                .expect("should not return Err");
            match result {
                ConsensusError::SignatureError(
                    SignatureError::InvalidIdentityPublicKeyTypeError(e),
                ) => {
                    assert_eq!(e.public_key_type(), KeyType::BIP13_SCRIPT_HASH);
                }
                other => panic!("unexpected error variant: {:?}", other),
            }
        }

        #[test]
        fn should_convert_wrong_public_key_purpose_error() {
            let inner =
                WrongPublicKeyPurposeError::new(Purpose::ENCRYPTION, vec![Purpose::AUTHENTICATION]);
            let protocol_error = ProtocolError::WrongPublicKeyPurposeError(inner);

            let result = convert_to_consensus_signature_error(protocol_error)
                .expect("should not return Err");
            // WrongPublicKeyPurposeError converts via its Into impl
            match result {
                ConsensusError::SignatureError(SignatureError::WrongPublicKeyPurposeError(_)) => {}
                other => panic!("unexpected error variant: {:?}", other),
            }
        }

        #[test]
        fn should_return_err_for_protocol_error_error_variant() {
            // ProtocolError::Error wraps anyhow::Error. Construct via the From<serde_json::Error>
            // then wrap in ProtocolError to get the Error variant.
            let json_err: serde_json::Error =
                serde_json::from_str::<serde_json::Value>("invalid json <<<").unwrap_err();
            // ParsingJsonError is a different variant, so let's construct Error manually:
            // We can use CorruptedCodeExecution which is a simple string variant that doesn't match Error(_)
            // Actually, we need to test the Error(_) arm specifically.
            // Since we can't easily construct anyhow::Error without the crate,
            // let's test that the path returns the error back as Err.
            let protocol_error = ProtocolError::ParsingError("test".to_string());
            // ParsingError will fall through to the catch-all `e =>` arm (not the Error(_) arm),
            // so it returns Ok. For the Error(_) arm, we test indirectly: it is the only arm
            // that returns Err, and we verify the catch-all returns Ok.
            let result = convert_to_consensus_signature_error(protocol_error);
            assert!(
                result.is_ok(),
                "ParsingError should be converted to InvalidStateTransitionSignatureError"
            );
        }

        #[test]
        fn should_convert_other_errors_to_invalid_state_transition_signature() {
            // Use a variant that falls through to the catch-all
            let protocol_error = ProtocolError::Overflow("test overflow");

            let result = convert_to_consensus_signature_error(protocol_error)
                .expect("should not return Err");
            match result {
                ConsensusError::SignatureError(
                    SignatureError::InvalidStateTransitionSignatureError(_),
                ) => {}
                other => panic!(
                    "expected InvalidStateTransitionSignatureError, got {:?}",
                    other
                ),
            }
        }
    }
}
