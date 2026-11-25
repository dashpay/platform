//! Shielded tips to a DashPay username: an ordinary shielded transfer to the
//! shielded address the name's profile publishes.
//!
//! The wallet decides everything a payment screen would otherwise decide for
//! itself, so two hosts cannot decide differently:
//!
//! - what the typed amount is ([`parse_shielded_tip_amount`]);
//! - who the name pays, whether that changed since this wallet last paid the
//!   name, and whether an earlier payment to it is still unresolved
//!   ([`PlatformWallet::review_shielded_tip`]);
//! - which account pays ([`ShieldedTipSource`]), proved with the seed when it
//!   is an identity's account;
//! - whether the tip may be sent now ([`PlatformWallet::send_shielded_tip`]);
//! - whether an identity's shielded balance is known
//!   ([`PlatformWallet::shielded_identity_balance`]).
//!
//! A tip keeps nothing of its own to survive a restart. Every shielded
//! spend saves its signed transition before its first dispatch
//! ([`PendingRedrive`](super::store::PendingRedrive)) and keeps it until the
//! spend is seen executed or is proven unable to execute, across restarts.
//! The address a saved transfer pays is read back from that transition with
//! the paying account's outgoing viewing key, so "an earlier payment to this
//! recipient is unresolved" is answered from the record that would execute.
//!
//! What that covers: shielded transfers paid from the wallet's notes, tips
//! and ordinary transfers alike. A shield funded by an identity or by
//! Platform addresses to another wallet's address is not a transfer and is
//! not counted.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::OwnedMutexGuard;

use dash_sdk::platform::dpns_usernames::normalize_dpns_label;
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializableTrusted;
use dpp::shielded::builder::OrchardProver;
use dpp::state_transition::StateTransition;

use super::activity::compute_activity_id;
use super::activity_recorder::visible_outputs;
use super::balance::ShieldedLocalBalanceState;
use super::file_store::FileBackedShieldedStore;
use super::keys::AccountViewingKeys;
use super::operations::shielded_actions;
use super::store::ShieldedStore;
use super::tips::{is_shielded_identity_account, SHIELDED_IDENTITY_ACCOUNT_BASE};
use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
use crate::PlatformWalletError;

/// Whom a tip to a username pays: the identity the name resolves to and the
/// shielded address that identity's profile publishes. A review returns it
/// and a send takes it back, so a name or a profile that changed in between
/// cannot redirect the payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShieldedTipRecipient {
    /// The identity the username resolves to.
    pub identity_id: Identifier,
    /// The raw Orchard address (11-byte diversifier, 32-byte `pk_d`) its
    /// profile publishes as `shieldedAddress`.
    pub address: [u8; 43],
}

/// The account a tip is paid from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedTipSource {
    /// The identity's own shielded account
    /// ([`shielded_identity_account_index`](super::shielded_identity_account_index)),
    /// at the slot the seed proves the identity holds when the tip is sent.
    Identity(Identifier),
    /// An account named by its number: an ordinary account, or the identity
    /// account of a slot that holds no identity (the funds of an identity
    /// this wallet no longer holds). The account of a slot that holds an
    /// identity is spent through [`Self::Identity`] instead.
    Account(u32),
}

/// How a tip recipient compares with the one this wallet last confirmed for
/// the same name.
///
/// A recipient is confirmed when a send to it is admitted, before the
/// transfer is built: none of these says a tip was paid. A send that then
/// found no notes to pay with still leaves its recipient confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShieldedTipRecipientChange {
    /// No recipient was confirmed for the name from this wallet before.
    New,
    /// The same identity and address as last confirmed.
    Unchanged,
    /// The name now resolves to another identity, or its profile publishes
    /// another address.
    Changed {
        /// The recipient last confirmed for the name.
        previous: ShieldedTipRecipient,
    },
}

/// What the user confirms before tipping a name
/// ([`PlatformWallet::review_shielded_tip`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShieldedTipReview {
    /// Whom the name pays now. A send takes it back unchanged.
    pub recipient: ShieldedTipRecipient,
    /// How it compares with the recipient last confirmed for the name.
    pub change: ShieldedTipRecipientChange,
    /// An earlier shielded transfer from this wallet's notes to the
    /// recipient may still execute, so a send to it is refused
    /// ([`PlatformWalletError::ShieldedTipPending`]) until a shielded sync
    /// resolves that transfer. `false` does not promise a send is admitted:
    /// a send is also refused while another tip from the wallet is being
    /// sent, whoever that one pays.
    ///
    /// The transfers counted are those to the recipient's current address
    /// and to the address this wallet last confirmed for the name. A
    /// transfer to an address the profile published earlier, which this
    /// wallet never confirmed for the name (one sent to the raw address,
    /// not as a tip), is not counted: nothing ties that address to the
    /// name. So `false`, and a hold that ended, say nothing about such a
    /// payment.
    pub unresolved: bool,
}

