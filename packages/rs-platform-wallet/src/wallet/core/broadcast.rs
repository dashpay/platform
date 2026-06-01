use dashcore::{Address as DashAddress, Transaction};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::Signer;

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
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        if outputs.is_empty() {
            return Err(PlatformWalletError::TransactionBuild(
                "No outputs specified".to_string(),
            ));
        }

        let tx = {
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

            // The blanket `impl<S: Signer> TransactionSigner for S` in
            // `key-wallet/src/wallet/managed_wallet_info/transaction_builder.rs:482`
            // makes the signer drop-in for the previously `Wallet`-backed
            // path; the funds-derived `address_derivation_path` lookup is
            // unchanged.
            let mut builder = TransactionBuilder::new()
                .set_current_height(current_height)
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_funding(managed_account, account);
            for (addr, amount) in &outputs {
                builder = builder.add_output(addr, *amount);
            }

            let (tx, _fee) = builder
                .build_signed(signer, |addr| {
                    managed_account.address_derivation_path(&addr)
                })
                .await
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;
            tx
        };

        self.broadcast_transaction(&tx).await?;
        Ok(tx)
    }

    /// Sweep the *entire* spendable balance of a CoinJoin account into a
    /// single output to `dest`, leaving no change behind.
    ///
    /// CoinJoin "mixed coins" live on a dedicated CoinJoin account (BIP44
    /// purpose 4'), which [`send_to_addresses`](Self::send_to_addresses)
    /// cannot reach — it only resolves standard BIP44/BIP32 accounts. This
    /// is used by the DashSync → SwiftDashSDK migration to move a user's
    /// mixed coins (no longer supported) into their spendable balance.
    ///
    /// All UTXOs are added as explicit inputs and the transaction is
    /// assembled and signed directly — it deliberately does NOT route through
    /// `TransactionBuilder::build_signed`, whose `LargestFirst` coin selection
    /// re-selects a *covering subset* and stops early, which can drop small
    /// UTXOs (e.g. a tiny fragment sitting behind larger denominations) and
    /// leave the account non-empty. The single output is `total_input - fee`
    /// (fee sized for N inputs + 1 output, no change), so there is no change
    /// output and every UTXO is consumed.
    pub async fn sweep_coinjoin_to_address<S: Signer>(
        &self,
        account_index: u32,
        dest: DashAddress,
        signer: &S,
    ) -> Result<Transaction, PlatformWalletError> {
        use dashcore::blockdata::witness::Witness;
        use dashcore::{ScriptBuf, TxIn, TxOut};
        use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionSigner;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let tx = {
            let mut wm = self.wallet_manager.write().await;
            let (_wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;

            let current_height = info.core_wallet.synced_height();

            let managed_account = info
                .core_wallet
                .accounts
                .coinjoin_accounts
                .get(&account_index)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(format!(
                        "CoinJoin managed account {} not found",
                        account_index
                    ))
                })?;

            // Snapshot every spendable UTXO — the sweep consumes all of them.
            let utxos: Vec<_> = managed_account
                .spendable_utxos(current_height)
                .into_iter()
                .cloned()
                .collect();

            if utxos.is_empty() {
                return Err(PlatformWalletError::TransactionBuild(
                    "no spendable CoinJoin UTXOs to sweep".to_string(),
                ));
            }

            let total_input: u64 = utxos.iter().map(|u| u.value()).sum();
            let input_count = utxos.len();

            // Exact fee for (input_count inputs, 1 output, no change). Mirrors
            // key-wallet's `calculate_base_size()` (8 + input-varint + output-
            // varint + 34) and the selector's 148 B/input, so `total_input -
            // fee` drives the selector to pick all inputs with zero change.
            let fee_rate = FeeRate::normal();
            const BASE_SIZE_1_OUTPUT_NO_CHANGE: usize = 8 + 1 + 1 + 34;
            const INPUT_SIZE: usize = 148;
            let fee = fee_rate
                .calculate_fee(BASE_SIZE_1_OUTPUT_NO_CHANGE + input_count * INPUT_SIZE);

            if total_input <= fee {
                return Err(PlatformWalletError::TransactionBuild(format!(
                    "CoinJoin balance {} is below the sweep fee {}",
                    total_input, fee
                )));
            }
            let output_amount = total_input - fee;

            // Assemble the tx with ALL inputs explicitly and sign it directly.
            // Do NOT use `TransactionBuilder::build_signed` — its `LargestFirst`
            // coin selection re-selects a covering subset and stops once
            // `output_amount + fee` is met, which can drop small UTXOs and
            // leave the CoinJoin account non-empty. A sweep must consume
            // everything, so we build the all-input, single-output tx by hand.
            let tx_inputs: Vec<TxIn> = utxos
                .iter()
                .map(|u| TxIn {
                    previous_output: u.outpoint,
                    script_sig: ScriptBuf::new(),
                    sequence: 0xffff_ffff, // Dash has no RBF
                    witness: Witness::new(),
                })
                .collect();
            let unsigned = Transaction {
                version: 3,
                lock_time: 0,
                input: tx_inputs,
                output: vec![TxOut {
                    value: output_amount,
                    script_pubkey: dest.script_pubkey(),
                }],
                special_transaction_payload: None,
            };

            // `sign_tx` signs `tx.input[i]` using `utxos[i]`, so input order
            // and the utxo vec must line up — both derive from the same vec.
            let signed = signer
                .sign_tx(unsigned, utxos, |addr| {
                    managed_account.address_derivation_path(&addr)
                })
                .await
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            debug_assert_eq!(
                signed.input.len(),
                input_count,
                "CoinJoin sweep must consume every UTXO"
            );
            signed
        };

        self.broadcast_transaction(&tx).await?;
        Ok(tx)
    }
}
