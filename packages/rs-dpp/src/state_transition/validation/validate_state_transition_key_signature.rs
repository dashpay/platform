use std::sync::Arc;

use anyhow::{bail, Context};
use async_trait::async_trait;
use dashcore::signer::verify_hash_signature;

use crate::{
    consensus::signature::SignatureError,
    identity::{
        state_transition::asset_lock_proof::{AssetLockProof, AssetLockPublicKeyHashFetcher},
        KeyType,
    },
    prelude::Identity,
    state_repository::StateRepositoryLike,
    state_transition::{
        fee::operations::{Operation, SignatureVerificationOperation},
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransition, StateTransitionConvert, StateTransitionLike,
    },
    validation::{AsyncDataValidator, SimpleValidationResult, ValidationResult},
    ProtocolError,
};

pub struct StateTransitionKeySignatureValidator<SR> {
    state_repository: Arc<SR>,
    asset_lock_public_key_hash_fetcher: AssetLockPublicKeyHashFetcher<SR>,
}

#[async_trait]
impl<SR> AsyncDataValidator for StateTransitionKeySignatureValidator<SR>
where
    SR: StateRepositoryLike,
{
    type Item = StateTransition;
    async fn validate(&self, data: &Self::Item) -> Result<SimpleValidationResult, ProtocolError> {
        validate_state_transition_key_signature(
            self.state_repository.as_ref(),
            &self.asset_lock_public_key_hash_fetcher,
            data,
        )
        .await
    }
}

impl<SR> StateTransitionKeySignatureValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(
        state_repository: Arc<SR>,
        asset_lock_public_key_hash_fetcher: AssetLockPublicKeyHashFetcher<SR>,
    ) -> Self {
        Self {
            state_repository,
            asset_lock_public_key_hash_fetcher,
        }
    }
}

