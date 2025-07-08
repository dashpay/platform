use crate::{Error, Sdk};

use super::broadcast::BroadcastStateTransition;
use super::waitable::Waitable;
use crate::platform::transition::put_settings::PutSettings;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::{Document, DocumentV0Getters};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::TokenPaymentInfo;

#[async_trait::async_trait]
/// A trait for updating the price of a document on Platform
pub trait UpdatePriceOfDocument<S: Signer>: Waitable {
    /// Updates the price of a document on platform
    /// Setting settings to `None` sets default connection behavior
    #[allow(clippy::too_many_arguments)]
    async fn update_price_of_document(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Updates the price of a document on platform and waits for the response
    #[allow(clippy::too_many_arguments)]
    async fn update_price_of_document_and_wait_for_response(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error>;
}

#[async_trait::async_trait]
impl<S: Signer> UpdatePriceOfDocument<S> for Document {
    async fn update_price_of_document(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
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

        let transition = BatchTransition::new_document_update_price_transition_from_document(
            self.clone(),
            document_type.as_ref(),
            price,
            &identity_public_key,
            new_identity_contract_nonce,
            settings.user_fee_increase.unwrap_or_default(),
            token_payment_info,
            signer,
            sdk.version(),
            settings.state_transition_creation_options,
        )?;

        // response is empty for a broadcast, result comes from the stream wait for state transition result
        transition.broadcast(sdk, Some(settings)).await?;
        Ok(transition)
    }

    async fn update_price_of_document_and_wait_for_response(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error> {
        let state_transition = self
            .update_price_of_document(
                price,
                sdk,
                document_type,
                identity_public_key,
                token_payment_info,
                signer,
                None,
            )
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}
