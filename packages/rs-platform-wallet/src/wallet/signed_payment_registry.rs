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
//!   the same identity the V2 finalized-transaction handle path
//!   (`dashpay/platform#4196`) uses). Two
//!   wallets sharing one multi-wallet `PlatformWalletManager`, or a re-created
//!   wallet under the same id whose in-memory `ReservationSet` no longer holds
//!   the inputs, are both told apart: broadcasting through either is a
//!   [`SignedPaymentError::WalletMismatch`] rather than a spend against stale
//!   state. That check happens at the registry lock, but the reservation
//!   cleanup that follows it runs later, off the registry lock — so the
//!   check-then-cleanup is *not* one atomic step against a same-id recreation.
//!   The cleanup is made safe on its own: every reservation release
//!   ([`CoreWallet::release_transaction_reservation`]) re-validates the
//!   generation and mutates the `ReservationSet` under a single manager-lock
//!   hold, acting only if the wallet still registered under the id is the same
//!   generation the token captured (its per-generation balance `Arc`). A
//!   recreation needs the manager write lock, so it cannot slip between that
//!   check and the release; a stale token can therefore never free a re-created
//!   generation's reservation.
//! * A token has a bounded lifetime ([`RESERVATION_MAX_AGE_BLOCKS`]). Once the
//!   wallet's `last_processed_height` has advanced far enough past the height at
//!   which `build_signed` / `finalize_transaction` stamped the reservation that
//!   key-wallet's own `ReservationSet` TTL could have swept and re-selected the
//!   funding UTXO for an unrelated build, a **broadcast** would spend against
//!   state that may no longer be its own — so it is refused with
//!   [`SignedPaymentError::StaleReservationToken`] and the caller must rebuild.
//!   The stale entry's *reservation*, however, is still reconciled on the way
//!   out: key-wallet's `release_reservation_if_owner` (the per-reservation
//!   ownership check the funding token unlocks) frees the inputs only while
//!   this build still owns them and no-ops after a sweep/re-reservation, so
//!   releasing is safe at any age — and necessary, since below the TTL the
//!   inputs are typically still held and the demanded rebuild would otherwise
//!   fail selection. Only a token-less entry falls back to drop-without-release
//!   (its by-outpoint release is unguarded).
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
use std::sync::{Arc, Mutex, MutexGuard};

use dashcore::{Transaction, Txid};
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
// key-wallet's UTXO-reservation token, distinct from this registry's own
// `ReservationToken` (the u64 payment handle below). Aliased so the two never
// blur: the funding token identifies the reserved *inputs* for an owner-guarded
// release, the payment handle identifies the *registered payment*.
use key_wallet::ReservationToken as FundingReservationToken;

use crate::broadcaster::TransactionBroadcaster;
use crate::wallet::core::{CoreWallet, SignedCoreTransaction};
use crate::PlatformWalletError;

/// Opaque handle to a registered, signed-but-unsent payment. Minted by
/// [`SignedPaymentRegistry::register`]; consumed by
/// [`SignedPaymentRegistry::broadcast`] or
/// [`SignedPaymentRegistry::release`]. Values are unique for the process
/// lifetime and never reused, so a stale token can always be recognised.
///
/// A distinct newtype rather than a bare `u64` alias so a payment handle can
/// never be silently confused with any other numeric identifier (the funding
/// [`FundingReservationToken`], an account index, a raw height). It crosses the
/// C ABI as a `u64` — [`from`](ReservationToken::from) / [`as_u64`](ReservationToken::as_u64)
/// are the only conversions, applied at the FFI boundary.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReservationToken(u64);

impl ReservationToken {
    /// The raw wire value handed back across the FFI boundary to the host.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl From<u64> for ReservationToken {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<ReservationToken> for u64 {
    fn from(token: ReservationToken) -> Self {
        token.0
    }
}

impl std::fmt::Display for ReservationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

