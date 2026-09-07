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
    AssetLockError, AssetLockFundingAccount, AssetLockFundingType, CreditOutputFunding,
};
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionError;
use key_wallet::wallet::managed_wallet_info::managed_account_operations::ManagedAccountOperations;
use key_wallet::wallet::managed_wallet_info::transaction_builder::BuilderError;
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;

use crate::changeset::{AccountRegistrationEntry, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::PlatformWalletInfo;
use crate::ASSET_LOCK_FUNDING_SOURCES;

use super::manager::{AssetLockManager, DEFAULT_FEE_PER_KB};
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
    /// Exact-amount form, **pooled**: it funds from
    /// [`ASSET_LOCK_FUNDING_SOURCES`] (BIP44 + BIP32 + every DashPay
    /// contact-receiving account), so the lock does not need its whole amount
    /// sitting in one account and change returns to BIP44. The
    /// funding-parameterized form is
    /// [`Self::build_asset_lock_transaction_with_funding`].
    ///
    /// # Arguments
    ///
    /// * `amount_duffs` — Amount to lock in duffs.
    /// * `account_index` — Index addressing the standard (BIP44/BIP32)
    ///   families; DashPay contact accounts span their own indices and are
    ///   pooled in regardless.
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
        self.build_asset_lock_transaction_with_funding(
            AssetLockBuildAmount::Exact(amount_duffs),
            &ASSET_LOCK_FUNDING_SOURCES,
            account_index,
            funding_type,
            identity_index,
            signer,
        )
        .await
        // Historical callers never had the reservation token or the funding
        // account list; the funded pipeline
        // (`broadcast_funded_asset_lock_with_funding`) threads both.
        .map(|(tx, path, _token, _accounts)| (tx, path))
    }

    /// Funding-parameterized form of [`Self::build_asset_lock_transaction`]:
    /// `funding_sources` names the account families to POOL, in order — the
    /// first supplies the change address — and `amount` picks exact-amount vs
    /// whole-balance drain semantics (see [`AssetLockBuildAmount`]).
    /// `source_index` addresses the standard families; DashPay set selectors
    /// span their own indices.
    ///
    /// A single-element list reproduces the old one-account behavior, including
    /// its strict account-not-found error; a pooled list skips the sources this
    /// wallet has nothing for. CoinJoin funding is drain-only *and* cannot be
    /// pooled — the key-wallet builder rejects both a non-drain CoinJoin build
    /// and a CoinJoin source combined with any other.
    ///
    /// Returns the transaction, the credit-output derivation path, the build's
    /// reservation token, and the accounts that contributed inputs — the
    /// caller's release path needs every one of them, since a pooled build
    /// reserves in each contributing account's own set under the one token.
    ///
    /// # This form hands the transaction back UNSENT
    ///
    /// It is the build-only entry point: the caller broadcasts through some
    /// other surface, or not at all. So the in-broadcast fence
    /// [`build_asset_lock_transaction_fenced`](Self::build_asset_lock_transaction_fenced)
    /// installs is released again before returning — there is no dispatch here
    /// to keep it alive, and a fence with no settler behind it would hold these
    /// inputs against every later build for the life of the process. The
    /// reservation is left held exactly as before. The internal funded pipeline
    /// ([`Self::broadcast_funded_asset_lock_with_funding`]) takes the fenced
    /// form instead and carries the pin through to its own broadcast.
    #[allow(clippy::type_complexity)]
    pub async fn build_asset_lock_transaction_with_funding<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_sources: &[AccountTypePreference],
        source_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<
        (
            Transaction,
            DerivationPath,
            Option<key_wallet::ReservationToken>,
            Vec<AccountType>,
        ),
        PlatformWalletError,
    > {
        let (transaction, path, token, accounts, pin) = self
            .build_asset_lock_transaction_fenced(
                amount,
                funding_sources,
                source_index,
                funding_type,
                identity_index,
                signer,
            )
            .await?;
        pin.settle_released();
        Ok((transaction, path, token, accounts))
    }

    /// [`build_asset_lock_transaction_with_funding`](Self::build_asset_lock_transaction_with_funding)
    /// that additionally returns the selection's IN-BROADCAST PIN, installed
    /// atomically with the reservation while the wallet-manager write guard was
    /// still held.
    ///
    /// The conflict check this build runs (below) stops it from consuming an
    /// input another dispatch has fenced. On its own that is only half of the
    /// contract: the transaction it just built carries no fence of its own, so
    /// everything the caller does afterwards — the pool durability gate, the
    /// tracking write, and the broadcast await itself — runs unfenced. The
    /// broadcaster can suspend before submission, catch-up can advance
    /// `last_processed_height` past key-wallet's 24-block reservation TTL in
    /// that gap, and a competing build can then sweep and re-reserve this very
    /// input, find no fence, pass its own copy of the check, and complete —
    /// after which this build's already-signed asset lock still goes to the wire
    /// against an input reassigned to another payment.
    ///
    /// The returned pin closes that. The CALLER OWNS ITS SETTLEMENT and must
    /// account for every exit: [`InBroadcastPin::settle_released`] on a
    /// definitive pre-send failure (an abort before the broadcaster is reached,
    /// or a definitive rejection), and
    /// [`InBroadcastPin::settle_pending_spend`] — or simply dropping it — on
    /// every other outcome, which leaves the pending-spend fence standing until
    /// the wallet observes the spend.
    #[allow(clippy::type_complexity)]
    pub(crate) async fn build_asset_lock_transaction_fenced<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_sources: &[AccountTypePreference],
        source_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<
        (
            Transaction,
            DerivationPath,
            Option<key_wallet::ReservationToken>,
            Vec<AccountType>,
            crate::wallet::core::InBroadcastPin,
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
        // caller's funding sources + drain semantics (the key-wallet side
        // pools the sources, enforces that CoinJoin funding is drain-only and
        // unpooled, and reserves the selected inputs in each contributing
        // account's own set under one owner token).
        let result = info
            .core_wallet
            .build_asset_lock_with_signer(
                wallet,
                funding_sources,
                source_index,
                vec![funding],
                DEFAULT_FEE_PER_KB,
                drain,
                signer,
            )
            .await
            .map_err(|e| {
                // A drain's credit-output value is a zero placeholder, so it
                // must not be advertised as the `required` amount of a typed
                // shortfall (an empty CoinJoin account would report
                // `available: 0, required: 0`). The shielded flow already
                // computed the positive floor and threads it through
                // `DrainAll`; use it so the pair describes the real gap.
                let required = match amount {
                    AssetLockBuildAmount::Exact(value) => value,
                    AssetLockBuildAmount::DrainAll { minimum_lock_duffs } => {
                        minimum_lock_duffs.unwrap_or(0)
                    }
                };
                map_builder_error(e, required)
            })?;

        // Refuse a selection that picked an input pinned by an IN-FLIGHT
        // BROADCAST dispatch (`WalletGeneration::pin_in_broadcast`): this
        // build's own selection swept that dispatch's aged reservation
        // (catch-up advanced past key-wallet's TTL while it was suspended
        // pre-submission) and re-reserved the input, so broadcasting this
        // asset lock would race the pinned, already-signed transaction on
        // the wire. Same backstop as `finalize_transaction` and the
        // contact-payment build. The release runs under the write guard
        // held since selection, so it is exact; the token form is
        // owner-guarded like the drain-floor abandon below. The consumed
        // funding key index is the same residue any discarded build leaves,
        // reclaimed by the gap-limit scan.
        if let Some(outpoint) = info.generation.in_broadcast_conflict(&result.transaction) {
            // The pooled build reserves in EVERY contributing account's own
            // set under the one owner token, so the release must sweep
            // `result.funding_accounts` — the same per-account idiom as
            // `release_reservation_after_rejected_broadcast`; accounts that
            // supplied nothing no-op.
            for funding_account in &result.funding_accounts {
                if let Some(account) = info.core_wallet.accounts.funds_account(funding_account) {
                    match result.reservation_token {
                        Some(token) => {
                            account.release_reservation_if_owner(&result.transaction, token)
                        }
                        None => account.release_reservation(&result.transaction),
                    }
                }
            }
            // Typed and shared with the other two choke points rather than an
            // `AssetLockTransaction` string — the condition and the correct
            // caller response are identical on all three
            // (`PlatformWalletError::InputMidBroadcast`).
            return Err(PlatformWalletError::InputMidBroadcast { outpoint });
        }

        // 4. Pull the (pubkey, path) for our single credit output.
        //
        // `build_asset_lock_with_signer` always returns the `Public`
        // variant. The `Private` arm would only come from the soft-
        // wallet `build_asset_lock` path, which platform-wallet does not
        // call — defensively bail if it appears.
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

        // FENCE THIS SELECTION IN TURN, before the write guard drops — the
        // other half of the conflict check above. Installed here rather than
        // beside that check so the two credit-key error paths in between cannot
        // return past a live pin: with the pending-spend phase carrying no
        // deadline, a pin dropped on an abort would fence these inputs against
        // every later build with no transaction to protect and nothing able to
        // clear it.
        //
        // Nothing between the check and this line touches reservations or the
        // fence map, and the wallet-manager WRITE guard has been held across
        // both, so check-and-pin is still one atomic step against the TTL sweep
        // and against `last_processed_height` advancement — the two mutations
        // that could otherwise interleave. See the method docs for the race
        // this closes.
        let in_broadcast_pin = info.generation.pin_in_broadcast(&result.transaction);

        Ok((
            result.transaction,
            path,
            result.reservation_token,
            result.funding_accounts,
            in_broadcast_pin,
        ))
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
    /// * `account_index` — Index addressing the standard (BIP44/BIP32)
    ///   families of [`ASSET_LOCK_FUNDING_SOURCES`]; DashPay contact accounts
    ///   span their own indices and are pooled in regardless.
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
        self.create_funded_asset_lock_proof_pooled(
            AssetLockBuildAmount::Exact(amount_duffs),
            &ASSET_LOCK_FUNDING_SOURCES,
            account_index,
            funding_type,
            identity_index,
            signer,
        )
        .await
    }

    /// Whole-balance drain form of [`Self::create_funded_asset_lock_proof`]:
    /// the caller names the ONE account to drain (`AssetLockFundingAccount`),
    /// which is how the CoinJoin → shielded path funds a lock directly from
    /// mixed coins. A drain has no change output, so the question a pooled
    /// source list answers — which account supplies change — does not arise,
    /// and CoinJoin must not be pooled with transparent sources anyway.
    pub async fn create_funded_asset_lock_proof_with_funding<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_account: AssetLockFundingAccount,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath, OutPoint), PlatformWalletError> {
        self.create_funded_asset_lock_proof_pooled(
            amount,
            &[AccountTypePreference::from(funding_account)],
            funding_account.account_index(),
            funding_type,
            identity_index,
            signer,
        )
        .await
    }

    /// Source-list form of [`Self::create_funded_asset_lock_proof`] — same
    /// build → broadcast → proof pipeline with the pooled funding and amount
    /// semantics of [`Self::build_asset_lock_transaction_with_funding`].
    async fn create_funded_asset_lock_proof_pooled<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_sources: &[AccountTypePreference],
        source_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath, OutPoint), PlatformWalletError> {
        let (path, out_point) = self
            .broadcast_funded_asset_lock_with_funding(
                amount,
                funding_sources,
                source_index,
                funding_type,
                identity_index,
                signer,
            )
            .await?;
        let proof = self
            .wait_for_funded_asset_lock_proof(&out_point, source_index)
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
            &ASSET_LOCK_FUNDING_SOURCES,
            account_index,
            funding_type,
            identity_index,
            signer,
        )
        .await
    }

    /// Funding-parameterized form of [`Self::broadcast_funded_asset_lock`].
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn broadcast_funded_asset_lock_with_funding<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_sources: &[AccountTypePreference],
        source_index: u32,
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

        // 1. Build the asset lock transaction. `funding_accounts` are the
        //    accounts that actually contributed inputs — a pooled build
        //    reserves in each of their own sets under the one token, so every
        //    release below has to reach all of them.
        //    `in_broadcast_pin` fences those inputs from the moment they were
        //    reserved — installed under the build's own write guard, so no
        //    competing build can sweep and re-reserve them across the durability
        //    gate and the broadcast await below. Every
        //    exit from here on settles it: released on the aborts that never
        //    reach the broadcaster and on a definitive rejection, left pending
        //    otherwise.
        let (tx, path, reservation_token, funding_accounts, in_broadcast_pin) = self
            .build_asset_lock_transaction_fenced(
                amount,
                funding_sources,
                source_index,
                funding_type,
                identity_index,
                signer,
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
                // Nothing reached the broadcaster, so the fence has no
                // transaction to protect: release it alongside the reservation
                // — but AFTER the cleanup, never before it. The cleanup awaits
                // the manager read lock, and an input that is unfenced while
                // still reserved-or-reusable is exactly the window the
                // contact-send path closes. This site's release is
                // owner-guarded by `reservation_token`, so a newer build's
                // reservation cannot be clobbered here even so; the ordering is
                // uniform across every settle-with-cleanup site rather than
                // resting on that one argument.
                //
                // The RELEASED verdict is recorded before the cleanup's first
                // await, so cancellation inside it still settles the fence as
                // released on drop: the abort is established and nothing was
                // sent, so a pending-spend settle there would fence inputs no
                // observed spend could ever clear — same shape as the
                // contact-send rejection arm.
                let mut in_broadcast_pin = in_broadcast_pin;
                in_broadcast_pin.settle_released_on_drop();
                crate::wallet::reservations::release_reservation_after_rejected_broadcast(
                    &self.wallet_manager,
                    &self.wallet_id,
                    &funding_accounts,
                    &tx,
                    reservation_token,
                )
                .await;
                in_broadcast_pin.settle_released();
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
                // The pooled build reserved every selected input across its
                // contributing accounts under `reservation_token`. Nothing
                // was broadcast, so abandon like the drain-floor branch
                // above: drop the serialization guard, owner-release across
                // every contributor, THEN surface the durability error —
                // otherwise an immediate retry cannot reselect the BIP44 /
                // BIP32 / DashPay inputs until the TTL sweep frees them. The
                // fence goes with the reservation for the same reason: the
                // broadcaster was never reached, so it protects nothing, and
                // leaving it would block the retry the release exists to enable.
                // It comes down AFTER the cleanup, not before — see the
                // drain-floor branch above for why every settle-with-cleanup
                // site keeps that order —
                // and the released verdict is recorded BEFORE the cleanup's
                // first await, so a cancellation inside it settles released
                // rather than opening an uncleanable pending-spend fence over
                // inputs that provably never went to the wire (same shape as
                // the drain-floor branch).
                drop(build_persist_guard);
                let mut in_broadcast_pin = in_broadcast_pin;
                in_broadcast_pin.settle_released_on_drop();
                crate::wallet::reservations::release_reservation_after_rejected_broadcast(
                    &self.wallet_manager,
                    &self.wallet_id,
                    &funding_accounts,
                    &tx,
                    reservation_token,
                )
                .await;
                in_broadcast_pin.settle_released();
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
                account_index: source_index,
                funding_type,
                identity_index,
                amount: locked_amount_duffs,
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
        //    `Built` row BEFORE releasing the funding reservation (held in
        //    every account of `funding_accounts`, under the one owner token):
        //    while the reservation is held the inputs
        //    cannot be re-selected by a new build, and once the row is gone
        //    `resume_asset_lock` can no longer re-drive the rejected
        //    transaction — so at no point is the row resumable while its
        //    inputs are re-spendable. A `MaybeSent` failure keeps both the
        //    reservation and the resumable row.
        //
        //    The reported error type and the in-broadcast fence both follow the
        //    cleanup, never the broadcaster's verdict alone — they are decided
        //    by the one predicate. The definite-rejection contract is reported
        //    only when the row was actually untracked AND its reservation
        //    released, because that contract is precisely the promise that both
        //    happened; the fence — held ACROSS this await, which is what it is
        //    for — is freed on exactly that same condition and left as a
        //    pending-spend fence everywhere else, until the wallet observes the
        //    spend. A cancellation or unwind inside `broadcast` reaches no arm
        //    at all and settles as pending through `InBroadcastPin::drop`.
        let broadcast_outcome = self.broadcaster.broadcast(&tx).await;
        if let Err(e) = broadcast_outcome {
            if matches!(e, crate::broadcaster::BroadcastError::Rejected { .. }) {
                // The rejection alone does NOT establish the released verdict
                // on this path — that is what the untrack guard below decides
                // — so nothing can be recorded on the pin before this await.
                // A cancellation inside it settles the pin as pending, which
                // is the correct least-informed state here: the `Built` row is
                // then still tracked (`untrack_asset_lock`'s only await is its
                // lock acquisition, before the removal — a cancelled call
                // cannot have half-removed the row), so `resume_asset_lock`
                // can still re-drive the transaction and the fence's observed
                // spend can still arrive.
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
                    // Provably nothing on the wire and the row is gone: free the
                    // fence with the reservation so the rebuild can reselect —
                    // the fence coming down LAST, after the cleanup await, so
                    // the input is never unfenced while still reusable (see
                    // the drain-floor branch for the full window).
                    //
                    // The released verdict IS established now — rejected AND
                    // unresumable — so it is recorded before the cleanup's
                    // first await: a cancellation inside the cleanup must
                    // settle released, not fence inputs whose transaction was
                    // never sent and can no longer be resumed (same shape as
                    // the contact-send rejection arm).
                    let mut in_broadcast_pin = in_broadcast_pin;
                    in_broadcast_pin.settle_released_on_drop();
                    crate::wallet::reservations::release_reservation_after_rejected_broadcast(
                        &self.wallet_manager,
                        &self.wallet_id,
                        &funding_accounts,
                        &tx,
                        reservation_token,
                    )
                    .await;
                    in_broadcast_pin.settle_released();
                } else {
                    // The untrack guard fired: a concurrent `resume_asset_lock`
                    // advanced the row past `Built`, which is positive evidence
                    // the transaction reached the network after all. The
                    // reservation stays held, and so must the fence.
                    in_broadcast_pin.settle_pending_spend();
                    // The cleanup did not run, so the definite-rejection
                    // contract does not hold either. `TransactionBroadcast`
                    // promises the caller that the row is gone, the inputs are
                    // free, and a rebuild is safe; here the advanced row is
                    // still tracked and resumable and its inputs are still
                    // reserved and fenced, so a caller honouring that promise
                    // would rebuild from other UTXOs and create a SECOND asset
                    // lock beside a transaction the advance says reached the
                    // network. The contract that matches what is actually true
                    // is the unknown outcome: do not retry, the row and its
                    // reservation are intact, resume the existing lock.
                    tracing::warn!(
                        %txid,
                        error = %e,
                        "asset lock broadcast was rejected, but a concurrent resume had \
                         already advanced the row past Built; keeping the row and its \
                         funding reservation and reporting an unknown outcome rather than \
                         a definite rejection"
                    );
                    return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                        format!(
                            "asset lock {out_point} stays tracked and reserved: the \
                             broadcast was rejected, but a concurrent resume had already \
                             advanced the row past Built, so the transaction may be on \
                             the network: {e}"
                        ),
                    ));
                }
            } else {
                // Ambiguous `MaybeSent`: the transaction may be on the network.
                in_broadcast_pin.settle_pending_spend();
            }
            return Err(e.into());
        }
        // Accepted. On the DAPI broadcaster nothing was injected locally, so the
        // inputs are still selectable here until the spend is observed.
        in_broadcast_pin.settle_pending_spend();

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

