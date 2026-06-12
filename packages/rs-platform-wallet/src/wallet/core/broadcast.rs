use dashcore::{Address as DashAddress, Transaction};
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::managed_account::address_pool::KeySource;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::Signer;
use key_wallet::{DerivationPath, Network};

use crate::broadcaster::TransactionBroadcaster;
use crate::{CoreWallet, PlatformWalletError};

/// Max inputs per CoinJoin sweep transaction. A single Dash transaction must
/// stay under the standard relay/mempool size limit (`MAX_STANDARD_TX_SIZE` =
/// 100 000 B); at ~148 B/input (`INPUT_SIZE` below) that is ~675 inputs, so 500
/// leaves a comfortable margin for the output + overhead. A heavy mixer's UTXOs
/// are therefore swept across `ceil(N / MAX_INPUTS_PER_SWEEP)` transactions
/// rather than one oversized, unrelayable transaction.
const MAX_INPUTS_PER_SWEEP: usize = 500;

/// Balanced input count per sweep transaction for `total` spendable UTXOs, so
/// that `utxos.chunks(sweep_chunk_size(total))` yields `ceil(total /
/// MAX_INPUTS_PER_SWEEP)` near-equal chunks, each within `MAX_INPUTS_PER_SWEEP`.
///
/// Using a ceil-divided chunk size keeps chunks near-equal (e.g. 501 → 251 +
/// 250, not 500 + 1), so no chunk is a lone sub-fee dust input. `total` must be
/// greater than zero (the sweep early-returns on an empty UTXO set).
fn sweep_chunk_size(total: usize) -> usize {
    debug_assert!(total > 0, "sweep_chunk_size requires at least one UTXO");
    let num_chunks = total.div_ceil(MAX_INPUTS_PER_SWEEP);
    total.div_ceil(num_chunks)
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

    /// Sweep the *entire* spendable balance of a CoinJoin account to `dest`,
    /// leaving no change behind, across one or more transactions.
    ///
    /// CoinJoin "mixed coins" live on a dedicated CoinJoin account (BIP44
    /// purpose 4'), which [`send_to_addresses`](Self::send_to_addresses)
    /// cannot reach — it only resolves standard BIP44/BIP32 accounts. This
    /// is used by the DashSync → SwiftDashSDK migration to move a user's
    /// mixed coins (no longer supported) into their spendable balance.
    ///
    /// The UTXO set is split into balanced chunks of at most
    /// [`MAX_INPUTS_PER_SWEEP`] inputs so no transaction exceeds the standard
    /// relay size limit (a heavy mixer can hold thousands of small mixed-coin
    /// UTXOs). Each chunk spends a *disjoint* slice of the snapshot, so the
    /// transactions have no inter-dependency and may broadcast in any order.
    ///
    /// Within each chunk all inputs are added explicitly and the transaction
    /// is assembled and signed directly — it deliberately does NOT route
    /// through `TransactionBuilder::build_signed`, whose `LargestFirst` coin
    /// selection re-selects a *covering subset* and stops early, which can drop
    /// small UTXOs and leave the account non-empty. Each chunk's single output
    /// is `chunk_total - chunk_fee` (no change), so every UTXO is consumed.
    ///
    /// Returns the broadcast transactions in chunk order. Broadcast tolerates
    /// partial failure: the successfully broadcast transactions are returned
    /// (the caller refreshes balance and may re-run to sweep any remainder,
    /// since a re-run sees only the still-unspent UTXOs). An error is returned
    /// only if *no* transaction broadcast at all.
    pub async fn sweep_coinjoin_to_address<S: Signer>(
        &self,
        account_index: u32,
        dest: DashAddress,
        signer: &S,
    ) -> Result<Vec<Transaction>, PlatformWalletError> {
        use dashcore::blockdata::witness::Witness;
        use dashcore::{ScriptBuf, TxIn, TxOut};
        use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionSigner;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        // Build + sign every chunk under the wallet write lock (signing borrows
        // `managed_account` for address derivation), then broadcast after the
        // lock is released.
        let signed_txs: Vec<Transaction> = {
            let mut wm = self.wallet_manager.write().await;
            let (wallet, info) = wm.get_wallet_and_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound(
                    "Wallet not found in wallet manager".to_string(),
                )
            })?;

            // CoinJoin account key material for deriving signing paths across
            // BOTH chains. DashSync CoinJoin puts mixing *change* on the internal
            // chain (.../1/i), but the SDK's CoinJoin account derives only the
            // external pool (.../0/i) — so internal-chain inputs are owned and
            // spendable yet have no derivation path. `coinjoin_sweep_path_map`
            // (below) re-derives both chains from the account xpub. Copied/cloned
            // out so the `wallet` borrow ends before the `info` borrow
            // (`account_xpub` is `Copy`).
            let (account_xpub, account_type, network) = {
                let acct = wallet
                    .accounts
                    .coinjoin_accounts
                    .get(&account_index)
                    .ok_or_else(|| {
                        PlatformWalletError::WalletNotFound(format!(
                            "CoinJoin account {} not found",
                            account_index
                        ))
                    })?;
                (acct.account_xpub, acct.account_type, acct.network)
            };

            // Fund-safety invariant owned by the sweep itself, not just the FFI
            // boundary: refuse to drain every CoinJoin UTXO to a destination that
            // isn't valid on this wallet's network. The sweep is irreversible, so
            // any non-FFI Rust caller (tests, future wrappers) is covered here by
            // construction.
            if !dest.as_unchecked().is_valid_for_network(network) {
                return Err(PlatformWalletError::AddressOperation(format!(
                    "CoinJoin sweep destination is not valid for the wallet network {network:?}"
                )));
            }

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

            // Resolve an absolute derivation path for every input address across
            // BOTH CoinJoin chains (external /0/ and internal /1/), so the signer
            // can sign internal-chain ("change") mixed coins too. Errors if any
            // input can't be resolved on either chain rather than failing mid-sign.
            let account_path = account_type
                .derivation_path(network)
                .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))?;
            let key_source = KeySource::Public(account_xpub);
            let input_addresses: Vec<DashAddress> =
                utxos.iter().map(|u| u.address.clone()).collect();
            let path_map = coinjoin_sweep_path_map(
                &account_path,
                &key_source,
                network,
                &input_addresses,
                COINJOIN_SWEEP_MAX_INDEX,
            )?;

            let fee_rate = FeeRate::normal();
            // Upper bounds for the serialized size of a (N-input, 1-output,
            // no-change) tx. `FeeRate::normal()` is exactly 1 duff/byte — the
            // relay minimum, with no headroom — so this size estimate must never
            // undershoot the real transaction or the chunk pays below the minimum
            // fee and gets rejected. The maximum compressed-P2PKH input is 149 B
            // (36 outpoint + 1 script-len + 108 scriptSig at a 73-byte low-S DER
            // signature + 4 sequence); the input-count CompactSize is sized per
            // chunk below (it grows from 1 to 3 bytes at 253 inputs). Slightly
            // over-paying on an all-funds sweep is harmless — the single output
            // just absorbs the difference.
            const VERSION_PLUS_LOCKTIME: usize = 8;
            const OUTPUT_COUNT_VARINT: usize = 1; // exactly one output
            const ONE_P2PKH_OUTPUT: usize = 34;
            const MAX_P2PKH_INPUT_SIZE: usize = 149;

            // Balanced chunks of <= MAX_INPUTS_PER_SWEEP so no transaction
            // exceeds the relay size limit. `chunks()` over disjoint slices
            // guarantees each UTXO is consumed by exactly one transaction.
            let chunk_size = sweep_chunk_size(utxos.len());
            let mut signed_txs = Vec::with_capacity(utxos.len().div_ceil(chunk_size));

            for chunk in utxos.chunks(chunk_size) {
                let chunk_utxos: Vec<_> = chunk.to_vec();
                let input_count = chunk_utxos.len();
                let total_input: u64 = chunk_utxos.iter().map(|u| u.value()).sum();

                // Upper-bound fee for (input_count inputs, 1 output, no change)
                // at the 1 duff/byte relay minimum, so `total_input - fee` is a
                // single zero-change output that always clears relay. The
                // input-count CompactSize is 1 byte below 253 inputs and 3 bytes
                // for 253..=MAX_INPUTS_PER_SWEEP (500).
                let input_count_varint = if input_count < 253 { 1 } else { 3 };
                let tx_size = VERSION_PLUS_LOCKTIME
                    + input_count_varint
                    + OUTPUT_COUNT_VARINT
                    + ONE_P2PKH_OUTPUT
                    + input_count * MAX_P2PKH_INPUT_SIZE;
                let fee = fee_rate.calculate_fee(tx_size);

                if total_input <= fee {
                    return Err(PlatformWalletError::TransactionBuild(format!(
                        "CoinJoin sweep chunk balance {} is below the chunk fee {}",
                        total_input, fee
                    )));
                }
                let output_amount = total_input - fee;

                // Assemble the chunk's tx with ITS inputs explicitly and sign
                // it directly. Do NOT use `TransactionBuilder::build_signed` —
                // its `LargestFirst` coin selection re-selects a covering subset
                // and stops once `output_amount + fee` is met, which can drop
                // small UTXOs and leave the CoinJoin account non-empty. A sweep
                // must consume everything, so each chunk is an all-input,
                // single-output tx built by hand.
                let tx_inputs: Vec<TxIn> = chunk_utxos
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

                // `sign_tx` signs `tx.input[i]` using `chunk_utxos[i]`, so input
                // order and the utxo vec must line up — both derive from `chunk`.
                let signed = signer
                    .sign_tx(unsigned, chunk_utxos, |addr| path_map.get(&addr).cloned())
                    .await
                    .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

                // Fund-safety invariant: the single output is `total_input -
                // fee`, which is only correct if the signer consumed every UTXO
                // in the chunk. Enforce at runtime (not debug_assert, which is
                // compiled out in release) so a signer that ever drops an input
                // aborts the chunk instead of broadcasting an under-consuming tx.
                if signed.input.len() != input_count {
                    return Err(PlatformWalletError::TransactionBuild(format!(
                        "CoinJoin sweep chunk must consume every UTXO in the chunk: \
                         signed {} inputs, expected {}",
                        signed.input.len(),
                        input_count
                    )));
                }
                signed_txs.push(signed);
            }

            signed_txs
        };

        // Broadcast each chunk (disjoint inputs, no inter-tx dependency, so
        // order is irrelevant). Collect successes and tolerate partial failure
        // so a flaky broadcast doesn't strand the chunks that did go out — the
        // caller can re-run to sweep any remainder. Error only if nothing
        // broadcast at all.
        let mut broadcast: Vec<Transaction> = Vec::with_capacity(signed_txs.len());
        let mut last_err: Option<PlatformWalletError> = None;
        for tx in signed_txs {
            match self.broadcast_transaction(&tx).await {
                Ok(_) => broadcast.push(tx),
                Err(e) => {
                    // Partial failure is tolerated (caller re-runs to sweep the
                    // remainder), but never silent: log each dropped chunk error.
                    tracing::warn!(
                        "CoinJoin sweep: a chunk failed to broadcast, continuing \
                         with remaining chunks (caller can re-run): {}",
                        e
                    );
                    // Keep the FIRST failure (usually the root cause); the later
                    // chunk errors are already surfaced via the warn! above.
                    last_err.get_or_insert(e);
                }
            }
        }

        if broadcast.is_empty() {
            return Err(last_err.unwrap_or_else(|| {
                PlatformWalletError::TransactionBuild(
                    "CoinJoin sweep produced no broadcastable transactions".to_string(),
                )
            }));
        }

        Ok(broadcast)
    }
}

