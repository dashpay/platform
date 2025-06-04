use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use crate::{Error, Sdk};
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
use dpp::version::PlatformVersion;
use std::sync::Arc;

/// A builder to configure and broadcast token unfreeze transitions
pub struct TokenUnfreezeTransitionBuilder {
    pub data_contract: Arc<DataContract>,
    pub token_position: TokenContractPosition,
    pub actor_id: Identifier,
    pub unfreeze_identity_id: Identifier,
    pub public_note: Option<String>,
    pub settings: Option<PutSettings>,
    pub user_fee_increase: Option<UserFeeIncrease>,
    pub using_group_info: Option<GroupStateTransitionInfoStatus>,
    pub signing_options: Option<StateTransitionCreationOptions>,
}

impl TokenUnfreezeTransitionBuilder {
    /// Start building a mint tokens request for the provided DataContract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - An Arc to the data contract
    /// * `token_position` - The position of the token in the contract
    /// * `actor_id` - The identifier of the actor
    /// * `unfreeze_identity_id` - The identifier of the identity to unfreeze
    ///
    /// # Returns
    ///
    /// * `Self` - The new builder instance
    pub fn new(
        data_contract: Arc<DataContract>,
        token_position: TokenContractPosition,
        actor_id: Identifier,
        unfreeze_identity_id: Identifier,
    ) -> Self {
        Self {
            data_contract,
            token_position,
            actor_id,
            unfreeze_identity_id,
            public_note: None,
            settings: None,
            user_fee_increase: None,
            using_group_info: None,
            signing_options: None,
        }
    }

    /// Adds a public note to the token unfreeze transition
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

    /// Adds a user fee increase to the token unfreeze transition
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

    /// Adds group information to the token unfreeze transition
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

    /// Adds settings to the token unfreeze transition
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

    /// Adds signing options to the token unfreeze transition
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

    /// Signs the token unfreeze transition
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

        let state_transition = BatchTransition::new_token_unfreeze_transition(
            token_id,
            self.actor_id,
            self.data_contract.id(),
            self.token_position,
            self.unfreeze_identity_id,
            self.public_note.clone(),
            self.using_group_info,
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
