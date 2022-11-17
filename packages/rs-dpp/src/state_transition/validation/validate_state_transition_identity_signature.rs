use std::collections::HashSet;

use lazy_static::lazy_static;

use crate::{
    consensus::{signature::SignatureError, ConsensusError},
    identity::KeyType,
    prelude::Identity,
    state_repository::StateRepositoryLike,
    state_transition::{
        fee::operations::{Operation, SignatureVerificationOperation},
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionIdentitySigned,
    },
    validation::ValidationResult,
    ProtocolError,
};

lazy_static! {
    static ref SUPPORTED_KEY_TYPES: HashSet<KeyType> = {
        let mut keys = HashSet::new();
        keys.insert(KeyType::ECDSA_SECP256K1);
        keys.insert(KeyType::BLS12_381);
        keys.insert(KeyType::ECDSA_HASH160);
        keys
    };
}

pub async fn validate_state_transition_identity_signature(
    state_repository: &impl StateRepositoryLike,
    state_transition: &mut impl StateTransitionIdentitySigned,
) -> Result<ValidationResult<()>, ProtocolError> {
    let mut validation_result = ValidationResult::<()>::default();

    // We use temporary execution context without dry run,
    // because despite the dryRun, we need to get the
    // identity to proceed with following logic
    let tmp_execution_context = StateTransitionExecutionContext::default();

    // Owner must exist
    let maybe_identity = state_repository
        .fetch_identity::<Identity>(state_transition.get_owner_id(), &tmp_execution_context)
        .await?;

    // Collect operations back from temporary context
    state_transition
        .get_execution_context()
        .add_operations(tmp_execution_context.get_operations());

    let identity = match maybe_identity {
        Some(identity) => identity,
        None => {
            validation_result.add_error(SignatureError::IdentityNotFoundError {
                identity_id: state_transition.get_owner_id().to_owned(),
            });
            return Ok(validation_result);
        }
    };

    let signature_public_key_id = state_transition.get_signature_public_key_id();
    let maybe_public_key = identity.get_public_key_by_id(signature_public_key_id);

    let public_key = match maybe_public_key {
        None => {
            validation_result.add_error(SignatureError::MissingPublicKeyError {
                public_key_id: signature_public_key_id,
            });
            return Ok(validation_result);
        }
        Some(pk) => pk,
    };

    if !SUPPORTED_KEY_TYPES.contains(&public_key.get_type()) {
        validation_result.add_error(SignatureError::InvalidIdentityPublicKeyTypeError {
            public_key_type: public_key.get_type(),
        });
        return Ok(validation_result);
    }

    let operation = SignatureVerificationOperation::new(public_key.key_type);
    state_transition
        .get_execution_context()
        .add_operation(Operation::SignatureVerification(operation));

    if state_transition.get_execution_context().is_dry_run() {
        return Ok(validation_result);
    }

    let signature_is_valid = state_transition.verify_signature(public_key);

    if let Err(err) = signature_is_valid {
        let consensus_error = convert_to_consensus_signature_error(err)?;
        validation_result.add_error(consensus_error);
        return Ok(validation_result);
    }

    Ok(validation_result)
}

