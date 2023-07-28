use std::sync::Arc;
use std::{collections::HashSet, convert::TryInto};

use anyhow::Context;
use lazy_static::lazy_static;

use crate::consensus::signature::SignatureError;
use crate::consensus::signature::{
    IdentityNotFoundError, InvalidIdentityPublicKeyTypeError, InvalidStateTransitionSignatureError,
    MissingPublicKeyError, PublicKeyIsDisabledError, PublicKeySecurityLevelNotMetError,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::{
    consensus::ConsensusError,
    identity::KeyType,
    state_repository::StateRepositoryLike,
    state_transition::{
        fee::operations::{Operation, SignatureVerificationOperation},
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionIdentitySignedV0,
    },
    BlsModule, ProtocolError,
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
    state_repository: Arc<impl StateRepositoryLike>,
    state_transition: &mut impl StateTransitionIdentitySignedV0,
    bls: &impl BlsModule,
    execution_context: &StateTransitionExecutionContext,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut validation_result = SimpleConsensusValidationResult::default();

    // We use temporary execution context without dry run,
    // because despite the dryRun, we need to get the
    // identity to proceed with following logic
    let tmp_execution_context = StateTransitionExecutionContext::default();

    // Owner must exist
    let maybe_identity = state_repository
        .fetch_identity(
            state_transition.get_owner_id(),
            Some(&tmp_execution_context),
        )
        .await?
        .map(TryInto::try_into)
        .transpose()
        .map_err(Into::into)?;

    // Collect operations back from temporary context
    execution_context.add_operations(tmp_execution_context.get_operations());

    let identity = match maybe_identity {
        Some(identity) => identity,
        None => {
            validation_result.add_error(SignatureError::IdentityNotFoundError(
                IdentityNotFoundError::new(state_transition.get_owner_id().to_owned()),
            ));
            return Ok(validation_result);
        }
    };

    let signature_public_key_id = state_transition
        .get_signature_public_key_id()
        //? what error should be returned here? MissingPublicKeyError only applies to the Identity
        .context("State transition doesn't have signature public key")?;
    let maybe_public_key = identity.get_public_key_by_id(signature_public_key_id);

    let public_key = match maybe_public_key {
        None => {
            validation_result.add_error(SignatureError::MissingPublicKeyError(
                MissingPublicKeyError::new(signature_public_key_id),
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

    let operation = SignatureVerificationOperation::new(public_key.key_type);
    execution_context.add_operation(Operation::SignatureVerification(operation));

    if execution_context.is_dry_run() {
        return Ok(validation_result);
    }

    let signature_is_valid = state_transition.verify_signature(public_key, bls);

    if let Err(err) = signature_is_valid {
        let consensus_error = convert_to_consensus_signature_error(err)?;
        validation_result.add_error(consensus_error);
        return Ok(validation_result);
    }

    Ok(validation_result)
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
                err.public_key().id,
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::consensus::signature::InvalidSignaturePublicKeySecurityLevelError;
    use crate::serialization::PlatformDeserializable;
    use crate::serialization::PlatformSerializable;
    use crate::serialization::Signable;
    use crate::state_transition::errors::{PublicKeyMismatchError, WrongPublicKeyPurposeError};
    use crate::version::FeatureVersion;
    use crate::{
        document::DocumentsBatchTransition,
        identity::{KeyID, Purpose, SecurityLevel},
        prelude::{Identifier, Identity, IdentityPublicKey},
        state_repository::MockStateRepositoryLike,
        state_transition::{
            state_transition_execution_context::StateTransitionExecutionContext, StateTransition,
            StateTransitionFieldTypes, StateTransitionLike, StateTransitionType,
        },
        tests::{
            fixtures::identity_fixture_raw_object,
            utils::{generate_random_identifier_struct, get_signature_error_from_result},
        },
        NativeBlsModule,
    };
    use bincode::{config, Decode, Encode};
    use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
    use platform_value::BinaryData;
    use serde::{Deserialize, Serialize};
    use std::vec;

    #[derive(
        Debug,
        Clone,
        Encode,
        Decode,
        Serialize,
        Deserialize,
        PlatformDeserialize,
        PlatformSerialize,
        PlatformSignable,
    )]

    #[serde(rename_all = "camelCase")]
    struct ExampleStateTransition {
        pub protocol_version: u32,
        pub transition_type: StateTransitionType,
        pub owner_id: Identifier,

        pub return_error: Option<usize>,
        #[platform_signable(exclude_from_sig_hash)]
        pub signature_public_key_id: KeyID,
        #[platform_signable(exclude_from_sig_hash)]
        pub signature: BinaryData,
    }

    impl StateTransitionFieldTypes for ExampleStateTransition {
        fn signature_property_paths() -> Vec<&'static str> {
            vec!["signature", "signaturePublicKeyId"]
        }
        fn identifiers_property_paths() -> Vec<&'static str> {
            vec![]
        }
        fn binary_property_paths() -> Vec<&'static str> {
            vec!["signature"]
        }
    }

    impl From<ExampleStateTransition> for StateTransition {
        fn from(_val: ExampleStateTransition) -> Self {
            let st = DocumentsBatchTransition::default();
            StateTransition::DocumentsBatch(st)
        }
    }

    impl StateTransitionLike for ExampleStateTransition {
        fn state_transition_protocol_version(&self) -> FeatureVersion {
            1
        }
        fn state_transition_type(&self) -> StateTransitionType {
            StateTransitionType::DocumentsBatch
        }
        fn signature(&self) -> &BinaryData {
            &self.signature
        }
        fn set_signature(&mut self, signature: BinaryData) {
            self.signature = signature
        }

        fn set_signature_bytes(&mut self, signature: Vec<u8>) {
            self.signature = BinaryData::new(signature)
        }

        fn modified_data_ids(&self) -> Vec<Identifier> {
            vec![]
        }
    }

    impl StateTransitionIdentitySignedV0 for ExampleStateTransition {
        fn get_owner_id(&self) -> &Identifier {
            &self.owner_id
        }

        fn verify_signature(
            &self,
            public_key: &IdentityPublicKey,
            _bls: &impl BlsModule,
        ) -> Result<(), ProtocolError> {
            if let Some(error_num) = self.return_error {
                match error_num {
                    0 => {
                        return Err(ProtocolError::PublicKeyMismatchError(
                            PublicKeyMismatchError::new(public_key.clone()),
                        ))
                    }
                    1 => {
                        return Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                            InvalidSignaturePublicKeySecurityLevelError::new(
                                SecurityLevel::CRITICAL,
                                vec![SecurityLevel::MASTER],
                            ),
                        ))
                    }
                    2 => {
                        return Err(ProtocolError::PublicKeySecurityLevelNotMetError(
                            crate::state_transition::errors::PublicKeySecurityLevelNotMetError::new(
                                SecurityLevel::CRITICAL,
                                SecurityLevel::HIGH,
                            ),
                        ))
                    }
                    3 => {
                        return Err(ProtocolError::PublicKeyIsDisabledError(
                            crate::data_contract::state_transition::errors::PublicKeyIsDisabledError::new(public_key.clone()),
                        ))
                    }
                    4 => {
                        return Err(ProtocolError::WrongPublicKeyPurposeError(
                            WrongPublicKeyPurposeError::new(Purpose::WITHDRAW, Purpose::DECRYPTION),
                        ))
                    }
                    _ => {}
                }
            }

            Ok(())
        }

        fn get_security_level_requirement(&self) -> Vec<SecurityLevel> {
            vec![SecurityLevel::MASTER]
        }

        fn get_signature_public_key_id(&self) -> Option<KeyID> {
            Some(self.signature_public_key_id)
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
        }
    }

    #[tokio::test]
    async fn should_pass_properly_signed_state_transition() {
        let bls = NativeBlsModule::default();
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = *owner_id;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        let execution_context = StateTransitionExecutionContext::default();

        let result = validate_state_transition_identity_signature(
            Arc::new(state_repository_mock),
            &mut state_transition,
            &bls,
            &execution_context,
        )
        .await
        .expect("the validation result should be returned");

        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn should_return_invalid_result_if_owner_id_does_not_exist() {
        let bls = NativeBlsModule::default();
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = *owner_id;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(None));
        let execution_context = StateTransitionExecutionContext::default();

        let result = validate_state_transition_identity_signature(
            Arc::new(state_repository_mock),
            &mut state_transition,
            &bls,
            &execution_context,
        )
        .await
        .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        match signature_error {
            SignatureError::IdentityNotFoundError(err) => {
                assert_eq!(err.identity_id(), identity.get_id().clone())
            }
            error => panic!("expected IdentityNotFoundError, got {}", error),
        }
    }

    #[tokio::test]
    async fn should_return_missing_public_key_error_if_identity_does_not_have_matching_public_key()
    {
        let bls = NativeBlsModule::default();
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = *owner_id;
        state_transition.signature_public_key_id = 12332;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        let execution_context = StateTransitionExecutionContext::default();

        let result = validate_state_transition_identity_signature(
            Arc::new(state_repository_mock),
            &mut state_transition,
            &bls,
            &execution_context,
        )
        .await
        .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        match signature_error {
            SignatureError::MissingPublicKeyError(err) => {
                assert_eq!(err.public_key_id(), 12332)
            }
            error => panic!("expected MissingPublicKeyError, got {}", error),
        }
    }

    #[tokio::test]
    async fn should_return_invalid_state_transition_signature_error_if_signature_is_invalid() {
        let bls = NativeBlsModule::default();
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = *owner_id;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(0);
        let execution_context = StateTransitionExecutionContext::default();

        let result = validate_state_transition_identity_signature(
            Arc::new(state_repository_mock),
            &mut state_transition,
            &bls,
            &execution_context,
        )
        .await
        .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        assert!(matches!(
            signature_error,
            SignatureError::InvalidStateTransitionSignatureError(
                InvalidStateTransitionSignatureError { .. }
            )
        ));
    }

    // Consensus errors:

    #[tokio::test]
    async fn should_return_invalid_signature_public_key_security_level_error() {
        let bls = NativeBlsModule::default();
        // 'should return InvalidSignaturePublicKeySecurityLevelConsensusError if InvalidSignaturePublicKeySecurityLevelError was thrown'
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = *owner_id;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(1);
        let execution_context = StateTransitionExecutionContext::default();

        let result = validate_state_transition_identity_signature(
            Arc::new(state_repository_mock),
            &mut state_transition,
            &bls,
            &execution_context,
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
        let bls = NativeBlsModule::default();
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = *owner_id;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(1);
        let execution_context = StateTransitionExecutionContext::default().with_dry_run();

        let result = validate_state_transition_identity_signature(
            Arc::new(state_repository_mock),
            &mut state_transition,
            &bls,
            &execution_context,
        )
        .await
        .expect("the validation result should be returned");

        assert!(result.is_valid())
    }

    #[tokio::test]
    async fn should_return_public_key_security_level_not_met() {
        let bls = NativeBlsModule::default();
        // 'should return PubicKeySecurityLevelNotMetConsensusError if PubicKeySecurityLevelNotMetError was thrown'
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = *owner_id;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(2);

        let execution_context = StateTransitionExecutionContext::default();

        let result = validate_state_transition_identity_signature(
            Arc::new(state_repository_mock),
            &mut state_transition,
            &bls,
            &execution_context,
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
        let bls = NativeBlsModule::default();
        // 'should return PublicKeyIsDisabledConsensusError if PublicKeyIsDisabledError was thrown'
        let mut state_repository_mock = MockStateRepositoryLike::new();
        let raw_identity = identity_fixture_raw_object();
        let identity = Identity::from_object(raw_identity).unwrap();
        let owner_id = identity.get_id();
        let mut state_transition = get_mock_state_transition();

        state_transition.owner_id = *owner_id;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));
        state_transition.return_error = Some(3);

        let execution_context = StateTransitionExecutionContext::default();

        let result = validate_state_transition_identity_signature(
            Arc::new(state_repository_mock),
            &mut state_transition,
            &bls,
            &execution_context,
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