/// The admission of one shielded tip ([`PlatformWallet::admit_shielded_tip`]):
/// while it lives, no other tip from the wallet is admitted. A send takes
/// it and holds it until it returns.
#[derive(Debug)]
pub struct ShieldedTipAdmission(OwnedMutexGuard<()>);

/// The shielded balance of a wallet identity's own account, as far as the
/// wallet can vouch for it ([`PlatformWallet::shielded_identity_balance`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedIdentityBalance {
    /// The seed has not proved in this session which account is the
    /// identity's. Preparing the identity's shielded address
    /// ([`PlatformWallet::prepare_shielded_identity_address`]) proves it.
    Unverified,
    /// The account is proved, but its ledger is not established: it is not
    /// restored or scanned yet, as after a shielded Clear. Its balance is
    /// not known, which is not the same as zero.
    Unavailable {
        /// The identity's shielded account.
        account: u32,
    },
    /// Spendable credits of the identity's account, without the notes that
    /// pending spends reserve.
    Available {
        /// The identity's shielded account.
        account: u32,
        /// Its spendable credits.
        credits: u64,
    },
}

/// How long [`PlatformWallet::shielded_identity_balance_now`] waits for the
/// shielded store before it reports it busy.
pub const SHIELDED_IDENTITY_BALANCE_WAIT: Duration = Duration::from_millis(100);

/// Credits in one DASH: 10^8 duffs of [`CREDITS_PER_DUFF`] credits each.
const CREDITS_PER_DASH: u64 = 100_000_000 * CREDITS_PER_DUFF;
/// Fractional DASH digits a credit amount can carry.
const DASH_DECIMALS: usize = 11;

/// Parse a tip amount the user typed in DASH into exact credits.
///
/// Accepts ASCII digits with one optional decimal separator, `.` or `,`
/// (decimal keypads type the locale's), and ignores surrounding whitespace.
/// Refuses signs, digit grouping, exponents, zero, an amount past `u64`
/// credits, and nonzero digits finer than one credit (extra trailing zeros
/// are exact and allowed). A host shows the parsed amount in its
/// confirmation, not the typed text: `1,000` is one DASH.
pub fn parse_shielded_tip_amount(text: &str) -> Result<u64, PlatformWalletError> {
    let invalid =
        || PlatformWalletError::InvalidParameter(format!("Invalid DASH amount: {text:?}"));
    let trimmed = text.trim();
    let (whole, fraction, separated) = match trimmed.split_once(['.', ',']) {
        Some((whole, fraction)) => (whole, fraction, true),
        None => (trimmed, "", false),
    };
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty())
        || (separated && fraction.is_empty())
        || !digits(whole)
        || !digits(fraction)
        || fraction.bytes().skip(DASH_DECIMALS).any(|b| b != b'0')
    {
        return Err(invalid());
    }
    let fraction = &fraction[..fraction.len().min(DASH_DECIMALS)];
    let fractional_credits = format!("{fraction:0<DASH_DECIMALS$}")
        .parse::<u64>()
        .map_err(|_| invalid())?;
    let whole = if whole.is_empty() { "0" } else { whole };
    whole
        .parse::<u64>()
        .ok()
        .and_then(|whole| whole.checked_mul(CREDITS_PER_DASH))
        .and_then(|credits| credits.checked_add(fractional_credits))
        .filter(|credits| *credits > 0)
        .ok_or_else(invalid)
}

/// The key a confirmed recipient is kept under: the trimmed name as the
/// SDK's DPNS resolver normalizes it, so the key names what resolution
/// matched (`Alice`, `alice.dash` and `a11ce` are one name).
pub(super) fn tip_recipient_label(username: &str) -> String {
    normalize_dpns_label(username.trim())
}

/// How `current` compares with `previous`, the recipient last confirmed for
/// the same name.
pub(super) fn recipient_change(
    previous: Option<ShieldedTipRecipient>,
    current: &ShieldedTipRecipient,
) -> ShieldedTipRecipientChange {
    match previous {
        None => ShieldedTipRecipientChange::New,
        Some(previous) if previous == *current => ShieldedTipRecipientChange::Unchanged,
        Some(previous) => ShieldedTipRecipientChange::Changed { previous },
    }
}

/// The recipient `wallet_id` last confirmed for `label`, if any.
fn confirmed_recipient(
    store: &FileBackedShieldedStore,
    wallet_id: WalletId,
    label: &str,
) -> Result<Option<ShieldedTipRecipient>, PlatformWalletError> {
    Ok(store
        .confirmed_tip_recipient(wallet_id, label)?
        .map(|(identity_id, address)| ShieldedTipRecipient {
            identity_id: Identifier::from(identity_id),
            address,
        }))
}

