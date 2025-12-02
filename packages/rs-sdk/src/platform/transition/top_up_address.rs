use std::collections::{BTreeMap, BTreeSet};

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use crate::platform::transfer::TransferInput;
use crate::platform::FetchMany;
use crate::{Error, Sdk};
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
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
    async fn top_up<F>(
        &self,
        sdk: &Sdk,
        funding: F,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error>
    where
        F: TryInto<TransferInput> + Send,
        <F as TryInto<TransferInput>>::Error: ToString;
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> TopUpAddress<S> for BTreeMap<PlatformAddress, Credits> {
    async fn top_up<F>(
        &self,
        sdk: &Sdk,
        funding: F,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<AddressInfos, Error>
    where
        F: TryInto<TransferInput> + Send,
        <F as TryInto<TransferInput>>::Error: ToString,
    {
        if self.is_empty() {
            return Err(Error::from(TransitionNoOutputsError::new()));
        }

        let funding_source = funding
            .try_into()
            .map_err(|err| Error::Generic(err.to_string()))?;

        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let state_transition = match &funding_source {
            TransferInput::AssetLock {
                asset_lock_proof,
                asset_lock_private_key,
            } => create_address_funding_from_asset_lock_transition(
                asset_lock_proof.clone(),
                asset_lock_private_key.inner.as_ref(),
                BTreeMap::new(),
                self.clone(),
                fee_strategy,
                signer,
                user_fee_increase,
                sdk,
            )?,
            TransferInput::Addresses { .. } | TransferInput::AddressesWithNonce { .. } => {
                return Err(Error::InvalidCreditTransfer(
                    "Address top up requires an asset lock funding source".to_string(),
                ))
            }
            TransferInput::Identity(_) => {
                return Err(Error::InvalidCreditTransfer(
                    "Identity funding cannot be used for address top ups".to_string(),
                ))
            }
        };

        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(sdk, settings)
            .await?;

        let addresses_to_refresh: BTreeSet<PlatformAddress> =
            self.keys().copied().collect::<BTreeSet<_>>();
        AddressInfo::fetch_many(sdk, addresses_to_refresh).await
    }
}

fn create_address_funding_from_asset_lock_transition<S: Signer<PlatformAddress>>(
    asset_lock_proof: AssetLockProof,
    asset_lock_private_key: &[u8],
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    outputs: BTreeMap<PlatformAddress, Credits>,
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
