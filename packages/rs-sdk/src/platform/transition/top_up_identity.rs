use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::waitable::Waitable;
use crate::{Error, Sdk};
use dpp::dashcore::PrivateKey;
use dpp::identity::{Identity, PartialIdentity};
use dpp::prelude::{AssetLockProof, UserFeeIncrease};
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;

#[async_trait::async_trait]
pub trait TopUpIdentity: Waitable {
    async fn top_up_identity(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        user_fee_increase: Option<UserFeeIncrease>,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>;
}

#[async_trait::async_trait]
impl TopUpIdentity for Identity {
    async fn top_up_identity(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        user_fee_increase: Option<UserFeeIncrease>,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error> {
        let state_transition = IdentityTopUpTransition::try_from_identity(
            self,
            asset_lock_proof,
            asset_lock_proof_private_key.inner.as_ref(),
            user_fee_increase.unwrap_or_default(),
            sdk.version(),
            None,
        )?;
        let identity: PartialIdentity = state_transition.broadcast_and_wait(sdk, settings).await?;

        identity
            .balance
            .ok_or(Error::Generic("expected an identity balance".to_string()))
    }
}
