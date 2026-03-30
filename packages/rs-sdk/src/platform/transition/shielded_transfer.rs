use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::shielded_transfer_transition::methods::ShieldedTransferTransitionMethodsV0;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use std::future::Future;
use std::pin::Pin;

/// Helper trait to transfer funds within the shielded pool.
pub trait TransferShielded {
    /// Transfer funds within the shielded pool.
    /// Authentication is via Orchard spend authorization signatures in the bundle actions.
    fn transfer_shielded<'a>(
        &'a self,
        bundle: OrchardBundleParams,
        value_balance: u64,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;
}

impl TransferShielded for Sdk {
    fn transfer_shielded<'a>(
        &'a self,
        bundle: OrchardBundleParams,
        value_balance: u64,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
        Box::pin(async move {
            let OrchardBundleParams {
                actions,
                anchor,
                proof,
                binding_signature,
            } = bundle;

            let state_transition = ShieldedTransferTransition::try_from_bundle(
                actions,
                value_balance,
                anchor,
                proof,
                binding_signature,
                self.version(),
            )?;
            ensure_valid_state_transition_structure(&state_transition, self.version())?;

            state_transition.broadcast(self, settings).await?;
            Ok(())
        })
    }
}
