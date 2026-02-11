use std::collections::BTreeMap;

use super::address_inputs::{fetch_inputs_with_nonce, nonce_inc};
use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::prelude::AddressNonce;
use dpp::shielded::SerializedAction;
use dpp::state_transition::shield_transition::methods::ShieldTransitionMethodsV0;
use dpp::state_transition::shield_transition::ShieldTransition;

/// Helper trait to shield platform credits into the shielded pool.
#[async_trait::async_trait]
pub trait ShieldFunds<S: Signer<PlatformAddress>> {
    /// Shield funds from platform addresses into the shielded pool.
    /// Address nonces are fetched automatically.
    #[allow(clippy::too_many_arguments)]
    async fn shield_funds(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;

    /// Shield funds with explicitly provided address nonces.
    #[allow(clippy::too_many_arguments)]
    async fn shield_funds_with_nonce(
        &self,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> ShieldFunds<S> for Sdk {
    async fn shield_funds(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error> {
        let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(self, &inputs).await?);
        self.shield_funds_with_nonce(
            inputs_with_nonce,
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            fee_strategy,
            signer,
            settings,
        )
        .await
    }

    async fn shield_funds_with_nonce(
        &self,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        let state_transition = ShieldTransition::try_from_bundle_with_signer(
            inputs,
            actions,
            flags,
            value_balance,
            anchor,
            proof,
            binding_signature,
            fee_strategy,
            signer,
            user_fee_increase,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        state_transition.broadcast(self, settings).await?;
        Ok(())
    }
}
