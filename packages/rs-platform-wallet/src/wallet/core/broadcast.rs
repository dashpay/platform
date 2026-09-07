use dashcore::Transaction;
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::ReservationToken;

use super::SignedCoreTransaction;
use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use crate::wallet::reservations::{broadcast_releasing_on_rejection, reservation_expired};
use crate::{CoreWallet, PlatformWalletError};

/// Outcome of [`CoreWallet::dispatch_unexpired`] — the guarded
/// age-check-and-send. `Stale` means the broadcaster was never touched.
pub(crate) enum GuardedDispatch {
    /// The reservation aged past the bound; nothing was sent.
    Stale,
    /// The broadcaster was reached; its verbatim outcome.
    Sent(Result<dashcore::Txid, BroadcastError>),
}

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// Age-check AND pin under the wallet-manager READ lock, then dispatch
    /// immediately after releasing it, keeping the pin until the broadcaster
    /// returns.
    ///
    /// The age check orders against key-wallet's `ReservationSet` TTL
    /// sweep — it runs inside coin selection, which mutates wallet state
    /// under the manager WRITE lock — and `last_processed_height`
    /// advancement (same lock): a reservation that passes the check under
    /// this guard cannot already have been swept, because key-wallet's TTL
    /// exceeds `RESERVATION_MAX_AGE_BLOCKS` on the same height clock, and
    /// self-releases only run on this transaction's own rejection/abandon
    /// paths, which are sequenced after this call returns. That proof of
    /// still-held ownership is what authorizes the pin taken in the same
    /// guarded section (the pin's owner check).
    ///
    /// The guard is deliberately DROPPED before the broadcaster await. The
    /// production `SpvBroadcaster` waits on dash-spv's mempool pipeline,
    /// and that pipeline's local-transaction handler takes `wallet.write()`
    /// on this same manager lock before it can process the very
    /// echo/IS-lock/confirmation events the wait needs — held across the
    /// await, the guard starves the pipeline and every dispatch rides the
    /// full acceptance timeout to an ambiguous verdict while the whole
    /// manager stalls behind tokio's write-preferring queue. (Same
    /// lock-free shape as `broadcast_releasing_on_rejection`.)
    ///
    /// What spans the await instead is the **in-broadcast pin**
    /// ([`WalletGeneration::pin_in_broadcast`](super::WalletGeneration::pin_in_broadcast)),
    /// installed on the manager-registered generation while the guard was
    /// still held. Both production broadcasters can suspend *before*
    /// submission (the SPV path awaits configuration, event subscription and
    /// the network lock ahead of its local dispatch), catch-up can advance
    /// the clock by many blocks in that gap, and async scheduling puts no
    /// bound on it — so a freshness check alone is not an ordering
    /// invariant against the TTL sweep + re-reserve race. The pin is: it
    /// has no TTL while the dispatch is in flight, and every coin-selection
    /// choke point refuses a build whose selection picked a pinned input
    /// (under the same write lock the sweep runs under).
    ///
    /// # Where the fence is released, and why that point is safe
    ///
    /// The pin is *not* simply dropped when the broadcaster returns. That
    /// return means "the transaction may now be on the network", not "this
    /// wallet has observed the spend", and the two differ per broadcaster:
    /// `SpvBroadcaster` injects into dash-spv's local mempool pipeline, so the
    /// inputs leave this wallet's selectable set within milliseconds;
    /// `DapiBroadcaster::broadcast` only awaits `sdk.execute` and injects
    /// nothing, so on that path the inputs are still selectable while the
    /// transaction is in flight. So:
    ///
    /// * **Definitive pre-send rejection** (`BroadcastError::Rejected`) — the
    ///   transaction provably did not reach the network. The fence is dropped
    ///   immediately here, and the caller releases the reservation in the same
    ///   breath, so an instant rebuild can reselect the inputs.
    /// * **Anything else** (accepted, or an ambiguous `MaybeSent`) — the pin is
    ///   converted to a **pending-spend fence** that lasts until this wallet
    ///   OBSERVES the outpoints spent
    ///   ([`WalletGeneration::observe_spent`](super::WalletGeneration::observe_spent)),
    ///   by the dispatch's own transaction or by a competing one.
    ///
    /// # Why the fence waits for an observation instead of a height bound
    ///
    /// A pending-spend phase bounded at `last_processed_height + N` is unsound
    /// wherever the height is sampled — before the await, after it, after it
    /// under a still-held guard. Any such bound can be consumed by a routine
    /// historical catch-up: the wallet advances that height by thousands of
    /// blocks in seconds, and those blocks were mined BEFORE this transaction
    /// was submitted, so they are not evidence that it has been seen or
    /// dropped. On the `DapiBroadcaster` path — which returns from
    /// `sdk.execute` without injecting anything into local wallet state — the
    /// input then becomes reselectable while the transaction is in flight.
    ///
    /// The release condition is therefore evidence, not elapsed chain: the
    /// outpoint is freed when the wallet sees it spent. That is a fact about
    /// this transaction rather than about the chain's past, and it arrives on
    /// both broadcaster paths — SPV within milliseconds via its local mempool
    /// pipeline, DAPI when the transaction is relayed back or lands in a block.
    ///
    /// There is NO backstop timeout behind that, and deliberately so. A
    /// monotonic deadline as a liveness valve would not help: a clock catch-up
    /// cannot fast-forward is still not evidence about this transaction, and
    /// once it lapsed the next build could sign a conflicting spend of inputs
    /// the original might still take. A transaction the wallet never observes
    /// at all — evicted
    /// for fee, conflicted away unseen — therefore holds its inputs for the rest
    /// of the process. That is the correct trade: those are exactly the inputs a
    /// possibly-live signed transaction spends. See the
    /// [`in_broadcast`](super::WalletGeneration) field docs for the invariant
    /// and for the two liveness shapes that may shorten the wait without
    /// weakening it.
    ///
    /// # Why there is no post-await manager guard
    ///
    /// A height-bounded fence would need one: its height would have to be
    /// sampled and installed inside a single manager read guard, or a writer
    /// queued behind it could advance the clock in between and the fence
    /// would land already lapsed. With no clock to sample at all there is
    /// nothing for a height writer to interleave with — the settle sets a flag
    /// inside the `in_broadcast` critical section. So it needs no manager
    /// lock, and this method touches the wallet-manager lock exactly once,
    /// before the send, which also spares every dispatch a lock acquisition.
    ///
    /// A wallet no longer in the manager skips the pin (there is no
    /// registered generation to fence builds on — they cannot fund from a
    /// removed wallet); liveness is the FFI layer's generation check,
    /// established before this runs.
    ///
    /// Callers do their stale/rejection reconciliation AFTER this returns:
    /// those paths retake manager locks.
    pub(crate) async fn dispatch_unexpired(
        &self,
        reservation_height: u32,
        transaction: &Transaction,
    ) -> GuardedDispatch {
        let in_broadcast_pin = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id);
            let height = info.map(|info| info.core_wallet.last_processed_height());
            if reservation_expired(reservation_height, height) {
                return GuardedDispatch::Stale;
            }
            // Pin BEFORE the guard drops: check-and-pin is one atomic step,
            // and freshness under this guard proves the reservation is still
            // ours to pin (see the method docs). The pin outlives the guard,
            // and — unless the send is definitively rejected — outlives the
            // broadcaster return too, as a pending-spend fence.
            //
            // `height` is NOT handed to the pin, and no height is sampled for
            // it later either. It authorizes the freshness check and nothing
            // else; the fence answers to observed spends and to NO clock of any
            // kind — not chain height, not wall time — because nothing that
            // merely elapses is evidence about this transaction (see the method
            // docs).
            info.map(|info| info.generation.pin_in_broadcast(transaction))
            // Guard dropped here — holding it across the await starves the
            // SPV pipeline that must complete the wait; the pin, not the
            // guard, covers check-to-wire.
        };
        let outcome = self.broadcaster.broadcast(transaction).await;
        // The pin already fences by default, so the inputs stay held on EVERY
        // exit from the await above — including one this code never observes:
        // the dispatching future being cancelled, or an unwind, mid-`broadcast`.
        // Neither says anything about whether the transaction reached the
        // network, and freeing the inputs there lets an immediate reselection
        // double-spend a transaction already on the wire.
        //
        // Only a definitive pre-send rejection proves nothing was sent, so it is
        // the one outcome that releases. An ambiguous `MaybeSent` stays fenced.
        if let Some(pin) = in_broadcast_pin {
            if matches!(
                outcome,
                Err(crate::broadcaster::BroadcastError::Rejected { .. })
            ) {
                // Provably nothing on the wire: free the outpoints outright, so
                // an immediate rebuild can reselect them.
                pin.settle_released();
            } else {
                // EVERY non-rejection outcome — accepted or ambiguous
                // `MaybeSent` — opens the pending-spend phase, which holds the
                // outpoints until the wallet observes them spent. No manager
                // guard is taken: there is no height to sample, so there is
                // nothing for a concurrent height writer to interleave with.
                pin.settle_pending_spend();
            }
        }
        GuardedDispatch::Sent(outcome)
    }

    /// Broadcast an atomically finalized transaction. A definitive rejection
    /// releases its reservation; an ambiguous `MaybeSent` outcome retains it.
    ///
    /// The release is owner-guarded by the finalized transaction's
    /// [`reservation_token`](SignedCoreTransaction::reservation_token): the
    /// broadcast is `.await`ed, and during that await key-wallet's TTL sweep can
    /// reclaim this build's reservation and a concurrent build re-reserve the
    /// same inputs under a new token. Releasing by outpoint alone would then
    /// free that other build's inputs (the release/re-reserve double-spend
    /// window); presenting the token frees only inputs this build still owns.
    ///
    /// # Reservation age guard
    ///
    /// A finalized-transaction handle can be pinned by the host for an
    /// arbitrary time between `finalize` and this broadcast. If the wallet's
    /// `last_processed_height` advances at least
    /// [`RESERVATION_MAX_AGE_BLOCKS`](crate::wallet::reservations::RESERVATION_MAX_AGE_BLOCKS)
    /// blocks past the height the funding reservation was stamped at
    /// ([`SignedCoreTransaction::reservation_height`]), key-wallet's own
    /// `ReservationSet` TTL could already have swept those inputs and let an
    /// unrelated build re-select them. Broadcasting then would spend against a
    /// newer, unrelated reservation, so the send is refused with
    /// [`PlatformWalletError::StaleReservation`] **before** the broadcaster is
    /// touched — mirroring the deferred registry token's
    /// [`broadcast`](crate::SignedPaymentRegistry::broadcast) guard, off the
    /// same bound and the same `last_processed_height` clock, and running after
    /// the FFI layer's generation-identity check just as the registry does.
    ///
    /// The refusal also reconciles the reservation, exactly like the registry's
    /// stale-token branch: the FFI wrapper has already consumed the opaque
    /// handle by the time this runs (and the host bindings clear their local
    /// handles before entering the ABI), so a follow-up
    /// [`abandon_transaction`](Self::abandon_transaction) is unreachable from
    /// the caller's side. Abandoning here releases owner-guarded
    /// (`release_reservation_if_owner`), which is safe at ANY age — between the
    /// guard bound and key-wallet's TTL the reservation is typically STILL this
    /// build's, so the release is what lets the instructed immediate rebuild
    /// reselect the inputs instead of stranding them until the TTL backstop.
    /// Only a token-less build (never reached on the funded finalize path)
    /// skips, leaving the aged reservation for the TTL to reclaim.
    pub async fn broadcast_finalized_transaction(
        &self,
        transaction: &SignedCoreTransaction,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        // The age check happens at dispatch time, inside
        // [`Self::dispatch_unexpired`] — not out here, where it would go
        // stale before the send (sync catch-up can age the reservation and
        // a concurrent finalization can sweep + re-reserve the same inputs
        // in the gap, letting the old signed transaction hit the wire
        // against reassigned UTXOs). The check also installs the
        // in-broadcast pin that fences the inputs against exactly that
        // sweep + re-reserve until the broadcaster returns; why the manager
        // guard itself must not span the broadcaster await is documented on
        // `dispatch_unexpired`. Reconciliation retakes manager locks after
        // it returns.
        match self
            .dispatch_unexpired(transaction.reservation_height(), transaction.transaction())
            .await
        {
            GuardedDispatch::Stale => {
                self.abandon_transaction(transaction).await;
                Err(PlatformWalletError::StaleReservation)
            }
            GuardedDispatch::Sent(Ok(txid)) => Ok(txid),
            GuardedDispatch::Sent(Err(error)) => {
                if matches!(error, crate::broadcaster::BroadcastError::Rejected { .. }) {
                    self.release_transaction_reservation(
                        transaction.funding_accounts(),
                        transaction.transaction(),
                        transaction.reservation_token(),
                    )
                    .await;
                }
                Err(error.into())
            }
        }
    }

    /// Broadcast a signed transaction to the network.
    ///
    /// Transactions can be built and signed with key-wallet's
    /// [`TransactionBuilder`](key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder)
    /// before being passed here; this method only broadcasts the
    /// caller-supplied signed transaction.
    ///
    /// Delegates to the injected [`TransactionBroadcaster`] which may use
    /// SPV (P2P) or DAPI (gRPC) depending on how the wallet was constructed.
    ///
    /// Returns the transaction ID on success.
    ///
    /// This plain form does **not** reconcile the funding account's UTXO
    /// reservation on failure. Prefer
    /// [`broadcast_transaction_releasing_reservation`](Self::broadcast_transaction_releasing_reservation)
    /// for the build-then-broadcast send path, where a `build_signed`
    /// reserved the selected inputs and a failed broadcast must release them.
    pub async fn broadcast_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        self.broadcaster
            .broadcast(transaction)
            .await
            .map_err(Into::into)
    }

    /// Broadcast a signed transaction, reconciling the funding account's UTXO
    /// reservation on failure.
    ///
    /// `build_signed` reserves the selected inputs in the funding account's
    /// `ReservationSet` and leaves the reservation held on success (expecting
    /// this broadcast). On a definitive rejection the reservation is released
    /// so an immediate retry can reselect those inputs; on an ambiguous
    /// failure it is kept. See
    /// [`broadcast_releasing_on_rejection`](crate::wallet::reservations::broadcast_releasing_on_rejection)
    /// for the full rationale.
    ///
    /// `account_type`/`account_index` identify the funding account handed to
    /// `set_funding` when the transaction was built.
    pub async fn broadcast_transaction_releasing_reservation(
        &self,
        account_type: StandardAccountType,
        account_index: u32,
        transaction: &Transaction,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        broadcast_releasing_on_rejection(
            self.broadcaster.as_ref(),
            &self.wallet_manager,
            &self.wallet_id,
            account_type,
            account_index,
            transaction,
        )
        .await
        .map_err(Into::into)
    }

    /// Broadcast a raw signed `transaction` for the deferred-payment
    /// [`SignedPaymentRegistry`](crate::SignedPaymentRegistry), reconciling the
    /// funding reservation on failure.
    ///
    /// Same policy as
    /// [`broadcast_finalized_transaction`](Self::broadcast_finalized_transaction):
    /// a definitive [`BroadcastError::Rejected`] releases the reservation for an
    /// immediate rebuild; an ambiguous `MaybeSent` keeps it. Unlike the
    /// `StandardAccountType`-typed
    /// [`broadcast_transaction_releasing_reservation`](Self::broadcast_transaction_releasing_reservation)
    /// used by the immediate send path, this takes an [`AccountTypePreference`]
    /// so it ALSO reconciles a CoinJoin-funded deferred payment — one whose
    /// `build_signed`/`finalize` reserved the selected inputs but which has no
    /// `StandardAccountType`, and whose reservation would otherwise stay held
    /// until the TTL backstop.
    ///
    /// The release delegates to
    /// [`release_transaction_reservation`](Self::release_transaction_reservation),
    /// so it acts only on the wallet *generation* this handle names (a wallet
    /// re-created under the same id between build and broadcast cannot have its
    /// reservation freed by this token) AND — via `token` — only on inputs this
    /// build still owns. The deferred registry can hold the reservation across a
    /// long build→broadcast gap, so a TTL sweep re-reserving the same inputs
    /// under a new token is a real risk; the owner guard closes that
    /// release/re-reserve race.
    ///
    /// `accounts` are the concrete accounts that contributed the transaction's
    /// inputs (`SignedCoreTransaction::funding_accounts`) — a pooled send spans
    /// several, and key-wallet reserves per account, so a rejection must
    /// release on EVERY one of them. `token` is the [`ReservationToken`] that
    /// build stamped across all of them
    /// (`SignedCoreTransaction::reservation_token`), `None` only when the build
    /// reserved nothing.
    /// `reservation_height` is the height the funding reservation was
    /// stamped at; the age bound is re-checked ATOMICALLY with dispatch
    /// under the manager read guard ([`Self::dispatch_unexpired`]) — a
    /// pre-checked age is not an invariant, because catch-up can advance
    /// the clock and a concurrent finalization can sweep + re-reserve the
    /// inputs between a caller's check and the send. The same guarded
    /// section installs the in-broadcast pin that fences the inputs against
    /// that sweep + re-reserve for the whole broadcaster await — the same
    /// primitive as the finalized-handle path. On the stale outcome
    /// nothing was sent and NOTHING is released here: the caller owns the
    /// reconciliation policy (the registry reconciles owner-guarded).
    pub(crate) async fn broadcast_payment_releasing_reservation(
        &self,
        accounts: &[key_wallet::account::AccountType],
        transaction: &Transaction,
        token: Option<ReservationToken>,
        reservation_height: u32,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        match self
            .dispatch_unexpired(reservation_height, transaction)
            .await
        {
            GuardedDispatch::Stale => Err(PlatformWalletError::StaleReservation),
            GuardedDispatch::Sent(Ok(txid)) => Ok(txid),
            GuardedDispatch::Sent(Err(error)) => {
                if matches!(error, BroadcastError::Rejected { .. }) {
                    self.release_transaction_reservation(accounts, transaction, token)
                        .await;
                }
                Err(error.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::GuardedDispatch;
    use dashcore::{Address as DashAddress, Network, Transaction};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::signer::Signer;
    use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
    use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

    use crate::broadcaster::TransactionBroadcaster;
    use crate::test_support::{
        funded_wallet_manager, AlwaysMaybeSentBroadcaster, AlwaysOkBroadcaster,
        RejectFirstBroadcaster, WalletSigner,
    };
    use crate::wallet::core::{CoreWallet, SpendObservationHandler};
    use crate::wallet::platform_wallet::WalletId;
    use crate::wallet::reservations::RESERVATION_MAX_AGE_BLOCKS;
    use crate::{PlatformWalletError, SignedCoreTransaction};

    /// Builds a testnet `CoreWallet` over the shared funded fixture and a
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
        let outputs = vec![(recipient, 1_000_000u64)];

        (core, signer, outputs)
    }

    /// Build and sign a payment the way the split send path does: `build_signed`
    /// reserves the selected inputs in the funding account's `ReservationSet`,
    /// leaving the reservation held for the subsequent broadcast. Mirrors the
    /// FFI `core_wallet_tx_builder_*` sequence.
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
            .add_funding(managed_account, account);
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

    /// Atomically fund + reserve + sign a `SignedCoreTransaction` the way the
    /// finalized-handle path (`core_wallet_tx_builder_finalize`) does, capturing
    /// the reservation's stamp height on the returned handle.
    async fn finalize_tx<B: TransactionBroadcaster>(
        core: &CoreWallet<B>,
        account_type: AccountTypePreference,
        outputs: &[(DashAddress, u64)],
        signer: &WalletSigner,
    ) -> SignedCoreTransaction {
        try_finalize_tx(core, account_type, outputs, signer)
            .await
            .expect("finalize should succeed")
    }

    /// Like [`finalize_tx`] but surfaces the build error instead of panicking —
    /// used to prove a *rebuild* fails when a still-held reservation keeps its
    /// inputs out of the selectable pool.
    async fn try_finalize_tx<B: TransactionBroadcaster>(
        core: &CoreWallet<B>,
        account_type: AccountTypePreference,
        outputs: &[(DashAddress, u64)],
        signer: &WalletSigner,
    ) -> Result<SignedCoreTransaction, PlatformWalletError> {
        let mut builder = TransactionBuilder::new();
        for (addr, amount) in outputs {
            builder = builder.add_output(addr, *amount);
        }
        core.finalize_transaction(builder, &[account_type], 0, signer)
            .await
    }

    /// Force the wallet's `last_processed_height` forward, simulating chain
    /// progress between `finalize` and a later broadcast of the pinned
    /// handle — the window in which key-wallet's `ReservationSet` TTL can sweep
    /// the funding reservation. Same clock the age guard reads.
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

    /// A freshly finalized handle — no chain progress since `finalize` —
    /// broadcasts normally: the age guard does not trip.
    #[tokio::test]
    async fn fresh_finalized_handle_broadcasts() {
        for account_type in [AccountTypePreference::BIP44, AccountTypePreference::BIP32] {
            let (core, signer, outputs) = funded_core_wallet(
                account_type_standard(account_type),
                Arc::new(AlwaysOkBroadcaster),
            )
            .await;
            let finalized = finalize_tx(&core, account_type, &outputs, &signer).await;
            let sent = core.broadcast_finalized_transaction(&finalized).await;
            assert!(
                sent.is_ok(),
                "a fresh handle must broadcast for {account_type:?}, got {sent:?}"
            );
        }
    }

    /// A handle pinned while the wallet syncs past `RESERVATION_MAX_AGE_BLOCKS`
    /// beyond its reservation stamp must be refused with `StaleReservation`
    /// (never a send — the broadcaster is `AlwaysOk`, so a leaked send would
    /// surface as `Ok`). The refusal itself reconciles the reservation,
    /// OWNER-GUARDED — this is terminal at the FFI boundary, where the opaque
    /// handle was consumed before the guard ran, so no follow-up abandon is
    /// possible. Below key-wallet's TTL the reservation is still this build's,
    /// `release_reservation_if_owner` frees it, and the instructed immediate
    /// rebuild reselects the inputs with NO further cleanup call. A late
    /// abandon of the stale original is then an owner-guarded no-op — ownership
    /// has transferred to the rebuild, whose reservation must survive it.
    #[tokio::test]
    async fn aged_finalized_handle_refusal_releases_for_rebuild() {
        for account_type in [AccountTypePreference::BIP44, AccountTypePreference::BIP32] {
            let (core, signer, outputs) = funded_core_wallet(
                account_type_standard(account_type),
                Arc::new(AlwaysOkBroadcaster),
            )
            .await;
            let stamped = core
                .last_processed_height()
                .await
                .expect("last processed height");
            let finalized = finalize_tx(&core, account_type, &outputs, &signer).await;

            // Advance past the guard bound (stay below key-wallet's 24-block TTL,
            // so the reservation is provably still held — only our guard tripped).
            advance_processed_height(&core, stamped + RESERVATION_MAX_AGE_BLOCKS + 2).await;

            let sent = core.broadcast_finalized_transaction(&finalized).await;
            assert!(
                matches!(sent, Err(PlatformWalletError::StaleReservation)),
                "an aged handle must refuse with StaleReservation for \
                 {account_type:?}, got {sent:?}"
            );

            // The refusal released the still-owned reservation: an immediate
            // rebuild reselects the single fixture UTXO without any abandon.
            let rebuilt = try_finalize_tx(&core, account_type, &outputs, &signer).await;
            let rebuilt = rebuilt.unwrap_or_else(|error| {
                panic!(
                    "the stale refusal must release the still-owned reservation \
                     so a rebuild succeeds for {account_type:?}, got {error:?}"
                )
            });

            // A late abandon of the stale original must be an owner-guarded
            // no-op: ownership transferred to the rebuild, so the rebuild's
            // reservation still holds the fixture's only UTXO and a competing
            // finalize must fail.
            core.abandon_transaction(&finalized).await;
            let competing = try_finalize_tx(&core, account_type, &outputs, &signer).await;
            assert!(
                competing.is_err(),
                "abandoning the consumed stale handle must not free the \
                 rebuild's reservation for {account_type:?}, got a successful \
                 competing finalize"
            );
            core.abandon_transaction(&rebuilt).await;
        }
    }

    /// The age bound is validated by [`CoreWallet::dispatch_unexpired`]
    /// itself, immediately before the send — never by a caller-side
    /// pre-check that could go stale in the gap. The height sample and the
    /// expiry verdict happen under a wallet-manager read guard that is
    /// dropped before the broadcaster await (holding it across the await
    /// starves the SPV mempool pipeline — see `dispatch_unexpired`'s doc);
    /// the check-to-wire gap is covered by the in-broadcast pin installed
    /// in the same guarded section (see
    /// `in_broadcast_pin_blocks_reselection_until_dispatch_returns`). The
    /// single-threaded proof here: the same handle's inputs dispatch while
    /// fresh, and the identical call refuses — broadcaster untouched —
    /// once catch-up advances the clock past the bound.
    #[tokio::test]
    async fn guarded_dispatch_rechecks_age_at_dispatch() {
        let (core, signer, outputs) = funded_core_wallet(
            account_type_standard(AccountTypePreference::BIP44),
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        let stamped = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let finalized = finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;

        // Fresh: the guarded dispatch reaches the broadcaster.
        let fresh = core
            .dispatch_unexpired(finalized.reservation_height(), finalized.transaction())
            .await;
        assert!(
            matches!(fresh, GuardedDispatch::Sent(Ok(_))),
            "a fresh reservation must dispatch"
        );

        // Catch-up advances the clock past the bound; the identical call now
        // refuses inside the guard with the broadcaster never touched
        // (`AlwaysOk` would have surfaced a leaked send as `Sent(Ok)`).
        advance_processed_height(&core, stamped + RESERVATION_MAX_AGE_BLOCKS + 2).await;
        let stale = core
            .dispatch_unexpired(finalized.reservation_height(), finalized.transaction())
            .await;
        assert!(
            matches!(stale, GuardedDispatch::Stale),
            "an aged reservation must refuse at the check, not dispatch"
        );

        core.abandon_transaction(&finalized).await;
    }

    /// Below the guard bound the reservation is provably still ours (no sweep
    /// possible yet), so abandon/free release it — owner-guarded, via the token
    /// the funded finalize stamped — returning the inputs so an immediate
    /// rebuild reselects them.
    #[tokio::test]
    async fn below_bound_finalized_handle_abandon_releases() {
        for account_type in [AccountTypePreference::BIP44, AccountTypePreference::BIP32] {
            let (core, signer, outputs) = funded_core_wallet(
                account_type_standard(account_type),
                Arc::new(AlwaysOkBroadcaster),
            )
            .await;
            let stamped = core
                .last_processed_height()
                .await
                .expect("last processed height");
            let finalized = finalize_tx(&core, account_type, &outputs, &signer).await;

            // Aged, but one shy of the guard bound: still below both the guard and
            // the TTL, so the reservation is unambiguously ours to release.
            advance_processed_height(&core, stamped + RESERVATION_MAX_AGE_BLOCKS - 1).await;

            core.abandon_transaction(&finalized).await;

            // The release freed the input: an immediate rebuild reselects it.
            let rebuilt = try_finalize_tx(&core, account_type, &outputs, &signer).await;
            assert!(
                rebuilt.is_ok(),
                "below-bound abandon must release the input so a rebuild reselects \
                 it for {account_type:?}, got {rebuilt:?}"
            );
            core.abandon_transaction(&rebuilt.expect("rebuild")).await;
        }
    }

    /// The guard boundary is exact: `current - stamped >= RESERVATION_MAX_AGE_BLOCKS`
    /// refuses, one block below still broadcasts — for both standard account
    /// types, like the fresh/aged tests.
    #[tokio::test]
    async fn finalized_handle_age_guard_boundary_is_exact() {
        for account_type in [AccountTypePreference::BIP44, AccountTypePreference::BIP32] {
            // One below the bound: still fresh enough to broadcast.
            let (below_core, below_signer, below_outputs) = funded_core_wallet(
                account_type_standard(account_type),
                Arc::new(AlwaysOkBroadcaster),
            )
            .await;
            let below_stamped = below_core
                .last_processed_height()
                .await
                .expect("last processed height");
            let below = finalize_tx(&below_core, account_type, &below_outputs, &below_signer).await;
            advance_processed_height(&below_core, below_stamped + RESERVATION_MAX_AGE_BLOCKS - 1)
                .await;
            assert!(
                below_core
                    .broadcast_finalized_transaction(&below)
                    .await
                    .is_ok(),
                "one block below the bound must still broadcast ({account_type:?})"
            );

            // Exactly at the bound: refused.
            let (at_core, at_signer, at_outputs) = funded_core_wallet(
                account_type_standard(account_type),
                Arc::new(AlwaysOkBroadcaster),
            )
            .await;
            let at_stamped = at_core
                .last_processed_height()
                .await
                .expect("last processed height");
            let at = finalize_tx(&at_core, account_type, &at_outputs, &at_signer).await;
            advance_processed_height(&at_core, at_stamped + RESERVATION_MAX_AGE_BLOCKS).await;
            assert!(
                matches!(
                    at_core.broadcast_finalized_transaction(&at).await,
                    Err(PlatformWalletError::StaleReservation)
                ),
                "exactly at the bound must refuse with StaleReservation ({account_type:?})"
            );
        }
    }

    /// Map a builder `AccountTypePreference` (BIP44/BIP32 only in these tests)
    /// to the `StandardAccountType` the funded fixture is keyed by.
    fn account_type_standard(account_type: AccountTypePreference) -> StandardAccountType {
        match account_type {
            AccountTypePreference::BIP44 => StandardAccountType::BIP44Account,
            AccountTypePreference::BIP32 => StandardAccountType::BIP32Account,
            other => {
                unreachable!("only standard-account funding is exercised by these tests: {other:?}")
            }
        }
    }

    /// A broadcaster that models the pre-submission suspension window of the
    /// production broadcasters: `broadcast` parks between two barriers, so the
    /// test can interleave catch-up and a competing build while the dispatch
    /// is provably mid-await (freshness already checked, guard already
    /// dropped, pin held).
    struct GatedBroadcaster {
        entered: Arc<tokio::sync::Barrier>,
        release: Arc<tokio::sync::Barrier>,
    }

    #[async_trait::async_trait]
    impl TransactionBroadcaster for GatedBroadcaster {
        async fn broadcast(
            &self,
            transaction: &Transaction,
        ) -> Result<dashcore::Txid, crate::broadcaster::BroadcastError> {
            self.entered.wait().await;
            self.release.wait().await;
            Ok(transaction.txid())
        }
    }

    /// The `WalletEvent` the wallet emits when it observes `tx` — the real
    /// shape the spend-observation seam consumes. Shared fixture, so this
    /// module and the manager-level wiring test cannot drift onto different
    /// event shapes.
    fn spend_event<B: TransactionBroadcaster>(
        core: &CoreWallet<B>,
        tx: &Transaction,
    ) -> key_wallet_manager::WalletEvent {
        crate::test_support::observed_spend_event(core.wallet_id(), tx)
    }

    /// An `Arc<PlatformWallet>` sharing `core`'s manager, wallet id, and
    /// generation — the entry the production wallets map holds for this
    /// wallet, so the spend-observation tests can resolve the REAL registered
    /// generation through a real map. Its own `SpvBroadcaster` is inert: the
    /// spend-observation seam only ever reads `generation()` through it.
    fn platform_wallet_sharing<B: TransactionBroadcaster>(
        core: &CoreWallet<B>,
    ) -> Arc<crate::wallet::PlatformWallet> {
        let spv = Arc::new(crate::spv::SpvRuntime::new(
            Arc::clone(&core.wallet_manager),
            Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
        ));
        Arc::new(crate::wallet::PlatformWallet::new(
            Arc::clone(&core.sdk),
            core.wallet_id(),
            Arc::clone(&core.wallet_manager),
            Arc::clone(core.generation()),
            Arc::new(tokio::sync::Notify::new()),
            Arc::new(crate::test_support::NoopTestPersister)
                as Arc<dyn crate::changeset::PlatformWalletPersistence>,
            Arc::new(crate::broadcaster::SpvBroadcaster::new(spv)),
        ))
    }

    /// A wallets map — the production `BTreeMap<WalletId, Arc<PlatformWallet>>`
    /// behind its own `ArcSwap` — holding one entry per fixture wallet.
    fn wallets_map<B: TransactionBroadcaster>(
        cores: &[&CoreWallet<B>],
    ) -> Arc<
        arc_swap::ArcSwap<std::collections::BTreeMap<WalletId, Arc<crate::wallet::PlatformWallet>>>,
    > {
        Arc::new(arc_swap::ArcSwap::from_pointee(
            cores
                .iter()
                .map(|core| (core.wallet_id(), platform_wallet_sharing(core)))
                .collect(),
        ))
    }

    /// Retire fences from `event` by driving the PRODUCTION spend-observation
    /// seam end to end: a real [`SpendObservationHandler`] over a real wallets
    /// map whose entry shares `core`'s registered generation. `on_wallet_event`
    /// therefore exercises the whole handler path — the variant gate
    /// (`observing_wallet`), the projection (`observed_spends`), the
    /// wallets-map `try_read`, the wallet-id lookup, and the selected
    /// generation's release — not a shortcut to `observe_spent`.
    fn observe_via_event_handler<B: TransactionBroadcaster>(
        core: &CoreWallet<B>,
        event: key_wallet_manager::WalletEvent,
    ) {
        assert!(
            !crate::wallet::core::spend_observer::observed_spends(&event).is_empty(),
            "the fixture event must report at least one spend, or the test \
             would pass without observing anything"
        );
        let handler = SpendObservationHandler::new(wallets_map(&[core]));
        dash_spv::EventHandler::on_wallet_event(&handler, &event);
    }

    /// Assert that `result` is the typed in-broadcast conflict, and return the
    /// outpoint it names.
    ///
    /// Matching the typed `PlatformWalletError::InputMidBroadcast` variant,
    /// never `message.contains("mid-broadcast")`: substring-matching prose is
    /// exactly what the typed variant exists to make unnecessary.
    fn expect_mid_broadcast(
        result: Result<SignedCoreTransaction, PlatformWalletError>,
        context: &str,
    ) -> dashcore::OutPoint {
        match result {
            Err(PlatformWalletError::InputMidBroadcast { outpoint }) => outpoint,
            other => panic!("{context}, got {other:?}"),
        }
    }

    /// THE CHECK-TO-WIRE RACE the in-broadcast pin closes: the freshness
    /// check passes under the manager read guard, the guard drops, and the
    /// dispatch suspends inside the broadcaster BEFORE submission. Catch-up
    /// then advances the clock past key-wallet's reservation TTL, so a
    /// competing finalize's own selection sweeps the dispatched build's
    /// reservation and re-selects its input — pre-pin, that build completed
    /// and raced the already-signed transaction on the wire. With the pin
    /// held across the await, the competing finalize must be REFUSED.
    ///
    /// The tail covers the HANDOFF from the dispatching pin to the
    /// pending-spend fence: the dispatch returns, the pin lifts, and the
    /// outpoint stays fenced anyway — now with no dependence on how far the
    /// 48-block catch-up moved the chain clock.
    #[tokio::test]
    async fn in_broadcast_pin_blocks_reselection_until_dispatch_returns() {
        let entered = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Barrier::new(2));
        let (core, signer, outputs) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(GatedBroadcaster {
                entered: Arc::clone(&entered),
                release: Arc::clone(&release),
            }),
        )
        .await;
        let stamped = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let finalized = finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let fenced = finalized.transaction().input[0].previous_output;

        // Dispatch at the oldest height the age guard admits, so the pin is
        // taken and the broadcaster then parks pre-submission.
        advance_processed_height(&core, stamped + RESERVATION_MAX_AGE_BLOCKS - 1).await;
        let dispatcher = tokio::spawn({
            let core = core.clone();
            async move { core.broadcast_finalized_transaction(&finalized).await }
        });
        entered.wait().await;

        // Catch-up well past key-wallet's reservation TTL while the dispatch is
        // suspended: the reservation is swept, so only the pin holds the input.
        advance_processed_height(&core, stamped + RESERVATION_MAX_AGE_BLOCKS + 48).await;
        let racing = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        assert_eq!(
            expect_mid_broadcast(
                racing,
                "a build that swept a mid-dispatch reservation must be refused"
            ),
            fenced,
            "the refusal must name the conflicting outpoint"
        );

        release.wait().await;
        let sent = dispatcher.await.expect("dispatcher task");
        assert!(
            sent.is_ok(),
            "the pinned dispatch itself must complete, got {sent:?}"
        );

        // The dispatching pin has now lifted — and the input is STILL not
        // selectable. Asserting that it is free again would be the bug, and a
        // height-anchored bound would only defer it. The assertion holds
        // regardless of the chain clock, because the fence is waiting for an
        // observed spend that this mock manager — which runs no mempool
        // pipeline, exactly like the `DapiBroadcaster` path — never produces.
        let still_fenced =
            try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        expect_mid_broadcast(
            still_fenced,
            "the broadcaster returning is not the spend being observed, so the \
             input must stay fenced",
        );
    }

    /// THE HISTORICAL-CATCH-UP HAZARD, STATED DIRECTLY.
    ///
    /// > after [the guard is released], a synchronization writer queued during
    /// > the short critical section — or ordinary catch-up completing before
    /// > the next build — can immediately advance `last_processed_height` by
    /// > the whole interval. […] Those elapsed heights may be historical blocks
    /// > mined BEFORE the transaction was submitted, so they provide no
    /// > evidence that the submitted transaction has been observed or dropped.
    ///
    /// This is the test that FAILS for any height-bounded fence. An
    /// implementation that installs `pending_until = <some height> + N` and
    /// reaps the fence once `last_processed_height` reaches it loses here: the
    /// catch-up below clears that bound by a wide margin no matter which
    /// height is sampled — pre-await, post-await, or post-await under a held
    /// guard — so every such variant leaves the input reselectable while the
    /// transaction is on the network.
    ///
    /// The broadcaster is `AlwaysOk`: the transaction is ACCEPTED, so it is
    /// certainly on the wire. The manager runs no mempool pipeline, which is
    /// the `DapiBroadcaster` shape — `sdk.execute` returns without injecting
    /// anything locally — so nothing has observed the spend.
    #[tokio::test]
    async fn fence_survives_a_full_historical_catch_up_advance() {
        let (core, signer, outputs) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        let stamped = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let finalized = finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let fenced = finalized.transaction().input[0].previous_output;

        assert!(core
            .broadcast_finalized_transaction(&finalized)
            .await
            .is_ok());

        // Historical catch-up. Not a few blocks past some bound — a whole
        // month of blocks, all of them mined long before this transaction was
        // submitted, applied in the instant between the dispatch returning and
        // the next build. This is the ordinary mobile resync, and it consumes
        // any height-anchored bound.
        let caught_up = stamped + 17_000;
        advance_processed_height(&core, caught_up).await;

        let racing = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        assert_eq!(
            expect_mid_broadcast(
                racing,
                "historical catch-up must not retire a fence: those blocks predate \
                 the transaction and are not evidence it was seen or dropped"
            ),
            fenced,
        );

        // And it is not merely slow to expire — no amount of further chain
        // progress retires it either.
        advance_processed_height(&core, caught_up + 500_000).await;
        expect_mid_broadcast(
            try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
            "no quantity of elapsed height may retire the fence",
        );

        // The ONLY thing that can: an observed spend.
        core.generation().observe_spent([fenced]);
        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after.unwrap_or_else(|error| {
            panic!("an observed spend must release the fence, got {error:?}")
        });
        core.abandon_transaction(&after).await;
    }

    /// The fence's designed release: the wallet OBSERVES the dispatched
    /// transaction's own spend, off the wallet-event fan-out.
    ///
    /// Drives the real seam — [`SpendObservationHandler`] fed a
    /// `TransactionDetected` event carrying the dispatched transaction — rather
    /// than calling `observe_spent` directly, so this covers the projection
    /// from a `WalletEvent` to the outpoints it retires. That projection is
    /// shared with `CoreChangeSet::spent_utxos`, so the fence and the
    /// persister's spent set cannot disagree.
    #[tokio::test]
    async fn observing_the_dispatched_transaction_releases_the_fence() {
        let (core, signer, outputs) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        let stamped = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let finalized = finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let sent_tx = finalized.transaction().clone();
        let fenced = sent_tx.input[0].previous_output;

        assert!(core
            .broadcast_finalized_transaction(&finalized)
            .await
            .is_ok());

        // Catch-up past key-wallet's 24-block reservation TTL, so the funding
        // reservation is swept and the input is back in the selectable pool.
        // That is the window the fence exists for — without it the reservation,
        // not the fence, is what refuses the competing build, and this test
        // would pass without exercising the fence at all.
        advance_processed_height(&core, stamped + 17_000).await;
        assert_eq!(
            expect_mid_broadcast(
                try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
                "the dispatched input must be fenced before the spend is observed",
            ),
            fenced,
        );

        // The wallet sees its own transaction — mempool relay on the DAPI path,
        // or the local pipeline on the SPV one. Either way this event is what
        // arrives.
        observe_via_event_handler(&core, spend_event(&core, &sent_tx));

        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after.unwrap_or_else(|error| {
            panic!("observing the dispatch's own spend must release the fence, got {error:?}")
        });
        core.abandon_transaction(&after).await;
    }

    /// A COMPETING spend releases the fence too. The outpoint has left this
    /// wallet's selectable set whoever spent it, so there is no re-selection
    /// left that could race anything on the wire — continuing to fence would
    /// hold the input for good and protect nothing.
    #[tokio::test]
    async fn observing_a_competing_spend_releases_the_fence() {
        let (core, signer, outputs) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        let stamped = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let finalized = finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let fenced = finalized.transaction().input[0].previous_output;

        assert!(core
            .broadcast_finalized_transaction(&finalized)
            .await
            .is_ok());

        // Sweep the funding reservation (see the sibling test above), so the
        // fence is the only thing holding the input.
        advance_processed_height(&core, stamped + 17_000).await;
        expect_mid_broadcast(
            try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
            "the dispatched input must be fenced before any spend is observed",
        );

        // A DIFFERENT transaction spending the same outpoint — a competing
        // spend the wallet observes. Its txid differs from the dispatched one.
        let mut competing = finalized.transaction().clone();
        competing.lock_time = finalized.transaction().lock_time + 1;
        assert_ne!(
            competing.txid(),
            finalized.transaction().txid(),
            "the fixture must model a genuinely different transaction"
        );

        observe_via_event_handler(&core, spend_event(&core, &competing));

        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after.unwrap_or_else(|error| {
            panic!("a competing spend must also release the fence, got {error:?}")
        });
        assert_eq!(
            after.transaction().input[0].previous_output,
            fenced,
            "the fixture has one UTXO, so the rebuild reselects the same outpoint"
        );
        core.abandon_transaction(&after).await;
    }

    /// A WALLETS-MAP WRITE IN FLIGHT MUST NOT COST A SPEND OBSERVATION.
    ///
    /// `SpendObservationHandler::on_wallet_event` is synchronous, so over a
    /// `tokio::sync::RwLock` wallets map it could only probe with `try_read` —
    /// a probe that fails while wallet registration/removal holds the write
    /// lock. For a DAPI-path dispatch the observation lost that way can be the
    /// only spend-bearing event the wallet ever gets: InstantLock promotions
    /// carry no record here by design, and an evicted or never-confirmed
    /// transaction produces no inserted `BlockProcessed` record. With no
    /// deadline behind the pending-spend fence, one moment of lock contention
    /// would leave the input fenced for the manager's lifetime even though the
    /// wallet HAD observed it spent.
    ///
    /// The map is an `ArcSwap`, so the read cannot fail and the handler
    /// applies every observation at delivery — no deferral queue, no window
    /// to lose it in. The closest reachable analogue of that contention is a
    /// lifecycle writer parked mid-`rcu`, which this test pins open across the
    /// delivery: the fence must clear anyway, before that writer commits.
    #[tokio::test]
    async fn a_wallets_map_write_in_flight_does_not_cost_a_spend_observation() {
        let (core, signer, outputs) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        let stamped = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let finalized = finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let sent_tx = finalized.transaction().clone();

        assert!(core
            .broadcast_finalized_transaction(&finalized)
            .await
            .is_ok());

        // Sweep the funding reservation, so the pending-spend fence is the
        // only thing holding the input (see the sibling release tests).
        advance_processed_height(&core, stamped + 17_000).await;
        expect_mid_broadcast(
            try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
            "the dispatched input must be fenced before any spend is observed",
        );

        let map = wallets_map(&[&core]);
        let handler = SpendObservationHandler::new(Arc::clone(&map));

        // Park a lifecycle writer mid-publication: its `rcu` closure has read
        // the current map but not yet committed the replacement. This is the
        // window the lock-based map failed its `try_read` in.
        let (entered_tx, entered_rx) = std::sync::mpsc::channel::<()>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let map_for_writer = Arc::clone(&map);
        let writer = std::thread::spawn(move || {
            // `rcu` re-runs its closure if the compare-and-swap loses, so the
            // release must be waited on ONCE: a second `recv()` would block
            // forever on a channel the test only sends to once, and
            // `writer.join()` below would hang the suite instead of failing
            // it. Nothing else writes this map today, so the retry is latent
            // — which is exactly why it must not be able to wedge the test.
            let mut parked = false;
            map_for_writer.rcu(|current| {
                if !parked {
                    parked = true;
                    let _ = entered_tx.send(());
                    let _ = release_rx.recv();
                }
                Arc::clone(current)
            });
        });
        entered_rx
            .recv()
            .expect("the writer must reach its rcu closure");

        dash_spv::EventHandler::on_wallet_event(&handler, &spend_event(&core, &sent_tx));

        // Applied at delivery — before the parked writer commits.
        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after.unwrap_or_else(|error| {
            panic!(
                "an observation delivered while a wallets-map write was in \
                 flight must still release the fence: {error:?}"
            )
        });

        release_tx.send(()).expect("writer still parked");
        writer.join().expect("writer thread completes");

        core.abandon_transaction(&after).await;
    }

    /// The handler releases ONLY the generation registered under the event's
    /// wallet id. Two fenced wallets
    /// share ONE wallets map — the production shape — and: an event naming a
    /// wallet id registered NOWHERE releases neither fence, and wallet A's own
    /// spend event releases A's fence while B's stands. A handler that routed
    /// by anything but the event's wallet id, or that failed its map lookup
    /// open, fails one of the two halves.
    #[tokio::test]
    async fn spend_observation_releases_only_the_matching_registered_generation() {
        let (core_a, signer_a, outputs_a) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        let (core_b, signer_b, outputs_b) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        assert_ne!(
            core_a.wallet_id(),
            core_b.wallet_id(),
            "the fixture must model two distinct wallets"
        );

        // Dispatch both wallets' single UTXO and sweep both funding
        // reservations, so each fence is the only thing holding its input
        // (see the sibling release tests).
        let mut sent = Vec::new();
        for (core, signer, outputs) in [
            (&core_a, &signer_a, &outputs_a),
            (&core_b, &signer_b, &outputs_b),
        ] {
            let stamped = core
                .last_processed_height()
                .await
                .expect("last processed height");
            let finalized = finalize_tx(core, AccountTypePreference::BIP44, outputs, signer).await;
            sent.push(finalized.transaction().clone());
            assert!(core
                .broadcast_finalized_transaction(&finalized)
                .await
                .is_ok());
            advance_processed_height(core, stamped + 17_000).await;
            expect_mid_broadcast(
                try_finalize_tx(core, AccountTypePreference::BIP44, outputs, signer).await,
                "the dispatched input must be fenced before any spend is observed",
            );
        }

        let handler = SpendObservationHandler::new(wallets_map(&[&core_a, &core_b]));

        // An event naming a wallet id registered NOWHERE: the lookup misses,
        // nothing panics, and neither fence moves — the fail-safe direction.
        let mut foreign = spend_event(&core_a, &sent[0]);
        match &mut foreign {
            key_wallet_manager::WalletEvent::TransactionDetected { wallet_id, .. } => {
                *wallet_id = [0xEE; 32];
            }
            other => unreachable!("the fixture builds TransactionDetected, got {other:?}"),
        }
        dash_spv::EventHandler::on_wallet_event(&handler, &foreign);
        for (core, signer, outputs) in [
            (&core_a, &signer_a, &outputs_a),
            (&core_b, &signer_b, &outputs_b),
        ] {
            expect_mid_broadcast(
                try_finalize_tx(core, AccountTypePreference::BIP44, outputs, signer).await,
                "an event for an unregistered wallet must release no fence",
            );
        }

        // Wallet A's own spend event: A's registered generation releases,
        // B's — same map, same handler, different wallet id — stands.
        dash_spv::EventHandler::on_wallet_event(&handler, &spend_event(&core_a, &sent[0]));
        let rebuilt = try_finalize_tx(&core_a, AccountTypePreference::BIP44, &outputs_a, &signer_a)
            .await
            .unwrap_or_else(|error| {
                panic!("the matching wallet's fence must release, got {error:?}")
            });
        core_a.abandon_transaction(&rebuilt).await;
        expect_mid_broadcast(
            try_finalize_tx(&core_b, AccountTypePreference::BIP44, &outputs_b, &signer_b).await,
            "the other registered wallet's fence must stand",
        );
    }

    /// THE END-TO-END ELAPSED-TIME REGRESSION.
    ///
    /// A pending-spend phase that expires one hour after the dispatch settles,
    /// on a monotonic clock, is unsound. The clock is the right kind — catch-up
    /// cannot move it — but a deadline of ANY kind is the wrong instrument: the
    /// signed transaction stays valid, and an hour passing proves nothing about
    /// whether a peer retained it. A DAPI endpoint that accepts the transaction
    /// while withholding it from the network, or an app backgrounded past the
    /// deadline, is enough. With key-wallet's reservation also swept by
    /// catch-up, the next build then re-selects the input and SIGNS A
    /// CONFLICTING TRANSACTION over a spend that might still land.
    ///
    /// This drives that exact sequence through the real send path: accept the
    /// transaction (`AlwaysOk`, and this manager runs no mempool pipeline — the
    /// `DapiBroadcaster` shape, so nothing observes the spend), run catch-up far
    /// past key-wallet's reservation TTL, bring due every timeout the fence might
    /// carry, and build again. Under a deadline-bearing fence that second build
    /// SUCCEEDS and returns a second signed transaction spending the same
    /// input. It must be refused, and released only by the observed spend.
    #[tokio::test]
    async fn an_elapsed_deadline_cannot_retire_the_fence_a_spend_still_needs() {
        let (core, signer, outputs) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        let stamped = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let finalized = finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let fenced = finalized.transaction().input[0].previous_output;

        assert!(core
            .broadcast_finalized_transaction(&finalized)
            .await
            .is_ok());

        // Catch-up runs far past key-wallet's reservation TTL, so the funding
        // reservation is swept and the input is selectable again as far as
        // key-wallet is concerned. The fence is the only thing still holding it.
        advance_processed_height(&core, stamped + 17_000).await;
        expect_mid_broadcast(
            try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
            "chain progress must not retire the fence",
        );

        // Now let every elapsed-time release the fence might carry come due —
        // the hour of wall clock a monotonic backstop would wait out.
        assert!(
            core.generation().test_elapse_time_based_release(&fenced),
            "the accepted dispatch must be in the pending-spend phase"
        );

        assert_eq!(
            expect_mid_broadcast(
                try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
                "an elapsed deadline must not hand back an input whose transaction \
                 may still be on the wire — the old backstop let this build sign a \
                 conflicting spend of it",
            ),
            fenced,
        );

        // The one release that carries evidence.
        core.generation().observe_spent([fenced]);
        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after.unwrap_or_else(|error| {
            panic!("an observed spend must release the fence, got {error:?}")
        });
        core.abandon_transaction(&after).await;
    }

    /// The CANCELLATION path.
    ///
    /// A caller wrapping the send in `timeout`/`select!` drops the dispatching
    /// future mid-`broadcast`. That path reaches neither the release nor any
    /// return value, and cancellation proves nothing: DAPI may have delivered
    /// the request while awaiting its response, SPV may have dispatched to
    /// peers while awaiting an echo or IS-lock. So the fence must survive it —
    /// and it needs no special case to do so: `Drop` sets the same flag the
    /// normal path does, so a cancelled dispatch settles exactly like a
    /// returning one.
    ///
    /// Catch-up runs far past any height bound a fence could have installed
    /// before the abort.
    #[tokio::test]
    async fn cancelled_dispatch_keeps_its_fence_across_catch_up() {
        let entered = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Barrier::new(2));
        let (core, signer, outputs) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(GatedBroadcaster {
                entered: Arc::clone(&entered),
                release: Arc::clone(&release),
            }),
        )
        .await;
        let stamped = core
            .last_processed_height()
            .await
            .expect("last processed height");
        let finalized = finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let fenced = finalized.transaction().input[0].previous_output;

        let dispatcher = tokio::spawn({
            let core = core.clone();
            async move { core.broadcast_finalized_transaction(&finalized).await }
        });
        // Parked inside `broadcast`: pin held, guard dropped, nothing decided.
        entered.wait().await;

        advance_processed_height(&core, stamped + 17_000).await;

        // Cancel mid-await, exactly as `timeout`/`select!` would. Awaiting the
        // handle guarantees the future — and with it `InBroadcastPin::drop` —
        // has actually run before the assertions below.
        dispatcher.abort();
        let cancelled = dispatcher.await;
        assert!(
            cancelled.is_err_and(|error| error.is_cancelled()),
            "the dispatching future must have been cancelled mid-broadcast"
        );

        assert_eq!(
            expect_mid_broadcast(
                try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
                "a cancelled dispatch may have reached the network, so its fence \
                 must survive — including across catch-up"
            ),
            fenced,
        );

        // A cancelled dispatch's fence is released the same way every other one
        // is — by evidence, and by nothing that merely elapses. Letting any
        // timeout it might carry come due changes nothing.
        assert!(
            core.generation().test_elapse_time_based_release(&fenced),
            "the cancelled dispatch must have settled into the pending phase"
        );
        expect_mid_broadcast(
            try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
            "cancellation says nothing about what reached the network, so no \
             elapsed deadline may hand the input back",
        );

        core.generation().observe_spent([fenced]);
        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after.unwrap_or_else(|error| {
            panic!("an observed spend must release a cancelled dispatch's fence, got {error:?}")
        });
        core.abandon_transaction(&after).await;
    }

    /// The rejection path is the one outcome that frees the inputs at dispatch
    /// return: Core definitively did not accept the transaction, so there is
    /// nothing on the wire to fence against and an immediate rebuild must
    /// reselect. No pending-spend fence may be installed.
    #[tokio::test]
    async fn definitively_rejected_dispatch_installs_no_fence() {
        let (core, signer, outputs) = funded_core_wallet(
            StandardAccountType::BIP44Account,
            Arc::new(RejectFirstBroadcaster::new()),
        )
        .await;
        let finalized = finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;

        let sent = core.broadcast_finalized_transaction(&finalized).await;
        assert!(
            matches!(sent, Err(PlatformWalletError::TransactionBroadcast(_))),
            "the fixture must reject the first send, got {sent:?}"
        );

        // Rejection released the reservation AND installed no fence, so the
        // rebuild succeeds at the very next height with no waiting.
        let rebuilt = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let rebuilt = rebuilt.unwrap_or_else(|error| {
            panic!("a definitively rejected send must leave its inputs free, got {error:?}")
        });
        core.abandon_transaction(&rebuilt).await;
    }

    /// A pre-send broadcast rejection must release the UTXO reservation taken
    /// while building the transaction, so an immediate retry can reselect those
    /// inputs instead of failing with spurious insufficient funds until the TTL
    /// backstop. Covers both funds-account arms of the release path.
    #[tokio::test]
    async fn broadcast_releases_reservation_on_rejection() {
        for account_type in [
            StandardAccountType::BIP44Account,
            StandardAccountType::BIP32Account,
        ] {
            let broadcaster = Arc::new(RejectFirstBroadcaster::new());
            let (core, signer, outputs) = funded_core_wallet(account_type, broadcaster).await;

            // First attempt: build + sign reserve the input, broadcast is rejected.
            let tx = build_signed_tx(&core, account_type, 0, &outputs, &signer)
                .await
                .expect("first build should succeed");
            let first = core
                .broadcast_transaction_releasing_reservation(account_type, 0, &tx)
                .await;
            assert!(
                matches!(first, Err(PlatformWalletError::TransactionBroadcast(_))),
                "first broadcast should surface the rejection for {account_type:?}, got {first:?}"
            );

            // Immediate retry: the build only succeeds if the failed broadcast
            // released the reservation. With the leak, coin selection sees no
            // spendable UTXO and the build fails.
            let retry_tx = build_signed_tx(&core, account_type, 0, &outputs, &signer).await;
            assert!(
                retry_tx.is_ok(),
                "retry build after a released reservation should succeed for \
                 {account_type:?}, got {retry_tx:?}"
            );
            let second = core
                .broadcast_transaction_releasing_reservation(
                    account_type,
                    0,
                    &retry_tx.expect("retry tx"),
                )
                .await;
            assert!(
                second.is_ok(),
                "retry broadcast should succeed for {account_type:?}, got {second:?}"
            );
        }
    }

    /// An *ambiguous* broadcast failure — the network may already have accepted
    /// the transaction — must NOT release the reservation: retrying would risk a
    /// double-spend. The reservation is kept, so an immediate retry fails at the
    /// build stage (no spendable UTXO) rather than reaching broadcast again.
    #[tokio::test]
    async fn broadcast_keeps_reservation_on_ambiguous_failure() {
        for account_type in [
            StandardAccountType::BIP44Account,
            StandardAccountType::BIP32Account,
        ] {
            let broadcaster = Arc::new(AlwaysMaybeSentBroadcaster);
            let (core, signer, outputs) = funded_core_wallet(account_type, broadcaster).await;

            let tx = build_signed_tx(&core, account_type, 0, &outputs, &signer)
                .await
                .expect("first build should succeed");
            let first = core
                .broadcast_transaction_releasing_reservation(account_type, 0, &tx)
                .await;
            assert!(
                matches!(
                    first,
                    Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
                ),
                "first broadcast should surface the ambiguous failure for \
                 {account_type:?}, got {first:?}"
            );

            // Reservation kept: the retry cannot reselect the reserved input and
            // fails while building, never reaching the broadcaster again.
            let second = build_signed_tx(&core, account_type, 0, &outputs, &signer).await;
            assert!(
                matches!(second, Err(PlatformWalletError::TransactionBuild(_))),
                "retry build must fail with the reservation kept for \
                 {account_type:?}, got {second:?}"
            );
        }
    }
}
