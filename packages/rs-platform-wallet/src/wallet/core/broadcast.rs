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
    /// Uses key-wallet's [`TransactionBuilder`] for UTXO selection, fee estimation, and signing.
    /// Change is sent to the next internal address of the specified account. Concurrent calls on
    /// the same wallet handle are race-safe via the reservation set in [`super::reservations`]:
    /// the second caller short-circuits with [`PlatformWalletError::NoSpendableInputs`] before
    /// touching the network if all UTXOs are reserved by an in-flight broadcast.
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

            let (managed_account, account) = match account_type {
                StandardAccountType::BIP44Account => (
                    info.core_wallet
                        .accounts
                        .standard_bip44_accounts
                        .get_mut(&account_index)
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "{:?} managed account {} not found",
                                account_type, account_index
                            ))
                        })?,
                    wallet
                        .accounts
                        .standard_bip44_accounts
                        .get(&account_index)
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "{:?} account {} not found in wallet",
                                account_type, account_index
                            ))
                        })?,
                ),
                StandardAccountType::BIP32Account => (
                    info.core_wallet
                        .accounts
                        .standard_bip32_accounts
                        .get_mut(&account_index)
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "{:?} managed account {} not found",
                                account_type, account_index
                            ))
                        })?,
                    wallet
                        .accounts
                        .standard_bip32_accounts
                        .get(&account_index)
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "{:?} account {} not found in wallet",
                                account_type, account_index
                            ))
                        })?,
                ),
            };

            let xpub = account.account_xpub;

            // Snapshot spendable UTXOs minus any in-flight reservations from
            // a concurrent `send_to_addresses` on this handle. Single lock
            // acquisition for the whole filter pass.
            let reserved = self.reservations.snapshot();
            let spendable: Vec<_> = managed_account
                .spendable_utxos(current_height)
                .into_iter()
                .filter(|utxo| !reserved.contains(&utxo.outpoint))
                .cloned()
                .collect();

            if spendable.is_empty() {
                return Err(PlatformWalletError::NoSpendableInputs {
                    account_index,
                    account_type,
                    context: "all UTXOs used or reserved by in-flight transactions".to_string(),
                });
            }

            // Peek at the next change address without advancing the derivation
            // index. We commit the advance only after post-build revalidation
            // succeeds, so a revalidation failure does not burn an index and
            // widen the gap-limit window on retry.
            let change_addr = managed_account
                .next_change_address(Some(&xpub), false)
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            let mut builder = TransactionBuilder::new()
                .set_current_height(current_height)
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_change_address(change_addr)
                .add_inputs(spendable.iter().cloned());
            for (addr, amount) in &outputs {
                builder = builder.add_output(addr, *amount);
            }

            let (tx, _fee) = builder
                .build_signed(wallet, |addr| {
                    managed_account.address_derivation_path(&addr)
                })
                .await
                .map_err(|e| {
                    // Map coin-selection failures to `NoSpendableInputs`. The string-match is
                    // brittle against upstream rephrasing and is currently unpinned by tests.
                    // TODO(typed-wrapper): drop once upstream exposes `SelectionError` typed via BuilderError.
                    let msg = e.to_string();
                    if msg.contains("Insufficient funds") || msg.contains("No UTXOs available") {
                        PlatformWalletError::NoSpendableInputs {
                            account_type,
                            account_index,
                            context: msg,
                        }
                    } else {
                        PlatformWalletError::TransactionBuild(msg)
                    }
                })?;

            // Defense-in-depth: unreachable under normal builder contract but guards against
            // a future regression where coin selection picks an outpoint outside `spendable`.
            let selected: BTreeSet<OutPoint> =
                tx.input.iter().map(|txin| txin.previous_output).collect();
            let spendable_outpoints: BTreeSet<OutPoint> =
                spendable.iter().map(|utxo| utxo.outpoint).collect();
            if !selected.is_subset(&spendable_outpoints) {
                // Typed retryable variant: forward-compatible with cross-process
                // concurrent-spend surfacing; today only a builder regression hits it.
                return Err(PlatformWalletError::ConcurrentSpendConflict {
                    selected: selected.into_iter().collect(),
                });
            }

            // Reserve before releasing the lock so the next caller sees these outpoints
            // filtered out. Guard held until `check_core_transaction` marks them spent
            // (success) or the error unwinds (failure → outpoints released for retry).
            let reservation = self.reservations.reserve(selected.into_iter().collect());

            (tx, xpub, reservation)
        };

        // Broadcast first — on error we leave wallet state untouched so the caller can retry.
        // If the network accepted but the call errored (ambiguous outcome), a retry will be
        // rejected as a duplicate spend rather than us marking UTXOs spent prematurely.
        self.broadcast_transaction(&tx).await?;

        // Mark inputs spent under the write lock, transitioning them from "reserved" to "spent"
        // before the reservation guard drops — no observable gap for concurrent callers.
        // Warning paths below do NOT return Err: the network already accepted the tx.
        {
            let mut wm = self.wallet_manager.write().await;
            if let Some((wallet, info)) = wm.get_wallet_mut_and_info_mut(&self.wallet_id) {
                // Commit the change-address advance post-broadcast; doing it before would burn
                // a derivation index on network rejection, widening the gap-limit window.
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
                    // Own-built tx unrecognised by our checker is an internal invariant
                    // violation, not a transient. Stable event field for operator alerting.
                    tracing::error!(
                        target: "platform_wallet::broadcast",
                        event = "post_broadcast_unrelated_to_own_wallet",
                        txid = %tx.txid(),
                        wallet_id = %hex::encode(self.wallet_id),
                        "Internal invariant violation: own-built broadcast not recognized by post-broadcast check"
                    );
                }
            } else {
                // Log-only: broadcast already succeeded; the wallet handle is stale and
                // future sends will surface a clean `WalletNotFound` from the lookup above.
                tracing::warn!(
                    target: "platform_wallet::broadcast",
                    event = "post_broadcast_wallet_missing",
                    wallet_id = %hex::encode(self.wallet_id),
                    txid = %tx.txid(),
                    "wallet missing during post-broadcast transaction registration"
                );
            }
        }

        // Explicit drop: inputs are already marked spent above; no gap between
        // "reservation released" and "spent visible" to concurrent callers.
        drop(_reservation);

        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    //! Broadcast and `send_to_addresses` contracts.
    //!
    //! Pins:
    //! - `broadcast_transaction` forwards the broadcaster's `Ok`/`Err` unchanged.
    //! - Concurrent `send_to_addresses` on the same wallet handle resolves via
    //!   the reservation set: the loser short-circuits with `NoSpendableInputs`
    //!   before reaching the broadcaster.
    //! - A broadcast failure releases the reservation so a retry sees the same
    //!   UTXO as spendable again.
    //! - An empty spendable snapshot (e.g. all UTXOs reserved) maps to
    //!   `NoSpendableInputs` via the early-exit guard.
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

    // Race-closing tests: same-UTXO concurrent `send_to_addresses`.
    // B must short-circuit with `NoSpendableInputs` before the network — a `TransactionBroadcast`
    // failure from B would mean the bug is still open.

    use std::collections::BTreeMap;

    use dashcore::hashes::Hash;
    use dashcore::{Address as DashAddress, OutPoint, TxOut};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::Wallet;
    use key_wallet::Utxo;
    use tokio::sync::Notify;

    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

    /// Mock broadcaster that gates the broadcast on an external `Notify`.
    /// `entered` fires the moment `broadcast()` is awaited — by then the
    /// caller has reserved its outpoints and dropped the wallet write lock.
    struct GatedBroadcaster {
        gate: Arc<Notify>,
        entered: Arc<Notify>,
        calls: AtomicUsize,
        succeed: bool,
    }

    #[async_trait]
    impl TransactionBroadcaster for GatedBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, PlatformWalletError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.entered.notify_one();
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

        // Height must be well past UTXO height: `select_coins_with_size` enforces
        // `min_confirmations >= 1`, which requires synced_height > utxo_height.
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

    /// Two concurrent `send_to_addresses` calls on one wallet with one UTXO must yield exactly
    /// one broadcast. The loser must get [`PlatformWalletError::NoSpendableInputs`] — never
    /// `TransactionBroadcast` (that would mean it reached the network, which is the bug closed).
    #[tokio::test]
    async fn concurrent_same_utxo_sends_resolve_via_reservation_set() {
        use key_wallet::account::account_type::StandardAccountType;

        let (wm, wallet_id, recipient) = build_funded_wallet_manager(2_000_000);
        let gate = Arc::new(Notify::new());
        let entered = Arc::new(Notify::new());
        let broadcaster = Arc::new(GatedBroadcaster {
            gate: Arc::clone(&gate),
            entered: Arc::clone(&entered),
            calls: AtomicUsize::new(0),
            succeed: true,
        });
        let core = make_core_wallet_for_manager(
            wm,
            wallet_id,
            Arc::clone(&broadcaster) as Arc<dyn TransactionBroadcaster>,
        );

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

        // Deterministic handshake: wait until A has reached the broadcast gate.
        // By that point A has reserved the outpoint and dropped the wallet write lock.
        entered.notified().await;

        // Caller B starts now. The wallet's only UTXO is reserved by A,
        // so B's spendable snapshot is empty → `NoSpendableInputs`.
        let b_result = core
            .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs_b)
            .await;

        match &b_result {
            Err(PlatformWalletError::NoSpendableInputs { context, .. }) => {
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

        // Pin "loser never reached the network" directly: only A invoked the broadcaster.
        assert_eq!(
            broadcaster.calls.load(Ordering::SeqCst),
            1,
            "broadcaster must be called exactly once across both concurrent senders"
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

        // Reservation released: the second call must reach the broadcaster (same UTXO visible),
        // not short-circuit with `NoSpendableInputs` (which would indicate a leaked reservation).
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

    /// Pins the early-exit guard: when the spendable snapshot is empty
    /// (e.g. all UTXOs reserved by in-flight broadcasts), `send_to_addresses`
    /// surfaces `NoSpendableInputs` without invoking the builder.
    ///
    /// Note: the upstream coin-selection string-match in `send_to_addresses`
    /// is not exercised here — that path is currently unpinned.
    #[tokio::test]
    async fn builder_error_text_contract_for_no_inputs() {
        use key_wallet::account::account_type::StandardAccountType;

        let (wm, wallet_id, recipient) = build_funded_wallet_manager(2_000_000);
        let broadcaster: Arc<dyn TransactionBroadcaster> = Arc::new(FailingBroadcaster);
        let core = make_core_wallet_for_manager(wm, wallet_id, broadcaster);

        let outputs = vec![(recipient.clone(), 100_000)];

        // Reserve the wallet's only outpoint so the spendable snapshot is
        // empty for the next caller, exercising the early-exit guard.
        let outpoint = OutPoint::new(Txid::from_byte_array([7u8; 32]), 0);
        let _guard = core.reservations.reserve(vec![outpoint]);

        let result = core
            .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs)
            .await;

        assert!(
            matches!(result, Err(PlatformWalletError::NoSpendableInputs { .. })),
            "send_to_addresses must map a fully-reserved wallet to NoSpendableInputs; got: {result:?}"
        );
    }
}
