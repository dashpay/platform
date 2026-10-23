use super::broadcast::{
    broadcast_with_observation, wait_after_broadcast, BroadcastStateTransition,
};
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::platform::transition::put_settings::PutSettings;
use crate::{Error, Sdk};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::TokenPaymentInfo;

#[async_trait::async_trait]
/// A trait for purchasing a document on Platform
pub trait PurchaseDocument<S: Signer<IdentityPublicKey>>: Waitable {
    /// Tries to purchase a document on platform
    /// Setting settings to `None` sets default connection behavior
    #[allow(clippy::too_many_arguments)]
    async fn purchase_document(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        purchaser_id: Identifier,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Tries to purchase a document on platform and waits for the response
    #[allow(clippy::too_many_arguments)]
    async fn purchase_document_and_wait_for_response(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        purchaser_id: Identifier,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error>;
}

#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>> PurchaseDocument<S> for Document {
    async fn purchase_document(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        purchaser_id: Identifier,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let transition = prepare_purchase_state_transition(
            self,
            price,
            sdk,
            document_type,
            purchaser_id,
            identity_public_key,
            token_payment_info,
            signer,
            settings,
        )
        .await?;
        transition.broadcast(sdk, settings).await?;
        // response is empty for a broadcast, result comes from the stream wait for state transition result
        Ok(transition)
    }

    async fn purchase_document_and_wait_for_response(
        &self,
        price: Credits,
        sdk: &Sdk,
        document_type: DocumentType,
        purchaser_id: Identifier,
        identity_public_key: IdentityPublicKey,
        token_payment_info: Option<TokenPaymentInfo>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error> {
        let state_transition = prepare_purchase_state_transition(
            self,
            price,
            sdk,
            document_type,
            purchaser_id,
            identity_public_key,
            token_payment_info,
            signer,
            settings,
        )
        .await?;

        let broadcast = broadcast_with_observation(&state_transition, sdk, settings).await;
        wait_after_broadcast(broadcast, async move {
            Self::wait_for_response(sdk, state_transition, settings).await
        })
        .await
    }
}

/// Construct and sign once; dispatch is chosen by the calling API.
#[allow(clippy::too_many_arguments)]
async fn prepare_purchase_state_transition<S: Signer<IdentityPublicKey>>(
    document: &Document,
    price: Credits,
    sdk: &Sdk,
    document_type: DocumentType,
    purchaser_id: Identifier,
    identity_public_key: IdentityPublicKey,
    token_payment_info: Option<TokenPaymentInfo>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    // A local failure after the nonce is reserved would leave the cached nonce ahead of
    // Platform's, so what can be refused without it is refused first.
    if let Some(creation_options) =
        settings.and_then(|settings| settings.state_transition_creation_options)
    {
        creation_options.validate_base_carries_action_fee_agreement(sdk.version())?;
    }

    let new_identity_contract_nonce = sdk
        .get_identity_contract_nonce(
            purchaser_id,
            document_type.data_contract_id(),
            true,
            settings,
        )
        .await?;

    let settings = settings.unwrap_or_default();

    let transition = BatchTransition::new_document_purchase_transition_from_document(
        document.clone(),
        document_type.as_ref(),
        purchaser_id,
        price,
        &identity_public_key,
        new_identity_contract_nonce,
        settings.user_fee_increase.unwrap_or_default(),
        token_payment_info,
        signer,
        sdk.version(),
        settings.state_transition_creation_options,
    )
    .await?;
    ensure_valid_state_transition_structure(&transition, sdk.version())?;

    Ok(transition)
}

#[cfg(test)]
mod submission_tests {
    use super::*;
    use crate::sdk::put_test_support::PutFixture;
    use dapi_grpc::tonic::Status;

    #[tokio::test]
    async fn should_wait_for_original_signed_transition_after_ambiguous_submission() {
        for (status, expected_waits) in [
            (Status::unavailable("acknowledgement lost"), 1),
            (Status::already_exists("tx already exists in cache"), 1),
            (
                Status::resource_exhausted("broadcasting state transition is timed out"),
                1,
            ),
            (Status::resource_exhausted("mempool is full"), 0),
        ] {
            let mut fixture = PutFixture::new().await;
            let prepared = prepare_purchase_state_transition(
                &fixture.document,
                10,
                &fixture.sdk,
                fixture.document_type.clone(),
                Identifier::new([9; 32]),
                fixture.key.clone(),
                None,
                &fixture.signer,
                PutFixture::settings(),
            )
            .await
            .unwrap();
            let requests = fixture.expect_submission(&prepared, status).await;
            let result = fixture
                .document
                .purchase_document_and_wait_for_response(
                    10,
                    &fixture.sdk,
                    fixture.document_type.clone(),
                    Identifier::new([9; 32]),
                    fixture.key.clone(),
                    None,
                    &fixture.signer,
                    PutFixture::settings(),
                )
                .await
                .map(|_| ());
            fixture
                .assert_calls(&requests, expected_waits, result)
                .await;
        }
    }

    #[tokio::test]
    async fn should_preserve_plain_broadcast_error_without_waiting() {
        let mut fixture = PutFixture::new().await;
        let prepared = prepare_purchase_state_transition(
            &fixture.document,
            10,
            &fixture.sdk,
            fixture.document_type.clone(),
            Identifier::new([9; 32]),
            fixture.key.clone(),
            None,
            &fixture.signer,
            PutFixture::settings(),
        )
        .await
        .unwrap();
        let requests = fixture
            .expect_submission(&prepared, Status::unavailable("acknowledgement lost"))
            .await;
        let result = fixture
            .document
            .purchase_document(
                10,
                &fixture.sdk,
                fixture.document_type.clone(),
                Identifier::new([9; 32]),
                fixture.key.clone(),
                None,
                &fixture.signer,
                PutFixture::settings(),
            )
            .await
            .map(|_| ());
        fixture.assert_calls(&requests, 0, result).await;
    }
}
