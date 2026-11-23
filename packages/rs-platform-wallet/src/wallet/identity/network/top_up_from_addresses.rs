//! Top-up an identity from platform addresses.

use std::collections::BTreeMap;
use std::future::Future;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::prelude::{AddressNonce, Identifier};

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::dash_platform_queries::transition::validation::ensure_valid_state_transition_structure;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::broadcast_request::checked_broadcast_request;
use dash_sdk::platform::transition::fetch_inputs_with_nonce;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::Fetch;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::state_transition::identity_topup_from_addresses_transition::methods::IdentityTopUpFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;

use dash_sdk::query_types::{AddressInfo, AddressInfos};

use crate::broadcast_outcome::{broadcast_tracking_delivery, BroadcastDelivery};
use crate::error::{preserve_signer_key_unavailable_or, PlatformWalletError};
use crate::BlockTime;
use tokio::sync::RwLockReadGuard;

use super::*;

#[cfg(test)]
mod external_identity_tests;

// ---------------------------------------------------------------------------
// Top-up from platform addresses
// ---------------------------------------------------------------------------

/// Which identities an address top-up accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TopUpScope {
    /// Any identity Platform knows, the wallet's own or someone else's.
    AnyIdentity,
    /// Only an identity the identity manager holds, as the deprecated
    /// [`PlatformWallet::top_up_from_addresses`] does.
    ///
    /// [`PlatformWallet::top_up_from_addresses`]:
    /// crate::wallet::PlatformWallet::top_up_from_addresses
    ManagedOnly,
}

impl IdentityWallet {
    /// Build and sign the top-up of an identity from platform address
    /// balances, whoever owns the identity; [`Self::send_signed_top_up`]
    /// sends it. Address nonces are looked up automatically.
    ///
    /// The identity is fetched from Platform (proved), never read from the
    /// identity manager, and is never added to it: the manager's own
    /// identities are written back by [`Self::record_top_up_balance`] once
    /// the top-up is confirmed. The balance the top-up must grow is the one
    /// in that fetched identity, read before anything is signed, so a cached
    /// balance above Platform's cannot make an executed top-up look
    /// unconfirmed.
    ///
    /// Crate-private: neither step holds the lifecycle gate, writes the
    /// managed balance back or reconciles the spent addresses. Callers
    /// outside the crate use
    /// [`PlatformWallet::top_up_identity_from_addresses`], which signs
    /// outside the lifecycle gate, sends under it, writes back and
    /// reconciles. The sign step reads the wallet's state only through this
    /// handle's generation: a wallet removed or re-created since the handle
    /// was taken is [`PlatformWalletError::WalletRemovedBeforeSend`] before
    /// the host is asked to sign, checked once the identity and the address
    /// nonces are read and, with [`TopUpScope::ManagedOnly`], when the
    /// identity manager is read.
    ///
    /// # Errors
    ///
    /// Before any network I/O, funding rows that are empty or whose sum for
    /// one address overflows are [`PlatformWalletError::InvalidParameter`].
    /// With [`TopUpScope::ManagedOnly`], an identity the identity manager
    /// does not hold is [`PlatformWalletError::IdentityNotFound`] before
    /// Platform is asked, and Platform returning no identity (not yet
    /// visible, or lagging) is
    /// [`PlatformWalletError::IdentityBalanceUnavailable`]; with
    /// [`TopUpScope::AnyIdentity`] that is
    /// [`PlatformWalletError::IdentityNotFound`]. A failed lookup is
    /// [`PlatformWalletError::IdentityLookupFailed`]. All of them before
    /// anything is signed or broadcast.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to top up.
    /// * `inputs` - Funding rows: platform addresses and the credits to
    ///   spend from each. Rows naming the same address are summed; a sum
    ///   that overflows is refused as an invalid parameter.
    /// * `address_signer` - Produces ECDSA signatures for the input
    ///   [`PlatformAddress`]es. Construction is the caller's concern —
    ///   seed-backed, hardware, FFI trampoline, whatever — the wallet
    ///   struct carries no key material itself.
    /// * `scope` - Which identities the top-up accepts; see [`TopUpScope`].
    ///
    /// [`PlatformWallet::top_up_identity_from_addresses`]:
    /// crate::wallet::PlatformWallet::top_up_identity_from_addresses
    pub(crate) async fn sign_top_up_from_addresses<S, I>(
        &self,
        identity_id: &Identifier,
        inputs: I,
        address_signer: &S,
        settings: Option<PutSettings>,
        scope: TopUpScope,
    ) -> Result<SignedTopUp, PlatformWalletError>
    where
        S: Signer<PlatformAddress> + Send + Sync,
        I: IntoIterator<Item = (PlatformAddress, Credits)> + Send,
    {
        let inputs = funding_inputs(inputs)?;
        if scope == TopUpScope::ManagedOnly {
            let wm = self.wallet_manager.read().await;
            // Read outside the lifecycle gate: a wallet removed or
            // re-created since the liveness check is refused here, before
            // its replacement's state is read or anything is signed.
            let info = self.current_wallet_info(&wm).map_err(|_| self.removed())?;
            if info.identity_manager.identity(identity_id).is_none() {
                return Err(PlatformWalletError::IdentityNotFound(*identity_id));
            }
        }
        let identity = Identity::fetch(&self.sdk, *identity_id)
            .await
            .map_err(|source| PlatformWalletError::IdentityLookupFailed {
                identity_id: *identity_id,
                source,
            })?
            .ok_or(match scope {
                TopUpScope::AnyIdentity => PlatformWalletError::IdentityNotFound(*identity_id),
                // The wallet's own identity, not visible on this node yet:
                // retryable against an up-to-date node.
                TopUpScope::ManagedOnly => {
                    PlatformWalletError::IdentityBalanceUnavailable(*identity_id)
                }
            })?;

        self.sign_top_up(
            &identity,
            identity.balance(),
            inputs,
            address_signer,
            settings,
        )
        .await
    }