/// The addresses of other wallets that `wallet_id`'s unresolved shielded
/// transfers pay: one whose signed transition is saved and has not been seen
/// executed or proven unable to execute, so it may still pay its recipient.
///
/// Each address is read from the saved transition itself. A transfer keys
/// the outgoing ciphertext of its outputs to the paying account's outgoing
/// viewing key, so that key opens the recipient of each. The change the
/// transfer returns to its own account is not a payment and is left out.
///
/// Fails, rather than reporting no payment, when it cannot tell whom a
/// saved transfer pays: a damaged record that was retained without being
/// loaded, a transition that does not decode, an account whose keys are not
/// bound, or a transfer whose readable outputs are not the outputs it was
/// saved with. A record is keyed by the hash of the outputs its account
/// could read when it was built ([`compute_activity_id`]), so an output
/// that no longer opens (the payment, with the change still readable) shows
/// as another hash.
pub(super) fn unresolved_transfer_recipients(
    store: &FileBackedShieldedStore,
    wallet_id: WalletId,
    keys: &BTreeMap<u32, AccountViewingKeys>,
) -> Result<BTreeSet<[u8; 43]>, PlatformWalletError> {
    if store.has_unloaded_note_spends(wallet_id) {
        return Err(PlatformWalletError::ShieldedRecoveryCorrupted {
            account_index: None,
            reason: "a saved shielded spend of this wallet does not read, so whom it pays is \
                     not known"
                .to_string(),
        });
    }
    let records = store
        .pending_redrives_for_wallet(wallet_id)
        .map_err(PlatformWalletError::from)?;
    let mut recipients = BTreeSet::new();
    for (id, record) in records {
        let account_index = id.account_index;
        let transition = StateTransition::deserialize_from_bytes_trusted(&record.st_bytes);
        // Whatever the record lists, a transition that does not decode may
        // be a transfer, and whom it pays is not known.
        let transition =
            transition.map_err(|e| PlatformWalletError::ShieldedRecoveryCorrupted {
                account_index: Some(account_index),
                reason: format!("a saved shielded spend does not decode: {e}"),
            })?;
        // A record that lists no notes is an identity-funded shield's: it
        // consumes none and is not a transfer. One whose bytes are a
        // transfer all the same lost its note list, and may still execute.
        if record.nullifiers.is_empty() {
            if matches!(transition, StateTransition::ShieldedTransfer(_)) {
                return Err(PlatformWalletError::ShieldedRecoveryCorrupted {
                    account_index: Some(account_index),
                    reason: "a saved shielded transfer lists none of the notes it spends"
                        .to_string(),
                });
            }
            continue;
        }
        // Only a transfer pays an Orchard address of the credit pool. The
        // other note spends pay a Platform address, a Core address, an
        // identity, a document or a token pool.
        if !matches!(transition, StateTransition::ShieldedTransfer(_)) {
            continue;
        }
        let views = keys.get(&account_index).ok_or_else(|| {
            PlatformWalletError::ShieldedRecoveryKeysRequired {
                account_index,
                reason: "an unresolved shielded transfer was paid from it".to_string(),
            }
        })?;
        // One pass reads both what the record's id is checked against and
        // whom the transfer pays: this runs under the store lock.
        let outputs = visible_outputs(shielded_actions(&transition), views);
        let cmxs: Vec<[u8; 32]> = outputs.iter().map(|output| output.cmx).collect();
        if compute_activity_id(&cmxs) != record.activity_id {
            return Err(PlatformWalletError::ShieldedRecoveryCorrupted {
                account_index: Some(account_index),
                reason: "the outputs of an unresolved shielded transfer that its account can \
                         read are not the outputs it was saved with, so whom it pays is not known"
                    .to_string(),
            });
        }
        // An output the incoming key decrypts is the account's own: the
        // change the transfer returns, not a payment. A tip is never paid
        // to the account it is paid from (the send refuses it), so no
        // payment is left out.
        recipients.extend(
            outputs
                .into_iter()
                .filter_map(|output| output.sent_to)
                .map(|address| address.to_raw_address_bytes())
                .filter(|address| !views.is_own_recipient(address)),
        );
    }
    Ok(recipients)
}

/// Refuse a name that resolves to another identity than the one the caller
/// expects it to belong to ([`PlatformWalletError::ShieldedTipRecipientMismatch`]).
/// `None` expects nothing: the user typed the name.
pub(super) fn ensure_expected_identity(
    expected: Option<&Identifier>,
    resolved: &Identifier,
) -> Result<(), PlatformWalletError> {
    match expected {
        Some(expected) if expected != resolved => {
            Err(PlatformWalletError::ShieldedTipRecipientMismatch {
                expected: *expected,
                resolved: *resolved,
            })
        }
        _ => Ok(()),
    }
}

