use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::{Error, Sdk};
use dpp::dashcore::PrivateKey;
use dpp::identity::{Identity, PartialIdentity};
use dpp::prelude::{AssetLockProof, UserFeeIncrease};
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use std::future::Future;
use std::pin::Pin;

pub trait TopUpIdentity: Waitable {
    fn top_up_identity<'a>(
        &'a self,
        sdk: &'a Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &'a PrivateKey,
        user_fee_increase: Option<UserFeeIncrease>,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, Error>> + Send + 'a>>;
}

impl TopUpIdentity for Identity {
    fn top_up_identity<'a>(
        &'a self,
        sdk: &'a Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &'a PrivateKey,
        user_fee_increase: Option<UserFeeIncrease>,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<u64, Error>> + Send + 'a>> {
        Box::pin(async move {
            let state_transition = IdentityTopUpTransition::try_from_identity(
                self,
                asset_lock_proof,
                asset_lock_proof_private_key.inner.as_ref(),
                user_fee_increase.unwrap_or_default(),
                sdk.version(),
                None,
            )?;
            ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
            let identity: PartialIdentity =
                state_transition.broadcast_and_wait(sdk, settings).await?;

            identity
                .balance
                .ok_or(Error::Generic("expected an identity balance".to_string()))
        })
    }
}