    /// The refusal of a top-up whose wallet was removed, or re-created under
    /// its id, since this handle was taken.
    fn removed(&self) -> PlatformWalletError {
        PlatformWalletError::WalletRemovedBeforeSend(hex::encode(self.wallet_id))
    }

    /// Write a confirmed top-up's proved balance (`new_balance` at the
    /// proof's `metadata`) back to the identity manager when it holds
    /// `identity_id`, and return the balance the caller reports: the one the
    /// manager keeps for a managed identity (a failed cache write stays
    /// pending for a later retry), `new_balance` otherwise. An identity the
    /// manager does not hold is left out of it.
    ///
    /// `_lifecycle` is the wallet generation's held lifecycle gate: it must
    /// be held across this manager write and the caller's address
    /// reconciliation.
    pub(crate) async fn record_top_up_balance(
        &self,
        identity_id: &Identifier,
        new_balance: Credits,
        metadata: ResponseMetadata,
        _lifecycle: &RwLockReadGuard<'_, ()>,
    ) -> Credits {
        let mut wm = self.wallet_manager.write().await;
        match self
            .current_wallet_info_mut(&mut wm)
            .ok()
            .and_then(|info| info.identity_manager.managed_identity_mut(identity_id))
        {
            Some(managed) => managed.persist_confirmed_balance(
                new_balance,
                BlockTime::from(metadata),
                &self.persister,
            ),
            None => new_balance,
        }
    }

