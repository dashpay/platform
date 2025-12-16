use std::collections::{BTreeMap, BTreeSet};

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::platform::transition::waitable::Waitable;
use crate::platform::{Fetch, FetchMany};
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::methods::IdentityCreditTransferToAddressesTransitionMethodsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use drive_proof_verifier::types::{AddressInfo, AddressInfos};

#[async_trait::async_trait]
pub trait TransferToAddresses: Waitable {
    /// Transfer credits from an identity to multiple Platform addresses.
    ///
    /// Returns tuple of:
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
    ) -> Result<(u64, AddressInfos), Error>;
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
    ) -> Result<(u64, AddressInfos), Error> {
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

        match state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(sdk, settings)
            .await?
        {
            // TODO: Return correct data from the proof result and avoid extra fetches.
            StateTransitionProofResult::VerifiedPartialIdentity(_) => {}
            other => {
                return Err(Error::Generic(format!(
                    "unexpected proof result received: {:?}",
                    other
                )))
            }
        }

        // Refresh identity balance after transfer to reflect final state
        let updated_identity = Identity::fetch(sdk, self.id())
            .await?
            .ok_or_else(|| Error::Generic("identity was not found after transfer".to_string()))?;
        let updated_balance = updated_identity.balance();

        // Fetch updated address balances/nonces for recipients
        let addresses_query: BTreeSet<PlatformAddress> =
            recipient_addresses.keys().copied().collect();
        let address_infos = AddressInfo::fetch_many(sdk, addresses_query).await?;

        Ok((updated_balance, address_infos))
    }
}