/// Whether a token stamped at `registered_height` is too old to act on at
/// `current_height` (see [`RESERVATION_MAX_AGE_BLOCKS`]). The registration
/// height is mandatory — it is derived from the finalized
/// [`SignedCoreTransaction::reservation_height`](crate::SignedCoreTransaction)
/// the registry consumed.
///
/// An unknown *current* height means the wallet is gone from the manager, which
/// disables the guard (`None` → not expired). That is safe only because every
/// caller establishes liveness first and so never reaches here with a removed
/// wallet: [`broadcast`](SignedPaymentRegistry::broadcast) refuses with
/// [`SignedPaymentError::WalletRemoved`] before sampling the height, and
/// [`reconcile_removed_entry`](SignedPaymentRegistry::reconcile_removed_entry)'s
/// release is itself generation-bound and no-ops on a missing wallet. The
/// earlier claim that "the wallet-mismatch / account-lookup paths already reject
/// those cases" was wrong for the broadcast path — `is_same_generation` compares
/// handles (a removed generation matches itself) and the broadcast path performs
/// no account lookup at all (`dashpay/platform#4185`).
fn reservation_expired(registered_height: u32, current_height: Option<u32>) -> bool {
    match current_height {
        Some(current) => current.saturating_sub(registered_height) >= RESERVATION_MAX_AGE_BLOCKS,
        None => false,
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

    /// The wallet the token was minted against is no longer registered in the
    /// manager — it was removed (`platform_wallet_manager_remove_wallet`), so
    /// its accounts and their `ReservationSet`s ceased to exist along with it.
    ///
    /// Distinct from [`WalletMismatch`](Self::WalletMismatch), which means a
    /// *different* live generation answers to the same id. Here there is no live
    /// generation at all, so there is nothing to broadcast against and nothing
    /// to reconcile: the token is dropped WITHOUT releasing (a release by
    /// outpoint would have no `ReservationSet` to act on, and the reservation
    /// died with the generation).
    ///
    /// Refusing here is what stops a retained handle from pushing a removed
    /// wallet's payment onto the network after the host believed the wallet was
    /// gone (`dashpay/platform#4185`). The network was NOT touched.
    #[error("reservation token {0} belongs to a wallet that is no longer in the manager")]
    WalletRemoved(ReservationToken),

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

/// The wallet handed to [`SignedPaymentRegistry::register`] is **not** the
/// generation the payment was finalized against, so registering it would bind
/// the reservation to the wrong wallet. Registration is refused up front rather
/// than minting a token that later broadcasts through — and runs cleanup
/// against — a wallet whose `ReservationSet` never held the inputs
/// (`dashpay/platform#4185`).
///
/// The rejected [`SignedCoreTransaction`] is returned so its held funding
/// reservation is **never stranded**: the caller still owns it and can release
/// it through the correct wallet ([`CoreWallet::abandon_transaction`]) or drop
/// it. This mirrors the owner-guarded discipline of the rest of the deferred
/// path — an ownership object is never dropped on a failure path without the
/// caller getting a chance to reconcile its reservation.
#[derive(Debug)]
pub struct RegisterWrongGeneration {
    /// The finalized payment `register` refused to bind, handed back intact.
    pub signed: SignedCoreTransaction,
}

impl std::fmt::Display for RegisterWrongGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("registration wallet is not the generation the payment was finalized against")
    }
}