fn convert_to_consensus_signature_error(
    error: ProtocolError,
) -> Result<ConsensusError, ProtocolError> {
    match error {
        ProtocolError::InvalidSignaturePublicKeySecurityLevelError {
            public_key_security_level,
            required_security_level,
        } => Ok(ConsensusError::SignatureError(
            SignatureError::InvalidSignaturePublicKeySecurityLevelError {
                public_key_security_level,
                required_key_security_level: required_security_level,
            },
        )),
        ProtocolError::PublicKeySecurityLevelNotMetError {
            public_key_security_level,
            required_security_level,
        } => Ok(ConsensusError::SignatureError(
            SignatureError::PublicKeySecurityLevelNotMetError {
                public_key_security_level,
                required_security_level,
            },
        )),
        ProtocolError::PublicKeyIsDisabledError { public_key } => Ok(
            ConsensusError::SignatureError(SignatureError::PublicKeyIsDisabledError {
                public_key_id: public_key.get_id(),
            }),
        ),
        ProtocolError::InvalidIdentityPublicKeyTypeError { public_key_type } => {
            Ok(ConsensusError::SignatureError(
                SignatureError::InvalidIdentityPublicKeyTypeError { public_key_type },
            ))
        }
        ProtocolError::Error(_) => Err(error),
        _ => Ok(ConsensusError::SignatureError(
            SignatureError::InvalidStateTransitionSignatureError,
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        document::DocumentsBatchTransition,
        identity::{KeyID, Purpose, SecurityLevel},
        prelude::{Identifier, Identity, IdentityPublicKey},
        state_repository::MockStateRepositoryLike,
        state_transition::{
            state_transition_execution_context::StateTransitionExecutionContext, StateTransition,
            StateTransitionConvert, StateTransitionLike, StateTransitionType,
        },
        tests::{
            fixtures::identity_fixture_raw_object,
            utils::{generate_random_identifier_struct, get_signature_error_from_result},
        },
    };
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExampleStateTransition {
        pub protocol_version: u32,
        pub signature: Vec<u8>,
        pub signature_public_key_id: KeyID,
        pub transition_type: StateTransitionType,
        pub owner_id: Identifier,

        pub return_error: Option<usize>,
        #[serde(skip)]
        pub execution_context: StateTransitionExecutionContext,
    }

    impl StateTransitionConvert for ExampleStateTransition {
        fn binary_property_paths() -> Vec<&'static str> {
            vec!["signature"]
        }
        fn identifiers_property_paths() -> Vec<&'static str> {
            vec![]
        }
        fn signature_property_paths() -> Vec<&'static str> {
            vec!["signature", "signaturePublicKeyId"]
        }
    }

    impl Into<StateTransition> for ExampleStateTransition {
        fn into(self) -> StateTransition {
            let st = DocumentsBatchTransition::default();
            StateTransition::DocumentsBatch(st)
        }
    }

    impl StateTransitionLike for ExampleStateTransition {
        fn get_protocol_version(&self) -> u32 {
            1
        }
        fn get_type(&self) -> StateTransitionType {
            StateTransitionType::DocumentsBatch
        }
        fn get_signature(&self) -> &Vec<u8> {
            &self.signature
        }
        fn set_signature(&mut self, signature: Vec<u8>) {
            self.signature = signature
        }
        fn get_execution_context(&self) -> &StateTransitionExecutionContext {
            &self.execution_context
        }

        fn get_execution_context_mut(&mut self) -> &mut StateTransitionExecutionContext {
            &mut self.execution_context
        }

        fn set_execution_context(&mut self, execution_context: StateTransitionExecutionContext) {
            self.execution_context = execution_context
        }
    }

    impl StateTransitionIdentitySigned for ExampleStateTransition {
        fn get_owner_id(&self) -> &Identifier {
            &self.owner_id
        }

        fn verify_signature(&self, public_key: &IdentityPublicKey) -> Result<(), ProtocolError> {
            if let Some(error_num) = self.return_error {
                match error_num {
                    0 => {
                        return Err(ProtocolError::PublicKeyMismatchError {
                            public_key: public_key.clone(),
                        })
                    }
                    1 => {
                        return Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError {
                            public_key_security_level: SecurityLevel::CRITICAL,
                            required_security_level: SecurityLevel::MASTER,
                        })
                    }
                    2 => {
                        return Err(ProtocolError::PublicKeySecurityLevelNotMetError {
                            public_key_security_level: SecurityLevel::CRITICAL,
                            required_security_level: SecurityLevel::HIGH,
                        })
                    }
                    3 => {
                        return Err(ProtocolError::PublicKeyIsDisabledError {
                            public_key: public_key.clone(),
                        })
                    }
                    4 => {
                        return Err(ProtocolError::WrongPublicKeyPurposeError {
                            public_key_purpose: Purpose::WITHDRAW,
                            key_purpose_requirement: Purpose::DECRYPTION,
                        })
                    }
                    _ => {}
                }
            }

            Ok(())
        }

        fn get_security_level_requirement(&self) -> SecurityLevel {
            SecurityLevel::MASTER
        }

        fn get_signature_public_key_id(&self) -> KeyID {
            self.signature_public_key_id
        }

        fn set_signature_public_key_id(&mut self, key_id: KeyID) {
            self.signature_public_key_id = key_id;
        }
    }

    fn get_mock_state_transition() -> ExampleStateTransition {
        ExampleStateTransition {
            protocol_version: 1,
            transition_type: StateTransitionType::DocumentsBatch,
            signature: Default::default(),
            signature_public_key_id: 1,
            owner_id: generate_random_identifier_struct(),
            return_error: None,
            execution_context: Default::default(),
        }
    }

    #[tokio::test]
    async fn should_pass_properly_signed_state_transition() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = owner_id.clone();
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));

        let result = validate_state_transition_identity_signature(
            &state_repository_mock,
            &mut state_transition,
        )
        .await
        .expect("the validation result should be returned");

        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn should_return_invalid_result_if_owner_id_does_not_exist() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = owner_id.clone();
        state_repository_mock
            .expect_fetch_identity::<Identity>()
            .returning(move |_, _| Ok(None));

        let result = validate_state_transition_identity_signature(
            &state_repository_mock,
            &mut state_transition,
        )
        .await
        .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        assert!(
            matches!(signature_error, SignatureError::IdentityNotFoundError { identity_id }  if {
             identity_id == identity.get_id()
            })
        );
    }

    #[tokio::test]
    async fn should_return_missing_public_key_error_if_identity_does_not_have_matching_public_key()
    {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = owner_id.clone();
        state_transition.signature_public_key_id = 12332;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));

        let result = validate_state_transition_identity_signature(
            &state_repository_mock,
            &mut state_transition,
        )
        .await
        .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        assert!(
            matches!(signature_error, SignatureError::MissingPublicKeyError { public_key_id }  if {
                 *public_key_id == 12332
            })
        );
    }

    #[tokio::test]
    async fn should_return_invalid_state_transition_signature_error_if_signature_is_invalid() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = owner_id.clone();
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(0);

        let result = validate_state_transition_identity_signature(
            &state_repository_mock,
            &mut state_transition,
        )
        .await
        .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        assert!(matches!(
            signature_error,
            SignatureError::InvalidStateTransitionSignatureError
        ));
    }

    // Consensus errors:

    #[tokio::test]
    async fn should_return_invalid_signature_public_key_security_level_error() {
        // 'should return InvalidSignaturePublicKeySecurityLevelConsensusError if InvalidSignaturePublicKeySecurityLevelError was thrown'
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = owner_id.clone();
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(1);

        let result = validate_state_transition_identity_signature(
            &state_repository_mock,
            &mut state_transition,
        )
        .await
        .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        assert!(matches!(
            signature_error,
            SignatureError::InvalidSignaturePublicKeySecurityLevelError { .. }
        ));
    }

    #[tokio::test]
    async fn should_not_verify_signature_on_dry_run() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = owner_id.clone();
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(1);
        state_transition.get_execution_context().enable_dry_run();

        let result = validate_state_transition_identity_signature(
            &state_repository_mock,
            &mut state_transition,
        )
        .await
        .expect("the validation result should be returned");

        assert!(result.is_valid())
    }

    #[tokio::test]
    async fn should_return_public_key_security_level_not_met() {
        // 'should return PubicKeySecurityLevelNotMetConsensusError if PubicKeySecurityLevelNotMetError was thrown'
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = owner_id.clone();
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(2);

        let result = validate_state_transition_identity_signature(
            &state_repository_mock,
            &mut state_transition,
        )
        .await
        .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        assert!(matches!(
            signature_error,
            SignatureError::PublicKeySecurityLevelNotMetError { .. }
        ));
    }

    #[tokio::test]
    async fn should_return_public_key_is_disabled_error() {
        // 'should return PublicKeyIsDisabledConsensusError if PublicKeyIsDisabledError was thrown'
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_raw_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = owner_id.clone();
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(3);

        let result = validate_state_transition_identity_signature(
            &state_repository_mock,
            &mut state_transition,
        )
        .await
        .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        assert!(matches!(
            signature_error,
            SignatureError::PublicKeyIsDisabledError { .. }
        ));
    }
}
