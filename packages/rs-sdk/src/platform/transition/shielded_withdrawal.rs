use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::identity::core_script::CoreScript;
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::shielded_withdrawal_transition::methods::ShieldedWithdrawalTransitionMethodsV0;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::withdrawal::Pooling;
use std::future::Future;
use std::pin::Pin;

/// Helper trait to withdraw funds from the shielded pool to L1.
pub trait WithdrawShielded {
    /// Withdraw funds from the shielded pool to a Core address.
    /// Authentication is via Orchard spend authorization signatures in the bundle actions.
    #[allow(clippy::too_many_arguments)]
    fn withdraw_shielded<'a>(
        &'a self,
        unshielding_amount: u64,
        bundle: OrchardBundleParams,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;
}

impl WithdrawShielded for Sdk {
    #[allow(clippy::too_many_arguments)]
    fn withdraw_shielded<'a>(
        &'a self,
        unshielding_amount: u64,
        bundle: OrchardBundleParams,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        settings: Option<PutSettings>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
        Box::pin(async move {
            let OrchardBundleParams {
                actions,
                anchor,
                proof,
                binding_signature,
            } = bundle;

            let state_transition = ShieldedWithdrawalTransition::try_from_bundle(
                actions,
                unshielding_amount,
                anchor,
                proof,
                binding_signature,
                core_fee_per_byte,
                pooling,
                output_script,
                self.version(),
            )?;
            ensure_valid_state_transition_structure(&state_transition, self.version())?;

            state_transition.broadcast(self, settings).await?;
            Ok(())
        })
    }
}
