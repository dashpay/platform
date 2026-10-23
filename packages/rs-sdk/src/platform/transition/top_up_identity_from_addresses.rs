use dapi_grpc::platform::v0::ResponseMetadata;
use std::collections::{BTreeMap, BTreeSet};

use super::address_inputs::fetch_inputs_with_nonce;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use crate::platform::transition::address_inputs::{collect_address_infos_from_proof, nonce_inc};
use crate::platform::transition::broadcast::{
    broadcast_with_observation, wait_after_broadcast, BroadcastStateTransition,
};
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::prelude::{AddressNonce, Identifier};
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
    ) -> Result<(AddressInfos, Credits, u64), Error>;

    /// Top up identity providing explicit address nonces.
    ///
    /// Inputs are not pre-validated client-side (Drive enforces authoritative checks).
    async fn top_up_from_addresses_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, u64), Error>;
}

/// Identity top-ups that preserve the full metadata of the balance proof.
#[async_trait::async_trait]
pub trait TopUpIdentityFromAddressesWithMetadata<S: Signer<PlatformAddress>>: Waitable {
    /// Top up with automatically resolved address nonces and return proof metadata.
    async fn top_up_from_addresses_with_metadata(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), Error>;

    /// Top up with explicit address nonces and return proof metadata.
    async fn top_up_from_addresses_with_nonce_with_metadata(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), Error>;
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> TopUpIdentityFromAddresses<S> for Identity {
    async fn top_up_from_addresses(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, u64), Error> {
        self.top_up_from_addresses_with_metadata(sdk, inputs, signer, settings)
            .await
            .map(|(infos, balance, metadata)| (infos, balance, metadata.height))
    }

    async fn top_up_from_addresses_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, u64), Error> {
        self.top_up_from_addresses_with_nonce_with_metadata(sdk, inputs, signer, settings)
            .await
            .map(|(infos, balance, metadata)| (infos, balance, metadata.height))
    }
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> TopUpIdentityFromAddressesWithMetadata<S> for Identity {
    async fn top_up_from_addresses_with_metadata(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), Error> {
        let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(sdk, &inputs).await?);
        self.top_up_from_addresses_with_nonce_with_metadata(
            sdk,
            inputs_with_nonce,
            signer,
            settings,
        )
        .await
    }

    async fn top_up_from_addresses_with_nonce_with_metadata(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), Error> {
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
        )
        .await?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

        // Only the wait future runs after accepted/possibly accepted submission.
        // Keep preparation and definite broadcast refusals outside the wrapper.
        wait_after_broadcast(
            broadcast_with_observation(&state_transition, sdk, settings).await,
            async {
                let result = async {
                    // `metadata.height` pins these proved balance absolutes.
                    let (st_result, metadata) = state_transition
                        .wait_for_affected_state_with_metadata::<StateTransitionProofResult>(
                            sdk, settings,
                        )
                        .await?;
                    top_up_result(self.id(), st_result, &expected_addresses)
                        .map(|(infos, balance)| (infos, balance, metadata))
                }
                .await;
                result.map_err(|error| Error::StateTransitionResultUnconfirmed(Box::new(error)))
            },
        )
        .await
    }
}

fn top_up_result(
    identity_id: Identifier,
    st_result: StateTransitionProofResult,
    expected_addresses: &BTreeSet<PlatformAddress>,
) -> Result<(AddressInfos, Credits), Error> {
    match st_result {
        StateTransitionProofResult::VerifiedIdentityWithAddressInfos(
            identity,
            address_infos_map,
        ) => {
            if identity.id != identity_id {
                return Err(Error::InvalidProvedResponse(format!(
                    "proof returned identity {} but {} was topped up",
                    identity.id, identity_id
                )));
            }

            let address_infos =
                collect_address_infos_from_proof(address_infos_map, expected_addresses)?;

            let balance = identity.balance.ok_or_else(|| {
                Error::InvalidProvedResponse(
                    "identity proof did not include updated balance".to_string(),
                )
            })?;

            Ok((address_infos, balance))
        }
        other => Err(Error::InvalidProvedResponse(format!(
            "identity top-up proof was expected for {}, but received {:?}",
            identity_id, other
        ))),
    }
}
