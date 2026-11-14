//! Top-up an identity from platform addresses.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use dpp::address_funds::AddressWitness;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::Identity;
use dpp::prelude::{AddressNonce, Identifier};

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::platform::transition::fetch_inputs_with_nonce;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity_from_addresses::TopUpIdentityFromAddressesWithMetadata;
use dash_sdk::platform::Fetch;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use tokio::sync::OwnedRwLockReadGuard;

use dash_sdk::query_types::AddressInfos;

use crate::error::{
    preserve_signer_key_unavailable_or, promote_address_nonce_error, PlatformWalletError,
};
use crate::BlockTime;

use super::*;

#[cfg(test)]
mod external_identity_tests;

// ---------------------------------------------------------------------------
// Top-up from platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an identity by spending platform address balances, whoever
    /// owns the identity.
    ///
    /// The identity is fetched from Platform (proved), never read from the
    /// identity manager, and is never added to it: the manager's own
    /// identities are written back by [`Self::record_top_up_balance`] once
    /// the top-up is confirmed. The balance the top-up must grow is the one
    /// in that fetched identity, read before anything is signed, so a cached
    /// balance above Platform's cannot make an executed top-up look
    /// unconfirmed.
    ///
    /// Returns the proof-attested post-spend `AddressInfos`, the identity's
    /// proved new balance and the proof's metadata. Address nonces are
    /// looked up automatically.
    ///
    /// The host signs through [`GatedAddressSigner`]: once the last input's
    /// witness returns, the wallet generation's lifecycle gate is taken and
    /// the generation checked current under it, so the broadcast and the
    /// result wait run under the gate while the host's signing does not. The
    /// gate comes back with the result, for the caller to hold through its
    /// bookkeeping. A wallet removed, or re-created under its id, while the
    /// host signed is [`PlatformWalletError::WalletRemovedBeforeSend`], with
    /// nothing broadcast.
    ///
    /// Crate-private: it does not write the managed balance back or
    /// reconcile the spent addresses. Callers outside the crate use
    /// [`PlatformWallet::top_up_identity_from_addresses`], which writes back
    /// and reconciles under the gate returned here.
    ///
    /// # Errors
    ///
    /// Before any network I/O, funding rows that are empty or whose sum for
    /// one address overflows are [`PlatformWalletError::InvalidParameter`].
    /// With `managed_only`, an identity the identity manager does not hold
    /// is [`PlatformWalletError::IdentityNotFound`] before Platform is asked
    /// (the contract of the deprecated managed-only route). Platform not
    /// knowing the identity is [`PlatformWalletError::IdentityNotFound`] and
    /// a failed lookup [`PlatformWalletError::IdentityLookupFailed`], both
    /// before anything is signed or broadcast.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to top up.
    /// * `inputs` - Funding rows: platform addresses and the credits to
    ///   spend from each. Rows naming the same address are summed.
    /// * `address_signer` - Produces ECDSA signatures for the input
    ///   [`PlatformAddress`]es. Construction is the caller's concern —
    ///   seed-backed, hardware, FFI trampoline, whatever — the wallet
    ///   struct carries no key material itself.
    /// * `managed_only` - Refuse an identity the identity manager does not
    ///   hold, as the deprecated
    ///   [`PlatformWallet::top_up_from_addresses`] does.
    ///
    /// [`PlatformWallet::top_up_identity_from_addresses`]:
    /// crate::wallet::PlatformWallet::top_up_identity_from_addresses
    /// [`PlatformWallet::top_up_from_addresses`]:
    /// crate::wallet::PlatformWallet::top_up_from_addresses
    pub(crate) async fn top_up_identity_from_addresses<S, I>(
        &self,
        identity_id: &Identifier,
        inputs: I,
        address_signer: &S,
        settings: Option<PutSettings>,
        managed_only: bool,
    ) -> Result<
        (
            (AddressInfos, Credits, ResponseMetadata),
            OwnedRwLockReadGuard<()>,
        ),
        PlatformWalletError,
    >
    where
        S: Signer<PlatformAddress> + Send + Sync,
        I: IntoIterator<Item = (PlatformAddress, Credits)> + Send,
    {
        let inputs = funding_inputs(inputs)?;
        if managed_only && !self.manages_identity(identity_id).await? {
            return Err(PlatformWalletError::IdentityNotFound(*identity_id));
        }
        let identity = Identity::fetch(&self.sdk, *identity_id)
            .await
            .map_err(|source| PlatformWalletError::IdentityLookupFailed {
                identity_id: *identity_id,
                source,
            })?
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let gated = GatedAddressSigner::new(address_signer, self, inputs.keys().copied().collect());
        let confirmed = self
            .submit_top_up_from_addresses(&identity, identity.balance(), inputs, &gated, settings)
            .await
            .map_err(|error| match gated.removed() {
                true => PlatformWalletError::WalletRemovedBeforeSend(hex::encode(self.wallet_id)),
                false => error,
            })?;
        Ok((confirmed, gated.into_gate().await))
    }

    /// Whether the identity manager holds `identity_id`;
    /// [`PlatformWalletError::WalletNotFound`] when the wallet manager has no
    /// info for this wallet.
    async fn manages_identity(
        &self,
        identity_id: &Identifier,
    ) -> Result<bool, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(
                "Wallet info not found in wallet manager".to_string(),
            )
        })?;
        Ok(info.identity_manager.identity(identity_id).is_some())
    }

    /// Write a confirmed top-up's proved balance (`new_balance` at the
    /// proof's `metadata`) back to the identity manager when it holds
    /// `identity_id`, and return the balance the caller reports: the one the
    /// manager keeps for a managed identity (a failed cache write stays
    /// pending for a later retry), `new_balance` otherwise. An identity the
    /// manager does not hold is left out of it.
    pub(crate) async fn record_top_up_balance(
        &self,
        identity_id: &Identifier,
        new_balance: Credits,
        metadata: ResponseMetadata,
    ) -> Credits {
        let mut wm = self.wallet_manager.write().await;
        match wm
            .get_wallet_info_mut(&self.wallet_id)
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

    /// Build, sign, broadcast and wait for the top-up of `identity` from
    /// `inputs`, mapping the SDK error the way every address top-up reports it.
    ///
    /// The result proof of this transition authenticates only the state it
    /// affects (the identity's balance and the input addresses as of the
    /// proof's block), not that this transition executed. The result stands
    /// only once the proof shows every input address at or past the nonce
    /// this transition was signed with and the identity balance above
    /// `balance_before`, Platform's balance of the identity read before
    /// signing ([`bind_to_submitted_nonces`]); otherwise the top-up is
    /// reported unconfirmed and nothing is reconciled from it.
    async fn submit_top_up_from_addresses<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        identity: &Identity,
        balance_before: Credits,
        inputs: BTreeMap<PlatformAddress, Credits>,
        address_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits, ResponseMetadata), PlatformWalletError> {
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
        let confirmed = identity
            .top_up_from_addresses_with_nonce_with_metadata(
                &self.sdk,
                inputs,
                address_signer,
                settings,
            )
            .await
            .map_err(top_up_error)?;
        bind_to_submitted_nonces(&submitted, balance_before, confirmed)
    }
}

