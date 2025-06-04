use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::version::PlatformVersion;
use std::sync::Arc;

/// A builder to configure and broadcast token claim transitions
pub struct TokenClaimTransitionBuilder {
    pub data_contract: Arc<DataContract>,
    pub token_position: TokenContractPosition,
    pub owner_id: Identifier,
    pub distribution_type: TokenDistributionType,
    pub public_note: Option<String>,
    pub settings: Option<PutSettings>,
    pub user_fee_increase: Option<UserFeeIncrease>,
    pub signing_options: Option<StateTransitionCreationOptions>,
}

impl TokenClaimTransitionBuilder {
    /// Start building a claim tokens transition for the provided DataContract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - An Arc to the data contract
    /// * `token_position` - The position of the token in the contract
    /// * `owner_id` - The identifier of the state transition owner
    /// * `distribution_type` - The token distribution type
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn new(
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        owner_id: Identifier,
        distribution_type: TokenDistributionType,
    ) -> Self {
        Self {
            data_contract,
            token_position,
            owner_id,
            distribution_type,
            public_note: None,
            settings: None,
            user_fee_increase: None,
            signing_options: None,
        }
    }

    /// Adds a public note to the token claim transition
    ///
    /// # Arguments
    ///
    /// * `note` - The public note to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_public_note(mut self, note: String) -> Self {
        self.public_note = Some(note);
        self
    }

    /// Adds a user fee increase to the token claim transition
    ///
    /// # Arguments
    ///
    /// * `user_fee_increase` - The user fee increase to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_user_fee_increase(mut self, user_fee_increase: UserFeeIncrease) -> Self {
        self.user_fee_increase = Some(user_fee_increase);
        self
    }

    /// Adds settings to the token claim transition
    ///
    /// # Arguments
    ///
    /// * `settings` - The settings to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_settings(mut self, settings: PutSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// Adds signing options to the token claim transition
    ///
    /// # Arguments
    ///
    /// * `signing_options` - The signing options to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_signing_options(mut self, signing_options: StateTransitionCreationOptions) -> Self {
        self.signing_options = Some(signing_options);
        self
    }

    /// Signs the token claim transition
    ///
    /// # Arguments
    ///
    /// * `sdk` - The SDK instance
    /// * `identity_public_key` - The public key of the identity
    /// * `signer` - The signer instance
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    ///
    /// * `Result<StateTransition, Error>` - The signed state transition or an error
    pub async fn sign(
        self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, Error> {
        let token_id = Identifier::from(calculate_token_id(
            self.data_contract.id().as_bytes(),
            self.token_position,
        ));

        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(
                self.owner_id,
                self.data_contract.id(),
                true,
                self.settings,
            )
            .await?;

        let state_transition = BatchTransition::new_token_claim_transition(
            token_id,
            self.owner_id,
            self.data_contract.id(),
            self.token_position,
            self.distribution_type,
            self.public_note.clone(),
            identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            signer,
            platform_version,
            self.signing_options,
        )?;

        Ok(state_transition)
    }
}