pub async fn validate_state_transition_key_signature<SR: StateRepositoryLike>(
    state_repository: &impl StateRepositoryLike,
    asset_lock_public_key_hash_fetcher: &AssetLockPublicKeyHashFetcher<SR>,
    state_transition: &StateTransition,
) -> Result<ValidationResult<()>, ProtocolError> {
    let mut result = SimpleValidationResult::default();

    let execution_context = state_transition.get_execution_context();
    // Validate target identity existence for top up
    if let StateTransition::IdentityTopUp(ref transition) = state_transition {
        let target_identity_id = transition.get_identity_id();

        // We use temporary execution context without dry run,
        // because despite the dry run, we need to get the
        // identity to proceed with following logic
        let tmp_execution_context = StateTransitionExecutionContext::default();

        // Target identity must exist
        let identity = state_repository
            .fetch_identity::<Identity>(target_identity_id, &tmp_execution_context)
            .await?;

        // Collect operations back from temporary context
        execution_context.add_operations(tmp_execution_context.get_operations());

        if identity.is_none() {
            result.add_error(SignatureError::IdentityNotFoundError {
                identity_id: target_identity_id.clone(),
            });
            return Ok(result);
        }
    }

    let state_transition_hash = state_transition.hash(true)?;
    let asset_lock_proof = get_asset_lock_proof(state_transition)?;
    let public_key_hash = asset_lock_public_key_hash_fetcher
        .fetch_public_key_hash(asset_lock_proof.to_owned(), execution_context)
        .await
        .with_context(|| format!("public key hash fetching failed for {:?}", asset_lock_proof))?;

    let operation = SignatureVerificationOperation::new(KeyType::ECDSA_SECP256K1);
    state_transition
        .get_execution_context()
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
                identity_topup_transition::IdentityTopUpTransition,
            },
            KeyType,
        },
        prelude::Identity,
        state_repository::MockStateRepositoryLike,
        state_transition::{StateTransition, StateTransitionLike},
        tests::{
            fixtures::{
                identity_create_transition_fixture_json, identity_topup_transition_fixture_json,
            },
            utils::get_signature_error_from_result,
        },
        NativeBlsModule,
    };

    use super::validate_state_transition_key_signature;

    struct TestData {
        state_repository: MockStateRepositoryLike,
        asset_lock_public_key_hash_fetcher: AssetLockPublicKeyHashFetcher<MockStateRepositoryLike>,
        bls: NativeBlsModule,
    }
    fn setup_test() -> TestData {
        let state_repository_mock = MockStateRepositoryLike::new();
        let shared_state_repository = Arc::new(state_repository_mock);
        let asset_lock_transaction_output_fetcher =
            AssetLockTransactionOutputFetcher::new(shared_state_repository.clone());

        let asset_lock_public_key_hash_fetcher = AssetLockPublicKeyHashFetcher::new(
            shared_state_repository,
            asset_lock_transaction_output_fetcher,
        );

        TestData {
            state_repository: MockStateRepositoryLike::new(),
            asset_lock_public_key_hash_fetcher,
            bls: NativeBlsModule::default(),
        }
    }

    #[tokio::test]
    async fn should_return_error_when_unsupported_state_transition_type() {
        let TestData {
            state_repository,
            asset_lock_public_key_hash_fetcher,
            bls,
        } = setup_test();
        let state_transition: StateTransition = DocumentsBatchTransition::default().into();

        let result = validate_state_transition_key_signature(
            &state_repository,
            &asset_lock_public_key_hash_fetcher,
            &state_transition,
        )
        .await
        .expect_err("error is expected");
        assert_eq!("key signature validation isn't supported for DocumentsBatch. Asset lock proof is required", result.to_string());
    }

    #[tokio::test]
    async fn should_return_valid_result_if_signature_is_valid() {
        let TestData {
            state_repository,
            asset_lock_public_key_hash_fetcher,
            bls,
        } = setup_test();
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
                &bls,
            )
            .expect("state transition should be signed");

        let result = validate_state_transition_key_signature(
            &state_repository,
            &asset_lock_public_key_hash_fetcher,
            &state_transition,
        )
        .await
        .expect("the validation result should be returned");

        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn should_return_invalid_signature_error_if_signature_is_invalid() {
        let TestData {
            state_repository,
            asset_lock_public_key_hash_fetcher,
            bls,
        } = setup_test();
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
                &bls,
            )
            .expect("state transition should be signed");

        // setting an invalid signature
        state_transition.set_signature(vec![0u8; 65]);

        let result = validate_state_transition_key_signature(
            &state_repository,
            &asset_lock_public_key_hash_fetcher,
            &state_transition,
        )
        .await
        .expect("the validation result should be returned");

        let signature_error = get_signature_error_from_result(&result, 0);
        assert!(matches!(
            signature_error,
            SignatureError::InvalidStateTransitionSignatureError
        ));
    }

    #[tokio::test]
    async fn should_return_error_if_identity_id_not_exist_on_top_up_transition() {
        let TestData {
            mut state_repository,
            asset_lock_public_key_hash_fetcher,
            bls,
        } = setup_test();
        let private_key_hex = "af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837";
        let secret_key = SecretKey::from_slice(&hex::decode(private_key_hex).unwrap())
            .expect("secret key should be created");

        let private_key = PrivateKey::new(secret_key, Network::Testnet);
        let state_transition: StateTransition =
            IdentityTopUpTransition::new(identity_topup_transition_fixture_json(Some(private_key)))
                .unwrap()
                .into();

        state_repository
            .expect_fetch_identity::<Identity>()
            .return_once(|_, _| Ok(None));

        let result = validate_state_transition_key_signature(
            &state_repository,
            &asset_lock_public_key_hash_fetcher,
            &state_transition,
        )
        .await
        .expect("the validation result should be returned");

        let signature_error = get_signature_error_from_result(&result, 0);
        assert!(matches!(
            signature_error,
            SignatureError::IdentityNotFoundError { .. }
        ));
    }
}