/// Where an address top-up's signing stands, for [`GatedAddressSigner`].
enum TopUpGate {
    /// The host has not signed every input yet: no gate is held. Holds the
    /// inputs still to sign.
    Signing(BTreeSet<PlatformAddress>),
    /// Every input is signed, and the wallet generation's lifecycle gate is
    /// held from then on, until the top-up's bookkeeping ends.
    Gated(OwnedRwLockReadGuard<()>),
    /// Signed after the wallet was removed or replaced: nothing goes out.
    Removed,
}

/// The host's signer for an address top-up, taking the wallet generation's
/// lifecycle gate once the last input's witness returns and before it can
/// leave, as the contact request send does (its `GatedSigner`): the SDK
/// broadcasts right after signing, so the broadcast, the result wait and
/// the caller's bookkeeping run under the gate, while the host's signing
/// prompts do not. A wallet removed meanwhile does not wait for the host:
/// the last signature made after a removal, or after a re-creation under
/// the same id, is refused, so nothing is broadcast.
///
/// The transition carries one witness per input, so it cannot be broadcast
/// before the last of them returned, and that one returns only with the
/// gate held.
pub(crate) struct GatedAddressSigner<'a, S: ?Sized> {
    signer: &'a S,
    wallet: &'a IdentityWallet,
    gate: Mutex<TopUpGate>,
}

