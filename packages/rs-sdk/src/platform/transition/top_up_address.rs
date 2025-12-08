use std::collections::{BTreeMap, BTreeSet};

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::dashcore::PrivateKey;
use dpp::errors::consensus::basic::state_transition::TransitionNoOutputsError;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::prelude::{AddressNonce, AssetLockProof, UserFeeIncrease};
use dpp::state_transition::address_funding_from_asset_lock_transition::methods::AddressFundingFromAssetLockTransitionMethodsV0;
use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;
use drive_proof_verifier::types::{AddressInfo, AddressInfos};

/// Trait for topping up Platform addresses using various funding sources.
#[async_trait::async_trait]
pub trait TopUpAddress<S: Signer<PlatformAddress>> {
    /// Tops up addresses using the provided funding source and fee strategy.
    ///
    /// Returns proof-backed [`AddressInfos`] for the funded addresses.
    async fn top_up(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error>;
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> TopUpAddress<S> for BTreeMap<PlatformAddress, Option<Credits>> {
    async fn top_up(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error> {
        if self.is_empty() {
            return Err(Error::from(TransitionNoOutputsError::new()));
        }

        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let state_transition = create_address_funding_from_asset_lock_transition(
            asset_lock_proof,
            asset_lock_private_key.inner.as_ref(),
            BTreeMap::new(),
            self.clone(),
            fee_strategy,
            signer,
            user_fee_increase,
            sdk,
        )?;

        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
        let st_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(sdk, settings)
            .await?;
        match st_result {
            StateTransitionProofResult::VerifiedAddressInfos(address_infos) => {
                let requested: BTreeSet<PlatformAddress> =
                    self.keys().copied().collect::<BTreeSet<_>>();
                let received: BTreeSet<PlatformAddress> =
                    address_infos.keys().copied().collect::<BTreeSet<_>>();
                if requested != received {
                    return Err(Error::InvalidProvedResponse(format!(
                        "proof returned different addresses. requested: {:?}, received: {:?}",
                        requested, received
                    )));
                }
                let infos: AddressInfos = address_infos
                    .into_iter()
                    .map(|(address, maybe_info)| {
                        let info = maybe_info.map(|(nonce, balance)| AddressInfo {
                            address,
                            nonce,
                            balance,
                        });
                        (address, info)
                    })
                    .collect();
                Ok(infos)
            }
            other => Err(Error::InvalidProvedResponse(format!(
                "address info proof was expected for {:?}, but received {:?}",
                state_transition, other
            ))),
        }
    }
}

fn create_address_funding_from_asset_lock_transition<S: Signer<PlatformAddress>>(
    asset_lock_proof: AssetLockProof,
    asset_lock_private_key: &[u8],
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    outputs: BTreeMap<PlatformAddress, Option<Credits>>,
    fee_strategy: AddressFundsFeeStrategy,
    signer: &S,
    user_fee_increase: UserFeeIncrease,
    sdk: &Sdk,
) -> Result<StateTransition, ProtocolError> {
    AddressFundingFromAssetLockTransition::try_from_asset_lock_with_signer(
        asset_lock_proof,
        asset_lock_private_key,
        inputs,
        outputs,
        fee_strategy,
        signer,
        user_fee_increase,
        sdk.version(),
    )
}
