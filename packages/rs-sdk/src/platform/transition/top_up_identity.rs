//! Top up identity balance
use super::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::dashcore::PrivateKey;
use dpp::identity::Identity;
use dpp::prelude::AssetLockProof;
use dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;

/// Top up an identity
///
/// Increase balance of the identity by the amount specified in the asset lock proof
#[async_trait::async_trait]
pub trait TopUpIdentity {
    async fn top_up_identity(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
    ) -> Result<u64, Error>;
}

#[async_trait::async_trait]
impl TopUpIdentity for Identity {
    async fn top_up_identity(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
    ) -> Result<u64, Error> {
        let state_transition = IdentityTopUpTransition::try_from_identity(
            self,
            asset_lock_proof,
            asset_lock_proof_private_key.inner.as_ref(),
            sdk.version(),
            None,
        )?;

        let result = state_transition.broadcast_and_wait(sdk, None).await?;

        match result {
            StateTransitionProofResult::VerifiedPartialIdentity(identity) => {
                identity.balance.ok_or(Error::DapiClientError(
                    "expected an identity balance".to_string(),
                ))
            }
            _ => Err(Error::DapiClientError("proved a non identity".to_string())),
        }
    }
}