/// Build an `address → absolute derivation path` map covering every address in
/// `needed_addrs` across BOTH CoinJoin chains — external (`/0/`) and internal
/// (`/1/`) — derived from the account's public xpub. `account_path` is the
/// hardened account path `m/9'/coin'/4'/account'`.
///
/// The SDK's CoinJoin account models only the external chain, but DashSync
/// CoinJoin puts mixing *change* on the internal chain. A migrated wallet's
/// internal-chain mixed coins are imported as spendable UTXOs yet have no
/// derivation path in the account, so signing a sweep that includes them fails
/// with "no derivation path for input address". This re-derives both chains
/// (non-hardened, public-key-only) and returns each input's absolute path so the
/// signer can sign every input regardless of chain.
///
/// Returns an error if any address can't be resolved within
/// `COINJOIN_SWEEP_MAX_INDEX` on either chain — defensive, so a sweep never
/// silently mis-signs or drops an input.
/// Per-chain index ceiling for the sweep resolver. A heavy mixer's CoinJoin
/// indices sit well under this; the cap only bounds the (never-hit-in-practice)
/// unresolved case so the search terminates.
const COINJOIN_SWEEP_MAX_INDEX: u32 = 20_000;

fn coinjoin_sweep_path_map(
    account_path: &DerivationPath,
    key_source: &KeySource,
    network: Network,
    needed_addrs: &[DashAddress],
    max_index: u32,
) -> Result<std::collections::HashMap<DashAddress, DerivationPath>, PlatformWalletError> {
    use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType};
    use key_wallet::ChildNumber;
    use std::collections::{HashMap, HashSet};

    const BATCH: u32 = 500;

    let mut needed: HashSet<DashAddress> = needed_addrs.iter().cloned().collect();
    let mut path_map: HashMap<DashAddress, DerivationPath> = HashMap::new();

    // chain 0 = external (receive / denominations), chain 1 = internal (change).
    for (chain, pool_type) in [
        (0u32, AddressPoolType::External),
        (1u32, AddressPoolType::Internal),
    ] {
        if needed.is_empty() {
            break;
        }
        let mut base = account_path.clone();
        base.push(
            ChildNumber::from_normal_idx(chain)
                .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))?,
        );
        // Empty pool (NoKeySource skips generation); we generate below with the
        // real public key source so each `AddressInfo` carries its full path.
        let mut pool = AddressPool::new(base, pool_type, 0, network, &KeySource::NoKeySource)
            .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))?;

        let mut generated = 0u32;
        while generated < max_index && !needed.is_empty() {
            let batch = pool
                .generate_addresses(BATCH, key_source, true)
                .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))?;
            for addr in &batch {
                // Only drop from `needed` once the path is actually recorded, so
                // "removed from needed ⇒ inserted into path_map" holds by
                // construction. If `address_info` ever returned None for a
                // freshly generated address (an AddressPool invariant break), the
                // address stays in `needed` and the check below errors loudly
                // rather than silently dropping it into a mid-sign failure.
                if needed.contains(addr) {
                    if let Some(info) = pool.address_info(addr) {
                        path_map.insert(addr.clone(), info.path.clone());
                        needed.remove(addr);
                    }
                }
            }
            generated += BATCH;
        }
    }

    if !needed.is_empty() {
        return Err(PlatformWalletError::TransactionBuild(format!(
            "CoinJoin sweep: {} input address(es) have no derivation path on either \
             CoinJoin chain (within {} indices)",
            needed.len(),
            max_index
        )));
    }

    Ok(path_map)
}

