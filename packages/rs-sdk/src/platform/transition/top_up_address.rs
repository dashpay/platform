use std::collections::{BTreeMap, BTreeSet};

use super::address_inputs::collect_address_infos_from_proof;
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
use drive_proof_verifier::types::AddressInfos;

/// Trait for topping up Platform addresses using various funding sources.
#[async_trait::async_trait]
pub trait TopUpAddress<S: Signer<PlatformAddress>> {
    /// Tops up addresses using a raw private key for the asset-lock proof.
    ///
    /// Returns proof-backed [`AddressInfos`] for the funded addresses,
    /// paired with the proof's committed block height — the balance
    /// height pin ([`AddressFunds::as_of_height`]) callers that persist
    /// the absolutes must record.
    ///
    /// [`AddressFunds::as_of_height`]:
    /// crate::platform::address_sync::AddressFunds::as_of_height
    ///
    /// Prefer [`Self::top_up_with_signers`] when the asset-lock private
    /// key lives outside Rust (Swift / hardware wallet / HSM): the
    /// `_with_signers` variant routes asset-lock signing through an
    /// external [`dpp::key_wallet::signer::Signer`] so no raw private
    /// key crosses the FFI boundary.
    async fn top_up(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, u64), Error>;

    /// Top up addresses with an external asset-lock signer.
    ///
    /// `signer` (the trait's `S: Signer<PlatformAddress>`) signs each
    /// per-input `AddressWitness`; `asset_lock_signer` produces the
    /// outer state-transition ECDSA signature for the key at
    /// `asset_lock_proof_path` — atomically deriving, signing, and
    /// zeroising inside the signer's trust boundary. This is the
    /// signing path used by hosts that hold their private keys outside
    /// Rust (the iOS Swift SDK, hardware wallets, remote signers).
    ///
    /// `settings.user_fee_increase` is threaded straight through to
    /// the transition builder. It both affects fee accounting AND
    /// changes the ST's signable bytes, which the upstream CL-height
    /// retry path in `platform-wallet` relies on to bypass
    /// Tenderdash's invalid-tx hash cache
    /// (`keep-invalid-txs-in-cache = true` in dashmate's
    /// mainnet/testnet templates). `None` / unset = unaltered fees.
    #[cfg(feature = "core_key_wallet")]
    #[allow(clippy::too_many_arguments)]
    async fn top_up_with_signers<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, u64), Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync;
}

pub type AddressWithBalance = (PlatformAddress, Option<Credits>);
pub type AddressesWithBalances = BTreeMap<PlatformAddress, Option<Credits>>;

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> TopUpAddress<S> for AddressWithBalance
where
    BTreeMap<PlatformAddress, Option<Credits>>: TopUpAddress<S>,
{
    async fn top_up(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, u64), Error> {
        BTreeMap::from([(self.0, self.1)])
            .top_up(
                sdk,
                asset_lock_proof,
                asset_lock_private_key,
                fee_strategy,
                signer,
                settings,
            )
            .await
    }

    #[cfg(feature = "core_key_wallet")]
    #[allow(clippy::too_many_arguments)]
    async fn top_up_with_signers<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, u64), Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
    {
        BTreeMap::from([(self.0, self.1)])
            .top_up_with_signers(
                sdk,
                asset_lock_proof,
                asset_lock_proof_path,
                fee_strategy,
                signer,
                asset_lock_signer,
                settings,
            )
            .await
    }
}

#[async_trait::async_trait]
impl<S: Signer<PlatformAddress>> TopUpAddress<S> for AddressesWithBalances {
    async fn top_up(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, u64), Error> {
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
        )
        .await?;

        broadcast_and_collect_address_infos(self, state_transition, sdk, settings).await
    }

    #[cfg(feature = "core_key_wallet")]
    #[allow(clippy::too_many_arguments)]
    async fn top_up_with_signers<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, u64), Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
    {
        if self.is_empty() {
            return Err(Error::from(TransitionNoOutputsError::new()));
        }

        // Pull `user_fee_increase` from settings *before* the
        // broadcast call. The upstream CL-height retry path
        // (`platform-wallet::wallet::asset_lock::orchestration::submit_with_cl_height_retry`)
        // bumps this value between attempts to change the ST's
        // signable bytes — if we silently dropped it here, retries
        // would hash identically and get cached out by Tenderdash.
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let state_transition =
            AddressFundingFromAssetLockTransition::try_from_asset_lock_with_signers::<S, AS>(
                asset_lock_proof,
                asset_lock_proof_path,
                BTreeMap::new(),
                self.clone(),
                fee_strategy,
                signer,
                asset_lock_signer,
                user_fee_increase,
                sdk.version(),
            )
            .await?;

        broadcast_and_collect_address_infos(self, state_transition, sdk, settings).await
    }
}

/// Broadcast the address-funding ST and convert the proof into the
/// `AddressInfos` map, paired with the proof's committed block height.
/// Shared between the legacy private-key path and the new signer-pair
/// path — both flows want the same proof-shape guarantee and the same
/// expected-addresses cross-check.
///
/// The returned height is the balances' height pin (see
/// `crate::platform::address_sync::AddressFunds::as_of_height`): callers
/// that persist these absolutes must record it so later balance-change
/// deltas at or below it are not re-applied on top.
async fn broadcast_and_collect_address_infos(
    expected: &AddressesWithBalances,
    state_transition: StateTransition,
    sdk: &Sdk,
    settings: Option<PutSettings>,
) -> Result<(AddressInfos, u64), Error> {
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
    let (st_result, metadata) = state_transition
        .broadcast_and_wait_with_metadata::<StateTransitionProofResult>(sdk, settings)
        .await?;
    match st_result {
        StateTransitionProofResult::VerifiedAddressInfos(address_infos) => {
            let expected_addresses = expected
                .keys()
                .copied()
                .collect::<BTreeSet<PlatformAddress>>();
            collect_address_infos_from_proof(address_infos, &expected_addresses)
                .map(|infos| (infos, metadata.height))
        }
        other => Err(Error::InvalidProvedResponse(format!(
            "address info proof was expected for {:?}, but received {:?}",
            state_transition, other
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
async fn create_address_funding_from_asset_lock_transition<S: Signer<PlatformAddress>>(
    asset_lock_proof: AssetLockProof,
    asset_lock_private_key: &[u8],
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    outputs: BTreeMap<PlatformAddress, Option<Credits>>,
    fee_strategy: AddressFundsFeeStrategy,
    signer: &S,
    user_fee_increase: UserFeeIncrease,
    sdk: &Sdk,
) -> Result<StateTransition, ProtocolError> {
    AddressFundingFromAssetLockTransition::try_from_asset_lock_with_signer_and_private_key(
        asset_lock_proof,
        asset_lock_private_key,
        inputs,
        outputs,
        fee_strategy,
        signer,
        user_fee_increase,
        sdk.version(),
    )
    .await
}
