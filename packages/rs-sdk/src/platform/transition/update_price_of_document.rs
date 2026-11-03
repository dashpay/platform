use crate::{Error, Sdk};

use super::broadcast::BroadcastStateTransition;
use super::validation::ensure_valid_state_transition_structure;
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
pub trait UpdatePriceOfDocument<S: Signer<IdentityPublicKey>>: Waitable {
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
impl<S: Signer<IdentityPublicKey>> UpdatePriceOfDocument<S> for Document {
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
        // A local failure after the nonce is reserved would leave the cached nonce ahead of
        // Platform's, so what can be refused without it is refused first.
        if let Some(creation_options) =
            settings.and_then(|settings| settings.state_transition_creation_options)
        {
            creation_options.validate_base_carries_action_fee_agreement(sdk.version())?;
        }

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
        )
        .await?;
        ensure_valid_state_transition_structure(&transition, sdk.version())?;

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
                settings,
            )
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SdkBuilder;
    use dpp::address_funds::AddressWitness;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::action_fees::agreement::v0::DocumentActionFeeAgreementV0;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::DataContract;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use dpp::ProtocolError;

    #[derive(Debug)]
    struct UnusedSigner;

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for UnusedSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Err(ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    /// Before protocol version 14 an agreement is refused before any nonce work, which only
    /// happens if the settings reached the transition.
    #[tokio::test]
    async fn should_build_the_transition_with_the_settings_it_waits_with() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let sdk = SdkBuilder::new_mock()
            .with_version(platform_version)
            .build()
            .expect("mock sdk");
        let contract: DataContract =
            load_system_data_contract(SystemDataContract::DPNS, platform_version)
                .expect("the DPNS contract");
        let document_type = contract
            .document_type_cloned_for_name("domain")
            .expect("the domain document type");
        let document = document_type
            .random_document(Some(1), platform_version)
            .expect("a random domain");
        let identity_public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![1; 33]),
            disabled_at: None,
        });
        let settings = PutSettings {
            state_transition_creation_options: Some(StateTransitionCreationOptions {
                action_fee_agreement: Some(
                    DocumentActionFeeAgreementV0 {
                        owner: 1,
                        moderators: 2,
                        fee_multiplier: None,
                    }
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };

        let error = document
            .update_price_of_document_and_wait_for_response(
                10,
                &sdk,
                document_type,
                identity_public_key,
                None,
                &UnusedSigner,
                Some(settings),
            )
            .await
            .expect_err("an agreement is refused before protocol version 14");

        assert!(
            matches!(
                &error,
                crate::Error::Protocol(ProtocolError::UnknownVersionMismatch { method, .. })
                    if method.ends_with("validate_base_carries_action_fee_agreement")
            ),
            "unexpected error: {error}"
        );
    }
}
