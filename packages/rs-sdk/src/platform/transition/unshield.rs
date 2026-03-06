use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::unshield_transition::methods::UnshieldTransitionMethodsV0;
use dpp::state_transition::unshield_transition::UnshieldTransition;

/// Helper trait to unshield funds from the shielded pool to a platform address.
#[async_trait::async_trait]
pub trait UnshieldFunds {
    /// Unshield funds from the shielded pool to a platform address.
    /// Authentication is via Orchard spend authorization signatures in the bundle actions.
    async fn unshield_funds(
        &self,
        output_address: PlatformAddress,
        unshielding_amount: u64,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl UnshieldFunds for Sdk {
    async fn unshield_funds(
        &self,
        output_address: PlatformAddress,
        unshielding_amount: u64,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<(), Error> {
        let OrchardBundleParams {
            actions,
            anchor,
            proof,
            binding_signature,
        } = bundle;

        let state_transition = UnshieldTransition::try_from_bundle(
            output_address,
            actions,
            unshielding_amount,
            anchor,
            proof,
            binding_signature,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        state_transition.broadcast(self, settings).await?;
        Ok(())
    }
}
