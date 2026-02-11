use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::shielded::SerializedAction;
use dpp::state_transition::unshield_transition::methods::UnshieldTransitionMethodsV0;
use dpp::state_transition::unshield_transition::UnshieldTransition;

/// Helper trait to unshield funds from the shielded pool to a platform address.
#[async_trait::async_trait]
pub trait UnshieldFunds {
    /// Unshield funds from the shielded pool to a platform address.
    /// Authentication is via Orchard spend authorization signatures in the bundle actions.
    #[allow(clippy::too_many_arguments)]
    async fn unshield_funds(
        &self,
        output_address: PlatformAddress,
        amount: u64,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl UnshieldFunds for Sdk {
    async fn unshield_funds(
        &self,
        output_address: PlatformAddress,
        amount: u64,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        settings: Option<PutSettings>,
    ) -> Result<(), Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        let state_transition = UnshieldTransition::try_from_bundle(
            output_address,
            amount,
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
