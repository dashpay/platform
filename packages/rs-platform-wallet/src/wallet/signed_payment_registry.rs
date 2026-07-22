//! In-memory registry backing the deferred build → broadcast/release core-send
//! lifecycle (BIP70 / BIP270 "sign now, submit on merchant ack").
//!
//! The regular send path
//! ([`CoreWallet::broadcast_transaction_releasing_reservation`](crate::CoreWallet::broadcast_transaction_releasing_reservation))
//! builds, signs, and broadcasts in one uninterrupted step. BIP70-style flows
//! must split that: sign now (reserving the funding UTXOs), hand the raw bytes
//! to a merchant server, and broadcast **only** once the server acks — or
//! release the reservation if it nacks / the user abandons.
//!
//! `TransactionBuilder::build_signed` already reserves the selected UTXOs in the
//! funding account's `ReservationSet` and leaves the reservation held on
//! success (see [`crate::wallet::reservations`]). This registry owns the built
//! transaction and its held reservation between build and submission, keyed by
//! an opaque [`ReservationToken`], and enforces the lifecycle invariants:
//!
//! * [`broadcast`](SignedPaymentRegistry::broadcast) validates the wallet
//!   binding **under the lock** and removes **only a matching** entry, so a
//!   repeated or concurrent broadcast of the same token can never
//!   double-broadcast — the second caller finds nothing and gets
//!   [`SignedPaymentError::StaleToken`] — and a wrong-wallet caller cannot
//!   consume (and thereby strand) the rightful owner's token.
//! * [`release`](SignedPaymentRegistry::release) is idempotent: releasing an
//!   unknown / already-consumed token is a silent no-op.
//! * A token is bound to the exact wallet *generation* it was minted against
//!   ([`CoreWallet::is_same_generation`](crate::CoreWallet::is_same_generation) —
//!   the same identity the V2 finalized-transaction handle path uses). Two
//!   wallets sharing one multi-wallet `PlatformWalletManager`, or a re-created
//!   wallet under the same id whose in-memory `ReservationSet` no longer holds
//!   the inputs, are both told apart: broadcasting through either is a
//!   [`SignedPaymentError::WalletMismatch`] rather than a spend against stale
//!   state.
//! * A token has a bounded lifetime ([`RESERVATION_MAX_AGE_BLOCKS`]). Once the
//!   wallet's `last_processed_height` has advanced far enough past the height at
//!   which `build_signed` / `finalize_transaction` stamped the reservation that
//!   key-wallet's own `ReservationSet` TTL could have swept and re-selected the
//!   funding UTXO for an unrelated build,
//!   broadcasting or releasing the token would act on state that may no longer
//!   be its own — so both are refused with
//!   [`SignedPaymentError::StaleReservationToken`] and the caller must rebuild.
//!   This guard is the primary defence: key-wallet exposes no per-outpoint
//!   ownership/generation check to make [`release`](SignedPaymentRegistry::release)
//!   itself generation-aware without modifying the pinned crate, so an
//!   unconditional release-by-outpoint after a sweep is prevented by never
//!   reaching it once the token is stale.
//!
//! ## Process-death semantics
//!
//! The registry and the underlying `ReservationSet` are both in-memory. An app
//! crash between build and broadcast drops the registry entry **and** the
//! reservation together, so nothing leaks across a restart — the UTXOs are
//! spendable again on reload. This matches dashj's behaviour (its in-flight
//! reservations are likewise memory-only). No on-disk reservation persistence
//! exists to follow.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use dashcore::{Transaction, Txid};
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;

use crate::broadcaster::TransactionBroadcaster;
use crate::wallet::core::CoreWallet;
use crate::PlatformWalletError;

/// Opaque handle to a registered, signed-but-unsent payment. Minted by
/// [`SignedPaymentRegistry::register`]; consumed by
/// [`SignedPaymentRegistry::broadcast`] or
/// [`SignedPaymentRegistry::release`]. Values are unique for the process
/// lifetime and never reused, so a stale token can always be recognised.
pub type ReservationToken = u64;

/// Maximum age, in `last_processed_height` blocks, of a registered token before
/// its broadcast or release is refused.
///
/// Kept strictly below key-wallet's `RESERVATION_TTL_BLOCKS` (24, ~1h at the
/// mainnet block target): a `build_signed` / `finalize_transaction` reservation
/// is stamped at the wallet's `last_processed_height` (via `set_current_height`)
/// and swept by a later `reserve`/`reserved` call — itself stamped with the same
/// `last_processed_height` clock — once it is `RESERVATION_TTL_BLOCKS` old,
/// silently returning the outpoint to the selectable pool where an unrelated
/// build can re-select and re-reserve it.
/// `ReservationSet::release` removes an outpoint unconditionally, with no
/// ownership/generation check, so acting on a token whose reservation was
/// already swept could free (or broadcast against) a newer, unrelated
/// reservation. Refusing at this lower bound guarantees the guard always trips
/// **before** the underlying reservation could have been swept, leaving a margin
/// for `last_processed_height` to lag a few blocks behind the true tip.
const RESERVATION_MAX_AGE_BLOCKS: u32 = 20;

/// Whether a token registered at `registered_height` is too old to act on at
/// `current_height` (see [`RESERVATION_MAX_AGE_BLOCKS`]). Unknown heights (the
/// wallet was gone at register or is gone now) disable the guard — the
/// wallet-mismatch / account-lookup paths already reject those cases.
fn reservation_expired(registered_height: Option<u32>, current_height: Option<u32>) -> bool {
    match (registered_height, current_height) {
        (Some(registered), Some(current)) => {
            current.saturating_sub(registered) >= RESERVATION_MAX_AGE_BLOCKS
        }
        _ => false,
    }
}

/// Failure of a deferred broadcast/release token operation.
#[derive(Debug, thiserror::Error)]
pub enum SignedPaymentError {
    /// The token is unknown, already broadcast, or already released. The
    /// registry never re-broadcasts, so this is the guard that turns a
    /// double-broadcast into a typed error instead of a second send.
    #[error("reservation token {0} is unknown, already broadcast, or already released")]
    StaleToken(ReservationToken),

