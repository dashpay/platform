use std::collections::{BTreeMap, BTreeSet};

use super::address_inputs::collect_address_infos_from_proof;
use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::platform::transition::waitable::Waitable;
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::methods::IdentityCreditTransferToAddressesTransitionMethodsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use drive_proof_verifier::types::AddressInfos;

#[async_trait::async_trait]
pub trait TransferToAddresses: Waitable {
    /// Transfer credits from an identity to multiple Platform addresses.
    ///
    /// Returns tuple of:
    /// * Proof-backed address infos for provided recipients
    /// * Updated identity balance
    /// * Proof-backed address infos for provided recipients
    #[allow(clippy::too_many_arguments)]
    async fn transfer_credits_to_addresses<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        recipient_addresses: BTreeMap<PlatformAddress, Credits>,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error>;
}

#[async_trait::async_trait]
impl TransferToAddresses for Identity {
    async fn transfer_credits_to_addresses<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        recipient_addresses: BTreeMap<PlatformAddress, Credits>,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error> {
        if recipient_addresses.is_empty() {
            return Err(Error::Generic(
                "recipient_addresses must contain at least one address".to_string(),
            ));
        }

        let new_identity_nonce = sdk.get_identity_nonce(self.id(), true, settings).await?;
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let state_transition = IdentityCreditTransferToAddressesTransition::try_from_identity(
            self,
            recipient_addresses.clone(),
            user_fee_increase,
            signer,
            signing_transfer_key_to_use,
            new_identity_nonce,
            sdk.version(),
            None,
        )?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

        let expected_addresses: BTreeSet<PlatformAddress> =
            recipient_addresses.keys().copied().collect();

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
                        "proof returned identity {} but {} initiated transfer",
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
