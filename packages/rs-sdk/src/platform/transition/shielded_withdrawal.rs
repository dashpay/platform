use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::identity::core_script::CoreScript;
use dpp::shielded::SerializedAction;
use dpp::state_transition::shielded_withdrawal_transition::methods::ShieldedWithdrawalTransitionMethodsV0;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::withdrawal::Pooling;

/// Helper trait to withdraw funds from the shielded pool to L1.
#[async_trait::async_trait]
pub trait WithdrawShielded {
    /// Withdraw funds from the shielded pool to a Core address.
    /// Authentication is via Orchard spend authorization signatures in the bundle actions.
    #[allow(clippy::too_many_arguments)]
    async fn withdraw_shielded(
        &self,
        amount: u64,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl WithdrawShielded for Sdk {
    async fn withdraw_shielded(
        &self,
        amount: u64,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        settings: Option<PutSettings>,
    ) -> Result<(), Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        let state_transition = ShieldedWithdrawalTransition::try_from_bundle(
            amount,
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            core_fee_per_byte,
            pooling,
            output_script,
            user_fee_increase,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        state_transition.broadcast(self, settings).await?;
        Ok(())
    }
}