/// Resolve whom a tip to `username` pays, from `name`, which looks the name
/// up, and `published_address`, which reads the shielded address the
/// identity's profile publishes.
///
/// The name's owner is checked against `expected_identity`
/// ([`ensure_expected_identity`]) as soon as the name is resolved and
/// before the profile is read. A name that moved to another identity is
/// then reported as a mismatch whatever that identity's profile holds,
/// instead of as a profile with no address.
pub(super) async fn resolve_recipient<N, NFut, A, AFut>(
    username: &str,
    expected_identity: Option<&Identifier>,
    name: N,
    published_address: A,
) -> Result<ShieldedTipRecipient, PlatformWalletError>
where
    N: FnOnce() -> NFut,
    NFut: Future<Output = Result<Option<Identifier>, PlatformWalletError>>,
    A: FnOnce(Identifier) -> AFut,
    AFut: Future<Output = Result<Option<[u8; 43]>, PlatformWalletError>>,
{
    let identity_id = name().await?.ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(format!("The username '{username}' was not found"))
    })?;
    ensure_expected_identity(expected_identity, &identity_id)?;
    let address = published_address(identity_id).await?.ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData(format!(
            "The profile of '{username}' publishes no valid shielded address"
        ))
    })?;
    Ok(ShieldedTipRecipient {
        identity_id,
        address,
    })
}

/// Whether an unresolved payment may still reach the recipient of a tip:
/// one to its current address, or one to the address last confirmed for the
/// name, which the recipient may have replaced since that payment.
pub(super) fn pays_unresolved(
    unresolved: &BTreeSet<[u8; 43]>,
    current: &ShieldedTipRecipient,
    previous: Option<&ShieldedTipRecipient>,
) -> bool {
    unresolved.contains(&current.address)
        || previous.is_some_and(|previous| unresolved.contains(&previous.address))
}

impl PlatformWallet {
    /// Resolve whom a tip to `username` pays: the identity the DPNS name
    /// resolves to and the shielded address its DashPay profile publishes,
    /// both fetched with proof. Accepts `alice` and `alice.dash`, with
    /// surrounding whitespace ignored.
    ///
    /// Fails with [`PlatformWalletError::InvalidIdentityData`] for a name
    /// that does not exist and for a profile that publishes no valid
    /// shielded address. The profile does not prove its owner can spend
    /// from the address.
    pub async fn resolve_shielded_tip(
        &self,
        username: &str,
    ) -> Result<ShieldedTipRecipient, PlatformWalletError> {
        self.resolve_tip_recipient(username, None).await
    }

    /// [`Self::resolve_shielded_tip`] for a name `expected_identity` is
    /// expected to belong to ([`resolve_recipient`]).
    async fn resolve_tip_recipient(
        &self,
        username: &str,
        expected_identity: Option<&Identifier>,
    ) -> Result<ShieldedTipRecipient, PlatformWalletError> {
        let username = username.trim();
        resolve_recipient(
            username,
            expected_identity,
            || async {
                self.sdk
                    .resolve_dpns_name(username)
                    .await
                    .map_err(PlatformWalletError::Sdk)
            },
            |identity_id| async move {
                self.identity
                    .dashpay()
                    .published_shielded_address(&identity_id)
                    .await
            },
        )
        .await
    }

