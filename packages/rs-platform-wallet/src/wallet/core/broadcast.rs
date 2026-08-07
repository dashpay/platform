use dashcore::Transaction;
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::ReservationToken;

use super::SignedCoreTransaction;
use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use crate::wallet::reservations::broadcast_releasing_on_rejection;
use crate::{CoreWallet, PlatformWalletError};

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
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
    pub async fn broadcast_finalized_transaction(
        &self,
        transaction: &SignedCoreTransaction,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        match self.broadcaster.broadcast(transaction.transaction()).await {
            Ok(txid) => Ok(txid),
            Err(error) => {
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
    pub(crate) async fn broadcast_payment_releasing_reservation(
        &self,
        accounts: &[key_wallet::account::AccountType],
        transaction: &Transaction,
        token: Option<ReservationToken>,
    ) -> Result<dashcore::Txid, PlatformWalletError> {
        match self.broadcaster.broadcast(transaction).await {
            Ok(txid) => Ok(txid),
            Err(error) => {
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

    use dashcore::{Address as DashAddress, Network, Transaction};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::signer::Signer;
    use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
    use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

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