#[cfg(test)]
mod sweep_chunking_tests {
    use super::{sweep_chunk_size, MAX_INPUTS_PER_SWEEP};

    /// The chunk plan must, for any UTXO count: produce `ceil(total / MAX)`
    /// transactions, keep every chunk within `MAX` inputs, and consume every
    /// UTXO exactly once (disjoint slices that sum back to `total`).
    #[test]
    fn partitions_every_utxo_within_the_relay_cap() {
        for &total in &[
            1usize, 30, 499, 500, 501, 675, 999, 1000, 1001, 1499, 5000, 12_345,
        ] {
            let chunk_size = sweep_chunk_size(total);
            let sizes: Vec<usize> = (0..total)
                .collect::<Vec<_>>()
                .chunks(chunk_size)
                .map(|c| c.len())
                .collect();

            let expected_chunks = total.div_ceil(MAX_INPUTS_PER_SWEEP);
            assert_eq!(sizes.len(), expected_chunks, "tx count for {total} UTXOs");
            assert_eq!(
                sizes.iter().sum::<usize>(),
                total,
                "every UTXO consumed exactly once for {total}"
            );
            assert!(
                sizes
                    .iter()
                    .all(|&n| (1..=MAX_INPUTS_PER_SWEEP).contains(&n)),
                "every chunk within [1, {MAX_INPUTS_PER_SWEEP}] for {total}: {sizes:?}"
            );
        }
    }
}

