use std::collections::BTreeSet;

use dashcore::{Address as DashAddress, OutPoint, Transaction};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

use crate::broadcaster::TransactionBroadcaster;
use crate::{CoreWallet, PlatformWalletError};

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// Broadcast a signed transaction to the network.
    ///
    /// Build the transaction using key-wallet's
    /// [`TransactionBuilder`](key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder),
    /// then pass the result here for broadcasting.
    ///
    /// Delegates to the injected [`TransactionBroadcaster`] which may use
    /// SPV (P2P) or DAPI (gRPC) depending on how the wallet was constructed.
    ///
    /// Returns the transaction ID on success.
    pub async fn broadcast_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        self.broadcaster.broadcast(transaction).await
    }

    /// Build, sign, and broadcast a payment to the given addresses.
    ///
    /// Uses key-wallet's [`TransactionBuilder`] for UTXO selection, fee
    /// estimation, and signing. Change is sent to the next internal address
    /// of the specified account.
    ///
    /// ## Race-safety against concurrent calls on the same wallet
    ///
    /// Coin selection consults a per-wallet **reservation set** (see
    /// [`super::reservations`]) under the write lock. Selected outpoints
    /// are reserved before the lock is dropped, so a second concurrent
    /// caller — which acquires the write lock after this one — sees the
    /// reserved outpoints filtered out of its spendable snapshot. If that
    /// leaves the second caller with insufficient inputs, it short-circuits
    /// with [`PlatformWalletError::NoSpendableInputs`] *before* touching
    /// the network. The reservation is held until either:
    ///
    /// - broadcast succeeds → `check_core_transaction(Mempool, …)` marks
    ///   the inputs spent under the second write lock, then the guard
    ///   drops; the spent transition is observable to other callers with
    ///   no gap, or
    /// - any error path → the guard drops and the outpoints are released,
    ///   so a retry can pick them up again.
    pub async fn send_to_addresses(
        &self,
        account_type: StandardAccountType,
        account_index: u32,
        outputs: Vec<(DashAddress, u64)>,
    ) -> Result<Transaction, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;

        if outputs.is_empty() {
            return Err(PlatformWalletError::TransactionBuild(
                "No outputs specified".to_string(),
            ));
        }

        let (tx, xpub, _reservation) = {
            let mut wm = self.wallet_manager.write().await;
            let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;

            let current_height = info.core_wallet.synced_height();

            // Look up managed account and immutable Account (for xpub) based on type.
            let (managed_accounts, wallet_accounts) = match account_type {
                StandardAccountType::BIP44Account => (
                    &mut info.core_wallet.accounts.standard_bip44_accounts,
                    &wallet.accounts.standard_bip44_accounts,
                ),
                StandardAccountType::BIP32Account => (
                    &mut info.core_wallet.accounts.standard_bip32_accounts,
                    &wallet.accounts.standard_bip32_accounts,
                ),
            };

            let account = managed_accounts.get(&account_index).ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "{:?} account {} not found",
                    account_type, account_index
                ))
            })?;

            // Snapshot spendable UTXOs minus any in-flight reservations from
            // a concurrent `send_to_addresses` on this handle. Any outpoint
            // already reserved is owned by another caller's still-pending
            // broadcast and must not be selected here.
            let spendable: Vec<_> = account
                .spendable_utxos(current_height)
                .into_iter()
                .filter(|utxo| !self.reservations.contains(&utxo.outpoint))
                .cloned()
                .collect();

            if spendable.is_empty() {
                return Err(PlatformWalletError::NoSpendableInputs {
                    context: format!(
                        "{:?} account {} (all UTXOs reserved by in-flight transactions)",
                        account_type, account_index
                    ),
                });
            }

            let xpub = wallet_accounts
                .get(&account_index)
                .map(|a| a.account_xpub)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(format!(
                        "{:?} account {} not found in wallet",
                        account_type, account_index
                    ))
                })?;

            let mut builder = TransactionBuilder::new();
            for (addr, amount) in &outputs {
                builder = builder
                    .add_output(addr, *amount)
                    .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
            }

            // Need mutable access for change address derivation.
            let change_account = managed_accounts.get_mut(&account_index).ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "{:?} managed account {} not found",
                    account_type, account_index
                ))
            })?;

            // Peek at the next change address without advancing the derivation
            // index. We commit the advance only after post-build revalidation
            // succeeds, so a revalidation failure does not burn an index and
            // widen the gap-limit window on retry.
            let change_addr = change_account
                .next_change_address(Some(&xpub), false)
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            builder = builder.set_change_address(change_addr);

            builder = builder
                .select_inputs(
                    &spendable,
                    SelectionStrategy::LargestFirst,
                    current_height,
                    |utxo| {
                        for account in info.core_wallet.accounts.all_accounts() {
                            // Address pools live on the keys variant
                            // after the funds/keys split; the funds
                            // account composes a keys account, and
                            // `keys_account()` exposes it for both
                            // `ManagedAccountRef` variants.
                            if let Some(path) = account
                                .keys_account()
                                .address_derivation_path(&utxo.address)
                            {
                                if let Ok(key) = wallet.derive_private_key(&path) {
                                    return Some(key);
                                }
                            }
                        }
                        None
                    },
                )
                .map_err(|e| {
                    // Insufficient/no-utxo errors from coin selection map
                    // to the typed `NoSpendableInputs` variant so callers
                    // can distinguish "race-loser" from "network rejected
                    // my tx". `select_inputs` wraps the underlying
                    // `SelectionError` in a `BuilderError`; we string-match
                    // here because the typed wrapper is not exposed.
                    let msg = e.to_string();
                    if msg.contains("Insufficient funds") || msg.contains("No UTXOs available") {
                        PlatformWalletError::NoSpendableInputs {
                            context: format!(
                                "{:?} account {} ({})",
                                account_type, account_index, msg
                            ),
                        }
                    } else {
                        PlatformWalletError::TransactionBuild(msg)
                    }
                })?;

            let tx = builder
                .build()
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            // Defense-in-depth: by builder contract `tx.input` outpoints are
            // a subset of the height-aware `spendable` set we passed to
            // `select_inputs`, so this branch is unreachable in normal
            // operation. Marking inputs spent is deferred to after broadcast
            // (see #3466) regardless.
            let selected: BTreeSet<OutPoint> =
                tx.input.iter().map(|txin| txin.previous_output).collect();
            let spendable_outpoints: BTreeSet<OutPoint> =
                spendable.iter().map(|utxo| utxo.outpoint).collect();
            if !selected.is_subset(&spendable_outpoints) {
                // INTENTIONAL(CMT-002): typed variant kept user-retryable for
                // forward compatibility with cross-process concurrent-spend
                // surfacing — even though today only builder regression hits.
                return Err(PlatformWalletError::ConcurrentSpendConflict);
            }

            // Reserve the selected outpoints *before* releasing the write
            // lock. The next caller acquiring the lock will see these
            // outpoints filtered out and either select disjoint inputs or
            // short-circuit with `NoSpendableInputs`.
            //
            // The guard is held until the end of the function: success
            // path drops it after `check_core_transaction` has marked the
            // inputs spent (no observable gap); error paths drop it on
            // unwinding the `Result`, releasing the outpoints for retry.
            let reservation = self.reservations.reserve(selected.into_iter().collect());

            (tx, xpub, reservation)
        };

        // Broadcast first; if the network rejects we leave wallet state
        // untouched so the caller can retry without manual sync repair.
        // This is intentional even if the remote accepted the transaction
        // but the broadcast path returned an error: in that ambiguous case
        // later attempts may reuse the same inputs locally, but the network
        // rejects the duplicate spend instead of us marking UTXOs spent for
        // a transaction that might not have propagated.
        self.broadcast_transaction(&tx).await?;

        // Now that the tx is in flight, register it as a mempool transaction
        // so subsequent callers see the inputs as spent and don't reselect
        // them. The reservation guard above kept those inputs filtered out
        // for concurrent callers throughout the broadcast `await`; this
        // call transitions them from "reserved" to "spent" before the
        // guard drops, so the spent state is observable with no gap.
        //
        // Broadcast-first semantics: by the time we get here the network has
        // already accepted the transaction, so the two warning paths below
        // intentionally do NOT convert into a post-success `Err`. They
        // simply mean local wallet state did not get updated to reflect the
        // mempool spend / change output. Recovery in both cases:
        //
        //   * The next `send_to_addresses` from the same handle may reselect
        //     the same UTXOs because they still look spendable locally. That
        //     follow-up transaction will be rejected by the network as a
        //     duplicate spend (the broadcaster surfaces that as an error to
        //     the caller), so funds are never double-spent on-chain.
        //   * Once mempool/block sync catches up, the wallet will see the
        //     original transaction and reconcile its UTXO set, after which
        //     subsequent sends pick up the correct change outputs.
        //
        // The two cases differ in what they imply:
        //
        //   * `!check_result.is_relevant` is the expected transient: the
        //     wallet just hasn't ingested the tx yet (or some derivation
        //     path/script is unrecognised), and a later sync will fix it.
        //   * The `else` branch (wallet missing in the manager) is NOT a
        //     normal transient — the broadcast succeeded against a
        //     `CoreWallet` handle whose underlying wallet entry is gone
        //     from the manager. That is a broken/inconsistent local handle
        //     and the warning exists so operators can spot it; future
        //     sends through the same handle will keep failing the lookup
        //     above and surface a clean `WalletNotFound` error.
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some((wallet, info)) = wm.get_wallet_mut_and_info_mut(&self.wallet_id) {
                // Broadcast succeeded — commit the change-address advance now
                // so a future send picks up a fresh index. Doing this before
                // the broadcast would burn a derivation index on a network
                // rejection, widening the gap-limit window on retry.
                let change_account = match account_type {
                    StandardAccountType::BIP44Account => info
                        .core_wallet
                        .accounts
                        .standard_bip44_accounts
                        .get_mut(&account_index),
                    StandardAccountType::BIP32Account => info
                        .core_wallet
                        .accounts
                        .standard_bip32_accounts
                        .get_mut(&account_index),
                };
                if let Some(change_account) = change_account {
                    if let Err(e) = change_account.next_change_address(Some(&xpub), true) {
                        // Broadcast already succeeded; surface as a warning
                        // rather than an error so the caller still sees the
                        // tx hash. A later sync reconciles the index.
                        tracing::warn!(
                            target: "platform_wallet::broadcast",
                            event = "post_broadcast_change_index_advance_failed",
                            txid = %tx.txid(),
                            wallet_id = %hex::encode(self.wallet_id),
                            error = %e,
                            "failed to advance change-address index after successful broadcast"
                        );
                    }
                }

                let check_result = info
                    .check_core_transaction(&tx, TransactionContext::Mempool, wallet, true, true)
                    .await;
                if !check_result.is_relevant {
                    // CMT-004: own-built tx unrecognised by our own checker
                    // is an internal-invariant violation, not a transient.
                    // Structured `error!` with stable fields so operators can
                    // alert independent of message text.
                    tracing::error!(
                        target: "platform_wallet::broadcast",
                        event = "post_broadcast_unrelated_to_own_wallet",
                        txid = %tx.txid(),
                        wallet_id = %hex::encode(self.wallet_id),
                        "Internal invariant violation: own-built broadcast not recognized by post-broadcast check"
                    );
                }
            } else {
                // INTENTIONAL(CMT-005): log-only is sufficient until metrics
                // infrastructure exists; see broadcast-first rationale above.
                tracing::warn!(
                    wallet_id = %hex::encode(self.wallet_id),
                    txid = %tx.txid(),
                    "wallet missing during post-broadcast transaction registration"
                );
            }
        }

        // Reservation guard drops here, releasing the outpoints. The
        // `check_core_transaction` call above already marked them spent
        // under the write lock — there is no observable gap during which
        // both the reservation is gone and the spent state isn't yet
        // visible.
        drop(_reservation);

        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    //! `broadcast_transaction` pass-through contract.
    //!
    //! Pins that the wrapper does not transform `Err` or modify the success
    //! result — the `Txid` returned by the broadcaster is forwarded unchanged.
    //! The higher-level `send_to_addresses` rollback contract (#3466) is not
    //! covered here; pinning it would require live wallet fixtures.
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use dashcore::consensus::deserialize;
    use dashcore::{Transaction, Txid};
    use tokio::sync::RwLock;

    use crate::broadcaster::TransactionBroadcaster;
    use crate::wallet::core::balance::WalletBalance;
    use crate::wallet::core::CoreWallet;
    use crate::PlatformWalletError;
    use key_wallet::Network;
    use key_wallet_manager::WalletManager;

    /// Records every call and returns a canned outcome.
    struct MockBroadcaster {
        outcome: BroadcastOutcome,
        calls: AtomicUsize,
    }

    enum BroadcastOutcome {
        Ok(Txid),
        Err(String),
    }

    impl MockBroadcaster {
        fn new(outcome: BroadcastOutcome) -> Self {
            Self {
                outcome,
                calls: AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl TransactionBroadcaster for MockBroadcaster {
        async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, PlatformWalletError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match &self.outcome {
                BroadcastOutcome::Ok(txid) => Ok(*txid),
                BroadcastOutcome::Err(msg) => {
                    Err(PlatformWalletError::TransactionBroadcast(msg.clone()))
                }
            }
        }
    }

    /// Minimal serialized tx (1 input, 1 output, 0 value) — only the
    /// broadcaster's Err/Ok branch matters here, not the shape.
    fn dummy_transaction() -> Transaction {
        let bytes = hex::decode(
            "010000000100000000000000000000000000000000000000000000000000000000000000\
             00ffffffff00ffffffff0100000000000000000000000000",
        )
        .expect("valid hex");
        deserialize(&bytes).expect("deserializable tx")
    }

    fn make_core_wallet<B: TransactionBroadcaster + ?Sized>(broadcaster: Arc<B>) -> CoreWallet<B> {
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .build()
                .expect("mock sdk build"),
        );
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(Network::Testnet)));
        CoreWallet::new(
            sdk,
            wallet_manager,
            [0u8; 32],
            broadcaster,
            Arc::new(WalletBalance::new()),
        )
    }

    /// `broadcast_transaction` forwards a broadcaster `Err` to the caller
    /// without transformation.
    #[tokio::test]
    async fn broadcast_transaction_passes_through_err_unchanged() {
        let broadcaster = Arc::new(MockBroadcaster::new(BroadcastOutcome::Err(
            "simulated network rejection".to_string(),
        )));
        let wallet = make_core_wallet(Arc::clone(&broadcaster));
        let tx = dummy_transaction();

        let result = wallet.broadcast_transaction(&tx).await;

        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "expected broadcast Err to propagate, got {:?}",
            result
        );
        assert_eq!(
            broadcaster.call_count(),
            1,
            "broadcaster must be called exactly once on a failed broadcast"
        );
    }

    /// `broadcast_transaction` forwards the broadcaster's `Txid` to the
    /// caller without transformation.
    #[tokio::test]
    async fn broadcast_transaction_passes_through_ok_unchanged() {
        let expected_txid = dummy_transaction().txid();
        let broadcaster = Arc::new(MockBroadcaster::new(BroadcastOutcome::Ok(expected_txid)));
        let wallet = make_core_wallet(Arc::clone(&broadcaster));
        let tx = dummy_transaction();

        let result = wallet.broadcast_transaction(&tx).await;

        assert_eq!(
            result.expect("broadcast Ok"),
            expected_txid,
            "broadcast_transaction must pass the broadcaster's Txid through unchanged"
        );
        assert_eq!(
            broadcaster.call_count(),
            1,
            "broadcaster must be called exactly once on a successful broadcast"
        );
    }

    // -----------------------------------------------------------------
    // Race-closing tests: same-UTXO concurrent `send_to_addresses`.
    //
    // The audit (`/tmp/pr3585-race-audit.md`) captures the property:
    // two callers A and B must not both broadcast against the same
    // outpoint. The reservation set guarantees B short-circuits with
    // `NoSpendableInputs` *before* hitting the network — never with a
    // `TransactionBroadcast` failure (that would mean B reached the
    // broadcaster, which is exactly the bug being closed).
    // -----------------------------------------------------------------

    use std::collections::BTreeMap;
    use std::time::Duration;

    use dashcore::hashes::Hash;
    use dashcore::{Address as DashAddress, OutPoint, TxOut};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use key_wallet::Utxo;
    use tokio::sync::Notify;

    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

    /// Mock broadcaster that gates the broadcast on an external `Notify`.
    /// Lets the test pin caller A inside its `await` while caller B
    /// races for the wallet's write lock.
    struct GatedBroadcaster {
        gate: Arc<Notify>,
        succeed: bool,
    }

    #[async_trait]
    impl TransactionBroadcaster for GatedBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, PlatformWalletError> {
            // Wait until the test signals the gate. Allows the test to
            // observe the wallet state mid-broadcast (specifically: the
            // reservation set populated, the input not yet marked spent).
            self.gate.notified().await;
            if self.succeed {
                Ok(transaction.txid())
            } else {
                Err(PlatformWalletError::TransactionBroadcast(
                    "mock failure".to_string(),
                ))
            }
        }
    }

    /// Always-failing mock broadcaster — used to assert that a failed
    /// broadcast releases the reservation so a retry can pick up the
    /// same UTXO.
    struct FailingBroadcaster;

    #[async_trait]
    impl TransactionBroadcaster for FailingBroadcaster {
        async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, PlatformWalletError> {
            Err(PlatformWalletError::TransactionBroadcast(
                "always fails".to_string(),
            ))
        }
    }

    /// Build a single-wallet `WalletManager` containing one BIP-44
    /// account (index 0) funded with one large UTXO at the account's
    /// first receive address. Returns the wallet manager handle, the
    /// wallet id, and a recipient address (a separate derived address
    /// in the same account — funding/sending to the same address is
    /// not the property under test).
    fn build_funded_wallet_manager(
        utxo_value: u64,
    ) -> (
        Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        crate::wallet::platform_wallet::WalletId,
        DashAddress,
    ) {
        let wallet = Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default)
            .expect("test wallet");

        let xpub = wallet
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .expect("bip44 account 0")
            .account_xpub;
        let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, 0);

        // Lift the synced height well past the UTXO height so the
        // `min_confirmations >= 1` filter in coin selection accepts the
        // UTXO. Without this, the UTXO appears in `spendable_utxos` (its
        // own `is_spendable` check passes) but `select_coins_with_size`
        // filters it out via the confirmation-count guard.
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface as _;
        wallet_info.update_synced_height(100);

        let funding_address = wallet_info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("managed bip44 account 0")
            .next_receive_address(Some(&xpub), true)
            .expect("derive receive address");

        let outpoint = OutPoint::new(Txid::from_byte_array([7u8; 32]), 0);
        let mut utxo = Utxo::new(
            outpoint,
            TxOut {
                value: utxo_value,
                script_pubkey: funding_address.script_pubkey(),
            },
            funding_address,
            1,
            false,
        );
        utxo.is_confirmed = true;
        wallet_info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("managed bip44 account 0")
            .utxos
            .insert(outpoint, utxo);

        let info = PlatformWalletInfo {
            core_wallet: wallet_info,
            balance: Arc::new(WalletBalance::new()),
            identity_manager: crate::wallet::identity::IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
        };

        let mut wm: WalletManager<PlatformWalletInfo> = WalletManager::new(Network::Testnet);
        let wallet_id = wm.insert_wallet(wallet, info).expect("insert");

        // Recipient — use the second receive address as a stable target.
        let recipient = {
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("acc")
                .next_receive_address(Some(&xpub), true)
                .expect("derive recipient")
        };

        (Arc::new(RwLock::new(wm)), wallet_id, recipient)
    }

    fn make_core_wallet_for_manager<B: TransactionBroadcaster + ?Sized>(
        wm: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: crate::wallet::platform_wallet::WalletId,
        broadcaster: Arc<B>,
    ) -> CoreWallet<B> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        CoreWallet::new(
            sdk,
            wm,
            wallet_id,
            broadcaster,
            Arc::new(WalletBalance::new()),
        )
    }

    /// Race test — the headline property: two concurrent `send_to_addresses`
    /// calls on the same wallet, against the same single spendable UTXO,
    /// must yield exactly one network broadcast. The loser must short-circuit
    /// with [`PlatformWalletError::NoSpendableInputs`] *before* hitting the
    /// network — i.e. the loser's error must NOT be a `TransactionBroadcast`
    /// failure (that would mean it reached the broadcaster, which is exactly
    /// the bug we're closing).
    #[tokio::test]
    async fn concurrent_same_utxo_sends_resolve_via_reservation_set() {
        use key_wallet::account::account_type::StandardAccountType;

        let (wm, wallet_id, recipient) = build_funded_wallet_manager(2_000_000);
        let gate = Arc::new(Notify::new());
        let broadcaster: Arc<dyn TransactionBroadcaster> = Arc::new(GatedBroadcaster {
            gate: Arc::clone(&gate),
            succeed: true,
        });
        let core = make_core_wallet_for_manager(wm, wallet_id, broadcaster);

        let send_value = 100_000;
        let outputs_a = vec![(recipient.clone(), send_value)];
        let outputs_b = vec![(recipient.clone(), send_value)];

        // Spawn caller A. It will reserve the only spendable outpoint
        // under the wallet write lock, drop the lock, and block on the
        // broadcast `Notify`.
        let core_a = core.clone();
        let a_handle = tokio::spawn(async move {
            core_a
                .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs_a)
                .await
        });

        // Give A enough scheduler time to acquire the lock, build the
        // tx, reserve the outpoint, and reach the gated broadcast.
        // The property is monotonic — once A is in the broadcast
        // `await`, the reservation is in place forever until the gate
        // fires.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Caller B starts now. The wallet's only UTXO is reserved by A,
        // so B's spendable snapshot is empty → `NoSpendableInputs`.
        let b_result = core
            .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs_b)
            .await;

        match &b_result {
            Err(PlatformWalletError::NoSpendableInputs { context }) => {
                assert!(
                    context.contains("reserved")
                        || context.contains("Insufficient")
                        || context.contains("No UTXOs"),
                    "B's NoSpendableInputs context should mention reservation \
                     or insufficient/no-utxos; got: {context}"
                );
            }
            other => panic!(
                "B must short-circuit with NoSpendableInputs (the race-loser \
                 must not reach the broadcaster); got: {other:?}"
            ),
        }

        // Now release A's broadcast.
        gate.notify_one();

        let a_result = a_handle.await.expect("a task panicked");
        assert!(
            a_result.is_ok(),
            "A must succeed once its broadcast gate fires; got: {a_result:?}"
        );
    }

    /// On broadcast failure, the reservation must be released so the
    /// caller can retry. This is the regression-tripwire for the
    /// reservation guard's Drop semantics.
    #[tokio::test]
    async fn broadcast_failure_releases_reservation_for_retry() {
        use key_wallet::account::account_type::StandardAccountType;

        let (wm, wallet_id, recipient) = build_funded_wallet_manager(2_000_000);
        let broadcaster: Arc<dyn TransactionBroadcaster> = Arc::new(FailingBroadcaster);
        let core = make_core_wallet_for_manager(wm, wallet_id, broadcaster);

        let outputs = vec![(recipient.clone(), 100_000)];

        // First call fails at the broadcast step → guard drops →
        // reservation released. The change-address index is also rolled
        // back by virtue of #3585's peek-then-commit pattern.
        let first = core
            .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs.clone())
            .await;
        assert!(
            matches!(first, Err(PlatformWalletError::TransactionBroadcast(_))),
            "first call must surface broadcast failure; got: {first:?}"
        );

        // Reservation set is now empty — verifiable through behaviour:
        // a second call sees the same UTXO as spendable again. We
        // can't broadcast successfully (broadcaster always fails) but
        // the second call must reach the broadcaster, not short-circuit
        // with `NoSpendableInputs` (which would mean the reservation
        // leaked).
        let second = core
            .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs)
            .await;
        match second {
            Err(PlatformWalletError::TransactionBroadcast(_)) => {
                // Expected — reservation released, coin selection
                // succeeded, broadcaster rejected as designed.
            }
            Err(PlatformWalletError::NoSpendableInputs { .. }) => {
                panic!(
                    "reservation leaked after broadcast failure — second \
                     call should have selected the released UTXO"
                );
            }
            other => panic!("unexpected second call result: {other:?}"),
        }
    }
}
