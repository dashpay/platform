use crate::platform::transition::broadcast::BroadcastStateTransition;
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
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::StateTransition;
use dpp::tokens::calculate_token_id;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::version::PlatformVersion;

/// A builder to configure minting tokens.
pub struct FreezeTokensStateTransitionBuilder<'a> {
    data_contract: &'a DataContract,
    token_position: TokenContractPosition,
    actor_id: Identifier,
    freeze_identity_id: Identifier,
    public_note: Option<String>,
    settings: Option<PutSettings>,
    user_fee_increase: Option<UserFeeIncrease>,
    using_group_info: Option<GroupStateTransitionInfoStatus>,
}

impl<'a> FreezeTokensStateTransitionBuilder<'a> {
    /// Start building a mint tokens request for the provided DataContract.
    pub fn new(
        data_contract: &'a DataContract,
        token_position: TokenContractPosition,
        actor_id: Identifier,
        freeze_identity_id: Identifier,
    ) -> Self {
        // TODO: Validate token position

        Self {
            data_contract,
            token_position,
            actor_id,
            freeze_identity_id,
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

    pub async fn broadcast(
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
                self.actor_id,
                self.data_contract.id(),
                false,
                self.settings,
            )
            .await?;

        let state_transition = BatchTransition::new_token_freeze_transition(
            token_id,
            self.actor_id,
            self.data_contract.id(),
            self.token_position,
            self.freeze_identity_id,
            self.public_note.clone(),
            self.using_group_info,
            &identity_public_key,
            identity_contract_nonce,
            self.user_fee_increase.unwrap_or_default(),
            signer,
            platform_version,
            None,
            None,
            None,
        )?;

        state_transition.broadcast(sdk, self.settings).await?;

        Ok(state_transition)
    }

    pub async fn broadcast_and_wait_for_result(
        &self,
        sdk: &Sdk,
        identity_public_key: &IdentityPublicKey,
        signer: &impl Signer,
        platform_version: &PlatformVersion,
    ) -> Result<(Identifier, IdentityTokenInfo), Error> {
        let state_transition = self
            .broadcast(sdk, identity_public_key, signer, platform_version)
            .await?;

        state_transition.wait_for_response(sdk, self.settings).await
    }
}
