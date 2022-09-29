use anyhow::{bail, Context};
use dashcore::signer::verify_hash_signature;

use crate::{
    consensus::signature::SignatureError,
    identity::{
        state_transition::asset_lock_proof::{AssetLockProof, AssetLockPublicKeyHashFetcher},
        KeyType,
    },
    state_repository::StateRepositoryLike,
    state_transition::{
        fee::operations::{Operation, SignatureVerificationOperation},
        StateTransition, StateTransitionConvert, StateTransitionLike,
    },
    validation::{SimpleValidationResult, ValidationResult},
    ProtocolError,
};

pub async fn validate_state_transition_key_signature<SR: StateRepositoryLike>(
    asset_lock_public_key_hash_fetcher: AssetLockPublicKeyHashFetcher<SR>,
    state_transition: &mut StateTransition,
) -> Result<ValidationResult<()>, ProtocolError> {
    let mut result = SimpleValidationResult::default();

    let execution_context = state_transition.get_execution_context();
    let state_transition_hash = state_transition.hash(true)?;

    let asset_lock_proof = get_asset_lock_proof(state_transition)?;
    let public_key_hash = asset_lock_public_key_hash_fetcher
        .fetch_public_key_hash(asset_lock_proof.to_owned(), execution_context)
        .await
        .with_context(|| format!("public key hash fetching failed for {:?}", asset_lock_proof))?;

    let operation = SignatureVerificationOperation::new(KeyType::ECDSA_SECP256K1);
    state_transition
        .get_execution_context_mut()
        .add_operation(Operation::SignatureVerification(operation));

    let verification_result = verify_hash_signature(
        &state_transition_hash,
        state_transition.get_signature(),
        &public_key_hash,
    );
    if verification_result.is_err() {
        result.add_error(SignatureError::InvalidStateTransitionSignatureError)
    }

    Ok(result)
}

fn get_asset_lock_proof(
    state_transition: &StateTransition,
) -> Result<&AssetLockProof, anyhow::Error> {
    match state_transition {
        StateTransition::IdentityCreate(st) => Ok(st.get_asset_lock_proof()),
        StateTransition::IdentityTopUp(st) => Ok(st.get_asset_lock_proof()),
        _ => {
            bail!(
                "key signature validation isn't supported for {}. Asset lock proof is required",
                state_transition.get_type()
            )
        }
    }
}

#[cfg(test)]
mod test {
    use dashcore::{secp256k1::SecretKey, Network, PrivateKey};
    use std::sync::Arc;

    use crate::{
        consensus::signature::SignatureError,
        document::DocumentsBatchTransition,
        identity::{
            state_transition::{
                asset_lock_proof::{
                    AssetLockPublicKeyHashFetcher, AssetLockTransactionOutputFetcher,
                },
                identity_create_transition::IdentityCreateTransition,
            },
            KeyType,
        },
        state_repository::MockStateRepositoryLike,
        state_transition::{StateTransition, StateTransitionLike},
        tests::{
            fixtures::identity_create_transition_fixture_json,
            utils::get_signature_error_from_result,
        },
    };

    use super::validate_state_transition_key_signature;

    fn get_asset_lock_public_key_hash_fetcher(
    ) -> AssetLockPublicKeyHashFetcher<MockStateRepositoryLike> {
        let state_repository_mock = MockStateRepositoryLike::new();
        let shared_state_repository = Arc::new(state_repository_mock);
        let asset_lock_transaction_output_fetcher =
            AssetLockTransactionOutputFetcher::new(shared_state_repository.clone());

        AssetLockPublicKeyHashFetcher::new(
            shared_state_repository,
            asset_lock_transaction_output_fetcher,
        )
    }

    #[tokio::test]
    async fn should_return_error_when_unsupported_state_transition_type() {
        let asset_lock_public_key_hash_fetcher = get_asset_lock_public_key_hash_fetcher();
        let mut state_transition: StateTransition = DocumentsBatchTransition::default().into();

        let result = validate_state_transition_key_signature(
            asset_lock_public_key_hash_fetcher,
            &mut state_transition,
        )
        .await
        .expect_err("error is expected");
        assert_eq!("key signature validation isn't supported for DocumentsBatch. Asset lock proof is required", result.to_string());
    }

    #[tokio::test]
    async fn should_return_valid_result_if_signature_is_valid() {
        let asset_lock_public_key_hash_fetcher = get_asset_lock_public_key_hash_fetcher();
        let private_key_hex = "af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837";
        let secret_key = SecretKey::from_slice(&hex::decode(private_key_hex).unwrap())
            .expect("secret key should be created");

        let private_key = PrivateKey::new(secret_key, Network::Testnet);
        let mut state_transition: StateTransition = IdentityCreateTransition::new(
            identity_create_transition_fixture_json(Some(private_key)),
        )
        .unwrap()
        .into();

        state_transition
            .sign_by_private_key(
                &hex::decode(private_key_hex).unwrap(),
                KeyType::ECDSA_SECP256K1,
            )
            .expect("state transition should be signed");

        let result = validate_state_transition_key_signature(
            asset_lock_public_key_hash_fetcher,
            &mut state_transition,
        )
        .await
        .expect("the validation result should be returned");

        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn should_return_invalid_signature_error_if_signature_is_invalid() {
        let asset_lock_public_key_hash_fetcher = get_asset_lock_public_key_hash_fetcher();
        let private_key_hex = "af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837";
        let secret_key = SecretKey::from_slice(&hex::decode(private_key_hex).unwrap())
            .expect("secret key should be created");

        let private_key = PrivateKey::new(secret_key, Network::Testnet);
        let mut state_transition: StateTransition = IdentityCreateTransition::new(
            identity_create_transition_fixture_json(Some(private_key)),
        )
        .unwrap()
        .into();

        state_transition
            .sign_by_private_key(
                &hex::decode(private_key_hex).unwrap(),
                KeyType::ECDSA_SECP256K1,
            )
            .expect("state transition should be signed");

        // setting an invalid signature
        state_transition.set_signature(vec![0u8; 65]);

        let result = validate_state_transition_key_signature(
            asset_lock_public_key_hash_fetcher,
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
}