    /// Build and sign the top-up of `identity` from `inputs`, mapping the SDK
    /// error the way every address top-up reports it. Nothing is sent.
    ///
    /// A signed transition over `max_state_transition_size` (a host signer's
    /// witness can be any size) is refused here with the SDK's typed
    /// `StateTransitionMaxSizeExceededError`, by the same check the SDK's
    /// broadcast makes before its first dispatch
    /// ([`checked_broadcast_request`]): the SDK would refuse it unsent, and
    /// the send could not tell that refusal from an endpoint's answer.
    async fn sign_top_up<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        identity: &Identity,
        balance_before: Credits,
        inputs: BTreeMap<PlatformAddress, Credits>,
        address_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<SignedTopUp, PlatformWalletError> {
        let inputs = fetch_inputs_with_nonce(&self.sdk, &inputs)
            .await
            .map_err(top_up_error)?
            .into_iter()
            .map(|(address, (nonce, credits))| {
                let next = nonce.checked_add(1).ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(format!(
                        "{address:?} has used every nonce and can fund nothing more"
                    ))
                })?;
                Ok((address, (next, credits)))
            })
            .collect::<Result<BTreeMap<_, _>, PlatformWalletError>>()?;
        let submitted = inputs
            .iter()
            .map(|(address, (nonce, _))| (*address, *nonce))
            .collect();
        // A point-in-time check after the identity and nonce reads: a wallet
        // removed or re-created during them asks the host for no signature.
        // The decisive check runs under the lifecycle gate once it signed.
        {
            let wm = self.wallet_manager.read().await;
            self.current_wallet_info(&wm).map_err(|_| self.removed())?;
        }
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();
        let state_transition = IdentityTopUpFromAddressesTransition::try_from_inputs_with_signer(
            identity,
            inputs,
            address_signer,
            user_fee_increase,
            self.sdk.version(),
            None,
        )
        .await
        .map_err(|e| top_up_error(e.into()))?;
        ensure_valid_state_transition_structure(&state_transition, self.sdk.version())
            .map_err(|e| top_up_error(e.into()))?;
        checked_broadcast_request(&state_transition, self.sdk.version()).map_err(top_up_error)?;
        Ok(SignedTopUp {
            identity_id: identity.id(),
            balance_before,
            submitted,
            state_transition,
            settings,
        })
    }

    /// Broadcast a signed top-up and wait for its result.
    ///
    /// Returns the proof-attested post-spend `AddressInfos`, the identity's
    /// proved new balance and the proof's metadata. The composite
    /// (`PlatformWallet`) writes a managed identity's balance back
    /// ([`Self::record_top_up_balance`]) and reconciles the spent
    /// platform-address balances, routing the returned `AddressInfos`
    /// through the platform-address wallet's shared reconciliation seam.
    ///
    /// The broadcast keeps what every dispatch proved
    /// ([`broadcast_tracking_delivery`]) and the outcome is settled by
    /// [`settle_top_up`]: only a top-up refused before anything was sent is
    /// an ordinary error; once a dispatch may have reached a node, an
    /// endpoint's refusal included (an address-nonce refusal too), the result
    /// wait decides, and anything short of a proof bound to this transition
    /// is the SDK's unconfirmed outcome.
    ///
    /// The result proof of this transition authenticates only the state it
    /// affects (the identity's balance and the input addresses as of the
    /// proof's block), not that this transition executed. The result stands
    /// only once the proof shows every input address at or past the nonce
    /// this transition was signed with and the identity balance above
    /// `balance_before`, Platform's balance of the identity read before
    /// signing ([`bind_to_submitted_nonces`]); otherwise the top-up is
    /// reported unconfirmed and nothing is reconciled from it.
    pub(crate) async fn send_signed_top_up(
        &self,
        signed: SignedTopUp,
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError> {
        let SignedTopUp {
            identity_id,
            balance_before,
            submitted,
            state_transition,
            settings,
        } = signed;
        let delivery = broadcast_tracking_delivery(&self.sdk, &state_transition, settings).await;
        settle_top_up(&submitted, balance_before, delivery, || async {
            let (result, metadata) = state_transition
                .wait_for_affected_state_with_metadata::<StateTransitionProofResult>(
                    &self.sdk, settings,
                )
                .await?;
            proved_top_up(identity_id, &submitted, result)
                .map(|(infos, balance)| (infos, balance, metadata))
        })
        .await
    }
}

/// An identity top-up from platform addresses, built and signed and not yet
/// sent: the output of [`IdentityWallet::sign_top_up_from_addresses`], the
/// input of
/// [`IdentityWallet::send_signed_top_up`]. The split lets the composite sign
/// (the host's signer may prompt) outside the wallet generation's lifecycle
/// gate and send under it.
pub(crate) struct SignedTopUp {
    identity_id: Identifier,
    /// Platform's balance of the identity read before signing, the baseline
    /// the proof must show growth from ([`bind_to_submitted_nonces`]).
    balance_before: Credits,
    /// Each input address with the nonce it was signed with.
    submitted: BTreeMap<PlatformAddress, AddressNonce>,
    state_transition: StateTransition,
    settings: Option<PutSettings>,
}