#[cfg(test)]
mod coinjoin_sweep_path_map_tests {
    use super::coinjoin_sweep_path_map;
    use dashcore::secp256k1::Secp256k1;
    use key_wallet::account::account_type::AccountType;
    use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
    use key_wallet::{ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey, Network};

    fn coinjoin_account(network: Network, seed_byte: u8) -> (ExtendedPubKey, DerivationPath) {
        let secp = Secp256k1::new();
        let master = ExtendedPrivKey::new_master(network, &[seed_byte; 32]).unwrap();
        let account_path = AccountType::CoinJoin { index: 0 }
            .derivation_path(network)
            .unwrap();
        let account_xpriv = master.derive_priv(&secp, &account_path).unwrap();
        (
            ExtendedPubKey::from_priv(&secp, &account_xpriv),
            account_path,
        )
    }

    /// Derive `<account_path>/<chain>/<index>` the way the resolver does, to get
    /// a known target address to look up.
    fn derive_addr(
        xpub: &ExtendedPubKey,
        account_path: &DerivationPath,
        network: Network,
        chain: u32,
        index: u32,
    ) -> dashcore::Address {
        let pool_type = if chain == 0 {
            AddressPoolType::External
        } else {
            AddressPoolType::Internal
        };
        let mut base = account_path.clone();
        base.push(ChildNumber::from_normal_idx(chain).unwrap());
        let mut pool =
            AddressPool::new(base, pool_type, 0, network, &KeySource::NoKeySource).unwrap();
        let addrs = pool
            .generate_addresses(index + 1, &KeySource::Public(*xpub), true)
            .unwrap();
        addrs[index as usize].clone()
    }

