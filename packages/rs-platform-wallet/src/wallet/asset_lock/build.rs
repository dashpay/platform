//! Asset lock transaction building.
//!
//! Contains methods for building asset lock transactions, peeking at funding
//! addresses, and the unified `create_funded_asset_lock_proof` entry point.

use crate::broadcaster::TransactionBroadcaster;
use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::{OutPoint, Transaction, TxOut};
use key_wallet::account::AccountType;
use key_wallet::bip32::DerivationPath;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::ExtendedPubKeySigner;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::{
    AssetLockFundingAccount, AssetLockFundingType, CreditOutputFunding,
};
use key_wallet::wallet::managed_wallet_info::managed_account_operations::ManagedAccountOperations;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;

use crate::changeset::{AccountRegistrationEntry, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::PlatformWalletInfo;

use super::manager::{AssetLockManager, DEFAULT_FEE_PER_KB};
use super::sync::tracking::BuiltPromotion;
use super::tracked::{AssetLockStatus, TrackedAssetLock};

// ---------------------------------------------------------------------------
// Asset lock transaction building
// ---------------------------------------------------------------------------

/// Amount semantics of a funded asset-lock build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetLockBuildAmount {
    /// Lock exactly this many duffs; funding UTXOs are coin-selected and
    /// change returns to the funding account.
    Exact(u64),
    /// Drain the funding account: every final UTXO is consumed and the
    /// lock value is `Σ inputs − fee`, computed by the key-wallet builder
    /// (see `build_asset_lock_with_signer`'s drain mode). Required for
    /// CoinJoin funding, whose accounts have no change semantics.
    DrainAll {
        /// Authoritative floor on the drained lock value, checked against
        /// the BUILT payload before anything is tracked or broadcast — the
        /// only sound place to enforce it, since the drained value is
        /// unknowable beforehand (a pre-build balance estimate races
        /// concurrent reservations and coin-selection filters). An
        /// undersized build is abandoned with an owner-guarded reservation
        /// release and nothing reaches the wire. `None` skips the check.
        minimum_lock_duffs: Option<u64>,
    },
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Build an asset lock transaction using the key-wallet builder.
    ///
    /// Delegates UTXO selection, fee calculation, and signing to
    /// `ManagedWalletInfo::build_asset_lock_with_signer`. The host
    /// never sees a raw credit-output private key — the returned
    /// `DerivationPath` is what the caller hands back to the same
    /// `signer` when the credit output is later consumed on Platform.
    ///
    /// Exact-amount BIP44 form — the historical entry point; the
    /// funding-parameterized form is
    /// [`Self::build_asset_lock_transaction_with_funding`].
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
    ///
    /// Serialized on [`build_persist_serial`] and refuses on a retired
    /// manager — see [`build_asset_lock_transaction_with_funding_locked`]
    /// for why the check has to happen under that mutex.
    ///
    /// [`build_persist_serial`]: AssetLockManager::build_persist_serial
    /// [`build_asset_lock_transaction_with_funding_locked`]:
    /// AssetLockManager::build_asset_lock_transaction_with_funding_locked
    pub async fn build_asset_lock_transaction<S: ExtendedPubKeySigner>(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(Transaction, DerivationPath), PlatformWalletError> {
        self.build_asset_lock_transaction_with_funding(
            AssetLockBuildAmount::Exact(amount_duffs),
            AssetLockFundingAccount::Bip44 { account_index },
            funding_type,
            identity_index,
            signer,
        )
        .await
        // Historical callers never had the reservation token; the funded
        // pipeline (`broadcast_funded_asset_lock_with_funding`) threads it.
        .map(|(tx, path, _token)| (tx, path))
    }

    /// Funding-parameterized form of [`Self::build_asset_lock_transaction`]:
    /// `funding_account` picks the account family supplying (and signing)
    /// the funding UTXOs, and `amount` picks exact-amount vs whole-balance
    /// drain semantics (see [`AssetLockBuildAmount`]). CoinJoin funding is
    /// drain-only — the key-wallet builder rejects a non-drain CoinJoin
    /// build.
    ///
    /// Serialized on [`build_persist_serial`](Self::build_persist_serial)
    /// and refuses on a retired manager. Callers that already hold the
    /// mutex (the funded broadcast path) must use
    /// [`build_asset_lock_transaction_with_funding_locked`](Self::build_asset_lock_transaction_with_funding_locked).
    pub async fn build_asset_lock_transaction_with_funding<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_account: AssetLockFundingAccount,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<
        (
            Transaction,
            DerivationPath,
            Option<key_wallet::ReservationToken>,
        ),
        PlatformWalletError,
    > {
        // Cheap early-out so an obviously-stale handle fails without
        // queueing behind an unrelated in-flight build.
        self.ensure_active()?;

        let build_serial = self.lock_build_persist_serial().await;
        self.build_asset_lock_transaction_with_funding_locked(
            amount,
            funding_account,
            funding_type,
            identity_index,
            signer,
            &build_serial,
        )
        .await
    }

    /// [`build_asset_lock_transaction_with_funding`](Self::build_asset_lock_transaction_with_funding)
    /// for callers that already hold
    /// [`build_persist_serial`](Self::build_persist_serial) — namely
    /// [`broadcast_funded_asset_lock_with_funding`](Self::broadcast_funded_asset_lock_with_funding),
    /// which holds it across build→pool-persist so a funding index can
    /// never be allocated and lost. Taking it again here would
    /// self-deadlock, hence the split.
    ///
    /// The activity check lives here, under the mutex, rather than in
    /// the public wrapper alone. Wallet ids are deterministic in (seed,
    /// network), so removing a wallet and re-importing the same mnemonic
    /// produces the *same* id over a fresh `PlatformWalletInfo`. A
    /// handle retained across that boundary — an `Arc<PlatformWallet>`
    /// the FFI still holds, or an operation already parked on an await —
    /// resolves `self.wallet_id` to the replacement generation. Without
    /// this check it would happily derive a top-up account into it,
    /// consume one of its funding addresses, and reserve its UTXOs on
    /// behalf of a wallet the user deleted. Because
    /// [`deactivate`](Self::deactivate) must take this same mutex to
    /// flip the flag, a pass here holds for the whole critical section.
    pub(super) async fn build_asset_lock_transaction_with_funding_locked<
        S: ExtendedPubKeySigner,
    >(
        &self,
        amount: AssetLockBuildAmount,
        funding_account: AssetLockFundingAccount,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
        build_serial: &tokio::sync::MutexGuard<'_, ()>,
    ) -> Result<
        (
            Transaction,
            DerivationPath,
            Option<key_wallet::ReservationToken>,
        ),
        PlatformWalletError,
    > {
        let (amount_duffs, drain) = match amount {
            AssetLockBuildAmount::Exact(v) => (v, false),
            // The credit-output value is a placeholder — the key-wallet
            // drain build rewrites it to Σ inputs − fee. The minimum is
            // enforced by `broadcast_funded_asset_lock_with_funding`
            // against the built payload.
            AssetLockBuildAmount::DrainAll { .. } => (0, true),
        };
        if amount_duffs == 0 && !drain {
            return Err(PlatformWalletError::AssetLockTransaction(
                "Amount must be greater than zero".to_string(),
            ));
        }

        // Authoritative: must precede the `wallet_manager` write lock,
        // because everything past it mutates the wallet this id now
        // resolves to.
        self.ensure_active_under_build_serial(build_serial)?;

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

        // 3. Delegate to the key-wallet signer-driven builder with the
        // caller's funding account + drain semantics (the key-wallet side
        // enforces that CoinJoin funding is drain-only).
        let result = info
            .core_wallet
            .build_asset_lock_with_signer(
                wallet,
                funding_account,
                vec![funding],
                DEFAULT_FEE_PER_KB,
                drain,
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

        Ok((result.transaction, path, result.reservation_token))
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
        self.create_funded_asset_lock_proof_with_funding(
            AssetLockBuildAmount::Exact(amount_duffs),
            AssetLockFundingAccount::Bip44 { account_index },
            funding_type,
            identity_index,
            signer,
        )
        .await
    }

    /// Funding-parameterized form of [`Self::create_funded_asset_lock_proof`]
    /// — same build → broadcast → proof pipeline with the account family and
    /// amount semantics of [`Self::build_asset_lock_transaction_with_funding`].
    pub async fn create_funded_asset_lock_proof_with_funding<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_account: AssetLockFundingAccount,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath, OutPoint), PlatformWalletError> {
        let (path, out_point) = self
            .broadcast_funded_asset_lock_with_funding(
                amount,
                funding_account,
                funding_type,
                identity_index,
                signer,
            )
            .await?;
        let proof = self
            .wait_for_funded_asset_lock_proof(&out_point, funding_account.account_index())
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
        self.broadcast_funded_asset_lock_with_funding(
            AssetLockBuildAmount::Exact(amount_duffs),
            AssetLockFundingAccount::Bip44 { account_index },
            funding_type,
            identity_index,
            signer,
        )
        .await
    }

    /// Funding-parameterized form of [`Self::broadcast_funded_asset_lock`].
    pub(crate) async fn broadcast_funded_asset_lock_with_funding<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_account: AssetLockFundingAccount,
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
        // Fail a stale handle before spending a build (which allocates a
        // funding index and reserves inputs) on a wallet that is gone.
        // Advisory only — the authoritative refusals are the same check
        // under `build_persist_serial` inside
        // `build_asset_lock_transaction_with_funding_locked` and under
        // `status_persist_serial` inside `track_asset_lock` and
        // `promote_built_to_broadcast`, since a removal can land during
        // the queue below, the build, or the broadcast await.
        self.ensure_active()?;

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
        let build_persist_guard = self.lock_build_persist_serial().await;

        // 1. Build the asset lock transaction. `_locked` because we
        //    already hold `build_persist_serial`; it re-checks activity
        //    under that guard, which is the authoritative refusal for a
        //    removal that landed while we were queued above.
        let (tx, path, reservation_token) = self
            .build_asset_lock_transaction_with_funding_locked(
                amount,
                funding_account,
                funding_type,
                identity_index,
                signer,
                &build_persist_guard,
            )
            .await?;

        let txid = tx.txid();
        let out_point = OutPoint::new(txid, 0);

        // The tracked/logged amount is read back from the built payload —
        // for `Exact` it equals the requested value; for `DrainAll` the
        // builder computed it (Σ inputs − fee) and this is the only place
        // it is known.
        let locked_amount_duffs: u64 = match &tx.special_transaction_payload {
            Some(
                dashcore::blockdata::transaction::special_transaction::TransactionPayload::AssetLockPayloadType(p),
            ) => p.credit_outputs.iter().map(|o| o.value).sum(),
            _ => 0,
        };

        // Authoritative drain floor: judged on the BUILT payload, before the
        // lock is tracked or broadcast. An undersized drain (its consumers
        // derive `shield_amount = lock_value − pool_fee`, so a lock at or
        // below the fee is unconsumable) is abandoned: owner-guarded
        // reservation release (the build `.await`ed, so the reservation may
        // have been swept and re-owned) and no transaction reaches the wire.
        // The funding key index consumed by the build is the same residue any
        // discarded build leaves and is reclaimed by the gap-limit scan.
        if let AssetLockBuildAmount::DrainAll {
            minimum_lock_duffs: Some(minimum),
        } = amount
        {
            if locked_amount_duffs < minimum {
                drop(build_persist_guard);
                let reserved_account = match funding_account {
                    AssetLockFundingAccount::Bip44 { account_index } => {
                        crate::wallet::reservations::ReservedFundingAccount::Standard(
                            key_wallet::account::account_type::StandardAccountType::BIP44Account,
                            account_index,
                        )
                    }
                    AssetLockFundingAccount::CoinJoin { account_index } => {
                        crate::wallet::reservations::ReservedFundingAccount::CoinJoin(account_index)
                    }
                };
                crate::wallet::reservations::release_reservation_after_rejected_broadcast(
                    &self.wallet_manager,
                    &self.wallet_id,
                    reserved_account,
                    &tx,
                    reservation_token,
                )
                .await;
                return Err(PlatformWalletError::AssetLockTransaction(format!(
                    "drained asset lock of {locked_amount_duffs} duffs is below the required \
                     minimum of {minimum} duffs (the balance cannot clear the shield pool fee); \
                     nothing was broadcast"
                )));
            }
        }

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
        // `track_asset_lock` queues the changeset itself, as one
        // serialized unit with the in-memory insert. It also re-checks
        // the manager's lifecycle state under the ordering mutex, so a
        // wallet removed during the build above aborts here — before a
        // transaction reaches the wire and before a row is written that
        // a replacement wallet would inherit.
        let _cs_built = self
            .track_asset_lock(TrackedAssetLock {
                out_point,
                transaction: tx.clone(),
                account_index: funding_account.account_index(),
                funding_type,
                identity_index,
                amount: locked_amount_duffs,
                status: AssetLockStatus::Built,
                proof: None,
            })
            .await?;

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
                // `untrack_asset_lock` queues the changeset itself, as one
                // serialized unit with the in-memory removal.
                //
                // It refuses outright once the wallet has been removed from
                // the manager. Treat that exactly like the untrack guard
                // below: skip the release. The wallet whose reservation this
                // call took no longer exists, and `wallet_id` now resolves to
                // whatever replacement was registered under the same
                // deterministic id — releasing there would free inputs the
                // replacement never reserved.
                let removed_built_row = match self.untrack_asset_lock(&out_point).await {
                    Ok(cs_untrack) => cs_untrack.removed.contains(&out_point),
                    Err(untrack_err) => {
                        tracing::warn!(
                            %txid,
                            error = %untrack_err,
                            "rejected broadcast could not untrack the Built row;                              leaving the funding reservation alone"
                        );
                        false
                    }
                };
                // Release only when the Built row was actually removed. If
                // the untrack guard fired instead — a concurrent
                // `resume_asset_lock` advanced the row past `Built`, positive
                // evidence the transaction reached the network after all —
                // the inputs must stay reserved exactly like a `MaybeSent`
                // outcome, or the still-tracked row would be resumable while
                // its inputs are re-spendable.
                if removed_built_row {
                    let reserved_account = match funding_account {
                        AssetLockFundingAccount::Bip44 {
                            account_index,
                        } => crate::wallet::reservations::ReservedFundingAccount::Standard(
                            key_wallet::account::account_type::StandardAccountType::BIP44Account,
                            account_index,
                        ),
                        AssetLockFundingAccount::CoinJoin {
                            account_index,
                        } => crate::wallet::reservations::ReservedFundingAccount::CoinJoin(
                            account_index,
                        ),
                    };
                    crate::wallet::reservations::release_reservation_after_rejected_broadcast(
                        &self.wallet_manager,
                        &self.wallet_id,
                        reserved_account,
                        &tx,
                        reservation_token,
                    )
                    .await;
                }
            }
            return Err(e.into());
        }

        // 4. Transition to Broadcast and queue the changeset.
        //
        // Compare-and-set, not an unconditional write: the row was tracked as
        // `Built` back in step 2 and the await above is unbounded, so a
        // concurrent `resume_asset_lock` (the FFI catch-up scanner and the
        // funding resolver both drive one for any tracked outpoint) can pick
        // the row up, broadcast the same transaction, obtain a proof and
        // finalize it to `InstantSendLocked` / `ChainLocked` while this call
        // is still parked in `broadcast(&tx)`. Assigning `Broadcast`
        // unconditionally on the way out would then downgrade that finalized
        // row, and because `advance_asset_lock_status` leaves the existing
        // proof attached when passed `None`, it would recreate — and persist,
        // changesets being last-write-wins — the inconsistent
        // `Broadcast + Some(proof)` state. A later resume takes the
        // `Broadcast` arm and waits for a proof the row already holds,
        // unbounded for the user-facing funding flows.
        //
        // So promote only while the row is still `Built`. If it advanced, its
        // status and proof are strictly further along than anything this call
        // could write, so leave them untouched: our broadcast still succeeded,
        // which is all this create-only half reports. Unlike
        // `resume_asset_lock` there is nothing to re-dispatch — the proof wait
        // lives in `wait_for_funded_asset_lock_proof`, the caller's next step.
        match self.promote_built_to_broadcast(&out_point).await? {
            // The promotion queued its own changeset, atomically with the
            // compare-and-set, so a concurrent finalize cannot have its
            // newer snapshot overtaken by this older one.
            BuiltPromotion::Promoted(_cs) => {}
            BuiltPromotion::AlreadyAdvanced {
                current_status,
                current_proof,
            } => {
                tracing::info!(
                    outpoint = %out_point,
                    status = ?current_status,
                    has_proof = current_proof.is_some(),
                    "broadcast_funded_asset_lock: row advanced past Built                      concurrently during the broadcast — keeping its current                      state instead of downgrading it to Broadcast"
                );
            }
        }

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
        // Queued by `advance_asset_lock_status` itself, atomically with
        // the in-memory write.
        let _cs_final = self
            .advance_asset_lock_status(out_point, status, Some(proof.clone()))
            .await?;

        Ok(proof)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use dashcore::OutPoint;
    use key_wallet::account::account_type::StandardAccountType;
    use tokio::sync::Notify;

    use async_trait::async_trait;
    use dashcore::{Transaction, Txid};
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::changeset::{
        AssetLockEntry, ClientStartState, PersistenceError, PlatformWalletChangeSet,
        PlatformWalletPersistence,
    };
    use crate::test_support::{
        funded_wallet_manager, AlwaysMaybeSentBroadcaster, AlwaysOkBroadcaster,
        AlwaysRejectedBroadcaster, WalletSigner,
    };
    use crate::wallet::asset_lock::manager::{
        AssetLockManager, PromotePostCasGate, ResumePrePromoteGate,
    };
    use crate::wallet::asset_lock::tracked::AssetLockStatus;
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use crate::wallet::platform_wallet::WalletId;
    use crate::{AssetLockFundingType, PlatformWalletError};

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
        /// The durable row for `out_point` after replaying every stored
        /// round in arrival order.
        ///
        /// Models the real downstream semantics exactly, which is what
        /// makes this a persistence-order assertion rather than a
        /// "was it ever queued" one:
        ///
        /// - `FFIPersister::store_round` serializes rounds by acquisition
        ///   order, so replay order == the order `store()` was called;
        /// - `AssetLockChangeSet::merge` is last-write-wins per outpoint;
        /// - Swift's `persistAssetLocks` upsert overwrites `statusRaw` /
        ///   `proofBytes` unconditionally, and each `removed` entry
        ///   deletes the row.
        ///
        /// `None` means no row survives — either none was ever written or
        /// the last round removed it. Since the load path reconstructs
        /// `tracked_asset_locks` from exactly these rows, this is also
        /// what a restart would read back.
        fn durable_asset_lock(&self, out_point: &OutPoint) -> Option<AssetLockEntry> {
            let stored = self.stored.lock().expect("capturing persistence mutex");
            let mut row: Option<AssetLockEntry> = None;
            for cs in stored.iter() {
                let Some(al) = cs.asset_locks.as_ref() else {
                    continue;
                };
                if let Some(entry) = al.asset_locks.get(out_point) {
                    row = Some(entry.clone());
                }
                if al.removed.contains(out_point) {
                    row = None;
                }
            }
            row
        }

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

    /// Broadcaster that succeeds and counts its calls, so a test can assert
    /// an abandoned build never reached the wire.
    #[derive(Default)]
    struct CountingOkBroadcaster {
        calls: std::sync::atomic::AtomicUsize,
    }

    impl CountingOkBroadcaster {
        fn calls(&self) -> usize {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl TransactionBroadcaster for CountingOkBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(transaction.txid())
        }
    }

    /// Builds an `AssetLockManager` over the CoinJoin-funded fixture
    /// (CoinJoin account 0 holds a single 10_000_000-duff spendable UTXO).
    async fn coinjoin_funded_asset_lock_manager<B: TransactionBroadcaster>(
        broadcaster: Arc<B>,
    ) -> (
        Arc<AssetLockManager<B>>,
        crate::test_support::WalletSigner,
        Arc<CapturingPersistence>,
    ) {
        let persistence = Arc::new(CapturingPersistence::default());
        let (wallet_manager, wallet_id, _generation, signer) =
            crate::test_support::funded_coinjoin_wallet_manager().await;

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));

        (manager, signer, persistence)
    }

    /// An undersized drain is abandoned BEFORE tracking or broadcast — the
    /// floor is judged on the BUILT payload (the fixture's 10_000_000-duff
    /// CoinJoin balance minus L1 fee), nothing reaches the wire, no row is
    /// tracked, and the owner-guarded reservation release frees the inputs
    /// so an immediate follow-up drain over the SAME single-UTXO account
    /// can select them and succeed.
    #[tokio::test]
    async fn undersized_drain_abandoned_before_broadcast() {
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer, persistence) =
            coinjoin_funded_asset_lock_manager(Arc::clone(&broadcaster)).await;

        let result = manager
            .broadcast_funded_asset_lock_with_funding(
                super::AssetLockBuildAmount::DrainAll {
                    // Far above the fixture balance: the built lock value
                    // (Σ inputs − fee < 10_000_000) must fail the floor.
                    minimum_lock_duffs: Some(u64::MAX),
                },
                key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount::CoinJoin {
                    account_index: 0,
                },
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await;
        let err = result.expect_err("undersized drain must be refused");
        assert!(
            err.to_string().contains("below the required minimum"),
            "unexpected error for undersized drain: {err}"
        );
        assert_eq!(
            broadcaster.calls(),
            0,
            "an abandoned drain must never reach the broadcaster"
        );
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            assert!(
                info.tracked_asset_locks.is_empty(),
                "an abandoned drain must not leave a tracked row, got {:?}",
                info.tracked_asset_locks
            );
        }
        assert!(
            persistence.removed_outpoints().is_empty(),
            "nothing was tracked, so nothing should be queued for removal"
        );

        // The reservation was released through the owner token: a follow-up
        // drain over the same single-UTXO CoinJoin account must be able to
        // select the inputs immediately and broadcast.
        manager
            .broadcast_funded_asset_lock_with_funding(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(1),
                },
                key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount::CoinJoin {
                    account_index: 0,
                },
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await
            .expect("follow-up drain must reselect the released inputs");
        assert_eq!(
            broadcaster.calls(),
            1,
            "the follow-up drain should broadcast exactly once"
        );
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
                    && entry.addresses.iter().any(|a| {
                        matches!(
                            a.state,
                            key_wallet::managed_account::address_pool::AddressState::Used
                        )
                    })
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
                let used = entry
                    .addresses
                    .iter()
                    .filter(|a| {
                        matches!(
                            a.state,
                            key_wallet::managed_account::address_pool::AddressState::Used
                        )
                    })
                    .count();
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

    /// Broadcaster that models the read-before-broadcast interleave between
    /// the create-path Rejected cleanup and a concurrent `resume_asset_lock`
    /// re-broadcast. Call 1 is create's broadcast (blocks until the test
    /// releases it, then returns `Rejected`); call 2 is resume's re-broadcast
    /// (blocks until the test releases it, then returns success). Each side
    /// signals a `Notify` when it enters so the test can order the race
    /// deterministically.
    struct RaceRejectDuringResumeBroadcaster {
        call_count: AtomicUsize,
        create_entered: Arc<Notify>,
        create_can_return: Arc<Notify>,
        resume_entered: Arc<Notify>,
        resume_can_return: Arc<Notify>,
    }

    #[async_trait]
    impl TransactionBroadcaster for RaceRejectDuringResumeBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            let call = self.call_count.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                self.create_entered.notify_one();
                self.create_can_return.notified().await;
                Err(BroadcastError::Rejected {
                    reason: "simulated rejection during concurrent resume".to_string(),
                })
            } else {
                self.resume_entered.notify_one();
                self.resume_can_return.notified().await;
                Ok(transaction.txid())
            }
        }
    }

    /// The read-before-broadcast interleave: a `resume_asset_lock` snapshots
    /// the `Built` row under a read lock, drops the lock, and calls
    /// `broadcast(&tx)` while the create path is still awaiting its own
    /// broadcast. When the create broadcast then returns `Rejected`, the
    /// cleanup must not remove the row or release the funding reservation —
    /// the resume path may still be handing the same transaction to the
    /// network. The `Built`-arm advance-before-broadcast in
    /// `resume_asset_lock` closes this window by pushing the status past
    /// `Built` under the write lock before the re-broadcast; the untrack
    /// guard then preserves the row and its reservation.
    #[tokio::test]
    async fn rejected_broadcast_racing_resume_read_before_broadcast_keeps_row_and_reservation() {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;

        let create_entered = Arc::new(Notify::new());
        let create_can_return = Arc::new(Notify::new());
        let resume_entered = Arc::new(Notify::new());
        let resume_can_return = Arc::new(Notify::new());

        let broadcaster = Arc::new(RaceRejectDuringResumeBroadcaster {
            call_count: AtomicUsize::new(0),
            create_entered: Arc::clone(&create_entered),
            create_can_return: Arc::clone(&create_can_return),
            resume_entered: Arc::clone(&resume_entered),
            resume_can_return: Arc::clone(&resume_can_return),
        });
        let persistence = Arc::new(CapturingPersistence::default());
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::clone(&broadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));

        // 1. Start the create path. It builds a fresh asset-lock tx, tracks
        //    the `Built` row, then blocks inside our broadcaster on the
        //    first call.
        let signer = Arc::new(signer);
        let manager_create = Arc::clone(&manager);
        let signer_create = Arc::clone(&signer);
        let create_handle = tokio::spawn(async move {
            manager_create
                .create_funded_asset_lock_proof(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    0,
                    &*signer_create,
                )
                .await
        });
        create_entered.notified().await;

        // 2. The row is now tracked at `Built`; snapshot its outpoint.
        let out_point = {
            let wm = wallet_manager.read().await;
            let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
            *info
                .tracked_asset_locks
                .keys()
                .next()
                .expect("built row tracked before broadcast")
        };

        // 3. Start the resume path against the same outpoint. It looks up
        //    the tracked row under a read lock, drops the lock, and — with
        //    the fix — advances Built → Broadcast under the write lock
        //    before entering the second broadcaster call.
        let manager_resume = Arc::clone(&manager);
        let resume_handle = tokio::spawn(async move {
            manager_resume
                .resume_asset_lock(&out_point, Some(Duration::from_millis(50)))
                .await
        });
        resume_entered.notified().await;

        // 4. Let the create broadcast return `Rejected`. Because the row
        //    has already been advanced past `Built` by the concurrent
        //    resume, the untrack guard must refuse to remove it and the
        //    reservation must stay held.
        create_can_return.notify_one();
        let create_result = create_handle.await.expect("create task joined");
        assert!(
            matches!(
                create_result,
                Err(PlatformWalletError::TransactionBroadcast(_))
            ),
            "rejection should still surface, got {create_result:?}"
        );

        // 5. Let the resume broadcast complete. Resume will then wait for a
        //    proof and time out (short deadline), which is fine — the
        //    interleave under test is already resolved by this point.
        resume_can_return.notify_one();
        let _ = resume_handle.await.expect("resume task joined");

        // Both sides actually attempted a broadcast: the interleave really
        // did happen (resume did not error out before its broadcast call).
        assert_eq!(
            broadcaster.call_count.load(Ordering::SeqCst),
            2,
            "both create and resume should have attempted a broadcast"
        );

        // The row survives the rejection cleanup, past `Built`.
        {
            let wm = wallet_manager.read().await;
            let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
            assert_eq!(
                info.tracked_asset_locks.len(),
                1,
                "row must survive: a concurrent resume broadcast the tx, so the \
                 rejection cleanup must not delete the row"
            );
            let lock = info
                .tracked_asset_locks
                .get(&out_point)
                .expect("row kept at the raced outpoint");
            assert_ne!(
                lock.status,
                AssetLockStatus::Built,
                "resume must have advanced the row past Built before its \
                 broadcast; found {:?}",
                lock.status
            );
        }

        // No persisted-row deletion was queued.
        assert!(
            persistence.removed_outpoints().is_empty(),
            "advanced row must not be queued for deletion, got {:?}",
            persistence.removed_outpoints()
        );

        // The reservation was NOT released: a fresh build cannot reselect
        // the single reserved UTXO — otherwise resume would have handed the
        // network a transaction whose inputs are re-spendable locally.
        let rebuild = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &*signer,
            )
            .await;
        assert!(
            matches!(rebuild, Err(PlatformWalletError::AssetLockTransaction(_))),
            "rebuild must fail at input selection while the reservation is \
             held across the concurrent resume, got {rebuild:?}"
        );
    }

    /// Broadcaster whose first call (create-path) returns `MaybeSent` (leaving
    /// the tracked row at `Built` with the funding reservation held) and whose
    /// second call (resume-path) returns `Rejected` — the case where resume
    /// pre-advances the row to `Broadcast` under the write lock and then the
    /// broadcast definitively fails to reach the network.
    struct MaybeSentThenRejectedBroadcaster {
        call_count: AtomicUsize,
    }

    #[async_trait]
    impl TransactionBroadcaster for MaybeSentThenRejectedBroadcaster {
        async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, BroadcastError> {
            let call = self.call_count.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                Err(BroadcastError::MaybeSent {
                    reason: "create-path leaves the row at Built for a later resume".to_string(),
                })
            } else {
                Err(BroadcastError::Rejected {
                    reason: "resume-side rejection after the pre-advance to Broadcast".to_string(),
                })
            }
        }
    }

    /// After the `Built`-arm race-guard advances the tracked row to
    /// `Broadcast`, a resume-side broadcast that returns `Rejected` must keep
    /// that status: the `Broadcast` arm can defensively re-broadcast on later
    /// resumes, and rolling back could clobber a concurrent successful resume.
    /// The funding reservation must stay held (the row is still tracked), and
    /// no persisted-row deletion must be queued.
    #[tokio::test]
    async fn resume_side_rejected_after_pre_advance_keeps_row_at_broadcast() {
        let broadcaster = Arc::new(MaybeSentThenRejectedBroadcaster {
            call_count: AtomicUsize::new(0),
        });
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::clone(&broadcaster)).await;

        // 1. Create leaves the row at `Built` (MaybeSent keeps the reservation
        //    and the resumable row).
        let create_result = manager
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
                create_result,
                Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
            ),
            "create-side MaybeSent should surface as TransactionBroadcastUnconfirmed, got {create_result:?}"
        );
        let out_point = {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            assert_eq!(info.tracked_asset_locks.len(), 1);
            let (op, lock) = info
                .tracked_asset_locks
                .iter()
                .next()
                .expect("built row tracked");
            assert_eq!(lock.status, AssetLockStatus::Built);
            *op
        };

        // 2. Resume: pre-advances Built → Broadcast under the write lock, then
        //    calls `broadcast(&tx)` which returns `Rejected`. The `Rejected`
        //    surfaces to the caller.
        let resume_result = manager
            .resume_asset_lock(&out_point, Some(Duration::from_millis(10)))
            .await;
        assert!(
            matches!(
                resume_result,
                Err(PlatformWalletError::TransactionBroadcast(_))
            ),
            "resume-side Rejected should surface as TransactionBroadcast, got {resume_result:?}"
        );
        assert_eq!(
            broadcaster.call_count.load(Ordering::SeqCst),
            2,
            "both create and resume must have attempted a broadcast"
        );

        // 3. Row remains at `Broadcast` — the pre-advance is retained because
        //    another resume may already own that shared status.
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            assert_eq!(
                info.tracked_asset_locks.len(),
                1,
                "row must remain tracked for a later Broadcast-arm resume"
            );
            let lock = info
                .tracked_asset_locks
                .get(&out_point)
                .expect("row still tracked");
            assert_eq!(
                lock.status,
                AssetLockStatus::Broadcast,
                "resume-side Rejected after pre-advance must keep Broadcast \
                 so it does not clobber a concurrent successful resume, got {:?}",
                lock.status
            );
        }

        // 4. No persisted-row deletion was queued — the row must survive for
        //    a later resume.
        assert!(
            persistence.removed_outpoints().is_empty(),
            "Broadcast row must not be queued for deletion, got {:?}",
            persistence.removed_outpoints()
        );

        // 5. Reservation is still held — the row is tracked, so a fresh build
        //    over the single-UTXO wallet fails at input selection.
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
             held for the retained Broadcast row, got {rebuild:?}"
        );
    }

    /// `MaybeSent` on every call (leaving the tracked row at `Built` with
    /// its funding reservation held), counting calls so a test can assert
    /// that a stale resume did NOT re-broadcast.
    struct CountingMaybeSentBroadcaster {
        call_count: AtomicUsize,
    }

    #[async_trait]
    impl TransactionBroadcaster for CountingMaybeSentBroadcaster {
        async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Err(BroadcastError::MaybeSent {
                reason: "create-path leaves the row at Built for a later resume".to_string(),
            })
        }
    }

    /// Regression: a `resume_asset_lock` holding a STALE `Built` snapshot
    /// must not downgrade a row that a concurrent flow already finalized.
    ///
    /// Two resumes can both snapshot `Built` under the read lock (which is
    /// dropped before the pre-broadcast promotion). If the first one
    /// broadcasts, obtains a proof and stores `ChainLocked` + proof, an
    /// unconditional `Built -> Broadcast` write from the delayed second
    /// caller would downgrade the finalized row to `Broadcast` while
    /// leaving the proof attached — an inconsistent `Broadcast +
    /// Some(proof)` state that also gets persisted (changesets are
    /// last-write-wins). A later resume would then take the `Broadcast`
    /// arm and wait for a proof the row already holds, potentially
    /// forever.
    ///
    /// The compare-and-set in `promote_built_to_broadcast` makes the
    /// stale caller observe the advanced row instead, re-dispatch from its
    /// current status, and reuse the attached proof.
    #[tokio::test]
    async fn stale_built_resume_does_not_downgrade_a_concurrently_finalized_row() {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

        // A `MaybeSent` create leaves the row at `Built` with the funding
        // reservation held — the state a resume picks up from. The
        // broadcaster counts calls so the test can prove the stale resume
        // never re-broadcast (it took the already-have-a-proof arm).
        let broadcaster = Arc::new(CountingMaybeSentBroadcaster {
            call_count: AtomicUsize::new(0),
        });
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::clone(&broadcaster)).await;
        let _ = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        let out_point = {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            let (op, lock) = info
                .tracked_asset_locks
                .iter()
                .next()
                .expect("built row tracked");
            assert_eq!(lock.status, AssetLockStatus::Built);
            *op
        };

        // 1. Install the pre-promote gate, then start a resume. It takes the
        //    read-locked `Built` snapshot and parks before the
        //    compare-and-set — this is the stale caller.
        let arrived = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        *manager
            .resume_pre_promote_gate
            .lock()
            .expect("resume pre-promote gate mutex") = Some(ResumePrePromoteGate {
            arrived: Arc::clone(&arrived),
            release: Arc::clone(&release),
        });

        let manager_stale = Arc::clone(&manager);
        let stale_resume = tokio::spawn(async move {
            manager_stale
                .resume_asset_lock(&out_point, Some(Duration::from_millis(10)))
                .await
        });

        // Wait until the resume has actually taken its `Built` snapshot.
        // Without this the finalize below could land first and the resume
        // would read `ChainLocked` directly — never exercising the race.
        arrived.notified().await;

        // 2. While it is parked, another flow finalizes the SAME row to
        //    `ChainLocked` with a proof attached — exactly what the winning
        //    resume's step 3 does.
        let chain_proof = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 1234,
            out_point,
        });
        let cs = manager
            .advance_asset_lock_status(
                &out_point,
                AssetLockStatus::ChainLocked,
                Some(chain_proof.clone()),
            )
            .await
            .expect("finalize the row");
        manager.queue_asset_lock_changeset(cs);

        // 3. Release the stale resume. Without the compare-and-set it would
        //    now write `Broadcast` over the finalized row.
        release.notify_one();
        let resumed = stale_resume.await.expect("stale resume task joined");

        // The stale caller re-dispatched into the already-have-a-proof arm:
        // it neither re-broadcast nor waited for a proof. Both are
        // observable — `wait_for_proof` would have returned `FinalityTimeout`
        // against the 10ms deadline (no SPV record exists in this fixture),
        // and the `Built`/`Broadcast` arms both broadcast before waiting.
        //
        // The resume still fails at its LAST step (step 4, credit-output
        // path re-derivation): this fixture's funding-account address pool
        // does not retain the peeked credit-output address, so
        // `rederive_credit_output_path` cannot resolve it. That is a
        // pre-existing fixture limitation unrelated to the race — reaching
        // it at all proves the proof was reused, since a resume that waited
        // would have failed earlier with `FinalityTimeout`.
        assert!(
            matches!(
                resumed,
                Err(PlatformWalletError::AssetLockTransaction(ref m))
                    if m.contains("not found in funding account")
            ),
            "stale resume must reach credit-output re-derivation (proving it \
             reused the attached proof rather than waiting for a new one), \
             got {resumed:?}"
        );
        assert_eq!(
            broadcaster.call_count.load(Ordering::SeqCst),
            1,
            "only the create-path broadcast may have happened: a stale resume \
             that re-dispatched from ChainLocked must not re-broadcast"
        );

        // The row is still finalized: status never regressed to `Broadcast`
        // and the proof is intact.
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            let lock = info
                .tracked_asset_locks
                .get(&out_point)
                .expect("row still tracked");
            assert_eq!(
                lock.status,
                AssetLockStatus::ChainLocked,
                "stale Built snapshot must not downgrade the finalized row"
            );
            assert_eq!(
                lock.proof.as_ref(),
                Some(&chain_proof),
                "the concurrently-attached proof must survive"
            );
        }

        // No persisted changeset carries the inconsistent
        // `Broadcast + Some(proof)` pair.
        let stored = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex");
        let inconsistent = stored
            .iter()
            .filter_map(|cs| cs.asset_locks.as_ref())
            .filter_map(|al| al.asset_locks.get(&out_point))
            .any(|entry| entry.status == AssetLockStatus::Broadcast && entry.proof.is_some());
        assert!(
            !inconsistent,
            "no changeset may persist a Broadcast row with a proof attached"
        );
    }

    /// Broadcaster that stages the create-side counterpart of the race:
    /// "during" the create path's own broadcast — i.e. while
    /// `broadcast_funded_asset_lock` is parked in this await, after it
    /// tracked the row as `Built` and before its `Built` → `Broadcast`
    /// promotion — a concurrent `resume_asset_lock` picks the same outpoint
    /// up, broadcasts, obtains a proof and finalizes the row to
    /// `ChainLocked` (its step 3). Then the original broadcast returns `Ok`.
    ///
    /// Mutating the row in place stands in for that winning resume without
    /// needing a second task: the point under test is what the create path
    /// writes on its way out, and doing it inline makes the interleave
    /// deterministic rather than scheduler-dependent. The finalize is
    /// deliberately in-memory only, so any persisted
    /// `Broadcast + Some(proof)` changeset can only have come from the
    /// create path itself.
    struct FinalizeDuringBroadcastBroadcaster {
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        call_count: AtomicUsize,
        /// The proof the simulated resume attaches, so the test can assert
        /// it survived byte-for-byte.
        proof: Mutex<Option<dpp::prelude::AssetLockProof>>,
    }

    #[async_trait]
    impl TransactionBroadcaster for FinalizeDuringBroadcastBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet present");
            let lock = info
                .tracked_asset_locks
                .values_mut()
                .next()
                .expect("Built row tracked before broadcast");
            assert_eq!(
                lock.status,
                AssetLockStatus::Built,
                "create path must have tracked the row as Built before broadcasting"
            );
            let proof = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: 4016,
                out_point: lock.out_point,
            });
            lock.status = AssetLockStatus::ChainLocked;
            lock.proof = Some(proof.clone());
            *self.proof.lock().expect("staged proof mutex") = Some(proof);
            drop(wm);

            Ok(transaction.txid())
        }
    }

    /// Regression: the create path's `Built` → `Broadcast` promotion must be
    /// a compare-and-set too, not just resume's.
    ///
    /// `broadcast_funded_asset_lock` tracks the row as `Built`, then awaits
    /// `broadcaster.broadcast(&tx)` — an unbounded network call. A
    /// concurrent `resume_asset_lock` (the FFI catch-up scanner and the
    /// funding resolver both drive one for any tracked outpoint) can pick
    /// the row up in that window, broadcast the same transaction, obtain a
    /// proof and finalize it to `InstantSendLocked` / `ChainLocked`. When
    /// the original broadcast then returns `Ok`, an unconditional
    /// `advance_asset_lock_status(.., Broadcast, None)` downgrades that
    /// finalized row — and because `None` leaves the existing proof
    /// attached, it recreates the inconsistent `Broadcast + Some(proof)`
    /// state and persists it (changesets are last-write-wins). A later
    /// resume takes the `Broadcast` arm and waits for a proof the row
    /// already holds, unbounded for the user-facing funding flows.
    ///
    /// With the compare-and-set the promotion is skipped, the finalized
    /// status and proof survive, and the successful broadcast is still
    /// reported as success — this create-only half has nothing to
    /// re-dispatch, since the proof wait lives in
    /// `wait_for_funded_asset_lock_proof`.
    #[tokio::test]
    async fn create_broadcast_does_not_downgrade_a_row_finalized_during_the_broadcast() {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;

        let broadcaster = Arc::new(FinalizeDuringBroadcastBroadcaster {
            wallet_manager: Arc::clone(&wallet_manager),
            wallet_id,
            call_count: AtomicUsize::new(0),
            proof: Mutex::new(None),
        });
        let persistence = Arc::new(CapturingPersistence::default());
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::clone(&broadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));

        // The broadcast itself succeeds, so the create half reports success
        // even though it did not own the final status.
        let (_path, out_point) = manager
            .broadcast_funded_asset_lock(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("a successful broadcast must still be reported as success");

        assert_eq!(
            broadcaster.call_count.load(Ordering::SeqCst),
            1,
            "the create-only half must broadcast exactly once — it has no \
             re-dispatch path and must not re-broadcast after observing the \
             advanced row"
        );

        let staged_proof = broadcaster
            .proof
            .lock()
            .expect("staged proof mutex")
            .clone()
            .expect("the simulated resume staged a proof");

        // The finalized row survives the create path's exit.
        {
            let wm = wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            let lock = info
                .tracked_asset_locks
                .get(&out_point)
                .expect("row still tracked");
            assert_eq!(
                lock.status,
                AssetLockStatus::ChainLocked,
                "a row finalized during the broadcast must not be downgraded \
                 to Broadcast on the way out"
            );
            assert_eq!(
                lock.proof.as_ref(),
                Some(&staged_proof),
                "the concurrently-attached proof must survive"
            );
        }

        // …and the inconsistent pair was never persisted either.
        let stored = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex");
        let inconsistent = stored
            .iter()
            .filter_map(|cs| cs.asset_locks.as_ref())
            .filter_map(|al| al.asset_locks.get(&out_point))
            .any(|entry| entry.status == AssetLockStatus::Broadcast && entry.proof.is_some());
        assert!(
            !inconsistent,
            "no changeset may persist a Broadcast row with a proof attached"
        );
    }

    /// Drives a promoter into the post-CAS / pre-enqueue window, finalizes
    /// and enqueues the same row from another task while it is parked, then
    /// releases it — and returns the durable row that results.
    ///
    /// Shared by both `Built` → `Broadcast` regression tests below so the
    /// create and resume paths are proven against the *same* interleave.
    /// The promotion is invoked directly rather than through
    /// `broadcast_funded_asset_lock` / `resume_asset_lock` because it is
    /// the single primitive both callers now route through — see the
    /// per-path tests for the proof that they do.
    async fn durable_row_after_promote_finalize_interleave<B: TransactionBroadcaster + 'static>(
        manager: Arc<AssetLockManager<B>>,
        persistence: Arc<CapturingPersistence>,
        out_point: OutPoint,
    ) -> (Option<AssetLockEntry>, dpp::prelude::AssetLockProof) {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

        // 1. Park a promoter between its compare-and-set and its enqueue —
        //    the exact window in which the stale `Broadcast + None`
        //    snapshot had not yet been handed to the persister.
        let arrived = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        *manager
            .promote_post_cas_gate
            .lock()
            .expect("promote post-CAS gate mutex") = Some(PromotePostCasGate {
            arrived: Arc::clone(&arrived),
            release: Arc::clone(&release),
        });

        let manager_promoter = Arc::clone(&manager);
        let promoter = tokio::spawn(async move {
            manager_promoter
                .promote_built_to_broadcast(&out_point)
                .await
        });

        // The promoter has mutated the row to `Broadcast` in memory and is
        // holding before the enqueue. Its snapshot is now genuinely stale
        // with respect to anything written next.
        arrived.notified().await;

        // 2. Finalize the SAME row to `ChainLocked + proof` and enqueue
        //    that — what a winning concurrent flow's step 3 does. Runs in
        //    its own task: with the fix this call BLOCKS on the ordering
        //    mutex until the promoter enqueues, so awaiting it inline here
        //    would deadlock against the `release` below.
        let chain_proof = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 4016,
            out_point,
        });

        // Sampled BEFORE the spawn, deliberately. A finalizer that got
        // all the way to its enqueue before we sampled would fold that
        // store into the baseline, after which `> baseline` can never
        // become true — and `status_serial_waiters` has already fallen
        // back to zero, so neither exit condition can ever fire. The
        // rendezvous below would spin forever, reporting a harness
        // timeout instead of the ordering inversion it exists to catch —
        // and it is precisely when the serialization REGRESSES that the
        // finalizer becomes free to enqueue early, so the sampling was
        // blind in the one direction it has to be sharpest.
        //
        // Sampling after the spawn happened to hold only because nothing
        // suspends between the two, so the finalizer cannot be polled
        // first. Nothing enforced that: one `.await` above the sample —
        // or this test moving to the `multi_thread` flavor already used
        // elsewhere in this file — reintroduces the hang.
        let queued_before_finalize = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex")
            .len();

        let manager_finalizer = Arc::clone(&manager);
        let finalize_proof = chain_proof.clone();
        let finalizer = tokio::spawn(async move {
            manager_finalizer
                .advance_asset_lock_status(
                    &out_point,
                    AssetLockStatus::ChainLocked,
                    Some(finalize_proof),
                )
                .await
        });

        // Wait until the finalizer has provably had its chance to enqueue
        // first — not merely until some time has passed. Exactly one of
        // two states must be observed, mirroring the build-gate test
        // above:
        //
        // - the finalizer ENQUEUED (a new changeset landed): the
        //   unserialized behavior, where nothing held it back and its
        //   newer snapshot is already durable. The promoter's older
        //   snapshot then lands last on release and the caller's ordering
        //   assertion fires — deterministically, because we release only
        //   after observing the store, never before it happened;
        // - the finalizer is QUEUED on `status_persist_serial` (waiter
        //   gauge non-zero): the fixed behavior. The promoter holds that
        //   mutex across its parked window, so the finalizer came to rest
        //   at the boundary and provably cannot enqueue before we release.
        //
        // A sleep distinguished neither: "no new changeset yet" could just
        // mean the finalizer had not been scheduled, so the pre-fix
        // implementation could enqueue in the non-regressing order after
        // the release and falsely pass.
        //
        // Bounded, and with a third exit for a finalizer that finished
        // without producing either signal — it panicked before reaching
        // the mutex, or a future edit made it return early. Either leaves
        // both gauges reading exactly like "not scheduled yet" forever;
        // breaking here hands the real cause to the joins below instead
        // of hanging on a state that can no longer change.
        let mut rendezvous = None;
        for _ in 0..2_000 {
            let finalizer_enqueued = persistence
                .stored
                .lock()
                .expect("capturing persistence mutex")
                .len()
                > queued_before_finalize;
            let finalizer_blocked = manager
                .status_serial_waiters
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 1;
            if finalizer_enqueued || finalizer_blocked || finalizer.is_finished() {
                rendezvous = Some(finalizer_enqueued || finalizer_blocked);
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let rendezvous = rendezvous.unwrap_or_else(|| {
            panic!(
                "timed out waiting for the finalizer to either enqueue or come \
                 to rest on status_persist_serial — the promoter is parked \
                 holding that mutex, so one of the two must happen"
            )
        });

        // 3. Release the stale promoter.
        release.notify_one();
        promoter
            .await
            .expect("promoter task joined")
            .expect("promotion must not error");
        finalizer
            .await
            .expect("finalizer task joined")
            .expect("finalize must not error");
        assert!(
            rendezvous,
            "the finalizer ran to completion without ever enqueueing or \
             blocking on status_persist_serial, so this interleave never \
             exercised the ordering it claims to assert — the joins above \
             passed, so it returned Ok having queued nothing"
        );

        (persistence.durable_asset_lock(&out_point), chain_proof)
    }

    /// Regression (persistence ordering): a `Built` → `Broadcast` promotion
    /// must not let its older snapshot reach the persister AFTER a
    /// concurrent finalize enqueued a newer proof-bearing one.
    ///
    /// The compare-and-set alone fixed only the in-memory half. The
    /// promoter mutated the row under `wallet_manager.write()`, returned a
    /// `Broadcast` changeset, and RELEASED that lock before the caller
    /// enqueued it. In that post-CAS / pre-enqueue window another flow
    /// could take the wallet lock, finalize the row to `ChainLocked +
    /// proof`, and enqueue that snapshot first — after which the delayed
    /// promoter enqueued its `Broadcast + None` one last.
    ///
    /// Nothing downstream repairs the inversion: `FFIPersister::store_round`
    /// serializes rounds by acquisition order (preserving the reversal),
    /// `AssetLockChangeSet::merge` is last-write-wins, and Swift's
    /// `persistAssetLocks` upsert overwrites `statusRaw` / `proofBytes`
    /// unconditionally. So memory stayed `ChainLocked` while the DURABLE row
    /// regressed to `Broadcast` with no proof — and since the load path
    /// treats `statusRaw < 2` as still-pending, a restart resumed a lock
    /// whose proof it already had.
    ///
    /// Pre-fix this test observes the regressed durable row and fails; the
    /// shared `status_persist_serial` makes mutation+enqueue one unit, so
    /// the finalize now blocks until the promoter has enqueued and the
    /// durable order matches the in-memory order.
    #[tokio::test]
    async fn promotion_cannot_enqueue_a_stale_snapshot_after_a_concurrent_finalize() {
        // A `MaybeSent` create leaves the row `Built` — the state a
        // promotion acts on.
        let broadcaster = Arc::new(CountingMaybeSentBroadcaster {
            call_count: AtomicUsize::new(0),
        });
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::clone(&broadcaster)).await;
        let _ = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        let out_point = {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            let (op, lock) = info
                .tracked_asset_locks
                .iter()
                .next()
                .expect("built row tracked");
            assert_eq!(lock.status, AssetLockStatus::Built);
            *op
        };

        let (durable, chain_proof) = durable_row_after_promote_finalize_interleave(
            Arc::clone(&manager),
            Arc::clone(&persistence),
            out_point,
        )
        .await;

        // In-memory finality was never in question — it is the durable
        // state that regressed.
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            let lock = info
                .tracked_asset_locks
                .get(&out_point)
                .expect("row still tracked");
            assert_eq!(lock.status, AssetLockStatus::ChainLocked);
        }

        let durable = durable.expect("a durable row must exist");
        assert_eq!(
            durable.status,
            AssetLockStatus::ChainLocked,
            "the durable row must not regress below the finalized in-memory \
             status: a stale promoter enqueued its Broadcast snapshot after \
             the finalize, and last-write-wins made it the row a restart reads"
        );
        assert_eq!(
            durable.proof.as_ref(),
            Some(&chain_proof),
            "the durable row must keep the finalized proof — dropping it makes \
             a restart re-wait for a proof the wallet already had"
        );
    }

    /// The resume path routes its `Built` → `Broadcast` promotion through
    /// the same serialized primitive as the create path.
    ///
    /// The test above proves the primitive orders mutation before enqueue;
    /// this one pins that `resume_asset_lock` actually goes through it, so
    /// the fix cannot regress to covering only the create caller. Asserted
    /// structurally rather than by re-running the interleave: the resume
    /// promotion happens mid-call, so parking it on the post-CAS gate would
    /// stall the whole resume rather than isolate the window.
    #[tokio::test]
    async fn resume_promotion_enqueues_under_the_shared_ordering_mutex() {
        let broadcaster = Arc::new(CountingMaybeSentBroadcaster {
            call_count: AtomicUsize::new(0),
        });
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::clone(&broadcaster)).await;
        let _ = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        let out_point = {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            *info
                .tracked_asset_locks
                .keys()
                .next()
                .expect("built row tracked")
        };

        // Hold the ordering mutex, then start a resume. Its promotion must
        // block on the mutex, which means it cannot have enqueued anything.
        let serial = manager.status_persist_serial.lock().await;
        let queued_before = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex")
            .len();

        let manager_resume = Arc::clone(&manager);
        let resume = tokio::spawn(async move {
            manager_resume
                .resume_asset_lock(&out_point, Some(Duration::from_millis(10)))
                .await
        });

        // Rendezvous on the resume actually REACHING the ordering
        // boundary, rather than sleeping and hoping it got there. The
        // waiter gauge is incremented by the production lock helper
        // before its `lock().await` and dropped on acquisition, so a
        // count of 1 while this test holds the mutex means the resume's
        // promotion is queued on it and cannot have enqueued anything.
        //
        // This is what makes the "nothing was queued" assertion below
        // evidence of serialization. After a sleep it was not: an
        // unchanged changeset count could equally mean the resume task
        // had never been scheduled. It also fails loudly if the resume
        // path ever stops routing its promotion through the mutex —
        // the gauge would stay at zero and the wait would panic, where
        // the sleep version would have read the resulting silence as
        // success.
        manager.await_status_serial_waiters(1).await;

        assert_eq!(
            persistence
                .stored
                .lock()
                .expect("capturing persistence mutex")
                .len(),
            queued_before,
            "a resume whose promotion is blocked on `status_persist_serial` \
             must not have enqueued an asset-lock changeset — if it did, the \
             resume path is bypassing the shared ordering mutex and can still \
             reorder against a concurrent finalize"
        );
        assert!(
            manager.status_persist_serial.try_lock().is_err(),
            "the ordering mutex must still be held by this test — otherwise the \
             assertion above proves nothing about serialization"
        );

        drop(serial);
        let _ = resume.await.expect("resume task joined");

        // Once released the promotion completes and its snapshot is durable.
        let durable = persistence
            .durable_asset_lock(&out_point)
            .expect("the released resume must have enqueued its promotion");
        assert!(
            durable.status != AssetLockStatus::Built,
            "the resume's promotion must have advanced the durable row past \
             Built, got {:?}",
            durable.status
        );
    }

    /// Regression (lifecycle): retiring an `AssetLockManager` must be a
    /// BARRIER around the mutate→enqueue unit, not a flag flipped
    /// whenever the removal happens to run.
    ///
    /// Two properties, both asserted here on a rendezvous rather than a
    /// sleep:
    ///
    /// 1. **Deactivation waits.** A promoter parked between its
    ///    compare-and-set and its enqueue holds `status_persist_serial`.
    ///    `deactivate` must queue behind it — if it could flip the flag
    ///    in that window, the promoter would resume, fail its own
    ///    (already-taken) check or, worse, be interrupted with the row
    ///    mutated in memory and its changeset never handed to the
    ///    persister: exactly the mid-unit tear this mutex exists to
    ///    prevent.
    /// 2. **Nothing lands afterwards.** Once `deactivate` returns, every
    ///    status mutate→enqueue primitive refuses with
    ///    `AssetLockManagerInactive` and queues nothing — including the
    ///    ones whose callers only arrive here after an unbounded
    ///    `broadcast` / proof wait, which is the span a `remove_wallet`
    ///    plus a same-mnemonic re-import completes inside.
    ///
    /// The rendezvous is the production `status_serial_waiters` gauge:
    /// while this test's promoter provably holds the mutex, a waiter
    /// count of 1 means `deactivate` came to rest at the boundary and
    /// cannot have flipped the flag. A sleep would have proved nothing —
    /// "the flag is still set" could equally mean the task had not been
    /// scheduled yet.
    #[tokio::test]
    async fn deactivation_waits_for_the_in_flight_unit_then_refuses_every_later_mutation() {
        let broadcaster = Arc::new(CountingMaybeSentBroadcaster {
            call_count: AtomicUsize::new(0),
        });
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::clone(&broadcaster)).await;
        // `MaybeSent` leaves the row at `Built` — the state a promotion
        // acts on.
        let _ = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        let out_point = {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            let (op, lock) = info
                .tracked_asset_locks
                .iter()
                .next()
                .expect("built row tracked");
            assert_eq!(lock.status, AssetLockStatus::Built);
            *op
        };

        // 1. Park a promoter post-CAS / pre-enqueue. It holds
        //    `status_persist_serial` for the whole parked window.
        let arrived = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        *manager
            .promote_post_cas_gate
            .lock()
            .expect("promote post-CAS gate mutex") = Some(PromotePostCasGate {
            arrived: Arc::clone(&arrived),
            release: Arc::clone(&release),
        });
        let manager_promoter = Arc::clone(&manager);
        let promoter = tokio::spawn(async move {
            manager_promoter
                .promote_built_to_broadcast(&out_point)
                .await
        });
        arrived.notified().await;

        let queued_before_deactivate = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex")
            .len();

        // 2. Start the removal-side retirement. It must block.
        let manager_deactivate = Arc::clone(&manager);
        let deactivator = tokio::spawn(async move { manager_deactivate.deactivate().await });

        // Rendezvous on `deactivate` REACHING the ordering boundary.
        manager.await_status_serial_waiters(1).await;

        assert!(
            manager.active.load(Ordering::SeqCst),
            "`deactivate` must not retire the manager while an in-flight \
             mutate→enqueue unit still holds `status_persist_serial` — it \
             would strand the promoter with the row mutated in memory and \
             its changeset never enqueued"
        );
        assert!(
            manager.status_persist_serial.try_lock().is_err(),
            "the parked promoter must still hold the ordering mutex — \
             otherwise the assertion above proves nothing about the barrier"
        );

        // 3. Release the promoter; its unit must complete in full.
        release.notify_one();
        let promotion = promoter
            .await
            .expect("promoter task joined")
            .expect("a promotion that started before the removal must complete");
        assert!(
            matches!(promotion, super::BuiltPromotion::Promoted(_)),
            "the in-flight promotion must have promoted the row, got {promotion:?}"
        );
        deactivator.await.expect("deactivator task joined");

        assert!(
            persistence
                .stored
                .lock()
                .expect("capturing persistence mutex")
                .len()
                > queued_before_deactivate,
            "the in-flight unit's changeset must have reached the persister \
             before deactivation completed — a retirement that cut in would \
             leave memory ahead of the durable row"
        );
        assert_eq!(
            persistence
                .durable_asset_lock(&out_point)
                .expect("the promoted row must be durable")
                .status,
            AssetLockStatus::Broadcast,
        );

        // 4. Every status mutate→enqueue primitive now refuses, and
        //    queues nothing.
        let queued_after_deactivate = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex")
            .len();

        let advance = manager
            .advance_asset_lock_status(&out_point, AssetLockStatus::ChainLocked, None)
            .await;
        assert!(
            matches!(
                advance,
                Err(PlatformWalletError::AssetLockManagerInactive(_))
            ),
            "a finalize arriving after retirement must be refused, got {advance:?}"
        );

        let promote = manager.promote_built_to_broadcast(&out_point).await;
        assert!(
            matches!(
                promote,
                Err(PlatformWalletError::AssetLockManagerInactive(_))
            ),
            "a promotion arriving after retirement must be refused, got {promote:?}"
        );

        let untrack = manager.untrack_asset_lock(&out_point).await;
        assert!(
            matches!(
                untrack,
                Err(PlatformWalletError::AssetLockManagerInactive(_))
            ),
            "an untrack arriving after retirement must be refused — it would \
             DELETE the durable row, got {untrack:?}"
        );

        let consume = manager.consume_asset_lock(&out_point).await;
        assert!(
            matches!(
                consume,
                Err(PlatformWalletError::AssetLockManagerInactive(_))
            ),
            "a consume arriving after retirement must be refused, got {consume:?}"
        );

        assert_eq!(
            persistence
                .stored
                .lock()
                .expect("capturing persistence mutex")
                .len(),
            queued_after_deactivate,
            "no refused operation may enqueue anything — a retired manager \
             that still reaches the shared persister can overwrite or delete \
             a replacement wallet's rows"
        );
        assert!(
            persistence.removed_outpoints().is_empty(),
            "the refused untrack must not have queued a row deletion"
        );
    }

    /// Regression (monotonicity): a delayed `InstantSendLocked + IS proof`
    /// write must not downgrade a row a concurrent flow already finalized
    /// to `ChainLocked + CL proof` — in memory or durably.
    ///
    /// Serialization alone does not close this. `status_persist_serial`
    /// makes each writer's mutation and enqueue one indivisible unit, so
    /// the durable order matches the in-memory order — but the two
    /// proof-bearing writes come from INDEPENDENT waiters
    /// (`wait_for_proof` returns whichever SPV event fires first; the
    /// IS→CL upgrade paths can finalize a ChainLock while an earlier IS
    /// waiter is still parked), so nothing orders them. Released after
    /// the finalize, the IS waiter's write was internally consistent and
    /// perfectly serialized — and still strictly regressed the row: the
    /// status fell below the `>= InstantSendLocked` predicates the
    /// catch-up scanner and the "ready to fund" UI filter on, and,
    /// because the caller passes `Some(is_proof)`, the ChainLock proof
    /// was swapped out for the IS one that a rejection retry had
    /// upgraded AWAY from. A restart then reloaded the weaker proof.
    ///
    /// Interleave, with no sleep anywhere in it:
    ///
    /// 1. an IS writer enters `advance_asset_lock_status` and parks on
    ///    the pre-lock gate — before the ordering mutex, so the finalize
    ///    below can run its whole unit rather than queueing behind it;
    /// 2. the test finalizes the SAME row to `ChainLocked` + chain proof
    ///    and awaits that call, so the stronger write is provably
    ///    complete and enqueued before the IS writer resumes;
    /// 3. the IS writer is released and joined.
    ///
    /// Then both halves of the shared state must still read
    /// `ChainLocked` + chain proof, and the refused write must have
    /// enqueued nothing at all.
    ///
    /// Caller-return semantics are asserted too: the refusal is `Ok`, not
    /// an error, and carries an empty changeset. The delayed caller's own
    /// IS proof is still valid evidence for the submission it is about to
    /// make (every production caller returns/uses the proof it already
    /// holds, never anything from the changeset) — the guard constrains
    /// SHARED state, not what the caller may do with the proof in hand.
    #[tokio::test]
    async fn a_late_instant_send_write_cannot_downgrade_a_chain_locked_row() {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;

        use crate::wallet::asset_lock::manager::AdvancePreLockGate;

        // `MaybeSent` leaves the row tracked at `Built`; the test drives
        // the status transitions itself.
        let broadcaster = Arc::new(CountingMaybeSentBroadcaster {
            call_count: AtomicUsize::new(0),
        });
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::clone(&broadcaster)).await;
        let _ = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        let out_point = {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            let (op, lock) = info
                .tracked_asset_locks
                .iter()
                .next()
                .expect("built row tracked");
            assert_eq!(lock.status, AssetLockStatus::Built);
            *op
        };

        // Advance to `Broadcast` first, so the IS write under test is a
        // legal forward transition in isolation — its refusal below is
        // then attributable to the concurrent finalize, not to the write
        // being backwards on its own.
        manager
            .advance_asset_lock_status(&out_point, AssetLockStatus::Broadcast, None)
            .await
            .expect("Broadcast is a forward transition from Built");

        let chain_proof = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 4016,
            out_point,
        });
        let instant_proof = dpp::prelude::AssetLockProof::Instant(InstantAssetLockProof::new(
            dashcore::ephemerealdata::instant_lock::InstantLock::default(),
            dashcore::Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: None,
            },
            0,
        ));
        assert_ne!(
            instant_proof, chain_proof,
            "the two proofs must be distinguishable for the assertions below \
             to mean anything"
        );

        // 1. Park an `InstantSendLocked` writer before the ordering
        //    mutex. The gate is one-shot (taken, not cloned), so the
        //    finalize in step 2 — same method — runs straight through
        //    instead of parking too.
        let arrived = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        *manager
            .advance_pre_lock_gate
            .lock()
            .expect("advance pre-lock gate mutex") = Some(AdvancePreLockGate {
            arrived: Arc::clone(&arrived),
            release: Arc::clone(&release),
        });

        let manager_is_writer = Arc::clone(&manager);
        let is_writer_proof = instant_proof.clone();
        let is_writer = tokio::spawn(async move {
            manager_is_writer
                .advance_asset_lock_status(
                    &out_point,
                    AssetLockStatus::InstantSendLocked,
                    Some(is_writer_proof),
                )
                .await
        });

        // The IS writer has provably entered the method and cannot
        // proceed. This is the arrival signal, not a sleep: it fires from
        // inside the call under test, so "the finalize below races a
        // parked IS write" is a fact rather than a scheduling hope.
        arrived.notified().await;

        // 2. Finalize the row to `ChainLocked` + chain proof and AWAIT
        //    it. Awaiting inline is safe (and is the point): the IS
        //    writer parks before taking `status_persist_serial`, so it
        //    holds nothing this call needs. On return the stronger write
        //    is complete — mutated in memory and enqueued.
        let finalize_cs = manager
            .advance_asset_lock_status(
                &out_point,
                AssetLockStatus::ChainLocked,
                Some(chain_proof.clone()),
            )
            .await
            .expect("the ChainLock finalize must succeed");
        assert!(
            !<crate::changeset::changeset::AssetLockChangeSet as crate::changeset::Merge>::is_empty(
                &finalize_cs
            ),
            "the finalize must have produced a real changeset — otherwise the \
             race below is not against a completed stronger write"
        );

        let queued_after_finalize = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex")
            .len();

        // 3. Release the delayed IS writer.
        release.notify_one();
        let refused = is_writer
            .await
            .expect("IS writer task joined")
            .expect("a refused downgrade must be reported as Ok, not an error");
        assert!(
            <crate::changeset::changeset::AssetLockChangeSet as crate::changeset::Merge>::is_empty(
                &refused
            ),
            "the refused downgrade must return an EMPTY changeset — a populated \
             one would be replayed as an `InstantSendLocked + IS proof` row by \
             any caller that queued it, reintroducing the regression"
        );

        // In-memory state kept the stronger status AND the stronger proof.
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            let lock = info
                .tracked_asset_locks
                .get(&out_point)
                .expect("row still tracked");
            assert_eq!(
                lock.status,
                AssetLockStatus::ChainLocked,
                "a late InstantSendLocked write must not roll the in-memory row \
                 back below ChainLocked — it would fall under the \
                 `>= InstantSendLocked` predicates the catch-up scanner and the \
                 ready-to-fund filter use"
            );
            assert_eq!(
                lock.proof.as_ref(),
                Some(&chain_proof),
                "the ChainLock proof must survive: it is what an IS-rejection \
                 retry upgraded TO, and reinstating the IS proof re-arms the \
                 very rejection that upgrade resolved"
            );
        }

        // Durable state — replayed through the real downstream semantics
        // (round order, last-write-wins merge, unconditional upsert) —
        // agrees, and the refused write reached the persister not at all.
        let durable = persistence
            .durable_asset_lock(&out_point)
            .expect("the finalized row must be durable");
        assert_eq!(
            durable.status,
            AssetLockStatus::ChainLocked,
            "the durable row must stay ChainLocked; the load path treats a \
             regressed status as still-pending and would resume a lock whose \
             proof it already had"
        );
        assert_eq!(
            durable.proof.as_ref(),
            Some(&chain_proof),
            "the durable proof must stay the ChainLock one — a restart reloads \
             exactly this row"
        );
        assert_eq!(
            persistence
                .stored
                .lock()
                .expect("capturing persistence mutex")
                .len(),
            queued_after_finalize,
            "the refused downgrade must enqueue nothing at all; even a \
             correctly-ordered stale round is last-write-wins downstream"
        );
    }

    /// The monotonicity guard must refuse only BACKWARD writes. Forward
    /// transitions and equal-rank proof attachment/refresh — the shapes
    /// the production callers actually depend on — must still mutate and
    /// enqueue.
    ///
    /// Equal-rank matters as much as forward here, and for two live
    /// reasons: `resolve_status_with_in_memory` classifies a tx with an
    /// InstantSend context as `InstantSendLocked` with NO proof (it has
    /// no IS-lock data), so the first real proof arrives as a same-status
    /// write; and the IS→CL upgrade paths re-write `ChainLocked` with a
    /// freshly-built ChainLock proof at a newer height. A guard written
    /// as `<=` instead of `<` would silently drop both.
    #[tokio::test]
    async fn monotonicity_guard_allows_forward_and_same_status_proof_writes() {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

        let broadcaster = Arc::new(CountingMaybeSentBroadcaster {
            call_count: AtomicUsize::new(0),
        });
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::clone(&broadcaster)).await;
        let _ = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        let out_point = {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            *info
                .tracked_asset_locks
                .keys()
                .next()
                .expect("built row tracked")
        };

        // Forward, one rank at a time, ending on a proof-bearing write.
        for status in [
            AssetLockStatus::Broadcast,
            AssetLockStatus::InstantSendLocked,
        ] {
            let cs = manager
                .advance_asset_lock_status(&out_point, status.clone(), None)
                .await
                .expect("a forward transition must succeed");
            assert!(
                !<crate::changeset::changeset::AssetLockChangeSet as crate::changeset::Merge>::is_empty(&cs),
                "the forward transition to {status:?} must produce a changeset"
            );
        }

        // Same-status proof ATTACHMENT: the row is already
        // `InstantSendLocked` with no proof (exactly what
        // `resolve_status_with_in_memory` leaves behind), and the first
        // real proof arrives at equal rank.
        let chain_proof_low = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 4016,
            out_point,
        });
        let attach = manager
            .advance_asset_lock_status(
                &out_point,
                AssetLockStatus::InstantSendLocked,
                Some(chain_proof_low.clone()),
            )
            .await
            .expect("same-status proof attachment must succeed");
        assert!(
            !<crate::changeset::changeset::AssetLockChangeSet as crate::changeset::Merge>::is_empty(
                &attach
            ),
            "attaching the first proof at equal rank must produce a changeset — \
             `resolve_status_with_in_memory` sets InstantSendLocked with no \
             proof, so this is how that row ever gets one"
        );

        // Forward to `ChainLocked`, then a same-status REFRESH with a
        // ChainLock proof at a newer height — the IS→CL upgrade shape.
        manager
            .advance_asset_lock_status(
                &out_point,
                AssetLockStatus::ChainLocked,
                Some(chain_proof_low),
            )
            .await
            .expect("advancing to ChainLocked must succeed");
        let chain_proof_high = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 4129,
            out_point,
        });
        let refresh = manager
            .advance_asset_lock_status(
                &out_point,
                AssetLockStatus::ChainLocked,
                Some(chain_proof_high.clone()),
            )
            .await
            .expect("same-status proof refresh must succeed");
        assert!(
            !<crate::changeset::changeset::AssetLockChangeSet as crate::changeset::Merge>::is_empty(
                &refresh
            ),
            "re-writing ChainLocked with a freshly-upgraded proof must produce a \
             changeset — this is the IS-rejection retry path"
        );

        // Both halves reflect the refreshed proof.
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet present");
            let lock = info
                .tracked_asset_locks
                .get(&out_point)
                .expect("row still tracked");
            assert_eq!(lock.status, AssetLockStatus::ChainLocked);
            assert_eq!(
                lock.proof.as_ref(),
                Some(&chain_proof_high),
                "the refreshed proof must be the one in memory"
            );
        }
        let durable = persistence
            .durable_asset_lock(&out_point)
            .expect("the row must be durable");
        assert_eq!(durable.status, AssetLockStatus::ChainLocked);
        assert_eq!(
            durable.proof.as_ref(),
            Some(&chain_proof_high),
            "the refreshed proof must be the durable one"
        );
    }

    /// Whether a per-index `IdentityTopUp` account exists for
    /// `identity_index`, on BOTH halves of the wallet.
    ///
    /// The probe the two regression tests below use, because deriving
    /// that account is the FIRST thing a build mutates
    /// (`ensure_identity_topup_account`, before any address is consumed
    /// or any input reserved). Still absent after a refused build means
    /// the build never touched the wallet at all, not that it touched it
    /// and rolled back.
    async fn topup_account_present<B: TransactionBroadcaster + ?Sized>(
        manager: &AssetLockManager<B>,
        identity_index: u32,
    ) -> (bool, bool) {
        let wm = manager.wallet_manager.read().await;
        let (wallet, info) = wm
            .get_wallet_and_info(&manager.wallet_id)
            .expect("wallet present");
        (
            wallet.accounts.identity_topup.contains_key(&identity_index),
            info.core_wallet
                .accounts
                .identity_topup
                .contains_key(&identity_index),
        )
    }

    /// Regression: a retired manager must refuse to BUILD, not only to
    /// mutate status rows.
    ///
    /// Wallet ids are deterministic in (seed, network), so deleting a
    /// wallet and re-importing the same mnemonic produces the SAME id
    /// over a fresh `PlatformWalletInfo` and a fresh `AssetLockManager`.
    /// A handle retained across that boundary — an `Arc<PlatformWallet>`
    /// the FFI still holds and can call
    /// `dash_platform_wallet_build_asset_lock_transaction` on — resolves
    /// `self.wallet_id` to the REPLACEMENT generation. `build_asset_lock_
    /// transaction` used to go straight to `wallet_manager.write()` with
    /// no activity check at all, so the retired handle would derive a
    /// top-up account into the replacement, consume one of its funding
    /// addresses, and reserve its UTXOs — on behalf of a wallet the user
    /// deleted. The consumed index is durable and unrecoverable: these
    /// accounts fund OP_RETURN-payload credit outputs that never appear
    /// as on-chain UTXOs, so SPV can never rediscover them.
    ///
    /// Retirement is what the removal path installs, so this drives it
    /// directly. The wallet still resolvable under the id afterwards
    /// stands in for the replacement generation — the manager cannot
    /// tell the two apart, which is precisely why the flag has to be the
    /// gate.
    #[tokio::test]
    async fn a_retired_manager_refuses_to_build_against_the_wallet_under_its_id() {
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysOkBroadcaster)).await;

        const TOPUP_INDEX: u32 = 7;
        assert_eq!(
            topup_account_present(&manager, TOPUP_INDEX).await,
            (false, false),
            "precondition: the per-index top-up account must not exist yet, \
             or a refused build touching the wallet would be invisible"
        );
        let queued_before = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex")
            .len();

        // The removal-side retirement.
        manager.deactivate().await;

        let built = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityTopUp,
                TOPUP_INDEX,
                &signer,
            )
            .await;
        assert!(
            matches!(built, Err(PlatformWalletError::AssetLockManagerInactive(_))),
            "a build on a retired handle must be refused — the wallet under \
             this id is no longer the one this manager was built for; got {built:?}"
        );

        assert_eq!(
            topup_account_present(&manager, TOPUP_INDEX).await,
            (false, false),
            "the refused build must not have derived a top-up account into \
             the wallet now registered under this id"
        );
        assert_eq!(
            persistence
                .stored
                .lock()
                .expect("capturing persistence mutex")
                .len(),
            queued_before,
            "the refused build must not have queued an address-pool snapshot — \
             a retired manager sharing the persister can overwrite the \
             replacement wallet's durable pool"
        );
    }

    /// Regression (the race the flag alone does not close): a build that
    /// was already queued at the build→persist boundary when the removal
    /// landed must refuse, and the retirement must not cut ahead of a
    /// build already past it.
    ///
    /// An advisory `ensure_active()` before the mutex proves nothing: it
    /// passes, the task then parks on `build_persist_serial`, and the
    /// removal completes during that park. Only a check taken UNDER the
    /// mutex that `deactivate` must itself acquire is authoritative —
    /// which is why `deactivate` now holds `build_persist_serial` (then
    /// `status_persist_serial`, the only place both are held, and always
    /// in that order) while it flips the flag.
    ///
    /// Interleave, with no sleep standing in for a signal:
    ///
    /// 1. the test holds `build_persist_serial`, standing in for a build
    ///    that is past the boundary and mid-flight;
    /// 2. `deactivate` is spawned and rendezvous'd on the production
    ///    `build_serial_waiters` gauge — an arrival observed while the
    ///    test provably holds the mutex is an arrival that cannot have
    ///    flipped the flag;
    /// 3. a build is spawned and rendezvous'd the same way, so it is
    ///    provably queued BEHIND the retirement. `tokio::sync::Mutex` is
    ///    FIFO-fair, so releasing hands the mutex to the retirement
    ///    first — the ordering this test needs is established by the two
    ///    rendezvous, not assumed.
    #[tokio::test]
    async fn a_build_queued_behind_a_retirement_refuses_instead_of_running() {
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysOkBroadcaster)).await;

        const TOPUP_INDEX: u32 = 7;
        let queued_before = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex")
            .len();

        // 1. Stand in for a build holding the boundary.
        let in_flight_build = manager.lock_build_persist_serial().await;

        // 2. The retirement must come to rest at the boundary.
        let manager_deactivate = Arc::clone(&manager);
        let deactivator = tokio::spawn(async move { manager_deactivate.deactivate().await });
        manager.await_build_serial_waiters(1).await;
        assert!(
            manager.active.load(Ordering::SeqCst),
            "`deactivate` must not retire the manager while a build still \
             holds `build_persist_serial` — it would strand a build that has \
             already allocated a funding index with its pool snapshot refused"
        );

        // 3. A build enters after the retirement queued. Its advisory
        //    pre-check passes (the flag is still set, per the assertion
        //    above) and it parks — the exact window the bug lived in.
        let manager_builder = Arc::clone(&manager);
        let builder_signer = signer.clone();
        let builder = tokio::spawn(async move {
            manager_builder
                .build_asset_lock_transaction(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityTopUp,
                    TOPUP_INDEX,
                    &builder_signer,
                )
                .await
        });
        manager.await_build_serial_waiters(2).await;

        // 4. Release; FIFO order gives the mutex to the retirement, then
        //    the build.
        drop(in_flight_build);
        deactivator.await.expect("deactivator task joined");
        let built = builder.await.expect("builder task joined");

        assert!(
            matches!(built, Err(PlatformWalletError::AssetLockManagerInactive(_))),
            "a build that was queued at the boundary when the removal landed \
             must refuse — resuming it would mutate the replacement wallet a \
             same-mnemonic re-import installs under this same id; got {built:?}"
        );
        assert_eq!(
            topup_account_present(&manager, TOPUP_INDEX).await,
            (false, false),
            "the refused build must not have derived a top-up account"
        );
        assert_eq!(
            persistence
                .stored
                .lock()
                .expect("capturing persistence mutex")
                .len(),
            queued_before,
            "the refused build must not have queued anything to the shared \
             persister"
        );
    }
}
