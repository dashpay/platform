use std::collections::BTreeSet;

use dashcore::{Address as DashAddress, OutPoint, Transaction};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::Signer;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

use crate::broadcaster::TransactionBroadcaster;
use crate::wallet::reservations::broadcast_releasing_on_rejection;
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
        self.broadcaster
            .broadcast(transaction)
            .await
            .map_err(Into::into)
    }

    /// Build, sign, and broadcast a payment to the given addresses.
    ///
    /// Uses key-wallet's
    /// [`TransactionBuilder`](key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder)
    /// for UTXO selection, fee estimation, and signing. Change is sent to
    /// the next internal address of the specified account.
    ///
    /// Signing is delegated to the caller-supplied
    /// [`Signer`](key_wallet::signer::Signer) via the
    /// `impl<S: Signer> TransactionSigner for S` blanket in
    /// `key-wallet`'s `transaction_builder.rs`. For Swift wallets this
    /// is typically a
    /// [`MnemonicResolverCoreSigner`](crate::wallet::asset_lock::build)
    /// from `platform-wallet-ffi`, backed by the Keychain-resolver
    /// vtable so private keys never cross the FFI boundary.
    ///
    /// Concurrent calls on the same wallet handle are race-safe via the
    /// reservation set in [`super::reservations`]: the second caller
    /// short-circuits with [`PlatformWalletError::NoSpendableInputs`]
    /// before touching the network if all UTXOs are reserved by an
    /// in-flight broadcast.
    ///
    /// **Note (smell):** the body of this method is a near-duplicate of
    /// `ManagedWalletInfo::build_and_sign_transaction` in `key-wallet`
    /// (`wallet/managed_wallet_info/transaction_building.rs`).
    /// It's reimplemented here because the upstream helper is BIP-44-only,
    /// parametrizing upstream on `AccountTypePreference` so it picks
    /// `standard_bip{32,44}_accounts` would be a trivial change
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
                .build_signed(signer, |addr| {
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

            let selected: BTreeSet<OutPoint> =
                tx.input.iter().map(|txin| txin.previous_output).collect();

            // Reserve before releasing the lock so the next caller sees these outpoints
            // filtered out. Guard held until `check_core_transaction` marks them spent
            // (success) or the error unwinds (failure → outpoints released for retry).
            let reservation = self.reservations.reserve(selected.into_iter().collect());

            (tx, reservation)
        };

        broadcast_releasing_on_rejection(
            self.broadcaster.as_ref(),
            &self.wallet_manager,
            &self.wallet_id,
            account_type,
            account_index,
            &tx,
        )
        .await?;
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dashcore::{Address as DashAddress, Network};
    use key_wallet::account::account_type::StandardAccountType;

    use crate::broadcaster::TransactionBroadcaster;
    use crate::test_support::{
        funded_wallet_manager, AlwaysMaybeSentBroadcaster, RejectFirstBroadcaster, WalletSigner,
    };
    use crate::wallet::core::CoreWallet;
    use crate::PlatformWalletError;

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

    /// A pre-send broadcast failure must release the UTXO reservation taken while
    /// building the transaction, so an immediate retry can reselect those inputs
    /// instead of failing with spurious insufficient funds until the TTL backstop.
    /// Covers both funds-account arms of the release path.
    #[tokio::test]
    async fn send_to_addresses_releases_reservation_on_broadcast_failure() {
        for account_type in [
            StandardAccountType::BIP44Account,
            StandardAccountType::BIP32Account,
        ] {
            let broadcaster = Arc::new(RejectFirstBroadcaster::new());
            let (core, signer, outputs) = funded_core_wallet(account_type, broadcaster).await;

            // First attempt: build + sign succeed, broadcast fails.
            let first = core
                .send_to_addresses(account_type, 0, outputs.clone(), &signer)
                .await;
            assert!(
                matches!(first, Err(PlatformWalletError::TransactionBroadcast(_))),
                "first send should surface the broadcast failure for {account_type:?}, got {first:?}"
            );

            // Immediate retry: only succeeds if the failed broadcast released the
            // reservation. With the leak, coin selection sees no spendable UTXO and
            // this fails with a build error instead.
            let second = core
                .send_to_addresses(account_type, 0, outputs, &signer)
                .await;
            assert!(
                second.is_ok(),
                "retry after a failed broadcast should succeed once the reservation \
                 is released for {account_type:?}, got {second:?}"
            );
        }
    }

    /// An *ambiguous* broadcast failure — the network may already have accepted
    /// the transaction — must NOT release the reservation: retrying would risk a
    /// double-spend. The reservation is kept, so an immediate retry fails at the
    /// build stage (no spendable UTXO) rather than reaching broadcast again.
    #[tokio::test]
    async fn send_to_addresses_keeps_reservation_on_ambiguous_broadcast_failure() {
        for account_type in [
            StandardAccountType::BIP44Account,
            StandardAccountType::BIP32Account,
        ] {
            let broadcaster = Arc::new(AlwaysMaybeSentBroadcaster);
            let (core, signer, outputs) = funded_core_wallet(account_type, broadcaster).await;

            let first = core
                .send_to_addresses(account_type, 0, outputs.clone(), &signer)
                .await;
            assert!(
                matches!(
                    first,
                    Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
                ),
                "first send should surface the ambiguous failure for {account_type:?}, got {first:?}"
            );

            // Reservation kept: the retry cannot reselect the reserved input and
            // fails while building, never reaching the broadcaster again.
            let second = core
                .send_to_addresses(account_type, 0, outputs, &signer)
                .await;
            assert!(
                matches!(second, Err(PlatformWalletError::TransactionBuild(_))),
                "retry after an ambiguous failure must fail at build with the reservation \
                 kept for {account_type:?}, got {second:?}"
            );
        }
    }
}
