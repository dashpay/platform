use std::collections::BTreeSet;
use std::sync::Arc;

use dashcore::{Address as DashAddress, OutPoint, Transaction, Txid};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::Signer;
use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionError;
use key_wallet::wallet::managed_wallet_info::transaction_builder::BuilderError;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet_manager::WalletManager;
use tokio::sync::RwLock;

use super::reservations::OutpointReservationGuard;
use crate::broadcaster::TransactionBroadcaster;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::{CoreWallet, PlatformWalletError};

/// Map a key-wallet [`BuilderError`] from `TransactionBuilder::build_signed` into a
/// [`PlatformWalletError`], distinguishing depleted-UTXO conditions (retryable
/// [`PlatformWalletError::NoSpendableInputs`]) from every other build failure
/// (fatal [`PlatformWalletError::TransactionBuild`]).
///
/// The variants treated as "no spendable inputs" are:
///
/// * [`BuilderError::InsufficientFunds`] — the builder's own pre-selection
///   shortfall check.
/// * [`BuilderError::CoinSelection`] wrapping either
///   [`SelectionError::NoUtxosAvailable`] or
///   [`SelectionError::InsufficientFunds`] — the coin-selector's two
///   depleted-UTXO outcomes.
///
/// `account_type` and `account_index` are threaded through unchanged so the
/// resulting `NoSpendableInputs` carries the same operator-facing context the
/// upstream call sites already populate.
pub(crate) fn classify_build_error(
    err: BuilderError,
    account_type: StandardAccountType,
    account_index: u32,
) -> PlatformWalletError {
    let context = err.to_string();
    match err {
        BuilderError::InsufficientFunds { .. }
        | BuilderError::CoinSelection(SelectionError::NoUtxosAvailable)
        | BuilderError::CoinSelection(SelectionError::InsufficientFunds { .. }) => {
            PlatformWalletError::NoSpendableInputs {
                account_type,
                account_index,
                context,
            }
        }
        _ => PlatformWalletError::TransactionBuild(context),
    }
}

/// Post-`build_signed` defense-in-depth: re-snapshot the funding account's
/// spendable UTXOs and confirm every outpoint the builder `selected` is
/// still present. Returns [`PlatformWalletError::ConcurrentSpendConflict`]
/// listing the missing outpoints otherwise.
///
/// Today every UTXO mutator goes through the wallet write lock the send
/// flow holds across build, so this is unreachable — but a future mutator
/// running outside that lock (mempool listener, chain reorg, etc.) would
/// slip through the pre-build spendable snapshot; this fresh re-fetch
/// catches it before broadcast. The reservations guard remains the primary
/// in-process race defense; this is the cross-process / cross-subsystem net.
///
/// Both `selected` and `fresh_spendable` are caller-computed outpoint sets
/// so this stays a pure set check, independent of the managed-account type.
pub(crate) fn assert_selected_still_spendable(
    selected: &BTreeSet<OutPoint>,
    fresh_spendable: &BTreeSet<OutPoint>,
) -> Result<(), PlatformWalletError> {
    if selected.is_subset(fresh_spendable) {
        return Ok(());
    }
    let missing: Vec<OutPoint> = selected.difference(fresh_spendable).copied().collect();
    Err(PlatformWalletError::ConcurrentSpendConflict { selected: missing })
}

