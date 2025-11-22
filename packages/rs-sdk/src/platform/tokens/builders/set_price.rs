use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::balances::credits::Credits;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::group::GroupStateTransitionInfoStatus;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A builder to configure and broadcast token change direct purchase price transitions
pub struct TokenChangeDirectPurchasePriceTransitionBuilder {
    pub data_contract: Arc<DataContract>,
    pub token_position: TokenContractPosition,
    pub actor_id: Identifier,
    pub token_pricing_schedule: Option<TokenPricingSchedule>,
    pub public_note: Option<String>,
    pub settings: Option<PutSettings>,
    pub user_fee_increase: Option<UserFeeIncrease>,
    pub using_group_info: Option<GroupStateTransitionInfoStatus>,
    pub state_transition_creation_options: Option<StateTransitionCreationOptions>,
}

impl TokenChangeDirectPurchasePriceTransitionBuilder {
    /// Start building a change direct purchase price tokens request for the provided DataContract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - An Arc to the data contract
    /// * `token_position` - The position of the token in the contract
    /// * `issuer_id` - The identifier of the issuer
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn new(
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        issuer_id: Identifier,
    ) -> Self {
        Self {
            data_contract,
            token_position,
            actor_id: issuer_id,
            token_pricing_schedule: None,
            public_note: None,
            settings: None,
            user_fee_increase: None,
            using_group_info: None,
            state_transition_creation_options: None,
        }
    }

    /// Sets a single price for all token amounts
    ///
    /// # Arguments
    ///
    /// * `price` - The price in credits for any token amount
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_single_price(mut self, price: Credits) -> Self {
        self.token_pricing_schedule = Some(TokenPricingSchedule::SinglePrice(price));
        self
    }

    /// Sets tiered pricing based on token amounts
    ///
    /// # Arguments
    ///
    /// * `price_entries` - A vector of (token_amount, price_in_credits) tuples
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_price_entries(mut self, price_entries: Vec<(TokenAmount, Credits)>) -> Self {
        let price_map: BTreeMap<TokenAmount, Credits> = price_entries.into_iter().collect();
        self.token_pricing_schedule = Some(TokenPricingSchedule::SetPrices(price_map));
        self
    }

    /// Sets the token pricing schedule directly
    ///
    /// # Arguments
    ///
    /// * `pricing_schedule` - The complete pricing schedule
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_token_pricing_schedule(mut self, pricing_schedule: TokenPricingSchedule) -> Self {
        self.token_pricing_schedule = Some(pricing_schedule);
        self
    }

    /// Adds a public note to the token change direct purchase price transition
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

    /// Adds a user fee increase to the token change direct purchase price transition
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

    /// Adds group information to the token change direct purchase price transition
    ///
    /// # Arguments
    ///
    /// * `group_info` - The group information to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_using_group_info(mut self, group_info: GroupStateTransitionInfoStatus) -> Self {
        self.using_group_info = Some(group_info);

        // TODO: Simplify group actions automatically find position if group action is required

        self
    }

    /// Adds settings to the token change direct purchase price transition
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

    /// Adds state transition creation options to the token change direct purchase price transition
    ///
    /// # Arguments
    ///
    /// * `state_transition_creation_options` - The state transition creation options to add
    ///
    /// # Returns
    ///
    /// * `Self` - The updated builder
    pub fn with_state_transition_creation_options(
        mut self,
        state_transition_creation_options: StateTransitionCreationOptions,
    ) -> Self {
        self.state_transition_creation_options = Some(state_transition_creation_options);
        self
    }

    /// Signs the token change direct purchase price transition
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
                self.actor_id,
                self.data_contract.id(),
                true,
                self.settings,
            )
            .await?;

        let state_transition = BatchTransition::new_token_change_direct_purchase_price_transition(
            token_id,
            self.actor_id,
            self.data_contract.id(),
            self.token_position,
            self.token_pricing_schedule.clone(),
            self.public_note.clone(),
            self.using_group_info,
            identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            signer,
            platform_version,
            self.state_transition_creation_options,
        )?;

        Ok(state_transition)
    }
}
