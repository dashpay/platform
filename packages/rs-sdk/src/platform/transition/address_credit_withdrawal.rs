use std::collections::{BTreeMap, BTreeSet};

use super::address_inputs::fetch_inputs_with_nonce;
use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::platform::transition::address_inputs::nonce_inc;
use crate::platform::FetchMany;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::prelude::AddressNonce;
use dpp::state_transition::address_credit_withdrawal_transition::methods::AddressCreditWithdrawalTransitionMethodsV0;
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::withdrawal::Pooling;
use drive_proof_verifier::types::{AddressInfo, AddressInfos};

/// Helper trait to withdraw credits from Platform addresses into a Core script.
#[async_trait::async_trait]
pub trait WithdrawAddressFunds<S: Signer<PlatformAddress>> {
    /// Withdraws address balances (nonces fetched automatically).
    #[allow(clippy::too_many_arguments)]
    async fn withdraw_address_funds(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        change_output: Option<(PlatformAddress, Credits)>,
        fee_strategy: AddressFundsFeeStrategy,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error>;

    /// Withdraws address balances with explicitly provided nonces.
    ///
    /// Inputs are not pre-validated client-side beyond signature preparation.
    #[allow(clippy::too_many_arguments)]
    async fn withdraw_address_funds_with_nonce(
        &self,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        change_output: Option<(PlatformAddress, Credits)>,
        fee_strategy: AddressFundsFeeStrategy,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error>;
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> WithdrawAddressFunds<S> for Sdk {
    async fn withdraw_address_funds(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        change_output: Option<(PlatformAddress, Credits)>,
        fee_strategy: AddressFundsFeeStrategy,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error> {
        let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(self, &inputs).await?);
        self.withdraw_address_funds_with_nonce(
            inputs_with_nonce,
            change_output,
            fee_strategy,
            core_fee_per_byte,
            pooling,
            output_script,
            signer,
            settings,
        )
        .await
    }

    async fn withdraw_address_funds_with_nonce(
        &self,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        change_output: Option<(PlatformAddress, Credits)>,
        fee_strategy: AddressFundsFeeStrategy,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let state_transition = AddressCreditWithdrawalTransition::try_from_inputs_with_signer(
            inputs.clone(),
            change_output,
            fee_strategy,
            core_fee_per_byte,
            pooling,
            output_script,
            signer,
            user_fee_increase,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        match state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, settings)
            .await?
        {
            StateTransitionProofResult::VerifiedPartialIdentity(_) => {}
            other => {
                return Err(Error::InvalidProvedResponse(format!(
                    "unexpected proof result for address withdrawal: {:?}",
                    other
                )))
            }
        }

        // Refresh balances for all affected addresses (inputs plus optional change output).
        let mut affected_addresses: BTreeSet<PlatformAddress> =
            inputs.keys().copied().collect::<BTreeSet<_>>();
        if let Some((change_address, _)) = change_output {
            affected_addresses.insert(change_address);
        }

        AddressInfo::fetch_many(self, affected_addresses).await
    }
}