/// Map a key-wallet [`AssetLockError`] to a [`PlatformWalletError`], promoting
/// every coin-selection shortfall shape to the typed
/// [`PlatformWalletError::AssetLockInsufficientFunds`] so callers get one
/// structured shortfall contract instead of a string
/// they must pattern-match:
///
///   - `BuilderError::InsufficientFunds` / `SelectionError::InsufficientFunds`
///     carry their own exact `available`/`required` duff amounts — preserved
///     verbatim.
///   - `SelectionError::NoUtxosAvailable` — the zero-spendable-candidate case,
///     the MOST extreme shortfall — carries no amounts, so it would otherwise
///     fall through to the generic string form while *partial* shortfalls
///     stayed typed. It maps to `available: 0` against the caller's
///     `requested` target, keeping the empty candidate set on the same
///     structured path.
///
/// `requested` is the caller's target in duffs. On a drain build the target is
/// the zero credit-output placeholder (key-wallet rewrites the value to
/// `Σ inputs − fee`), so the mapper substitutes the drain floor —
/// `minimum_lock_duffs.unwrap_or(0)` — as `required`: an empty account reports
/// `available: 0` against the configured floor (positive for the shielded
/// flow, which installs the Type 18 pool-fee floor before building), and 0
/// only when no floor was supplied. The floor is additionally enforced
/// downstream by `broadcast_funded_asset_lock_with_funding` against the built
/// payload.
///
/// Every other builder error keeps the pre-existing generic
/// `AssetLockTransaction` string form.
fn map_builder_error(e: AssetLockError, requested: u64) -> PlatformWalletError {
    match e {
        AssetLockError::Builder(
            BuilderError::InsufficientFunds {
                available,
                required,
            }
            | BuilderError::CoinSelection(SelectionError::InsufficientFunds {
                available,
                required,
            }),
        ) => PlatformWalletError::AssetLockInsufficientFunds {
            available,
            required,
        },
        AssetLockError::Builder(BuilderError::CoinSelection(SelectionError::NoUtxosAvailable)) => {
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
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
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
        funded_wallet_manager, funded_wallet_manager_dual_standard,
        funded_wallet_manager_with_contact, AlwaysMaybeSentBroadcaster, AlwaysOkBroadcaster,
        AlwaysRejectedBroadcaster, WalletSigner,
    };
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::asset_lock::tracked::AssetLockStatus;
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use crate::wallet::platform_wallet::WalletId;
    use crate::{AssetLockFundingType, PlatformWalletError};

    /// The zero-spendable-candidate selection error must surface the SAME
    /// typed shortfall as a partial shortfall (not the generic string form),
    /// so hosts stay on one structured path; and a partial shortfall must
    /// still carry its own exact amounts.
    #[test]
    fn coin_selection_shortfalls_map_to_typed_insufficient_funds() {
        use super::{map_builder_error, AssetLockError, BuilderError, SelectionError};

        // Zero spendable candidates -> typed, available: 0, required = requested.
        match map_builder_error(
            AssetLockError::Builder(BuilderError::CoinSelection(
                SelectionError::NoUtxosAvailable,
            )),
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
            AssetLockError::Builder(BuilderError::CoinSelection(
                SelectionError::InsufficientFunds {
                    available: 100,
                    required: 500,
                },
            )),
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

        // A non-shortfall builder error keeps the pre-existing generic string
        // form — the typed promotion must not swallow unrelated failures.
        match map_builder_error(AssetLockError::WatchOnlyWallet, 42) {
            PlatformWalletError::AssetLockTransaction(msg) => {
                assert!(
                    msg.starts_with("Asset lock builder failed: "),
                    "generic form preserved, got {msg}"
                );
            }
            other => panic!("expected generic AssetLockTransaction, got {other:?}"),
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

    /// Broadcaster that PARKS inside `broadcast` — the production suspension
    /// the in-broadcast fence exists to cover. Signals `entered` once it has
    /// the transaction (manager guard already dropped, nothing submitted) and
    /// waits on `release` before returning.
    struct GatedBroadcaster {
        entered: Arc<tokio::sync::Barrier>,
        release: Arc<tokio::sync::Barrier>,
    }

    #[async_trait]
    impl TransactionBroadcaster for GatedBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.entered.wait().await;
            self.release.wait().await;
            Ok(transaction.txid())
        }
    }

    /// Run ordinary historical catch-up on the fixture wallet: advance both
    /// height clocks well past key-wallet's 24-block reservation TTL, so a
    /// reservation stamped before the call is swept by the next selection.
    async fn catch_up_past_the_reservation_ttl(
        wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        height: u32,
    ) {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        let mut wm = wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
        info.core_wallet.update_last_processed_height(height);
        info.core_wallet.update_synced_height(height);
    }

    /// The fixture's single spendable BIP-44 outpoint.
    async fn the_only_funded_outpoint(
        wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
    ) -> OutPoint {
        let wm = wallet_manager.read().await;
        let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
        let utxos = &info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .expect("BIP-44 managed account 0")
            .utxos;
        assert_eq!(
            utxos.len(),
            1,
            "the race needs exactly one selectable UTXO, so both builds must \
             contend for the same input"
        );
        *utxos.keys().next().expect("one utxo")
    }

    /// THE ASSET-LOCK BUILD'S OWN FENCE.
    ///
    /// The build's conflict check stops it from CONSUMING an input another
    /// dispatch has fenced. Without a fence on the selection it has just made,
    /// everything between the check and the direct `broadcaster.broadcast(&tx)`
    /// — the pool durability gate, the `Built` tracking write, and the await
    /// itself — would run with no pin on those inputs.
    ///
    /// 1. A funded asset lock builds, signs, releases the manager guard, and
    ///    SUSPENDS inside the broadcaster before submission.
    /// 2. Catch-up advances the wallet far past key-wallet's 24-block
    ///    reservation TTL, so the parked build's reservation is swept.
    /// 3. A competing asset-lock build runs. There is exactly one spendable
    ///    UTXO, so it selects the same input the parked lock already spends.
    ///
    /// Step 3 must be refused with `InputMidBroadcast` rather than returning a
    /// second signed asset lock against that input.
    ///
    /// The two builds run through two `AssetLockManager`s over ONE shared
    /// wallet manager. That is not a workaround for the per-manager
    /// build→persist serialization guard: production drops that guard before
    /// the broadcast (it orders pool snapshots, nothing else), so a single
    /// manager leaves exactly the same window open. Two managers just make the
    /// second build's broadcaster independent of the parked one. The fence
    /// lives on the shared wallet generation, which is what both see.
    #[tokio::test]
    async fn a_suspended_asset_lock_fences_its_inputs_against_a_competing_build() {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let funded = the_only_funded_outpoint(&wallet_manager, wallet_id).await;

        let entered = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Barrier::new(2));
        let (parked_manager, _p1) = asset_lock_manager_over(
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(GatedBroadcaster {
                entered: Arc::clone(&entered),
                release: Arc::clone(&release),
            }),
        );
        let (competing_manager, _p2) = asset_lock_manager_over(
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );

        let parked = async {
            parked_manager
                .broadcast_funded_asset_lock(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    0,
                    &signer,
                )
                .await
        };

        let competitor = async {
            // Parked inside `broadcast`: signed, guard dropped, nothing
            // submitted — the window the fence has to cover.
            entered.wait().await;
            catch_up_past_the_reservation_ttl(&wallet_manager, wallet_id, 17_000).await;

            let racing = competing_manager
                .broadcast_funded_asset_lock(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    0,
                    &signer,
                )
                .await;
            release.wait().await;
            racing
        };

        let (sent, racing) = tokio::join!(parked, competitor);

        match racing {
            Err(PlatformWalletError::InputMidBroadcast { outpoint }) => assert_eq!(
                outpoint, funded,
                "the refusal must name the input the parked lock spends"
            ),
            other => panic!(
                "a competing asset-lock build must be refused while the original is \
                 mid-broadcast — unfenced, it returned a second signed lock spending \
                 the same input, got {other:?}"
            ),
        }

        assert!(
            sent.is_ok(),
            "the parked asset lock itself must complete normally, got {sent:?}"
        );
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
                &[AccountTypePreference::CoinJoin],
                0,
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
                &[AccountTypePreference::CoinJoin],
                0,
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

    /// A whole-account drain that finds nothing selectable must report the
    /// `DrainAll` minimum-lock floor as the shortfall's `required`, judged at
    /// BUILD level rather than by calling `map_builder_error` directly.
    ///
    /// This is the branch guard for the `AssetLockBuildAmount::DrainAll`
    /// arm of that `required` computation. A drain's credit output carries a
    /// ZERO placeholder value (the key-wallet builder rewrites it to
    /// `Σ inputs − fee`), so reverting the arm to the built `amount_duffs`
    /// would advertise the meaningless pair `available: 0, required: 0` — and
    /// the direct-call unit test above, which passes its own `requested`
    /// argument in, would stay green through that revert. This one would not.
    ///
    /// The zero-spendable-candidate state is reached by holding the first
    /// build's reservation token for the whole test, which keeps the fixture's
    /// single CoinJoin UTXO reserved and leaves the account fully committed.
    #[tokio::test]
    async fn drain_shortfall_reports_the_minimum_lock_floor_as_required() {
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer, _persistence) =
            coinjoin_funded_asset_lock_manager(Arc::clone(&broadcaster)).await;

        // Reserve the account's only UTXO. `_token` is a live binding, so the
        // reservation cannot be released before the second build runs; `None`
        // skips the floor check, which a build never applies anyway (it is
        // judged downstream against the BUILT payload).
        let (_tx, _path, _token, _accounts) = manager
            .build_asset_lock_transaction_with_funding(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: None,
                },
                &[AccountTypePreference::CoinJoin],
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await
            .expect("first drain builds over the funded CoinJoin account");

        // Second drain: zero spendable candidates over a CoinJoin account,
        // which is exactly the whole-account form the shielded flow uses.
        let shortfall = manager
            .build_asset_lock_transaction_with_funding(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(12_345),
                },
                &[AccountTypePreference::CoinJoin],
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await;

        match shortfall {
            Err(PlatformWalletError::AssetLockInsufficientFunds {
                available,
                required,
            }) => {
                assert_eq!(
                    available, 0,
                    "the fully-reserved CoinJoin account has nothing selectable"
                );
                assert_eq!(
                    required, 12_345,
                    "a drain must report the floor threaded through DrainAll, \
                     not the zero credit-output placeholder"
                );
            }
            other => panic!("expected typed AssetLockInsufficientFunds, got {other:?}"),
        }

        assert_eq!(
            broadcaster.calls(),
            0,
            "a build-level shortfall must never reach the broadcaster"
        );
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
        // The reserved UTXO leaves zero spendable candidates, so this is the
        // typed selection shortfall — a stronger assertion than the old generic
        // build-error match, which any unrelated failure would also satisfy.
        assert!(
            matches!(
                rebuild,
                Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. })
            ),
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
    ///
    /// The error must say the same thing the cleanup did. The definite
    /// rejection promises a released reservation and a safe rebuild, and
    /// neither holds on this branch: a caller acting on that promise builds
    /// a second asset lock beside a transaction the advance says reached the
    /// network. Only the unknown outcome describes what actually happened.
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
            matches!(
                result,
                Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
            ),
            "a rejection whose cleanup released nothing must surface as the \
             unknown outcome, never as the definite rejection that promises a \
             released reservation and a safe rebuild, got {result:?}"
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
        // As above: zero spendable candidates is the typed selection shortfall.
        assert!(
            matches!(
                rebuild,
                Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. })
            ),
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

        // The abort released the pooled reservations across every
        // contributing account: an IMMEDIATE rebuild must get through coin
        // selection on the same fixture UTXOs and reach the durability gate
        // again (the same "aborted before broadcast" error). Stranded
        // reservations would surface here as a selection failure instead,
        // stuck until the TTL sweep.
        let rebuild = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await;
        match rebuild {
            Err(PlatformWalletError::AssetLockTransaction(msg)) => assert!(
                msg.contains("aborted before broadcast"),
                "the rebuild must reselect the released inputs and reach the \
                 durability gate again — a selection failure means the abort \
                 stranded the pooled reservations; got: {msg}"
            ),
            other => panic!(
                "the rebuild must reach the durability gate again (inputs \
                 released), got {other:?}"
            ),
        }
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

    // -- Pooled asset-lock funding ---------------------------------------

    /// Build an `AssetLockManager` over an already-built wallet manager.
    fn asset_lock_manager_over<B: TransactionBroadcaster>(
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        broadcaster: Arc<B>,
    ) -> (Arc<AssetLockManager<B>>, Arc<CapturingPersistence>) {
        let persistence = Arc::new(CapturingPersistence::default());
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
        (manager, persistence)
    }

    /// THE POINT OF THIS CHANGE: an asset lock larger than either standard
    /// family holds is funded from BOTH in one transaction. Before pooling
    /// this was `CoreInsufficientFunds` unless the caller first swept the
    /// accounts together and locked out of the sweep — an extra on-chain hop
    /// and fee.
    #[tokio::test]
    async fn pooled_asset_lock_spans_the_standard_families() {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager_dual_standard(&[700_000], &[700_000]).await;
        let (manager, _persistence) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );

        // 1_000_000 exceeds either family's 700_000, so selection must pool.
        let (_path, out_point) = manager
            .broadcast_funded_asset_lock(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("a lock above either family's balance must pool both");

        let wm = manager.wallet_manager.read().await;
        let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
        let tracked = info
            .tracked_asset_locks
            .get(&out_point)
            .expect("the broadcast lock is tracked");
        assert!(
            tracked.transaction.input.len() >= 2,
            "a lock above either family's balance needs inputs from both, got {}",
            tracked.transaction.input.len()
        );
    }

    /// The DashPay half of the pooled set, end to end: a lock larger than
    /// BIP44 alone holds reaches into a real contact-receiving account and
    /// signs its inputs (DIP-15 `Normal256` path). Without this, every lookup
    /// in the pooled path could resolve `None` for contact accounts and the
    /// feature would silently degrade to BIP44 + BIP32.
    #[tokio::test]
    async fn pooled_asset_lock_spends_dashpay_contact_funds() {
        let (wallet_manager, wallet_id, _generation, signer, _contact_account) =
            funded_wallet_manager_with_contact(&[700_000], &[700_000]).await;
        let (manager, _persistence) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );

        let (_path, out_point) = manager
            .broadcast_funded_asset_lock(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("a lock above BIP44's balance must reach the contact account");

        let wm = manager.wallet_manager.read().await;
        let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
        let tracked = info
            .tracked_asset_locks
            .get(&out_point)
            .expect("the broadcast lock is tracked");
        assert!(
            tracked.transaction.input.len() >= 2,
            "the contact's coin must be spent alongside BIP44's"
        );
    }

    /// The reservation hazard pooling introduces, and the one this change had
    /// to get right: a rejected broadcast must release the reservation in
    /// EVERY contributing account. The pooled build reserves per account under
    /// one owner token, so releasing only the first would leave the rest of
    /// the inputs held until the 24-block TTL backstop — and an immediate
    /// retry would fail with spurious insufficient funds. The rebuild below
    /// can only succeed if both families' inputs came back.
    #[tokio::test]
    async fn rejected_pooled_broadcast_releases_every_contributing_account() {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager_dual_standard(&[700_000], &[700_000]).await;
        let (manager, _persistence) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(AlwaysRejectedBroadcaster),
        );

        let rejected = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(rejected, Err(PlatformWalletError::TransactionBroadcast(_))),
            "the pooled build must have succeeded and only the broadcast failed, got {rejected:?}"
        );
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
            assert!(
                info.tracked_asset_locks.is_empty(),
                "a definitively rejected lock leaves no resumable row"
            );
        }

        // Identical rebuild: only possible if BOTH accounts' inputs were
        // released. A release that reached only the first funding account
        // would strand the other family's coin, leaving 700_000 available
        // against a 1_000_000 lock — insufficient funds, not a rebuild.
        let (rebuilt, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("every contributing account's reservation must have been released");
        assert!(
            rebuilt.input.len() >= 2,
            "the rebuild must reselect inputs from both families, got {}",
            rebuilt.input.len()
        );
    }
}
