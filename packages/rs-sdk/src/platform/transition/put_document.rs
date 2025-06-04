use super::broadcast::BroadcastStateTransition;
use super::waitable::Waitable;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters, INITIAL_REVISION};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::TokenPaymentInfo;

#[async_trait::async_trait]
/// A trait for putting a document to platform
pub trait PutDocument<S: Signer>: Waitable {
    /// Puts a document on platform
    /// setting settings to `None` sets default connection behavior
    #[allow(clippy::too_many_arguments)]
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Puts an identity on platform and waits for the confirmation proof
    #[allow(clippy::too_many_arguments)]
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error>;
}

#[async_trait::async_trait]
impl<S: Signer> PutDocument<S> for Document {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let new_identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                self.owner_id(),
                document_type.data_contract_id(),
                true,
                settings,
            )
            .await?;

        let settings = settings.unwrap_or_default();
        let transition = if self.revision().is_some()
            && self.revision().unwrap() != INITIAL_REVISION
        {
            BatchTransition::new_document_replacement_transition_from_document(
                self.clone(),
                document_type.as_ref(),
                &identity_public_key,
                new_identity_contract_nonce,
                settings.user_fee_increase.unwrap_or_default(),
                token_payment_info,
                signer,
                sdk.version(),
                settings.state_transition_creation_options,
            )
        } else {
            let (document, document_state_transition_entropy) = document_state_transition_entropy
                .map(|entropy| (self.clone(), entropy))
                .unwrap_or_else(|| {
                    let mut rng = StdRng::from_entropy();
                    let mut document = self.clone();
                    let entropy = rng.gen::<[u8; 32]>();
                    document.set_id(Document::generate_document_id_v0(
                        &document_type.data_contract_id(),
                        &document.owner_id(),
                        document_type.name(),
                        entropy.as_slice(),
                    ));
                    (document, entropy)
                });
            BatchTransition::new_document_creation_transition_from_document(
                document,
                document_type.as_ref(),
                document_state_transition_entropy,
                &identity_public_key,
                new_identity_contract_nonce,
                settings.user_fee_increase.unwrap_or_default(),
                token_payment_info,
                signer,
                sdk.version(),
                settings.state_transition_creation_options,
            )
        }?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result
        transition.broadcast(sdk, Some(settings)).await?;
        Ok(transition)
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        document_type: DocumentType,
        document_state_transition_entropy: Option<[u8; 32]>,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error> {
        let state_transition = self
            .put_to_platform(
                sdk,
                document_type,
                document_state_transition_entropy,
                identity_public_key,
                token_payment_info,
                signer,
                settings,
            )
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}
