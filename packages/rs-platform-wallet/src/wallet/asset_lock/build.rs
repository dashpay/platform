//! Asset lock transaction building.
//!
//! Contains methods for building asset lock transactions, peeking at funding
//! addresses, and the unified `create_funded_asset_lock_proof` entry point.

use crate::broadcaster::TransactionBroadcaster;
use std::time::Duration;

use dashcore::blockdata::transaction::special_transaction::asset_lock::AssetLockPayload;
use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::Address as DashAddress;
use dashcore::{OutPoint, Transaction, TxOut};
use key_wallet::account::AccountType;
use key_wallet::bip32::DerivationPath;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::ExtendedPubKeySigner;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::{
    AssetLockFundingType, CreditOutputFunding,
};
use key_wallet::wallet::managed_wallet_info::coin_selection::{SelectionError, SelectionStrategy};
use key_wallet::wallet::managed_wallet_info::fee::FeeRate;
use key_wallet::wallet::managed_wallet_info::managed_account_operations::ManagedAccountOperations;
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    BuilderError, TransactionBuilder,
};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::ManagedAccountType;
use key_wallet::Utxo;

use crate::changeset::{AccountRegistrationEntry, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::PlatformWalletInfo;

use super::manager::{AssetLockManager, DEFAULT_FEE_PER_KB};
use super::tracked::{AssetLockStatus, TrackedAssetLock};

// ---------------------------------------------------------------------------
// Asset lock transaction building
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Build an asset lock transaction using the key-wallet builder.
    ///
    /// Delegates UTXO selection, fee calculation, and signing to
    /// `ManagedWalletInfo::build_asset_lock_with_signer`. The host
    /// never sees a raw credit-output private key — the returned
    /// `DerivationPath` is what the caller hands back to the same
    /// `signer` when the credit output is later consumed on Platform.
    ///
    /// # Arguments
    ///
    /// * `amount_duffs` — Amount to lock in duffs.
    /// * `account_index` — BIP44 account index to select UTXOs from.
    /// * `funding_type` — Which account to derive the one-time key from
    ///   (e.g., `IdentityRegistration`, `IdentityTopUp`).
    /// * `identity_index` — Identity index (used by `IdentityTopUp`, ignored by others).
    /// * `signer` — External signer that produces both the funding-input
    ///   P2PKH signatures and the credit-output public key. For Swift,
    ///   this is typically a
    ///   [`MnemonicResolverCoreSigner`](crate::wallet::asset_lock::build)
    ///   from `platform-wallet-ffi` — built on top of the
    ///   Keychain-resolver vtable so private keys never cross the FFI
    ///   boundary.
    pub async fn build_asset_lock_transaction<S: ExtendedPubKeySigner>(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(Transaction, DerivationPath), PlatformWalletError> {
        if amount_duffs == 0 {
            return Err(PlatformWalletError::AssetLockTransaction(
                "Amount must be greater than zero".to_string(),
            ));
        }

        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_mut_and_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        // 0. For a per-index identity top-up, lazily derive + insert the
        //    `IdentityTopUp { registration_index }` account (both the
        //    xpub-bearing `Wallet.accounts` side and the managed
        //    `ManagedWalletInfo.accounts` side) if it isn't there yet.
        //    Wallet setup only derives the *singleton* special accounts
        //    (identity_registration, etc.); per-index topup accounts are
        //    keyed by the identity's registration index and can't be
        //    enumerated ahead of time, so we derive one on demand here.
        if funding_type == AssetLockFundingType::IdentityTopUp {
            self.ensure_identity_topup_account(wallet, info, identity_index, signer)
                .await?;
        }

        // 1. Peek at the next unused address from the funding account to
        //    build the credit output P2PKH script.
        let funding_address = Self::peek_next_funding_address(
            &mut info.core_wallet,
            wallet,
            funding_type,
            identity_index,
        )?;

        // 2. Build the credit output for the asset lock payload.
        let credit_output = TxOut {
            value: amount_duffs,
            script_pubkey: funding_address.script_pubkey(),
        };

        let funding = CreditOutputFunding {
            output: credit_output,
            funding_type,
            identity_index,
        };

        // 3. Fund the asset lock.
        //
        // Shielded funding (`AssetLockShieldedAddressTopUp`) must be able to
        // draw on previously-mixed CoinJoin coins, which live on the DIP-9
        // CoinJoin derivation account — the pinned key-wallet
        // `build_asset_lock_with_signer` funds from a SINGLE BIP44 account
        // only, so those coins counted in the balance but could not be
        // shielded (dashpay/platform#4073). Route shielded funding through the
        // union-of-accounts builder; every other funding type keeps the
        // single-BIP44-account path (spending mixed CoinJoin coins into an
        // identity registration would de-anonymize them — a deliberate
        // privacy choice left out of scope here).
        if funding_type == AssetLockFundingType::AssetLockShieldedAddressTopUp {
            return self
                .build_asset_lock_tx_from_all_funding_accounts(
                    wallet,
                    info,
                    account_index,
                    vec![funding],
                    DEFAULT_FEE_PER_KB,
                    signer,
                )
                .await;
        }

        // Delegate to the key-wallet signer-driven builder (single BIP44 account).
        let result = info
            .core_wallet
            .build_asset_lock_with_signer(
                wallet,
                account_index,
                vec![funding],
                DEFAULT_FEE_PER_KB,
                signer,
            )
            .await
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Asset lock builder failed: {}",
                    e
                ))
            })?;

        // 4. Pull the (pubkey, path) for our single credit output.
        //
        // `build_asset_lock_with_signer` always returns the `Public`
        // variant. The `Private` arm would only come from the soft-
        // wallet `build_asset_lock` path which we no longer call from
        // platform-wallet — defensively bail if it appears.
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockCreditKeys;
        let path = match result.keys {
            AssetLockCreditKeys::Public(mut keys) => {
                let (_pubkey, path) = keys.drain(..).next().ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(
                        "Builder returned no credit-output keys".to_string(),
                    )
                })?;
                path
            }
            AssetLockCreditKeys::Private(_) => {
                return Err(PlatformWalletError::AssetLockTransaction(
                    "Builder returned Private keys; signer-driven path expected Public".to_string(),
                ));
            }
        };

        Ok((result.transaction, path))
    }

    /// Build + sign an asset-lock transaction whose funding inputs are drawn
    /// from the UNION of every spendable Core funds account (BIP44 + BIP32 +
    /// CoinJoin + DashPay), not just the single BIP44 account at
    /// `account_index`.
    ///
    /// ## Why this exists (dashpay/platform#4073)
    ///
    /// The pinned key-wallet `ManagedWalletInfo::build_asset_lock_with_signer`
    /// funds an asset lock from exactly ONE BIP44 standard account: it calls
    /// `TransactionBuilder::set_funding` on
    /// `standard_bip44_accounts[account_index]` and signs with a
    /// single-account path resolver (`funds_acc.address_derivation_path`).
    /// Previously-mixed CoinJoin coins live on the DIP-9 CoinJoin derivation
    /// account (`coinjoin_accounts`), so they are counted in the wallet
    /// balance (which sums `all_funding_accounts`) yet were invisible to the
    /// asset-lock coin selector — shielding failed with a coin-selection
    /// "Insufficient funds" even though the wallet-wide balance covered the
    /// amount.
    ///
    /// This method keeps `account_index` as the PRIMARY account (its
    /// reservation ledger gates concurrent primary-account builds, and change
    /// flows back to it via `set_funding`'s change address) but ADDS the
    /// spendable UTXOs of every other funds account as explicit builder
    /// inputs (`add_inputs`), and signs with a resolver that spans all funds
    /// accounts. The credit-output key is still derived from the
    /// shielded-topup account exactly as the single-account builder does
    /// (peek path → signer pubkey → mark used), so the returned
    /// `DerivationPath` lines up with the credit-output script the caller
    /// already peeked.
    ///
    /// ## Interim caveat (superseded by the upstream fix)
    ///
    /// The clean long-term fix belongs upstream in key-wallet
    /// (`build_asset_lock_with_signer` gathering inputs + reservations across
    /// accounts). Until that pin bump lands, this workspace composition makes
    /// CoinJoin funds shieldable today. `TransactionBuilder::set_funding`
    /// captures only the PRIMARY account's `ReservationSet` (the type is
    /// `pub(crate)` in key-wallet, so this crate cannot reserve per-account),
    /// so inputs selected from non-primary accounts are recorded in the
    /// primary account's reservation ledger rather than their own. They are
    /// therefore not protected against a concurrent build on their own
    /// account for the brief window before the broadcast tx is processed back
    /// into the wallet (which releases the outpoints from every account's
    /// ledger). Shielded funding is single-flighted under `shield_guard` and
    /// the whole build runs under the wallet write lock, and the app issues no
    /// concurrent non-shielded spend on the CoinJoin/BIP32 accounts, so the
    /// race window is not reachable in practice.
    async fn build_asset_lock_tx_from_all_funding_accounts<S: ExtendedPubKeySigner>(
        &self,
        wallet: &Wallet,
        info: &mut PlatformWalletInfo,
        account_index: u32,
        credit_output_fundings: Vec<CreditOutputFunding>,
        fee_per_kb: u64,
        signer: &S,
    ) -> Result<(Transaction, DerivationPath), PlatformWalletError> {
        use std::collections::{HashMap, HashSet};

        let target_duffs: u64 = credit_output_fundings.iter().map(|f| f.output.value).sum();
        let height = info.core_wallet.last_processed_height();
        tracing::debug!(
            target_duffs,
            height,
            primary_account_index = account_index,
            funding_accounts = info.core_wallet.accounts.all_funding_accounts().len(),
            "multi-account asset-lock funding: enumerating spendable funds accounts"
        );

        // Snapshot the primary account's spendable outpoints so the union
        // sweep below does not add them twice: `set_funding` already seeds
        // them, and `add_inputs` must contribute only the OTHER accounts.
        let primary_outpoints: HashSet<OutPoint> = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&account_index)
            .map(|a| {
                a.spendable_utxos(height)
                    .into_iter()
                    .map(|u| u.outpoint)
                    .collect()
            })
            .unwrap_or_default();

        // Build, from an immutable borrow of every funds account:
        //   (a) an owned `Address -> DerivationPath` resolver covering every
        //       spendable input across ALL accounts, so signing can resolve a
        //       key for an input selected from any account; and
        //   (b) the explicit extra inputs (all non-primary accounts).
        let mut path_map: HashMap<DashAddress, DerivationPath> = HashMap::new();
        let mut extra_inputs: Vec<Utxo> = Vec::new();
        let mut union_value: u64 = 0;
        let mut union_count: usize = 0;
        for acc in info.core_wallet.accounts.all_funding_accounts() {
            // Never fund an asset lock from coins the local wallet holds no
            // signing key for. `all_funding_accounts()` (in the pinned
            // key-wallet fork) includes `dashpay_external_accounts`, and
            // `spendable_utxos()` filters on maturity but not ownership. The
            // managed layer stores every account's PUBLIC key only (its
            // `KeySource` is always `Public`), so a managed-level watch-only
            // flag cannot discriminate here — the ownership signal is the
            // account-type variant. `DashpayExternalAccount` is the sole
            // watch-only funds account type: production builds it from a
            // CONTACT's decrypted xpub with `is_watch_only: true` (see
            // `wallet/identity/network/contacts.rs`), so its UTXOs are the
            // contact's coins. `MnemonicResolverCoreSigner` would blindly
            // derive `acc.address_derivation_path(&utxo.address)` from the
            // LOCAL mnemonic, yielding a different key and therefore an invalid
            // input signature. Skip such accounts entirely — both from the
            // path resolver and from the explicit extra inputs.
            if matches!(
                acc.managed_account_type(),
                ManagedAccountType::DashpayExternalAccount { .. }
            ) {
                continue;
            }
            for utxo in acc.spendable_utxos(height) {
                union_value = union_value.saturating_add(utxo.value());
                union_count += 1;
                if let Some(path) = acc.address_derivation_path(&utxo.address) {
                    path_map.insert(utxo.address.clone(), path);
                }
                if !primary_outpoints.contains(&utxo.outpoint) {
                    extra_inputs.push(utxo.clone());
                }
            }
        }
        tracing::debug!(
            union_count,
            union_value,
            primary_count = primary_outpoints.len(),
            extra_count = extra_inputs.len(),
            resolver_entries = path_map.len(),
            "multi-account asset-lock funding: union UTXO set assembled"
        );

        let acc = wallet
            .get_bip44_account(account_index)
            .ok_or_else(|| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "BIP44 account {account_index} not found for asset-lock funding"
                ))
            })?
            .clone();
        let credit_outputs: Vec<TxOut> = credit_output_fundings
            .iter()
            .map(|f| f.output.clone())
            .collect();

        // Seed the primary account (inputs + change address + reservations),
        // then append the union of the other accounts' spendable inputs. The
        // `&mut` borrow of the primary account is scoped to this block; the
        // returned builder owns cloned inputs / reservations / change address,
        // so no account borrow is held across the signer await below.
        let builder = {
            let primary_funds = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&account_index)
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(format!(
                        "managed BIP44 account {account_index} not found for asset-lock funding"
                    ))
                })?;
            TransactionBuilder::new()
                .set_fee_rate(FeeRate::new(fee_per_kb))
                .set_current_height(height)
                // LargestFirst, NOT the `TransactionBuilder::new()` default
                // `BranchAndBound`. This is load-bearing, not an optimization:
                // BranchAndBound routes to a recursive exact-match subset-sum
                // (`CoinSelector::find_exact_match`) whose search space is
                // EXPONENTIAL in the number of sub-target UTXOs. The
                // single-BIP44-account path tolerates it (a handful of UTXOs),
                // but a CoinJoin account holds many small mixed denominations
                // (0.001 / 0.01 / 0.1 DASH ...); feeding that whole set to
                // BranchAndBound hangs the FFI call for minutes with no logs and
                // no broadcast (observed on-device, dashpay/platform#4073
                // follow-up). LargestFirst uses the linear greedy accumulator
                // (`accumulate_coins_with_size`), which also minimizes the input
                // count — fewer signer round-trips (each input is one resolver
                // upcall) and a smaller tx/fee.
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_special_payload(TransactionPayload::AssetLockPayloadType(
                    AssetLockPayload::new(credit_outputs),
                ))
                .set_funding(primary_funds, &acc)
                .add_inputs(extra_inputs)
                .require_final_inputs()
        };

        tracing::debug!(
            target_duffs,
            "multi-account asset-lock funding: selecting + signing (LargestFirst)"
        );
        let (transaction, fee) = builder
            .build_signed(signer, move |addr| path_map.get(&addr).cloned())
            .await
            .map_err(|e| map_builder_error(e, target_duffs))?;
        tracing::debug!(
            selected_inputs = transaction.input.len(),
            fee,
            txid = %transaction.txid(),
            "multi-account asset-lock funding: transaction built + signed"
        );

        // Derive the single credit-output key from the shielded-topup account,
        // mirroring the pinned single-account builder's phase-1/2/3 sequence
        // (peek without marking → signer round-trip → commit the index) so a
        // signer failure never irreversibly consumes a pool index.
        let (path, index) = {
            let credit_account = info
                .core_wallet
                .accounts
                .asset_lock_shielded_address_topup
                .as_mut()
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(
                        "Asset lock shielded address top-up account not found".to_string(),
                    )
                })?;
            credit_account
                .peek_next_path()
                .map_err(|e| PlatformWalletError::AssetLockTransaction(e.to_string()))?
        };
        signer.public_key(&path).await.map_err(|e| {
            PlatformWalletError::AssetLockTransaction(format!("signer public_key failed: {e}"))
        })?;
        {
            let credit_account = info
                .core_wallet
                .accounts
                .asset_lock_shielded_address_topup
                .as_mut()
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(
                        "Asset lock shielded address top-up account not found".to_string(),
                    )
                })?;
            credit_account
                .mark_first_pool_index_used(index)
                .map_err(|e| PlatformWalletError::AssetLockTransaction(e.to_string()))?;
        }

        tracing::debug!(
            selected_inputs = transaction.input.len(),
            "multi-account asset-lock funding: credit-output key derived; returning built tx"
        );
        Ok((transaction, path))
    }

    /// Peek at the next unused address from a funding account without
    /// consuming it (i.e. without marking it as used).
    ///
    /// The key-wallet builder's `next_private_key` will later find the same
    /// address, derive the private key, and mark it as used.
    fn peek_next_funding_address(
        wallet_info: &mut ManagedWalletInfo,
        wallet: &Wallet,
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let (managed_account, account_xpub) = match funding_type {
            AssetLockFundingType::IdentityRegistration => {
                let xpub = wallet
                    .accounts
                    .identity_registration
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_registration
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity registration account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityTopUp => {
                let xpub = wallet
                    .accounts
                    .identity_topup
                    .get(&identity_index)
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_topup
                    .get_mut(&identity_index)
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(format!(
                            "Identity top-up account for index {} not found",
                            identity_index
                        ))
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityTopUpNotBound => {
                let xpub = wallet
                    .accounts
                    .identity_topup_not_bound
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_topup_not_bound
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity top-up (unbound) account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityInvitation => {
                let xpub = wallet
                    .accounts
                    .identity_invitation
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_invitation
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity invitation account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::AssetLockAddressTopUp => {
                let xpub = wallet
                    .accounts
                    .asset_lock_address_topup
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .asset_lock_address_topup
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Asset lock address top-up account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::AssetLockShieldedAddressTopUp => {
                let xpub = wallet
                    .accounts
                    .asset_lock_shielded_address_topup
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .asset_lock_shielded_address_topup
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Asset lock shielded address top-up account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
        };

        // Get the next unused address from the pool. `next_address`
        // always persists the newly-generated address into the pool's
        // state so the builder's `next_private_key` can find it. The
        // address is NOT marked as used yet — that happens inside the
        // builder after a successful transaction build.
        managed_account
            .next_address(account_xpub.as_ref(), false)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Failed to get next funding address: {}",
                    e
                ))
            })
    }

    /// Idempotently derive + insert the per-index `IdentityTopUp`
    /// derivation account into BOTH the xpub-bearing `Wallet.accounts`
    /// and the managed `ManagedWalletInfo.accounts`, and persist its
    /// registration.
    ///
    /// Wallet setup (`create_special_purpose_accounts`) only derives the
    /// *singleton* special accounts (`identity_registration`, etc.);
    /// per-index topup accounts are keyed by the identity's registration
    /// index, so we derive one on demand the first time a given identity
    /// is topped up. Safe to call on every build / retry: existing
    /// accounts are left untouched by the `contains_*` guards.
    ///
    /// ## Persistence
    ///
    /// A newly created account is persisted as an
    /// [`AccountRegistrationEntry`] plus its initial address-pool
    /// snapshot(s) — the same round shape `manager::wallet_lifecycle`
    /// emits at wallet registration — before this method returns. This
    /// is load-bearing for crash recovery: the load path rebuilds
    /// `Wallet.accounts` from persisted registrations only (the
    /// `account_registrations` / `account_address_pools` changeset
    /// fields are not replayed by `apply_changeset`), so without this
    /// round a restart between broadcast and consumption leaves
    /// `resume_asset_lock` unable to re-derive the credit-output path
    /// ("Funding account IdentityTopUp not found for re-derivation")
    /// and the already-broadcast top-up stranded. Re-deriving the
    /// account at resume time instead is not an option: the hardened
    /// topup xpub needs the external signer on production wallets, and
    /// `resume_asset_lock` (and the FFI launch-time catch-up that
    /// drives it) runs without one.
    ///
    /// A failed store rolls back the in-memory inserts, so a later
    /// retry re-creates AND re-persists the account instead of the
    /// `contains_*` guards skipping a persist that never happened.
    ///
    /// ## Two derivation paths
    ///
    /// The production platform wallet is **external-signable**: at
    /// registration it is `downgrade_to_external_signable()`'d and holds
    /// only account xpubs — no root xpriv/seed (that lives behind the
    /// Swift Keychain, reachable only through the `signer`). The
    /// `IdentityTopUp` derivation path is HARDENED, so the seedless
    /// `Wallet::add_account(_, None)` "derive from root xpriv" path fails
    /// for such wallets. We therefore derive the account xpub through the
    /// `signer` (`ExtendedPubKeySigner::extended_public_key`, which the
    /// `MnemonicResolverCoreSigner` resolves via the Keychain mnemonic) and
    /// insert the resulting xpub explicitly.
    ///
    /// Full-signable wallets (unit tests, in-memory soft wallets) keep the
    /// cheaper local `add_account(_, None)` path — no signer round-trip.
    async fn ensure_identity_topup_account<S: ExtendedPubKeySigner>(
        &self,
        wallet: &mut Wallet,
        info: &mut PlatformWalletInfo,
        identity_index: u32,
        signer: &S,
    ) -> Result<(), PlatformWalletError> {
        let account_type = AccountType::IdentityTopUp {
            registration_index: identity_index,
        };

        // (a) xpub side — insert the account into `Wallet.accounts` if it
        //     isn't there yet.
        let created_xpub_side = !wallet.accounts.contains_account_type(&account_type);
        if created_xpub_side {
            // NOTE: gate on `is_external_signable()`, NOT `can_sign()` —
            // `can_sign()` is `!watch_only`, so it's TRUE for external-signable
            // wallets (they CAN sign, just via the external signer), which
            // would wrongly take the local `add_account(_, None)` path and fail
            // with "External signable wallet has no private key".
            if !wallet.is_external_signable() {
                // Full-signable wallet (tests / soft wallets): derive the
                // account xpub locally from the wallet's root xpriv.
                wallet.add_account(account_type, None).map_err(|e| {
                    PlatformWalletError::AssetLockTransaction(format!(
                        "Failed to derive identity top-up account for index {}: {}",
                        identity_index, e
                    ))
                })?;
            } else {
                // External-signable wallet (production): no root key at
                // rest — derive the hardened account xpub through the
                // external signer, then insert it explicitly.
                let path = account_type.derivation_path(wallet.network).map_err(|e| {
                    PlatformWalletError::AssetLockTransaction(format!(
                        "Failed to compute identity top-up derivation path for index {}: {}",
                        identity_index, e
                    ))
                })?;
                let account_xpub = signer.extended_public_key(&path).await.map_err(|e| {
                    PlatformWalletError::AssetLockTransaction(format!(
                        "Failed to derive identity top-up account xpub for index {} via signer: {}",
                        identity_index, e
                    ))
                })?;
                wallet
                    .add_account(account_type, Some(account_xpub))
                    .map_err(|e| {
                        PlatformWalletError::AssetLockTransaction(format!(
                            "Failed to add identity top-up account for index {}: {}",
                            identity_index, e
                        ))
                    })?;
            }
        }

        // (b) managed side — mirror the account (keys-bearing, with its
        //     address pool initialized from the xpub) into
        //     `ManagedWalletInfo.accounts.identity_topup`.
        let created_managed_side = !info
            .core_wallet
            .accounts
            .identity_topup
            .contains_key(&identity_index);
        if created_managed_side {
            info.add_managed_account(wallet, account_type)
                .map_err(|e| {
                    PlatformWalletError::AssetLockTransaction(format!(
                        "Failed to register managed identity top-up account for index {}: {}",
                        identity_index, e
                    ))
                })?;
        }

        if !(created_xpub_side || created_managed_side) {
            return Ok(());
        }

        // (c) persist the new account as an `AccountRegistrationEntry`
        //     + initial pool snapshot(s) — the only record the load
        //     path can rebuild the account from (see the method docs).
        let account_xpub = wallet
            .accounts
            .identity_topup
            .get(&identity_index)
            .map(|a| a.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Identity top-up account for index {} missing after insert",
                    identity_index
                ))
            })?;
        let mut cs = PlatformWalletChangeSet {
            account_registrations: vec![AccountRegistrationEntry {
                account_type,
                account_xpub,
            }],
            ..Default::default()
        };
        if let Some(managed) = info
            .core_wallet
            .accounts
            .identity_topup
            .get(&identity_index)
        {
            cs.account_address_pools = crate::changeset::account_address_pool_entries(
                account_type,
                managed.managed_account_type().address_pools(),
            );
        }
        if let Err(e) = self.persister.store(cs) {
            // Roll back whichever sides this call inserted: a resident
            // but unpersisted account would make every retry hit the
            // `contains_*` guards above and skip the persist forever.
            if created_xpub_side {
                wallet.accounts.identity_topup.remove(&identity_index);
            }
            if created_managed_side {
                info.core_wallet
                    .accounts
                    .identity_topup
                    .remove(&identity_index);
            }
            return Err(PlatformWalletError::Persistence(format!(
                "Failed to persist identity top-up account registration for index {}: {}",
                identity_index, e
            )));
        }

        Ok(())
    }

    /// Persist the asset-lock funding accounts' address-pool snapshots so a
    /// consumed `funding_index` survives an app restart.
    ///
    /// The `IdentityRegistration` / `IdentityTopUp` / `IdentityInvitation` /
    /// asset-lock-top-up accounts fund credit outputs that live only in an
    /// asset-lock special-tx payload; the on-chain output is an `OP_RETURN`
    /// burn, so these addresses never appear as UTXOs and SPV can never
    /// rediscover their used indices. Without persisting the pool, the
    /// in-memory `mark_used` is lost on restart and `next_unused` resets to 0 —
    /// which for `IdentityInvitation` reuses the EXPORTED one-time voucher key
    /// across invitations (a bearer-key reuse: one leaked link could then claim
    /// every same-key invite). The pool round-trips through the existing
    /// `account_address_pools` persist path and is rebuilt by
    /// `restore_address_pool` on load. Funds accounts are skipped — they already
    /// persist their pools via the normal address-sync path. Best-effort.
    ///
    /// The snapshot re-acquires a read lock after the build's write lock is
    /// released, so callers must serialize asset-lock builds per wallet (the app
    /// creates invitations one-at-a-time from the UI); two concurrent builds on
    /// one wallet could otherwise persist a stale snapshot that drops the higher
    /// burned index — self-healing on the next build, but a residual to respect.
    async fn persist_asset_lock_account_pools(
        &self,
    ) -> Result<(), crate::changeset::PersistenceError> {
        use crate::changeset::{AccountAddressPoolEntry, PlatformWalletChangeSet};
        use key_wallet::account::AccountType;

        let entries: Vec<AccountAddressPoolEntry> = {
            let wm = self.wallet_manager.read().await;
            let Some(wallet_info) = wm.get_wallet_info(&self.wallet_id) else {
                return Ok(());
            };
            wallet_info
                .core_wallet
                .all_managed_accounts()
                .iter()
                .filter(|managed| {
                    matches!(
                        managed.managed_account_type().to_account_type(),
                        AccountType::IdentityRegistration
                            | AccountType::IdentityTopUp { .. }
                            | AccountType::IdentityTopUpNotBoundToIdentity
                            | AccountType::IdentityInvitation
                            | AccountType::AssetLockAddressTopUp
                            | AccountType::AssetLockShieldedAddressTopUp
                    )
                })
                .flat_map(|managed| {
                    let account_type = managed.managed_account_type().to_account_type();
                    crate::changeset::account_address_pool_entries(
                        account_type,
                        managed.managed_account_type().address_pools(),
                    )
                })
                .collect()
        };

        if entries.is_empty() {
            return Ok(());
        }
        self.persister.store(PlatformWalletChangeSet {
            account_address_pools: entries,
            ..Default::default()
        })
    }

    /// Build, broadcast, and wait for an asset lock proof.
    ///
    /// This is the **unified** entry point for obtaining a funded asset lock
    /// proof, replacing the earlier `create_registration_asset_lock_proof` and
    /// `create_topup_asset_lock_proof` methods.
    ///
    /// ## Flow
    ///
    /// 1. Build the asset lock transaction via the key-wallet
    ///    signer-driven builder.
    /// 2. Track the lifecycle as `Built` (in-memory).
    /// 3. Broadcast the transaction.
    /// 4. Wait for an InstantLock or ChainLock proof via the event channel.
    /// 5. Track the lifecycle as `InstantSendLocked` or `ChainLocked`.
    /// 6. Return `(proof, credit_output_derivation_path, txid)` — the
    ///    caller hands the path back to the same `signer` when
    ///    consuming the credit on Platform.
    ///
    /// ## Persistence
    ///
    /// This method tracks the asset lock in memory before broadcasting, so
    /// the lock is recoverable even if the proof wait is interrupted. However,
    /// the `AssetLockManager` does not persist state directly — **callers MUST
    /// persist the wallet state** after this method returns (or after broadcast
    /// if crash-safety before finality is required). The changeset system
    /// (`AssetLockChangeSet`) will capture the tracked lock state when the
    /// persister flushes.
    ///
    /// ## Parameters
    ///
    /// * `amount_duffs` — Amount to lock.
    /// * `account_index` — BIP44 account index to select UTXOs from.
    /// * `funding_type` — Which account to derive the one-time key from.
    /// * `identity_index` — HD identity index (for `IdentityTopUp`, this is
    ///   the registration index identifying which identity is being topped up).
    /// * `signer` — External ECDSA signer (Swift Keychain-backed in
    ///   production via `MnemonicResolverCoreSigner`).
    pub async fn create_funded_asset_lock_proof<S: ExtendedPubKeySigner>(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath, OutPoint), PlatformWalletError> {
        let (path, out_point) = self
            .broadcast_funded_asset_lock(
                amount_duffs,
                account_index,
                funding_type,
                identity_index,
                signer,
            )
            .await?;
        let proof = self
            .wait_for_funded_asset_lock_proof(&out_point, account_index)
            .await?;
        Ok((proof, path, out_point))
    }

    /// Broadcast half of [`Self::create_funded_asset_lock_proof`] — steps 1–4:
    /// build + fund the asset-lock transaction, persist the funding account's
    /// address pool, track the lifecycle row, and broadcast. Returns as soon as
    /// the transaction is on the wire (status `Broadcast`), BEFORE any proof
    /// wait, so a caller can durably record its own bookkeeping for the funded
    /// lock (e.g. the inviter-side invitation row) between the broadcast and
    /// the potentially long proof wait in
    /// [`Self::wait_for_funded_asset_lock_proof`].
    pub(crate) async fn broadcast_funded_asset_lock<S: ExtendedPubKeySigner>(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(DerivationPath, OutPoint), PlatformWalletError> {
        // Serialize build→persist so a concurrent build cannot interleave its
        // pool snapshot with ours. The snapshot is collected from live wallet
        // state at persist time; without this guard, build A's snapshot
        // (missing B's just-marked index) can be persisted AFTER B's,
        // rolling the durable used-index state back — after a restart the
        // next invitation would re-select B's index and re-export the same
        // bearer voucher key. Held through the persist/flush gate below and
        // dropped before the broadcast (only snapshot ordering needs
        // serializing; the UI's own single-flight guard is NOT sufficient —
        // a dismissed sheet's unstructured task keeps running).
        // Test-only occupancy gauge for the serialization gate (see
        // `build_serial_gate`). RAII so every exit path — including the
        // pre-broadcast aborts below — decrements.
        #[cfg(test)]
        struct GateGauge<'a>(&'a std::sync::atomic::AtomicUsize);
        #[cfg(test)]
        impl Drop for GateGauge<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        #[cfg(test)]
        let _gate_gauge = {
            self.build_serial_gate
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            GateGauge(&self.build_serial_gate)
        };
        let build_persist_guard = self.build_persist_serial.lock().await;

        // 1. Build the asset lock transaction.
        let (tx, path) = self
            .build_asset_lock_transaction(
                amount_duffs,
                account_index,
                funding_type,
                identity_index,
                signer,
            )
            .await?;

        let txid = tx.txid();
        let out_point = OutPoint::new(txid, 0);

        // Persist the funding account's address pool now that the build marked
        // its index used. These asset-lock accounts fund OP_RETURN-payload
        // credit outputs that never appear as on-chain UTXOs, so SPV can never
        // rediscover the used index — the persisted pool is the only thing that
        // carries `funding_index` across a restart. For an INVITATION this write
        // is a security gate: the voucher key is exported into a bearer link, so
        // a failed persist would let the next restart reuse this index/key.
        // `store()` alone is only a buffer hint under the persistence contract
        // (backends may defer I/O until `flush`), so the invitation gate also
        // drives `flush()` — the contract's durability boundary — before
        // anything hits the wire.
        // Aborting BEFORE broadcast is harmless (no tx on the wire); the other
        // asset-lock accounts keep their keys on-device, so they stay best-effort.
        let pool_durability = match self.persist_asset_lock_account_pools().await {
            Ok(()) if funding_type == AssetLockFundingType::IdentityInvitation => {
                self.persister.flush()
            }
            other => other,
        };
        if let Err(e) = pool_durability {
            tracing::error!(error = %e, "failed to persist asset-lock funding index");
            if funding_type == AssetLockFundingType::IdentityInvitation {
                return Err(PlatformWalletError::AssetLockTransaction(format!(
                    "aborted before broadcast: could not durably record the invitation \
                     funding index (broadcasting anyway would risk voucher-key reuse on \
                     restart): {e}"
                )));
            }
        }

        // The durable snapshot now includes this build's index; broadcast and
        // everything after it can safely run concurrently with the next build.
        drop(build_persist_guard);

        // 2. Track as Built and queue the changeset onto the persister
        //    so a crash after broadcast leaves a row we can recover from.
        let cs_built = self
            .track_asset_lock(TrackedAssetLock {
                out_point,
                transaction: tx.clone(),
                account_index,
                funding_type,
                identity_index,
                amount: amount_duffs,
                status: AssetLockStatus::Built,
                proof: None,
            })
            .await;
        self.queue_asset_lock_changeset(cs_built);

        tracing::debug!(
            %txid,
            "Asset lock tracked as Built and queued for persistence; broadcasting."
        );

        // 3. Broadcast. On a definitive pre-send rejection, untrack the
        //    `Built` row BEFORE releasing the funding reservation (the
        //    asset-lock builder funds from the BIP44 account at
        //    `account_index`): while the reservation is held the inputs
        //    cannot be re-selected by a new build, and once the row is gone
        //    `resume_asset_lock` can no longer re-drive the rejected
        //    transaction — so at no point is the row resumable while its
        //    inputs are re-spendable. A `MaybeSent` failure keeps both the
        //    reservation and the resumable row.
        if let Err(e) = self.broadcaster.broadcast(&tx).await {
            if matches!(e, crate::broadcaster::BroadcastError::Rejected { .. }) {
                let cs_untrack = self.untrack_asset_lock(&out_point).await;
                // Release only when the Built row was actually removed. If
                // the untrack guard fired instead — a concurrent
                // `resume_asset_lock` advanced the row past `Built`, positive
                // evidence the transaction reached the network after all —
                // the inputs must stay reserved exactly like a `MaybeSent`
                // outcome, or the still-tracked row would be resumable while
                // its inputs are re-spendable.
                let removed_built_row = cs_untrack.removed.contains(&out_point);
                self.queue_asset_lock_changeset(cs_untrack);
                if removed_built_row {
                    crate::wallet::reservations::release_reservation_after_rejected_broadcast(
                        &self.wallet_manager,
                        &self.wallet_id,
                        key_wallet::account::account_type::StandardAccountType::BIP44Account,
                        account_index,
                        &tx,
                    )
                    .await;
                }
            }
            return Err(e.into());
        }

        // 4. Transition to Broadcast and queue the changeset.
        let cs_broadcast = self
            .advance_asset_lock_status(&out_point, AssetLockStatus::Broadcast, None)
            .await?;
        self.queue_asset_lock_changeset(cs_broadcast);

        Ok((path, out_point))
    }

    /// Proof half of [`Self::create_funded_asset_lock_proof`] — steps 5–6:
    /// wait for the InstantSend/ChainLock proof of an already-broadcast asset
    /// lock, upgrade it when Platform would reject it, and attach it to the
    /// tracked row.
    pub(crate) async fn wait_for_funded_asset_lock_proof(
        &self,
        out_point: &OutPoint,
        account_index: u32,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        // 5. Wait for proof via SPV events. The 300s bound is an
        //    InstantSend-preference window, NOT a finality timeout: on
        //    expiry the resolver falls back to an unbounded ChainLock wait
        //    (`upgrade_to_chain_lock_proof(None)`), so a broadcast lock is
        //    never surfaced as "failed" just because IS was slow.
        let proof = self
            .wait_for_proof(out_point, Some(Duration::from_secs(300)))
            .await?;

        // 5b. If we got an IS-lock proof, check whether the transaction is
        // old enough that Platform might reject it. If so, upgrade to a
        // ChainLock proof proactively.
        let proof = self
            .validate_or_upgrade_proof(proof, account_index, out_point)
            .await?;

        // 6. Attach proof — status matches the proof type received —
        //    and queue the final changeset.
        let status = match &proof {
            dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
            dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
        };
        let cs_final = self
            .advance_asset_lock_status(out_point, status, Some(proof.clone()))
            .await?;
        self.queue_asset_lock_changeset(cs_final);

        Ok(proof)
    }
}