/// An SDK error of an address top-up before anything was sent, as the
/// wallet reports it: kept typed under [`PlatformWalletError::Sdk`] (a
/// signer-key marker included). No such error is an address-nonce refusal:
/// a refusal is an endpoint's answer, which [`settle_top_up`] handles.
fn top_up_error(e: dash_sdk::Error) -> PlatformWalletError {
    preserve_signer_key_unavailable_or(e, PlatformWalletError::Sdk)
}

/// What a signed top-up's broadcast (`delivery`) and result wait (`wait`,
/// run only when a dispatch may have reached a node) came to, by the rule
/// every address and credit spend of this crate follows
/// ([`BroadcastDelivery`]).
///
/// Only a transition that never left the wallet
/// ([`BroadcastDelivery::NeverDelivered`]: refused before anything was sent)
/// is an ordinary error: nothing holds its bytes but the wallet. Once a
/// dispatch may have reached a node, an endpoint's refusal of the broadcast
/// is no proof, whatever it says: an address-nonce refusal can answer a
/// transition that endpoint kept or forwarded, or whose nonce an earlier
/// dispatch already used, so it is never reported as
/// [`PlatformWalletError::AddressNonceMismatch`] (safe to retry). The
/// result wait decides: a proof bound to this transition
/// ([`bind_to_submitted_nonces`]) stands, and every other outcome, a failed
/// wait or a verdict on it included, is the SDK's unconfirmed outcome.
async fn settle_top_up<W, Fut>(
    submitted: &BTreeMap<PlatformAddress, AddressNonce>,
    balance_before: Credits,
    delivery: BroadcastDelivery,
    wait: W,
) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError>
where
    W: FnOnce() -> Fut,
    Fut: Future<Output = Result<(AddressInfos, Credits, ResponseMetadata), dash_sdk::Error>>,
{
    match delivery {
        BroadcastDelivery::Admitted => {}
        BroadcastDelivery::NeverDelivered(e) => return Err(top_up_error(e).before_dispatch()),
        BroadcastDelivery::MaybeDelivered(e) => {
            tracing::warn!(
                error = %e,
                "address top-up broadcast may have been delivered; falling through to the \
                 result wait"
            );
        }
    }
    let confirmed = wait().await.map_err(|e| {
        PlatformWalletError::Sdk(if e.is_state_transition_result_unconfirmed() {
            e
        } else {
            dash_sdk::Error::StateTransitionResultUnconfirmed(Box::new(e))
        })
    })?;
    bind_to_submitted_nonces(submitted, balance_before, confirmed)
}

/// The proved identity balance and input addresses of a top-up's result
/// proof: the proof must name `identity_id`, carry its balance and show
/// exactly the `submitted` input addresses.
fn proved_top_up(
    identity_id: Identifier,
    submitted: &BTreeMap<PlatformAddress, AddressNonce>,
    result: StateTransitionProofResult,
) -> Result<(AddressInfos, Credits), dash_sdk::Error> {
    let StateTransitionProofResult::VerifiedIdentityWithAddressInfos(identity, addresses) = result
    else {
        return Err(dash_sdk::Error::InvalidProvedResponse(format!(
            "identity top-up proof was expected for {identity_id}, but received {result:?}"
        )));
    };
    if identity.id != identity_id {
        return Err(dash_sdk::Error::InvalidProvedResponse(format!(
            "proof returned identity {} but {identity_id} was topped up",
            identity.id
        )));
    }
    if !addresses.keys().eq(submitted.keys()) {
        return Err(dash_sdk::Error::InvalidProvedResponse(
            "proof returned different addresses than the top-up spent".to_string(),
        ));
    }
    let balance = identity.balance.ok_or_else(|| {
        dash_sdk::Error::InvalidProvedResponse(
            "identity proof did not include updated balance".to_string(),
        )
    })?;
    let infos = AddressInfos::from_iter(addresses.into_iter().map(|(address, info)| {
        (
            address,
            info.map(|(nonce, balance)| AddressInfo {
                address,
                nonce,
                balance,
            }),
        )
    }));
    Ok((infos, balance))
}