    /// Resolve `username` for the user to confirm a tip: whom it pays now
    /// ([`Self::resolve_shielded_tip`]), how that compares with the recipient
    /// this wallet last confirmed for the name, and whether an earlier
    /// shielded payment to that recipient is still unresolved, in which case
    /// a send is refused.
    ///
    /// Confirmed recipients are kept per wallet in the coordinator's store by
    /// the name's normalized label. A send records the recipient it pays, a
    /// shielded Clear keeps the record, and removing the wallet drops it.
    ///
    /// `expected_identity` is the identity the caller expects the name to
    /// belong to, when the tip starts from one: a saved contact. A name that
    /// resolves to another identity is refused with
    /// [`PlatformWalletError::ShieldedTipRecipientMismatch`], so a name that
    /// moved is not shown as a first tip to that contact. `None` when the
    /// user typed the name.
    ///
    /// `unresolved` counts shielded transfers paid from the wallet's notes,
    /// to the recipient's current address or to the address last confirmed
    /// for the name ([`ShieldedTipReview::unresolved`]). A shield funded by
    /// an identity or by Platform addresses is not counted, whomever it
    /// pays.
    ///
    /// Requires the wallet's shielded accounts to be bound, with no Clear
    /// since ([`PlatformWalletError::ShieldedNotBound`] otherwise). Changes
    /// nothing.
    pub async fn review_shielded_tip(
        &self,
        username: &str,
        expected_identity: Option<&Identifier>,
    ) -> Result<ShieldedTipReview, PlatformWalletError> {
        let coordinator = self.shielded_coordinator().await?;
        // Refused before the lookups, and checked again below: the wallet is
        // registered and no Clear is running. The epoch tells a Clear that
        // finishes meanwhile.
        let epoch = coordinator
            .settled_epoch()
            .ok_or(PlatformWalletError::ShieldedNotBound)?;
        if !coordinator.is_wallet_registered(self.wallet_id()).await {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        let recipient = self
            .resolve_tip_recipient(username, expected_identity)
            .await?;
        // Read after the lookups: an account bound meanwhile is in them.
        let keys = self.bound_shielded_keys().await?;
        // This wait can be as long as a scan. It is made without the
        // coordinator's operation admission, which would refuse a Clear for
        // as long as the review waited.
        let store = coordinator.store().read().await;
        // An account bound during that wait too, when the keys can be read
        // at once. They are not waited for under the store guard: a bind
        // holds them while it waits for the store.
        let keys = self.bound_shielded_keys_now().unwrap_or(keys);
        let previous =
            confirmed_recipient(&store, self.wallet_id(), &tip_recipient_label(username))?;
        let unresolved = pays_unresolved(
            &unresolved_transfer_recipients(&store, self.wallet_id(), &keys)?,
            &recipient,
            previous.as_ref(),
        );
        // The rows were read under the store guard still held. They are not
        // this wallet's history if a Clear is purging them (it is admitted
        // from before its purge until it has advanced the epoch) or if one
        // finished since the review started: the review would then show a
        // changed or still unpaid recipient as new and free. Neither check
        // waits, so nothing is awaited under the store guard.
        if coordinator.clear_in_progress() || coordinator.settled_epoch() != Some(epoch) {
            return Err(PlatformWalletError::ShieldedNotBound);
        }
        // Nor if the wallet was removed during the lookups: a removal marks
        // the handle before it purges the wallet's confirmed recipients,
        // and that purge waits for the store guard held here.
        self.ensure_shielded_attached()?;
        Ok(ShieldedTipReview {
            change: recipient_change(previous, &recipient),
            recipient,
            unresolved,
        })
    }

    /// Send a shielded tip of `amount` credits to `username`, from `source`,
    /// with `memo` on the recipient's note.
    ///
    /// The tip pays `expected_recipient`, what the user confirmed in a review
    /// ([`Self::review_shielded_tip`]), and nothing else: the name and its
    /// profile are resolved again, and a recipient that differs is refused
    /// with [`PlatformWalletError::InvalidIdentityData`], for the user to
    /// review again. There is no fallback to a transparent address.
    ///
    /// `expected_identity` is the identity the caller expects the name to
    /// belong to, as for [`Self::review_shielded_tip`]: a reviewed recipient
    /// of another identity is refused with
    /// [`PlatformWalletError::ShieldedTipRecipientMismatch`], also when the
    /// review that returned it was not given the expectation.
    ///
    /// `admission` is this tip's admission ([`Self::admit_shielded_tip`]),
    /// taken before the seed is resolved: while it lives no other tip from
    /// the wallet is admitted, so a second request cannot wait behind this
    /// one and pay again.
    ///
    /// Refused with [`PlatformWalletError::ShieldedTipPending`], before
    /// anything is selected, built or sent, while an earlier shielded
    /// transfer to the recipient (its current address, or the address last
    /// confirmed for the name) may still execute. A tip to an address of
    /// the account it is paid from is refused too
    /// ([`PlatformWalletError::InvalidParameter`]): its payment could not be
    /// told from its change. That payment's record is saved before its
    /// first dispatch and survives a restart, so a relaunch refuses the same
    /// tip. A shielded sync resolves it: it is seen executed, or its anchor
    /// is proven pruned, after which it can never execute.
    ///
    /// The paying account is settled before the recipient is resolved, and
    /// checked again once the transfer has claimed its notes, before
    /// anything is built or sent: an identity's tip is paid from the
    /// account of the slot the seed proves the identity holds with the
    /// notes claimed ([`ShieldedTipSource::Identity`]), and a tip whose
    /// source no longer stands then (the identity was removed, or an
    /// identity now holds the slot of an account named by number) releases
    /// its notes and is refused. A change of the wallet's identities after
    /// that, while the tip is proved and broadcast, does not stop it.
    ///
    /// A failure with [`PlatformWalletError::ShieldedSpendUnconfirmed`] means
    /// the tip may have left the wallet and may still execute: it must not
    /// be sent again. Every other failure means this call sent nothing.
    /// [`PlatformWalletError::ShieldedTipPending`] among them says nothing
    /// about the earlier payment that caused it, which may pay the
    /// recipient: see the variant.
    ///
    /// Holds the wallet's lifecycle gate, the coordinator's admission and
    /// the account-set lock through the spend, so a removal of the wallet
    /// and a bind wait for it, and a shielded Clear is refused meanwhile.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_shielded_tip<P: OrchardProver>(
        &self,
        admission: ShieldedTipAdmission,
        seed: &[u8],
        source: ShieldedTipSource,
        username: &str,
        expected_recipient: &ShieldedTipRecipient,
        expected_identity: Option<&Identifier>,
        amount: u64,
        memo: [u8; 36],
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        // The tip pays `expected_recipient` or nothing, so a recipient of
        // another identity than the caller expects is refused before any
        // wait or lookup.
        ensure_expected_identity(expected_identity, &expected_recipient.identity_id)?;
        // Held until this returns. An admission of another wallet would
        // leave this wallet's tips unguarded.
        if !Arc::ptr_eq(
            OwnedMutexGuard::mutex(&admission.0),
            &self.tip_send_in_progress,
        ) {
            return Err(PlatformWalletError::InvalidParameter(
                "the shielded tip admission was taken on another wallet".to_string(),
            ));
        }
        let _sending = admission;
        // Held through settlement, so a removal waits for the tip and a
        // Clear is refused instead of purging the store it reserves in and
        // settles against. Refused for a wallet removed, or not bound since
        // the last Clear.
        let locks = self.lock_for_shielded_operation(false).await?;
        let coordinator = &locks.coordinator;
        coordinator.ensure_network(self.sdk.network)?;
        let _admission = locks.admit_spend(self).await?;
        // A bind that dropped the paying account would purge the notes the
        // tip reserves, and its saved record with them.
        let _config = self.shielded_config_lock.lock().await;
        let account = self.tip_source_account(seed, source).await?;
        // The name is looked up again, and its owner checked against the
        // expected identity before its profile is read: a name that moved
        // since the review is reported as that, whatever the new owner's
        // profile holds.
        let current = self
            .resolve_tip_recipient(username, expected_identity)
            .await?;
        if current != *expected_recipient {
            return Err(PlatformWalletError::InvalidIdentityData(
                "The tip recipient changed; review and confirm the payment again".to_string(),
            ));
        }
        // The lookups above took network time, during which the wallet's
        // identities can have changed. Refused here before the wait for the
        // store; the check that counts runs once the notes are claimed.
        self.ensure_tip_source_selects(seed, source, account)
            .await?;
        self.send_resolved_shielded_tip(
            seed,
            source,
            account,
            &tip_recipient_label(username),
            &current,
            amount,
            memo,
            prover,
        )
        .await
    }