    /// The token was minted against a different (re-created) wallet instance
    /// than the one it is being broadcast through. Its reservation lives in
    /// that other instance's `ReservationSet`, so submitting it here would spend
    /// against state this wallet never reserved.
    #[error("reservation token {0} was minted against a different wallet instance")]
    WalletMismatch(ReservationToken),

    /// The token has outlived [`RESERVATION_MAX_AGE_BLOCKS`], so its underlying
    /// UTXO reservation may already have been swept by key-wallet's TTL and
    /// re-selected by an unrelated build. Acting on it (broadcast or release)
    /// could touch a newer reservation, so it is refused and the caller must
    /// rebuild the payment.
    #[error("reservation token {0} has outlived its reservation lifetime; rebuild the payment")]
    StaleReservationToken(ReservationToken),

    /// The underlying broadcast failed. Carries the still-typed wallet error so
    /// the FFI boundary can preserve the retry semantics (e.g. the ambiguous
    /// [`PlatformWalletError::TransactionBroadcastUnconfirmed`] "may already be
    /// on the network" signal).
    #[error(transparent)]
    Broadcast(#[from] PlatformWalletError),
}

/// A built, signed transaction whose funding UTXOs are reserved, awaiting a
/// deferred broadcast or an explicit release.
struct RegisteredPayment<B: TransactionBroadcaster + ?Sized> {
    /// The wallet instance the payment was built against — captured so the
    /// broadcast/release act on the exact `ReservationSet` that holds the
    /// inputs, and so a re-created wallet can be detected via `Arc::ptr_eq`.
    core: CoreWallet<B>,
    /// The signed transaction to broadcast.
    tx: Transaction,
    /// The releasable funding-account handle — the account whose reservation
    /// `finalize` took and which a rejected broadcast or an explicit release
    /// must reconcile. An [`AccountTypePreference`] (not the narrower
    /// `StandardAccountType`) so CoinJoin-funded deferred payments retain a
    /// releasable handle too: `finalize` reserves the selected inputs for EVERY
    /// account variant, so a CoinJoin token must be able to release them
    /// immediately on rejection/abandon rather than stranding them until the
    /// key-wallet TTL backstop.
    account_type: AccountTypePreference,
    account_index: u32,
    /// Wallet `last_processed_height` captured at registration — the exact clock
    /// `build_signed` / `finalize_transaction` stamps the funding reservation
    /// with. Compared against the wallet's current `last_processed_height` to
    /// refuse a broadcast/release once the reservation could plausibly have been
    /// swept by key-wallet's TTL (see [`RESERVATION_MAX_AGE_BLOCKS`]). `None` when
    /// the wallet was not resolvable at registration, which disables the age
    /// guard for this entry.
    registered_height: Option<u32>,
}

/// Registry of signed-but-unsent payments keyed by [`ReservationToken`].
///
/// Generic over the broadcaster `B` so it can be unit-tested with mock
/// broadcasters; the FFI layer instantiates a single process-global registry
/// pinned to the production `SpvBroadcaster`.
pub struct SignedPaymentRegistry<B: TransactionBroadcaster + ?Sized> {
    next_token: AtomicU64,
    entries: Mutex<HashMap<ReservationToken, RegisteredPayment<B>>>,
}

impl<B: TransactionBroadcaster + ?Sized> Default for SignedPaymentRegistry<B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B: TransactionBroadcaster + ?Sized> SignedPaymentRegistry<B> {
    /// A fresh, empty registry.
    pub fn new() -> Self {
        Self {
            // Start at 1 so 0 is never a valid token (matches the FFI's
            // null-handle convention).
            next_token: AtomicU64::new(1),
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Lock the entries map, recovering from a poisoned mutex rather than
    /// panicking. The registry is a single process-global, so a panic elsewhere
    /// while the lock was held would otherwise permanently disable deferred
    /// payments for every wallet; the guarded `HashMap` has no invariant a
    /// partial write could break, so recovery is safe (mirrors key-wallet's
    /// sibling `ReservationSet::lock`).
    fn lock(&self) -> MutexGuard<'_, HashMap<ReservationToken, RegisteredPayment<B>>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Take ownership of a built, signed `tx` (whose funding UTXOs `finalize`
    /// already reserved) and return an opaque token for a later
    /// [`broadcast`](Self::broadcast) or [`release`](Self::release).
    ///
    /// `core` is the wallet the payment was built against; it is captured so the
    /// later operation acts on the exact reservation state that holds the inputs.
    ///
    /// `registered_height` MUST be the `last_processed_height` the funding
    /// reservation was stamped with — the height captured **inside** the funding
    /// critical section, *before* signing (`SignedCoreTransaction::reservation_height`).
    /// The caller passes it in rather than the registry sampling a fresh
    /// `last_processed_height` here, which would be taken *after* the
    /// (potentially slow, external) signer ran: a slow signer could let the
    /// wallet advance so that a freshly-sampled height makes the token look
    /// young while the reservation it covers has already aged toward
    /// key-wallet's TTL. `None` disables the age guard for this entry (the
    /// wallet-mismatch / account-lookup paths still reject a re-created wallet).
    /// See [`RESERVATION_MAX_AGE_BLOCKS`].
    pub async fn register(
        &self,
        core: CoreWallet<B>,
        tx: Transaction,
        account_type: AccountTypePreference,
        account_index: u32,
        registered_height: Option<u32>,
    ) -> ReservationToken {
        let token = self.next_token.fetch_add(1, Ordering::SeqCst);
        self.lock().insert(
            token,
            RegisteredPayment {
                core,
                tx,
                account_type,
                account_index,
                registered_height,
            },
        );
        token
    }

    /// Broadcast the payment behind `token`, reconciling its UTXO reservation on
    /// failure, then consume the token.
    ///
    /// The wallet binding is validated **under the registry lock**, and only a
    /// *matching* entry is removed. So a wrong-wallet caller can never consume
    /// (and thereby destroy) the rightful owner's token: a mismatched token is
    /// left in the registry for its owner and this call returns
    /// [`SignedPaymentError::WalletMismatch`]. `current` must be the same wallet
    /// *generation* the token was minted against
    /// (`CoreWallet::is_same_generation`); a re-created wallet under the same id
    /// is a mismatch, not a spend against stale state.
    ///
    /// Because the check-and-consume happen atomically under one lock hold, a
    /// repeated or concurrent broadcast of the same token by the rightful owner
    /// gets [`SignedPaymentError::StaleToken`] instead of a second send — the
    /// first consumer removed it.
    ///
    /// On a definitive rejection the reservation is released for an immediate
    /// rebuild; on an ambiguous ("may already be on the network") failure it is
    /// kept — the same policy as the non-deferred send path.
    pub async fn broadcast(
        &self,
        token: ReservationToken,
        current: &CoreWallet<B>,
    ) -> Result<Txid, SignedPaymentError> {
        // Validate the wallet binding UNDER the lock and consume ONLY a matching
        // entry. Peeking first means a mismatched caller leaves the entry in
        // place for its rightful owner rather than removing it (which would
        // strand the owner's reservation until the TTL backstop). The
        // check-then-remove is one lock hold, so it is atomic against a
        // concurrent broadcast; the std::Mutex guard is dropped before any await.
        let entry = {
            let mut entries = self.lock();
            match entries.get(&token) {
                None => return Err(SignedPaymentError::StaleToken(token)),
                Some(entry) => {
                    // Same wallet generation the token was minted against — the
                    // single identity the V2 handle path also uses. A re-created
                    // wallet (same id + manager, new generation) is a mismatch.
                    if !entry.core.is_same_generation(current) {
                        // Leave the entry for its rightful owner.
                        return Err(SignedPaymentError::WalletMismatch(token));
                    }
                }
            }
            entries
                .remove(&token)
                .expect("entry present under the same lock hold")
        };

        // Refuse a token whose reservation could already have been swept and
        // re-selected by an unrelated build. The entry is already removed, so we
        // simply drop it — deliberately WITHOUT releasing, since a release by
        // outpoint here could free a newer build's reservation. The stale
        // reservation is reclaimed by key-wallet's own TTL sweep.
        if reservation_expired(
            entry.registered_height,
            current.last_processed_height().await,
        ) {
            return Err(SignedPaymentError::StaleReservationToken(token));
        }

        // One releasing-broadcast path for every funding variant, CoinJoin
        // included: a definitive rejection releases the reservation for an
        // immediate rebuild, an ambiguous outcome keeps it, and the release is
        // bound to the token's own wallet generation.
        let txid = entry
            .core
            .broadcast_payment_releasing_reservation(
                entry.account_type,
                entry.account_index,
                &entry.tx,
            )
            .await?;
        Ok(txid)
    }

    /// Reconcile one already-removed entry's reservation, honouring the age
    /// guard: if the token has outlived its reservation lifetime the funding
    /// outpoint may already have been swept and re-selected by an unrelated
    /// build, so releasing it by outpoint could free that newer reservation —
    /// drop it without touching the `ReservationSet` (key-wallet's TTL reclaims
    /// the original). Otherwise release the funding-account reservation (any
    /// variant, CoinJoin included), bound to the token's own wallet generation.
    async fn reconcile_removed_entry(entry: RegisteredPayment<B>) {
        if reservation_expired(
            entry.registered_height,
            entry.core.last_processed_height().await,
        ) {
            return;
        }
        entry
            .core
            .release_transaction_reservation(entry.account_type, entry.account_index, &entry.tx)
            .await;
    }

    /// Release the funding reservation behind `token` and drop it. Idempotent:
    /// releasing an unknown / already-consumed token is a silent no-op, so a
    /// double release (or a release after a broadcast) is harmless.
    ///
    /// The release acts on the wallet instance the token was minted against —
    /// the one whose `ReservationSet` actually holds the inputs — so no wallet
    /// handle need be threaded in.
    pub async fn release(&self, token: ReservationToken) {
        let entry = { self.lock().remove(&token) };
        let Some(entry) = entry else {
            // Unknown / already consumed — idempotent no-op.
            return;
        };
        Self::reconcile_removed_entry(entry).await;
    }

    /// Release and drop every outstanding token bound to `wallet`'s *generation*
    /// ([`CoreWallet::is_same_generation`](crate::CoreWallet::is_same_generation)),
    /// returning how many were removed. Called from `platform_wallet_destroy`
    /// when the **final** handle to a live wallet generation is destroyed.
    ///
    /// Unlike [`remove_entries_for_wallet`](Self::remove_entries_for_wallet)
    /// (which drops without releasing at generation *teardown*), the generation
    /// here is still live in its manager — destroying the last wrapper handle
    /// does not remove the logical wallet, and the same wallet can be handed out
    /// again. So each token's reservation is RELEASED against that still-live
    /// generation (honouring the age guard), rather than left stranded in the
    /// account `ReservationSet` until key-wallet's TTL. Race-free: matching is by
    /// generation, and a generation that was actually torn down
    /// (`remove_wallet`) has already had its tokens swept there, so this finds
    /// none and cannot release against a re-created generation's inputs.
    pub async fn release_entries_for_wallet(&self, wallet: &CoreWallet<B>) -> usize {
        // Take the matching entries out under the lock, then reconcile each with
        // the guard dropped (the reconcile path awaits).
        let taken: Vec<RegisteredPayment<B>> = {
            let mut entries = self.lock();
            let tokens: Vec<ReservationToken> = entries
                .iter()
                .filter(|(_, entry)| entry.core.is_same_generation(wallet))
                .map(|(token, _)| *token)
                .collect();
            tokens
                .into_iter()
                .filter_map(|token| entries.remove(&token))
                .collect()
        };
        let count = taken.len();
        for entry in taken {
            Self::reconcile_removed_entry(entry).await;
        }
        count
    }

    /// Drop every outstanding token bound to `wallet` (same shared
    /// `WalletManager` and `wallet_id`), WITHOUT releasing, returning how many
    /// were removed.
    ///
    /// Called from the FFI at actual wallet-generation *teardown*
    /// (`platform_wallet_manager_remove_wallet`): the wallet — and its accounts'
    /// `ReservationSet`s — are removed from the manager, so the reservations
    /// cease to exist and there is nothing to reconcile. Dropping the tokens here
    /// also makes any stale handle to that generation inert, so a later
    /// destroy/release of a lingering handle can never release-by-outpoint
    /// against a re-created generation's inputs — this is the teardown half of
    /// the single generation policy the deferred paths share.
    pub fn remove_entries_for_wallet(&self, wallet: &CoreWallet<B>) -> usize {
        let mut entries = self.lock();
        let before = entries.len();
        entries.retain(|_, entry| !entry.core.is_same_generation(wallet));
        before - entries.len()
    }

    /// Number of outstanding (registered but not yet broadcast/released) tokens.
    /// Exposed under `test-utils` so downstream FFI-layer tests (e.g. the
    /// `platform_wallet_destroy` final-alias sweep) can observe registry state.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn outstanding(&self) -> usize {
        self.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use dashcore::{Address as DashAddress, Network, Transaction, Txid};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::signer::Signer;
    use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
    use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

    use super::{SignedPaymentError, SignedPaymentRegistry, RESERVATION_MAX_AGE_BLOCKS};
    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::test_support::{funded_wallet_manager, AlwaysMaybeSentBroadcaster, WalletSigner};
    use crate::wallet::core::CoreWallet;

    /// The [`AccountTypePreference`] a `build_signed_tx` funding account maps to
    /// — the registry now retains the full account handle (CoinJoin included),
    /// so the tests register with the preference rather than the narrower
    /// `StandardAccountType`.
    fn preference(account_type: StandardAccountType) -> AccountTypePreference {
        match account_type {
            StandardAccountType::BIP44Account => AccountTypePreference::BIP44,
            StandardAccountType::BIP32Account => AccountTypePreference::BIP32,
        }
    }
    use crate::PlatformWalletError;

    /// Broadcaster that records the exact bytes handed to it and succeeds,
    /// so a test can assert the broadcast tx is byte-identical to the one the
    /// caller registered.
    struct RecordingBroadcaster {
        sent: Mutex<Vec<Vec<u8>>>,
    }

    impl RecordingBroadcaster {
        fn new() -> Self {
            Self {
                sent: Mutex::new(Vec::new()),
            }
        }

        fn last_sent(&self) -> Option<Vec<u8>> {
            self.sent.lock().unwrap().last().cloned()
        }
    }

    #[async_trait]
    impl TransactionBroadcaster for RecordingBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.sent
                .lock()
                .unwrap()
                .push(dashcore::consensus::serialize(transaction));
            Ok(transaction.txid())
        }
    }

