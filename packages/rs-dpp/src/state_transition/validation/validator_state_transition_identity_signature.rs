use anyhow::anyhow;

use crate::{
    consensus::signature::SignatureError,
    identity::{validation::validate_identity_existence, KeyType},
    state_repository::StateRepositoryLike,
    state_transition::StateTransitionIdentitySigned,
    validation::ValidationResult,
    ProtocolError,
};

pub async fn validate_state_transition_identity_signature(
    state_repository: &impl StateRepositoryLike,
    state_transition: &impl StateTransitionIdentitySigned,
) -> Result<ValidationResult<()>, ProtocolError> {
    let mut validation_result = ValidationResult::<()>::default();

    // Owner must exist
    let result =
        validate_identity_existence(state_repository, state_transition.get_owner_id()).await?;
    if !result.is_valid() {
        return Ok(result.into_result_without_data());
    }

    let identity = result
        .data()
        .ok_or_else(|| anyhow!("the result doesn't contain any Identity"))?;
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

    if public_key.get_type() != KeyType::ECDSA_SECP256K1
        && public_key.get_type() != KeyType::ECDSA_HASH160
    {
        validation_result.add_error(SignatureError::InvalidIdentityPublicKeyTypeError {
            key_type: public_key.get_type(),
        });
        return Ok(validation_result);
    }

    let signature_is_valid = state_transition.verify_signature(public_key);
    if signature_is_valid.is_err() {
        validation_result.add_error(SignatureError::InvalidStateTransitionSignatureError);
        return Ok(validation_result);
    }

    Ok(validation_result)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        document::DocumentsBatchTransition,
        identity::{KeyID, SecurityLevel},
        prelude::{Identifier, Identity, IdentityPublicKey},
        state_repository::MockStateRepositoryLike,
        state_transition::{
            StateTransition, StateTransitionConvert, StateTransitionLike, StateTransitionType,
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
        pub is_signature_valid: bool,
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
        fn calculate_fee(&self) -> Result<u64, crate::ProtocolError> {
            unimplemented!()
        }
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
    }

    impl StateTransitionIdentitySigned for ExampleStateTransition {
        fn get_owner_id(&self) -> &Identifier {
            &self.owner_id
        }

        fn verify_signature(&self, public_key: &IdentityPublicKey) -> Result<(), ProtocolError> {
            if self.is_signature_valid {
                Ok(())
            } else {
                Err(ProtocolError::PublicKeyMismatchError {
                    public_key: public_key.clone(),
                })
            }
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
            is_signature_valid: true,
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
            .returning(move |_| Ok(Some(identity.clone())));

        let result =
            validate_state_transition_identity_signature(&state_repository_mock, &state_transition)
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
            .returning(move |_| Ok(None));

        let result =
            validate_state_transition_identity_signature(&state_repository_mock, &state_transition)
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
            .returning(move |_| Ok(Some(identity.clone())));

        let result =
            validate_state_transition_identity_signature(&state_repository_mock, &state_transition)
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
            .returning(move |_| Ok(Some(identity.clone())));
        state_transition.is_signature_valid = false;

        let result =
            validate_state_transition_identity_signature(&state_repository_mock, &state_transition)
                .await
                .expect("the validation result should be returned");
        let signature_error = get_signature_error_from_result(&result, 0);

        assert!(matches!(
            signature_error,
            SignatureError::InvalidStateTransitionSignatureError
        ));
    }
}
