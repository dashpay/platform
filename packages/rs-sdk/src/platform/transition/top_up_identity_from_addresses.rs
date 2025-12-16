use std::collections::{BTreeMap, BTreeSet};

use super::address_inputs::fetch_inputs_with_nonce;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::platform::transition::address_inputs::{collect_address_infos_from_proof, nonce_inc};
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::prelude::AddressNonce;
use dpp::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use drive_proof_verifier::types::AddressInfos;

/// Helper trait to top up an identity using balances from Platform addresses.
#[async_trait::async_trait]
pub trait TopUpIdentityFromAddresses<S: Signer<PlatformAddress>>: Waitable {
    /// Top up an identity by spending address balances (nonces looked up automatically).
    async fn top_up_from_addresses(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error>;

    /// Top up identity providing explicit address nonces.
    ///
    /// Inputs are not pre-validated client-side (Drive enforces authoritative checks).
    async fn top_up_from_addresses_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error>;
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> TopUpIdentityFromAddresses<S> for Identity {
    async fn top_up_from_addresses(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error> {
        let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(sdk, &inputs).await?);
        self.top_up_from_addresses_with_nonce(sdk, inputs_with_nonce, signer, settings)
            .await
    }

    async fn top_up_from_addresses_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let expected_addresses: BTreeSet<PlatformAddress> =
            inputs.keys().copied().collect::<BTreeSet<_>>();

        let state_transition = IdentityTopUpFromAddressesTransition::try_from_inputs_with_signer(
            self,
            inputs,
            signer,
            user_fee_increase,
            sdk.version(),
            None,
        )?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

        match state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(sdk, settings)
            .await?
        {
            StateTransitionProofResult::VerifiedIdentityWithAddressInfos(
                identity,
                address_infos_map,
            ) => {
                if identity.id != self.id() {
                    return Err(Error::InvalidProvedResponse(format!(
                        "proof returned identity {} but {} was topped up",
                        identity.id,
                        self.id()
                    )));
                }

                let address_infos =
                    collect_address_infos_from_proof(address_infos_map, &expected_addresses)?;

                let balance = identity.balance.ok_or_else(|| {
                    Error::InvalidProvedResponse(
                        "identity proof did not include updated balance".to_string(),
                    )
                })?;

                Ok((address_infos, balance))
            }
            other => Err(Error::InvalidProvedResponse(format!(
                "identity proof was expected for {:?}, but received {:?}",
                state_transition, other
            ))),
        }
    }
}