impl<'a, S: ?Sized> GatedAddressSigner<'a, S> {
    /// A signer for a top-up spending `inputs`, gated once each is signed.
    pub(crate) fn new(
        signer: &'a S,
        wallet: &'a IdentityWallet,
        inputs: BTreeSet<PlatformAddress>,
    ) -> Self {
        Self {
            signer,
            wallet,
            gate: Mutex::new(TopUpGate::Signing(inputs)),
        }
    }

    fn gate(&self) -> MutexGuard<'_, TopUpGate> {
        self.gate.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether a signature was refused because the wallet was removed or
    /// replaced while the host signed.
    pub(crate) fn removed(&self) -> bool {
        matches!(*self.gate(), TopUpGate::Removed)
    }

    /// The lifecycle gate, for the caller to hold through the bookkeeping
    /// of a submitted top-up. A submitted top-up signed every input, so the
    /// gate is held already; were it not, it is taken here, so the
    /// bookkeeping still runs under it.
    pub(crate) async fn into_gate(self) -> OwnedRwLockReadGuard<()> {
        let gate = self
            .gate
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner);
        match gate {
            TopUpGate::Gated(guard) => guard,
            TopUpGate::Signing(_) | TopUpGate::Removed => {
                self.wallet.generation.payment_guard_owned().await
            }
        }
    }

    /// Note `address` signed. Once every input is, take the lifecycle gate
    /// and check the wallet is still current under it. The gate is taken
    /// before the wallet manager's lock, as the lock ordering requires.
    async fn signed(&self, address: &PlatformAddress) -> Result<(), ProtocolError> {
        match &mut *self.gate() {
            TopUpGate::Signing(unsigned) => {
                unsigned.remove(address);
                if !unsigned.is_empty() {
                    return Ok(());
                }
            }
            TopUpGate::Gated(_) => return Ok(()),
            TopUpGate::Removed => return Err(removed_while_signing()),
        }
        let guard = self.wallet.generation.payment_guard_owned().await;
        let current = self
            .wallet
            .wallet_manager
            .read()
            .await
            .get_wallet_info(&self.wallet.wallet_id)
            .is_some_and(|info| Arc::ptr_eq(&info.generation, &self.wallet.generation));
        if current {
            *self.gate() = TopUpGate::Gated(guard);
            Ok(())
        } else {
            *self.gate() = TopUpGate::Removed;
            Err(removed_while_signing())
        }
    }
}

fn removed_while_signing() -> ProtocolError {
    ProtocolError::Generic(
        "the wallet was removed while the host signed the top-up: nothing is sent".to_string(),
    )
}

impl<S: ?Sized> std::fmt::Debug for GatedAddressSigner<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GatedAddressSigner")
    }
}

#[async_trait::async_trait]
impl<S> Signer<PlatformAddress> for GatedAddressSigner<'_, S>
where
    S: Signer<PlatformAddress> + ?Sized + Send + Sync,
{
    async fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        let signature = self.signer.sign(key, data).await?;
        self.signed(key).await?;
        Ok(signature)
    }

    async fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let witness = self.signer.sign_create_witness(key, data).await?;
        self.signed(key).await?;
        Ok(witness)
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        self.signer.can_sign_with(key)
    }
}

/// An SDK error of an address top-up as the wallet reports it: a nonce
/// race promoted to its own error, every other error kept typed under
/// [`PlatformWalletError::Sdk`].
fn top_up_error(e: dash_sdk::Error) -> PlatformWalletError {
    preserve_signer_key_unavailable_or(e, |e| {
        promote_address_nonce_error(&e).unwrap_or(PlatformWalletError::Sdk(e))
    })
}

/// The input map of an address-funded top-up from its funding rows: rows
/// naming the same address are one input of their sum. A sum that overflows
/// is refused with [`PlatformWalletError::InvalidParameter`], and so are no
/// rows at all: the top-up calls this before any network I/O, so an empty
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
/// (or of an identity that spent meanwhile, which reads the same way and
/// errs on the safe side). Every refusal is the SDK's unconfirmed outcome,
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
/// only the wallet generation's shared lifecycle gate (a read guard, taken
/// once the host signed), and
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
    use dash_sdk::query_types::AddressInfo;

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
}