    /// Broadcaster that counts how many times it was asked to send.
    struct CountingBroadcaster {
        count: AtomicUsize,
    }

    impl CountingBroadcaster {
        fn new() -> Self {
            Self {
                count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl TransactionBroadcaster for CountingBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(transaction.txid())
        }
    }

    /// A testnet `CoreWallet` over the shared funded fixture plus a
    /// 1_000_000-duff payment to a dummy recipient.
    async fn funded_core_wallet<B: TransactionBroadcaster>(
        account_type: StandardAccountType,
        broadcaster: Arc<B>,
    ) -> (CoreWallet<B>, WalletSigner, Vec<(DashAddress, u64)>) {
        let (wallet_manager, wallet_id, balance, signer) =
            funded_wallet_manager(account_type).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(sdk, wallet_manager, wallet_id, broadcaster, balance);
        let recipient = DashAddress::dummy(Network::Testnet, 42);
        (core, signer, vec![(recipient, 1_000_000u64)])
    }

    /// A testnet `CoreWallet` whose CoinJoin account 0 holds the funded UTXO —
    /// the fixture for the CoinJoin-funded deferred-payment reservation tests.
    async fn funded_coinjoin_core_wallet<B: TransactionBroadcaster>(
        broadcaster: Arc<B>,
    ) -> (CoreWallet<B>, WalletSigner, Vec<(DashAddress, u64)>) {
        let (wallet_manager, wallet_id, balance, signer) =
            crate::test_support::funded_coinjoin_wallet_manager().await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(sdk, wallet_manager, wallet_id, broadcaster, balance);
        let recipient = DashAddress::dummy(Network::Testnet, 42);
        (core, signer, vec![(recipient, 1_000_000u64)])
    }