    fn abs_path(account_path: &DerivationPath, chain: u32, index: u32) -> DerivationPath {
        let mut p = account_path.clone();
        p.push(ChildNumber::from_normal_idx(chain).unwrap());
        p.push(ChildNumber::from_normal_idx(index).unwrap());
        p
    }

    /// Both an external (/0/) and an internal (/1/) CoinJoin address resolve to
    /// their correct absolute paths — the internal case is the bug this fixes.
    #[test]
    fn resolves_external_and_internal_chain_addresses() {
        let network = Network::Testnet;
        let (xpub, account_path) = coinjoin_account(network, 7);
        let key_source = KeySource::Public(xpub);

        let external = derive_addr(&xpub, &account_path, network, 0, 7);
        let internal = derive_addr(&xpub, &account_path, network, 1, 42);

        let map = coinjoin_sweep_path_map(
            &account_path,
            &key_source,
            network,
            &[external.clone(), internal.clone()],
            200,
        )
        .unwrap();

        assert_eq!(map.get(&external), Some(&abs_path(&account_path, 0, 7)));
        assert_eq!(map.get(&internal), Some(&abs_path(&account_path, 1, 42)));
    }

    /// An address not derivable from this account xpub on either chain yields the
    /// defensive error (here: a different wallet's CoinJoin address).
    #[test]
    fn errors_when_an_input_address_is_unresolvable() {
        let network = Network::Testnet;
        let (xpub, account_path) = coinjoin_account(network, 7);
        let key_source = KeySource::Public(xpub);

        let (other_xpub, other_path) = coinjoin_account(network, 9);
        let foreign = derive_addr(&other_xpub, &other_path, network, 0, 3);

        let result = coinjoin_sweep_path_map(&account_path, &key_source, network, &[foreign], 200);
        assert!(result.is_err(), "foreign address must not resolve");
    }
}
