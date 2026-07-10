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
//! * [`broadcast`](SignedPaymentRegistry::broadcast) removes the entry **before**
//!   sending, so a repeated or concurrent broadcast of the same token can never
//!   double-broadcast — the second caller finds nothing and gets
//!   [`SignedPaymentError::StaleToken`].
//! * [`release`](SignedPaymentRegistry::release) is idempotent: releasing an
//!   unknown / already-consumed token is a silent no-op.
//! * A token is bound to the exact wallet instance it was minted against
//!   (`Arc::ptr_eq` on the shared `WalletManager`). Broadcasting it through a
//!   re-created wallet — whose in-memory `ReservationSet` no longer holds the
//!   inputs — is a [`SignedPaymentError::WalletMismatch`] rather than a spend
//!   against stale state.
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
use std::sync::{Arc, Mutex};

use dashcore::{Transaction, Txid};
use key_wallet::account::account_type::StandardAccountType;

use crate::broadcaster::TransactionBroadcaster;
use crate::wallet::core::CoreWallet;
use crate::PlatformWalletError;

/// Opaque handle to a registered, signed-but-unsent payment. Minted by
/// [`SignedPaymentRegistry::register`]; consumed by
/// [`SignedPaymentRegistry::broadcast`] or
/// [`SignedPaymentRegistry::release`]. Values are unique for the process
/// lifetime and never reused, so a stale token can always be recognised.
pub type ReservationToken = u64;

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
    /// The funding account whose reservation must be released on a rejected
    /// broadcast or an explicit release. `None` for a CoinJoin funding, which
    /// has no standard-account reservation to reconcile (it rides the
    /// TTL backstop), mirroring `CoreAccountTypeFFI::as_standard_account_type`.
    account_type: Option<StandardAccountType>,
    account_index: u32,
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

    /// Take ownership of a built, signed `tx` (whose funding UTXOs `build_signed`
    /// already reserved) and return an opaque token for a later
    /// [`broadcast`](Self::broadcast) or [`release`](Self::release).
    ///
    /// `core` is the wallet the payment was built against; it is captured so the
    /// later operation acts on the exact reservation state that holds the inputs.
    pub fn register(
        &self,
        core: CoreWallet<B>,
        tx: Transaction,
        account_type: Option<StandardAccountType>,
        account_index: u32,
    ) -> ReservationToken {
        let token = self.next_token.fetch_add(1, Ordering::SeqCst);
        self.entries
            .lock()
            .expect("signed-payment registry mutex poisoned")
            .insert(
                token,
                RegisteredPayment {
                    core,
                    tx,
                    account_type,
                    account_index,
                },
            );
        token
    }

    /// Broadcast the payment behind `token`, reconciling its UTXO reservation on
    /// failure, then consume the token.
    ///
    /// The entry is removed **before** the send, so a repeated or concurrent
    /// broadcast of the same token gets [`SignedPaymentError::StaleToken`]
    /// instead of a second send. `current` must be the same wallet instance the
    /// token was minted against (checked by `Arc::ptr_eq` on the shared
    /// `WalletManager`); otherwise the call fails with
    /// [`SignedPaymentError::WalletMismatch`] and the stale token is dropped.
    ///
    /// On a definitive rejection the reservation is released for an immediate
    /// rebuild; on an ambiguous ("may already be on the network") failure it is
    /// kept — the same policy as the non-deferred send path.
    pub async fn broadcast(
        &self,
        token: ReservationToken,
        current: &CoreWallet<B>,
    ) -> Result<Txid, SignedPaymentError> {
        // Remove under the lock and drop the guard *before* awaiting — a
        // std::Mutex guard must never be held across an await point, and the
        // atomic take is what makes a double-broadcast impossible.
        let entry = {
            let mut entries = self
                .entries
                .lock()
                .expect("signed-payment registry mutex poisoned");
            entries.remove(&token)
        }
        .ok_or(SignedPaymentError::StaleToken(token))?;

        if !Arc::ptr_eq(&entry.core.wallet_manager, &current.wallet_manager) {
            // The token belongs to another wallet instance; it has been removed,
            // so it can never be replayed here.
            return Err(SignedPaymentError::WalletMismatch(token));
        }

        let txid = match entry.account_type {
            Some(account_type) => {
                entry
                    .core
                    .broadcast_transaction_releasing_reservation(
                        account_type,
                        entry.account_index,
                        &entry.tx,
                    )
                    .await?
            }
            None => entry.core.broadcast_transaction(&entry.tx).await?,
        };
        Ok(txid)
    }

    /// Release the funding reservation behind `token` and drop it. Idempotent:
    /// releasing an unknown / already-consumed token is a silent no-op, so a
    /// double release (or a release after a broadcast) is harmless.
    ///
    /// The release acts on the wallet instance the token was minted against —
    /// the one whose `ReservationSet` actually holds the inputs — so no wallet
    /// handle need be threaded in.
    pub async fn release(&self, token: ReservationToken) {
        let entry = {
            let mut entries = self
                .entries
                .lock()
                .expect("signed-payment registry mutex poisoned");
            entries.remove(&token)
        };
        let Some(entry) = entry else {
            // Unknown / already consumed — idempotent no-op.
            return;
        };
        if let Some(account_type) = entry.account_type {
            entry
                .core
                .release_transaction_reservation(account_type, entry.account_index, &entry.tx)
                .await;
        }
    }

    /// Number of outstanding (registered but not yet broadcast/released) tokens.
    #[cfg(test)]
    pub(crate) fn outstanding(&self) -> usize {
        self.entries
            .lock()
            .expect("signed-payment registry mutex poisoned")
            .len()
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
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

    use super::{SignedPaymentError, SignedPaymentRegistry};
    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::test_support::{funded_wallet_manager, AlwaysMaybeSentBroadcaster, WalletSigner};
    use crate::wallet::core::CoreWallet;
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
        let current_height = info.core_wallet.synced_height();
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
            .build_signed(signer, |addr| managed_account.address_derivation_path(&addr))
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

        let tx = build_signed_tx(&core, StandardAccountType::BIP44Account, 0, &outputs, &signer)
            .await
            .expect("build should succeed");
        let expected_bytes = dashcore::consensus::serialize(&tx);
        let expected_txid = tx.txid();

        let token = registry.register(
            core.clone(),
            tx,
            Some(StandardAccountType::BIP44Account),
            0,
        );
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
            let token = registry.register(core.clone(), tx, Some(account_type), 0);

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

    /// A second broadcast of the same token is a typed `StaleToken` error, never
    /// a second send.
    #[tokio::test]
    async fn double_broadcast_is_a_stale_token_error() {
        let broadcaster = Arc::new(CountingBroadcaster::new());
        let (core, signer, outputs) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster)).await;
        let registry = SignedPaymentRegistry::new();

        let tx = build_signed_tx(&core, StandardAccountType::BIP44Account, 0, &outputs, &signer)
            .await
            .expect("build should succeed");
        let token = registry.register(core.clone(), tx, Some(StandardAccountType::BIP44Account), 0);

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

        let tx = build_signed_tx(&core, StandardAccountType::BIP44Account, 0, &outputs, &signer)
            .await
            .expect("build should succeed");
        let token = registry.register(core.clone(), tx, Some(StandardAccountType::BIP44Account), 0);

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

        let tx = build_signed_tx(&core, StandardAccountType::BIP44Account, 0, &outputs, &signer)
            .await
            .expect("build should succeed");
        let token = registry.register(core.clone(), tx, Some(StandardAccountType::BIP44Account), 0);

        registry.release(token).await;
        let sent = registry.broadcast(token, &core).await;
        assert!(
            matches!(sent, Err(SignedPaymentError::StaleToken(_))),
            "broadcast of a released token must be StaleToken, got {sent:?}"
        );
        assert_eq!(broadcaster.count.load(Ordering::SeqCst), 0, "nothing was sent");
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
        let (core_a, signer_a, outputs_a) =
            funded_core_wallet(StandardAccountType::BIP44Account, Arc::clone(&broadcaster_a)).await;
        // A separate wallet-manager instance stands in for a re-created wallet.
        let broadcaster_b = Arc::new(CountingBroadcaster::new());
        let (core_b, _signer_b, _outputs_b) =
            funded_core_wallet(StandardAccountType::BIP44Account, broadcaster_b).await;
        let registry = SignedPaymentRegistry::new();

        let tx =
            build_signed_tx(&core_a, StandardAccountType::BIP44Account, 0, &outputs_a, &signer_a)
                .await
                .expect("build should succeed");
        let token = registry.register(
            core_a.clone(),
            tx,
            Some(StandardAccountType::BIP44Account),
            0,
        );

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
        assert_eq!(registry.outstanding(), 0, "the stale token is dropped");
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

        let tx = build_signed_tx(&core, StandardAccountType::BIP44Account, 0, &outputs, &signer)
            .await
            .expect("build should succeed");
        let token = registry.register(core.clone(), tx, Some(StandardAccountType::BIP44Account), 0);

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
        let rebuilt = build_signed_tx(&core, StandardAccountType::BIP44Account, 0, &outputs, &signer)
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

        let tx = build_signed_tx(&core, StandardAccountType::BIP44Account, 0, &outputs, &signer)
            .await
            .expect("build should succeed");
        let token = registry.register(core.clone(), tx, Some(StandardAccountType::BIP44Account), 0);

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
        let tx = build_signed_tx(&core, StandardAccountType::BIP44Account, 0, &outputs, &signer)
            .await
            .expect("build should succeed");
        let registry = Arc::new(SignedPaymentRegistry::new());

        let mut handles = Vec::new();
        for _ in 0..16 {
            let registry = Arc::clone(&registry);
            let core = core.clone();
            let tx = tx.clone();
            handles.push(tokio::spawn(async move {
                registry.register(core, tx, Some(StandardAccountType::BIP44Account), 0)
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
}