    /// Build + sign a payment exactly as the deferred send path does:
    /// `build_signed` reserves the inputs and leaves the reservation held for
    /// the later broadcast/release.
    async fn build_signed_tx<B: TransactionBroadcaster, S: Signer>(
        core: &CoreWallet<B>,
        account_type: StandardAccountType,
        account_index: u32,
        outputs: &[(DashAddress, u64)],
        signer: &S,
    ) -> Result<Transaction, PlatformWalletError> {
        let mut wm = core.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_and_info_mut(&core.wallet_id())
            .expect("wallet present in manager");
        // Stamp the reservation with `last_processed_height` exactly as the
        // production `build_signed` / `finalize_transaction` paths do, so the
        // registry's age guard (which now reads the same clock) is exercised
        // against a faithfully-stamped reservation.
        let current_height = info.core_wallet.last_processed_height();
        let (managed_account, account) = match account_type {
            StandardAccountType::BIP44Account => (
                info.core_wallet
                    .accounts
                    .standard_bip44_accounts
                    .get_mut(&account_index)
                    .expect("bip44 managed account"),
                wallet
                    .accounts
                    .standard_bip44_accounts
                    .get(&account_index)
                    .expect("bip44 account"),
            ),
            StandardAccountType::BIP32Account => (
                info.core_wallet
                    .accounts
                    .standard_bip32_accounts
                    .get_mut(&account_index)
                    .expect("bip32 managed account"),
                wallet
                    .accounts
                    .standard_bip32_accounts
                    .get(&account_index)
                    .expect("bip32 account"),
            ),
        };
        let mut builder = TransactionBuilder::new()
            .set_current_height(current_height)
            .set_selection_strategy(SelectionStrategy::LargestFirst)
            .set_funding(managed_account, account);
        for (addr, amount) in outputs {
            builder = builder.add_output(addr, *amount);
        }
        let (tx, _fee) = builder
            .build_signed(signer, |addr| {
                managed_account.address_derivation_path(&addr)
            })
            .await
            .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
        Ok(tx)
    }

