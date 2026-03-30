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
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::shield_transition::methods::ShieldTransitionMethodsV0;
use dpp::state_transition::shield_transition::ShieldTransition;

/// Helper trait to shield platform credits into the shielded pool.
#[async_trait::async_trait]
pub trait ShieldFunds<S: Signer<PlatformAddress>> {
    /// Shield funds from platform addresses into the shielded pool.
    /// Address nonces are fetched automatically.
    async fn shield_funds(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        bundle: OrchardBundleParams,
        amount: u64,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;

    /// Shield funds with explicitly provided address nonces.
    async fn shield_funds_with_nonce(
        &self,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        bundle: OrchardBundleParams,
        amount: u64,
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
        bundle: OrchardBundleParams,
        amount: u64,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error> {
        let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(self, &inputs).await?);
        self.shield_funds_with_nonce(
            inputs_with_nonce,
            bundle,
            amount,
            fee_strategy,
            signer,
            settings,
        )
        .await
    }

    async fn shield_funds_with_nonce(
        &self,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        bundle: OrchardBundleParams,
        amount: u64,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();

        let OrchardBundleParams {
            actions,
            anchor,
            proof,
            binding_signature,
        } = bundle;

        let state_transition = ShieldTransition::try_from_bundle_with_signer(
            inputs,
            actions,
            amount,
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