/// The input map of an address-funded top-up from its funding rows: rows
/// naming the same address are one input of their sum. A sum that overflows
/// is refused with [`PlatformWalletError::InvalidParameter`], and so are no
/// rows at all: every route calls this before any network I/O, so an empty
/// top-up is the caller's mistake whatever the identity lookup would say.
pub(crate) fn funding_inputs(
    rows: impl IntoIterator<Item = (PlatformAddress, Credits)>,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    let mut inputs = BTreeMap::new();
    for (address, credits) in rows {
        let sum: &mut Credits = inputs.entry(address).or_default();
        *sum = sum.checked_add(credits).ok_or_else(|| {
            PlatformWalletError::InvalidParameter(format!(
                "the funding amounts for {address:?} overflow"
            ))
        })?;
    }
    if inputs.is_empty() {
        return Err(PlatformWalletError::InvalidParameter(
            "no funding rows were provided".to_string(),
        ));
    }
    Ok(inputs)
}

/// The submitted top-up's result once its proof is bound to this
/// transition's address nonces (`submitted`) and to the identity balance
/// it started from (`balance_before`).
///
/// An address nonce is used exactly once and only in order (Platform takes
/// stored + 1), and only the holder of the input addresses' keys (the
/// signer seam this top-up was signed through) can sign for them. A proof
/// showing every input address at or past the submitted nonce shows those
/// nonces consumed: this top-up executed, or a transition signed by the same
/// seam with the same nonces did and this one never can. Either way it can
/// no longer execute later. A proof showing an input address before its
/// submitted nonce (or not at all) is a snapshot that does not reflect this
/// top-up, which may still execute.
///
/// Used nonces alone do not show that the identity was paid: the other
/// transition may have spent them elsewhere. The result stands only when
/// the proved identity balance is also above `balance_before`; a balance at
/// or below it is the snapshot of a top-up that did not reach the identity
/// (or of an identity that spent at least the top-up amount meanwhile, or
/// of an identity in debt that the top-up only repaid, its balance staying
/// at 0; both read the same way and err on the safe side). Every refusal is
/// the SDK's unconfirmed outcome,
/// never done: the host must not pay again.
///
/// Residual limit: the two checks bind the proof to this transition's
/// nonces and to some credit of the identity, not to this transition's
/// execution. A competing transition signed by the same signer seam with
/// the same nonces (which spends them elsewhere) together with an unrelated
/// credit to the same identity in the same window satisfies both, and an
/// unexecuted top-up is then reported done. Closing that fully needs an
/// execution-bound proof of this transition. The wallet does not serialise
/// address spends per input address in-process: the address top-up holds
/// only the wallet generation's shared lifecycle gate (a read guard), and
/// the per-wallet `shield_guard` single-flight covers the shield-class
/// operations, not this path, so two concurrent spends from one address
/// in one process can be signed with the same nonce.
fn bind_to_submitted_nonces(
    submitted: &BTreeMap<PlatformAddress, AddressNonce>,
    balance_before: Credits,
    confirmed: (AddressInfos, Credits, ResponseMetadata),
) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError> {
    if confirmed.1 <= balance_before {
        return Err(unconfirmed(format!(
            "the identity top-up proof shows the identity balance at {}, not above the {} it \
             started from",
            confirmed.1, balance_before
        )));
    }
    for (address, nonce) in submitted {
        let proved = confirmed.0.get(address).and_then(|info| info.as_ref());
        if proved.is_none_or(|info| info.nonce < *nonce) {
            return Err(unconfirmed(format!(
                "the identity top-up proof shows {address:?} at nonce {:?}, before the \
                 submitted nonce {nonce}",
                proved.map(|info| info.nonce)
            )));
        }
    }
    Ok(confirmed)
}

/// The SDK's unconfirmed outcome for a proof that does not show this top-up.
fn unconfirmed(reason: String) -> PlatformWalletError {
    PlatformWalletError::Sdk(dash_sdk::Error::StateTransitionResultUnconfirmed(Box::new(
        dash_sdk::Error::ExecutionNotProved(reason),
    )))
}

#[cfg(test)]
mod nonce_binding_tests {
    use super::*;
    use crate::broadcast_outcome::track_delivery;
    use dpp::consensus::state::address_funds::AddressInvalidNonceError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::StateTransition;

