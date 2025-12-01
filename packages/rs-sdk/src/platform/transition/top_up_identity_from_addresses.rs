use std::collections::BTreeMap;

use super::address_inputs::fetch_inputs_with_nonce;
use super::put_settings::PutSettings;
use super::waitable::Waitable;
use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::prelude::AddressNonce;
use dpp::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;

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
    ) -> Result<u64, Error>;

    /// Top up identity providing explicit address nonces.
    ///
    /// Inputs are not pre-validated client-side (Drive enforces authoritative checks).
    async fn top_up_from_addresses_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error>;
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> TopUpIdentityFromAddresses<S> for Identity {
    async fn top_up_from_addresses(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error> {
        let inputs_with_nonce = fetch_inputs_with_nonce(sdk, &inputs).await?;
        self.top_up_from_addresses_with_nonce(sdk, inputs_with_nonce, signer, settings)
            .await
    }

    async fn top_up_from_addresses_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<u64, Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let state_transition = IdentityTopUpFromAddressesTransition::try_from_inputs_with_signer(
            self,
            inputs,
            signer,
            user_fee_increase,
            sdk.version(),
            None,
        )?;

        match state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(sdk, settings)
            .await?
        {
            StateTransitionProofResult::VerifiedPartialIdentity(identity) => identity
                .balance
                .ok_or_else(|| Error::Generic("expected an identity balance".to_string())),
            _ => Err(Error::Generic(
                "identity proof was expected but not returned".to_string(),
            )),
        }
    }
}
