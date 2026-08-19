use dashcore::Transaction;
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::ReservationToken;

use super::generation::InBroadcastPin;
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
    /// transaction is in flight (`dashpay/platform#4309`). So:
    ///
    /// * **Definitive pre-send rejection** (`BroadcastError::Rejected`) — the
    ///   transaction provably did not reach the network. The fence is dropped
    ///   immediately here, and the caller releases the reservation in the same
    ///   breath, so an instant rebuild can reselect the inputs.
    /// * **Anything else** (accepted, or an ambiguous `MaybeSent`) — the pin is
    ///   converted to a pending-spend fence
    ///   ([`settle_dispatch_fence`](Self::settle_dispatch_fence))
    ///   lasting
    ///   [`IN_BROADCAST_FENCE_BLOCKS`](crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS)
    ///   past a `last_processed_height` sampled **after** the broadcaster
    ///   returned. Once the wallet does observe the spend the outpoint stops
    ///   reaching selection at all, so the fence goes inert without waiting for
    ///   that bound; the bound is only the backstop for a transaction that is
    ///   never observed, and matches the TTL the reservation itself would have
    ///   had, re-anchored at the moment the transaction actually went out.
    ///
    /// # Why the fence anchor is sampled after the await, not before
    ///
    /// The height that authorizes the send and the height that bounds the fence
    /// are the same *clock* but must not be the same *reading*. A broadcast
    /// await can suspend for minutes in the middle of chain catch-up — the
    /// ordinary mobile case — and if the wallet advances a full
    /// `IN_BROADCAST_FENCE_BLOCKS` in that gap, a fence anchored on the
    /// pre-await reading is already expired at the instant it is installed. The
    /// next coin selection reaps it and may reselect an input of a transaction
    /// that reached the network: an expired fence is indistinguishable from no
    /// fence (`dashpay/platform#4309`).
    ///
    /// So the post-await sample is the SINGLE anchor for the installed fence,
    /// which keeps the check-vs-fence clock consistency intact — both readings
    /// come from `last_processed_height` under the manager guard, the same clock
    /// `in_broadcast_conflict` and key-wallet's TTL sweep run on. The
    /// **dispatching** phase stays held across that sampling, so there is no
    /// window in which the outpoints are neither pinned nor fenced.
    ///
    /// Sampling *and* installing happen under ONE guard
    /// ([`settle_dispatch_fence`](Self::settle_dispatch_fence)), so the
    /// dispatching→pending handoff is atomic with respect to height writers: no
    /// manager writer can advance the clock between the reading and the install,
    /// which is what would otherwise let the fence arrive already lapsed while
    /// the dispatching hold was being lifted (`dashpay/platform#4309`, review
    /// round 2).
    ///
    /// A dispatch that never reaches the sample — the caller's future cancelled
    /// or unwound inside `broadcast`, or the wallet removed from the manager —
    /// settles its fence UNANCHORED, blocking unconditionally until the first
    /// coin selection stamps it from its own height. That is the drop-time
    /// clock, deferred to the first moment a synchronous `Drop` could act on it,
    /// and the fence is unreachable in between.
    ///
    /// Neither phase touches the wallet-manager lock while the broadcaster is
    /// running, so nothing here can starve the SPV mempool pipeline: the guard
    /// is still dropped before the broadcaster await, exactly as it was, and the
    /// post-await sample is a short read taken only once the send has returned —
    /// the same point the callers below already retake manager locks at.
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
            // `height` is NOT handed to the pin. It anchors the freshness
            // check and nothing else; the fence's own anchor is sampled after
            // the await below, for the reason documented there.
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
        // double-spend a transaction already on the wire (`dashpay/platform#4309`).
        //
        // Only a definitive pre-send rejection proves nothing was sent, so it is
        // the one outcome that releases. An ambiguous `MaybeSent` stays fenced.
        if let Some(pin) = in_broadcast_pin {
            if matches!(
                outcome,
                Err(crate::broadcaster::BroadcastError::Rejected { .. })
            ) {
                // Provably nothing on the wire: free the outpoints outright.
                // No height and no guard — this installs no bound, so there is
                // nothing a concurrent height advance could mistime.
                pin.settle_released();
            } else {
                // EVERY non-rejection outcome — accepted or ambiguous
                // `MaybeSent` — hands the pin to the guarded settle below.
                self.settle_dispatch_fence(pin).await;
            }
        }
        GuardedDispatch::Sent(outcome)
    }

    /// Close out one dispatch's **dispatching** phase and install its
    /// **pending-spend** fence, both inside a SINGLE wallet-manager read guard.
    ///
    /// The fence is bounded a full
    /// [`IN_BROADCAST_FENCE_BLOCKS`](crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS)
    /// past a `last_processed_height` sampled here, after the broadcaster
    /// returned. The pre-await sample cannot serve: `broadcast` can suspend for
    /// minutes mid-catch-up (mobile), and if the wallet advances a whole fence
    /// interval inside the await, a fence anchored back there is ALREADY LAPSED
    /// when it is installed — the next coin selection reaps it and can reselect
    /// an input of a transaction that may have reached the network. An expired
    /// fence is no fence (`dashpay/platform#4309`).
    ///
    /// # Why the guard spans the settle, not just the sample
    ///
    /// Sampling the height under the guard is not enough on its own. The
    /// dispatching→pending transition is the pin's settle, and while the sample
    /// and the settle sat in two different critical sections the guard was
    /// released between them: a manager writer queued behind it — SPV catch-up
    /// applying a batch of blocks is exactly that writer — could advance
    /// `last_processed_height` in the gap, so the fence landed anchored on a
    /// height the wallet had already left, at the same instant the dispatching
    /// hold was lifted. The outpoint went from fully held to fully free with no
    /// live pending-spend phase in between (`dashpay/platform#4309`, review
    /// round 2).
    ///
    /// Holding the guard across both makes the transition atomic with respect to
    /// height writers, so the installed bound is measured from the height that
    /// is current at the instant the fence becomes the only protection — the
    /// fence is never born dead. `settle_pending_spend` CONSUMES the pin so this
    /// is a statement placed inside the guard's scope rather than an
    /// end-of-scope drop that a later edit could float back outside it.
    ///
    /// # This cannot deadlock, and cannot starve the SPV pipeline
    ///
    /// The settle takes only `WalletGeneration::in_broadcast`, a
    /// `std::sync::Mutex`, for a few hash operations; it never awaits and never
    /// touches the wallet-manager lock. That is the crate's existing lock order
    /// — manager lock, then `in_broadcast`, exactly as every
    /// `in_broadcast_conflict` call site takes them under the manager WRITE
    /// guard — so no inversion is possible. And the guarded section is still the
    /// short, await-free read the broadcaster await was deliberately kept out
    /// of; nothing here is held across a network wait.
    ///
    /// A wallet that left the manager yields no height and settles the pin
    /// UNANCHORED, which fences unconditionally until the first coin selection
    /// stamps it from its own clock: strictly the safer side.
    async fn settle_dispatch_fence(&self, pin: InBroadcastPin) {
        let manager = self.wallet_manager.read().await;
        let height = manager
            .get_wallet_info(&self.wallet_id)
            .map(|info| info.core_wallet.last_processed_height());
        // Consumes the pin: the dispatching hold lifts and the bound lands in
        // one critical section, with `manager` still held around both.
        pin.settle_pending_spend(height);
        drop(manager);
    }

    /// Broadcast an atomically finalized transaction. A definitive rejection
    /// releases its reservation; an ambiguous `MaybeSent` outcome retains it.
    ///
    /// The release is owner-guarded by the finalized transaction's
    /// [`reservation_token`](SignedCoreTransaction::reservation_token): the
    /// broadcast is `.await`ed, and during that await key-wallet's TTL sweep can
    /// reclaim this build's reservation and a concurrent build re-reserve the
    /// same inputs under a new token. Releasing by outpoint alone would then
    /// free that other build's inputs (the `dashpay/platform#4185` double-spend
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
    /// `StandardAccountType`, and which previously kept its reservation held
    /// until the TTL backstop.
    ///
    /// The release delegates to
    /// [`release_transaction_reservation`](Self::release_transaction_reservation),
    /// so it acts only on the wallet *generation* this handle names (a wallet
    /// re-created under the same id between build and broadcast cannot have its
    /// reservation freed by this token) AND — via `token` — only on inputs this
    /// build still owns. The deferred registry can hold the reservation across a
    /// long build→broadcast gap, so a TTL sweep re-reserving the same inputs
    /// under a new token is a real risk; the owner guard closes the
    /// `dashpay/platform#4185` release/re-reserve race.
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
    use crate::wallet::core::CoreWallet;
    use crate::wallet::reservations::{IN_BROADCAST_FENCE_BLOCKS, RESERVATION_MAX_AGE_BLOCKS};
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

    /// THE CHECK-TO-WIRE RACE the in-broadcast pin closes: the freshness
    /// check passes under the manager read guard, the guard drops, and the
    /// dispatch suspends inside the broadcaster BEFORE submission. Catch-up
    /// then advances the clock past key-wallet's reservation TTL, so a
    /// competing finalize's own selection sweeps the dispatched build's
    /// reservation and re-selects its input — pre-pin, that build completed
    /// and raced the already-signed transaction on the wire. With the pin
    /// held across the await, the competing finalize must be REFUSED.
    ///
    /// The tail then covers the HANDOFF from the dispatching pin to the
    /// pending-spend fence, which is where the catch-up in this test matters a
    /// second time: the fence is anchored on the height sampled AFTER the
    /// broadcaster returned, so a 48-block advance *inside* the await extends
    /// the protection instead of expiring it (`dashpay/platform#4309`).
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

        // Age the handle to ONE BELOW the guard bound: the freshness check
        // must pass, which is exactly what makes the pre-submission window
        // dangerous without the pin.
        advance_processed_height(&core, stamped + RESERVATION_MAX_AGE_BLOCKS - 1).await;

        let dispatcher = tokio::spawn({
            let core = core.clone();
            async move { core.broadcast_finalized_transaction(&finalized).await }
        });
        // The dispatcher is now suspended INSIDE the broadcaster: freshness
        // checked, manager guard dropped, pin held.
        entered.wait().await;

        // Catch-up races far past key-wallet's TTL measured from the original
        // reservation stamp, so the NEXT selection's sweep reclaims the
        // dispatched build's reservation and its input returns to the
        // selectable pool.
        advance_processed_height(&core, stamped + RESERVATION_MAX_AGE_BLOCKS + 48).await;

        // The competing finalize re-selects the fixture's only UTXO — the
        // pinned input — and must be refused by the pin backstop, not
        // completed into a conflicting signed transaction.
        let competing =
            try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        match competing {
            Err(PlatformWalletError::TransactionBuild(message)) => assert!(
                message.contains("mid-broadcast"),
                "the refusal must name the in-flight broadcast, got: {message}"
            ),
            other => panic!("a build re-selecting a pinned input must be refused, got {other:?}"),
        }

        // Let the dispatch complete: the send succeeds (the age check passed
        // before the suspension).
        release.wait().await;
        let sent = dispatcher.await.expect("dispatcher task");
        assert!(
            sent.is_ok(),
            "the pinned dispatch itself must complete, got {sent:?}"
        );

        // The dispatching pin has now lifted — but the input is NOT selectable
        // again, and the difference is the point of `dashpay/platform#4309`.
        //
        // This assertion used to read the other way: the fence was anchored on
        // the height sampled BEFORE the await, the catch-up above raced 48
        // blocks past it, and the fence was therefore installed already lapsed,
        // so a new build took the input immediately. That "the pin lifted" pass
        // was the bug in test form — the transaction had reached the network.
        //
        // The fence is now anchored on the POST-await sample, which is the
        // height this catch-up moved to, so the input stays held.
        let post_await = stamped + RESERVATION_MAX_AGE_BLOCKS + 48;
        assert!(
            post_await >= (stamped + RESERVATION_MAX_AGE_BLOCKS - 1) + IN_BROADCAST_FENCE_BLOCKS,
            "this test's catch-up must outrun a PRE-await anchor, so the \
             assertion below distinguishes the two anchors"
        );
        let still_fenced =
            try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        match still_fenced {
            Err(PlatformWalletError::TransactionBuild(message)) => assert!(
                message.contains("mid-broadcast"),
                "the post-dispatch refusal must name the in-flight broadcast, got: {message}"
            ),
            other => panic!(
                "catch-up during the await must not expire the fence before it \
                 is installed, got {other:?}"
            ),
        }

        // And it lapses a full interval past that post-await anchor — bounded,
        // not permanent.
        advance_processed_height(&core, post_await + IN_BROADCAST_FENCE_BLOCKS).await;
        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after.unwrap_or_else(|error| {
            panic!("the fence must lapse a bound past the post-await anchor, got {error:?}")
        });
        core.abandon_transaction(&after).await;
    }

    /// `dashpay/platform#4309`: THE RACE THE DISPATCHING PIN ALONE LEFT OPEN.
    /// The broadcaster returning is not the spend being observed. The mock
    /// manager here runs no mempool pipeline, which is precisely the
    /// `DapiBroadcaster` shape — `broadcast` awaits `sdk.execute` and injects
    /// nothing into this wallet's state — so at dispatch return the input is
    /// still in the selectable set while the transaction is in flight. With the
    /// pin dropped at that point, a competing build re-selected it immediately
    /// (the previous revision of the test above asserted exactly that). The
    /// pending-spend fence keeps it out until
    /// `IN_BROADCAST_FENCE_BLOCKS` past the dispatch height, and no longer:
    /// a never-observed transaction must not strand its inputs forever.
    ///
    /// Heights are chosen so the reservation is provably swept while the fence
    /// still stands — the state pre-fix was "unreserved AND unfenced".
    #[tokio::test]
    async fn dispatched_input_stays_fenced_after_the_broadcaster_returns() {
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

        // Dispatch at the OLDEST height the age guard still admits — one below
        // `RESERVATION_MAX_AGE_BLOCKS`. That is what separates the two clocks:
        // the reservation's TTL runs from `stamped`, the fence's bound from
        // here, so there is a window in which the reservation is swept and only
        // the fence protects the input. (A handle sitting between finalize and
        // broadcast is exactly how that gap arises in production.)
        let dispatch_height = stamped + RESERVATION_MAX_AGE_BLOCKS - 1;
        advance_processed_height(&core, dispatch_height).await;
        assert!(core
            .broadcast_finalized_transaction(&finalized)
            .await
            .is_ok());

        // Catch-up past key-wallet's 24-block reservation TTL (measured from
        // the reservation stamp), so the funding reservation is swept and the
        // input returns to the selectable pool — but still short of the fence's
        // dispatch-anchored bound. The fence is now the ONLY thing holding it;
        // pre-fix this window was unreserved AND unfenced.
        let swept_but_fenced = stamped + IN_BROADCAST_FENCE_BLOCKS + 4;
        assert!(
            swept_but_fenced >= stamped + 24
                && swept_but_fenced < dispatch_height + IN_BROADCAST_FENCE_BLOCKS,
            "the probe height must be past key-wallet's reservation TTL and below the fence bound"
        );
        advance_processed_height(&core, swept_but_fenced).await;
        let racing = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        match racing {
            Err(PlatformWalletError::TransactionBuild(message)) => assert!(
                message.contains("mid-broadcast"),
                "the post-dispatch refusal must name the in-flight broadcast, got: {message}"
            ),
            other => panic!(
                "an input handed to the network must stay fenced after the \
                 broadcaster returns, got {other:?}"
            ),
        }

        // At the bound the fence lapses and the input is selectable again.
        advance_processed_height(&core, dispatch_height + IN_BROADCAST_FENCE_BLOCKS).await;
        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after
            .unwrap_or_else(|error| panic!("the fence must lapse at its bound, got {error:?}"));
        core.abandon_transaction(&after).await;
    }

    /// `dashpay/platform#4309`: THE FENCE MUST NOT BE INSTALLED ALREADY EXPIRED.
    ///
    /// The scenario the reviewer identified, which fencing-by-default alone did
    /// not close. The pending-spend bound used to be anchored on the
    /// `last_processed_height` sampled BEFORE `broadcaster.broadcast().await`.
    /// A broadcast await can suspend for minutes in the middle of chain
    /// catch-up — routine on mobile — and if the wallet advances a full
    /// `IN_BROADCAST_FENCE_BLOCKS` inside that await, the fence installed when
    /// the send returns is ALREADY LAPSED. The very next coin selection reaps it
    /// and reselects the input of a transaction that may be on the network: an
    /// expired fence is indistinguishable from no fence at all.
    ///
    /// Here the broadcaster parks, catch-up runs a full fence interval plus 5
    /// blocks past the pre-await sample, and the send then returns `Ok` — the
    /// accepted case, where the transaction is certainly on the wire. The input
    /// must still be fenced afterwards, and the bound must run from the
    /// POST-await height.
    ///
    /// The reservation is provably swept by then (the catch-up outruns
    /// key-wallet's 24-block TTL measured from the build stamp), so the fence is
    /// the only thing holding the input — pre-fix this window was unreserved AND
    /// unfenced, and the competing finalize below returned `Ok`.
    #[tokio::test]
    async fn fence_anchors_after_the_await_so_catch_up_cannot_pre_expire_it() {
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

        // Dispatch while fresh: the pre-await sample is `stamped`, so that is
        // the anchor the old code would have used.
        let dispatcher = tokio::spawn({
            let core = core.clone();
            async move { core.broadcast_finalized_transaction(&finalized).await }
        });
        entered.wait().await;

        // Catch-up INSIDE the await runs a whole fence interval past the
        // pre-await sample. A fence anchored back there expires at
        // `stamped + IN_BROADCAST_FENCE_BLOCKS`, which this height has passed.
        let post_await = stamped + IN_BROADCAST_FENCE_BLOCKS + 5;
        assert!(
            post_await >= stamped + IN_BROADCAST_FENCE_BLOCKS,
            "the catch-up must outrun a pre-await anchor for this test to \
             distinguish the two"
        );
        assert!(
            post_await >= stamped + 24,
            "the catch-up must also outrun key-wallet's reservation TTL, so the \
             fence is the only thing holding the input"
        );
        advance_processed_height(&core, post_await).await;

        // The send returns accepted — the transaction IS on the network.
        release.wait().await;
        let sent = dispatcher.await.expect("dispatcher task");
        assert!(
            sent.is_ok(),
            "the dispatch itself must succeed, got {sent:?}"
        );

        // Pre-fix: the fence was installed already lapsed and this build
        // succeeded, constructing a double-spend of a transaction on the wire.
        let racing = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        match racing {
            Err(PlatformWalletError::TransactionBuild(message)) => assert!(
                message.contains("mid-broadcast"),
                "the refusal must name the in-flight broadcast, got: {message}"
            ),
            other => panic!(
                "a fence anchored before the await expires during catch-up and \
                 leaves the input reselectable, got {other:?}"
            ),
        }

        // The bound runs from the POST-await height: still fenced one below it,
        // lapsed at it. Bounded protection, not a permanent hold.
        advance_processed_height(&core, post_await + IN_BROADCAST_FENCE_BLOCKS - 1).await;
        assert!(
            matches!(
                try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
                Err(PlatformWalletError::TransactionBuild(_))
            ),
            "one block below the post-await bound must still fence"
        );
        advance_processed_height(&core, post_await + IN_BROADCAST_FENCE_BLOCKS).await;
        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after.unwrap_or_else(|error| {
            panic!("the fence must lapse at the post-await bound, got {error:?}")
        });
        core.abandon_transaction(&after).await;
    }

    /// `dashpay/platform#4309`, REVIEW ROUND 2: THE DISPATCHING→PENDING HANDOFF
    /// MUST BE ATOMIC AGAINST MANAGER WRITERS.
    ///
    /// Anchoring on the post-await sample is not enough if the guard that sample
    /// was read under is released before the fence is installed. The sample sat
    /// in one critical section and `drop(pin)` — the statement that lifts the
    /// dispatching hold AND installs the bound — sat outside it. A manager
    /// writer queued behind that guard (SPV catch-up applying a batch of blocks
    /// is exactly that writer) is woken the instant it is released, and on
    /// another worker thread it can advance `last_processed_height` before the
    /// resuming dispatch reaches the install. The fence then lands anchored on a
    /// height the wallet has already left, in the same instant the dispatching
    /// hold goes away: born dead, reaped by the next selection, with the input
    /// of a possibly-sent transaction reselectable.
    ///
    /// The writer here parks on the manager WRITE lock while the dispatch is
    /// inside its post-await section, then advances a full fence interval, and
    /// records what the transition looked like at the moment it was granted the
    /// lock. Exactly two interleavings are legal, and the fence must be live
    /// under both:
    ///
    /// * granted AFTER the handoff — it must see the pin settled (`dispatching`
    ///   lifted) with a bound that is still ahead of the height it holds, i.e.
    ///   the fence was born live; or
    /// * granted BEFORE the dispatch even sampled — the dispatch then reads the
    ///   advanced height, so the installed bound must run from the writer's own
    ///   advance.
    ///
    /// The torn third case — granted mid-handoff, so it sees the pin still
    /// dispatching AND the fence ends up anchored below its advance — is what
    /// the released guard allowed, and is what this fails on.
    ///
    /// Repeated, because the interleaving is scheduler-dependent: the writer has
    /// to be granted the lock inside the window to observe a split, and with the
    /// sample and the settle adjacent that window is a handful of instructions.
    /// It widens the moment anything suspends between them — an added `.await`,
    /// a second guarded read — which is the regression this guards against, and
    /// which a single attempt catches only intermittently.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn settle_does_not_interleave_with_a_parked_height_writer() {
        for attempt in 0..16 {
            parked_writer_handoff_attempt(attempt).await;
        }
    }

    /// One scenario run of
    /// [`settle_does_not_interleave_with_a_parked_height_writer`] — see its docs
    /// for what the two legal interleavings are and why the third fails.
    async fn parked_writer_handoff_attempt(attempt: u32) {
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

        // A full fence interval plus a margin: a bound anchored on `stamped` is
        // dead at this height, so an anchor below it is observably stale.
        let writer_height = stamped + IN_BROADCAST_FENCE_BLOCKS + 5;

        let dispatcher = tokio::spawn({
            let core = core.clone();
            async move { core.broadcast_finalized_transaction(&finalized).await }
        });
        // Parked inside `broadcast`: freshness checked, pre-await guard already
        // dropped, pin held, and nothing holds the manager lock.
        entered.wait().await;

        let writer = tokio::spawn({
            let manager = Arc::clone(&core.wallet_manager);
            let generation = Arc::clone(core.generation());
            let wallet_id = core.wallet_id();
            async move {
                // Wait for the dispatch to be INSIDE its post-await manager read
                // section. While nothing holds the lock a `try_write` succeeds,
                // so the first failure is that read guard being held. Bounded:
                // if the section is missed the writer simply parks late, which
                // is the "granted after the handoff" case below.
                let mut spins = 0u32;
                while manager.try_write().is_ok() {
                    spins += 1;
                    if spins >= 200_000 {
                        break;
                    }
                    if spins.is_multiple_of(512) {
                        tokio::task::yield_now().await;
                    }
                }

                // Park behind that read guard, exactly as an SPV catch-up batch
                // waiting to apply blocks does.
                let mut guard = manager.write().await;
                // What did the handoff look like the moment we were granted it?
                let observed = generation.in_broadcast_fence_state(&fenced);
                let (_, info) = guard
                    .get_wallet_and_info_mut(&wallet_id)
                    .expect("wallet present in manager");
                let height_at_grant = info.core_wallet.last_processed_height();
                info.core_wallet.update_last_processed_height(writer_height);
                drop(guard);
                (observed, height_at_grant)
            }
        });

        release.wait().await;
        let sent = dispatcher.await.expect("dispatcher task");
        assert!(
            sent.is_ok(),
            "the dispatch itself must succeed, got {sent:?}"
        );
        let (observed, height_at_grant) = writer.await.expect("writer task");

        let installed = core
            .generation()
            .in_broadcast_fence_state(&fenced)
            .expect("the dispatched input must still carry a fence");

        match observed {
            // Granted after the handoff completed: the bound must already have
            // been installed, and installed LIVE at the height then current.
            Some((0, false, Some(until))) => assert!(
                until > height_at_grant,
                "attempt {attempt}: the fence must be born live — bound {until} \
                 was already lapsed at the height {height_at_grant} current when \
                 the dispatching hold was lifted"
            ),
            // Granted before the dispatch sampled: the dispatch then reads the
            // advanced height, so the bound must run from the writer's advance.
            Some((1, false, None)) => assert!(
                installed
                    .2
                    .is_some_and(|until| until >= writer_height + IN_BROADCAST_FENCE_BLOCKS),
                "attempt {attempt}: a writer that advanced the clock to \
                 {writer_height} before the dispatch sampled must have its \
                 advance reflected in the installed fence, got {installed:?} — a \
                 lower bound means the dispatching→pending handoff interleaved \
                 with the advance and the fence was installed on a stale height"
            ),
            other => panic!(
                "attempt {attempt}: unexpected fence state at the writer's \
                 grant: {other:?} (installed: {installed:?})"
            ),
        }

        // Whichever way it went, the fence is still BOUNDED — this hardening
        // must not turn the pending-spend phase into a permanent hold. (The
        // writer's own advance may already have consumed the bound: that is the
        // designed lapse, not a defect, so only probe below it when there is a
        // below.)
        let lapses_at = installed.2.expect("an anchored bound");
        if lapses_at > writer_height {
            assert!(
                matches!(
                    try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
                    Err(PlatformWalletError::TransactionBuild(_))
                ),
                "the input must stay fenced below the installed bound"
            );
        }
        advance_processed_height(&core, lapses_at.max(writer_height)).await;
        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after
            .unwrap_or_else(|error| panic!("the fence must lapse at its bound, got {error:?}"));
        core.abandon_transaction(&after).await;
    }

    /// `dashpay/platform#4309`, the CANCELLATION twin of the test above.
    ///
    /// A caller wrapping the send in `timeout`/`select!` drops the dispatching
    /// future mid-`broadcast`. That path reaches neither the release nor the
    /// post-await height sample, and cancellation proves nothing: DAPI may have
    /// delivered the request while awaiting its response, SPV may have
    /// dispatched to peers while awaiting an echo or IS-lock. Retaining the
    /// fence there is necessary but NOT sufficient — if the retained fence is
    /// stamped from a height sampled before the await, the same catch-up that
    /// made the caller time out has already expired it.
    ///
    /// So a cancelled dispatch settles its fence with NO bound, and the first
    /// coin selection to consult it stamps a full interval from its own height,
    /// read under the manager write guard. Here catch-up runs well past a fence
    /// interval before the abort, and the input must still be refused.
    #[tokio::test]
    async fn cancelled_dispatch_fence_survives_catch_up_during_the_await() {
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

        let dispatcher = tokio::spawn({
            let core = core.clone();
            async move { core.broadcast_finalized_transaction(&finalized).await }
        });
        // Parked inside `broadcast`: pin held, guard dropped, nothing decided.
        entered.wait().await;

        // Catch-up past both key-wallet's reservation TTL and a whole fence
        // interval measured from the pre-await sample.
        let cancelled_at = stamped + IN_BROADCAST_FENCE_BLOCKS + 5;
        advance_processed_height(&core, cancelled_at).await;

        // Cancel mid-await, exactly as `timeout`/`select!` would. Awaiting the
        // handle guarantees the future — and with it `InBroadcastPin::drop` —
        // has actually run before the assertions below.
        dispatcher.abort();
        let cancelled = dispatcher.await;
        assert!(
            cancelled.is_err_and(|error| error.is_cancelled()),
            "the dispatching future must have been cancelled mid-broadcast"
        );

        // Pre-fix: the retained fence carried the pre-await anchor and was
        // already lapsed here, so this build reselected an input whose
        // transaction may have been delivered.
        let racing = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        match racing {
            Err(PlatformWalletError::TransactionBuild(message)) => assert!(
                message.contains("mid-broadcast"),
                "the refusal must name the in-flight broadcast, got: {message}"
            ),
            other => panic!(
                "a cancelled dispatch must not leave an already-expired fence, \
                 got {other:?}"
            ),
        }

        // The build above is what anchored the fence, on the height it read.
        // The interval runs from there, and then lapses.
        advance_processed_height(&core, cancelled_at + IN_BROADCAST_FENCE_BLOCKS - 1).await;
        assert!(
            matches!(
                try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await,
                Err(PlatformWalletError::TransactionBuild(_))
            ),
            "one block below the anchored bound must still fence"
        );
        advance_processed_height(&core, cancelled_at + IN_BROADCAST_FENCE_BLOCKS).await;
        let after = try_finalize_tx(&core, AccountTypePreference::BIP44, &outputs, &signer).await;
        let after = after.unwrap_or_else(|error| {
            panic!("a cancelled dispatch's fence must still lapse, got {error:?}")
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