    /// Happy path: a registered token broadcasts the exact bytes it was built
    /// with, and the token is consumed afterwards.
    #[tokio::test]
    async fn build_then_broadcast_sends_registered_bytes() {
        let broadcaster = Arc::new(RecordingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let expected_bytes = dashcore::consensus::serialize(&tx);
        let expected_txid = tx.txid();

        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;
        assert_eq!(registry.outstanding(), 1);

        // Broadcast through a *clone* of the same wallet instance — the
        // wallet-identity guard must accept it (same `Arc`).
        let txid = registry
            .broadcast(token, &core.clone())
            .await
            .expect("broadcast should succeed");

        assert_eq!(txid, expected_txid, "returned txid must match the built tx");
        assert_eq!(
            broadcaster.last_sent().expect("a tx was sent"),
            expected_bytes,
            "broadcast bytes must be byte-identical to the registered tx"
        );
        assert_eq!(registry.outstanding(), 0, "token consumed after broadcast");
    }

    /// build → release makes the reserved UTXO spendable again: a subsequent
    /// build can reselect the released input.
    #[tokio::test]
    async fn build_then_release_frees_the_reservation() {
        for account_type in [
            StandardAccountType::BIP44Account,
            StandardAccountType::BIP32Account,
        ] {
            let broadcaster = Arc::new(RecordingBroadcaster::new());
            let (core, signer, outputs) = funded_core_wallet(account_type, broadcaster).await;
            let registry = SignedPaymentRegistry::new();

            let tx = build_signed_tx(&core, account_type, 0, &outputs, &signer)
                .await
                .expect("build should succeed");
            let token = registry
                .register(
                    core.clone(),
                    tx,
                    preference(account_type),
                    0,
                    core.last_processed_height().await,
                )
                .await;

            // With the reservation held, an immediate rebuild finds no
            // spendable UTXO and fails.
            let blocked = build_signed_tx(&core, account_type, 0, &outputs, &signer).await;
            assert!(
                matches!(blocked, Err(PlatformWalletError::TransactionBuild(_))),
                "rebuild must fail while the reservation is held for {account_type:?}, got {blocked:?}"
            );

            registry.release(token).await;
            assert_eq!(registry.outstanding(), 0, "token consumed after release");

            // The released input is spendable again — the rebuild succeeds.
            let rebuilt = build_signed_tx(&core, account_type, 0, &outputs, &signer).await;
            assert!(
                rebuilt.is_ok(),
                "rebuild after release should succeed for {account_type:?}, got {rebuilt:?}"
            );
        }
    }

    /// Regression for the deferred CoinJoin reservation leak: a CoinJoin-funded
    /// deferred payment reserves its inputs (finalize reserves for EVERY account
    /// variant), so releasing/abandoning it must free that reservation
    /// immediately — not strand it until key-wallet's 24-block TTL. Before the
    /// fix the registry entry carried only a `StandardAccountType`, so a CoinJoin
    /// funding (which has none) reconciled nothing on release.
    ///
    /// Uses the production `finalize_transaction` path (the atomic
    /// select+reserve+sign the FFI runs), which is the only builder that funds a
    /// CoinJoin account, then registers/releases through the registry exactly as
    /// `core_wallet_signed_payment_finalize` / `_release` do. The CoinJoin
    /// funding path is a sweep (`SelectionStrategy::All`): the single output
    /// drains the input minus fee, so no change address is derived — the only
    /// shape a non-standard CoinJoin account can fund.
    #[tokio::test]
    async fn coinjoin_funded_release_frees_the_reservation_immediately() {
        // A CoinJoin sweep of the funded account to a single recipient.
        fn sweep_builder(recipient: &DashAddress) -> TransactionBuilder {
            TransactionBuilder::new()
                .set_selection_strategy(SelectionStrategy::All)
                .add_output(recipient, 1_000_000)
        }

        let broadcaster = Arc::new(RecordingBroadcaster::new());
        let (core, signer, outputs) = funded_coinjoin_core_wallet(broadcaster).await;
        let recipient = outputs[0].0.clone();
        let registry = SignedPaymentRegistry::new();

        // finalize: atomic select + reserve + sign against the CoinJoin account.
        let finalized = core
            .finalize_transaction(
                sweep_builder(&recipient),
                AccountTypePreference::CoinJoin,
                0,
                &signer,
            )
            .await
            .expect("coinjoin finalize should succeed");

        let token = registry
            .register(
                core.clone(),
                finalized.transaction().clone(),
                AccountTypePreference::CoinJoin,
                0,
                Some(finalized.reservation_height()),
            )
            .await;

        // Reservation held: a second CoinJoin finalize finds no unreserved input.
        let blocked = core
            .finalize_transaction(
                sweep_builder(&recipient),
                AccountTypePreference::CoinJoin,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                blocked,
                Err(PlatformWalletError::CoreInsufficientFunds { .. })
            ),
            "rebuild must fail while the CoinJoin reservation is held, got {blocked:?}"
        );

        // Abandon/nack: the release MUST free the CoinJoin reservation now, not
        // strand it until the TTL backstop.
        registry.release(token).await;
        assert_eq!(registry.outstanding(), 0, "token consumed after release");

        let rebuilt = core
            .finalize_transaction(
                sweep_builder(&recipient),
                AccountTypePreference::CoinJoin,
                0,
                &signer,
            )
            .await;
        assert!(
            rebuilt.is_ok(),
            "releasing a CoinJoin-funded token must free its reservation immediately, \
             got {rebuilt:?}"
        );
    }

    /// A second broadcast of the same token is a typed `StaleToken` error, never
    /// a second send.
    #[tokio::test]
    async fn double_broadcast_is_a_stale_token_error() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;

        registry
            .broadcast(token, &core)
            .await
            .expect("first broadcast should succeed");
        let second = registry.broadcast(token, &core).await;
        assert!(
            matches!(second, Err(SignedPaymentError::StaleToken(t)) if t == token),
            "second broadcast must be StaleToken, got {second:?}"
        );
        assert_eq!(
            broadcaster.count.load(Ordering::SeqCst),
            1,
            "the network must have been hit exactly once"
        );
    }