    /// The part of [`Self::send_shielded_tip`] that follows the resolution
    /// of the recipient, under the locks it holds: refuse the tip while an
    /// earlier payment to `recipient` is unresolved, record `recipient` as
    /// the one confirmed for `label`, and send the transfer from `account`,
    /// the account `source` selected.
    ///
    /// `source` is checked again once the transfer has claimed its notes,
    /// before anything is built or sent. Claiming them waits for the store,
    /// which a scan can hold for minutes, and an identity can be removed, or
    /// one added into the slot of an account named by number, meanwhile.
    /// The notes are then this tip's and nothing else has waited, so the
    /// tip is paid only if `source` selected `account` with the notes
    /// claimed; otherwise they are released and nothing is sent.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn send_resolved_shielded_tip<P: OrchardProver>(
        &self,
        seed: &[u8],
        source: ShieldedTipSource,
        account: u32,
        label: &str,
        recipient: &ShieldedTipRecipient,
        amount: u64,
        memo: [u8; 36],
        prover: P,
    ) -> Result<(), PlatformWalletError> {
        let coordinator = &self.shielded_coordinator().await?;
        // Refused before the recipient is recorded as confirmed: the
        // transfer would refuse it only afterwards.
        if amount == 0 {
            return Err(PlatformWalletError::InvalidParameter(
                "A tip amount must be more than zero".to_string(),
            ));
        }
        // The account must be bound and the seed must derive its key, or the
        // transfer refuses the tip only after the recipient was recorded.
        // Derived once: the transfer below spends with this keyset.
        let keyset = self.derive_spend_keyset(seed, account).await?;
        let keys = self.bound_shielded_keys().await?;
        // Paid to the paying account itself, the payment is one more of the
        // account's own outputs beside its change, and an unresolved one
        // could not be told from change: it would not hold back a repeat.
        if keys
            .get(&account)
            .is_some_and(|views| views.is_own_recipient(&recipient.address))
        {
            return Err(PlatformWalletError::InvalidParameter(
                "A tip cannot pay the shielded account it is paid from; pay from another account"
                    .to_string(),
            ));
        }
        {
            let mut store = coordinator.store().write().await;
            let previous = confirmed_recipient(&store, self.wallet_id(), label)?;
            let unresolved = unresolved_transfer_recipients(&store, self.wallet_id(), &keys)?;
            if pays_unresolved(&unresolved, recipient, previous.as_ref()) {
                return Err(PlatformWalletError::ShieldedTipPending);
            }
            // What the user confirmed becomes the recipient on file for the
            // name, whatever happens to the payment.
            store.confirm_tip_recipient(
                self.wallet_id(),
                label,
                &(recipient.identity_id.to_buffer(), recipient.address),
            )?;
        }
        self.shielded_transfer_once_claimed(
            coordinator,
            &keyset,
            account,
            &recipient.address,
            amount,
            memo,
            prover,
            || self.ensure_tip_source_selects(seed, source, account),
        )
        .await
    }

    /// Refuse unless `source` still selects `account`
    /// ([`Self::tip_source_account`]): the identity still holds the slot the
    /// seed proves, or the slot of an account named by number still holds
    /// no identity.
    async fn ensure_tip_source_selects(
        &self,
        seed: &[u8],
        source: ShieldedTipSource,
        account: u32,
    ) -> Result<(), PlatformWalletError> {
        if self.tip_source_account(seed, source).await? == account {
            Ok(())
        } else {
            Err(PlatformWalletError::InvalidIdentityData(
                "The paying identity's shielded account changed while the tip was prepared; \
                 send it again"
                    .to_string(),
            ))
        }
    }

    /// The shielded balance of the wallet identity `identity_id`'s own
    /// account, with what the wallet knows about it:
    ///
    /// - [`ShieldedIdentityBalance::Unverified`] until the seed has proved
    ///   in this session which account is the identity's
    ///   ([`Self::prepared_shielded_identity_account`]). A recorded identity
    ///   index is not proof: it may be a placeholder a host restored, whose
    ///   account belongs to another identity.
    /// - [`ShieldedIdentityBalance::Unavailable`] while that account's
    ///   ledger is not established (not bound, or neither restored nor
    ///   scanned).
    /// - [`ShieldedIdentityBalance::Available`] with its spendable credits.
    ///
    /// A host shows these states and does not add up stored notes, which
    /// cannot tell an empty account from one whose ledger was cleared.
    /// Reads local state only. Fails with
    /// [`PlatformWalletError::IdentityNotFound`] for an identity this wallet
    /// does not hold, and with [`PlatformWalletError::ShieldedBalanceBusy`]
    /// while a shielded Clear is admitted and has not finished: the caller
    /// keeps the state it last read and reads again. Before the manager's
    /// shielded store is configured no account can be proved, and every
    /// identity is `Unverified`.
    pub async fn shielded_identity_balance(
        &self,
        identity_id: &Identifier,
    ) -> Result<ShieldedIdentityBalance, PlatformWalletError> {
        self.ensure_wallet_identity(identity_id).await?;
        let Ok(coordinator) = self.shielded_coordinator().await else {
            return Ok(ShieldedIdentityBalance::Unverified);
        };
        let Some(account) = self.prepared_shielded_identity_account(identity_id).await else {
            // A preparation is not read while a Clear holds the admission,
            // and that Clear may yet give up with the preparation standing.
            return match coordinator.clear_in_progress() {
                true => Err(PlatformWalletError::ShieldedBalanceBusy),
                false => Ok(ShieldedIdentityBalance::Unverified),
            };
        };
        // The amount read below is this epoch's or the read is retried: a
        // Clear that holds the admission, or waits for it, reads as none.
        let Some(epoch) = coordinator.settled_epoch() else {
            return Err(PlatformWalletError::ShieldedBalanceBusy);
        };
        let established = match coordinator.local_balance_snapshot(self.wallet_id()).await? {
            ShieldedLocalBalanceState::Ready(snapshot) => snapshot
                .accounts
                .get(&account)
                .filter(|balance| balance.is_established())
                .map(|balance| balance.spendable_credits),
            ShieldedLocalBalanceState::Unbound | ShieldedLocalBalanceState::RestoreIncomplete => {
                None
            }
        };
        #[cfg(test)]
        super::tips::test_hooks::after_balance_read(self).await;
        // Whether a Clear is admitted is sampled before the preparation is
        // read again, not after: a Clear that wiped the ledger just read
        // and finished between the two checks would otherwise be seen by
        // neither, and the wiped ledger's balance returned.
        let clearing = coordinator.clear_in_progress();
        // A Clear ends the preparation: one that finished since the
        // preparation was read leaves the account unproved. Checked after
        // the read rather than by holding the coordinator's admission
        // across it, which would refuse a Clear for as long as the read
        // waited for the store.
        let standing = self.prepared_shielded_identity_account(identity_id).await;
        // A Clear that was admitted and had not finished may be purging the
        // ledger just read, and may also give up without wiping anything,
        // with the preparation still standing; while it holds the admission
        // the preparation is not read at all. None of it is known yet, so
        // nothing is reported: the caller reads again.
        if clearing || coordinator.clear_in_progress() {
            return Err(PlatformWalletError::ShieldedBalanceBusy);
        }
        // A Clear that started and finished since the amount was read wiped
        // the ledger it came from, whatever the preparation read says: the
        // epoch moved, and the caller reads again.
        if coordinator.settled_epoch() != Some(epoch) {
            return Err(PlatformWalletError::ShieldedBalanceBusy);
        }
        if standing != Some(account) {
            return Ok(ShieldedIdentityBalance::Unverified);
        }
        Ok(match established {
            Some(credits) => ShieldedIdentityBalance::Available { account, credits },
            None => ShieldedIdentityBalance::Unavailable { account },
        })
    }

    /// [`Self::shielded_identity_balance`] for a caller that must not wait
    /// (a screen refreshing a balance): the whole read is given
    /// [`SHIELDED_IDENTITY_BALANCE_WAIT`], and fails with
    /// [`PlatformWalletError::ShieldedBalanceBusy`] when it has not finished
    /// by then. What it waits for is the wallet's own state and the
    /// shielded store, which a scan or a spend can hold for minutes; the
    /// error does not say which was held, nor whether it was a Clear in
    /// progress instead. It is the one error of this read that means keep
    /// the state last read and read again; every other one is a failure.
    pub async fn shielded_identity_balance_now(
        &self,
        identity_id: &Identifier,
    ) -> Result<ShieldedIdentityBalance, PlatformWalletError> {
        tokio::time::timeout(
            SHIELDED_IDENTITY_BALANCE_WAIT,
            self.shielded_identity_balance(identity_id),
        )
        .await
        .unwrap_or(Err(PlatformWalletError::ShieldedBalanceBusy))
    }

    /// Admit one shielded tip from this wallet, or refuse with
    /// [`PlatformWalletError::ShieldedTipPending`] while another is admitted.
    ///
    /// Never waits: a second request while a tip is being sent is the same
    /// tip sent twice, and queued behind the first it would pay again once
    /// the first had executed. Take the admission before anything that can
    /// take time, resolving the seed above all (a host may prompt for it),
    /// and pass it to [`Self::send_shielded_tip`]. Dropping it unused admits
    /// the next tip.
    pub fn admit_shielded_tip(&self) -> Result<ShieldedTipAdmission, PlatformWalletError> {
        Arc::clone(&self.tip_send_in_progress)
            .try_lock_owned()
            .map(ShieldedTipAdmission)
            .map_err(|_| PlatformWalletError::ShieldedTipPending)
    }

    /// The account `source` spends.
    ///
    /// An identity spends the account of the slot the seed proves it holds
    /// ([`Self::verified_identity_account`]). An identity account named by
    /// its number is refused while its slot holds an identity: the number
    /// says nothing about which identity the user meant, and that identity's
    /// tips go through [`ShieldedTipSource::Identity`], which proves it.
    pub(crate) async fn tip_source_account(
        &self,
        seed: &[u8],
        source: ShieldedTipSource,
    ) -> Result<u32, PlatformWalletError> {
        match source {
            ShieldedTipSource::Identity(identity_id) => {
                self.verified_identity_account(seed, &identity_id).await
            }
            ShieldedTipSource::Account(account) if is_shielded_identity_account(account) => {
                let wm = self.wallet_manager.read().await;
                let info = wm.get_wallet_info(&self.wallet_id()).ok_or_else(|| {
                    PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id()))
                })?;
                let occupant = info
                    .identity_manager
                    .wallet_identities
                    .get(&self.wallet_id())
                    .and_then(|slots| slots.get(&(account - SHIELDED_IDENTITY_ACCOUNT_BASE)));
                match occupant {
                    Some(owner) => Err(PlatformWalletError::InvalidIdentityData(format!(
                        "Account {account} is the shielded account of identity {}; send from \
                         that identity instead",
                        owner.identity.id()
                    ))),
                    None => Ok(account),
                }
            }
            ShieldedTipSource::Account(account) => Ok(account),
        }
    }

    /// The viewing keys of the wallet's bound shielded accounts, or
    /// [`PlatformWalletError::ShieldedNotBound`].
    async fn bound_shielded_keys(
        &self,
    ) -> Result<BTreeMap<u32, AccountViewingKeys>, PlatformWalletError> {
        self.shielded_keys
            .read()
            .await
            .clone()
            .ok_or(PlatformWalletError::ShieldedNotBound)
    }

    /// [`Self::bound_shielded_keys`] without waiting: `None` while the keys
    /// are being replaced, or when the wallet is not bound.
    fn bound_shielded_keys_now(&self) -> Option<BTreeMap<u32, AccountViewingKeys>> {
        self.shielded_keys.try_read().ok()?.clone()
    }

    /// [`PlatformWalletError::IdentityNotFound`] unless this wallet holds
    /// `identity_id` as one of its own.
    async fn ensure_wallet_identity(
        &self,
        identity_id: &Identifier,
    ) -> Result<(), PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id())
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id())))?
            .identity_manager
            .managed_identity(identity_id)
            .filter(|identity| identity.wallet_id == Some(self.wallet_id()))
            .map(|_| ())
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))
    }
}
