use std::collections::{BTreeMap, BTreeSet};

use super::address_inputs::fetch_inputs_with_nonce;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::platform::transition::address_inputs::nonce_inc;
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::FetchMany;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::errors::consensus::basic::state_transition::TransitionNoOutputsError;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::prelude::AddressNonce;
use dpp::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use drive_proof_verifier::types::{AddressInfo, AddressInfos};

/// Helper trait to transfer funds directly between Platform addresses.
#[async_trait::async_trait]
pub trait TransferAddressFunds<S: Signer<PlatformAddress>> {
    /// Broadcast address funds transfer (nonces looked up automatically) and return refreshed balances.
    async fn transfer_address_funds(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error>;

    /// Broadcast address funds transfer with explicitly provided address nonces.
    ///
    /// Inputs are not pre-validated client-side (Drive will perform authoritative checks).
    async fn transfer_address_funds_with_nonce(
        &self,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error>;
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> TransferAddressFunds<S> for Sdk {
    async fn transfer_address_funds(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error> {
        let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(self, &inputs).await?);
        self.transfer_address_funds_with_nonce(
            inputs_with_nonce,
            outputs,
            fee_strategy,
            signer,
            settings,
        )
        .await
    }

    async fn transfer_address_funds_with_nonce(
        &self,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error> {
        if outputs.is_empty() {
            return Err(Error::from(TransitionNoOutputsError::new()));
        }

        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let state_transition = AddressFundsTransferTransition::try_from_inputs_with_signer(
            inputs.clone(),
            outputs.clone(),
            fee_strategy,
            signer,
            user_fee_increase,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, settings)
            .await?;

        // Refresh balances for all addresses involved.
        let addresses: BTreeSet<PlatformAddress> =
            inputs.keys().chain(outputs.keys()).copied().collect();
        AddressInfo::fetch_many(self, addresses).await
    }
}