    const FIRST: PlatformAddress = PlatformAddress::P2pkh([1; 20]);
    const SECOND: PlatformAddress = PlatformAddress::P2pkh([2; 20]);

    fn proved(nonces: &[(PlatformAddress, Option<AddressNonce>)]) -> AddressInfos {
        AddressInfos::from_iter(nonces.iter().map(|(address, nonce)| {
            (
                *address,
                nonce.map(|nonce| AddressInfo {
                    address: *address,
                    nonce,
                    balance: 5,
                }),
            )
        }))
    }

    const BALANCE_BEFORE: Credits = 700;

    fn bind_with_balance(
        proved_nonces: &[(PlatformAddress, Option<AddressNonce>)],
        proved_balance: Credits,
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError> {
        let submitted = BTreeMap::from([(FIRST, 4), (SECOND, 9)]);
        bind_to_submitted_nonces(
            &submitted,
            BALANCE_BEFORE,
            (
                proved(proved_nonces),
                proved_balance,
                ResponseMetadata::default(),
            ),
        )
    }

    fn bind(
        proved_nonces: &[(PlatformAddress, Option<AddressNonce>)],
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError> {
        bind_with_balance(proved_nonces, 777)
    }

    /// A proof that shows an input address before the nonce this top-up
    /// was signed with is a snapshot from before it executed: the top-up
    /// may still execute, so it is reported unconfirmed, not done.
    #[test]
    fn should_report_a_snapshot_before_the_submitted_nonces_as_unconfirmed() {
        for proved_nonces in [
            [(FIRST, Some(3)), (SECOND, Some(8))],
            [(FIRST, Some(4)), (SECOND, Some(8))],
            [(FIRST, Some(4)), (SECOND, None)],
        ] {
            let result = bind(&proved_nonces);
            assert!(
                matches!(
                    &result,
                    Err(PlatformWalletError::Sdk(e)) if e.is_state_transition_result_unconfirmed()
                ),
                "{proved_nonces:?}: {result:?}"
            );
        }
    }

    /// Every input address at (or past) the nonce this top-up was signed
    /// with: the nonces it used are consumed, so the result stands.
    #[test]
    fn should_keep_a_result_whose_proof_shows_the_submitted_nonces_used() {
        for proved_nonces in [
            [(FIRST, Some(4)), (SECOND, Some(9))],
            [(FIRST, Some(5)), (SECOND, Some(9))],
        ] {
            let (_, balance, _) = bind(&proved_nonces).expect("bound to the submitted nonces");
            assert_eq!(balance, 777);
        }
    }

    /// The submitted nonces are used but the identity balance did not grow:
    /// another transition this wallet signed with the same nonces took them
    /// and paid elsewhere. This top-up did not reach the identity, so the
    /// result is unconfirmed, not done.
    #[test]
    fn should_report_used_nonces_without_a_higher_identity_balance_as_unconfirmed() {
        for proved_balance in [BALANCE_BEFORE, BALANCE_BEFORE - 1] {
            let result = bind_with_balance(&[(FIRST, Some(4)), (SECOND, Some(9))], proved_balance);
            assert!(
                matches!(
                    &result,
                    Err(PlatformWalletError::Sdk(e)) if e.is_state_transition_result_unconfirmed()
                ),
                "{proved_balance}: {result:?}"
            );
        }
    }

    /// The baseline is Platform's balance read before signing, not a cached
    /// one: with the cache above Platform's value (credits spent from
    /// another device, not yet synced), a top-up that grew the identity
    /// above Platform's balance stands, where the stale cache as baseline
    /// would have reported it unconfirmed.
    #[test]
    fn should_keep_growth_over_platforms_balance_that_a_stale_cache_hides() {
        const STALE_CACHE: Credits = 800;
        const PROVED: Credits = 750;
        let submitted = BTreeMap::from([(FIRST, 4), (SECOND, 9)]);
        let proof = || {
            (
                proved(&[(FIRST, Some(4)), (SECOND, Some(9))]),
                PROVED,
                ResponseMetadata::default(),
            )
        };

        let (_, balance, _) = bind_to_submitted_nonces(&submitted, BALANCE_BEFORE, proof())
            .expect("growth over Platform's balance stands");
        assert_eq!(balance, PROVED);
        let against_cache = bind_to_submitted_nonces(&submitted, STALE_CACHE, proof());
        assert!(
            matches!(
                &against_cache,
                Err(PlatformWalletError::Sdk(e)) if e.is_state_transition_result_unconfirmed()
            ),
            "{against_cache:?}"
        );
    }

    /// The submitted nonces as [`bind`] signs them, as the inputs of a
    /// signed address spend.
    fn spend() -> StateTransition {
        AddressFundsTransferTransitionV0 {
            inputs: BTreeMap::from([(FIRST, (4, 1_000)), (SECOND, (9, 1_000))]),
            ..Default::default()
        }
        .into()
    }

    /// An endpoint's refusal of `FIRST` at the submitted nonce 4, expecting
    /// `expected`.
    fn nonce_refusal(expected: AddressNonce) -> dash_sdk::Error {
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            ConsensusError::StateError(StateError::AddressInvalidNonceError(
                AddressInvalidNonceError::new(FIRST, 4, expected),
            )),
        )))
    }

    /// An endpoint answers the FIRST broadcast dispatch with an address-nonce
    /// refusal, the nonce already used (expected 5) or ahead of that node
    /// (expected 3). The answer is unauthenticated and the nonce may have
    /// been used by this very transition, so the top-up is never reported as
    /// a safe-to-retry nonce mismatch: it waits for its result, reports a
    /// failed wait as unconfirmed, and keeps a proof that shows it executed.
    #[tokio::test]
    async fn should_not_report_a_nonce_refusal_of_the_first_broadcast_as_safe_to_retry() {
        let submitted = BTreeMap::from([(FIRST, 4), (SECOND, 9)]);
        let transition = spend();
        for expected in [5, 3] {
            let delivery = || {
                track_delivery(&transition, 2, || {
                    std::future::ready(Err(nonce_refusal(expected)))
                })
            };

            let mut waited = false;
            let failed_wait = settle_top_up(&submitted, BALANCE_BEFORE, delivery().await, || {
                waited = true;
                std::future::ready(Err(nonce_refusal(expected)))
            })
            .await;
            assert!(waited, "{expected}: the result wait must run");
            assert!(
                matches!(
                    &failed_wait,
                    Err(PlatformWalletError::Sdk(e)) if e.is_state_transition_result_unconfirmed()
                ),
                "{expected}: {failed_wait:?}"
            );

            let proved = settle_top_up(&submitted, BALANCE_BEFORE, delivery().await, || {
                std::future::ready(Ok((
                    proved(&[(FIRST, Some(4)), (SECOND, Some(9))]),
                    777,
                    ResponseMetadata::default(),
                )))
            })
            .await;
            assert!(matches!(&proved, Ok((_, 777, _))), "{expected}: {proved:?}");
        }
    }

    /// A top-up refused before anything was sent never left the wallet: it
    /// is an ordinary error and no result is waited for.
    #[tokio::test]
    async fn should_report_a_top_up_refused_before_sending_without_waiting() {
        use dash_sdk::dapi_client::DapiClientError;

        let submitted = BTreeMap::from([(FIRST, 4), (SECOND, 9)]);
        let result = settle_top_up(
            &submitted,
            BALANCE_BEFORE,
            BroadcastDelivery::NeverDelivered(dash_sdk::Error::DapiClientError(
                DapiClientError::NoAvailableAddresses,
            )),
            || -> std::future::Ready<Result<_, dash_sdk::Error>> {
                panic!("a top-up that was never sent must not wait")
            },
        )
        .await;
        let error = result.expect_err("a top-up never sent must fail");
        // The SDK error alone would read as possibly sent; the emitter knows
        // nothing left, so the host is told so instead of waiting.
        assert_eq!(
            error.payment_delivery(),
            crate::error::PaymentDelivery::NotSent,
            "{error:?}"
        );
        assert!(
            matches!(
                error.dispatched_source(),
                PlatformWalletError::Sdk(e) if !e.is_state_transition_result_unconfirmed()
            ),
            "{error:?}"
        );
    }
}