impl std::error::Error for RegisterWrongGeneration {}

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
    /// Wallet `last_processed_height` captured inside the funding critical
    /// section — the exact clock `finalize_transaction` stamps the funding
    /// reservation with (`SignedCoreTransaction::reservation_height`). Compared
    /// against the wallet's current `last_processed_height` to refuse a
    /// broadcast/release once the reservation could plausibly have been swept by
    /// key-wallet's TTL (see [`RESERVATION_MAX_AGE_BLOCKS`]). Mandatory: it is
    /// derived from the consumed ownership object, never sampled independently.
    registered_height: u32,
    /// The key-wallet [`FundingReservationToken`] stamped onto the funding
    /// inputs when `finalize_transaction` reserved them
    /// (`SignedCoreTransaction::reservation_token`), or `None` if the build
    /// reserved nothing. A deferred payment can sit here across many blocks, so
    /// key-wallet's TTL may sweep its reservation and a concurrent build
    /// re-reserve the same inputs under a new token before this entry is
    /// broadcast or released. Presenting this token to the owner-guarded release
    /// frees only inputs still owned by this build, never the other build's
    /// (`dashpay/platform#4185`).
    funding_reservation_token: Option<FundingReservationToken>,
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

    /// Take ownership of a finalized [`SignedCoreTransaction`] (whose funding
    /// UTXOs `finalize` already reserved) and return an opaque token for a later
    /// [`broadcast`](Self::broadcast) or [`release`](Self::release).
    ///
    /// `signed` is **consumed**, which is what enforces unique reservation
    /// ownership: `SignedCoreTransaction` is not `Clone`, so a single finalize
    /// can be registered at most once — there is no way to mint two live tokens
    /// that name the same held reservation (`dashpay/platform#4185`). The built
    /// transaction, the funding account, the mandatory reservation height
    /// (`SignedCoreTransaction::reservation_height` — captured inside the
    /// funding critical section before the potentially-slow external signer ran,
    /// so the age guard measures the reservation's true age rather than a
    /// post-signing sample), and the owner-guard token
    /// (`SignedCoreTransaction::reservation_token`) are all derived from that
    /// object here rather than supplied independently by the caller.
    ///
    /// `core` is the wallet the token is bound to for its later broadcast /
    /// release. It **must** be the same wallet *generation* the payment was
    /// finalized against — validated here against the unforgeable
    /// `origin_generation` marker `SignedCoreTransaction` captured at finalize.
    /// Binding to any other wallet is refused with [`RegisterWrongGeneration`]
    /// (the rejected `signed` handed back so its reservation is not stranded):
    /// otherwise safe public code could finalize through wallet A and
    /// `register(core_b, signed_from_a)`, after which broadcasting through B
    /// would pass the generation check and submit A's transaction through B's
    /// broadcaster while cleanup ran against B and A's real reservation leaked
    /// until its TTL. Deriving/validating the core from the consumed object
    /// (rather than trusting a separate argument) upholds the documented
    /// guarantee that a token is bound to the generation whose `ReservationSet`
    /// owns the inputs.
    ///
    /// Synchronous **by design**: the body performs the reservation-owning
    /// insertion with no `.await`, so there is no future that could be dropped
    /// before its first poll and silently drop the consumed `signed` — and its
    /// held reservation — without inserting it. An `async fn` here would only
    /// move `signed` into a future whose body runs on the first poll; dropping
    /// that future before polling would leak the reservation to key-wallet's TTL
    /// (`dashpay/platform#4185`). Callers invoke it directly.
    ///
    /// # Liveness is the caller's obligation
    ///
    /// The generation check here is `signed`-relative: it proves `core` is the
    /// wallet that *finalized* the payment. It says nothing about whether that
    /// wallet is still registered in the manager, and being synchronous it
    /// cannot ask (the manager lock is `async`). `finalize_transaction` drops
    /// the manager write lock before awaiting the signer, so a teardown can run
    /// to completion — sweep included — while a finalize is mid-signature; the
    /// `register` that follows would then insert a live token for a removed
    /// generation, defeating the documented teardown invariant that dropping
    /// tokens makes stale handles inert.
    ///
    /// Callers must therefore hold
    /// [`CoreWallet::generation_payment_guard`] — the finalizing generation's own
    /// lifecycle gate — across `CoreWallet::is_current_generation` and this call,
    /// and abandon the payment (releasing its reservation) when the wallet is
    /// gone. The FFI's `core_wallet_signed_payment_finalize` is the production
    /// caller and does exactly that.
    ///
    /// The gate is acquired **after** the external signer returns, not around it:
    /// holding a generation's gate across an open signing prompt would stall that
    /// wallet's teardown for as long as the user takes, and the liveness check
    /// makes it unnecessary. A finalizer whose wallet was torn down mid-signature
    /// therefore observes the missing generation at its check and abandons
    /// instead of registering.
    pub fn register(
        &self,
        core: CoreWallet<B>,
        signed: SignedCoreTransaction,
    ) -> Result<ReservationToken, RegisterWrongGeneration> {
        // Bind the payment to the EXACT generation it was finalized against.
        // `core.generation()` and `signed.origin_generation()` are the same kind
        // of per-generation balance `Arc` `is_same_generation` pointer-compares;
        // a mismatch means `core` is a different (switched / stale / unrelated)
        // wallet than the one whose `ReservationSet` holds the inputs. Refuse
        // BEFORE consuming `signed`, and hand it back so the caller can reconcile
        // its reservation.
        if !Arc::ptr_eq(core.generation(), signed.origin_generation()) {
            return Err(RegisterWrongGeneration { signed });
        }
        let parts = signed.into_registered_parts();
        let token = ReservationToken(self.next_token.fetch_add(1, Ordering::SeqCst));
        self.lock().insert(
            token,
            RegisteredPayment {
                core,
                tx: parts.transaction,
                account_type: parts.funding_account_type,
                account_index: parts.funding_account_index,
                registered_height: parts.reservation_height,
                funding_reservation_token: parts.reservation_token,
            },
        );
        Ok(token)
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
        //
        // Hold `current`'s OWN generation lifecycle gate for the whole operation.
        // That generation's teardown needs the exclusive side, so it cannot
        // interleave between the liveness check below and the send: either the
        // wallet is gone before we enter (our entry was already swept →
        // `StaleToken`), or it stays live until we leave. Shared, so concurrent
        // payments — on this generation and on every other — are unaffected, and
        // scoped per generation, so holding it across the network send below
        // blocks only THIS wallet's teardown rather than every wallet's
        // (`dashpay/platform#4185`).
        //
        // Taking `current`'s gate rather than the entry's is sound because the
        // only path that proceeds past the check below is one where
        // `entry.core.is_same_generation(current)` held — i.e. they are the same
        // generation and therefore the same gate. A mismatched caller returns
        // without touching the entry or the network.
        let _lifecycle = current.generation_payment_guard().await;

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

        // Refuse a token whose wallet is no longer registered in the manager.
        //
        // `is_same_generation` above compares two HANDLES, so it passes for a
        // removed generation: both sides are the same removed wallet. Nothing
        // further down re-checks — `broadcast_payment_releasing_reservation`
        // goes straight to the broadcaster with no manager lookup, and the age
        // guard below is *disabled* for a removed wallet
        // (`last_processed_height` is `None`). So without this check a retained
        // handle broadcasts a removed wallet's payment onto the network, and the
        // teardown sweep cannot stop it: the sweep and the removal are one
        // linearization point, but a broadcast that entered the gate first is
        // outside it (`dashpay/platform#4185`).
        //
        // The entry is already removed, so we drop it WITHOUT releasing — the
        // reservation ceased to exist with the generation, and a release by
        // outpoint has no live `ReservationSet` to act on. Held under the
        // lifecycle gate, so this is not a check-then-act: the wallet cannot be
        // removed between here and the send below.
        if !current.is_current_generation().await {
            return Err(SignedPaymentError::WalletRemoved(token));
        }

        // Refuse to SEND a token whose reservation could already have been
        // swept and re-selected by an unrelated build — but reconcile its
        // reservation first. With the build's owner token present the release
        // is safe at ANY age: `release_reservation_if_owner` frees the inputs
        // only while this build still owns them and no-ops after a TTL sweep
        // or re-reservation transferred ownership. Between the guard bound
        // (RESERVATION_MAX_AGE_BLOCKS) and key-wallet's TTL the reservation is
        // typically STILL HELD, so dropping without releasing would strand the
        // inputs for several more blocks while telling the caller to rebuild —
        // and the rebuild would fail selection. Only a token-less entry falls
        // back to the drop-without-release policy (an unguarded by-outpoint
        // release could free a newer build's reservation).
        if reservation_expired(
            entry.registered_height,
            current.last_processed_height().await,
        ) {
            Self::reconcile_removed_entry(entry).await;
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
                entry.funding_reservation_token,
            )
            .await?;
        Ok(txid)
    }

    /// Reconcile one already-removed entry's reservation, bound to the token's
    /// own wallet generation and — when the build stamped an owner token —
    /// owner-guarded, which makes it safe at ANY age:
    /// `release_reservation_if_owner` frees the inputs only while this build
    /// still owns them, and no-ops once a TTL sweep or a re-reservation has
    /// transferred ownership. Between [`RESERVATION_MAX_AGE_BLOCKS`] and
    /// key-wallet's own TTL the reservation is typically still held, so
    /// releasing here is what lets an immediate rebuild reselect the inputs
    /// instead of stranding them until the TTL backstop.
    ///
    /// Only a token-less entry (`funding_reservation_token == None` — a build
    /// that reserved nothing, unreachable on the funded finalize path) honours
    /// the age guard and is dropped without touching the `ReservationSet`: its
    /// only release primitive is the unguarded by-outpoint form, which after a
    /// sweep could free a newer build's reservation.
    async fn reconcile_removed_entry(entry: RegisteredPayment<B>) {
        if entry.funding_reservation_token.is_none()
            && reservation_expired(
                entry.registered_height,
                entry.core.last_processed_height().await,
            )
        {
            return;
        }
        entry
            .core
            .release_transaction_reservation(
                entry.account_type,
                entry.account_index,
                &entry.tx,
                entry.funding_reservation_token,
            )
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
        // Same per-generation lifecycle gate as `broadcast`: the reconciliation
        // below reads the manager to bind its release to a live generation, so
        // that generation's teardown must not interleave between taking the entry
        // and acting on it.
        //
        // No wallet handle is threaded in, so the gate has to come from the entry
        // itself. PEEK the entry's generation without consuming it, drop the map
        // lock (a `std::sync::Mutex` — it must never be held across an `.await`),
        // take that generation's gate, and only then consume. Both ways the peek
        // can go stale are already the correct outcome: if a teardown swept the
        // entry, or a concurrent release/broadcast consumed it, the `remove`
        // below returns `None` and this is the documented idempotent no-op.
        let generation = {
            let entries = self.lock();
            match entries.get(&token) {
                // Unknown / already consumed — idempotent no-op.
                None => return,
                Some(entry) => Arc::clone(entry.core.generation()),
            }
        };
        let _lifecycle = generation.payment_guard().await;

        let entry = { self.lock().remove(&token) };
        let Some(entry) = entry else {
            // Swept or consumed while we were acquiring the gate — no-op.
            return;
        };
        Self::reconcile_removed_entry(entry).await;
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
    ///
    /// # Must be called under the removed generation's [`WalletGeneration::teardown_guard`]
    ///
    /// Dropping the tokens is only half of teardown; the other half is the
    /// manager removal itself, and the two are one atomic step only if the
    /// caller holds that generation's exclusive lifecycle gate across BOTH.
    /// Sweeping without it leaves two windows a payment operation slips through —
    /// a broadcast between the removal and this sweep still finds its entry, and
    /// an in-flight finalizer registers a fresh token *after* this sweep has run
    /// (`dashpay/platform#4185`). This function cannot take the gate itself: it
    /// is synchronous, and the removal it must be atomic with is `async`.
    ///
    /// [`PlatformWalletManager::remove_wallet_with_teardown`](crate::PlatformWalletManager::remove_wallet_with_teardown)
    /// is the supported way to satisfy this: it holds the gate across the removal
    /// and runs the sweep as its teardown hook, so the ordering cannot be got
    /// wrong by a caller — including a direct Rust embedder that never goes
    /// through the FFI.
    pub fn remove_entries_for_wallet(&self, wallet: &CoreWallet<B>) -> usize {
        let mut entries = self.lock();
        let before = entries.len();
        entries.retain(|_, entry| !entry.core.is_same_generation(wallet));
        before - entries.len()
    }

    /// Number of outstanding (registered but not yet broadcast/released) tokens.
    /// Exposed under `test-utils` so downstream FFI-layer tests (e.g. the
    /// `platform_wallet_destroy` lifecycle tests) can observe registry state.
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

    use super::{
        RegisterWrongGeneration, ReservationToken, SignedPaymentError, SignedPaymentRegistry,
        RESERVATION_MAX_AGE_BLOCKS,
    };
    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::test_support::{
        funded_wallet_manager, AlwaysMaybeSentBroadcaster, AlwaysRejectedBroadcaster, WalletSigner,
    };
    use crate::wallet::core::{CoreWallet, SignedCoreTransaction};
    use crate::PlatformWalletError;

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
    /// `build_signed_reserved` reserves the inputs and leaves the reservation
    /// held for the later broadcast/release. Returns a finalized
    /// [`SignedCoreTransaction`] — the same non-`Clone` ownership object the
    /// production `finalize_transaction` path yields — so the test hands it to
    /// [`SignedPaymentRegistry::register`] exactly once (it captures the funding
    /// account, the reservation height, and the key-wallet owner-guard token).
    async fn build_signed_tx<B: TransactionBroadcaster, S: Signer>(
        core: &CoreWallet<B>,
        account_type: StandardAccountType,
        account_index: u32,
        outputs: &[(DashAddress, u64)],
        signer: &S,
    ) -> Result<SignedCoreTransaction, PlatformWalletError> {
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
        let (tx, fee, reservation_token) = builder
            .build_signed_reserved(signer, |addr| {
                managed_account.address_derivation_path(&addr)
            })
            .await
            .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
        Ok(SignedCoreTransaction::new_for_test(
            tx,
            fee,
            preference(account_type),
            account_index,
            current_height,
            reservation_token,
            // Stamp the finalizing generation so registering through this same
            // `core` passes the registry's generation binding, exactly as the
            // production finalize path does.
            core.generation().clone(),
        ))
    }

    /// Happy path: a registered token broadcasts the exact bytes it was built
    /// with, and the token is consumed afterwards.
    #[tokio::test]
    async fn build_then_broadcast_sends_registered_bytes() {
        let broadcaster = Arc::new(RecordingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = SignedPaymentRegistry::new();

        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let expected_bytes = dashcore::consensus::serialize(signed.transaction());
        let expected_txid = signed.transaction().txid();

        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");
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

            let signed = build_signed_tx(&core, account_type, 0, &outputs, &signer)
                .await
                .expect("build should succeed");
            let token = registry
                .register(core.clone(), signed)
                .expect("test registers with the finalizing generation");

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
            .register(core.clone(), finalized)
            .expect("test registers with the finalizing generation");

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

        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

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

        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

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

        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

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

        let unknown = ReservationToken::from(9999);
        let sent = registry.broadcast(unknown, &core).await;
        assert!(matches!(sent, Err(SignedPaymentError::StaleToken(t)) if t == unknown));
        // Releasing an unknown token is a no-op, not a panic.
        registry.release(unknown).await;
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

        let signed = build_signed_tx(
            &core_a,
            StandardAccountType::BIP44Account,
            0,
            &outputs_a,
            &signer_a,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core_a.clone(), signed)
            .expect("test registers with the finalizing generation");

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

    /// Regression for `dashpay/platform#4185` blocker: registration must bind the
    /// token to the SAME wallet generation the payment was finalized against, not
    /// to a separately-supplied wallet. Registering a payment finalized through
    /// wallet A through an unrelated wallet B is refused up front with
    /// [`RegisterWrongGeneration`], no token is minted (so B can never broadcast
    /// A's transaction through B's broadcaster or run cleanup against B), and the
    /// rejected `SignedCoreTransaction` is handed back so A's reservation is not
    /// stranded — releasing it through A frees the input for an immediate rebuild.
    #[tokio::test]
    async fn register_rejects_a_different_wallet_generation() {
        let (core_a, signer_a, outputs_a) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(CountingBroadcaster::new()),
        )
        .await;
        // A separate wallet-manager instance stands in for an unrelated / re-created
        // generation: same account shape, different generation-identity `Arc`.
        let (core_b, _signer_b, _outputs_b) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(CountingBroadcaster::new()),
        )
        .await;
        let registry = SignedPaymentRegistry::new();

        let signed = build_signed_tx(
            &core_a,
            StandardAccountType::BIP44Account,
            0,
            &outputs_a,
            &signer_a,
        )
        .await
        .expect("build should succeed");

        // Registering A's finalized payment through wallet B is refused, and no
        // token is minted.
        let baseline = registry.outstanding();
        let rejected = registry.register(core_b.clone(), signed);
        let RegisterWrongGeneration { signed } = match rejected {
            Err(err) => err,
            Ok(_) => panic!("registering through a different generation must be refused"),
        };
        assert_eq!(
            registry.outstanding(),
            baseline,
            "a rejected registration must not mint a token"
        );

        // Registering through the correct generation (an alias of A) is accepted:
        // the guard binds to generation identity, not wallet-manager pointer.
        let token = registry
            .register(core_a.clone(), signed)
            .expect("registering through the finalizing generation must be accepted");
        assert_eq!(registry.outstanding(), baseline + 1);

        // The reservation is A's and is reachable: releasing the token frees the
        // input, so an immediate rebuild on A succeeds — nothing was stranded.
        registry.release(token).await;
        assert_eq!(registry.outstanding(), baseline);
        let rebuilt = build_signed_tx(
            &core_a,
            StandardAccountType::BIP44Account,
            0,
            &outputs_a,
            &signer_a,
        )
        .await;
        assert!(
            rebuilt.is_ok(),
            "the reservation must be reachable after a rejected mis-binding, got {rebuilt:?}"
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

        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

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

        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

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

    // NOTE: the former `concurrent_registers_yield_distinct_tokens` test
    // registered sixteen clones of ONE reserved transaction to probe the token
    // allocator. That is exactly the duplicate-capability pattern unique
    // ownership now forbids: `register` consumes a non-`Clone`
    // `SignedCoreTransaction`, so a single reservation can be registered at most
    // once (`dashpay/platform#4185`). Token distinctness is guaranteed by
    // construction (the `AtomicU64` allocator), and concurrent consumption is
    // covered by `concurrent_broadcasts_serialize_to_one_send`.

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
    /// registration height, a broadcast must be refused with
    /// `StaleReservationToken` (never a send) — but the entry's reservation is
    /// reconciled OWNER-GUARDED on the way out: below key-wallet's TTL the
    /// reservation is still this build's, `release_reservation_if_owner` frees
    /// it, and the immediate rebuild the error demands can actually reselect
    /// the inputs. (Had a sweep already transferred ownership, the same call
    /// would no-op — safe either way.)
    #[tokio::test]
    async fn expired_token_broadcast_is_stale_and_releases_owner_guarded() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = SignedPaymentRegistry::new();

        let registered_height = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

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

        // The reservation WAS released (owner-guarded — still ours below the
        // TTL): the immediate rebuild the error demands reselects the inputs.
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
            "stale broadcast must release owner-guarded so a rebuild succeeds, got {rebuilt:?}"
        );
    }

    /// Releasing an expired token reconciles owner-guarded too: below the TTL
    /// the reservation is still this build's, so `releaseReservation` reported
    /// success must actually free the inputs for an immediate rebuild (the old
    /// drop-without-release left them stranded until the TTL backstop).
    #[tokio::test]
    async fn expired_token_release_frees_reservation_owner_guarded() {
        let broadcaster = Arc::new(RecordingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster).await;
        let registry = SignedPaymentRegistry::new();

        let registered_height = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

        advance_processed_height(&core, registered_height + RESERVATION_MAX_AGE_BLOCKS + 2).await;

        registry.release(token).await;
        assert_eq!(registry.outstanding(), 0, "the expired token is dropped");

        // Owner-guarded release freed the inputs: an immediate rebuild
        // reselects them.
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
            "stale release must free the still-owned reservation, got {rebuilt:?}"
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

        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

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

        let signed_a = build_signed_tx(
            &core_a,
            StandardAccountType::BIP44Account,
            0,
            &outputs_a,
            &signer_a,
        )
        .await
        .expect("build A should succeed");
        let token_a = registry
            .register(core_a.clone(), signed_a)
            .expect("test registers with the finalizing generation");
        let signed_b = build_signed_tx(
            &core_b,
            StandardAccountType::BIP44Account,
            0,
            &outputs_b,
            &signer_b,
        )
        .await
        .expect("build B should succeed");
        let _token_b = registry
            .register(core_b.clone(), signed_b)
            .expect("test registers with the finalizing generation");
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

    // NOTE: the former `release_entries_for_wallet_frees_the_reservation` test
    // is removed with the `release_entries_for_wallet` method it exercised.
    // Destroying wrapper aliases no longer releases deferred-payment tokens: a
    // wrapper handle does not own the payment, so its destruction must leave the
    // token live and broadcastable (`dashpay/platform#4185`, blocker 2). Token
    // reservations are reconciled by the payment owner (explicit
    // broadcast/release) or dropped at actual generation teardown
    // (`remove_entries_for_wallet`).

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

        let signed = build_signed_tx(
            &core_a,
            StandardAccountType::BIP44Account,
            0,
            &outputs_a,
            &signer_a,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core_a.clone(), signed)
            .expect("test registers with the finalizing generation");

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
        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        // The finalized object carries the reservation's OWN stamp height,
        // captured at build time — not a value the caller samples at register.
        assert_eq!(signed.reservation_height(), reservation_height);

        // Slow signer: the wallet advanced to just under the age bound while
        // signing. A fresh sample here would read `reservation_height +
        // MAX_AGE - 1`.
        advance_processed_height(&core, reservation_height + RESERVATION_MAX_AGE_BLOCKS - 1).await;

        // Register: the age baseline is the reservation height the consumed
        // object carries, not the advanced `last_processed_height` sampled now.
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

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

    /// Replace the wallet's per-generation balance `Arc` under the manager write
    /// lock, modelling a same-id remove-then-recreate: `wallet_id`, the manager
    /// `Arc`, and the account `ReservationSet` (with the token's input still
    /// reserved) are all preserved, only the generation marker is fresh. The
    /// still-reserved input now conceptually belongs to the NEW generation.
    async fn simulate_same_id_recreation<B: TransactionBroadcaster>(core: &CoreWallet<B>) {
        let mut wm = core.wallet_manager.write().await;
        let (_, info) = wm
            .get_wallet_and_info_mut(&core.wallet_id())
            .expect("wallet present in manager");
        info.generation = Arc::new(crate::wallet::core::WalletGeneration::new());
    }

    /// Regression for the non-atomic generation-validation + cleanup: a token's
    /// generation is validated at the registry lock, but its reservation cleanup
    /// runs later off that lock. If the wallet is removed and re-created under
    /// the SAME id in that window, an unguarded release-by-outpoint would free
    /// the NEW generation's reservation on the same input.
    ///
    /// This test recreates the generation (same id, fresh balance `Arc`) between
    /// registration and the release, then releases the now-stale token and
    /// asserts the reservation SURVIVES — the release, bound to the token's own
    /// generation under the manager lock, refuses to touch the re-created
    /// generation. An unconditional release-by-outpoint would instead free the
    /// new generation's reservation, and the rebuild below would succeed; the
    /// generation guard makes it still fail.
    #[tokio::test]
    async fn recreation_between_validation_and_cleanup_cannot_release_new_generation() {
        let broadcaster = Arc::new(RecordingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster).await;
        let registry = SignedPaymentRegistry::new();

        let signed = build_signed_tx(
            &core,
            StandardAccountType::BIP44Account,
            0,
            &outputs,
            &signer,
        )
        .await
        .expect("build should succeed");
        let token = registry
            .register(core.clone(), signed)
            .expect("test registers with the finalizing generation");

        // Reservation held: a rebuild fails at input selection.
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

        // Same-id wallet recreation between the token's validation and its
        // cleanup: the wallet under this id is now a DIFFERENT generation.
        simulate_same_id_recreation(&core).await;

        // Old cleanup runs. The token is dropped, but its release must NOT touch
        // the re-created generation's reservation.
        registry.release(token).await;
        assert_eq!(registry.outstanding(), 0, "the stale token is dropped");

        // The (new generation's) reservation on the input SURVIVES: a rebuild
        // still cannot reselect it. Pre-fix, the unconditional release-by-outpoint
        // would have freed it and this rebuild would succeed.
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
            "a stale token's cleanup must NOT release a re-created generation's \
             reservation, got {rebuilt:?}"
        );
    }

    /// A funding builder over the fixture's outputs, selecting largest-first
    /// like the production send path.
    fn payment_builder(outputs: &[(DashAddress, u64)]) -> TransactionBuilder {
        let mut builder =
            TransactionBuilder::new().set_selection_strategy(SelectionStrategy::LargestFirst);
        for (addr, amount) in outputs {
            builder = builder.add_output(addr, *amount);
        }
        builder
    }

    /// Unconditionally release `tx`'s input reservation on the BIP44 account,
    /// modelling key-wallet's TTL sweep returning the outpoint to the selectable
    /// pool — WITHOUT touching the registry entry, which still holds the token.
    async fn force_release_reservation<B: TransactionBroadcaster>(
        core: &CoreWallet<B>,
        tx: &Transaction,
    ) {
        let wm = core.wallet_manager.read().await;
        let (_, info) = wm
            .get_wallet_and_info(&core.wallet_id())
            .expect("wallet present in manager");
        info.core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .expect("bip44 managed account")
            .release_reservation(tx);
    }

    /// Owner-guarded release regression (`dashpay/platform#4185`): a rejected
    /// deferred broadcast must free ONLY the inputs its own build still owns. If
    /// key-wallet's TTL swept this build's reservation and a concurrent build
    /// re-reserved the same outpoint under a new token, the rejection's release
    /// must leave that other build's reservation intact — freeing it would let
    /// coin selection hand the outpoint to a third build and double-spend it.
    /// The registry threads the build's key-wallet `ReservationToken` to the
    /// reject path, so the release is owner-guarded rather than by-outpoint.
    #[tokio::test]
    async fn rejected_broadcast_releases_only_its_own_reservation_not_one_retaken_after_a_sweep() {
        let broadcaster = Arc::new(AlwaysRejectedBroadcaster);
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster).await;
        let registry = SignedPaymentRegistry::new();

        // Build 1 reserves the sole funding UTXO under token T1 and registers it
        // for deferred submission.
        let finalized = core
            .finalize_transaction(
                payment_builder(&outputs),
                AccountTypePreference::BIP44,
                0,
                &signer,
            )
            .await
            .expect("first finalize should succeed");
        // Capture the built tx before `register` consumes the ownership object;
        // the sweep below needs it to release the outpoint by hand.
        let finalized_tx = finalized.transaction().clone();
        let token = registry
            .register(core.clone(), finalized)
            .expect("test registers with the finalizing generation");

        // Model key-wallet's TTL sweep: the outpoint returns to the selectable
        // pool, but the registry still holds T1.
        force_release_reservation(&core, &finalized_tx).await;

        // A concurrent build re-selects and re-reserves that same outpoint under
        // a NEW token T2. Held alive so its reservation persists to the end.
        let retaken = core
            .finalize_transaction(
                payment_builder(&outputs),
                AccountTypePreference::BIP44,
                0,
                &signer,
            )
            .await
            .expect("re-reserving finalize should succeed after the sweep");

        // Build 1's deferred broadcast is definitively rejected. Its release is
        // owner-guarded by T1, so it must NOT free T2's reservation.
        let sent = registry.broadcast(token, &core).await;
        assert!(
            matches!(
                sent,
                Err(SignedPaymentError::Broadcast(
                    PlatformWalletError::TransactionBroadcast(_)
                ))
            ),
            "a rejected deferred broadcast must surface the rejection, got {sent:?}"
        );

        // T2 still owns the outpoint: a third build finds no free UTXO. Under the
        // pre-fix unconditional release, build 1's rejection would have freed it
        // and this build would succeed — double-spending T2's outpoint.
        let third = core
            .finalize_transaction(
                payment_builder(&outputs),
                AccountTypePreference::BIP44,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                third,
                Err(PlatformWalletError::CoreInsufficientFunds { .. })
            ),
            "the re-taken reservation must survive build 1's rejected broadcast, got {third:?}"
        );

        // Keep T2's build (and thus its reservation) alive until the assertions run.
        drop(retaken);
    }
}
