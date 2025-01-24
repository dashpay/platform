use crate::platform::transition::broadcast::BroadcastStateTransition;
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
use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::version::PlatformVersion;

/// A builder to configure and broadcast burning tokens transition
pub struct BurnTokensStateTransitionBuilder<'a> {
    data_contract: &'a DataContract,
    token_position: TokenContractPosition,
    owner_id: Identifier,
    amount: TokenAmount,
    public_note: Option<String>,
    settings: Option<PutSettings>,
    user_fee_increase: Option<UserFeeIncrease>,
    using_group_info: Option<GroupStateTransitionInfoStatus>,
}

impl<'a> BurnTokensStateTransitionBuilder<'a> {
    pub fn new(
        data_contract: &'a DataContract,
        token_position: TokenContractPosition,
        owner_id: Identifier,
        amount: TokenAmount,
    ) -> Self {
        // TODO: Validate token position

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

    pub fn with_public_note(mut self, note: String) -> Self {
        self.public_note = Some(note);
        self
    }

    pub fn with_user_fee_increase(mut self, user_fee_increase: UserFeeIncrease) -> Self {
        self.user_fee_increase = Some(user_fee_increase);
        self
    }

    pub fn with_using_group_info(mut self, group_info: GroupStateTransitionInfoStatus) -> Self {
        self.using_group_info = Some(group_info);

        // TODO: Simplify group actions automatically find position if group action is required

        self
    }

    pub fn with_settings(mut self, settings: PutSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    pub async fn sign(
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

    pub async fn broadcast(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, Error> {
        let state_transition = self
            .sign(sdk, identity_public_key, signer, platform_version)
            .await?;

        state_transition.broadcast(sdk, self.settings).await?;

        Ok(state_transition)
    }

    pub async fn broadcast_and_wait_for_result(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
    ) -> Result<(Identifier, TokenAmount), Error> {
        let state_transition = self
            .broadcast(sdk, identity_public_key, signer, platform_version)
            .await?;

        state_transition.wait_for_response(sdk, self.settings).await
    }
}
