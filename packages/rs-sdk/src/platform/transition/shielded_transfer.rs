use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::shielded_transfer_transition::methods::ShieldedTransferTransitionMethodsV0;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;

/// Helper trait to transfer funds within the shielded pool.
#[async_trait::async_trait]
pub trait TransferShielded {
    /// Transfer funds within the shielded pool.
    /// Authentication is via Orchard spend authorization signatures in the bundle actions.
    async fn transfer_shielded(
        &self,
        bundle: OrchardBundleParams,
        value_balance: u64,
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl TransferShielded for Sdk {
    async fn transfer_shielded(
        &self,
        bundle: OrchardBundleParams,
        value_balance: u64,
        settings: Option<PutSettings>,
    ) -> Result<(), Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        let OrchardBundleParams {
            actions,
            flags,
            anchor,
            proof,
            binding_signature,
        } = bundle;

        let state_transition = ShieldedTransferTransition::try_from_bundle(
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            user_fee_increase,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        state_transition.broadcast(self, settings).await?;
        Ok(())
    }
}