/// Shared send-flow tail used by [`CoreWallet::send_to_addresses`] and
/// `IdentityWallet::send_payment`.
///
/// Broadcasts `tx`, then under a single write lock acquisition reconciles
/// wallet state and either releases or leaks the reservation guard
/// according to the post-broadcast invariant:
///
/// * On broadcast failure the reservation is implicitly dropped — the
///   guard is moved into this function and unwinds via `Drop` when the
///   early `?` returns, releasing the outpoints so the caller can retry.
/// * On broadcast success, `check_core_transaction` is run inside the
///   write lock. If `is_relevant == true` the reservation is released via
///   [`OutpointReservationGuard::release_after_commit`]; otherwise it is
///   leaked via [`OutpointReservationGuard::leak_until_sync`] to prevent a
///   concurrent caller from re-selecting an already-broadcast outpoint.
/// * `on_reconcile` is invoked inside the write lock *after*
///   `check_core_transaction` so call-site-specific bookkeeping (e.g.
///   `IdentityWallet::send_payment` recording a `PaymentEntry`) sees the
///   same locked wallet state and runs in the same critical section.
///
/// Returns the broadcasted [`Txid`].
pub(crate) async fn broadcast_and_reconcile<B, F>(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: WalletId,
    broadcaster: &Arc<B>,
    tx: &Transaction,
    reservation: OutpointReservationGuard,
    on_reconcile: F,
) -> Result<Txid, PlatformWalletError>
where
    B: TransactionBroadcaster + ?Sized,
    F: FnOnce(&mut PlatformWalletInfo, &Txid, bool),
{
    // Broadcast first — on error the `reservation` guard is dropped via
    // `?` unwind, releasing the outpoints so the caller can retry. If the
    // network accepted but the call errored (ambiguous outcome), a retry
    // will be rejected as a duplicate spend rather than us marking UTXOs
    // spent prematurely.
    let txid = broadcaster.broadcast(tx).await?;

    // Mark inputs spent under the write lock, transitioning them from
    // "reserved" to "spent" before the reservation guard drops. The guard
    // is consumed below by either `release_after_commit` (success) or
    // `leak_until_sync` (post-broadcast reconcile failure).
    let mut reconciled = false;
    {
        let mut wm = wallet_manager.write().await;
        if let Some((wallet, info)) = wm.get_wallet_mut_and_info_mut(&wallet_id) {
            let check_result = info
                .check_core_transaction(tx, TransactionContext::Mempool, wallet, true, true)
                .await;
            reconciled = check_result.is_relevant;
            if !reconciled {
                tracing::error!(
                    target: "platform_wallet::broadcast",
                    event = "post_broadcast_unrelated_to_own_wallet",
                    txid = %txid,
                    wallet_id = %hex::encode(wallet_id),
                    "Internal invariant violation: own-built broadcast not recognized by post-broadcast check"
                );
            }
            on_reconcile(info, &txid, reconciled);
        } else {
            tracing::warn!(
                target: "platform_wallet::broadcast",
                event = "post_broadcast_wallet_missing",
                wallet_id = %hex::encode(wallet_id),
                %txid,
                "wallet missing during post-broadcast transaction registration"
            );
        }
    }

    if reconciled {
        // Inputs are now marked spent; safe to release reservation.
        reservation.release_after_commit();
    } else {
        // Broadcast succeeded but state could not be reconciled. Releasing
        // the reservation now risks a concurrent send re-selecting the
        // same UTXO and producing a double-spend the network would
        // reject. Keep it held until process restart — see
        // `OutpointReservationGuard::leak_until_sync`.
        tracing::warn!(
            target: "platform_wallet::broadcast",
            event = "post_broadcast_reservation_leaked_until_sync",
            %txid,
            wallet_id = %hex::encode(wallet_id),
            leaked_reservations = reservation.reserved_count(),
            "leaking outpoint reservation: post-broadcast reconciliation failed"
        );
        reservation.leak_until_sync();
    }

    Ok(txid)
}

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
    /// Uses key-wallet's
    /// [`TransactionBuilder`](key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder)
    /// for UTXO selection, fee estimation, and signing. Change is sent to
    /// the next internal address of the specified account.
    ///
    /// Concurrent calls on the same wallet handle are race-safe via the
    /// reservation set in [`super::reservations`]: the second caller
    /// short-circuits with [`PlatformWalletError::NoSpendableInputs`]
    /// before touching the network if all UTXOs are reserved by an
    /// in-flight broadcast.
    ///
    /// Signing is delegated to the caller-supplied
    /// [`Signer`](key_wallet::signer::Signer) via the
    /// `impl<S: Signer> TransactionSigner for S` blanket in
    /// `key-wallet`'s `transaction_builder.rs`. For Swift wallets this
    /// is typically a `MnemonicResolverCoreSigner` from
    /// `platform-wallet-ffi`, backed by the Keychain-resolver vtable so
    /// private keys never cross the FFI boundary.
    pub async fn send_to_addresses<S: Signer>(
        &self,
        account_type: StandardAccountType,
        account_index: u32,
        outputs: Vec<(DashAddress, u64)>,
        signer: &S,
    ) -> Result<Transaction, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;

        if outputs.is_empty() {
            return Err(PlatformWalletError::TransactionBuild(
                "No outputs specified".to_string(),
            ));
        }

        let (tx, _reservation) = {
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

            // Pick a change address no concurrent in-flight send has peeked,
            // committing the index advance under this write lock. Recorded
            // into `pending_change` via the outpoint reservation below so a
            // concurrent caller can never select the same change address.
            let change_addr = super::change_address::pick_and_reserve_change_address(
                &self.reservations,
                managed_account,
                &xpub,
            )?;

            let mut builder = TransactionBuilder::new()
                .set_current_height(current_height)
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_change_address(change_addr.clone())
                .add_inputs(spendable.iter().cloned());
            for (addr, amount) in &outputs {
                builder = builder.add_output(addr, *amount);
            }

            let (tx, _fee) = builder
                .build_signed(signer, |addr| {
                    managed_account.address_derivation_path(&addr)
                })
                .await
                .map_err(|e| classify_build_error(e, account_type, account_index))?;

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

            // Defense-in-depth: re-snapshot spendable UTXOs after
            // `build_signed` and confirm every selected outpoint is still
            // present. See [`assert_selected_still_spendable`] for why.
            let fresh_spendable_outpoints: BTreeSet<OutPoint> = managed_account
                .spendable_utxos(current_height)
                .into_iter()
                .map(|utxo| utxo.outpoint)
                .collect();
            assert_selected_still_spendable(&selected, &fresh_spendable_outpoints)?;

            // Reserve before releasing the lock so the next caller sees these outpoints
            // filtered out *and* skips the peeked change address. Guard held until
            // `check_core_transaction` marks them spent (success) or the error
            // unwinds (failure → outpoints released for retry).
            let reservation = self
                .reservations
                .reserve(selected.into_iter().collect(), Some(change_addr.clone()));

            (tx, reservation)
        };

        broadcast_and_reconcile(
            &self.wallet_manager,
            self.wallet_id,
            &self.broadcaster,
            &tx,
            _reservation,
            |_info, _txid, _reconciled| {
                // No path-specific post-reconcile bookkeeping for the
                // raw send-to-addresses flow.
            },
        )
        .await?;

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

    /// In-memory `Signer` for tests — mirrors the one in key-wallet's own
    /// transaction-building tests. Backed by the mnemonic wallet's root
    /// extended private key, so derivation paths from the funded account
    /// resolve to the keys that actually own the seeded UTXO.
    struct InMemorySigner {
        root: key_wallet::wallet::root_extended_keys::RootExtendedPrivKey,
        network: Network,
    }

    const IN_MEMORY_METHODS: &[key_wallet::signer::SignerMethod] =
        &[key_wallet::signer::SignerMethod::Digest];

    #[async_trait]
    impl key_wallet::signer::Signer for InMemorySigner {
        type Error = String;

        fn supported_methods(&self) -> &[key_wallet::signer::SignerMethod] {
            IN_MEMORY_METHODS
        }

        async fn sign_ecdsa(
            &self,
            path: &key_wallet::bip32::DerivationPath,
            sighash: [u8; 32],
        ) -> Result<
            (
                dashcore::secp256k1::ecdsa::Signature,
                dashcore::secp256k1::PublicKey,
            ),
            Self::Error,
        > {
            let secp = dashcore::secp256k1::Secp256k1::new();
            let xpriv = self
                .root
                .to_extended_priv_key(self.network)
                .derive_priv(&secp, path)
                .map_err(|e| e.to_string())?;
            let msg = dashcore::secp256k1::Message::from_digest(sighash);
            let sig = secp.sign_ecdsa(&msg, &xpriv.private_key);
            let pk = dashcore::secp256k1::PublicKey::from_secret_key(&secp, &xpriv.private_key);
            Ok((sig, pk))
        }

        async fn public_key(
            &self,
            path: &key_wallet::bip32::DerivationPath,
        ) -> Result<dashcore::secp256k1::PublicKey, Self::Error> {
            let secp = dashcore::secp256k1::Secp256k1::new();
            let xpriv = self
                .root
                .to_extended_priv_key(self.network)
                .derive_priv(&secp, path)
                .map_err(|e| e.to_string())?;
            Ok(dashcore::secp256k1::PublicKey::from_secret_key(
                &secp,
                &xpriv.private_key,
            ))
        }
    }

    fn root_from(
        wallet: &key_wallet::wallet::Wallet,
    ) -> key_wallet::wallet::root_extended_keys::RootExtendedPrivKey {
        match &wallet.wallet_type {
            key_wallet::wallet::WalletType::Mnemonic {
                root_extended_private_key,
                ..
            } => root_extended_private_key.clone(),
            _ => unreachable!("test wallets are mnemonic"),
        }
    }

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
    /// wallet id, a recipient address (a separate derived address in the
    /// same account — funding/sending to the same address is not the
    /// property under test), and a signer backed by the wallet's
    /// mnemonic root so `build_signed` can sign the seeded UTXO.
    fn build_funded_wallet_manager(
        utxo_value: u64,
    ) -> (
        Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        crate::wallet::platform_wallet::WalletId,
        DashAddress,
        InMemorySigner,
    ) {
        let wallet = Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default)
            .expect("test wallet");
        let signer = InMemorySigner {
            root: root_from(&wallet),
            network: Network::Testnet,
        };

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

        (Arc::new(RwLock::new(wm)), wallet_id, recipient, signer)
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

    /// CW-002: a standalone change-address hand-out must not return a
    /// change address that an in-flight `send_to_addresses` already peeked
    /// into `pending_change`. The avoid-loop in `reserve_bip44_address`
    /// skips it and hands out the next index instead.
    #[tokio::test]
    async fn next_change_address_skips_pending_change_reservation() {
        let (wm, wallet_id, _recipient, _signer) = build_funded_wallet_manager(2_000_000);
        let cw =
            make_core_wallet_for_manager(Arc::clone(&wm), wallet_id, Arc::new(FailingBroadcaster));

        // Peek (advance=false) the change address the bridge would pick
        // first, without committing the index or bridge-reserving it.
        let pending = {
            let mut guard = wm.write().await;
            let (wallet, info) = guard.get_wallet_and_info_mut(&wallet_id).unwrap();
            let xpub = wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .unwrap()
                .account_xpub;
            let managed = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .unwrap();
            managed.next_change_address(Some(&xpub), false).unwrap()
        };

        // Mark it pending, as the broadcast loop does before a send confirms.
        let _guard = cw.reservations.reserve(vec![], Some(pending.clone()));
        assert!(cw.reservations.change_address_pending(&pending));

        // A standalone hand-out must steer clear of the pending address.
        let handed_out = cw.next_change_address_for_account(0).await.unwrap();
        assert_ne!(
            handed_out, pending,
            "change hand-out returned an address already pending from an in-flight send"
        );
    }

    /// Two concurrent `send_to_addresses` calls on one wallet with one UTXO must yield exactly
    /// one broadcast. The loser must get [`PlatformWalletError::NoSpendableInputs`] — never
    /// `TransactionBroadcast` (that would mean it reached the network, which is the bug closed).
    #[tokio::test]
    async fn concurrent_same_utxo_sends_resolve_via_reservation_set() {
        use key_wallet::account::account_type::StandardAccountType;

        let (wm, wallet_id, recipient, signer) = build_funded_wallet_manager(2_000_000);
        let signer = Arc::new(signer);
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
        let signer_a = Arc::clone(&signer);
        let a_handle = tokio::spawn(async move {
            core_a
                .send_to_addresses(
                    StandardAccountType::BIP44Account,
                    0,
                    outputs_a,
                    signer_a.as_ref(),
                )
                .await
        });

        // Deterministic handshake: wait until A has reached the broadcast gate.
        // By that point A has reserved the outpoint and dropped the wallet write lock.
        entered.notified().await;

        // Caller B starts now. The wallet's only UTXO is reserved by A,
        // so B's spendable snapshot is empty → `NoSpendableInputs`.
        let b_result = core
            .send_to_addresses(
                StandardAccountType::BIP44Account,
                0,
                outputs_b,
                signer.as_ref(),
            )
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

        let (wm, wallet_id, recipient, signer) = build_funded_wallet_manager(2_000_000);
        let broadcaster: Arc<dyn TransactionBroadcaster> = Arc::new(FailingBroadcaster);
        let core = make_core_wallet_for_manager(wm, wallet_id, broadcaster);

        let outputs = vec![(recipient.clone(), 100_000)];

        // First call fails at the broadcast step → guard drops →
        // reservation released. The change-address index is also rolled
        // back by virtue of #3585's peek-then-commit pattern.
        let first = core
            .send_to_addresses(
                StandardAccountType::BIP44Account,
                0,
                outputs.clone(),
                &signer,
            )
            .await;
        assert!(
            matches!(first, Err(PlatformWalletError::TransactionBroadcast(_))),
            "first call must surface broadcast failure; got: {first:?}"
        );

        // Reservation released: the second call must reach the broadcaster (same UTXO visible),
        // not short-circuit with `NoSpendableInputs` (which would indicate a leaked reservation).
        let second = core
            .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs, &signer)
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

    /// If `check_core_transaction` returns `is_relevant = false`
    /// after a successful broadcast (an internal invariant violation but a
    /// real-world possibility on a corrupted/stale wallet state), the
    /// reservation must stay held — releasing it could let a concurrent
    /// caller select the same already-broadcast outpoint and produce a
    /// double-spend the network would reject.
    ///
    /// We force `is_relevant = false` by clearing the funding account's
    /// address-pool entries between the broadcast and the reconcile step;
    /// the post-broadcast `check_core_transaction` then matches nothing.
    #[tokio::test]
    async fn unreconciled_broadcast_keeps_reservation_held() {
        use key_wallet::account::account_type::StandardAccountType;

        let (wm, wallet_id, recipient, signer) = build_funded_wallet_manager(2_000_000);
        let signer = Arc::new(signer);

        let gate = Arc::new(Notify::new());
        let entered = Arc::new(Notify::new());
        let broadcaster = Arc::new(GatedBroadcaster {
            gate: Arc::clone(&gate),
            entered: Arc::clone(&entered),
            calls: AtomicUsize::new(0),
            succeed: true,
        });

        let core = make_core_wallet_for_manager(
            Arc::clone(&wm),
            wallet_id,
            Arc::clone(&broadcaster) as Arc<dyn TransactionBroadcaster>,
        );

        let outputs = vec![(recipient.clone(), 100_000)];

        let core_send = core.clone();
        let signer_send = Arc::clone(&signer);
        let handle = tokio::spawn(async move {
            core_send
                .send_to_addresses(
                    StandardAccountType::BIP44Account,
                    0,
                    outputs,
                    signer_send.as_ref(),
                )
                .await
        });

        // Wait until the broadcast call has been entered — by then the
        // outpoint is reserved, the change index is committed, and the
        // wallet write lock has been released.
        entered.notified().await;

        // Sabotage the wallet so `check_core_transaction` cannot match
        // anything. Clearing the account-collection map entirely is the
        // surest way: with no accounts the checker walks zero entries.
        {
            let mut wm_guard = wm.write().await;
            let info = wm_guard.get_wallet_info_mut(&wallet_id).expect("info");
            info.core_wallet.accounts.standard_bip44_accounts.clear();
        }

        // Capture the reservation's outpoint *before* releasing the gate
        // so the assertion target is stable.
        let funding_outpoint = OutPoint::new(Txid::from_byte_array([7u8; 32]), 0);

        // Release the broadcast — the post-broadcast reconcile sees
        // `is_relevant=false` and leaks the reservation.
        gate.notify_one();

        let result = handle.await.expect("task panicked");
        assert!(
            result.is_ok(),
            "send_to_addresses must succeed when broadcast succeeds; got: {result:?}"
        );

        assert!(
            core.reservations.contains(&funding_outpoint),
            "reservation must remain held when post-broadcast reconcile fails (is_relevant=false), \
             otherwise a concurrent caller could re-select the same already-broadcast outpoint"
        );
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

        let (wm, wallet_id, recipient, signer) = build_funded_wallet_manager(2_000_000);
        let broadcaster: Arc<dyn TransactionBroadcaster> = Arc::new(FailingBroadcaster);
        let core = make_core_wallet_for_manager(wm, wallet_id, broadcaster);

        let outputs = vec![(recipient.clone(), 100_000)];

        // Reserve the wallet's only outpoint so the spendable snapshot is
        // empty for the next caller, exercising the early-exit guard.
        let outpoint = OutPoint::new(Txid::from_byte_array([7u8; 32]), 0);
        let _guard = core.reservations.reserve(vec![outpoint], None);

        let result = core
            .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs, &signer)
            .await;

        assert!(
            matches!(result, Err(PlatformWalletError::NoSpendableInputs { .. })),
            "send_to_addresses must map a fully-reserved wallet to NoSpendableInputs; got: {result:?}"
        );
    }

    // ---- classify_build_error: typed BuilderError → PlatformWalletError ----
    //
    // These tests pin the typed contract directly so a future rename or
    // rewording of the upstream `Display` impl cannot silently downgrade
    // `NoSpendableInputs` back to `TransactionBuild`.

    use super::classify_build_error;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionError;
    use key_wallet::wallet::managed_wallet_info::transaction_builder::BuilderError;

    #[test]
    fn classify_builder_insufficient_funds_maps_to_no_spendable_inputs() {
        let err = BuilderError::InsufficientFunds {
            available: 1_000,
            required: 10_000,
        };
        let mapped = classify_build_error(err, StandardAccountType::BIP44Account, 7);
        match mapped {
            PlatformWalletError::NoSpendableInputs {
                account_type,
                account_index,
                context,
            } => {
                assert!(matches!(account_type, StandardAccountType::BIP44Account));
                assert_eq!(account_index, 7);
                assert!(
                    context.contains("Insufficient funds"),
                    "context should preserve the upstream Display; got: {context}"
                );
            }
            other => panic!("expected NoSpendableInputs, got: {other:?}"),
        }
    }

    #[test]
    fn classify_coin_selection_no_utxos_available_maps_to_no_spendable_inputs() {
        let err = BuilderError::CoinSelection(SelectionError::NoUtxosAvailable);
        let mapped = classify_build_error(err, StandardAccountType::BIP32Account, 3);
        match mapped {
            PlatformWalletError::NoSpendableInputs {
                account_type,
                account_index,
                ..
            } => {
                assert!(matches!(account_type, StandardAccountType::BIP32Account));
                assert_eq!(account_index, 3);
            }
            other => panic!("expected NoSpendableInputs, got: {other:?}"),
        }
    }

    #[test]
    fn classify_coin_selection_insufficient_funds_maps_to_no_spendable_inputs() {
        let err = BuilderError::CoinSelection(SelectionError::InsufficientFunds {
            available: 5,
            required: 100,
        });
        let mapped = classify_build_error(err, StandardAccountType::BIP44Account, 0);
        assert!(
            matches!(mapped, PlatformWalletError::NoSpendableInputs { .. }),
            "CoinSelection(InsufficientFunds) must map to NoSpendableInputs; got: {mapped:?}"
        );
    }

    #[test]
    fn classify_no_inputs_falls_through_to_transaction_build() {
        let err = BuilderError::NoInputs;
        let mapped = classify_build_error(err, StandardAccountType::BIP44Account, 0);
        assert!(
            matches!(mapped, PlatformWalletError::TransactionBuild(_)),
            "non-depleted BuilderError must fall through to TransactionBuild; got: {mapped:?}"
        );
    }

    #[test]
    fn classify_signing_failed_falls_through_to_transaction_build() {
        let err = BuilderError::SigningFailed("bad signer".to_string());
        let mapped = classify_build_error(err, StandardAccountType::BIP44Account, 0);
        match mapped {
            PlatformWalletError::TransactionBuild(msg) => {
                assert!(
                    msg.contains("bad signer"),
                    "context should preserve message"
                );
            }
            other => panic!("expected TransactionBuild, got: {other:?}"),
        }
    }

    #[test]
    fn classify_coin_selection_failed_falls_through_to_transaction_build() {
        let err =
            BuilderError::CoinSelection(SelectionError::SelectionFailed("wraparound".to_string()));
        let mapped = classify_build_error(err, StandardAccountType::BIP44Account, 0);
        assert!(
            matches!(mapped, PlatformWalletError::TransactionBuild(_)),
            "CoinSelection(SelectionFailed) is not a depleted-UTXO outcome; \
             must fall through to TransactionBuild; got: {mapped:?}"
        );
    }

    // ---- assert_selected_still_spendable: post-build fresh-spendable net ----

    use std::collections::BTreeSet;

    use super::assert_selected_still_spendable;

    fn op(byte: u8, vout: u32) -> OutPoint {
        OutPoint::new(Txid::from_byte_array([byte; 32]), vout)
    }

    #[test]
    fn selected_subset_of_fresh_spendable_is_ok() {
        let selected: BTreeSet<OutPoint> = [op(1, 0), op(2, 0)].into_iter().collect();
        let fresh: BTreeSet<OutPoint> = [op(1, 0), op(2, 0), op(3, 0)].into_iter().collect();
        assert!(
            assert_selected_still_spendable(&selected, &fresh).is_ok(),
            "a selected set fully contained in the fresh spendable set must pass"
        );
    }

    #[test]
    fn selected_missing_from_fresh_spendable_reports_concurrent_spend_conflict() {
        let gone = op(2, 0);
        let selected: BTreeSet<OutPoint> = [op(1, 0), gone].into_iter().collect();
        // `gone` disappeared between build and the post-build re-snapshot.
        let fresh: BTreeSet<OutPoint> = [op(1, 0), op(3, 0)].into_iter().collect();

        match assert_selected_still_spendable(&selected, &fresh) {
            Err(PlatformWalletError::ConcurrentSpendConflict { selected: missing }) => {
                assert_eq!(
                    missing,
                    vec![gone],
                    "the conflict must list exactly the outpoint that vanished from the fresh set"
                );
            }
            other => panic!(
                "expected ConcurrentSpendConflict listing the missing outpoint; got: {other:?}"
            ),
        }
    }

    // ---- broadcast_and_reconcile: shared post-broadcast helper ----
    //
    // The helper centralises the broadcast → reconcile → release-or-leak
    // decision tree used by both `CoreWallet::send_to_addresses` and
    // `IdentityWallet::send_payment`. These tests pin the helper's three
    // invariants directly so the shared path stays honest:
    //
    // 1. Broadcast failure unwinds the reservation (caller can retry).
    // 2. The `on_reconcile` hook is only invoked when the wallet lookup
    //    succeeded, and is passed the same `is_relevant` flag the helper
    //    used to decide release-vs-leak.
    // 3. When the wallet is missing post-broadcast, the hook is NOT
    //    invoked and the reservation is leaked (no double-spend risk).

    use std::sync::atomic::AtomicBool;

    use super::broadcast_and_reconcile;

    /// On broadcast failure, the reservation guard moved into the helper
    /// must be dropped via early-return, releasing the outpoints so the
    /// caller can retry. The `on_reconcile` hook must NOT fire.
    #[tokio::test]
    async fn broadcast_and_reconcile_drops_reservation_on_broadcast_failure() {
        use crate::wallet::core::reservations::OutpointReservations;
        use dashcore::hashes::Hash;
        use dashcore::{OutPoint, Txid};

        let reservations = OutpointReservations::new();
        let outpoint = OutPoint::new(Txid::from_byte_array([42u8; 32]), 0);
        let guard = reservations.reserve(vec![outpoint], None);
        assert!(reservations.contains(&outpoint));

        let broadcaster: Arc<dyn TransactionBroadcaster> = Arc::new(FailingBroadcaster);
        let wm = Arc::new(RwLock::new(WalletManager::<
            crate::wallet::platform_wallet::PlatformWalletInfo,
        >::new(Network::Testnet)));
        let tx = dummy_transaction();

        let hook_called = Arc::new(AtomicBool::new(false));
        let hook_called_inner = Arc::clone(&hook_called);

        let result = broadcast_and_reconcile(
            &wm,
            [0u8; 32],
            &broadcaster,
            &tx,
            guard,
            move |_info, _txid, _reconciled| {
                hook_called_inner.store(true, Ordering::SeqCst);
            },
        )
        .await;

        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "broadcast failure must propagate to the caller; got: {result:?}"
        );
        assert!(
            !hook_called.load(Ordering::SeqCst),
            "on_reconcile must NOT be invoked when the broadcast itself fails"
        );
        assert!(
            !reservations.contains(&outpoint),
            "broadcast failure must release the reservation so a retry can succeed"
        );
    }

    /// When the wallet is absent from the manager post-broadcast (stale
    /// handle), the hook must NOT fire and the reservation must be leaked
    /// (held until process restart) so a concurrent caller cannot
    /// re-select the same already-broadcast outpoint.
    #[tokio::test]
    async fn broadcast_and_reconcile_leaks_when_wallet_missing() {
        use crate::wallet::core::reservations::OutpointReservations;
        use dashcore::hashes::Hash;
        use dashcore::{OutPoint, Txid};

        let reservations = OutpointReservations::new();
        let outpoint = OutPoint::new(Txid::from_byte_array([99u8; 32]), 0);
        let guard = reservations.reserve(vec![outpoint], None);

        // OK-broadcasting mock so we reach the post-broadcast write lock.
        let broadcaster: Arc<dyn TransactionBroadcaster> = Arc::new(MockBroadcaster::new(
            BroadcastOutcome::Ok(dummy_transaction().txid()),
        ));
        // Empty wallet manager — `get_wallet_mut_and_info_mut` returns `None`.
        let wm = Arc::new(RwLock::new(WalletManager::<
            crate::wallet::platform_wallet::PlatformWalletInfo,
        >::new(Network::Testnet)));
        let tx = dummy_transaction();

        let hook_called = Arc::new(AtomicBool::new(false));
        let hook_called_inner = Arc::clone(&hook_called);

        let result = broadcast_and_reconcile(
            &wm,
            [0u8; 32],
            &broadcaster,
            &tx,
            guard,
            move |_info, _txid, _reconciled| {
                hook_called_inner.store(true, Ordering::SeqCst);
            },
        )
        .await
        .expect("broadcast succeeded; missing wallet is a log-only branch");
        assert_eq!(
            result,
            tx.txid(),
            "helper must surface the broadcaster's Txid"
        );
        assert!(
            !hook_called.load(Ordering::SeqCst),
            "hook must not fire when the wallet handle is stale"
        );
        assert!(
            reservations.contains(&outpoint),
            "missing-wallet post-broadcast must leak the reservation \
             (concurrent re-selection of an already-broadcast outpoint \
             would race the network)"
        );
    }

    /// End-to-end through `send_to_addresses` with the full funded
    /// fixture: `is_relevant = true` post-broadcast → hook receives
    /// `true` and the reservation is released. Pins the success path of
    /// the shared helper as exercised by the core call site.
    #[tokio::test]
    async fn send_to_addresses_success_invokes_hook_and_releases_reservation() {
        let (wm, wallet_id, recipient, signer) = build_funded_wallet_manager(2_000_000);
        let broadcaster: Arc<dyn TransactionBroadcaster> = Arc::new(MockBroadcaster::new(
            BroadcastOutcome::Ok(dummy_transaction().txid()),
        ));
        let core = make_core_wallet_for_manager(wm, wallet_id, broadcaster);

        let outpoint = OutPoint::new(Txid::from_byte_array([7u8; 32]), 0);
        let outputs = vec![(recipient.clone(), 100_000)];

        let result = core
            .send_to_addresses(StandardAccountType::BIP44Account, 0, outputs, &signer)
            .await;
        assert!(
            result.is_ok(),
            "happy-path send must succeed; got: {result:?}"
        );

        assert!(
            !core.reservations.contains(&outpoint),
            "successful send + relevant tx must release the reservation"
        );
    }
}