/// Map a key-wallet [`BuilderError`] to a [`PlatformWalletError`], promoting
/// every shortfall shape to the typed
/// [`PlatformWalletError::AssetLockInsufficientFunds`] so callers get one
/// structured shortfall contract (dashpay/platform#4073's typed-error ask)
/// instead of a string they must pattern-match:
///   - `BuilderError::InsufficientFunds` / `SelectionError::InsufficientFunds`
///     carry their own exact `available`/`required` duff amounts — preserved.
///   - `SelectionError::NoUtxosAvailable` — the zero-spendable-candidate case,
///     the MOST extreme shortfall — carries no amounts, so it previously fell
///     through to the generic string form while partial shortfalls stayed typed
///     (dashpay/platform#4074 prior-no-utxos-846). It now maps to `available: 0`
///     with the caller's `requested` target as `required`, keeping the empty
///     candidate set on the same structured path.
/// Every other builder error keeps the generic `AssetLockTransaction` string.
fn map_builder_error(e: BuilderError, requested: u64) -> PlatformWalletError {
    match e {
        BuilderError::InsufficientFunds {
            available,
            required,
        }
        | BuilderError::CoinSelection(SelectionError::InsufficientFunds {
            available,
            required,
        }) => PlatformWalletError::AssetLockInsufficientFunds {
            available,
            required,
        },
        BuilderError::CoinSelection(SelectionError::NoUtxosAvailable) => {
            PlatformWalletError::AssetLockInsufficientFunds {
                available: 0,
                required: requested,
            }
        }
        other => {
            PlatformWalletError::AssetLockTransaction(format!("Asset lock builder failed: {other}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use dashcore::OutPoint;
    use key_wallet::account::account_type::StandardAccountType;
    use tokio::sync::Notify;

    use async_trait::async_trait;
    use dashcore::{Transaction, Txid};
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::test_support::{
        funded_wallet_manager, AlwaysMaybeSentBroadcaster, AlwaysOkBroadcaster,
        AlwaysRejectedBroadcaster, DashpayLeg, WalletSigner,
    };
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::asset_lock::tracked::AssetLockStatus;
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use crate::wallet::platform_wallet::WalletId;
    use crate::{AssetLockFundingType, PlatformWalletError};

    /// prior-no-utxos-846 (dashpay/platform#4074): the zero-spendable-candidate
    /// selection error must surface the SAME typed shortfall as a partial
    /// shortfall (not the generic string), so hosts stay on one structured path;
    /// and a partial shortfall must still carry its own exact amounts.
    #[test]
    fn no_utxos_available_maps_to_typed_insufficient_funds() {
        use super::{map_builder_error, BuilderError, SelectionError};

        // Zero spendable candidates -> typed, available: 0, required = requested.
        match map_builder_error(
            BuilderError::CoinSelection(SelectionError::NoUtxosAvailable),
            12_345,
        ) {
            PlatformWalletError::AssetLockInsufficientFunds {
                available,
                required,
            } => {
                assert_eq!(available, 0, "empty candidate set means nothing available");
                assert_eq!(
                    required, 12_345,
                    "requested target threaded through as required"
                );
            }
            other => panic!("expected typed AssetLockInsufficientFunds, got {other:?}"),
        }

        // A partial shortfall keeps its own exact amounts; the requested arg is
        // NOT substituted for the builder's carried values.
        match map_builder_error(
            BuilderError::CoinSelection(SelectionError::InsufficientFunds {
                available: 100,
                required: 500,
            }),
            999,
        ) {
            PlatformWalletError::AssetLockInsufficientFunds {
                available,
                required,
            } => {
                assert_eq!(available, 100);
                assert_eq!(required, 500, "carried amounts win over the requested arg");
            }
            other => panic!("expected typed AssetLockInsufficientFunds, got {other:?}"),
        }
    }

    /// Persistence stub that records every stored changeset so tests can
    /// assert what the asset-lock flow queued. `fail_flush` simulates a
    /// backend whose durability boundary fails; `flushes` counts `flush`
    /// calls so tests can assert the invitation gate drove one.
    #[derive(Default)]
    struct CapturingPersistence {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
        flushes: std::sync::atomic::AtomicUsize,
        fail_flush: bool,
    }

    impl CapturingPersistence {
        /// Outpoints queued for persisted-row deletion across all stored
        /// changesets.
        fn removed_outpoints(&self) -> Vec<OutPoint> {
            self.stored
                .lock()
                .expect("capturing persistence mutex")
                .iter()
                .filter_map(|cs| cs.asset_locks.as_ref())
                .flat_map(|al| al.removed.iter().copied())
                .collect()
        }
    }

    impl PlatformWalletPersistence for CapturingPersistence {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stored
                .lock()
                .expect("capturing persistence mutex")
                .push(changeset);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            self.flushes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.fail_flush {
                return Err(PersistenceError::backend("simulated flush failure"));
            }
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// Builds an `AssetLockManager` over the shared BIP44-funded fixture.
    async fn funded_asset_lock_manager<B: TransactionBroadcaster>(
        broadcaster: Arc<B>,
    ) -> (
        Arc<AssetLockManager<B>>,
        WalletSigner,
        Arc<CapturingPersistence>,
    ) {
        let persistence = Arc::new(CapturingPersistence::default());
        let (manager, signer) =
            funded_asset_lock_manager_with_persistence(broadcaster, Arc::clone(&persistence)).await;
        (manager, signer, persistence)
    }

    /// Like [`funded_asset_lock_manager`] but over a caller-built persistence
    /// stub (e.g. one with `fail_flush` set).
    async fn funded_asset_lock_manager_with_persistence<B: TransactionBroadcaster>(
        broadcaster: Arc<B>,
        persistence: Arc<CapturingPersistence>,
    ) -> (Arc<AssetLockManager<B>>, WalletSigner) {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            WalletPersister::new(wallet_id, persistence as Arc<dyn PlatformWalletPersistence>),
        ));

        (manager, signer)
    }

    /// Regression: a build must persist the funding account's address-pool
    /// snapshot with the newly-used index. The asset-lock funding accounts fund
    /// OP_RETURN-payload credit outputs that never appear as on-chain UTXOs, so
    /// SPV can't rediscover the used index — the persisted pool is the only
    /// thing that carries `funding_index` across a restart. Before the fix the
    /// pool was never emitted, so `funding_index` reset to 0 each launch and (for
    /// `IdentityInvitation`) the EXPORTED one-time voucher key was reused across
    /// invitations. The pool snapshot is emitted right after the tx is built,
    /// before broadcast, so a rejected broadcast still exercises it.
    #[tokio::test]
    async fn asset_lock_build_persists_funding_account_used_index() {
        use key_wallet::account::AccountType;

        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysRejectedBroadcaster)).await;

        let _ = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await;

        let stored = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex");
        let persisted_invitation_used = stored.iter().any(|cs| {
            cs.account_address_pools.iter().any(|entry| {
                matches!(entry.account_type, AccountType::IdentityInvitation)
                    && entry.addresses.iter().any(|a| a.used)
            })
        });
        assert!(
            persisted_invitation_used,
            "a build must persist the IdentityInvitation account's pool with the used \
             funding index; without it funding_index resets on restart and the exported \
             voucher key is reused across invitations"
        );
    }

    /// A definitively rejected asset-lock broadcast must untrack the `Built`
    /// row (in-memory and via the changeset's `removed` set) and release the
    /// funding reservation, so nothing can resume the dead transaction and a
    /// fresh funding attempt can reselect the inputs immediately.
    #[tokio::test]
    async fn rejected_asset_lock_broadcast_untracks_row_and_releases_reservation() {
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysRejectedBroadcaster)).await;

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "rejected broadcast should surface as TransactionBroadcast, got {result:?}"
        );

        // The Built row is gone in memory…
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            assert!(
                info.tracked_asset_locks.is_empty(),
                "rejected lock must be untracked, got {:?}",
                info.tracked_asset_locks
            );
        }
        // …and its persisted row was queued for deletion.
        assert_eq!(
            persistence.removed_outpoints().len(),
            1,
            "exactly the rejected lock's outpoint should be queued as removed"
        );

        // The funding reservation was released: a fresh build over the same
        // single-UTXO wallet can reselect the inputs immediately.
        let rebuild = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            rebuild.is_ok(),
            "rebuild after a rejected broadcast should reselect the released \
             inputs, got {rebuild:?}"
        );
    }

    /// An *ambiguous* asset-lock broadcast failure must keep both the funding
    /// reservation and the resumable `Built` row: the transaction may already
    /// be propagating, so a retry must not double-spend and a resume must
    /// stay possible.
    #[tokio::test]
    async fn ambiguous_asset_lock_broadcast_keeps_reservation_and_built_row() {
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysMaybeSentBroadcaster)).await;

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
            ),
            "ambiguous broadcast should surface as TransactionBroadcastUnconfirmed, got {result:?}"
        );

        // The Built row survives for a later resume…
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            assert_eq!(info.tracked_asset_locks.len(), 1);
            let lock = info.tracked_asset_locks.values().next().expect("built row");
            assert_eq!(lock.status, AssetLockStatus::Built);
        }
        // …no persisted-row deletion was queued…
        assert!(
            persistence.removed_outpoints().is_empty(),
            "ambiguous failure must not queue a row deletion"
        );

        // …and the reservation is kept: a fresh build cannot reselect the
        // single reserved UTXO and fails at input selection.
        let rebuild = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(rebuild, Err(PlatformWalletError::AssetLockTransaction(_))),
            "rebuild must fail at input selection while the reservation is \
             kept, got {rebuild:?}"
        );
    }

    /// Broadcaster that simulates the racing interleave the release gate
    /// exists for: "during" the broadcast a concurrent `resume_asset_lock`
    /// advances the tracked row to `Broadcast`, then the original call still
    /// comes back `Rejected`. The advanced row is positive evidence the
    /// transaction reached the network, so the cleanup must keep it AND keep
    /// the funding reservation.
    struct RejectAfterConcurrentResumeBroadcaster {
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
    }

    #[async_trait]
    impl TransactionBroadcaster for RejectAfterConcurrentResumeBroadcaster {
        async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, BroadcastError> {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet present");
            let lock = info
                .tracked_asset_locks
                .values_mut()
                .next()
                .expect("Built row tracked before broadcast");
            lock.status = AssetLockStatus::Broadcast;
            drop(wm);
            Err(BroadcastError::Rejected {
                reason: "simulated rejection racing a concurrent resume".to_string(),
            })
        }
    }

    /// If a concurrent resume advanced the row past `Built` in the rejection
    /// window, the cleanup must keep the row (guard) AND keep the funding
    /// reservation (release gate) — otherwise the still-tracked transaction
    /// would be resumable while its inputs are re-spendable.
    #[tokio::test]
    async fn rejected_broadcast_racing_concurrent_resume_keeps_row_and_reservation() {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;

        let broadcaster = Arc::new(RejectAfterConcurrentResumeBroadcaster {
            wallet_manager: Arc::clone(&wallet_manager),
            wallet_id,
        });
        let persistence = Arc::new(CapturingPersistence::default());
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "rejection should still surface, got {result:?}"
        );

        // The concurrently-advanced row survives the cleanup…
        {
            let wm = wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            assert_eq!(info.tracked_asset_locks.len(), 1);
            let lock = info.tracked_asset_locks.values().next().expect("row kept");
            assert_eq!(lock.status, AssetLockStatus::Broadcast);
        }
        // …no persisted-row deletion was queued…
        assert!(
            persistence.removed_outpoints().is_empty(),
            "advanced row must not be queued for deletion"
        );

        // …and the reservation was NOT released: a fresh build cannot
        // reselect the single reserved UTXO.
        let rebuild = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(rebuild, Err(PlatformWalletError::AssetLockTransaction(_))),
            "rebuild must fail at input selection while the reservation is \
             kept for the advanced row, got {rebuild:?}"
        );
    }

    /// Persistence stub whose FIRST address-pool store blocks on a 2-party
    /// barrier until the test arrives, holding that build inside its persist
    /// while the other build runs. Later stores pass straight through.
    struct GatedPoolPersistence {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
        first_pool_store: std::sync::Barrier,
        gate_used: std::sync::atomic::AtomicBool,
        /// Total pool-bearing `store` calls seen (counted before parking or
        /// pushing). Reaching 2 while the first store is parked proves the
        /// second build persisted concurrently — the exact regression.
        pool_stores_seen: std::sync::atomic::AtomicUsize,
        /// Set just before the first pool store parks at the barrier, so the
        /// test can spawn the second build only once the first is provably
        /// inside its persist.
        first_parked: std::sync::atomic::AtomicBool,
    }

    impl PlatformWalletPersistence for GatedPoolPersistence {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if !changeset.account_address_pools.is_empty() {
                self.pool_stores_seen
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if !self
                    .gate_used
                    .swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    self.first_parked
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    self.first_pool_store.wait();
                }
            }
            self.stored
                .lock()
                .expect("gated persistence mutex")
                .push(changeset);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// Two concurrent invitation builds must not be able to roll the durable
    /// used-index snapshot backwards. The pool snapshot is collected from live
    /// wallet state at persist time; unserialized, build A's snapshot
    /// (collected before B marked its index) can be persisted AFTER B's, so
    /// the last durable snapshot loses B's index — after a restart the next
    /// invitation re-selects it and re-exports the same bearer voucher key.
    /// The barrier holds the first-persisting build inside its store while
    /// the other runs: without the
    /// build→persist serialization, B's fuller snapshot lands first and A's
    /// stale one overwrites it (this test is red); with it, B parks until A's
    /// persist completes, so snapshots are monotonic.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_invitation_builds_cannot_roll_back_the_used_index_snapshot() {
        use key_wallet::account::AccountType;

        let (wallet_manager, wallet_id, _balance, signer) =
            crate::test_support::funded_wallet_manager_with_outputs(
                StandardAccountType::BIP44Account,
                &[10_000_000, 10_000_000],
            )
            .await;

        let persistence = Arc::new(GatedPoolPersistence {
            stored: Mutex::new(Vec::new()),
            first_pool_store: std::sync::Barrier::new(2),
            gate_used: std::sync::atomic::AtomicBool::new(false),
            pool_stores_seen: std::sync::atomic::AtomicUsize::new(0),
            first_parked: std::sync::atomic::AtomicBool::new(false),
        });
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysOkBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));

        let manager_a = Arc::clone(&manager);
        let signer_a = signer.clone();
        let a = tokio::spawn(async move {
            manager_a
                .broadcast_funded_asset_lock(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityInvitation,
                    0,
                    &signer_a,
                )
                .await
        });

        // Spawn B only once A is provably parked inside its pool persist
        // (holding `build_persist_serial`), so the interleaving is staged,
        // not scheduled.
        while !persistence
            .first_parked
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        let manager_b = Arc::clone(&manager);
        let b = tokio::spawn(async move {
            manager_b
                .broadcast_funded_asset_lock(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityInvitation,
                    0,
                    &signer,
                )
                .await
        });

        // Release A only after B has provably reached the relevant stage —
        // no scheduling assumption. Exactly one of two states must occur:
        // - `pool_stores_seen >= 2`: B built and persisted its own (fuller)
        //   snapshot while A was parked — the regression manifested (an
        //   unserialized implementation always reaches this state, however
        //   slowly, so the rollback assertion below fires deterministically);
        // - `build_serial_gate >= 2`: B is queued at the build→persist
        //   serialization gate while A still holds it, so B cannot have
        //   collected a snapshot yet — the fixed behavior, verified
        //   positively rather than by the absence of a store within a delay.
        loop {
            let regressed = persistence
                .pool_stores_seen
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 2;
            let serialized = manager
                .build_serial_gate
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 2;
            if regressed || serialized {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        persistence.first_pool_store.wait();

        a.await.expect("join A").expect("build A succeeds");
        b.await.expect("join B").expect("build B succeeds");

        // Successive persisted invitation-pool snapshots must never lose a
        // used index, and both builds' indices must end up durably used.
        let stored = persistence.stored.lock().expect("gated persistence mutex");
        let mut last_used = 0usize;
        for cs in stored.iter() {
            for entry in cs
                .account_address_pools
                .iter()
                .filter(|e| matches!(e.account_type, AccountType::IdentityInvitation))
            {
                let used = entry.addresses.iter().filter(|a| a.used).count();
                assert!(
                    used >= last_used,
                    "invitation pool snapshot rolled back: {used} used after {last_used}"
                );
                last_used = used;
            }
        }
        assert!(
            last_used >= 2,
            "both builds' funding indices must be durably marked used, got {last_used}"
        );
    }

    /// The invitation pre-broadcast gate must treat `flush()` — the
    /// persistence contract's durability boundary — as part of recording the
    /// funding index, and abort BEFORE broadcast when it fails. `store()`
    /// alone may only buffer; an unflushed funding index can be re-selected
    /// after a restart, re-exporting the same bearer voucher key.
    #[tokio::test]
    async fn invitation_gate_aborts_before_broadcast_when_flush_fails() {
        let persistence = Arc::new(CapturingPersistence {
            fail_flush: true,
            ..Default::default()
        });
        // The broadcaster rejects loudly: reaching it at all would surface as
        // a `TransactionBroadcast` error, so the gate's own "aborted before
        // broadcast" message proves nothing hit the wire.
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(AlwaysRejectedBroadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await;
        match result {
            Err(PlatformWalletError::AssetLockTransaction(msg)) => assert!(
                msg.contains("aborted before broadcast"),
                "expected the pre-broadcast durability abort, got: {msg}"
            ),
            other => panic!("expected the pre-broadcast durability abort, got {other:?}"),
        }
        assert!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 1,
            "the invitation gate must have driven flush()"
        );
    }

    /// Non-invitation funding types stay best-effort: their one-time keys
    /// never leave the device, so a failing durability boundary must NOT gate
    /// them — the flow proceeds to broadcast.
    #[tokio::test]
    async fn flush_failure_does_not_gate_non_invitation_funding() {
        let persistence = Arc::new(CapturingPersistence {
            fail_flush: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(AlwaysRejectedBroadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "a registration build must reach the broadcaster despite the flush \
             failure (best-effort persistence), got {result:?}"
        );
    }

    /// The broadcast half returns as soon as the transaction is on the wire:
    /// the tracked row is `Broadcast` (recoverable/resumable) and the
    /// invitation funding pool was persisted AND flushed — all BEFORE any
    /// proof wait (the test completing at all proves no SPV wait ran), so a
    /// caller can durably record its own bookkeeping for the funded lock
    /// between the broadcast and the proof wait.
    #[tokio::test]
    async fn broadcast_half_leaves_broadcast_row_and_flushed_pool() {
        let persistence = Arc::new(CapturingPersistence::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(AlwaysOkBroadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let (_path, out_point) = manager
            .broadcast_funded_asset_lock(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await
            .expect("broadcast half should succeed");

        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            let lock = info
                .tracked_asset_locks
                .get(&out_point)
                .expect("broadcast lock must stay tracked");
            assert_eq!(
                lock.status,
                AssetLockStatus::Broadcast,
                "the broadcast half must stop at Broadcast (no proof attached)"
            );
        }
        assert!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 1,
            "the invitation funding pool must be flushed before broadcast"
        );
    }

    /// An `IdentityInvitation`-typed lock is a shared bearer voucher: the
    /// funding resolver must refuse to consume it through the generic
    /// `FromExistingAssetLock` path (no explicit authorization), and must
    /// let the explicitly-authorized reclaim variant past the gate. Consuming
    /// a voucher generically would both misdirect the funds into an unrelated
    /// local identity and invalidate the invitee's already-shared claim.
    #[tokio::test]
    async fn generic_resume_refuses_invitation_voucher_locks() {
        use crate::wallet::asset_lock::orchestration::AssetLockFunding;

        let persistence = Arc::new(CapturingPersistence::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(AlwaysOkBroadcaster),
            Arc::clone(&persistence),
        )
        .await;

        // A real tracked invitation voucher, stopped at Broadcast (the
        // broadcast half never attaches a proof).
        let (_path, out_point) = manager
            .broadcast_funded_asset_lock(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await
            .expect("invitation broadcast half succeeds");

        // Unauthorized (generic) consume: refused by the gate, immediately.
        let refused = manager
            .resolve_funding_with_is_timeout_fallback(
                AssetLockFunding::FromExistingAssetLock {
                    out_point,
                    consume_invitation_voucher: false,
                },
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        match refused {
            Err(PlatformWalletError::AssetLockFundingMismatch {
                actual_funding_type: AssetLockFundingType::IdentityInvitation,
                ..
            }) => {}
            Err(e) => panic!("expected the voucher-refusal error, got {e:?}"),
            Ok(_) => panic!("expected the voucher-refusal error, got Ok(..)"),
        }

        // Authorized (reclaim) consume: passes the gate. The lock has no
        // proof yet, so the resolver proceeds into the proof wait — getting
        // parked there (rather than an immediate refusal) is the positive
        // signal that the gate admitted the call.
        for reclaim_target in [
            AssetLockFundingType::IdentityRegistration,
            AssetLockFundingType::IdentityTopUp,
        ] {
            let authorized = tokio::time::timeout(
                std::time::Duration::from_millis(500),
                manager.resolve_funding_with_is_timeout_fallback(
                    AssetLockFunding::FromExistingAssetLock {
                        out_point,
                        consume_invitation_voucher: true,
                    },
                    reclaim_target,
                    0,
                    &signer,
                ),
            )
            .await;
            match authorized {
                Err(_elapsed) => {} // parked in the proof wait — past the gate
                Ok(Err(PlatformWalletError::AssetLockFundingMismatch { .. })) => {
                    panic!("authorized {reclaim_target:?} reclaim must pass the voucher gate")
                }
                Ok(other) => {
                    // Any other outcome also proves the gate admitted the call.
                    drop(other);
                }
            }
        }
    }

    // -- Multi-account asset-lock funding (dashpay/platform#4073) --

    /// Wraps the split BIP44 + CoinJoin fixture in an `AssetLockManager`.
    /// `build_asset_lock_transaction` never broadcasts, so the broadcaster is
    /// irrelevant here.
    async fn split_asset_lock_manager(
        bip44_duffs: u64,
        coinjoin_duffs: u64,
    ) -> (
        Arc<AssetLockManager<AlwaysRejectedBroadcaster>>,
        WalletSigner,
    ) {
        let (wallet_manager, wallet_id, signer) =
            crate::test_support::split_funded_wallet_manager(bip44_duffs, coinjoin_duffs).await;
        let persistence = Arc::new(CapturingPersistence::default());
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));
        (manager, signer)
    }

    /// Wraps the many-CoinJoin-UTXO fixture in an `AssetLockManager`.
    async fn split_asset_lock_manager_many_coinjoin(
        bip44_duffs: u64,
        coinjoin_values: &[u64],
    ) -> (
        Arc<AssetLockManager<AlwaysRejectedBroadcaster>>,
        WalletSigner,
    ) {
        let (wallet_manager, wallet_id, signer) =
            crate::test_support::split_funded_wallet_manager_many_coinjoin(
                bip44_duffs,
                coinjoin_values,
            )
            .await;
        let persistence = Arc::new(CapturingPersistence::default());
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));
        (manager, signer)
    }

    /// The `(BIP44 account 0, CoinJoin account 0)` UTXO outpoint sets, so a
    /// test can prove a built transaction drew inputs from both accounts.
    async fn account_outpoints(
        manager: &AssetLockManager<AlwaysRejectedBroadcaster>,
    ) -> (
        std::collections::HashSet<OutPoint>,
        std::collections::HashSet<OutPoint>,
    ) {
        let wm = manager.wallet_manager.read().await;
        let (_, info) = wm
            .get_wallet_and_info(&manager.wallet_id)
            .expect("wallet present");
        let bip44 = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .map(|a| a.utxos.keys().copied().collect())
            .unwrap_or_default();
        let coinjoin = info
            .core_wallet
            .accounts
            .coinjoin_accounts
            .get(&0)
            .map(|a| a.utxos.keys().copied().collect())
            .unwrap_or_default();
        (bip44, coinjoin)
    }

    /// The bug: shielded asset-lock funding must be able to spend
    /// previously-mixed CoinJoin coins, not just the BIP44 slice. Split the
    /// balance so NEITHER account alone can fund the lock (0.09 DASH each) and
    /// require 0.15 DASH — coin selection must reach across both the BIP44 and
    /// the DIP-9 CoinJoin account, and the mixed inputs must each be signed
    /// under their own account's derivation path.
    #[tokio::test]
    async fn shielded_asset_lock_funds_from_bip44_and_coinjoin_union() {
        // 0.09 DASH on BIP44, 0.09 DASH on CoinJoin; require 0.15 DASH.
        let (manager, signer) = split_asset_lock_manager(9_000_000, 9_000_000).await;
        let (bip44_outpoints, coinjoin_outpoints) = account_outpoints(&manager).await;

        let (tx, _path) = manager
            .build_asset_lock_transaction(
                15_000_000,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await
            .expect("shielded asset lock must fund from the BIP44 + CoinJoin union");

        // Neither account alone covers 0.15 DASH, so both must be selected.
        let spent: std::collections::HashSet<OutPoint> =
            tx.input.iter().map(|i| i.previous_output).collect();
        assert!(
            spent.iter().any(|o| bip44_outpoints.contains(o)),
            "expected at least one BIP44 input, tx spent {spent:?}"
        );
        assert!(
            spent.iter().any(|o| coinjoin_outpoints.contains(o)),
            "expected at least one CoinJoin input (the #4073 fix), tx spent {spent:?}"
        );

        // Per-account signing: every selected input, regardless of which
        // account's derivation path it needed, must carry a signature.
        assert!(!tx.input.is_empty(), "asset lock must have selected inputs");
        for (i, txin) in tx.input.iter().enumerate() {
            assert!(
                !txin.script_sig.is_empty(),
                "input {i} ({}) has an empty script_sig — the cross-account \
                 resolver failed to derive/sign its key",
                txin.previous_output
            );
        }
    }

    /// The widening is deliberately scoped to shielded funding: spending mixed
    /// CoinJoin coins into an identity registration would de-anonymize them.
    /// With the balance split 0.09/0.09, an identity-registration lock for
    /// 0.15 DASH must still fail (BIP44 alone is short) while the shielded lock
    /// for the same amount succeeds from the union.
    #[tokio::test]
    async fn non_shielded_asset_lock_stays_single_bip44_account() {
        let (manager, signer) = split_asset_lock_manager(9_000_000, 9_000_000).await;

        let identity = manager
            .build_asset_lock_transaction(
                15_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            identity.is_err(),
            "identity registration must NOT reach CoinJoin coins — BIP44 alone \
             is short of 0.15 DASH, got {identity:?}"
        );

        let shielded = manager
            .build_asset_lock_transaction(
                15_000_000,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await;
        assert!(
            shielded.is_ok(),
            "shielded funding must reach the union, got {shielded:?}"
        );
    }

    /// A shielded lock exceeding even the UNION balance surfaces the typed
    /// [`PlatformWalletError::AssetLockInsufficientFunds`], and its `available`
    /// reflects the whole spendable balance (both accounts), not the BIP44
    /// slice — the pre-#4073 symptom was `available` reporting only the BIP44
    /// portion.
    #[tokio::test]
    async fn shielded_asset_lock_union_shortfall_is_typed() {
        // Union spendable is 0.18 DASH; ask for 1.0 DASH.
        let (manager, signer) = split_asset_lock_manager(9_000_000, 9_000_000).await;

        let result = manager
            .build_asset_lock_transaction(
                100_000_000,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await;

        match result {
            Err(PlatformWalletError::AssetLockInsufficientFunds {
                available,
                required,
            }) => {
                // `available` must reflect the union (both 0.09 UTXOs), i.e.
                // strictly more than the BIP44-only slice the old path saw.
                assert!(
                    available > 9_000_000,
                    "available ({available}) should reflect the BIP44 + CoinJoin \
                     union (> the 9_000_000 BIP44 slice)"
                );
                assert!(
                    available <= 18_000_000,
                    "available ({available}) cannot exceed the 18_000_000 union"
                );
                assert!(
                    required >= 100_000_000,
                    "required ({required}) should be at least the requested amount"
                );
            }
            other => panic!(
                "expected typed AssetLockInsufficientFunds carrying the union \
                 available/required, got {other:?}"
            ),
        }
    }

    /// On-device regression: a real CoinJoin account holds many small mixed
    /// denominations. The first version of the multi-account builder inherited
    /// `TransactionBuilder`'s default `BranchAndBound`, whose recursive
    /// exact-match subset-sum (`CoinSelector::find_exact_match`) is EXPONENTIAL
    /// in the count of sub-target UTXOs — feeding it a large CoinJoin set hung
    /// the whole FFI call for minutes with no logs and no broadcast. The builder
    /// now pins `LargestFirst` (linear greedy).
    ///
    /// The blowup is SYNCHRONOUS CPU work with no `.await` points, so it cannot
    /// be interrupted by `tokio::time::timeout` (that is exactly why on-device
    /// it hangs RUNNABLE-in-native and the enclosing coroutine never yields).
    /// The build therefore runs on a **detached OS thread**, and the test body
    /// waits on a channel with a wall-clock deadline: a regression to an
    /// exponential strategy makes `recv_timeout` fire and the test FAIL (rather
    /// than hang the whole suite). The detached thread is reclaimed at process
    /// exit; on the happy path (LargestFirst) it finishes in well under a
    /// millisecond and the channel delivers immediately.
    #[test]
    fn shielded_asset_lock_over_many_coinjoin_utxos_does_not_hang() {
        use std::sync::mpsc;
        use std::time::Duration;

        // 40 x 0.02 DASH CoinJoin UTXOs (0.8 DASH), 0.09 DASH on BIP44; shield
        // 0.2 DASH. BranchAndBound would explore ~sum_k C(40, k<=10) subsets —
        // empirically minutes+; LargestFirst returns instantly.
        let coinjoin: Vec<u64> = vec![2_000_000; 40];

        // Fixture build is async; drive it on a throwaway current-thread runtime.
        let setup_rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("setup runtime");
        let (manager, signer) =
            setup_rt.block_on(split_asset_lock_manager_many_coinjoin(9_000_000, &coinjoin));

        let (result_tx, result_rx) = mpsc::channel();
        // Detached: NOT joined anywhere, so a hung build can't wedge runtime
        // teardown; libtest reclaims it at process exit.
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("worker runtime");
            let outcome = rt
                .block_on(manager.build_asset_lock_transaction(
                    20_000_000,
                    0,
                    AssetLockFundingType::AssetLockShieldedAddressTopUp,
                    0,
                    &signer,
                ))
                .map(|(tx, _path)| tx);
            let _ = result_tx.send(outcome);
        });

        // LargestFirst completes in ~25ms; a 30s deadline is a ~1000x margin
        // against CI contention while still bounding a regression to an
        // exponential strategy (which never returns) to a prompt failure.
        match result_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(tx)) => {
                // LargestFirst minimizes the input count; every selected input
                // must be signed under its own account's derivation path.
                assert!(!tx.input.is_empty(), "must select inputs");
                for txin in &tx.input {
                    assert!(
                        !txin.script_sig.is_empty(),
                        "input {} is unsigned",
                        txin.previous_output
                    );
                }
            }
            Ok(Err(e)) => panic!("funding must succeed from the CoinJoin union, got {e:?}"),
            Err(_) => panic!(
                "multi-account asset-lock funding did not return within 30s — \
                 regression to an exponential coin-selection strategy over the \
                 CoinJoin UTXO set (dashpay/platform#4073 on-device hang)"
            ),
        }
    }

    /// Aggregate wallet balance across all funds accounts (recomputes from the
    /// current UTXO maps).
    async fn aggregate_total(manager: &AssetLockManager<AlwaysRejectedBroadcaster>) -> u64 {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        let mut wm = manager.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&manager.wallet_id)
            .expect("wallet present");
        info.core_wallet.update_balance();
        WalletInfoInterface::balance(&info.core_wallet).total()
    }

    /// `true` iff CoinJoin account 0 still holds `outpoint` as an unspent UTXO.
    async fn coinjoin_has_utxo(
        manager: &AssetLockManager<AlwaysRejectedBroadcaster>,
        outpoint: &OutPoint,
    ) -> bool {
        let wm = manager.wallet_manager.read().await;
        let (_, info) = wm
            .get_wallet_and_info(&manager.wallet_id)
            .expect("wallet present");
        info.core_wallet
            .accounts
            .coinjoin_accounts
            .get(&0)
            .is_some_and(|a| a.utxos.contains_key(outpoint))
    }

    /// Build a CoinJoin-funded shield over the split fixture and return the
    /// manager, the pre-spend aggregate, the spent-input total, the wallet
    /// change total, and the built tx. 0.09 DASH BIP44 + one 2.0 DASH CoinJoin
    /// UTXO, shield 0.2 DASH — LargestFirst funds it entirely from the CoinJoin
    /// UTXO, so the tx spends only a CoinJoin input and the change lands on BIP44.
    async fn build_coinjoin_shield() -> (
        Arc<AssetLockManager<AlwaysRejectedBroadcaster>>,
        u64,
        u64,
        u64,
        Transaction,
    ) {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let (manager, signer) = split_asset_lock_manager(9_000_000, 200_000_000).await;

        let (before_total, utxo_values) = {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&manager.wallet_id)
                .expect("wallet present");
            info.core_wallet.update_balance();
            let before = WalletInfoInterface::balance(&info.core_wallet).total();
            let mut values = std::collections::HashMap::new();
            for acc in info.core_wallet.accounts.all_funding_accounts() {
                for (op, utxo) in &acc.utxos {
                    values.insert(*op, utxo.txout.value);
                }
            }
            (before, values)
        };

        let (tx, _path) = manager
            .build_asset_lock_transaction(
                20_000_000,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await
            .expect("build multi-account asset lock");

        let sum_spent: u64 = tx
            .input
            .iter()
            .map(|i| utxo_values.get(&i.previous_output).copied().unwrap_or(0))
            .sum();
        // Wallet-owned outputs = the change (the AssetLock burn is an OP_RETURN).
        let sum_change: u64 = tx
            .output
            .iter()
            .filter(|o| !o.script_pubkey.is_op_return())
            .map(|o| o.value)
            .sum();
        assert!(sum_spent > 0, "tx must spend a wallet (CoinJoin) UTXO");
        assert!(sum_change > 0, "tx must return change to the wallet");

        (manager, before_total, sum_spent, sum_change, tx)
    }

    /// THE CASE THE DEVICE IS STUCK ON: after a hard reset the wallet is rebuilt
    /// from scratch and the asset-lock tx is re-seen by a block/rescan. The
    /// `check_core_transaction` scan — with NO broadcast-time mitigation in
    /// play (a rescan never runs the broadcast path) — must debit the spent
    /// CoinJoin input, so the balance settles to `previous − inputs + change`
    /// rather than re-inflating by the spent amount and re-triggering the
    /// reset→rescan→inflate loop. This passes ONLY because the vendored
    /// rust-dashcore carries the router fix (CoinJoin in the AssetLock relevant
    /// types); on the un-patched pin the CoinJoin input stays counted.
    #[tokio::test]
    async fn router_fix_debits_coinjoin_asset_lock_spend_on_rescan() {
        use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};

        let (manager, before_total, sum_spent, sum_change, tx) = build_coinjoin_shield().await;
        let spent_outpoint = tx.input[0].previous_output;
        assert!(
            coinjoin_has_utxo(&manager, &spent_outpoint).await,
            "the CoinJoin UTXO must be present before the scan (rescan re-added it)"
        );

        // Confirmation/rescan processing ONLY — no broadcast-time mitigation.
        {
            let mut wm = manager.wallet_manager.write().await;
            let (wallet, info) = wm
                .get_wallet_mut_and_info_mut(&manager.wallet_id)
                .expect("wallet present");
            info.core_wallet
                .check_core_transaction(
                    &tx,
                    TransactionContext::InChainLockedBlock(
                        key_wallet::transaction_checking::BlockInfo::new(
                            10,
                            dashcore::BlockHash::from_raw_hash(dashcore::hashes::Hash::all_zeros()),
                            1_700_001_000,
                        ),
                    ),
                    wallet,
                    true,
                    true,
                )
                .await;
        }

        assert!(
            !coinjoin_has_utxo(&manager, &spent_outpoint).await,
            "router fix must mark the spent CoinJoin UTXO spent on the scan"
        );
        assert_eq!(
            aggregate_total(&manager).await,
            before_total - sum_spent + sum_change,
            "post-rescan balance must be previous − inputs + change (no re-inflation)"
        );
    }

    /// The persistence half of dashpay/dash-wallet#1507: proving the router-
    /// fixed scan not only debits the CoinJoin input in memory but produces a
    /// [`TransactionRecord`] whose `input_details` reference the spent CoinJoin
    /// outpoint — the exact data `core_bridge::derive_spent_utxos` walks to tell
    /// the persister to DELETE that UTXO row. This is what makes the debit
    /// survive restart. The removed broadcast-time mitigation defeated this:
    /// by `.remove()`ing the UTXO in-memory first, it made the later scan's
    /// `ManagedCoreFundsAccount::check_transaction_for_match` find no UTXO, emit
    /// no CoinJoin record, and thus leave `input_details` empty — so nothing was
    /// ever persisted and the stale row reloaded on restart. With the mitigation
    /// gone, the scan owns the debit and the record carries the deletion through.
    #[tokio::test]
    async fn router_fix_records_spent_coinjoin_input_for_persistence() {
        use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};

        let (manager, _before_total, _sum_spent, _sum_change, tx) = build_coinjoin_shield().await;
        let spent_outpoint = tx.input[0].previous_output;
        assert!(
            coinjoin_has_utxo(&manager, &spent_outpoint).await,
            "the CoinJoin UTXO must be present before the scan"
        );

        // Normal-pipeline scan (no mitigation), capturing the emitted records.
        let result = {
            let mut wm = manager.wallet_manager.write().await;
            let (wallet, info) = wm
                .get_wallet_mut_and_info_mut(&manager.wallet_id)
                .expect("wallet present");
            info.core_wallet
                .check_core_transaction(&tx, TransactionContext::Mempool, wallet, true, true)
                .await
        };

        // A record must exist whose `input_details` resolve — through the same
        // `record.transaction.input[detail.index].previous_output` lookup
        // `derive_spent_utxos` uses — to the spent CoinJoin outpoint. Without
        // that entry the persister is never told to delete the row (the #1507
        // gap). It must belong to the CoinJoin account, the account the pinned
        // router omitted and the mitigation used to suppress.
        let persisted_spend = result
            .new_records
            .iter()
            .chain(result.updated_records.iter())
            .filter(|r| matches!(r.account_type, key_wallet::AccountType::CoinJoin { .. }))
            .flat_map(|r| {
                r.input_details.iter().filter_map(move |d| {
                    r.transaction
                        .input
                        .get(d.index as usize)
                        .map(|i| i.previous_output)
                })
            })
            .any(|op| op == spent_outpoint);

        assert!(
            persisted_spend,
            "the router-fixed scan must emit a CoinJoin TransactionRecord whose \
             input_details cover the spent outpoint {spent_outpoint}, so \
             derive_spent_utxos persists the deletion (dashpay/dash-wallet#1507)"
        );

        // And the in-memory debit itself must have happened.
        assert!(
            !coinjoin_has_utxo(&manager, &spent_outpoint).await,
            "the scan must mark the spent CoinJoin UTXO spent"
        );
    }

    // -- DashPay-leg asset-lock persistence (dashpay/dash-wallet#1507) --
    //
    // The vendored router fix and the removed broadcast-time mitigation's own
    // doc comment (see the `no broadcast-time balance mitigation` note above)
    // cover THREE previously-omitted fund-bearing account types for asset-lock
    // spend detection: `CoinJoin`, `DashpayReceivingFunds`, and
    // `DashpayExternalAccount`. The `build_coinjoin_shield` tests above exercise
    // only the CoinJoin leg; the following mirror them for BOTH DashPay legs, so
    // a latent gap in either DashPay arm of `get_relevant_account_types(AssetLock)`
    // (e.g. a `DashpayReceivingFunds`-vs-`DashpayExternalAccount` routing edge
    // case) cannot silently recur the exact persistence regression this PR closes.

    /// Wraps the split BIP44 + DashPay fixture (`leg` selects which DashPay
    /// account type carries the mixed slice) in an `AssetLockManager`.
    async fn split_asset_lock_manager_dashpay(
        bip44_duffs: u64,
        dashpay_duffs: u64,
        leg: DashpayLeg,
    ) -> (
        Arc<AssetLockManager<AlwaysRejectedBroadcaster>>,
        WalletSigner,
    ) {
        let (wallet_manager, wallet_id, signer) =
            crate::test_support::split_funded_wallet_manager_dashpay(
                bip44_duffs,
                dashpay_duffs,
                leg,
            )
            .await;
        let persistence = Arc::new(CapturingPersistence::default());
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));
        (manager, signer)
    }

    /// The set of outpoints held by the DashPay funds account of the given
    /// `leg` on account key index 0.
    async fn dashpay_account_outpoints(
        manager: &AssetLockManager<AlwaysRejectedBroadcaster>,
        leg: DashpayLeg,
    ) -> std::collections::HashSet<OutPoint> {
        let wm = manager.wallet_manager.read().await;
        let (_, info) = wm
            .get_wallet_and_info(&manager.wallet_id)
            .expect("wallet present");
        let map = match leg {
            DashpayLeg::ReceivingFunds => &info.core_wallet.accounts.dashpay_receival_accounts,
            DashpayLeg::ExternalAccount => &info.core_wallet.accounts.dashpay_external_accounts,
        };
        map.values().flat_map(|a| a.utxos.keys().copied()).collect()
    }

    /// `true` iff the DashPay funds account of the given `leg` still holds
    /// `outpoint` as an unspent UTXO.
    async fn dashpay_has_utxo(
        manager: &AssetLockManager<AlwaysRejectedBroadcaster>,
        leg: DashpayLeg,
        outpoint: &OutPoint,
    ) -> bool {
        let wm = manager.wallet_manager.read().await;
        let (_, info) = wm
            .get_wallet_and_info(&manager.wallet_id)
            .expect("wallet present");
        let map = match leg {
            DashpayLeg::ReceivingFunds => &info.core_wallet.accounts.dashpay_receival_accounts,
            DashpayLeg::ExternalAccount => &info.core_wallet.accounts.dashpay_external_accounts,
        };
        map.values().any(|a| a.utxos.contains_key(outpoint))
    }

    /// `true` iff `account_type` is the DashPay funds variant matching `leg`.
    fn record_matches_dashpay_leg(account_type: &key_wallet::AccountType, leg: DashpayLeg) -> bool {
        match leg {
            DashpayLeg::ReceivingFunds => matches!(
                account_type,
                key_wallet::AccountType::DashpayReceivingFunds { .. }
            ),
            DashpayLeg::ExternalAccount => matches!(
                account_type,
                key_wallet::AccountType::DashpayExternalAccount { .. }
            ),
        }
    }

    /// DashPay analogue of [`build_coinjoin_shield`]: fund a shield entirely
    /// from a single 2.0-DASH DashPay UTXO (the DashPay account of `leg`) over a
    /// BIP44 slice too small to cover it, so the tx spends exactly the DashPay
    /// input and change lands on BIP44. Returns the manager, the pre-spend
    /// aggregate, the spent-input total, the change total, the built tx, and the
    /// spent DashPay outpoint (resolved against the DashPay account's own UTXO
    /// set rather than assumed positionally).
    async fn build_dashpay_shield(
        leg: DashpayLeg,
    ) -> (
        Arc<AssetLockManager<AlwaysRejectedBroadcaster>>,
        u64,
        u64,
        u64,
        Transaction,
        OutPoint,
    ) {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let (manager, signer) = split_asset_lock_manager_dashpay(9_000_000, 200_000_000, leg).await;
        let dashpay_outpoints = dashpay_account_outpoints(&manager, leg).await;

        let (before_total, utxo_values) = {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&manager.wallet_id)
                .expect("wallet present");
            info.core_wallet.update_balance();
            let before = WalletInfoInterface::balance(&info.core_wallet).total();
            let mut values = std::collections::HashMap::new();
            for acc in info.core_wallet.accounts.all_funding_accounts() {
                for (op, utxo) in &acc.utxos {
                    values.insert(*op, utxo.txout.value);
                }
            }
            (before, values)
        };

        let (tx, _path) = manager
            .build_asset_lock_transaction(
                20_000_000,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await
            .expect("build DashPay-funded asset lock");

        let sum_spent: u64 = tx
            .input
            .iter()
            .map(|i| utxo_values.get(&i.previous_output).copied().unwrap_or(0))
            .sum();
        let sum_change: u64 = tx
            .output
            .iter()
            .filter(|o| !o.script_pubkey.is_op_return())
            .map(|o| o.value)
            .sum();
        assert!(sum_spent > 0, "tx must spend a wallet (DashPay) UTXO");
        assert!(sum_change > 0, "tx must return change to the wallet");

        // The spent DashPay outpoint, resolved against the DashPay account's own
        // UTXO set (not `input[0]`), so a future change in selection ordering
        // can't quietly make this assert about the wrong input.
        let dashpay_spent = tx
            .input
            .iter()
            .map(|i| i.previous_output)
            .find(|op| dashpay_outpoints.contains(op))
            .expect("shield must spend a DashPay UTXO (the multi-account #4073 fix)");

        (
            manager,
            before_total,
            sum_spent,
            sum_change,
            tx,
            dashpay_spent,
        )
    }

    /// Rescan-debit body shared by both DashPay legs: mirrors
    /// [`router_fix_debits_coinjoin_asset_lock_spend_on_rescan`] but over a
    /// DashPay-funded shield. A confirmation/rescan `check_core_transaction`
    /// (no broadcast-time mitigation) must debit the spent DashPay input so the
    /// balance settles to `previous − inputs + change` rather than re-inflating.
    async fn assert_router_fix_debits_dashpay_on_rescan(leg: DashpayLeg) {
        use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};

        let (manager, before_total, sum_spent, sum_change, tx, spent_outpoint) =
            build_dashpay_shield(leg).await;
        assert!(
            dashpay_has_utxo(&manager, leg, &spent_outpoint).await,
            "the DashPay UTXO must be present before the scan (rescan re-added it)"
        );

        {
            let mut wm = manager.wallet_manager.write().await;
            let (wallet, info) = wm
                .get_wallet_mut_and_info_mut(&manager.wallet_id)
                .expect("wallet present");
            info.core_wallet
                .check_core_transaction(
                    &tx,
                    TransactionContext::InChainLockedBlock(
                        key_wallet::transaction_checking::BlockInfo::new(
                            10,
                            dashcore::BlockHash::from_raw_hash(dashcore::hashes::Hash::all_zeros()),
                            1_700_001_000,
                        ),
                    ),
                    wallet,
                    true,
                    true,
                )
                .await;
        }

        assert!(
            !dashpay_has_utxo(&manager, leg, &spent_outpoint).await,
            "router fix must mark the spent DashPay UTXO spent on the scan ({leg:?})"
        );
        assert_eq!(
            aggregate_total(&manager).await,
            before_total - sum_spent + sum_change,
            "post-rescan balance must be previous − inputs + change (no re-inflation, {leg:?})"
        );
    }

    /// Persistence-record body shared by both DashPay legs: mirrors
    /// [`router_fix_records_spent_coinjoin_input_for_persistence`]. The scan must
    /// emit a `TransactionRecord` belonging to the DashPay account whose
    /// `input_details` resolve — through the same
    /// `record.transaction.input[detail.index].previous_output` lookup
    /// `derive_spent_utxos` uses — to the spent DashPay outpoint, so the
    /// persister is told to delete that UTXO row and the debit survives restart.
    async fn assert_router_fix_records_spent_dashpay_input(leg: DashpayLeg) {
        use key_wallet::transaction_checking::{TransactionContext, WalletTransactionChecker};

        let (manager, _before_total, _sum_spent, _sum_change, tx, spent_outpoint) =
            build_dashpay_shield(leg).await;
        assert!(
            dashpay_has_utxo(&manager, leg, &spent_outpoint).await,
            "the DashPay UTXO must be present before the scan"
        );

        let result = {
            let mut wm = manager.wallet_manager.write().await;
            let (wallet, info) = wm
                .get_wallet_mut_and_info_mut(&manager.wallet_id)
                .expect("wallet present");
            info.core_wallet
                .check_core_transaction(&tx, TransactionContext::Mempool, wallet, true, true)
                .await
        };

        let persisted_spend = result
            .new_records
            .iter()
            .chain(result.updated_records.iter())
            .filter(|r| record_matches_dashpay_leg(&r.account_type, leg))
            .flat_map(|r| {
                r.input_details.iter().filter_map(move |d| {
                    r.transaction
                        .input
                        .get(d.index as usize)
                        .map(|i| i.previous_output)
                })
            })
            .any(|op| op == spent_outpoint);

        assert!(
            persisted_spend,
            "the router-fixed scan must emit a {leg:?} TransactionRecord whose \
             input_details cover the spent outpoint {spent_outpoint}, so \
             derive_spent_utxos persists the deletion (dashpay/dash-wallet#1507)"
        );

        assert!(
            !dashpay_has_utxo(&manager, leg, &spent_outpoint).await,
            "the scan must mark the spent DashPay UTXO spent ({leg:?})"
        );
    }

    /// DashpayReceivingFunds analogue of
    /// `router_fix_debits_coinjoin_asset_lock_spend_on_rescan`.
    #[tokio::test]
    async fn router_fix_debits_dashpay_receiving_asset_lock_spend_on_rescan() {
        assert_router_fix_debits_dashpay_on_rescan(DashpayLeg::ReceivingFunds).await;
    }

    /// DashpayReceivingFunds analogue of
    /// `router_fix_records_spent_coinjoin_input_for_persistence`.
    #[tokio::test]
    async fn router_fix_records_spent_dashpay_receiving_input_for_persistence() {
        assert_router_fix_records_spent_dashpay_input(DashpayLeg::ReceivingFunds).await;
    }

    // -- Watch-only DashpayExternalAccount exclusion (finding 5b52d9844055) --
    //
    // A `DashpayExternalAccount` is created in production from a CONTACT's
    // decrypted xpub with `is_watch_only: true` (wallet/identity/network/
    // contacts.rs): its UTXOs are the contact's coins, and the local mnemonic
    // holds no key for them. `all_funding_accounts()` in the pinned key-wallet
    // fork nonetheless includes it, so the multi-account asset-lock builder must
    // filter it out — otherwise it would select those UTXOs, sign them with the
    // wrong local key, and produce an invalid input signature.
    //
    // Unlike the DashPay RECEIVING-funds leg (which IS ours and signable, and is
    // correctly INCLUDED — see the receiving tests above), these two tests assert
    // the EXCLUSION. The fixture builds the external account watch-only from a
    // FOREIGN seed's xpub, mirroring production; the earlier `add_account(_, None)`
    // fixture derived it from the test wallet's own seed, making it locally
    // signable and MASKING this defect.

    /// The union-of-accounts asset-lock builder must NOT select watch-only
    /// `DashpayExternalAccount` UTXOs. The external account here holds the
    /// LARGEST UTXO (0.5 DASH) while signable BIP44 (0.15 DASH) alone covers the
    /// 0.1-DASH shield, so a naive `LargestFirst` over the union would grab the
    /// external UTXO FIRST (the pre-fix bug). The build must instead succeed from
    /// BIP44 alone and leave every external UTXO unspent.
    #[tokio::test]
    async fn asset_lock_funding_excludes_watch_only_dashpay_external_utxos() {
        let (manager, signer) =
            split_asset_lock_manager_dashpay(15_000_000, 50_000_000, DashpayLeg::ExternalAccount)
                .await;
        let external_outpoints =
            dashpay_account_outpoints(&manager, DashpayLeg::ExternalAccount).await;
        assert!(
            !external_outpoints.is_empty(),
            "fixture must seed at least one watch-only external UTXO"
        );

        let (tx, _path) = manager
            .build_asset_lock_transaction(
                10_000_000,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await
            .expect("asset lock must build from signable BIP44 funds alone");

        for op in &external_outpoints {
            assert!(
                !tx.input.iter().any(|i| i.previous_output == *op),
                "watch-only external UTXO {op} must be excluded from asset-lock funding \
                 (no invalid-signature input reachable)"
            );
            assert!(
                dashpay_has_utxo(&manager, DashpayLeg::ExternalAccount, op).await,
                "excluded external UTXO {op} must remain unspent"
            );
        }
    }

    /// Watch-only external coins must not be *borrowed* to cover a shortfall.
    /// Signable BIP44 (0.05 DASH) alone is too small for the 0.1-DASH shield;
    /// the only way to cover it would be to (wrongly) spend the 0.5-DASH
    /// watch-only external UTXO. The build must FAIL with the typed
    /// insufficient-funds error rather than emit an invalid-signature input.
    #[tokio::test]
    async fn asset_lock_funding_cannot_borrow_watch_only_dashpay_external_utxos() {
        let (manager, signer) =
            split_asset_lock_manager_dashpay(5_000_000, 50_000_000, DashpayLeg::ExternalAccount)
                .await;

        let result = manager
            .build_asset_lock_transaction(
                10_000_000,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await;

        assert!(
            matches!(
                result,
                Err(PlatformWalletError::AssetLockInsufficientFunds { .. })
            ),
            "must not fund an asset lock from watch-only external coins; got {result:?}"
        );
    }

    /// Heavy-mixer CoinJoin discovery (dashpay/dash-wallet#1507): the CoinJoin
    /// account's default gap limit must watch a discovery window wide enough to
    /// bridge the address gaps a heavy mixer leaves — matched to dashj's
    /// ~100-key lookahead. Index 50 is beyond the OLD 30-address window; a fresh
    /// account must pre-generate it (so the BIP158 filter watches it) and
    /// recognize a tx paying it. On the old gap of 30 the address was never
    /// watched, so txs at far CoinJoin indices were skipped entirely — the
    /// starvation that survived a clean re-creation + full rescan, missing both
    /// the txs that created far-index UTXOs and the txs that spent nearer ones.
    #[tokio::test]
    async fn coinjoin_gap_limit_discovers_addresses_beyond_the_old_window() {
        use key_wallet::managed_account::address_pool::AddressPoolType;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::transaction_checking::{
            BlockInfo, TransactionContext, WalletTransactionChecker,
        };

        // Fresh wallet (BIP44 funded, CoinJoin account provisioned but empty).
        let (wallet_manager, wallet_id, _signer) =
            crate::test_support::split_funded_wallet_manager_many_coinjoin(9_000_000, &[]).await;

        // `far_index` sits past the old 30-address gap but within dashj's window.
        let far_index: u32 = 50;
        let far_address = {
            let wm = wallet_manager.read().await;
            let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
            let cj = info
                .core_wallet
                .accounts
                .coinjoin_accounts
                .get(&0)
                .expect("coinjoin account 0");
            assert_eq!(
                cj.gap_limit(),
                Some(100),
                "CoinJoin gap limit must match dashj's lookahead (100)"
            );
            let external = cj
                .managed_account_type()
                .address_pools()
                .into_iter()
                .find(|p| p.pool_type == AddressPoolType::External)
                .expect("external CoinJoin pool");
            // The load-bearing assertion: index 50 is pre-generated (and thus
            // filter-watched) ONLY because the gap was widened. On the old gap
            // of 30 this is `None` and the address below can't be fetched.
            external
                .address_at_index(far_index)
                .expect("index 50 must be pre-generated with the widened CoinJoin gap")
        };

        // A tx paying the far-index CoinJoin address must be discovered and its
        // UTXO tracked — proving the filter watched an address the old window
        // would have missed.
        let tx = Transaction::dummy(&far_address, 0..1, &[12_345_678]);
        let result = {
            let mut wm = wallet_manager.write().await;
            let (wallet, info) = wm
                .get_wallet_mut_and_info_mut(&wallet_id)
                .expect("wallet present");
            info.core_wallet
                .check_core_transaction(
                    &tx,
                    TransactionContext::InChainLockedBlock(BlockInfo::new(
                        5,
                        dashcore::BlockHash::from_raw_hash(dashcore::hashes::Hash::all_zeros()),
                        1_700_002_000,
                    )),
                    wallet,
                    true,
                    true,
                )
                .await
        };
        assert!(
            result.is_relevant,
            "a payment to the far-index CoinJoin address must be discovered"
        );
        assert!(result.is_new_transaction);

        let wm = wallet_manager.read().await;
        let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
        let cj = info
            .core_wallet
            .accounts
            .coinjoin_accounts
            .get(&0)
            .expect("coinjoin account 0");
        assert!(
            cj.utxos.values().any(|u| u.txout.value == 12_345_678),
            "the far-index CoinJoin UTXO must be tracked after discovery"
        );
    }
}