    /// Releasing twice — or releasing after a broadcast — is a harmless no-op.
    #[tokio::test]
    async fn double_release_is_idempotent() {
        let broadcaster = Arc::new(RecordingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;

        registry.release(token).await;
        // Second release: no panic, no error, still consumed.
        registry.release(token).await;
        assert_eq!(registry.outstanding(), 0);
    }

    /// Broadcasting after a release is a `StaleToken` error (the released token
    /// can never reach the network).
    #[tokio::test]
    async fn broadcast_after_release_is_stale() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;

        registry.release(token).await;
        let sent = registry.broadcast(token, &core).await;
        assert!(
            matches!(sent, Err(SignedPaymentError::StaleToken(_))),
            "broadcast of a released token must be StaleToken, got {sent:?}"
        );
        assert_eq!(
            broadcaster.count.load(Ordering::SeqCst),
            0,
            "nothing was sent"
        );
    }

    /// An unknown token is a `StaleToken` error.
    #[tokio::test]
    async fn unknown_token_is_stale() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, _signer, _outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster).await;
        let registry: SignedPaymentRegistry<CountingBroadcaster> = SignedPaymentRegistry::new();

        let sent = registry.broadcast(9999, &core).await;
        assert!(matches!(sent, Err(SignedPaymentError::StaleToken(9999))));
        // Releasing an unknown token is a no-op, not a panic.
        registry.release(9999).await;
    }

    /// A token minted against one wallet instance cannot be broadcast through a
    /// different (re-created) instance — its reservation lives elsewhere.
    #[tokio::test]
    async fn broadcast_rejects_a_different_wallet_instance() {
        let broadcaster_a = Arc::new(CountingBroadcaster::new());
        let (core_a, signer_a, outputs_a) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::clone(&broadcaster_a),
        )
        .await;
        // A separate wallet-manager instance stands in for a re-created wallet.
        let broadcaster_b = Arc::new(CountingBroadcaster::new());
        let (core_b, _signer_b, _outputs_b) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster_b).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(
            &core_a,
            StandardAccountType::BIP44Account,
            0,
            &outputs_a,
            &signer_a,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core_a.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core_a.last_processed_height().await,
            )
            .await;

        let sent = registry.broadcast(token, &core_b).await;
        assert!(
            matches!(sent, Err(SignedPaymentError::WalletMismatch(t)) if t == token),
            "broadcast through a different wallet instance must be WalletMismatch, got {sent:?}"
        );
        assert_eq!(
            broadcaster_a.count.load(Ordering::SeqCst),
            0,
            "nothing was sent on the original wallet"
        );
        assert_eq!(
            registry.outstanding(),
            1,
            "a mismatched broadcast must NOT consume the rightful owner's token"
        );
    }

    /// An ambiguous ("may already be on the network") broadcast failure keeps
    /// the reservation and surfaces the typed unconfirmed error; the token is
    /// still consumed so it cannot be retried into a double-spend.
    #[tokio::test]
    async fn ambiguous_broadcast_keeps_reservation_and_consumes_token() {
        let broadcaster = Arc::new(AlwaysMaybeSentBroadcaster);
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;

        let sent = registry.broadcast(token, &core).await;
        assert!(
            matches!(
                sent,
                Err(SignedPaymentError::Broadcast(
                    PlatformWalletError::TransactionBroadcastUnconfirmed(_)
                ))
            ),
            "ambiguous failure must surface the typed unconfirmed error, got {sent:?}"
        );
        assert_eq!(registry.outstanding(), 0, "token consumed even on failure");

        // Reservation kept: an immediate rebuild fails at input selection.
        let rebuilt = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await;
        assert!(
            matches!(rebuilt, Err(PlatformWalletError::TransactionBuild(_))),
            "rebuild must fail with the reservation kept, got {rebuilt:?}"
        );
    }

    /// Concurrent broadcasts of the same token serialise on the registry mutex:
    /// exactly one wins, every other gets `StaleToken`, and the network is hit
    /// once.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_broadcasts_serialize_to_one_send() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = Arc::new(SignedPaymentRegistry::new());

        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;

        let mut handles = Vec::new();
        for _ in 0..8 {
            let registry = Arc::clone(&registry);
            let core = core.clone();
            handles.push(tokio::spawn(async move {
                registry.broadcast(token, &core).await
            }));
        }
        let mut successes = 0;
        let mut stale = 0;
        for handle in handles {
            match handle.await.expect("task panicked") {
                Ok(_) => successes += 1,
                Err(SignedPaymentError::StaleToken(_)) => stale += 1,
                Err(other) => panic!("unexpected error: {other:?}"),
            }
        }
        assert_eq!(successes, 1, "exactly one broadcast must win");
        assert_eq!(stale, 7, "every other broadcast must be StaleToken");
        assert_eq!(
            broadcaster.count.load(Ordering::SeqCst),
            1,
            "the network must have been hit exactly once"
        );
    }

    /// Concurrent registrations hand out distinct tokens.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_registers_yield_distinct_tokens() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster).await;
        // One built tx is enough; we register clones of it many times to probe
        // the token allocator, not the reservation logic.
        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let registry = Arc::new(SignedPaymentRegistry::new());

        let mut handles = Vec::new();
        for _ in 0..16 {
            let registry = Arc::clone(&registry);
            let core = core.clone();
            let tx = tx.clone();
            handles.push(tokio::spawn(async move {
                let height = core.last_processed_height().await;
                registry
                    .register(core, tx, AccountTypePreference::BIP44, 0, height)
                    .await
            }));
        }
        let mut tokens = Vec::new();
        for handle in handles {
            tokens.push(handle.await.expect("task panicked"));
        }
        let unique: std::collections::HashSet<_> = tokens.iter().copied().collect();
        assert_eq!(unique.len(), tokens.len(), "all tokens must be distinct");
        assert_eq!(registry.outstanding(), 16);
    }

    /// Force the wallet's `last_processed_height` forward, simulating chain
    /// progress between build/register and a later broadcast/release — the window
    /// in which key-wallet's `ReservationSet` TTL can sweep the funding
    /// reservation. This is the same clock the registry's age guard reads.
    async fn advance_processed_height<B: TransactionBroadcaster>(
        core: &CoreWallet<B>,
        height: u32,
    ) {
        let mut wm = core.wallet_manager.write().await;
        let (_, info) = wm
            .get_wallet_and_info_mut(&core.wallet_id())
            .expect("wallet present in manager");
        info.core_wallet.update_last_processed_height(height);
    }

    /// Once the wallet has synced past `RESERVATION_MAX_AGE_BLOCKS` beyond the
    /// registration height, the reservation could have been swept and
    /// re-selected — so a broadcast must be refused with `StaleReservationToken`
    /// (never a send) and must NOT release the reservation by outpoint (which
    /// could free a newer, unrelated build's reservation).
    #[tokio::test]
    async fn expired_token_broadcast_is_stale_and_keeps_reservation() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = SignedPaymentRegistry::new();

        let registered_height = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;

        // Advance past the age bound but stay below key-wallet's 24-block TTL, so
        // the reservation is provably still held (only our guard has tripped).
        advance_processed_height(&core, registered_height + RESERVATION_MAX_AGE_BLOCKS + 2).await;

        let sent = registry.broadcast(token, &core).await;
        assert!(
            matches!(sent, Err(SignedPaymentError::StaleReservationToken(t)) if t == token),
            "an expired token must broadcast as StaleReservationToken, got {sent:?}"
        );
        assert_eq!(
            broadcaster.count.load(Ordering::SeqCst),
            0,
            "an expired token must never hit the network"
        );
        assert_eq!(registry.outstanding(), 0, "the expired token is dropped");

        // The reservation was NOT released: an immediate rebuild still can't
        // reselect the input (it is reclaimed only by key-wallet's own TTL).
        let rebuilt = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await;
        assert!(
            matches!(rebuilt, Err(PlatformWalletError::TransactionBuild(_))),
            "expired broadcast must not release the reservation, got {rebuilt:?}"
        );
    }

    /// Releasing an expired token must likewise NOT touch the `ReservationSet`:
    /// its outpoint may already belong to a newer build. The token is dropped
    /// and the original reservation is left to key-wallet's TTL sweep.
    #[tokio::test]
    async fn expired_token_release_keeps_reservation() {
        let broadcaster = Arc::new(RecordingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster).await;
        let registry = SignedPaymentRegistry::new();

        let registered_height = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;

        advance_processed_height(&core, registered_height + RESERVATION_MAX_AGE_BLOCKS + 2).await;

        registry.release(token).await;
        assert_eq!(registry.outstanding(), 0, "the expired token is dropped");

        // Reservation intentionally kept (not released by outpoint): rebuild
        // still fails until the TTL backstop reclaims it.
        let rebuilt = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await;
        assert!(
            matches!(rebuilt, Err(PlatformWalletError::TransactionBuild(_))),
            "expired release must not free the reservation by outpoint, got {rebuilt:?}"
        );
    }

    /// Two wallets sharing one multi-wallet `PlatformWalletManager` have the same
    /// `wallet_manager` `Arc` (so `Arc::ptr_eq` alone can't tell them apart); the
    /// `wallet_id` comparison must reject a token broadcast through the sibling.
    #[tokio::test]
    async fn broadcast_rejects_same_manager_different_wallet_id() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;

        // A sibling handle over the SAME manager Arc but a different wallet_id —
        // `Arc::ptr_eq` on `wallet_manager` is true, so only the wallet_id check
        // distinguishes it.
        let mut sibling = core.clone();
        sibling.wallet_id[0] ^= 0xFF;
        assert!(Arc::ptr_eq(&core.wallet_manager, &sibling.wallet_manager));

        let sent = registry.broadcast(token, &sibling).await;
        assert!(
            matches!(sent, Err(SignedPaymentError::WalletMismatch(t)) if t == token),
            "a sibling wallet in the same manager must be WalletMismatch, got {sent:?}"
        );
        assert_eq!(
            broadcaster.count.load(Ordering::SeqCst),
            0,
            "nothing was sent for the mismatched wallet"
        );
        assert_eq!(
            registry.outstanding(),
            1,
            "a mismatched broadcast must NOT consume the rightful owner's token"
        );
    }

    /// Destroying a wallet sweeps only its own tokens from the registry, so its
    /// captured `CoreWallet` clone stops pinning the `WalletManager` alive —
    /// other wallets' tokens are untouched.
    #[tokio::test]
    async fn remove_entries_for_wallet_drops_only_that_wallets_tokens() {
        let (core_a, signer_a, outputs_a) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(CountingBroadcaster::new()),
        )
        .await;
        let (core_b, signer_b, outputs_b) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(CountingBroadcaster::new()),
        )
        .await;
        let registry = SignedPaymentRegistry::new();

        let tx_a = build_signed_tx(
            &core_a,
            StandardAccountType::BIP44Account,
            0,
            &outputs_a,
            &signer_a,
        )
        .await
        .expect("build A should succeed");
        let token_a = registry
            .register(
                core_a.clone(),
                tx_a,
                AccountTypePreference::BIP44,
                0,
                core_a.last_processed_height().await,
            )
            .await;
        let tx_b = build_signed_tx(
            &core_b,
            StandardAccountType::BIP44Account,
            0,
            &outputs_b,
            &signer_b,
        )
        .await
        .expect("build B should succeed");
        let _token_b = registry
            .register(
                core_b.clone(),
                tx_b,
                AccountTypePreference::BIP44,
                0,
                core_b.last_processed_height().await,
            )
            .await;
        assert_eq!(registry.outstanding(), 2);

        let removed = registry.remove_entries_for_wallet(&core_a);
        assert_eq!(removed, 1, "exactly wallet A's one token is swept");
        assert_eq!(registry.outstanding(), 1, "wallet B's token survives");

        // Wallet A's token is gone: broadcasting it is a plain StaleToken.
        let sent = registry.broadcast(token_a, &core_a).await;
        assert!(
            matches!(sent, Err(SignedPaymentError::StaleToken(t)) if t == token_a),
            "a swept token must be StaleToken, got {sent:?}"
        );

        // Generation teardown drops WITHOUT releasing: A's input stays reserved
        // (the account's ReservationSet is conceptually gone with the wallet, so
        // there is nothing to reconcile). An immediate rebuild on A still fails.
        let blocked = build_signed_tx(
            &core_a,
            StandardAccountType::BIP44Account,
            0,
            &outputs_a,
            &signer_a,
        )
        .await;
        assert!(
            matches!(blocked, Err(PlatformWalletError::TransactionBuild(_))),
            "remove_entries_for_wallet must NOT release by outpoint, got {blocked:?}"
        );
    }

    /// Regression for the final-alias-destroy leak: `release_entries_for_wallet`
    /// must RELEASE each of the generation's reservations against the still-live
    /// wallet, not merely drop them, so a wallet handed out again can respend the
    /// inputs instead of leaving them reserved until key-wallet's TTL. This is
    /// the destroy-time half of the teardown policy, and the counterpart to
    /// `remove_entries_for_wallet` (drop-only, at actual generation teardown).
    #[tokio::test]
    async fn release_entries_for_wallet_frees_the_reservation() {
        let broadcaster = Arc::new(RecordingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let _token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core.last_processed_height().await,
            )
            .await;

        // Reservation held: an immediate rebuild fails at input selection.
        let blocked = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await;
        assert!(
            matches!(blocked, Err(PlatformWalletError::TransactionBuild(_))),
            "rebuild must fail while the reservation is held, got {blocked:?}"
        );

        // Final-alias destroy path: release (not drop) the generation's tokens.
        let released = registry.release_entries_for_wallet(&core).await;
        assert_eq!(released, 1, "the generation's one token is reconciled");
        assert_eq!(registry.outstanding(), 0);

        // The released input is spendable again — the rebuild now succeeds.
        let rebuilt = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await;
        assert!(
            rebuilt.is_ok(),
            "release_entries_for_wallet must free the reservation, got {rebuilt:?}"
        );
    }

    /// Regression for the wrong-wallet-broadcast token theft: a mismatched
    /// caller must return `WalletMismatch` WITHOUT consuming the entry, so the
    /// rightful owner's token — and its reservation — survive and it can still
    /// be broadcast. Previously `broadcast` removed the entry and *then*
    /// validated, so a wrong-wallet caller destroyed the owner's token and
    /// stranded its reservation until the TTL backstop.
    #[tokio::test]
    async fn wrong_wallet_broadcast_preserves_the_owners_token() {
        let broadcaster_a = Arc::new(CountingBroadcaster::new());
        let (core_a, signer_a, outputs_a) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::clone(&broadcaster_a),
        )
        .await;
        // A separate wallet-manager instance is a different generation.
        let broadcaster_b = Arc::new(CountingBroadcaster::new());
        let (core_b, _signer_b, _outputs_b) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster_b).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(
            &core_a,
            StandardAccountType::BIP44Account,
            0,
            &outputs_a,
            &signer_a,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(
                core_a.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                core_a.last_processed_height().await,
            )
            .await;

        // Wrong wallet: mismatch, and the token MUST survive for its owner.
        let mismatched = registry.broadcast(token, &core_b).await;
        assert!(
            matches!(mismatched, Err(SignedPaymentError::WalletMismatch(t)) if t == token),
            "a wrong-wallet broadcast must be WalletMismatch, got {mismatched:?}"
        );
        assert_eq!(
            registry.outstanding(),
            1,
            "the owner's token must survive a wrong-wallet broadcast"
        );
        assert_eq!(
            broadcaster_a.count.load(Ordering::SeqCst),
            0,
            "nothing was sent for the mismatched caller"
        );

        // The rightful owner can still broadcast its own token.
        registry
            .broadcast(token, &core_a)
            .await
            .expect("the owner's broadcast should still succeed");
        assert_eq!(
            broadcaster_a.count.load(Ordering::SeqCst),
            1,
            "the owner's broadcast must reach the network exactly once"
        );
        assert_eq!(
            registry.outstanding(),
            0,
            "the token is consumed by its owner"
        );
    }

    /// Regression for the "reservation height captured before signing, token
    /// height sampled after" gap: `register` takes the reservation's OWN stamp
    /// height, so a slow external signer that let `last_processed_height`
    /// advance between stamping and registration cannot make the token look
    /// younger than the reservation it covers.
    ///
    /// The wallet is advanced to `H + (MAX_AGE - 1)` *before* the token is
    /// registered — modelling a signer slow enough that a fresh
    /// post-signing sample would read that higher height. The token is
    /// registered with the reservation's real stamp height `H`. One more block
    /// (`H + MAX_AGE`) then trips the guard: exactly `MAX_AGE` past the
    /// reservation. Under the old behaviour (sampling `last_processed_height`
    /// at register time) the baseline would have been `H + MAX_AGE - 1`, so the
    /// same final height would read an age of 1 and the token would broadcast —
    /// this test would fail. Baselining on the passed-in reservation height is
    /// what keeps the guard tripping before key-wallet's TTL sweep.
    #[tokio::test]
    async fn register_baselines_on_reservation_height_not_a_post_signing_sample() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = SignedPaymentRegistry::new();

        let reservation_height = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let tx = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");

        // Slow signer: the wallet advanced to just under the age bound while
        // signing. A fresh sample here would read `reservation_height +
        // MAX_AGE - 1`.
        advance_processed_height(&core, reservation_height + RESERVATION_MAX_AGE_BLOCKS - 1).await;

        // Register with the reservation's OWN stamp height, not a fresh sample.
        let token = registry
            .register(
                core.clone(),
                tx,
                AccountTypePreference::BIP44,
                0,
                Some(reservation_height),
            )
            .await;

        // One block past the reservation height (still below the 24-block TTL)
        // trips the guard because the baseline is `reservation_height`.
        advance_processed_height(&core, reservation_height + RESERVATION_MAX_AGE_BLOCKS).await;

        let sent = registry.broadcast(token, &core).await;
        assert!(
            matches!(sent, Err(SignedPaymentError::StaleReservationToken(t)) if t == token),
            "a token past MAX_AGE from its reservation height must be StaleReservationToken, \
             got {sent:?}"
        );
        assert_eq!(
            broadcaster.count.load(Ordering::SeqCst),
            0,
            "the network must not have been hit"
        );
    }
}
