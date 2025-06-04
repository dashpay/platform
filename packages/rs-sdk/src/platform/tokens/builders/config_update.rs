use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
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
use dpp::version::PlatformVersion;
use std::sync::Arc;

/// A builder to configure and broadcast token config_update transitions
pub struct TokenConfigUpdateTransitionBuilder {
    data_contract: Arc<DataContract>,
    token_position: TokenContractPosition,
    owner_id: Identifier,
    update_token_configuration_item: TokenConfigurationChangeItem,
    public_note: Option<String>,
    using_group_info: Option<GroupStateTransitionInfoStatus>,
    settings: Option<PutSettings>,
    user_fee_increase: Option<UserFeeIncrease>,
}

impl TokenConfigUpdateTransitionBuilder {
    /// Start building a config_update tokens transition for the provided DataContract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - An Arc to the data contract
    /// * `token_position` - The position of the token in the contract
    /// * `owner_id` - The identifier of the state transition owner
    /// * `update_token_configuration_item` - The token configuration change item
    /// * `using_group_info` - Group transition info status
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn new(
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        owner_id: Identifier,
        update_token_configuration_item: TokenConfigurationChangeItem,
    ) -> Self {
        // TODO: Validate token position

        Self {
            data_contract,
            token_position,
            owner_id,
            update_token_configuration_item,
            public_note: None,
            using_group_info: None,
            settings: None,
            user_fee_increase: None,
        }
    }

    /// Adds a public note to the token config_update transition
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

    /// Adds a user fee increase to the token config_update transition
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

    /// Adds group information to the token config update transition
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

    /// Adds settings to the token config_update transition
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

    /// Signs the token config_update transition
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
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
        options: Option<StateTransitionCreationOptions>,
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

        let state_transition = BatchTransition::new_token_config_update_transition(
            token_id,
            self.owner_id,
            self.data_contract.id(),
            self.token_position,
            self.update_token_configuration_item.clone(),
            self.public_note.clone(),
            self.using_group_info,
            identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            signer,
            platform_version,
            options,
        )?;

        Ok(state_transition)
    }
}
