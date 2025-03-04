use crate::platform::transition::builder::StateTransitionBuilder;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::{DataContract, TokenContractPosition};
use dpp::group::GroupStateTransitionInfoStatus;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::state_transitions::document::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dpp::state_transition::state_transitions::document::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::version::PlatformVersion;

/// A builder to configure and broadcast token burn transitions
pub struct TokenBurnTransitionBuilder<'a> {
    data_contract: &'a DataContract,
    token_position: TokenContractPosition,
    owner_id: Identifier,
    amount: TokenAmount,
    public_note: Option<String>,
    settings: Option<PutSettings>,
    user_fee_increase: Option<UserFeeIncrease>,
    using_group_info: Option<GroupStateTransitionInfoStatus>,
}

impl<'a> TokenBurnTransitionBuilder<'a> {
    /// Creates a new `TokenBurnTransitionBuilder`
    ///
    /// # Arguments
    ///
    /// * `data_contract` - A reference to the data contract
    /// * `token_position` - The position of the token in the contract
    /// * `owner_id` - The identifier of the token owner
    /// * `amount` - The amount of tokens to burn
    pub fn new(
        data_contract: &'a DataContract,
        token_position: TokenContractPosition,
        owner_id: Identifier,
        amount: TokenAmount,
    ) -> Self {
        Self {
            data_contract,
            token_position,
            owner_id,
            amount,
            public_note: None,
            settings: None,
            user_fee_increase: None,
            using_group_info: None,
        }
    }

    /// Adds a public note to the token burn transition
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

    /// Adds a user fee increase to the token burn transition
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

    /// Adds group information to the token burn transition
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
        self
    }

    /// Adds settings to the token burn transition
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
}

impl StateTransitionBuilder for TokenBurnTransitionBuilder<'_> {
    /// Returns the settings for the token burn transition
    ///
    /// # Returns
    ///
    /// * `Option<PutSettings>` - The settings, if any
    fn settings(&self) -> Option<PutSettings> {
        self.settings
    }

    /// Signs the token burn transition
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
    async fn sign(
        &self,
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
                false,
                self.settings,
            )
            .await?;

        let state_transition = BatchTransition::new_token_burn_transition(
            token_id,
            self.owner_id,
            self.data_contract.id(),
            self.token_position,
            self.amount,
            self.public_note.clone(),
            self.using_group_info,
            identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            signer,
            platform_version,
            None,
            None,
            None,
        )?;

        Ok(state_transition)
    }
}
