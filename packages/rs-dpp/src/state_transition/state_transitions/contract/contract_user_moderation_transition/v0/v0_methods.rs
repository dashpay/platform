#[cfg(feature = "state-transition-signing")]
use crate::consensus::signature::{MissingPublicKeyError, SignatureError};
#[cfg(feature = "state-transition-signing")]
use crate::consensus::ConsensusError;
#[cfg(feature = "state-transition-signing")]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey, KeyID};
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::prelude::{Identifier, IdentityNonce};
use crate::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use crate::state_transition::contract_user_moderation_transition::methods::ContractUserModerationTransitionMethodsV0;
use crate::state_transition::contract_user_moderation_transition::v0::{
    ContractUserModerationAction, ContractUserModerationTransitionV0,
};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition};
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ContractUserModerationTransitionMethodsV0 for ContractUserModerationTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        signing_key_id: &KeyID,
        data_contract_id: Identifier,
        action: ContractUserModerationAction,
        identity_contract_nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let signing_key = identity
            .public_keys()
            .get(signing_key_id)
            .ok_or::<ConsensusError>(
                SignatureError::MissingPublicKeyError(MissingPublicKeyError::new(*signing_key_id))
                    .into(),
            )?;

        let mut state_transition: StateTransition = ContractUserModerationTransitionV0 {
            owner_id: identity.id(),
            data_contract_id,
            identity_contract_nonce,
            action,
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        state_transition
            .sign_external(
                signing_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )
            .await?;

        Ok(state_transition)
    }
}

impl ContractUserModerationTransitionAccessorsV0 for ContractUserModerationTransitionV0 {
    fn set_owner_id(&mut self, id: Identifier) {
        self.owner_id = id;
    }

    fn set_data_contract_id(&mut self, id: Identifier) {
        self.data_contract_id = id;
    }

    fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        self.identity_contract_nonce = nonce;
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.identity_contract_nonce
    }

    fn set_action(&mut self, action: ContractUserModerationAction) {
        self.action = action;
    }

    fn action(&self) -> &ContractUserModerationAction {
        &self.action
    }
}
