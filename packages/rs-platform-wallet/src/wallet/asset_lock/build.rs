//! Asset lock transaction building.
//!
//! Contains methods for building asset lock transactions, peeking at funding
//! addresses, and the unified `create_funded_asset_lock_proof` entry point.

use crate::broadcaster::TransactionBroadcaster;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use crate::changeset::{AssetLockRecipient, ShieldedRecipientRaw};
use dashcore::Address as DashAddress;
use dashcore::{OutPoint, Transaction, TxOut};
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use key_wallet::bip32::DerivationPath;
use key_wallet::managed_account::address_pool::AddressPoolType;
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

use crate::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, AssetLockChangeSet, PersistenceCapabilities,
    PersistenceError, PlatformWalletChangeSet,
};
use crate::error::PlatformWalletError;
use crate::wallet::core::InBroadcastPin;
use crate::wallet::persister::NotDurable;
use crate::wallet::platform_wallet::PlatformWalletInfo;
use crate::ASSET_LOCK_FUNDING_SOURCES;

use super::funding_account::funding_account_type;
use super::manager::{AssetLockManager, DEFAULT_FEE_PER_KB};
use super::top_up_floor::min_top_up_duffs;
use super::tracked::{AssetLockStatus, TrackedAssetLock};
use super::{input_cap_violation, AssetLockOperationContext, AssetLockOperationState};

/// A funding address handed back in memory, located for clearing its stored
/// used mark.
struct ReleasedFundingAddress {
    account_type: AccountType,
    pool_type: AddressPoolType,
    index: u32,
    address: DashAddress,
}

/// Callback that receives the outpoint of an asset lock built for a
/// [`AssetLockFunding::FromWalletBalance`](super::AssetLockFunding::FromWalletBalance),
/// [`AssetLockFunding::DrainAccountBalance`](super::AssetLockFunding::DrainAccountBalance) or
/// [`AssetLockFunding::DrainWalletBalance`](super::AssetLockFunding::DrainWalletBalance) request. It runs synchronously
/// once the transaction is signed, before the lock's row is persisted or the
/// lock is broadcast. A caller that watches persisted lock rows uses it to
/// tell its own new lock apart from rows the wallet re-inserts during
/// recovery; it must record the outpoint before returning, durably if that
/// distinction has to survive a restart. The ordering covers persisted rows
/// only, and only for calls that pass an observer: a fresh lock from a call
/// that passes none (identity registration, the managed identity top-up, a
/// generic funded proof, a wallet drain over FFI) also appears as a row
/// nobody reported.
///
/// It runs while the wallet serializes asset-lock builds, so it must not
/// block and must not call back into the wallet: another build would wait
/// for it. It runs at most once per call and must not panic.
///
/// - Without an operation context (`FromWalletBalance`,
///   `DrainAccountBalance`) the lock is tracked in memory first, so
///   in-memory listings can show it just before the callback runs, and its
///   row is queued best-effort afterwards. The callback does not run when
///   the wallet is gone at tracking. A lock whose broadcast then fails is
///   removed again after a definite rejection, durably: if the removal
///   cannot be made durable the row may come back after a restart, so the
///   inputs stay reserved and the call fails with
///   [`PlatformWalletError::TransactionBroadcastUnconfirmed`] rather than
///   as a definite rejection. Otherwise the lock stays tracked and can be
///   resumed by its outpoint (a resume may then find it already consumed).
///   A failed best-effort row store can leave no row for a broadcast lock
///   after a restart.
/// - For a wallet drain (`DrainWalletBalance`) the callback runs before the
///   row is stored durably and before the lock is tracked. If that store
///   fails, the lock is not tracked and is recovered by resuming the drain
///   with the same operation UUID, not by its outpoint.
/// - For a lock whose row records its recipient durably (an unbound identity
///   top-up, or a shielded funding on a backend that records shielded
///   recipients) the callback likewise runs before the row is stored and
///   flushed and before the lock is tracked. If that store fails, the row's
///   removal is stored behind it: once that removal is durable the inputs are
///   released and the call fails with a persistence error, and nothing was
///   sent; otherwise the row may come back after a restart, the inputs stay
///   reserved, and the call fails with
///   [`PlatformWalletError::TransactionBroadcastUnconfirmed`].
///
/// It does not run for [`AssetLockFunding::FromExistingAssetLock`](super::AssetLockFunding::FromExistingAssetLock) (a resumed
/// drain included), whose outpoint the caller already holds, nor when the
/// build itself fails or is abandoned before the outpoint is handed out.
pub type AssetLockBuiltObserver<'a> = dyn Fn(OutPoint) + Send + Sync + 'a;

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

/// Hold a top-up lock (bound or unbound) to Platform's top-up minimum
/// ([`min_top_up_duffs`]) wherever it is built, before anything is selected,
/// reserved or broadcast: Core accepts a smaller lock, but Platform refuses to
/// top up from it, stranding its funds. An exact amount below the minimum is
/// refused; a drain gets it as its floor, which the build enforces on the
/// built lock's value. Other funding types pass through unchanged.
fn hold_top_up_to_minimum(
    funding_type: AssetLockFundingType,
    amount: AssetLockBuildAmount,
) -> Result<AssetLockBuildAmount, PlatformWalletError> {
    if !matches!(
        funding_type,
        AssetLockFundingType::IdentityTopUp | AssetLockFundingType::IdentityTopUpNotBound
    ) {
        return Ok(amount);
    }
    let min_duffs = min_top_up_duffs()?;
    match amount {
        AssetLockBuildAmount::Exact(duffs) if duffs < min_duffs => {
            Err(PlatformWalletError::InvalidParameter(format!(
                "a top-up of {duffs} duffs is below the minimum top-up asset-lock balance of \
                 {min_duffs} duffs"
            )))
        }
        AssetLockBuildAmount::DrainAll { minimum_lock_duffs } => {
            Ok(AssetLockBuildAmount::DrainAll {
                minimum_lock_duffs: Some(minimum_lock_duffs.unwrap_or(0).max(min_duffs)),
            })
        }
        amount => Ok(amount),
    }
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
        // An unbound top-up lock pays only the identity its tracked row
        // records, and this form tracks nothing: the lock would surface
        // later, rebuilt from the chain, with no recipient. The funded
        // pipeline builds those, recording the recipient first.
        if funding_type == AssetLockFundingType::IdentityTopUpNotBound {
            return Err(PlatformWalletError::InvalidParameter(
                "an unbound identity top-up lock must record its recipient; build it through \
                 the funded top-up, not the build-only API"
                    .to_string(),
            ));
        }
        // A drained top-up is held to Platform's minimum on the BUILT lock's
        // value, which only the funded pipeline checks before handing the
        // transaction on; this form returns it for the caller to broadcast.
        if matches!(funding_type, AssetLockFundingType::IdentityTopUp)
            && matches!(amount, AssetLockBuildAmount::DrainAll { .. })
        {
            return Err(PlatformWalletError::InvalidParameter(
                "a drained identity top-up lock must be held to the top-up minimum on its \
                 built value; build it through the funded top-up, not the build-only API"
                    .to_string(),
            ));
        }
        let (transaction, path, token, accounts, pin, _funding_pools) = self
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
            InBroadcastPin,
            Vec<AccountAddressPoolEntry>,
        ),
        PlatformWalletError,
    > {
        // Every top-up lock, whichever entry builds it: the funded pipeline,
        // the funded-proof export and the build-only (signed-transaction)
        // export all come through here.
        let amount = hold_top_up_to_minimum(funding_type, amount)?;
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

        // Only on the generation this manager was created for. A caller can
        // await (an identity lookup, a quote) between taking its handle and
        // building, and the wallet can be removed and re-created under the
        // same id meanwhile: the info found here would be the replacement's,
        // and the pin below would be created on its generation, so the
        // pre-broadcast generation check would pass and the stale handle would
        // spend the replacement's funds. Checked before anything is derived,
        // reserved or signed, under the write guard the pin is created under,
        // so the generation cannot change in between.
        if !Arc::ptr_eq(&info.generation, &self.generation) {
            return Err(PlatformWalletError::WalletRemovedBeforeSend(hex::encode(
                self.wallet_id,
            )));
        }

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
            funding_type,
            identity_index,
            &info.funding_issuance_floors,
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
        // owner-guarded like the drain-floor abandon below. The funding key
        // index the build consumed is handed back, as for every discarded
        // build.
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
            if let AssetLockCreditKeys::Public(keys) = &result.keys {
                if let Some((_, path)) = keys.first() {
                    Self::release_discarded_funding_index(
                        &mut info.core_wallet,
                        funding_type,
                        identity_index,
                        path,
                    );
                }
            }
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
        // Capture used funding indices under the build's existing manager guard.
        // The funded path must not reacquire it after reserving inputs: a
        // cancellation there has no durable row from which to recover.
        let funding_pools = Self::asset_lock_account_pool_entries(info);
        let in_broadcast_pin = info.generation.pin_in_broadcast(&result.transaction);

        Ok((
            result.transaction,
            path,
            result.reservation_token,
            result.funding_accounts,
            in_broadcast_pin,
            funding_pools,
        ))
    }

    /// Peek at the next unused address from a funding account without
    /// consuming it (i.e. without marking it as used).
    ///
    /// The key-wallet builder's `next_private_key` will later find the same
    /// address, derive the private key, and mark it as used.
    ///
    /// Refuses an address at or below the account's entry in
    /// `issuance_floors`: its key may already be handed out.
    fn peek_next_funding_address(
        wallet_info: &mut ManagedWalletInfo,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        issuance_floors: &BTreeMap<AccountType, u32>,
    ) -> Result<DashAddress, PlatformWalletError> {
        let managed_account = match funding_type {
            AssetLockFundingType::IdentityRegistration => wallet_info
                .accounts
                .identity_registration
                .as_mut()
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(
                        "Identity registration account not found".to_string(),
                    )
                })?,
            AssetLockFundingType::IdentityTopUp => wallet_info
                .accounts
                .identity_topup
                .get_mut(&identity_index)
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(format!(
                        "Identity top-up account for index {} not found",
                        identity_index
                    ))
                })?,
            AssetLockFundingType::IdentityTopUpNotBound => wallet_info
                .accounts
                .identity_topup_not_bound
                .as_mut()
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(
                        "Identity top-up (unbound) account not found".to_string(),
                    )
                })?,
            AssetLockFundingType::IdentityInvitation => wallet_info
                .accounts
                .identity_invitation
                .as_mut()
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(
                        "Identity invitation account not found".to_string(),
                    )
                })?,
            AssetLockFundingType::AssetLockAddressTopUp => wallet_info
                .accounts
                .asset_lock_address_topup
                .as_mut()
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(
                        "Asset lock address top-up account not found".to_string(),
                    )
                })?,
            AssetLockFundingType::AssetLockShieldedAddressTopUp => wallet_info
                .accounts
                .asset_lock_shielded_address_topup
                .as_mut()
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(
                        "Asset lock shielded address top-up account not found".to_string(),
                    )
                })?,
        };

        // The first generated, unused address of the pool. It is NOT marked
        // used yet: the builder does that once the signer returns the
        // credit-output key. The builder resolves that key from the pool alone,
        // with no key source, so the pool must already hold an unused address;
        // no key source is passed here either, so nothing is derived. The window
        // moves when a lock is seen on chain; a discarded build hands its
        // index back (`release_discarded_funding_index`).
        let address = managed_account.next_address(None).map_err(|e| {
            PlatformWalletError::AssetLockTransaction(format!(
                "Failed to get next funding address: {}",
                e
            ))
        })?;
        let account_type = funding_account_type(funding_type, identity_index);
        if let Some(floor) = issuance_floors.get(&account_type) {
            let index = managed_account
                .managed_account_type()
                .address_pools()
                .iter()
                .find_map(|pool| pool.address_index(&address));
            if index.is_none_or(|index| index <= *floor) {
                return Err(PlatformWalletError::AssetLockTransaction(format!(
                    "The {account_type:?} funding keys up to index {floor} may already be \
                     handed out, but the wallet's stored addresses could not be restored that \
                     far; this account cannot fund an asset lock"
                )));
            }
        }
        Ok(address)
    }

    /// Hand back the funding-key index a discarded build consumed.
    ///
    /// The builder marks the credit-output index used once the signer returns
    /// its key, and a lock refused after that (input cap, drain floor,
    /// in-broadcast conflict) never reaches the wire. Left used, each refusal
    /// costs one of the pool's few generated addresses: key-wallet derives
    /// further funding addresses only for a lock it sees on chain, so after
    /// five refusals every build of that funding type fails with "No unused
    /// address available". Extending the window instead would leave a gap
    /// seed-only discovery cannot cross: it matches credit outputs only against
    /// the first `DEFAULT_SPECIAL_GAP_LIMIT` addresses until one is seen on
    /// chain, so the next lock would not be recovered from the seed. Nothing
    /// was broadcast or exported with this key, so the next build reuses it.
    ///
    /// Returns the address it released, for a caller that also clears the
    /// stored mark ([`Self::hand_back_unsent_funding_index`]).
    fn release_discarded_funding_index(
        wallet_info: &mut ManagedWalletInfo,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        path: &DerivationPath,
    ) -> Option<ReleasedFundingAddress> {
        let account_type = funding_account_type(funding_type, identity_index);
        for mut account in wallet_info.accounts.all_accounts_mut() {
            if account.managed_account_type().to_account_type() != account_type {
                continue;
            }
            for pool in account.managed_account_type_mut().address_pools_mut() {
                let Some((address, index)) = pool
                    .addresses()
                    .values()
                    .find(|info| info.path == *path && info.is_used())
                    .map(|info| (info.address.clone(), info.index))
                else {
                    continue;
                };
                pool.unmark_used(&address);
                return Some(ReleasedFundingAddress {
                    account_type,
                    pool_type: pool.pool_type(),
                    index,
                    address,
                });
            }
        }
        None
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

    /// Capture the asset-lock funding accounts' address-pool snapshots so a
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
    /// Capture under the build's existing guard. The funded caller holds
    /// `build_persist_serial` until this snapshot is durably recorded, so
    /// another funded build cannot persist an older used-index snapshot later.
    fn asset_lock_account_pool_entries(
        wallet_info: &PlatformWalletInfo,
    ) -> Vec<AccountAddressPoolEntry> {
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
                crate::changeset::account_address_pool_entries(
                    managed.managed_account_type().to_account_type(),
                    managed.managed_account_type().address_pools(),
                )
            })
            .collect()
    }

    fn persist_asset_lock_account_pools(
        &self,
        entries: Vec<AccountAddressPoolEntry>,
    ) -> Result<(), PersistenceError> {
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
        self.create_funded_asset_lock_proof_observed(
            amount_duffs,
            account_index,
            funding_type,
            identity_index,
            None,
            signer,
            None,
            |_| {},
        )
        .await
    }

    /// [`Self::create_funded_asset_lock_proof`] that reports the built lock's
    /// outpoint to `on_built` (see [`AssetLockBuiltObserver`]).
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn create_funded_asset_lock_proof_observed<S: ExtendedPubKeySigner>(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        recipient: Option<AssetLockRecipient>,
        signer: &S,
        on_built: Option<&AssetLockBuiltObserver<'_>>,
        on_dispatch: impl FnOnce(OutPoint) + Send,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath, OutPoint), PlatformWalletError> {
        self.create_funded_asset_lock_proof_pooled(
            AssetLockBuildAmount::Exact(amount_duffs),
            &ASSET_LOCK_FUNDING_SOURCES,
            account_index,
            funding_type,
            identity_index,
            recipient,
            signer,
            on_built,
            on_dispatch,
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
        self.create_funded_asset_lock_proof_with_funding_observed(
            amount,
            funding_account,
            funding_type,
            identity_index,
            None,
            signer,
            None,
            |_| {},
        )
        .await
    }

    /// [`Self::create_funded_asset_lock_proof_with_funding`] that reports the
    /// built lock's outpoint to `on_built` (see [`AssetLockBuiltObserver`]).
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn create_funded_asset_lock_proof_with_funding_observed<
        S: ExtendedPubKeySigner,
    >(
        &self,
        amount: AssetLockBuildAmount,
        funding_account: AssetLockFundingAccount,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        recipient: Option<AssetLockRecipient>,
        signer: &S,
        on_built: Option<&AssetLockBuiltObserver<'_>>,
        on_dispatch: impl FnOnce(OutPoint) + Send,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath, OutPoint), PlatformWalletError> {
        self.create_funded_asset_lock_proof_pooled(
            amount,
            &[AccountTypePreference::from(funding_account)],
            funding_account.account_index(),
            funding_type,
            identity_index,
            recipient,
            signer,
            on_built,
            on_dispatch,
        )
        .await
    }

    /// Source-list form of [`Self::create_funded_asset_lock_proof`] — same
    /// build → broadcast → proof pipeline with the pooled funding and amount
    /// semantics of [`Self::build_asset_lock_transaction_with_funding`].
    ///
    /// `on_built` receives the new lock's outpoint once the lock is tracked,
    /// before its row is persisted or it is broadcast (see
    /// [`AssetLockBuiltObserver`]).
    /// `on_dispatch` receives it after persistence admission, before awaiting
    /// network acceptance, which may arrive only with the InstantSend lock.
    #[allow(clippy::too_many_arguments)]
    async fn create_funded_asset_lock_proof_pooled<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_sources: &[AccountTypePreference],
        source_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        recipient: Option<AssetLockRecipient>,
        signer: &S,
        on_built: Option<&AssetLockBuiltObserver<'_>>,
        on_dispatch: impl FnOnce(OutPoint) + Send,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath, OutPoint), PlatformWalletError> {
        let (path, out_point) = self
            .broadcast_funded_asset_lock_tracked(
                amount,
                funding_sources,
                source_index,
                funding_type,
                identity_index,
                recipient,
                signer,
                None,
                on_built,
                on_dispatch,
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
    /// the broadcaster accepts the transaction, with the row at `Broadcast` or
    /// a later status installed by a concurrent resume, BEFORE any proof wait.
    /// This lets a caller durably record its own bookkeeping for the funded lock
    /// (e.g. the inviter-side invitation row) between the broadcast and the
    /// potentially long proof wait in
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
            None,
        )
        .await
    }

    /// Funding-parameterized form of [`Self::broadcast_funded_asset_lock`].
    /// `on_built` receives the new lock's outpoint once the lock is tracked,
    /// before its row is persisted or it is broadcast (see
    /// [`AssetLockBuiltObserver`]).
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn broadcast_funded_asset_lock_with_funding<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_sources: &[AccountTypePreference],
        source_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
        on_built: Option<&AssetLockBuiltObserver<'_>>,
    ) -> Result<(DerivationPath, OutPoint), PlatformWalletError> {
        self.broadcast_funded_asset_lock_with_operation(
            amount,
            funding_sources,
            source_index,
            funding_type,
            identity_index,
            signer,
            None,
            on_built,
        )
        .await
    }

    /// `on_built` receives the new lock's outpoint before its row is stored
    /// (see [`AssetLockBuiltObserver`]).
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create_wallet_drain_asset_lock_proof<S: ExtendedPubKeySigner>(
        &self,
        account_index: u32,
        minimum_lock_duffs: Option<u64>,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        recipient: Option<AssetLockRecipient>,
        signer: &S,
        operation: &AssetLockOperationContext,
        on_built: Option<&AssetLockBuiltObserver<'_>>,
        on_dispatch: impl FnOnce(OutPoint) + Send,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath, OutPoint), PlatformWalletError> {
        self.require_wallet_drain_persistence()?;
        let (path, out_point) = self
            .broadcast_funded_asset_lock_tracked(
                AssetLockBuildAmount::DrainAll { minimum_lock_duffs },
                &ASSET_LOCK_FUNDING_SOURCES,
                account_index,
                funding_type,
                identity_index,
                recipient,
                signer,
                Some(operation),
                on_built,
                on_dispatch,
            )
            .await?;
        let proof = self
            .wait_for_funded_asset_lock_proof(&out_point, account_index)
            .await?;
        Ok((proof, path, out_point))
    }

    #[allow(clippy::too_many_arguments)]
    async fn broadcast_funded_asset_lock_with_operation<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_sources: &[AccountTypePreference],
        source_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
        operation: Option<&AssetLockOperationContext>,
        on_built: Option<&AssetLockBuiltObserver<'_>>,
    ) -> Result<(DerivationPath, OutPoint), PlatformWalletError> {
        self.broadcast_funded_asset_lock_tracked(
            amount,
            funding_sources,
            source_index,
            funding_type,
            identity_index,
            None,
            signer,
            operation,
            on_built,
            |_| {},
        )
        .await
    }

    /// Settles a build that is abandoned before it reached the broadcaster:
    /// the owner-guarded reservation is released across every contributing
    /// account and the in-broadcast fence comes down after it. The released
    /// verdict is recorded before the cleanup's first await, so a cancellation
    /// inside it still settles released — nothing was sent, and a pending-spend
    /// fence would wait for a spend that can never be observed. The caller
    /// drops the build→persist gate first.
    async fn abandon_unsent_build(
        &self,
        mut in_broadcast_pin: InBroadcastPin,
        funding_accounts: &[AccountType],
        tx: &Transaction,
        reservation_token: Option<key_wallet::ReservationToken>,
    ) {
        in_broadcast_pin.settle_released_on_drop();
        crate::wallet::reservations::release_reservation_after_rejected_broadcast(
            &self.wallet_manager,
            &self.wallet_id,
            funding_accounts,
            tx,
            reservation_token,
        )
        .await;
        in_broadcast_pin.settle_released();
    }

    /// Hand back the funding key index of a lock that provably never reached
    /// the network and can no longer be resumed: nothing was sent, and
    /// nothing on chain would ever mark the index used, so kept it would use
    /// up one of the account's few pre-generated addresses on every such exit,
    /// until none is left to build on. Only on the generation the build ran
    /// on, which a replacement wallet may since have built at this very
    /// index. Not for an invitation: its funding key becomes a bearer
    /// voucher, so an index once handed out is never reused.
    ///
    /// The stored used mark is cleared too
    /// ([`WalletPersister::release_unsent_pool_address`](crate::wallet::persister::WalletPersister::release_unsent_pool_address)),
    /// while the manager is still held, so no build can take the index in
    /// between. Kept stored, every such exit would move the account's next
    /// lock one address further after a restart, past the addresses a
    /// restore from the seed scans, and that lock would not be found again.
    /// A release that fails is only logged: the index is then skipped after a
    /// restart, as before, and nothing is lost that was sent.
    async fn hand_back_unsent_funding_index(
        &self,
        in_broadcast_pin: &InBroadcastPin,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        path: &DerivationPath,
    ) {
        if funding_type == AssetLockFundingType::IdentityInvitation {
            return;
        }
        let mut wm = self.wallet_manager.write().await;
        if let Some((_, info)) = wm.get_wallet_mut_and_info_mut(&self.wallet_id) {
            if in_broadcast_pin.is_on(&info.generation) {
                let released = Self::release_discarded_funding_index(
                    &mut info.core_wallet,
                    funding_type,
                    identity_index,
                    path,
                );
                if let Some(released) = released {
                    if let Err(error) = self.persister.release_unsent_pool_address(
                        &released.account_type,
                        released.pool_type,
                        released.index,
                        &released.address,
                    ) {
                        tracing::warn!(
                            account_type = ?released.account_type,
                            index = released.index,
                            %error,
                            "unsent funding index handed back in memory only; \
                             a restart will skip it"
                        );
                    }
                }
            }
        }
    }

    /// Stores the removal of an unsent lock's row and makes it durable, before
    /// the caller releases the lock's inputs: a removal that is lost would
    /// bring the `Built` row back after a restart, resumable to its
    /// recipient, while the released inputs fund the user's next attempt, and
    /// the recipient would be paid twice.
    ///
    /// Through [`WalletPersister::withdraw_durably`](crate::wallet::persister::WalletPersister::withdraw_durably):
    /// `Ok` only once the removal is on disk and no flush can still write the
    /// row after it.
    fn store_lock_removal_durably(&self, removal: AssetLockChangeSet) -> Result<(), NotDurable> {
        self.persister.withdraw_durably(PlatformWalletChangeSet {
            asset_locks: Some(removal),
            ..Default::default()
        })
    }

    /// An unbound identity top-up lock is built, and a recipient assigned,
    /// only for a backend that keeps the recipient and the lock's signing key
    /// across a restart
    /// ([`PersistenceCapabilities::IDENTITY_RECIPIENT_TOP_UP`]). The recipient
    /// is not on chain, so a backend that loses it would reload the lock
    /// unassigned; it arrives beside the row in a separate write, so only an
    /// atomic changeset keeps a crash from committing one without the other;
    /// and the credit output's key index is recoverable only from the stored
    /// address pool. Checked before anything is built, reserved or assigned.
    pub(crate) fn require_identity_recipient_persistence(&self) -> Result<(), PlatformWalletError> {
        let capabilities = self.persister.persistence_capabilities();
        let required = PersistenceCapabilities::IDENTITY_RECIPIENT_TOP_UP;
        if capabilities.contains(required) {
            Ok(())
        } else {
            Err(PlatformWalletError::Persistence(format!(
                "an unbound identity top-up requires persistence capabilities {:?}",
                capabilities.missing(required).names()
            )))
        }
    }

    /// The build → persist → track → broadcast pipeline every funded lock
    /// goes through. `recipient` is who the lock will pay. For a shielded
    /// funding (Type 18, the Orchard address) and an unbound identity top-up
    /// (the identity) it is recorded on the row created here (see
    /// [`recorded_recipients`]), so it is exactly as durable before dispatch
    /// as `operation_id`: flushed before the broadcast for a drain, and for a
    /// recipient the backend can record; queued before it otherwise.
    #[allow(clippy::too_many_arguments)]
    async fn broadcast_funded_asset_lock_tracked<S: ExtendedPubKeySigner>(
        &self,
        amount: AssetLockBuildAmount,
        funding_sources: &[AccountTypePreference],
        source_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        recipient: Option<AssetLockRecipient>,
        signer: &S,
        operation: Option<&AssetLockOperationContext>,
        on_built: Option<&AssetLockBuiltObserver<'_>>,
        on_dispatch: impl FnOnce(OutPoint) + Send,
    ) -> Result<(DerivationPath, OutPoint), PlatformWalletError> {
        if operation.is_some() {
            self.require_wallet_drain_persistence()?;
        }
        let (shielded_recipient, identity_recipient) =
            recorded_recipients(funding_type, recipient)?;
        if identity_recipient.is_some() {
            self.require_identity_recipient_persistence()?;
        }
        let amount = hold_top_up_to_minimum(funding_type, amount)?;
        // Serialize build→persist so a concurrent build cannot interleave its
        // pool snapshot with ours. The snapshot is collected under the build's
        // wallet-manager guard; without this serialization, build A's snapshot
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
        //    reach the broadcaster and leave no row a restart could resume,
        //    and on a definitive rejection; left pending otherwise.
        let (tx, path, reservation_token, funding_accounts, in_broadcast_pin, funding_pools) = self
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
        // The funding key index the build consumed is handed back, so the
        // next build of that funding type reuses it.
        //
        // The same verdict covers the asset-lock input cap: Dash Core relays
        // at most `max_asset_lock_transaction_inputs` (100) inputs and drops a
        // larger lock silently (`assetlocktx-too-many-inputs`), and Platform
        // consensus holds an instant proof to the same versioned bound —
        // tighter than the standard-transaction limit key-wallet's builder
        // applies. Any amount can cross it: a drain pools every eligible
        // input, and a fixed amount held only in many small coins selects as
        // many. An oversized lock is abandoned here rather than stranded on
        // the wire; the caller can consolidate the coins and build again.
        let abandon = input_cap_violation(&tx, self.sdk.version()).or_else(|| match amount {
            AssetLockBuildAmount::DrainAll {
                minimum_lock_duffs: Some(minimum),
            } if locked_amount_duffs < minimum => {
                Some(PlatformWalletError::AssetLockTransaction(format!(
                    "drained asset lock of {locked_amount_duffs} duffs is below the required \
                     minimum of {minimum} duffs (the balance cannot clear the shield pool fee); \
                     nothing was broadcast"
                )))
            }
            _ => None,
        });
        if let Some(error) = abandon {
            {
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
                {
                    // Only on the generation this build ran on. The build's
                    // guard has ended: the wallet may since have been removed
                    // and re-created from the same seed, and the new generation
                    // may already have published a lock at this very index.
                    let mut wm = self.wallet_manager.write().await;
                    if let Some((_, info)) = wm.get_wallet_mut_and_info_mut(&self.wallet_id) {
                        if in_broadcast_pin.is_on(&info.generation) {
                            Self::release_discarded_funding_index(
                                &mut info.core_wallet,
                                funding_type,
                                identity_index,
                                &path,
                            );
                        }
                    }
                }
                // The build→persist gate is held until the index is handed
                // back. Released earlier, a concurrent build could snapshot
                // every asset-lock funding pool with this index still used and
                // persist it, and a restart would restore it as used. Lock
                // order is the one every build takes: this gate, then the
                // manager.
                drop(build_persist_guard);
                in_broadcast_pin.settle_released();
                return Err(error);
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
        // anything hits the wire. That flush succeeding is no proof by itself:
        // another flush of the wallet's records may have taken the buffered
        // pool and failed for good, dropping it, or have written an older
        // pool after it, so the gate trusts only a store no other flush can
        // have emptied or written over (`WalletPersister::store_durably`).
        // For an unbound identity top-up the pool is the only record of the
        // credit output's key index, which a resume after a restart needs to
        // sign the top-up: it is not stored here but in the same durable
        // changeset as the row that records the recipient (below), so the two
        // reach disk together or not at all, and a flush between two separate
        // writes cannot commit the row without it.
        // Aborting BEFORE broadcast is harmless (no tx on the wire); the other
        // asset-lock accounts keep their keys on-device, so they stay best-effort.
        let (funding_pools, row_funding_pools) = if identity_recipient.is_some() {
            (Vec::new(), funding_pools)
        } else {
            (funding_pools, Vec::new())
        };
        let pool_durability = if operation.is_some() {
            // A drain runs only where a store is the commit
            // (`require_wallet_drain_persistence`), so no flush can drop its
            // pools: the flush here only notifies the host.
            self.persist_asset_lock_account_pools(funding_pools)
                .and_then(|()| self.persister.flush())
                .map_err(|e| e.to_string())
        } else if funding_type == AssetLockFundingType::IdentityInvitation {
            self.persister
                .store_durably(PlatformWalletChangeSet {
                    account_address_pools: funding_pools,
                    ..Default::default()
                })
                .map_err(|e| e.to_string())
        } else {
            self.persist_asset_lock_account_pools(funding_pools)
                .map_err(|e| e.to_string())
        };
        if let Err(e) = pool_durability {
            tracing::error!(error = %e, "failed to persist asset-lock funding index");
            if operation.is_some() || funding_type == AssetLockFundingType::IdentityInvitation {
                // Only funding pools have been written: no lock row can be
                // recovered and no transaction was sent. Keep the consumed
                // funding index, but discard the unpublished outpoint.
                if let Some(operation) = operation {
                    operation.set(AssetLockOperationState::NotSent, None);
                }
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
                self.abandon_unsent_build(
                    in_broadcast_pin,
                    &funding_accounts,
                    &tx,
                    reservation_token,
                )
                .await;
                if operation.is_some() {
                    return Err(PlatformWalletError::Persistence(format!(
                        "drain funding indices could not be made durable before dispatch: {e}"
                    )));
                }
                return Err(PlatformWalletError::AssetLockTransaction(format!(
                    "aborted before broadcast: could not durably record the invitation \
                     funding index (broadcasting anyway would risk voucher-key reuse on \
                     restart): {e}"
                )));
            }
        }

        // Only where the backend can record a recipient at all: a host without
        // the capability keeps the best-effort row every other funding has.
        // An identity recipient always: the backend's capability was required
        // before the build.
        let recipient_must_be_durable = operation.is_none()
            && (identity_recipient.is_some()
                || (shielded_recipient.is_some()
                    && self
                        .persister
                        .persistence_capabilities()
                        .contains(PersistenceCapabilities::ASSET_LOCK_SHIELDED_RECIPIENTS)));
        let tracked = TrackedAssetLock {
            operation_id: operation.map(|context| context.operation_id),
            out_point,
            transaction: tx.clone(),
            account_index: source_index,
            funding_type,
            identity_index,
            amount: locked_amount_duffs,
            status: AssetLockStatus::Built,
            proof: None,
            shielded_recipient,
            identity_recipient,
        };
        if let Some(operation) = operation {
            operation.retain_checkpoint(tracked.clone());
            // No await separates the successful build, pool checkpoint and row
            // store. Only this boundary can durably expose the outpoint; retain
            // it if store/flush fails because the row may already be committed.
            operation.set(AssetLockOperationState::Pending, Some(out_point));
            // Hand the outpoint to the caller before the row is stored, so a
            // caller that watches persisted rows never sees this lock unowned.
            if let Some(on_built) = on_built {
                on_built(out_point);
            }
            // Durable BEFORE visible: a generic resume can broadcast any
            // tracked Built row immediately. No in-memory insertion is allowed
            // until both the row and the previously stored pools are flushed.
            let mut changeset = AssetLockChangeSet::default();
            changeset.asset_locks.insert(out_point, (&tracked).into());
            self.persister
                .store(PlatformWalletChangeSet {
                    asset_locks: Some(changeset),
                    ..Default::default()
                })
                .and_then(|()| self.persister.flush())
                .map_err(|error| {
                    PlatformWalletError::Persistence(format!(
                        "drain asset lock was not made durable before dispatch: {error}"
                    ))
                })?;
            self.track_asset_lock(tracked, in_broadcast_pin.generation())
                .await;
            operation.clear_checkpoint();
        } else if recipient_must_be_durable {
            // The recipient exists nowhere but this row: a lock recovered from
            // the chain after a kill carries none, and a resume would then pay
            // whatever address the caller names. So, like the drain above, the
            // row is durable BEFORE it is visible or sent, and a backend that
            // cannot record it stops the funding while nothing is on the wire.
            if let Some(on_built) = on_built {
                on_built(out_point);
            }
            let mut changeset = AssetLockChangeSet::default();
            changeset.asset_locks.insert(out_point, (&tracked).into());
            // Durable only once no other flush of the wallet's records can
            // have dropped the row (`WalletPersister::store_durably`), and
            // with the funding pool for an unbound top-up (see the pool gate
            // above): a resume after a restart signs with the key index it
            // records.
            let durable = self.persister.store_durably(PlatformWalletChangeSet {
                asset_locks: Some(changeset),
                account_address_pools: row_funding_pools,
                ..Default::default()
            });
            if let Err(error) = durable {
                // The row may survive the failure: a failed flush may leave it
                // buffered (a backend that restores a failed flush to its
                // buffer) or committed (a store that succeeded before it), and
                // a refused store may still have written it (a host without
                // atomic changesets can write the row and fail on its recipient
                // sidecar). A later flush or a restart would then surface a
                // resumable row. Its removal is stored behind it — stores apply
                // removals after upserts, and a buffered removal commits with
                // the row it cancels — and made durable before the inputs are
                // released: on a buffered backend another flush may already
                // have committed the row, and a removal left only buffered
                // would be lost with a crash. If the removal is refused or
                // cannot be made durable, the row may outlive this call:
                // releasing its inputs would let the user's retry be funded
                // while a restart re-broadcasts the abandoned lock and pays
                // the recipient a second time. The reservation and the
                // in-broadcast fence then stay held, as for an uncertain
                // removal after a rejected broadcast. Once the removal is
                // durable nothing can send the lock, so its funding index is
                // handed back, before the build→persist gate is dropped.
                let removal = self.store_lock_removal_durably(AssetLockChangeSet {
                    removed: [out_point].into(),
                    ..Default::default()
                });
                let mut in_broadcast_pin = in_broadcast_pin;
                if removal.is_ok() {
                    in_broadcast_pin.settle_released_on_drop();
                    self.hand_back_unsent_funding_index(
                        &in_broadcast_pin,
                        funding_type,
                        identity_index,
                        &path,
                    )
                    .await;
                }
                drop(build_persist_guard);
                let kind = if identity_recipient.is_some() {
                    "identity"
                } else {
                    "shielded"
                };
                if let Err(removal_error) = removal {
                    tracing::warn!(
                        %out_point,
                        %error,
                        %removal_error,
                        "abandoned asset lock's row may survive; keeping its inputs reserved"
                    );
                    in_broadcast_pin.settle_pending_spend();
                    // Not `Persistence`, which the exit below returns once the
                    // row is gone and the inputs are free, so a retry is safe:
                    // here a retry funded from other coins would pay the
                    // recipient a second time when a restart resumes the row.
                    // The unknown outcome says what holds: do not retry,
                    // reconcile (resume or let the row settle) first.
                    return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                        format!(
                            "asset lock {out_point} was not sent, but its {kind} recipient \
                             was not made durable before dispatch and the abandoned row's \
                             removal was not made durable ({removal_error}), so a restart \
                             may still resume it and its inputs stay reserved: {error}"
                        ),
                    ));
                }
                self.abandon_unsent_build(
                    in_broadcast_pin,
                    &funding_accounts,
                    &tx,
                    reservation_token,
                )
                .await;
                return Err(PlatformWalletError::Persistence(format!(
                    "{kind} recipient was not made durable before dispatch: {error}"
                )));
            }
            self.track_asset_lock(tracked, in_broadcast_pin.generation())
                .await;
        } else {
            let cs_built = self
                .track_asset_lock(tracked, in_broadcast_pin.generation())
                .await;
            // Hand the outpoint to the caller before the row is persisted, so a
            // caller that watches persisted rows never sees this lock unowned.
            // Nothing is awaited between here and the queueing below, so once
            // the caller has the outpoint the lock is tracked and its row is
            // queued (the persist itself is best-effort: a failed store is only
            // logged).
            if cs_built.asset_locks.contains_key(&out_point) {
                if let Some(on_built) = on_built {
                    on_built(out_point);
                }
            }
            self.queue_asset_lock_changeset(cs_built);
        }
        drop(build_persist_guard);

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
        //    A resume that has snapshotted the row but not yet recorded its
        //    send is still `Built`, so `claim_resume_dispatch` excludes the
        //    removal. The exclusion survives an ambiguous cancellation after
        //    a possible send or observed proof, until the row advances.
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
        //
        //    The send runs under the build generation's lifecycle gate, after
        //    checking that generation is still the wallet's: a removal can
        //    otherwise complete while the build or its durability gate is
        //    awaiting, discarding the reservation, and a stale handle would then
        //    push the removed wallet's transaction onto the network beside
        //    whatever the wallet (or one re-created under its id) builds from
        //    the same inputs. A removed generation sends nothing. The gate is
        //    taken holding no manager lock, as teardown takes it.
        let lifecycle = in_broadcast_pin.generation().payment_guard_owned().await;
        let is_current = self
            .wallet_manager
            .read()
            .await
            .get_wallet_info(&self.wallet_id)
            .is_some_and(|info| in_broadcast_pin.is_on(&info.generation));
        if !is_current {
            // This generation's teardown has completed; there is nothing left
            // for its gate to exclude.
            drop(lifecycle);
            // Nothing was sent by this call, but the outcome is unknown, never
            // a retry-safe removal: the row was published (and possibly
            // stored) before the gate, so a resume may have sent it already,
            // a wallet re-created under this id may have restored it, or a
            // load may be about to publish it from the store. None of that can
            // be ruled out from here, and deleting the row then releasing the
            // inputs would let the caller's retry pay the recipient twice. The
            // row and the in-broadcast fence stay; the caller resumes this
            // outpoint (or lets the row settle) instead of retrying.
            in_broadcast_pin.settle_pending_spend();
            return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                format!(
                    "asset lock {out_point} was not sent by this call: wallet {} was removed \
                     before the send, and the lock may still be sent by a resume or by the \
                     wallet now under that id, so its row and inputs stay; resume it instead \
                     of funding another",
                    hex::encode(self.wallet_id)
                ),
            ));
        }
        on_dispatch(out_point);
        let broadcast_outcome = self.broadcaster.broadcast(&tx).await;
        // On an error the gate stays held through the cleanup below (every
        // arm returns): a teardown that completed in between could otherwise
        // hand the outpoint to a wallet re-created under this id, whose
        // resume the cleanup's claim check would not see.
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
                // an untrack guard fired instead — the row advanced past
                // `Built`, or active/sticky resume state says this transaction
                // may already be live or committed to dispatch — the inputs
                // must stay reserved exactly like a `MaybeSent` outcome, or the
                // still-tracked row would be resumable while its inputs are
                // re-spendable.
                let removed_built_row = cs_untrack.removed.contains(&out_point);
                // Every removed row is removed durably, whatever its funding
                // type: a row that reached the store (durably for a drain or a
                // recipient-carrying lock, best-effort for the others) and
                // whose removal is lost comes back after a restart as a
                // resumable `Built` lock, while the inputs released below fund
                // the user's next attempt, so the same payment could be made
                // twice. The retry verdict is given only once the removal is
                // durable.
                if removed_built_row {
                    let kind = if operation.is_some() {
                        "drain"
                    } else if recipient_must_be_durable {
                        "recipient-carrying"
                    } else {
                        "asset-lock"
                    };
                    if let Err(error) = self.store_lock_removal_durably(cs_untrack) {
                        // The durable removal is unknown. Keep the context
                        // and input fence pending, but do NOT re-expose the row:
                        // the removal may have committed, and a generic resume
                        // must never dispatch a row with uncertain durability.
                        // A failed flush lands here only on a buffered backend,
                        // which may not have committed the removal; on one
                        // that commits each store inline the removal is
                        // already durable, and a failed notification must not
                        // strand the context or the inputs.
                        //
                        // The unknown outcome, not `Persistence`: the row may
                        // come back after a restart and be resumed, so a retry
                        // funded from other coins could pay twice.
                        return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                            format!(
                                "asset lock {out_point} was rejected at broadcast, but the \
                                 removal of its {kind} row could not be made durable, so the \
                                 row may survive a restart and its inputs stay reserved: {error}"
                            ),
                        ));
                    }
                } else {
                    self.queue_asset_lock_changeset(cs_untrack);
                }
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
                    // The durable row is gone. Publish the known-unsent result
                    // before cancellable cleanup can leave a stale context.
                    if let Some(operation) = operation {
                        operation.set(AssetLockOperationState::NotSent, None);
                    }
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
                    // Rejected, and its row durably gone: nothing can send the
                    // lock, so its funding index goes back too. Under the
                    // build→persist gate, as on the pre-broadcast exits: a
                    // build that ran while this lock was out snapshotted every
                    // funding pool with the index still used, and stores that
                    // snapshot before it lets the gate go. Released before
                    // that store, the index would be marked used again on disk
                    // and skipped after a restart. Lock order is the one every
                    // build takes: this gate, then the manager.
                    let build_persist_guard = self.build_persist_serial.lock().await;
                    self.hand_back_unsent_funding_index(
                        &in_broadcast_pin,
                        funding_type,
                        identity_index,
                        &path,
                    )
                    .await;
                    drop(build_persist_guard);
                    in_broadcast_pin.settle_released();
                } else {
                    // An untrack guard fired: the row advanced past `Built`, or
                    // resume state says the transaction may already be live or
                    // committed to dispatch. The reservation stays held, and
                    // so must the fence.
                    in_broadcast_pin.settle_pending_spend();
                    // The cleanup did not run, so the definite-rejection
                    // contract does not hold either. `TransactionBroadcast`
                    // promises the caller that the row is gone, the inputs are
                    // free, and a rebuild is safe; here the row is still
                    // tracked and resumable and its inputs are still reserved
                    // and fenced, so a caller honouring that promise would
                    // rebuild from other UTXOs and create a SECOND asset lock.
                    // The concurrent resume may have sent the transaction or
                    // may still be committed to dispatch. The contract that
                    // matches what is actually true is the unknown outcome: do
                    // not retry; the row and its reservation are intact, so
                    // resume the existing lock.
                    //
                    // The price is that the reservation and the fence outlive
                    // this call: the fence ends on an observed spend, and no
                    // second cleanup pass exists to reconsider once the
                    // concurrent resume settles. That is the same price every
                    // ambiguous outcome already pays, and it is the only side
                    // that is safe to be wrong on — the row stays tracked and
                    // resumable, so the value behind it is recovered by a
                    // later resume rather than lost.
                    tracing::warn!(
                        %txid,
                        error = %e,
                        "asset lock broadcast was rejected, but resume state excludes \
                         cleanup of the same row; keeping the row and its funding \
                         reservation and reporting an unknown outcome rather than a \
                         definite rejection"
                    );
                    return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                        format!(
                            "asset lock {out_point} stays tracked and reserved: the \
                             broadcast was rejected, but resume state excludes cleanup \
                             of the same row, so the transaction may be on the network \
                             or committed to dispatch: {e}"
                        ),
                    ));
                }
            } else {
                // Ambiguous `MaybeSent`: the transaction may be on the network.
                in_broadcast_pin.settle_pending_spend();
            }
            return Err(e.into());
        }
        drop(lifecycle);
        // Accepted. On the DAPI broadcaster nothing was injected locally, so the
        // inputs are still selectable here until the spend is observed.
        in_broadcast_pin.settle_pending_spend();

        // 4. Transition to Broadcast only if no concurrent flow advanced the
        //    row while this call awaited the network. Replacing a finalized
        //    status here would retain its proof under the weaker status.
        if let Some(cs_broadcast) = self
            .advance_asset_lock_status_if(
                &out_point,
                |current| *current == AssetLockStatus::Built,
                AssetLockStatus::Broadcast,
                None,
            )
            .await?
        {
            self.queue_asset_lock_changeset(cs_broadcast);
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
        let cs_final = self
            .advance_asset_lock_status(out_point, status, Some(proof.clone()))
            .await?;
        self.queue_asset_lock_changeset(cs_final);

        Ok(proof)
    }
}

/// The recipients the tracked row of a new lock records, as
/// `(shielded_recipient, identity_recipient)`.
///
/// - A shielded funding records its Orchard address when it has one.
/// - An unbound identity top-up must name the identity it pays: its credit
///   output is the next address of the wallet's unbound top-up account, so
///   nothing on chain names the identity, and a lock built without a record
///   could later be resumed into whichever identity a recovery passes.
/// - A bound top-up may name its identity, but records nothing: the account's
///   registration index already names it.
/// - Every other pairing of a funding type and a recipient is refused, so a
///   recipient is never silently dropped.
fn recorded_recipients(
    funding_type: AssetLockFundingType,
    recipient: Option<AssetLockRecipient>,
) -> Result<(Option<ShieldedRecipientRaw>, Option<Identifier>), PlatformWalletError> {
    match (funding_type, recipient) {
        (
            AssetLockFundingType::AssetLockShieldedAddressTopUp,
            Some(AssetLockRecipient::Shielded(raw)),
        ) => Ok((Some(raw), None)),
        (AssetLockFundingType::IdentityTopUpNotBound, Some(AssetLockRecipient::Identity(id))) => {
            Ok((None, Some(id)))
        }
        (AssetLockFundingType::IdentityTopUpNotBound, _) => {
            Err(PlatformWalletError::InvalidParameter(
                "an unbound identity top-up lock must name the identity it pays".into(),
            ))
        }
        (AssetLockFundingType::IdentityTopUp, Some(AssetLockRecipient::Identity(_)))
        | (_, None) => Ok((None, None)),
        (funding_type, Some(recipient)) => Err(PlatformWalletError::InvalidParameter(format!(
            "a {funding_type:?} asset lock cannot record recipient {recipient:?}"
        ))),
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
/// `BuilderError::FundsAwaitingFinality` is not a shortfall: the coins exist
/// but are not final yet, so it maps to the typed
/// [`PlatformWalletError::AssetLockFundsAwaitingFinality`] instead.
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
        AssetLockError::Builder(BuilderError::TooManyInputs { count, max }) => {
            PlatformWalletError::AssetLockTooManyInputs { count, max }
        }
        AssetLockError::Builder(BuilderError::FundsAwaitingFinality { waiting }) => {
            PlatformWalletError::AssetLockFundsAwaitingFinality { waiting }
        }
        other => {
            PlatformWalletError::AssetLockTransaction(format!("Asset lock builder failed: {other}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    use dashcore::Address as DashAddress;
    use dashcore::OutPoint;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::bip32::{ChildNumber, DerivationPath};
    use key_wallet::managed_account::address_pool::AddressPoolType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{TransactionContext, TransactionType};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
    use key_wallet::wallet::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;
    use tokio::sync::Notify;

    use async_trait::async_trait;
    use dashcore::{Transaction, Txid};
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::changeset::{
        AssetLockRecipient, ClientStartState, ClientWalletStartState, PersistenceError,
        PersistenceErrorKind, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    #[cfg(feature = "shielded")]
    use crate::test_support::orchard_address_fixture;
    use crate::test_support::{
        funded_wallet_manager, funded_wallet_manager_dual_standard,
        funded_wallet_manager_with_contact, funded_wallet_manager_with_outputs,
        AlwaysMaybeSentBroadcaster, AlwaysOkBroadcaster, AlwaysRejectedBroadcaster, WalletSigner,
    };
    use crate::wallet::asset_lock::funding_account::funding_account_type;
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::asset_lock::tracked::AssetLockStatus;
    use crate::wallet::asset_lock::tracked::TrackedAssetLock;
    use crate::wallet::asset_lock::AssetLockFunding;
    use crate::wallet::core::WalletGeneration;
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use crate::wallet::platform_wallet::WalletId;
    use crate::{AssetLockFundingType, PlatformWalletError, ASSET_LOCK_FUNDING_SOURCES};
    use std::collections::BTreeMap;

    use crate::changeset::AccountAddressPoolEntry;
    use crate::manager::restore_account_address_pools;
    use crate::wallet::address_pool_restore::{derive_through, MAX_RESTORE_GAP_FILL};
    use key_wallet::managed_account::address_pool::AddressState;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use super::{AssetLockBuildAmount, AssetLockOperationContext};

    /// An invitation row persisted past what the restore may derive leaves
    /// every index up to it possibly exported in a voucher. A lock seen on
    /// chain later refills the pool with keys below that row, and none of
    /// them may fund the next voucher.
    #[test]
    fn should_refuse_a_funding_key_below_a_row_the_restore_could_not_reach() {
        let wallet = Wallet::from_seed_bytes(
            [5u8; 64],
            Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .expect("wallet from seed");
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let key_source = wallet.key_source_for_account(AccountType::IdentityInvitation);
        let mut probe = info
            .accounts
            .identity_invitation
            .as_mut()
            .expect("managed invitation account")
            .managed_account_type_mut()
            .address_pools_mut()
            .remove(0)
            .clone();
        let far = probe.highest_generated().map_or(0, |h| h + 1) + MAX_RESTORE_GAP_FILL + 1;
        derive_through(&mut probe, &key_source, far).expect("derive the persisted row");
        let mut row = probe.addresses()[&far].clone();
        row.state = AddressState::Used;

        let stats = restore_account_address_pools(
            &wallet,
            &mut info,
            vec![AccountAddressPoolEntry {
                account_type: AccountType::IdentityInvitation,
                pool_type: probe.pool_type(),
                addresses: vec![row],
            }],
        );
        assert_eq!(
            stats.funding_issuance_floors,
            BTreeMap::from([(AccountType::IdentityInvitation, far)])
        );
        // The refill key-wallet's transaction check runs for a lock that pays
        // a pool address.
        let account = info
            .accounts
            .identity_invitation
            .as_mut()
            .expect("managed invitation account");
        let first = probe.addresses()[&0].address.clone();
        account.mark_address_used(&first);
        for pool in account.managed_account_type_mut().address_pools_mut() {
            pool.maintain_gap_limit(&key_source)
                .expect("refill the pool");
        }

        let next = AssetLockManager::<AlwaysOkBroadcaster>::peek_next_funding_address(
            &mut info,
            AssetLockFundingType::IdentityInvitation,
            0,
            &stats.funding_issuance_floors,
        );

        assert!(
            matches!(next, Err(PlatformWalletError::AssetLockTransaction(_))),
            "{next:?}"
        );
    }

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

        // Coins that are not final yet -> typed, carrying the waiting amount.
        match map_builder_error(
            AssetLockError::Builder(BuilderError::FundsAwaitingFinality { waiting: 600 }),
            999,
        ) {
            PlatformWalletError::AssetLockFundsAwaitingFinality { waiting } => {
                assert_eq!(waiting, 600);
            }
            other => panic!("expected typed AssetLockFundsAwaitingFinality, got {other:?}"),
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
        fail_load: std::sync::atomic::AtomicBool,
        fail_next_pool_store: std::sync::atomic::AtomicBool,
        fail_on_flush: Option<usize>,
        restored_wallet: Option<(Wallet, ManagedWalletInfo)>,
        before_remove: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
        /// Run as a recipient assignment reaches the backend.
        on_assign: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
        restore_stored_locks: std::sync::atomic::AtomicBool,
        commits_inline: bool,
        fail_remove_store: bool,
        committed_stores: std::sync::atomic::AtomicUsize,
        fail_lock_store: std::sync::atomic::AtomicBool,
        /// With `fail_lock_store`: refuse the lock store as transient (a
        /// retryable condition) rather than fatal.
        lock_store_failure_is_transient: std::sync::atomic::AtomicBool,
        /// Fail flushes from now on, without touching `fail_flush`.
        fail_flushes: std::sync::atomic::AtomicBool,
        /// With `fail_flushes`: fail them as transient (the backend keeps
        /// what it buffered) rather than fatal.
        flush_failure_is_transient: std::sync::atomic::AtomicBool,
        failed_lock_stores: Mutex<Vec<PlatformWalletChangeSet>>,
        /// Recipients committed through `assign_asset_lock_identity_recipient`.
        assigned: Mutex<Vec<(OutPoint, dpp::prelude::Identifier)>>,
        /// Addresses whose stored used mark was cleared through
        /// `release_unsent_pool_address`.
        released: Mutex<Vec<(AccountType, AddressPoolType, u32, DashAddress)>>,
        records_identity_recipients: bool,
        /// Declare exactly these capabilities instead of the fixture's.
        capabilities_override: Option<crate::changeset::PersistenceCapabilities>,
        /// Whether the backend also attests `ASSET_LOCK_SHIELDED_RECIPIENTS`.
        records_shielded_recipients: bool,
        /// Once set, the next flush is overtaken: before it commits, a flush
        /// through this persister fails for good, as one from another thread
        /// would, dropping what was buffered; the overtaken flush then
        /// succeeds over nothing.
        overtaken_by: Mutex<Option<WalletPersister>>,
        /// Whether the overtaking flush fails transiently instead, putting
        /// back what it took: the overtaken flush then writes it.
        overtaking_puts_back: std::sync::atomic::AtomicBool,
        /// While set, a flush fails for good: the backend drops what it
        /// buffered.
        flush_drops_buffer: std::sync::atomic::AtomicBool,
        /// While set, a flush fails transiently: the backend keeps what it
        /// buffered.
        flush_keeps_buffer: std::sync::atomic::AtomicBool,
        /// Whether the overtaking flush succeeds instead, writing what was
        /// buffered: the overtaken flush then fails.
        overtaking_commits: std::sync::atomic::AtomicBool,
        /// A store of asset-lock rows writes them and then fails, as on a
        /// host that does not roll a failed round back and whose later
        /// callback failed.
        lock_store_fails_after_writing: bool,
        /// Once set, the next flush finds what was buffered taken by a flush
        /// through this persister, begun on another thread, that has not
        /// returned: it succeeds over nothing. The other flush writes what
        /// it took, behind anything written meanwhile, when it is let go
        /// (`let_stalled_flush_write`, or the next flush that writes).
        stalled_by: Mutex<Option<WalletPersister>>,
        /// Where the flush begun on the other thread stalls.
        stall_point: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
        /// The stalled flush: how to let it go, and how to hear it returned.
        stalled_flush: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
    }

    impl CapturingPersistence {
        fn for_wallet(
            manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
            wallet_id: WalletId,
        ) -> Self {
            let manager = manager.try_read().expect("fixture wallet is not locked");
            let (wallet, info) = manager.get_wallet_and_info(&wallet_id).unwrap();
            Self {
                restored_wallet: Some((wallet.clone(), info.core_wallet.clone())),
                // Drains admit inline-committing backends only; a test that
                // needs the refused buffered mode overrides this explicitly.
                commits_inline: true,
                ..Default::default()
            }
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

        /// Outpoints of the asset-lock rows on disk, which a restart would
        /// load: what the committed changesets leave, each one's removals
        /// applied after its rows.
        fn rows_on_disk(&self) -> Vec<OutPoint> {
            let mut rows = std::collections::BTreeSet::new();
            for locks in self
                .stored
                .lock()
                .expect("capturing persistence mutex")
                .iter()
                .take(
                    self.committed_stores
                        .load(std::sync::atomic::Ordering::SeqCst),
                )
                .filter_map(|cs| cs.asset_locks.as_ref())
            {
                rows.extend(locks.asset_locks.keys().copied());
                for removed in &locks.removed {
                    rows.remove(removed);
                }
            }
            rows.into_iter().collect()
        }

        /// Lets a flush stalled on another thread write what it took, and
        /// waits until it has returned. Does nothing when none is stalled.
        fn let_stalled_flush_write(&self) {
            let stalled = self.stalled_flush.lock().unwrap().take();
            if let Some((let_go, has_returned)) = stalled {
                let_go.send(()).expect("the stalled flush waits");
                has_returned
                    .recv_timeout(Duration::from_secs(10))
                    .expect("the stalled flush returns");
            }
        }

        /// Outpoints of asset-lock rows stored across all changesets.
        fn stored_outpoints(&self) -> Vec<OutPoint> {
            self.stored
                .lock()
                .expect("capturing persistence mutex")
                .iter()
                .filter_map(|cs| cs.asset_locks.as_ref())
                .flat_map(|al| al.asset_locks.keys().copied())
                .collect()
        }
    }

    impl CapturingPersistence {
        /// Forget the recipient the stored rows record at `out_point`, as a
        /// row rebuilt from the chain records none.
        fn forget_recipient(&self, out_point: &OutPoint) {
            for cs in self.stored.lock().unwrap().iter_mut() {
                if let Some(entry) = cs
                    .asset_locks
                    .as_mut()
                    .and_then(|locks| locks.asset_locks.get_mut(out_point))
                {
                    entry.identity_recipient = None;
                }
            }
        }

        /// The recipient the stored rows and assignments record at
        /// `out_point`, and whether a row is stored there at all (stored and
        /// not removed since).
        fn stored_recipient(
            &self,
            out_point: &OutPoint,
        ) -> (bool, Option<dpp::prelude::Identifier>) {
            let mut row = false;
            let mut recipient = None;
            for cs in self.stored.lock().unwrap().iter() {
                let Some(locks) = cs.asset_locks.as_ref() else {
                    continue;
                };
                if let Some(entry) = locks.asset_locks.get(out_point) {
                    row = true;
                    recipient = recipient.or(entry.identity_recipient);
                }
                if locks.removed.contains(out_point) {
                    row = false;
                    recipient = None;
                }
            }
            let assigned = self
                .assigned
                .lock()
                .unwrap()
                .iter()
                .find(|(op, _)| op == out_point)
                .map(|(_, identity)| *identity);
            (row, recipient.or(assigned))
        }
    }

    impl PlatformWalletPersistence for CapturingPersistence {
        fn store_commits_inline(&self) -> bool {
            self.commits_inline
        }

        fn release_unsent_pool_address(
            &self,
            _wallet_id: WalletId,
            account_type: &AccountType,
            pool_type: AddressPoolType,
            index: u32,
            address: &DashAddress,
        ) -> Result<(), PersistenceError> {
            self.released
                .lock()
                .unwrap()
                .push((*account_type, pool_type, index, address.clone()));
            Ok(())
        }

        /// The recipient alone, at once: refused like a lock store under
        /// `fail_lock_store` and like a flush under `fail_flushes` (the
        /// backend writes what it buffered first), and refused as a
        /// constraint for a row it does not store or one recording another
        /// identity. Nothing of a refused one is kept.
        fn assign_asset_lock_identity_recipient(
            &self,
            _wallet_id: WalletId,
            out_point: OutPoint,
            identity: dpp::prelude::Identifier,
        ) -> Result<(), PersistenceError> {
            if let Some(hook) = self.on_assign.lock().unwrap().as_ref() {
                hook();
            }
            if self
                .fail_lock_store
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                if self
                    .lock_store_failure_is_transient
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    return Err(PersistenceError::backend_with_kind(
                        crate::changeset::PersistenceErrorKind::Transient,
                        "simulated busy recipient assignment",
                    ));
                }
                return Err(PersistenceError::backend(
                    "simulated recipient assignment rollback",
                ));
            }
            if !self.commits_inline && self.fail_flushes.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(PersistenceError::backend("simulated flush failure"));
            }
            match self.stored_recipient(&out_point) {
                (false, _) => Err(PersistenceError::backend_with_kind(
                    crate::changeset::PersistenceErrorKind::Constraint,
                    "no asset lock row at the outpoint",
                )),
                (true, Some(recorded)) if recorded != identity => {
                    Err(PersistenceError::backend_with_kind(
                        crate::changeset::PersistenceErrorKind::Constraint,
                        "the row records another identity",
                    ))
                }
                (true, Some(_)) => Ok(()),
                (true, None) => {
                    self.assigned.lock().unwrap().push((out_point, identity));
                    Ok(())
                }
            }
        }

        fn persistence_capabilities(&self) -> crate::changeset::PersistenceCapabilities {
            if let Some(capabilities) = self.capabilities_override {
                return capabilities;
            }
            let mut capabilities = crate::changeset::PersistenceCapabilities::WALLET_DRAIN;
            if self.records_identity_recipients {
                capabilities = capabilities.union(
                    crate::changeset::PersistenceCapabilities::ASSET_LOCK_IDENTITY_RECIPIENTS,
                );
            }
            if self.records_shielded_recipients {
                capabilities = capabilities.union(
                    crate::changeset::PersistenceCapabilities::ASSET_LOCK_SHIELDED_RECIPIENTS,
                );
            }
            capabilities
        }

        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if !changeset.account_address_pools.is_empty()
                && self
                    .fail_next_pool_store
                    .swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                return Err(PersistenceError::backend("simulated pool store failure"));
            }
            if changeset
                .asset_locks
                .as_ref()
                .is_some_and(|locks| !locks.asset_locks.is_empty())
                && self
                    .fail_lock_store
                    .load(std::sync::atomic::Ordering::SeqCst)
            {
                self.failed_lock_stores.lock().unwrap().push(changeset);
                if self
                    .lock_store_failure_is_transient
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    return Err(PersistenceError::backend_with_kind(
                        crate::changeset::PersistenceErrorKind::Transient,
                        "simulated busy lock store",
                    ));
                }
                return Err(PersistenceError::backend("simulated lock store rollback"));
            }
            if changeset
                .asset_locks
                .as_ref()
                .is_some_and(|locks| !locks.removed.is_empty())
            {
                if self.fail_remove_store {
                    return Err(PersistenceError::backend("simulated removal store failure"));
                }
                if let Some(hook) = self.before_remove.lock().unwrap().as_ref() {
                    hook();
                }
            }
            let fails_after_writing = self.lock_store_fails_after_writing
                && changeset
                    .asset_locks
                    .as_ref()
                    .is_some_and(|locks| !locks.asset_locks.is_empty());
            let mut stored = self.stored.lock().expect("capturing persistence mutex");
            stored.push(changeset);
            if self.commits_inline || fails_after_writing {
                self.committed_stores
                    .store(stored.len(), std::sync::atomic::Ordering::SeqCst);
            }
            if fails_after_writing {
                return Err(PersistenceError::backend(
                    "simulated failure after the row was written",
                ));
            }
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            let count = self
                .flushes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            // The flush begun on another thread: it took what was buffered,
            // and writes it only once it is let go.
            let stall_point = self.stall_point.lock().unwrap().take();
            if let Some((entered, let_go)) = stall_point {
                let taken = self.stored.lock().unwrap().clone();
                entered.send(()).expect("the flush it overtook waits");
                let_go
                    .recv_timeout(Duration::from_secs(10))
                    .expect("let go");
                let mut stored = self.stored.lock().unwrap();
                stored.extend(taken);
                self.committed_stores
                    .store(stored.len(), std::sync::atomic::Ordering::SeqCst);
                return Ok(());
            }
            let stalled_by = self.stalled_by.lock().unwrap().take();
            if let Some(persister) = stalled_by {
                let (entered, has_entered) = mpsc::channel();
                let (let_go, released) = mpsc::channel();
                let (returned, has_returned) = mpsc::channel();
                *self.stall_point.lock().unwrap() = Some((entered, released));
                std::thread::spawn(move || {
                    let flushed = persister.flush();
                    assert!(flushed.is_ok(), "{flushed:?}");
                    let _ = returned.send(());
                });
                has_entered
                    .recv_timeout(Duration::from_secs(10))
                    .expect("the other flush is inside the backend");
                *self.stalled_flush.lock().unwrap() = Some((let_go, has_returned));
                return Ok(());
            }
            // Taken in its own statement: the flush below comes back here.
            let overtaking = self.overtaken_by.lock().unwrap().take();
            if let Some(persister) = overtaking {
                if self
                    .overtaking_commits
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    persister.flush().expect("the overtaking flush commits");
                    return Err(PersistenceError::backend("simulated flush failure"));
                } else if self
                    .overtaking_puts_back
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    self.flush_keeps_buffer
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    let overtook = persister.flush();
                    self.flush_keeps_buffer
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    assert!(overtook.is_err_and(|error| error.is_transient()));
                    // What it put back is written by this flush, below.
                } else {
                    self.flush_drops_buffer
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    let overtook = persister.flush();
                    self.flush_drops_buffer
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                    assert!(overtook.is_err_and(|error| !error.is_transient()));
                    return Ok(());
                }
            }
            if self
                .flush_drops_buffer
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                return Err(PersistenceError::backend("the database is corrupt"));
            }
            if self
                .flush_keeps_buffer
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                return Err(PersistenceError::backend_with_kind(
                    PersistenceErrorKind::Transient,
                    "the database is busy",
                ));
            }
            if self.fail_flush
                || self.fail_on_flush == Some(count)
                || self.fail_flushes.load(std::sync::atomic::Ordering::SeqCst)
            {
                if self
                    .flush_failure_is_transient
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    return Err(PersistenceError::backend_with_kind(
                        crate::changeset::PersistenceErrorKind::Transient,
                        "simulated busy flush",
                    ));
                }
                return Err(PersistenceError::backend("simulated flush failure"));
            }
            self.committed_stores.store(
                self.stored.lock().unwrap().len(),
                std::sync::atomic::Ordering::SeqCst,
            );
            // A flush still stalled writes what it took behind this one, and
            // has returned by the time this one does.
            self.let_stalled_flush_write();
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            if self.fail_load.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(PersistenceError::backend("simulated restore failure"));
            }
            let mut state = ClientStartState::default();
            if let Some((wallet, info)) = &self.restored_wallet {
                state.wallets.insert(
                    wallet.compute_wallet_id(),
                    ClientWalletStartState {
                        dashpay_backfill: Default::default(),
                        funding_issuance_floors: Default::default(),
                        wallet: wallet.clone(),
                        wallet_info: info.clone(),
                        identity_manager: Default::default(),
                        unused_asset_locks: Default::default(),
                        unconfirmed_outgoing_txs: Vec::new(),
                    },
                );
            }
            if self
                .restore_stored_locks
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                let wallet = state.wallets.values_mut().next().unwrap();
                for cs in self
                    .stored
                    .lock()
                    .unwrap()
                    .iter()
                    .take(
                        self.committed_stores
                            .load(std::sync::atomic::Ordering::SeqCst),
                    )
                    .filter_map(|cs| cs.asset_locks.as_ref())
                {
                    for entry in cs.asset_locks.values() {
                        wallet
                            .unused_asset_locks
                            .entry(entry.account_index)
                            .or_default()
                            .insert(
                                entry.out_point,
                                TrackedAssetLock {
                                    operation_id: entry.operation_id,
                                    out_point: entry.out_point,
                                    transaction: entry.transaction.clone(),
                                    account_index: entry.account_index,
                                    funding_type: entry.funding_type,
                                    identity_index: entry.identity_index,
                                    amount: entry.amount_duffs,
                                    status: entry.status.clone(),
                                    proof: entry.proof.clone(),
                                    shielded_recipient: entry.shielded_recipient,
                                    identity_recipient: entry.identity_recipient,
                                },
                            );
                    }
                    for removed in &cs.removed {
                        for locks in wallet.unused_asset_locks.values_mut() {
                            locks.remove(removed);
                        }
                    }
                }
            }
            Ok(state)
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

    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_notify_speculation_before_network_acceptance() {
        for drain in [false, true] {
            let (wallet_manager, wallet_id, _, signer) =
                funded_wallet_manager(StandardAccountType::BIP44Account).await;
            let entered = Arc::new(tokio::sync::Barrier::new(2));
            let (manager, persistence) = asset_lock_manager_over(
                wallet_manager,
                wallet_id,
                Arc::new(GatedBroadcaster {
                    entered: entered.clone(),
                    release: Arc::new(tokio::sync::Barrier::new(2)),
                }),
            );
            let operation = Arc::new(AssetLockOperationContext::new([0x71; 16]));
            let _registration = manager.register_wallet_drain_operation(operation.clone());
            let funding = if drain {
                manager.begin_wallet_drain(&operation).await.unwrap();
                AssetLockFunding::DrainWalletBalance {
                    account_index: 0,
                    minimum_lock_duffs: Some(1),
                    operation: operation.clone(),
                }
            } else {
                AssetLockFunding::FromWalletBalance {
                    amount_duffs: 1_000_000,
                    account_index: 0,
                }
            };
            let built = Mutex::new(Vec::new());
            let on_built = |out_point| built.lock().unwrap().push(out_point);
            let (dispatch_tx, mut dispatch_rx) = tokio::sync::oneshot::channel();
            let persisted_at_dispatch = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let drain_out_point_at_dispatch = Arc::new(Mutex::new(None));
            let persisted_at_dispatch_callback = Arc::clone(&persisted_at_dispatch);
            let drain_out_point_at_dispatch_callback = Arc::clone(&drain_out_point_at_dispatch);
            let persistence_for_dispatch = Arc::clone(&persistence);
            let operation_for_dispatch = operation.clone();
            let resolve = manager.resolve_funding_observing_out_point(
                funding,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                Some(AssetLockRecipient::Shielded(
                    orchard_address_fixture(0x71).to_raw_bytes(),
                )),
                &signer,
                Some(&on_built),
                |out_point| {
                    persisted_at_dispatch_callback.store(
                        persistence_for_dispatch
                            .stored_outpoints()
                            .contains(&out_point),
                        std::sync::atomic::Ordering::SeqCst,
                    );
                    if drain {
                        *drain_out_point_at_dispatch_callback.lock().unwrap() =
                            operation_for_dispatch.snapshot().out_point;
                    }
                    dispatch_tx.send(out_point).unwrap();
                },
            );
            tokio::pin!(resolve);
            tokio::select! {
                _ = &mut resolve => panic!("acceptance is withheld"),
                _ = entered.wait() => {}
            }

            let out_point = dispatch_rx
                .try_recv()
                .expect("speculation must be notified before network acceptance");
            assert_eq!(*built.lock().unwrap(), vec![out_point]);
            assert!(
                persisted_at_dispatch.load(std::sync::atomic::Ordering::SeqCst),
                "dispatch callback must run after the asset-lock row is persisted"
            );
            assert!(persistence.stored_outpoints().contains(&out_point));
            let rows = manager.list_tracked_locks().await;
            let row = rows.iter().find(|row| row.out_point == out_point).unwrap();
            assert_eq!(row.status, AssetLockStatus::Built);
            if drain {
                assert_eq!(
                    *drain_out_point_at_dispatch.lock().unwrap(),
                    Some(out_point)
                );
                assert_eq!(operation.snapshot().out_point, Some(out_point));
            }
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
        let (wallet_manager, wallet_id, generation, signer) =
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
            Arc::clone(&generation),
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
                None,
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
                None,
            )
            .await
            .expect("follow-up drain must reselect the released inputs");
        assert_eq!(
            broadcaster.calls(),
            1,
            "the follow-up drain should broadcast exactly once"
        );
    }

    /// The built-lock observer sees the new outpoint exactly once, once the
    /// lock is tracked but before its row is stored or anything has reached
    /// the broadcaster, and the outpoint it sees is the one the build returns.
    #[tokio::test]
    async fn should_report_the_built_outpoint_before_persisting_or_broadcast() {
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer, persistence) =
            coinjoin_funded_asset_lock_manager(Arc::clone(&broadcaster)).await;
        let seen = Mutex::new(Vec::new());
        let observer = |out_point: OutPoint| {
            let tracked = manager
                .wallet_manager
                .try_read()
                .expect("the build holds no wallet-manager lock while observing")
                .get_wallet_and_info(&manager.wallet_id)
                .is_some_and(|(_, info)| info.tracked_asset_locks.contains_key(&out_point));
            let stored = persistence.stored_outpoints().contains(&out_point);
            seen.lock()
                .unwrap()
                .push((out_point, tracked, stored, broadcaster.calls()));
        };

        let (_, out_point) = manager
            .broadcast_funded_asset_lock_with_funding(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(1),
                },
                &[AccountTypePreference::CoinJoin],
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
                Some(&observer),
            )
            .await
            .expect("drain must broadcast");

        assert_eq!(
            seen.into_inner().unwrap(),
            vec![(out_point, true, false, 0)]
        );
        assert_eq!(broadcaster.calls(), 1);
        assert!(persistence.stored_outpoints().contains(&out_point));
    }

    /// The transaction of the asset-lock row `persistence` stored at
    /// `out_point`.
    fn stored_lock_transaction(
        persistence: &CapturingPersistence,
        out_point: &OutPoint,
    ) -> Transaction {
        persistence
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter_map(|cs| cs.asset_locks.as_ref())
            .find_map(|locks| locks.asset_locks.get(out_point))
            .expect("the lock's row was stored")
            .transaction
            .clone()
    }

    /// A wallet removed or re-created under its id after the lock was built
    /// sends nothing: the stale build's transaction never reaches the
    /// broadcaster. The row was published before the send gate, so a resume,
    /// the wallet now under the id or a load from the store may still send
    /// it; that cannot be ruled out, so the row is never removed, its inputs
    /// stay fenced and the outcome is reported as unknown (20) whether or not
    /// anyone is seen holding the lock, and the caller does not rebuild.
    #[tokio::test]
    async fn should_not_broadcast_a_lock_built_on_a_wallet_since_replaced() {
        for (replacement_tracks_row, resume_claimed) in
            [(false, false), (true, false), (false, true)]
        {
            let broadcaster = Arc::new(CountingOkBroadcaster::default());
            let (manager, signer, persistence) =
                coinjoin_funded_asset_lock_manager(Arc::clone(&broadcaster)).await;
            let built_on = Mutex::new(None);
            let claim = Mutex::new(None);
            // Stands in for a removal and re-creation landing while the build
            // awaits: the wallet id now names a different generation, and the
            // removed generation's tracked row went with it unless the
            // replacement restored it from the store.
            let observer = |out_point: OutPoint| {
                if resume_claimed {
                    *claim.lock().unwrap() = Some(manager.claim_resume_dispatch(out_point));
                }
                let mut wm = manager
                    .wallet_manager
                    .try_write()
                    .expect("the build holds no wallet-manager lock while observing");
                let info = wm
                    .get_wallet_info_mut(&manager.wallet_id)
                    .expect("wallet info");
                if !replacement_tracks_row {
                    info.tracked_asset_locks.remove(&out_point);
                }
                let old =
                    std::mem::replace(&mut info.generation, Arc::new(WalletGeneration::new()));
                *built_on.lock().unwrap() = Some((out_point, old));
            };

            let result = manager
                .broadcast_funded_asset_lock_with_funding(
                    super::AssetLockBuildAmount::DrainAll {
                        minimum_lock_duffs: Some(1),
                    },
                    &[AccountTypePreference::CoinJoin],
                    0,
                    AssetLockFundingType::AssetLockShieldedAddressTopUp,
                    0,
                    &signer,
                    Some(&observer),
                )
                .await;

            assert!(
                matches!(
                    result,
                    Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
                ),
                "{replacement_tracks_row} {resume_claimed}: {result:?}"
            );
            assert_eq!(broadcaster.calls(), 0);
            drop(claim);
            let (out_point, built_on) = built_on.into_inner().unwrap().expect("observed");
            let tx = stored_lock_transaction(&persistence, &out_point);
            assert!(persistence.removed_outpoints().is_empty());
            assert!(built_on.in_broadcast_conflict(&tx).is_some());
        }
    }

    /// A lock that records its identity recipient was stored durably before
    /// the wallet was replaced: its row stays (a load may restore it and a
    /// resume send it), the replacement is not handed it by this build, the
    /// inputs stay fenced and the outcome is unknown (20) with the outpoint,
    /// so the caller resumes it instead of paying the identity again.
    #[tokio::test]
    async fn should_keep_a_recipient_lock_built_on_a_wallet_since_replaced() {
        use crate::changeset::AssetLockRecipient;

        let identity = dpp::prelude::Identifier::from([0x58; 32]);
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        let built_on = Mutex::new(None);
        // Runs before the row is stored and tracked, so the build's own
        // tracking finds the replacement.
        let observer = |out_point: OutPoint| {
            let mut wm = manager
                .wallet_manager
                .try_write()
                .expect("the build holds no wallet-manager lock while observing");
            let info = wm
                .get_wallet_info_mut(&manager.wallet_id)
                .expect("wallet info");
            let old = std::mem::replace(&mut info.generation, Arc::new(WalletGeneration::new()));
            *built_on.lock().unwrap() = Some((out_point, old));
        };

        let result = manager
            .broadcast_funded_asset_lock_tracked(
                super::AssetLockBuildAmount::Exact(1_000_000),
                &ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::IdentityTopUpNotBound,
                0,
                Some(AssetLockRecipient::Identity(identity)),
                &signer,
                None,
                Some(&observer),
                |_| {},
            )
            .await;

        assert_eq!(broadcaster.calls(), 0);
        assert!(
            manager.list_tracked_locks().await.is_empty(),
            "the replacement must not be handed a lock it never built"
        );
        let (out_point, built_on) = built_on.into_inner().unwrap().expect("observed");
        let tx = stored_lock_transaction(&persistence, &out_point);
        assert!(
            matches!(&result, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                if message.contains(&out_point.to_string())),
            "{result:?}"
        );
        assert!(
            persistence.removed_outpoints().is_empty(),
            "the row is kept for a resume"
        );
        assert!(built_on.in_broadcast_conflict(&tx).is_some());
    }

    /// A handle whose wallet was removed and re-created under the same id
    /// while it awaited (an identity lookup before a top-up) builds nothing on
    /// the replacement: the build is refused as a removal before anything is
    /// derived, reserved, signed or stored, and nothing reaches the
    /// broadcaster. Once the handle's generation is current again the same
    /// single input funds a lock, so the refusal reserved nothing.
    #[tokio::test]
    async fn should_refuse_to_build_on_a_wallet_since_replaced() {
        use crate::changeset::AssetLockRecipient;

        let identity = dpp::prelude::Identifier::from([0x5d; 32]);
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        let replaced = {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&manager.wallet_id).unwrap();
            std::mem::replace(&mut info.generation, Arc::new(WalletGeneration::new()))
        };

        let result = fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(identity)),
        )
        .await;

        assert!(
            matches!(result, Err(PlatformWalletError::WalletRemovedBeforeSend(_))),
            "{result:?}"
        );
        assert_eq!(broadcaster.calls(), 0);
        assert!(persistence.stored.lock().unwrap().is_empty());
        assert!(manager.list_tracked_locks().await.is_empty());

        manager
            .wallet_manager
            .write()
            .await
            .get_wallet_info_mut(&manager.wallet_id)
            .unwrap()
            .generation = replaced;
        fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(identity)),
        )
        .await
        .expect("the refused build reserved nothing");
        assert_eq!(broadcaster.calls(), 1);
    }

    /// A lock whose broadcast ends without a verdict was reported to the
    /// observer, and its row stays tracked and stored, so the caller can
    /// resume it by the outpoint it was given.
    #[tokio::test]
    async fn should_keep_a_reported_lock_whose_broadcast_may_have_been_sent() {
        let (manager, signer, persistence) =
            coinjoin_funded_asset_lock_manager(Arc::new(AlwaysMaybeSentBroadcaster)).await;
        let seen = Mutex::new(Vec::new());
        let observer = |out_point: OutPoint| seen.lock().unwrap().push(out_point);

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
                Some(&observer),
            )
            .await
            .expect_err("an unconfirmed broadcast must fail the call");

        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.len(), 1);
        assert!(persistence.removed_outpoints().is_empty());
        assert!(persistence.stored_outpoints().contains(&seen[0]));
        let wm = manager.wallet_manager.read().await;
        let (_, info) = wm
            .get_wallet_and_info(&manager.wallet_id)
            .expect("wallet still present");
        assert!(info.tracked_asset_locks.contains_key(&seen[0]));
    }

    /// A lock whose broadcast is rejected was still reported to the observer,
    /// which ran before the broadcast; the call then returns the rejection and
    /// the lock's row is removed again.
    #[tokio::test]
    async fn should_report_a_lock_whose_broadcast_is_then_rejected() {
        let (manager, signer, persistence) =
            coinjoin_funded_asset_lock_manager(Arc::new(AlwaysRejectedBroadcaster)).await;
        let seen = Mutex::new(Vec::new());
        let observer = |out_point: OutPoint| seen.lock().unwrap().push(out_point);

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
                Some(&observer),
            )
            .await
            .expect_err("a rejected broadcast must fail the call");

        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(persistence.removed_outpoints(), seen);
    }

    /// Builds, tracks and broadcasts one unbound identity top-up lock from the
    /// wallet balance, paying `recipient`, without waiting for its proof.
    async fn fund_unbound_top_up<B: TransactionBroadcaster>(
        manager: &AssetLockManager<B>,
        signer: &WalletSigner,
        recipient: Option<crate::changeset::AssetLockRecipient>,
    ) -> Result<(DerivationPath, OutPoint), PlatformWalletError> {
        manager
            .broadcast_funded_asset_lock_tracked(
                super::AssetLockBuildAmount::Exact(1_000_000),
                &ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::IdentityTopUpNotBound,
                0,
                recipient,
                signer,
                None,
                None,
                |_| {},
            )
            .await
    }

    /// An unbound identity top-up lock is refused before anything is built
    /// when it names no identity, names something else, or the backend
    /// cannot keep its recipient across a restart: nothing is stored and
    /// nothing reaches the broadcaster.
    #[tokio::test]
    async fn should_refuse_an_unbound_top_up_lock_that_could_not_keep_its_recipient() {
        use crate::changeset::AssetLockRecipient;

        let identity = AssetLockRecipient::Identity(dpp::prelude::Identifier::from([0x41; 32]));
        for (recipient, records) in [
            (None, true),
            (Some(AssetLockRecipient::Shielded([0x42; 43])), true),
            (Some(identity), false),
        ] {
            let broadcaster = Arc::new(CountingOkBroadcaster::default());
            let persistence = Arc::new(CapturingPersistence {
                records_identity_recipients: records,
                commits_inline: true,
                ..Default::default()
            });
            let (manager, signer) = funded_asset_lock_manager_with_persistence(
                Arc::clone(&broadcaster),
                Arc::clone(&persistence),
            )
            .await;

            let error = fund_unbound_top_up(&manager, &signer, recipient)
                .await
                .expect_err("the lock must be refused");

            if records {
                assert!(
                    matches!(error, PlatformWalletError::InvalidParameter(_)),
                    "{recipient:?}: {error}"
                );
            } else {
                assert!(
                    matches!(&error, PlatformWalletError::Persistence(message)
                        if message.contains("asset_lock_identity_recipients")),
                    "{error}"
                );
            }
            assert!(
                persistence.stored.lock().unwrap().is_empty(),
                "{recipient:?}"
            );
            assert_eq!(broadcaster.calls(), 0, "{recipient:?}");
        }
    }

    /// The identity an unbound top-up lock pays is on its row, durable,
    /// before the lock is broadcast: at its store on an inline-committing
    /// backend (no flush needed), after a flush on a buffered one. A row that
    /// cannot be made durable stops the funding before the broadcaster and
    /// leaves the lock untracked.
    #[tokio::test]
    async fn should_record_the_identity_recipient_with_the_lock_before_dispatch() {
        use crate::changeset::AssetLockRecipient;

        let identity = dpp::prelude::Identifier::from([0x43; 32]);
        for (commits_inline, fail_on_flush) in [(true, None), (false, None), (false, Some(1))] {
            let fail_flush = fail_on_flush.is_some();
            let broadcaster = Arc::new(CountingOkBroadcaster::default());
            let persistence = Arc::new(CapturingPersistence {
                records_identity_recipients: true,
                commits_inline,
                fail_on_flush,
                ..Default::default()
            });
            let (manager, signer) = funded_asset_lock_manager_with_persistence(
                Arc::clone(&broadcaster),
                Arc::clone(&persistence),
            )
            .await;

            let result = fund_unbound_top_up(
                &manager,
                &signer,
                Some(AssetLockRecipient::Identity(identity)),
            )
            .await;

            let stored: Vec<_> = persistence
                .stored
                .lock()
                .unwrap()
                .iter()
                .filter_map(|cs| cs.asset_locks.as_ref())
                .flat_map(|locks| locks.asset_locks.values().cloned())
                .collect();
            let first = stored.first().expect("the lock's row was stored");
            assert_eq!(
                first.funding_type,
                AssetLockFundingType::IdentityTopUpNotBound
            );
            assert_eq!(first.identity_recipient, Some(identity));
            assert_eq!(first.status, AssetLockStatus::Built);
            if fail_flush {
                assert!(
                    matches!(&result, Err(PlatformWalletError::Persistence(message))
                        if message.contains("identity recipient was not made durable")),
                    "{:?}",
                    result.as_ref().err()
                );
                assert_eq!(broadcaster.calls(), 0);
                assert!(manager.list_tracked_locks().await.is_empty());
            } else {
                assert_eq!(broadcaster.calls(), 1);
                assert_eq!(
                    persistence
                        .flushes
                        .load(std::sync::atomic::Ordering::SeqCst)
                        >= 1,
                    !commits_inline,
                    "only a buffered backend needs a flush to make the row durable"
                );
                let tracked = manager.list_tracked_locks().await;
                assert_eq!(tracked.len(), 1);
                assert_eq!(tracked[0].identity_recipient, Some(identity));
            }
        }
    }

    /// The stored address pool is the only record of an unbound top-up's
    /// credit-output key, which a resume after a restart signs the top-up
    /// with, so it is stored in the same durable write as the row recording
    /// the recipient: the two reach disk together or not at all, and no flush
    /// between two separate writes can commit the row without it. A write
    /// that fails stops the funding before the broadcaster: nothing is tracked
    /// and the inputs are released, so an immediate retry funds the lock.
    #[tokio::test]
    async fn should_refuse_an_unbound_top_up_whose_funding_pool_was_not_stored() {
        use crate::changeset::AssetLockRecipient;

        let identity = dpp::prelude::Identifier::from([0x44; 32]);
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            fail_next_pool_store: std::sync::atomic::AtomicBool::new(true),
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let result = fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(identity)),
        )
        .await;

        assert!(
            matches!(&result, Err(PlatformWalletError::Persistence(message))
                if message.contains("identity recipient was not made durable")),
            "{result:?}"
        );
        assert_eq!(broadcaster.calls(), 0);
        assert!(manager.list_tracked_locks().await.is_empty());
        assert!(
            persistence.stored_outpoints().is_empty(),
            "no lock row was stored"
        );

        fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(identity)),
        )
        .await
        .expect("the refused build released its inputs, so a retry funds the lock");
        assert_eq!(broadcaster.calls(), 1);
        let stored = persistence.stored.lock().unwrap();
        let row_write = stored
            .iter()
            .find(|cs| {
                cs.asset_locks
                    .as_ref()
                    .is_some_and(|locks| !locks.asset_locks.is_empty())
            })
            .expect("the row was stored");
        assert!(
            !row_write.account_address_pools.is_empty(),
            "the funding pool is in the row's own write"
        );
        assert!(
            stored
                .iter()
                .filter(|cs| !cs.account_address_pools.is_empty())
                .all(|cs| cs.asset_locks.is_some()),
            "no separate pool write precedes the row"
        );
    }

    /// A refused unbound top-up hands its funding index back: nothing was
    /// sent and nothing on chain would ever mark the index used, so kept, each
    /// failed pool store would use up one of the account's few pre-generated
    /// addresses. However many fail, the next build signs with the first one.
    #[tokio::test]
    async fn should_hand_back_the_funding_index_of_an_unbound_top_up_whose_pool_was_not_stored() {
        use crate::changeset::AssetLockRecipient;
        use key_wallet::bip32::ChildNumber;

        let identity = dpp::prelude::Identifier::from([0x45; 32]);
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;

        for _ in 0..8 {
            persistence
                .fail_next_pool_store
                .store(true, std::sync::atomic::Ordering::SeqCst);
            let result = fund_unbound_top_up(
                &manager,
                &signer,
                Some(AssetLockRecipient::Identity(identity)),
            )
            .await;
            assert!(
                matches!(&result, Err(PlatformWalletError::Persistence(_))),
                "{result:?}"
            );
        }

        let (path, _) = fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(identity)),
        )
        .await
        .expect("the refused builds left the pool's addresses unused");
        assert_eq!(
            path.into_iter().last(),
            Some(&ChildNumber::Normal { index: 0 }),
            "the build signs with the first funding address: {path}"
        );
        assert_eq!(broadcaster.calls(), 1);
    }

    /// The builder holds every top-up lock to Platform's top-up minimum, on
    /// every entry, not only the identity routes: an exact amount below it is
    /// refused before anything is selected, stored or broadcast, for a bound
    /// top-up through the public funded-proof entry too.
    #[tokio::test]
    async fn should_refuse_a_top_up_lock_below_the_minimum_at_the_builder() {
        let below = super::min_top_up_duffs().unwrap() - 1;
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let result = manager
            .create_funded_asset_lock_proof(
                below,
                0,
                AssetLockFundingType::IdentityTopUp,
                0,
                &signer,
            )
            .await;

        assert!(
            matches!(&result, Err(PlatformWalletError::InvalidParameter(message))
                if message.contains("below the minimum top-up")),
            "{:?}",
            result.as_ref().err()
        );
        assert_eq!(broadcaster.calls(), 0);
        assert!(manager.list_tracked_locks().await.is_empty());
        assert!(persistence.stored.lock().unwrap().is_empty());
    }

    /// The build-only (signed-transaction) API is held to the same minimum:
    /// a bound top-up below it is refused before anything is selected, and a
    /// drained one is refused outright, since only the funded pipeline checks
    /// a drain's built value. Nothing is reserved, so a funded build of the
    /// same coins still succeeds.
    #[tokio::test]
    async fn should_hold_the_build_only_api_to_the_top_up_minimum() {
        let below = super::min_top_up_duffs().unwrap() - 1;
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;

        for amount in [
            super::AssetLockBuildAmount::Exact(below),
            super::AssetLockBuildAmount::DrainAll {
                minimum_lock_duffs: None,
            },
        ] {
            let result = manager
                .build_asset_lock_transaction_with_funding(
                    amount,
                    &ASSET_LOCK_FUNDING_SOURCES,
                    0,
                    AssetLockFundingType::IdentityTopUp,
                    0,
                    &signer,
                )
                .await;
            assert!(
                matches!(&result, Err(PlatformWalletError::InvalidParameter(_))),
                "{:?}",
                result.as_ref().err()
            );
        }
        let result = manager
            .build_asset_lock_transaction(below, 0, AssetLockFundingType::IdentityTopUp, 0, &signer)
            .await;
        assert!(
            matches!(&result, Err(PlatformWalletError::InvalidParameter(message))
                if message.contains("below the minimum top-up")),
            "{:?}",
            result.as_ref().err()
        );
        assert_eq!(broadcaster.calls(), 0);
        manager
            .build_asset_lock_transaction(
                super::min_top_up_duffs().unwrap(),
                0,
                AssetLockFundingType::IdentityTopUp,
                0,
                &signer,
            )
            .await
            .expect("the refused builds reserved nothing");
    }

    /// The recipient reaches a host in its own callback beside the lock's
    /// row, so a backend that declares identity recipients without atomic
    /// changesets could commit the row without it in a crash: an unbound
    /// top-up is refused for it before anything is built.
    #[tokio::test]
    async fn should_refuse_an_unbound_top_up_on_a_backend_without_atomic_changesets() {
        use crate::changeset::{AssetLockRecipient, PersistenceCapabilities};

        let identity = dpp::prelude::Identifier::from([0x45; 32]);
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            commits_inline: true,
            capabilities_override: Some(
                PersistenceCapabilities::ASSET_LOCK_FUNDING_INDICES
                    .union(PersistenceCapabilities::ASSET_LOCK_IDENTITY_RECIPIENTS),
            ),
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let result = fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(identity)),
        )
        .await;

        assert!(
            matches!(&result, Err(PlatformWalletError::Persistence(message))
                if message.contains("atomic_changesets")),
            "{result:?}"
        );
        assert_eq!(broadcaster.calls(), 0);
        assert!(persistence.stored.lock().unwrap().is_empty());
    }

    /// A lock row whose store or flush failed before the broadcast has its
    /// removal stored behind it, and made durable, before the inputs are
    /// released, so the abandoned lock never comes back. On a buffered backend
    /// the row's flush fails and the removal's own flush succeeds; on an
    /// inline-committing one the row's store is refused and the removal is
    /// durable at its store. When the store refuses that removal, the row may
    /// survive a restart and be re-broadcast after the user's retry, so the
    /// inputs stay reserved and the error says so.
    #[tokio::test]
    async fn should_durably_remove_a_lock_row_that_was_not_made_durable() {
        use crate::changeset::AssetLockRecipient;

        let identity = dpp::prelude::Identifier::from([0x57; 32]);
        let fund = |persistence: Arc<CapturingPersistence>| async move {
            let broadcaster = Arc::new(CountingOkBroadcaster::default());
            let (manager, signer) = funded_asset_lock_manager_with_persistence(
                Arc::clone(&broadcaster),
                Arc::clone(&persistence),
            )
            .await;
            let result = fund_unbound_top_up(
                &manager,
                &signer,
                Some(AssetLockRecipient::Identity(identity)),
            )
            .await;
            assert_eq!(broadcaster.calls(), 0);
            assert!(manager.list_tracked_locks().await.is_empty());
            result.map(|_| ())
        };
        let persistence = || CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        };

        // The row's flush fails: its removal is stored behind it, and the
        // inputs are released.
        let row_flush_fails = Arc::new(CapturingPersistence {
            commits_inline: false,
            fail_on_flush: Some(1),
            ..persistence()
        });
        let result = fund(Arc::clone(&row_flush_fails)).await;
        assert!(
            matches!(&result, Err(PlatformWalletError::Persistence(message))
                if message.contains("identity recipient was not made durable")
                    && !message.contains("stay reserved")),
            "{result:?}"
        );
        assert_eq!(row_flush_fails.removed_outpoints().len(), 1);

        // The store also refuses the removal: the inputs stay reserved, and
        // the outcome is unknown rather than a retry-safe persistence error.
        let result = fund(Arc::new(CapturingPersistence {
            commits_inline: false,
            fail_on_flush: Some(1),
            fail_remove_store: true,
            ..persistence()
        }))
        .await;
        assert!(
            matches!(&result, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                if message.contains("stay reserved")),
            "{result:?}"
        );

        // The store refuses the row itself, which a host without atomic
        // changesets may still have written in part: it is removed the same
        // way, and the inputs are released only once that removal is stored.
        let row_store_fails = Arc::new(persistence());
        row_store_fails
            .fail_lock_store
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let result = fund(Arc::clone(&row_store_fails)).await;
        assert!(
            matches!(&result, Err(PlatformWalletError::Persistence(message))
                if !message.contains("stay reserved")),
            "{result:?}"
        );
        assert_eq!(row_store_fails.removed_outpoints().len(), 1);

        let both_refused = Arc::new(CapturingPersistence {
            fail_remove_store: true,
            ..persistence()
        });
        both_refused
            .fail_lock_store
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let result = fund(both_refused).await;
        assert!(
            matches!(&result, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                if message.contains("stay reserved")),
            "{result:?}"
        );
    }

    /// On a buffered backend, an abandoned row's removal is released only once
    /// flushed: another flush may already have committed the row, and a
    /// removal left only buffered would be lost with a crash, bringing the
    /// lock back resumable while the released inputs funded the retry. When
    /// that flush fails the inputs stay reserved and fenced, and the error
    /// says so. Nothing reaches the broadcaster either way.
    #[tokio::test]
    async fn should_keep_inputs_reserved_when_an_abandoned_row_removal_cannot_be_flushed() {
        use crate::changeset::AssetLockRecipient;

        let identity = dpp::prelude::Identifier::from([0x5c; 32]);
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: false,
            fail_flush: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let result = fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(identity)),
        )
        .await;

        assert!(
            matches!(&result, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                if message.contains("identity recipient")
                    && message.contains("stay reserved")),
            "{result:?}"
        );
        assert_eq!(broadcaster.calls(), 0);
        assert!(manager.list_tracked_locks().await.is_empty());
        let removed = persistence.removed_outpoints();
        assert_eq!(
            removed.len(),
            1,
            "the removal is still stored behind the row"
        );
        let tx = stored_lock_transaction(&persistence, &removed[0]);
        assert!(
            manager.generation.in_broadcast_conflict(&tx).is_some(),
            "the inputs stay fenced"
        );
    }

    /// Assigns `identity` to the lock at `out_point` through a handle on the
    /// wallet's current generation.
    async fn assign_on_current_generation<B: TransactionBroadcaster>(
        manager: &AssetLockManager<B>,
        out_point: &OutPoint,
        identity: dpp::prelude::Identifier,
    ) -> Result<(), PlatformWalletError> {
        let generation = Arc::clone(
            &manager
                .wallet_manager
                .read()
                .await
                .get_wallet_info(&manager.wallet_id)
                .expect("wallet info")
                .generation,
        );
        manager
            .assign_identity_top_up_recipient(out_point, identity, false, &generation)
            .await
    }

    /// An unbound top-up lock that records no recipient (as one rebuilt from
    /// the chain does) is unassigned; assigning it an identity has the backend
    /// record the recipient alone, with no snapshot of the row, and then
    /// publishes it. Assigning the same identity again succeeds, another
    /// identity is refused, and so is an untracked outpoint.
    #[tokio::test]
    async fn should_assign_an_unassigned_unbound_top_up_lock_once() {
        use crate::changeset::AssetLockRecipient;

        let built_for = dpp::prelude::Identifier::from([0x51; 32]);
        let assigned = dpp::prelude::Identifier::from([0x52; 32]);
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        let (_, out_point) = fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(built_for)),
        )
        .await
        .expect("an unbound top-up lock builds and broadcasts");
        // Forget the recipient, as a lock rebuilt from the chain has none.
        {
            let mut wm = manager.wallet_manager.write().await;
            let lock = wm
                .get_wallet_info_mut(&manager.wallet_id)
                .unwrap()
                .tracked_asset_locks
                .get_mut(&out_point)
                .unwrap();
            lock.identity_recipient = None;
            assert!(lock.is_unassigned_identity_top_up());
        }
        persistence.forget_recipient(&out_point);
        let stores_before = persistence.stored.lock().unwrap().len();

        assign_on_current_generation(&manager, &out_point, assigned)
            .await
            .expect("an unassigned lock takes the identity the user chose");

        let tracked = manager.list_tracked_locks().await;
        assert_eq!(tracked[0].identity_recipient, Some(assigned));
        assert!(!tracked[0].is_unassigned_identity_top_up());
        assert_eq!(
            *persistence.assigned.lock().unwrap(),
            vec![(out_point, assigned)],
            "the backend recorded the recipient"
        );
        assert_eq!(
            persistence.stored.lock().unwrap().len(),
            stores_before,
            "the recipient alone is written, never a snapshot of the row"
        );

        assign_on_current_generation(&manager, &out_point, assigned)
            .await
            .expect("assigning the recorded identity again succeeds");
        assert!(matches!(
            assign_on_current_generation(&manager, &out_point, built_for)
                .await,
            Err(PlatformWalletError::AssetLockIdentityRecipientMismatch {
                recorded,
                requested,
                ..
            }) if recorded == assigned && requested == built_for
        ));
        assert!(matches!(
            assign_on_current_generation(&manager, &OutPoint::null(), assigned).await,
            Err(PlatformWalletError::AssetLockNotTracked(_))
        ));
    }

    /// A handle taken before its wallet was removed or re-created under its
    /// id assigns nothing on the wallet now under that id: the assignment is
    /// refused as a removal, and nothing is recorded or stored.
    #[tokio::test]
    async fn should_refuse_to_assign_a_recipient_on_a_wallet_since_replaced() {
        use crate::changeset::AssetLockRecipient;

        let built_for = dpp::prelude::Identifier::from([0x59; 32]);
        let assigned = dpp::prelude::Identifier::from([0x5a; 32]);
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(CountingOkBroadcaster::default()),
            Arc::clone(&persistence),
        )
        .await;
        let (_, out_point) = fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(built_for)),
        )
        .await
        .expect("an unbound top-up lock builds and broadcasts");
        // The stale handle's generation; the wallet id now names another one,
        // which tracks the same lock unassigned (as one restored from the
        // store without its recipient would).
        let stale = {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&manager.wallet_id).unwrap();
            info.tracked_asset_locks
                .get_mut(&out_point)
                .unwrap()
                .identity_recipient = None;
            std::mem::replace(&mut info.generation, Arc::new(WalletGeneration::new()))
        };
        let stores_before = persistence.stored.lock().unwrap().len();

        let result = manager
            .assign_identity_top_up_recipient(&out_point, assigned, false, &stale)
            .await;

        assert!(
            matches!(result, Err(PlatformWalletError::WalletRemovedBeforeSend(_))),
            "{result:?}"
        );
        assert_eq!(
            manager.list_tracked_locks().await[0].identity_recipient,
            None
        );
        assert_eq!(persistence.stored.lock().unwrap().len(), stores_before);
    }

    /// A lock recording its recipient was stored durably before the
    /// broadcast, so a definite rejection removes its row durably too: a
    /// removal left queued could be lost, and the `Built` row would come back
    /// after a restart, resumable to the same recipient while its inputs fund
    /// the user's next attempt. A removal a buffered backend cannot flush is
    /// reported, not swallowed.
    #[tokio::test]
    async fn should_durably_remove_a_rejected_lock_that_records_its_recipient() {
        use crate::changeset::AssetLockRecipient;

        let identity = dpp::prelude::Identifier::from([0x56; 32]);
        let fund = |persistence: Arc<CapturingPersistence>| async move {
            let (manager, signer) = funded_asset_lock_manager_with_persistence(
                Arc::new(AlwaysRejectedBroadcaster),
                Arc::clone(&persistence),
            )
            .await;
            fund_unbound_top_up(
                &manager,
                &signer,
                Some(AssetLockRecipient::Identity(identity)),
            )
            .await
            .map(|_| ())
        };

        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let result = fund(Arc::clone(&persistence)).await;
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "the row is gone and the inputs free, so the rejection is retry-safe: {result:?}"
        );
        assert_eq!(persistence.removed_outpoints().len(), 1);
        assert_eq!(
            persistence
                .committed_stores
                .load(std::sync::atomic::Ordering::SeqCst),
            persistence.stored.lock().unwrap().len(),
            "the removal was committed at its store, not left queued"
        );

        // A buffered backend whose flush of the removal fails may not have
        // committed it. The row's flush is the first; the removal's is the
        // second.
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: false,
            fail_on_flush: Some(2),
            ..Default::default()
        });
        let result = fund(Arc::clone(&persistence)).await;
        // The row may survive, so the outcome is unknown rather than a
        // retry-safe persistence failure, and the outpoint is named.
        assert!(
            matches!(&result, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                if message.contains("could not be made durable")
                    && message.contains("may survive a restart")),
            "{result:?}"
        );
    }

    /// Every funding type removes a rejected lock's row durably before it
    /// reports the retry-safe rejection, not only the ones whose row was
    /// stored durably: a best-effort row that reached the store and whose
    /// removal is lost comes back after a restart, resumable, while the
    /// released inputs fund the next attempt. A buffered backend that cannot
    /// flush the removal gets the unknown outcome, and the inputs stay
    /// reserved.
    #[tokio::test]
    async fn should_durably_remove_a_rejected_lock_of_every_funding_type() {
        for funding_type in [
            AssetLockFundingType::IdentityRegistration,
            AssetLockFundingType::IdentityTopUp,
            AssetLockFundingType::AssetLockAddressTopUp,
        ] {
            for flush_fails in [false, true] {
                let persistence = Arc::new(CapturingPersistence {
                    commits_inline: false,
                    fail_flush: flush_fails,
                    ..Default::default()
                });
                let (manager, signer) = funded_asset_lock_manager_with_persistence(
                    Arc::new(AlwaysRejectedBroadcaster),
                    Arc::clone(&persistence),
                )
                .await;

                let result = manager
                    .broadcast_funded_asset_lock_tracked(
                        super::AssetLockBuildAmount::Exact(1_000_000),
                        &ASSET_LOCK_FUNDING_SOURCES,
                        0,
                        funding_type,
                        0,
                        None,
                        &signer,
                        None,
                        None,
                        |_| {},
                    )
                    .await;

                assert_eq!(
                    persistence.removed_outpoints().len(),
                    1,
                    "{funding_type:?}: the removal reached the store"
                );
                if flush_fails {
                    assert!(
                        matches!(&result, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                            if message.contains("could not be made durable")),
                        "{funding_type:?}: {result:?}"
                    );
                    let retry = manager
                        .broadcast_funded_asset_lock_tracked(
                            super::AssetLockBuildAmount::Exact(1_000_000),
                            &ASSET_LOCK_FUNDING_SOURCES,
                            0,
                            funding_type,
                            0,
                            None,
                            &signer,
                            None,
                            None,
                            |_| {},
                        )
                        .await;
                    assert!(
                        !matches!(retry, Err(PlatformWalletError::TransactionBroadcast(_))),
                        "{funding_type:?}: the inputs stay reserved, so no second lock is \
                         built from them and rejected: {retry:?}"
                    );
                } else {
                    assert!(
                        matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
                        "{funding_type:?}: the removal is durable, so the rejection is \
                         retry-safe: {result:?}"
                    );
                    assert!(
                        persistence
                            .flushes
                            .load(std::sync::atomic::Ordering::SeqCst)
                            >= 1,
                        "{funding_type:?}: the removal was flushed"
                    );
                }
            }
        }
    }

    /// On a backend whose store commits inline, a rejected lock's removal is
    /// durable once stored, so no flush is asked of it: a backend whose
    /// flushes would all fail must not keep the inputs fenced, since the row
    /// is gone and no observed spend could ever clear the fence. The call
    /// reports the rejection, and the inputs are free for the next attempt.
    #[tokio::test]
    async fn should_release_a_rejected_lock_removed_inline_despite_a_failed_flush() {
        use crate::changeset::AssetLockRecipient;

        let identity = dpp::prelude::Identifier::from([0x5b; 32]);
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            fail_flush: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(AlwaysRejectedBroadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let result = fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(identity)),
        )
        .await;

        assert!(
            matches!(&result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "{result:?}"
        );
        assert_eq!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst),
            0,
            "an inline store is durable: neither the row nor its removal is flushed"
        );
        let removed = persistence.removed_outpoints();
        assert_eq!(removed.len(), 1);
        let tx = stored_lock_transaction(&persistence, &removed[0]);
        assert_eq!(manager.generation.in_broadcast_conflict(&tx), None);
        assert!(manager.list_tracked_locks().await.is_empty());
    }

    /// The build-only API tracks nothing, so it builds no unbound top-up
    /// lock: one would later be rebuilt from the chain with no recipient.
    #[tokio::test]
    async fn should_refuse_an_unbound_top_up_lock_from_the_build_only_api() {
        let (manager, signer, _persistence) =
            funded_asset_lock_manager(Arc::new(CountingOkBroadcaster::default())).await;
        let result = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityTopUpNotBound,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::InvalidParameter(_))),
            "{result:?}"
        );
    }

    /// An assignment the backend did not commit is not published, and leaves
    /// nothing behind: a failed one (here transiently, on an inline backend
    /// and on a buffered one) does not pin the identity it tried, so the user
    /// can assign another, which is then committed and published.
    #[tokio::test]
    async fn should_leave_nothing_behind_after_a_failed_assignment() {
        let tried = dpp::prelude::Identifier::from([0x54; 32]);
        let chosen = dpp::prelude::Identifier::from([0x55; 32]);
        for commits_inline in [true, false] {
            let persistence = Arc::new(CapturingPersistence {
                records_identity_recipients: true,
                commits_inline,
                ..Default::default()
            });
            let (manager, out_point) =
                unassigned_top_up_lock(Arc::clone(&persistence), AssetLockStatus::Broadcast).await;
            persistence
                .fail_lock_store
                .store(true, std::sync::atomic::Ordering::SeqCst);
            persistence
                .lock_store_failure_is_transient
                .store(true, std::sync::atomic::Ordering::SeqCst);

            let result = assign_on_current_generation(&manager, &out_point, tried).await;

            assert!(
                matches!(result, Err(PlatformWalletError::Persistence(_))),
                "{result:?}"
            );
            let tracked = manager.list_tracked_locks().await;
            assert_eq!(tracked[0].identity_recipient, None, "not published");
            assert!(tracked[0].is_unassigned_identity_top_up());
            assert!(persistence.assigned.lock().unwrap().is_empty());

            persistence
                .fail_lock_store
                .store(false, std::sync::atomic::Ordering::SeqCst);
            assign_on_current_generation(&manager, &out_point, chosen)
                .await
                .expect("the failed identity pinned nothing");
            assert_eq!(
                manager.list_tracked_locks().await[0].identity_recipient,
                Some(chosen)
            );
        }
    }

    /// Rejects the first `remaining` broadcasts before anything is sent, then
    /// accepts.
    struct RejectFirstBroadcaster {
        remaining: std::sync::atomic::AtomicUsize,
    }

    #[async_trait::async_trait]
    impl TransactionBroadcaster for RejectFirstBroadcaster {
        async fn broadcast(
            &self,
            transaction: &dashcore::Transaction,
        ) -> Result<dashcore::Txid, crate::broadcaster::BroadcastError> {
            let rejected = self
                .remaining
                .fetch_update(
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                    |n| n.checked_sub(1),
                )
                .is_ok();
            if rejected {
                return Err(crate::broadcaster::BroadcastError::Rejected {
                    reason: "simulated pre-send rejection".to_string(),
                });
            }
            Ok(transaction.txid())
        }
    }

    /// Rejects every broadcast once let through, after reporting it was
    /// reached.
    struct GatedRejectBroadcaster {
        entered: Arc<tokio::sync::Barrier>,
        release: Arc<tokio::sync::Barrier>,
    }

    #[async_trait]
    impl TransactionBroadcaster for GatedRejectBroadcaster {
        async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.entered.wait().await;
            self.release.wait().await;
            Err(BroadcastError::Rejected {
                reason: "simulated pre-send rejection".to_string(),
            })
        }
    }

    /// A rejected lock's funding index is released only under the
    /// build→persist gate. A build that ran while the lock was out holds the
    /// gate until it has stored its funding-pool snapshot, which still shows
    /// the index used; released before that store, the index would be marked
    /// used again on disk.
    #[tokio::test]
    async fn should_release_a_rejected_lock_funding_index_only_under_the_build_persist_gate() {
        let entered = Arc::new(tokio::sync::Barrier::new(2));
        let release = Arc::new(tokio::sync::Barrier::new(2));
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(GatedRejectBroadcaster {
                entered: Arc::clone(&entered),
                release: Arc::clone(&release),
            }),
            Arc::clone(&persistence),
        )
        .await;
        let fund = manager.broadcast_funded_asset_lock_tracked(
            super::AssetLockBuildAmount::Exact(1_000_000),
            &ASSET_LOCK_FUNDING_SOURCES,
            0,
            AssetLockFundingType::IdentityTopUp,
            0,
            None,
            &signer,
            None,
            None,
            |_| {},
        );
        let other_build = async {
            entered.wait().await;
            // Another build holds the gate while the lock is rejected.
            let gate = manager.build_persist_serial.lock().await;
            release.wait().await;
            for _ in 0..50 {
                tokio::task::yield_now().await;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert!(
                persistence.released.lock().unwrap().is_empty(),
                "the index is not released while another build holds the gate"
            );
            drop(gate);
        };
        let (result, ()) = tokio::join!(fund, other_build);
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "{result:?}"
        );
        assert_eq!(
            persistence.released.lock().unwrap().len(),
            1,
            "the index is released once the gate is free"
        );
    }

    /// A lock Core rejects before sending, whose row is then durably gone,
    /// hands its funding index back, whatever its funding type, in memory and
    /// in the backend: however many are rejected, the next lock that goes out
    /// signs with the first funding address, so rejections cannot use up the
    /// pool, nor push a restarted wallet's next lock past the addresses a
    /// restore from the seed scans.
    #[tokio::test]
    async fn should_hand_back_the_funding_index_of_a_rejected_lock() {
        use crate::changeset::AssetLockRecipient;
        use key_wallet::bip32::ChildNumber;

        let identity = dpp::prelude::Identifier::from([0x46; 32]);
        for (funding_type, recipient) in [
            (AssetLockFundingType::IdentityTopUp, None),
            (
                AssetLockFundingType::IdentityTopUpNotBound,
                Some(AssetLockRecipient::Identity(identity)),
            ),
        ] {
            let persistence = Arc::new(CapturingPersistence {
                records_identity_recipients: true,
                commits_inline: true,
                ..Default::default()
            });
            let (manager, signer) = funded_asset_lock_manager_with_persistence(
                Arc::new(RejectFirstBroadcaster {
                    remaining: std::sync::atomic::AtomicUsize::new(8),
                }),
                Arc::clone(&persistence),
            )
            .await;
            let fund = || {
                manager.broadcast_funded_asset_lock_tracked(
                    super::AssetLockBuildAmount::Exact(1_000_000),
                    &ASSET_LOCK_FUNDING_SOURCES,
                    0,
                    funding_type,
                    0,
                    recipient,
                    &signer,
                    None,
                    None,
                    |_| {},
                )
            };
            for _ in 0..8 {
                let result = fund().await;
                assert!(
                    matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
                    "{funding_type:?}: {result:?}"
                );
            }
            let released = persistence.released.lock().unwrap().clone();
            assert_eq!(
                released.len(),
                8,
                "{funding_type:?}: every rejection clears the stored used mark too"
            );
            assert!(
                released.iter().all(|(account_type, _, index, _)| {
                    *account_type == funding_account_type(funding_type, 0) && *index == 0
                }),
                "{funding_type:?}: the release names the lock's funding address: {released:?}"
            );
            let (path, _) = fund()
                .await
                .expect("the rejected locks left the pool's addresses unused");
            assert_eq!(
                path.into_iter().last(),
                Some(&ChildNumber::Normal { index: 0 }),
                "{funding_type:?}: the lock signs with the first funding address: {path}"
            );
        }
    }

    /// Builds an unbound top-up lock for `built_for` on a backend that records
    /// identity recipients, then forgets the recipient, as a lock rebuilt from
    /// the chain has none, and sets its status to `status`.
    async fn unassigned_top_up_lock(
        persistence: Arc<CapturingPersistence>,
        status: AssetLockStatus,
    ) -> (Arc<AssetLockManager<CountingOkBroadcaster>>, OutPoint) {
        use crate::changeset::AssetLockRecipient;

        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(CountingOkBroadcaster::default()),
            Arc::clone(&persistence),
        )
        .await;
        let (_, out_point) = fund_unbound_top_up(
            &manager,
            &signer,
            Some(AssetLockRecipient::Identity(
                dpp::prelude::Identifier::from([0x59; 32]),
            )),
        )
        .await
        .expect("an unbound top-up lock builds and broadcasts");
        {
            let mut wm = manager.wallet_manager.write().await;
            let lock = wm
                .get_wallet_info_mut(&manager.wallet_id)
                .unwrap()
                .tracked_asset_locks
                .get_mut(&out_point)
                .unwrap();
            lock.identity_recipient = None;
            lock.status = status;
        }
        persistence.forget_recipient(&out_point);
        (manager, out_point)
    }

    /// An assignment the backend refuses for good (here a fatal store
    /// failure, as a store refusing a conflicting recipient reports, or a
    /// fatal flush on a buffered backend, which dropped what it held) is never
    /// published: the lock stays unassigned, and the refused identity cannot
    /// poison later snapshots or block the user's next choice.
    #[tokio::test]
    async fn should_not_publish_an_assignment_the_backend_refused() {
        let refused = dpp::prelude::Identifier::from([0x5a; 32]);
        let chosen = dpp::prelude::Identifier::from([0x5b; 32]);
        for buffered_flush_fails in [false, true] {
            let persistence = Arc::new(CapturingPersistence {
                records_identity_recipients: true,
                commits_inline: !buffered_flush_fails,
                ..Default::default()
            });
            let (manager, out_point) =
                unassigned_top_up_lock(Arc::clone(&persistence), AssetLockStatus::Broadcast).await;
            if buffered_flush_fails {
                persistence
                    .fail_flushes
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            } else {
                persistence
                    .fail_lock_store
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }

            let result = assign_on_current_generation(&manager, &out_point, refused).await;

            assert!(
                matches!(result, Err(PlatformWalletError::Persistence(_))),
                "{result:?}"
            );
            let tracked = manager.list_tracked_locks().await;
            assert_eq!(tracked[0].identity_recipient, None);
            assert!(tracked[0].is_unassigned_identity_top_up());

            persistence
                .fail_lock_store
                .store(false, std::sync::atomic::Ordering::SeqCst);
            persistence
                .fail_flushes
                .store(false, std::sync::atomic::Ordering::SeqCst);
            assign_on_current_generation(&manager, &out_point, chosen)
                .await
                .expect("the refused identity left nothing behind to conflict with");
            assert_eq!(
                manager.list_tracked_locks().await[0].identity_recipient,
                Some(chosen)
            );
        }
    }

    /// A lock the backend no longer stores (sync swept it, or its row was
    /// removed) is not written back by an assignment: the backend refuses it,
    /// so nothing is recorded or published, even while memory still tracks
    /// the lock.
    #[tokio::test]
    async fn should_refuse_an_assignment_for_a_lock_the_backend_no_longer_stores() {
        let identity = dpp::prelude::Identifier::from([0x5c; 32]);
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, out_point) =
            unassigned_top_up_lock(Arc::clone(&persistence), AssetLockStatus::Broadcast).await;
        persistence
            .stored
            .lock()
            .unwrap()
            .push(PlatformWalletChangeSet {
                asset_locks: Some(crate::changeset::AssetLockChangeSet {
                    removed: [out_point].into(),
                    ..Default::default()
                }),
                ..Default::default()
            });

        let result = assign_on_current_generation(&manager, &out_point, identity).await;

        assert!(
            matches!(&result, Err(PlatformWalletError::Persistence(message))
                if message.contains("no asset lock row")),
            "{result:?}"
        );
        assert!(persistence.assigned.lock().unwrap().is_empty());
        assert_eq!(
            manager.list_tracked_locks().await[0].identity_recipient,
            None
        );
    }

    /// A lock rebuilt from the chain may already have been used on Platform,
    /// which the wallet cannot tell: it is assigned only when the caller
    /// acknowledges that, and refused as of unknown consumption otherwise.
    /// Platform uses an outpoint once, so the acknowledgement cannot pay
    /// twice; it lets the user recover a lock that is genuinely unused.
    #[tokio::test]
    async fn should_assign_a_lock_rebuilt_from_the_chain_only_when_acknowledged() {
        let identity = dpp::prelude::Identifier::from([0x5f; 32]);
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, out_point) = unassigned_top_up_lock(
            Arc::clone(&persistence),
            AssetLockStatus::RecoveredFromChain,
        )
        .await;
        let generation = Arc::clone(
            &manager
                .wallet_manager
                .read()
                .await
                .get_wallet_info(&manager.wallet_id)
                .expect("wallet info")
                .generation,
        );

        let refused = manager
            .assign_identity_top_up_recipient(&out_point, identity, false, &generation)
            .await;
        assert!(
            matches!(refused, Err(PlatformWalletError::AssetLockConsumptionUnknown(op)) if op == out_point),
            "{refused:?}"
        );
        assert!(persistence.assigned.lock().unwrap().is_empty());

        manager
            .assign_identity_top_up_recipient(&out_point, identity, true, &generation)
            .await
            .expect("an acknowledged assignment of a recovered lock is recorded");
        assert_eq!(
            *persistence.assigned.lock().unwrap(),
            vec![(out_point, identity)]
        );
        assert_eq!(
            manager.list_tracked_locks().await[0].identity_recipient,
            Some(identity)
        );
    }

    /// On a backend that commits inline the assignment is a committed round
    /// of its own, so nothing is flushed for it: a backend whose flushes would
    /// all fail still assigns, and the assignment is published.
    #[tokio::test]
    async fn should_assign_without_a_flush_on_an_inline_backend() {
        let identity = dpp::prelude::Identifier::from([0x5d; 32]);
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, out_point) =
            unassigned_top_up_lock(Arc::clone(&persistence), AssetLockStatus::Broadcast).await;
        persistence
            .fail_flushes
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let flushes_before = persistence
            .flushes
            .load(std::sync::atomic::Ordering::SeqCst);

        let result = assign_on_current_generation(&manager, &out_point, identity).await;

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst),
            flushes_before,
            "an inline store needs no flush"
        );
        assert_eq!(
            manager.list_tracked_locks().await[0].identity_recipient,
            Some(identity)
        );
    }

    /// An assignment the backend has committed is published even when the
    /// call is dropped while it waits for the manager lock: memory would
    /// otherwise show no recipient where the store holds one, so a resume
    /// would refuse the lock and an assignment of another identity would pass
    /// the in-memory check only to be refused by the store.
    #[tokio::test]
    async fn should_publish_a_committed_assignment_whose_call_was_dropped() {
        let identity = dpp::prelude::Identifier::from([0x5f; 32]);
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, out_point) =
            unassigned_top_up_lock(Arc::clone(&persistence), AssetLockStatus::Broadcast).await;

        let held = manager.wallet_manager.read().await;
        let assignment = assign_on_current_generation(&manager, &out_point, identity);
        let timed_out = tokio::time::timeout(Duration::from_millis(200), assignment).await;
        assert!(
            timed_out.is_err(),
            "the publication waits for the held lock"
        );
        assert_eq!(
            *persistence.assigned.lock().unwrap(),
            vec![(out_point, identity)],
            "the backend committed before the call was dropped"
        );
        drop(held);

        let mut published = None;
        for _ in 0..100 {
            published = manager.list_tracked_locks().await[0].identity_recipient;
            if published.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(published, Some(identity));
    }

    /// A resume's authorization runs on the row it claims, under the guard it
    /// claims the dispatch window under: a row recreated at the outpoint for
    /// another recipient after an earlier read is seen as it is now, and a
    /// refusal sends nothing and leaves no claim to hold back rejection
    /// cleanup.
    #[tokio::test]
    async fn should_authorize_a_resume_on_the_row_it_claims() {
        let authorized_for = dpp::prelude::Identifier::from([0x62; 32]);
        let rebuilt_for = dpp::prelude::Identifier::from([0x63; 32]);
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, out_point) =
            unassigned_top_up_lock(Arc::clone(&persistence), AssetLockStatus::Built).await;
        manager
            .wallet_manager
            .write()
            .await
            .get_wallet_info_mut(&manager.wallet_id)
            .unwrap()
            .tracked_asset_locks
            .get_mut(&out_point)
            .unwrap()
            .identity_recipient = Some(rebuilt_for);
        let sends_before = manager.broadcaster.calls();

        let result = manager
            .resume_authorized_asset_lock(&out_point, Some(Duration::from_millis(50)), &|row| {
                match row.identity_recipient {
                    Some(recorded) if recorded == authorized_for => Ok(()),
                    recorded => Err(PlatformWalletError::AssetLockIdentityRecipientMismatch {
                        out_point,
                        recorded: recorded.unwrap_or_default(),
                        requested: authorized_for,
                    }),
                }
            })
            .await;

        assert!(
            matches!(
                result,
                Err(PlatformWalletError::AssetLockIdentityRecipientMismatch { recorded, .. })
                    if recorded == rebuilt_for
            ),
            "{result:?}"
        );
        assert_eq!(
            manager.broadcaster.calls(),
            sends_before,
            "nothing was sent"
        );
        assert!(
            !manager.resume_cleanup_excluded(&out_point),
            "no dispatch claim was left behind"
        );
    }

    /// A committed assignment is not published over another recipient the
    /// row in memory records: rejection cleanup can remove the row assigned
    /// and a build for another identity recreate the same outpoint, with its
    /// own recipient, before the publication takes the manager lock. That
    /// recipient stays, and the assignment reports the mismatch.
    #[tokio::test]
    async fn should_not_publish_an_assignment_over_a_recreated_rows_recipient() {
        let assigned = dpp::prelude::Identifier::from([0x60; 32]);
        let rebuilt_for = dpp::prelude::Identifier::from([0x61; 32]);
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, out_point) =
            unassigned_top_up_lock(Arc::clone(&persistence), AssetLockStatus::Built).await;
        let wallet_manager = Arc::clone(&manager.wallet_manager);
        let wallet_id = manager.wallet_id;
        *persistence.on_assign.lock().unwrap() = Some(Box::new(move || {
            // The row recreated for another identity, as its build tracks it.
            let mut wm = wallet_manager
                .try_write()
                .expect("the assignment holds no manager lock at the backend");
            wm.get_wallet_info_mut(&wallet_id)
                .unwrap()
                .tracked_asset_locks
                .get_mut(&out_point)
                .unwrap()
                .identity_recipient = Some(rebuilt_for);
        }));

        let result = assign_on_current_generation(&manager, &out_point, assigned).await;

        assert!(
            matches!(
                result,
                Err(PlatformWalletError::AssetLockIdentityRecipientMismatch {
                    recorded,
                    requested,
                    ..
                }) if recorded == rebuilt_for && requested == assigned
            ),
            "{result:?}"
        );
        assert_eq!(
            manager.list_tracked_locks().await[0].identity_recipient,
            Some(rebuilt_for)
        );
    }

    /// A recipient-less lock whose Platform consumption is unknown (rebuilt
    /// from a finalized transaction after a restore) is not listed as
    /// unassigned and cannot be assigned: Platform may have consumed it long
    /// ago, and nothing can check. A consumed lock is refused as consumed.
    /// Nothing is recorded or stored either way.
    #[tokio::test]
    async fn should_refuse_to_assign_a_lock_whose_consumption_is_unknown_or_known() {
        let identity = dpp::prelude::Identifier::from([0x5e; 32]);
        for (status, consumed) in [
            (AssetLockStatus::RecoveredFromChain, false),
            (AssetLockStatus::Consumed, true),
        ] {
            let persistence = Arc::new(CapturingPersistence {
                records_identity_recipients: true,
                commits_inline: true,
                ..Default::default()
            });
            let (manager, out_point) =
                unassigned_top_up_lock(Arc::clone(&persistence), status.clone()).await;
            let tracked = manager.list_tracked_locks().await;
            assert!(!tracked[0].is_unassigned_identity_top_up(), "{status:?}");
            let stores_before = persistence.stored.lock().unwrap().len();

            let result = assign_on_current_generation(&manager, &out_point, identity).await;

            if consumed {
                assert!(
                    matches!(result, Err(PlatformWalletError::AssetLockAlreadyConsumed(o))
                        if o == out_point),
                    "{result:?}"
                );
            } else {
                assert!(
                    matches!(result, Err(PlatformWalletError::AssetLockConsumptionUnknown(o))
                        if o == out_point),
                    "{result:?}"
                );
            }
            assert_eq!(
                manager.list_tracked_locks().await[0].identity_recipient,
                None
            );
            assert_eq!(persistence.stored.lock().unwrap().len(), stores_before);
        }
    }

    /// Only an unbound top-up lock takes an assigned recipient, and only on a
    /// backend that keeps it.
    #[tokio::test]
    async fn should_refuse_to_assign_a_recipient_a_lock_cannot_keep() {
        let identity = dpp::prelude::Identifier::from([0x53; 32]);
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let persistence = Arc::new(CapturingPersistence {
            records_identity_recipients: true,
            commits_inline: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        let (_, out_point) = manager
            .broadcast_funded_asset_lock_tracked(
                super::AssetLockBuildAmount::Exact(1_000_000),
                &ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockAddressTopUp,
                0,
                None,
                &signer,
                None,
                None,
                |_| {},
            )
            .await
            .expect("an address top-up lock builds and broadcasts");
        assert!(matches!(
            assign_on_current_generation(&manager, &out_point, identity).await,
            Err(PlatformWalletError::AssetLockFundingMismatch { .. })
        ));

        let (manager, _signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(CountingOkBroadcaster::default()),
            Arc::new(CapturingPersistence::default()),
        )
        .await;
        assert!(matches!(
            assign_on_current_generation(&manager, &out_point, identity).await,
            Err(PlatformWalletError::Persistence(_))
        ));
    }

    /// The funding resolver every fund-from-asset-lock flow calls hands the
    /// observer to the builder for an amount from the wallet balance: the
    /// rejected lock was reported once, with the outpoint whose row was
    /// removed again.
    #[tokio::test]
    async fn should_report_the_lock_built_for_a_wallet_balance_funding() {
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysRejectedBroadcaster)).await;
        let seen = Mutex::new(Vec::new());
        let observer = |out_point: OutPoint| seen.lock().unwrap().push(out_point);

        manager
            .resolve_funding_with_is_timeout_fallback(
                AssetLockFunding::FromWalletBalance {
                    amount_duffs: 1_000_000,
                    account_index: 0,
                },
                AssetLockFundingType::AssetLockAddressTopUp,
                0,
                None,
                &signer,
                Some(&observer),
            )
            .await
            .err()
            .expect("a rejected broadcast must fail the funding");

        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(persistence.removed_outpoints(), seen);
    }

    /// The same resolver hands the observer to the builder for a whole-account
    /// drain (the CoinJoin → shielded path).
    #[tokio::test]
    async fn should_report_the_lock_built_for_an_account_drain_funding() {
        let (manager, signer, persistence) =
            coinjoin_funded_asset_lock_manager(Arc::new(AlwaysRejectedBroadcaster)).await;
        let seen = Mutex::new(Vec::new());
        let observer = |out_point: OutPoint| seen.lock().unwrap().push(out_point);

        manager
            .resolve_funding_with_is_timeout_fallback(
                AssetLockFunding::DrainAccountBalance {
                    account: AssetLockFundingAccount::CoinJoin { account_index: 0 },
                    minimum_lock_duffs: Some(1),
                },
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                None,
                &signer,
                Some(&observer),
            )
            .await
            .err()
            .expect("a rejected broadcast must fail the funding");

        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(persistence.removed_outpoints(), seen);
    }

    /// A build abandoned before tracking never reaches the observer: there is
    /// no lock for the caller to own.
    #[tokio::test]
    async fn should_not_report_an_outpoint_for_an_abandoned_build() {
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer, _persistence) =
            coinjoin_funded_asset_lock_manager(Arc::clone(&broadcaster)).await;
        let seen = Mutex::new(Vec::new());
        let observer = |out_point: OutPoint| seen.lock().unwrap().push(out_point);

        manager
            .broadcast_funded_asset_lock_with_funding(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(u64::MAX),
                },
                &[AccountTypePreference::CoinJoin],
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
                Some(&observer),
            )
            .await
            .expect_err("undersized drain must be refused");

        assert!(seen.into_inner().unwrap().is_empty());
        assert_eq!(broadcaster.calls(), 0);
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
    async fn funded_asset_lock_manager_with_persistence<
        B: TransactionBroadcaster,
        P: PlatformWalletPersistence + 'static,
    >(
        broadcaster: Arc<B>,
        persistence: Arc<P>,
    ) -> (Arc<AssetLockManager<B>>, WalletSigner) {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            WalletPersister::new(wallet_id, persistence as Arc<dyn PlatformWalletPersistence>),
            Arc::clone(&generation),
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
    /// comes back `Rejected`. The advanced row means another attempt owns the
    /// transaction and may still deliver it, so the cleanup must keep it AND
    /// keep the funding reservation.
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
    /// a second asset lock beside a transaction another attempt may deliver.
    /// Only the unknown outcome describes what actually happened.
    #[tokio::test]
    async fn rejected_broadcast_racing_concurrent_resume_keeps_row_and_reservation() {
        let (wallet_manager, wallet_id, generation, signer) =
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
            Arc::clone(&generation),
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

    /// Simulates a resume attaching finality while the create path is waiting
    /// for its broadcast result.
    struct FinalizeDuringCreateBroadcast {
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        proof: Mutex<Option<dpp::prelude::AssetLockProof>>,
    }

    #[async_trait]
    impl TransactionBroadcaster for FinalizeDuringCreateBroadcast {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

            let mut wm = self.wallet_manager.write().await;
            let lock = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet present")
                .tracked_asset_locks
                .values_mut()
                .next()
                .expect("row tracked before broadcast");
            assert_eq!(lock.status, AssetLockStatus::Built);
            let proof = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: 1_234,
                out_point: lock.out_point,
            });
            lock.status = AssetLockStatus::ChainLocked;
            lock.proof = Some(proof.clone());
            *self.proof.lock().expect("staged proof mutex") = Some(proof);
            Ok(transaction.txid())
        }
    }

    /// A successful create broadcast must not overwrite a status and proof
    /// that a concurrent resume already advanced beyond `Built`.
    #[tokio::test]
    async fn create_broadcast_does_not_downgrade_a_concurrently_finalized_row() {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let broadcaster = Arc::new(FinalizeDuringCreateBroadcast {
            wallet_manager: Arc::clone(&wallet_manager),
            wallet_id,
            proof: Mutex::new(None),
        });
        let persistence = Arc::new(CapturingPersistence::default());
        let manager = AssetLockManager::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk")),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::clone(&broadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
            Arc::clone(&generation),
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
            .expect("accepted create broadcast");
        let staged_proof = broadcaster
            .proof
            .lock()
            .expect("staged proof mutex")
            .clone()
            .expect("proof staged during broadcast");

        {
            let wm = wallet_manager.read().await;
            let lock = wm
                .get_wallet_info(&wallet_id)
                .expect("wallet present")
                .tracked_asset_locks
                .get(&out_point)
                .expect("row stays tracked");
            assert_eq!(lock.status, AssetLockStatus::ChainLocked);
            assert_eq!(lock.proof.as_ref(), Some(&staged_proof));
        }

        let stored = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex");
        let persisted_downgrade = stored
            .iter()
            .filter_map(|changeset| changeset.asset_locks.as_ref())
            .filter_map(|changeset| changeset.asset_locks.get(&out_point))
            .any(|entry| entry.status == AssetLockStatus::Broadcast && entry.proof.is_some());
        assert!(
            !persisted_downgrade,
            "no persisted snapshot may combine Broadcast with an attached proof"
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

        let (wallet_manager, wallet_id, generation, signer) =
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
            Arc::clone(&generation),
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
        // The rejection proves the broadcaster was reached. Its row's removal
        // then cannot be flushed either, so the rejection is reported as the
        // unknown outcome rather than as retry-safe.
        assert!(
            matches!(&result, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                if message.contains("was rejected at broadcast")),
            "a registration build must reach the broadcaster despite the flush \
             failure (best-effort persistence), got {result:?}"
        );
    }

    /// The invitation gate does not trust a flush that succeeded over
    /// nothing: another flush of the wallet's records took the buffered
    /// funding pool first and failed for good, dropping it, so the funding
    /// index is on no disk and a restart would export the same voucher key
    /// again. The build is abandoned before anything is broadcast, and its
    /// inputs come back for the next one.
    #[tokio::test]
    async fn should_abort_an_invitation_whose_funding_index_another_flush_dropped() {
        let persistence = Arc::new(CapturingPersistence::default());
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        *persistence.overtaken_by.lock().unwrap() = Some(manager.persister.clone());

        let refused = manager
            .broadcast_funded_asset_lock(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(&refused, Err(PlatformWalletError::AssetLockTransaction(message))
                if message.contains("aborted before broadcast")),
            "{refused:?}"
        );
        assert!(
            persistence.overtaken_by.lock().unwrap().is_none(),
            "a flush was overtaken"
        );
        assert_eq!(broadcaster.calls(), 0, "nothing reached the wire");
        assert!(manager.list_tracked_locks().await.is_empty());

        manager
            .broadcast_funded_asset_lock(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await
            .expect("the abandoned build released its inputs, and this pool is durable");
        assert_eq!(broadcaster.calls(), 1);
    }

    /// A funding that pays a shielded recipient is not sent when another
    /// flush of the wallet's records took its row first and failed for good,
    /// though the row's own flush then succeeded, over nothing: the recipient
    /// is recorded nowhere else, so a lock recovered after a kill would pay
    /// whatever address a resume names. The build is abandoned before
    /// anything is broadcast, its row's removal stored behind it, and its
    /// inputs come back for the next one.
    #[tokio::test]
    async fn should_not_send_a_shielded_funding_whose_recipient_row_another_flush_dropped() {
        let persistence = Arc::new(CapturingPersistence {
            records_shielded_recipients: true,
            ..Default::default()
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        let fund = || {
            manager.broadcast_funded_asset_lock_tracked(
                super::AssetLockBuildAmount::Exact(1_000_000),
                &ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                Some(AssetLockRecipient::Shielded([7; 43])),
                &signer,
                None,
                None,
                |_| {},
            )
        };
        *persistence.overtaken_by.lock().unwrap() = Some(manager.persister.clone());

        let refused = fund().await;
        assert!(
            matches!(&refused, Err(PlatformWalletError::Persistence(message))
                if message.contains("shielded recipient was not made durable before dispatch")),
            "{refused:?}"
        );
        assert!(
            persistence.overtaken_by.lock().unwrap().is_none(),
            "a flush was overtaken"
        );
        assert_eq!(broadcaster.calls(), 0, "nothing reached the wire");
        assert!(manager.list_tracked_locks().await.is_empty());
        assert_eq!(
            persistence.removed_outpoints(),
            persistence.stored_outpoints(),
            "the abandoned row's removal is stored behind it"
        );
        assert!(persistence.rows_on_disk().is_empty());

        let (_, out_point) = fund()
            .await
            .expect("the abandoned build released its inputs, and this row is durable");
        assert_eq!(broadcaster.calls(), 1);
        assert_eq!(
            manager
                .list_tracked_locks()
                .await
                .iter()
                .map(|lock| (lock.out_point, lock.shielded_recipient))
                .collect::<Vec<_>>(),
            vec![(out_point, Some([7; 43]))]
        );
    }

    /// A funding that pays a shielded recipient and is abandoned leaves no
    /// row a restart would resume. Another flush of the wallet's records took
    /// the row, failed transiently and put it back, and the row's own flush
    /// then wrote it: the row is on disk, though not known to be, so the
    /// build is abandoned. Its removal is written before the inputs are
    /// released: only queued, it would be lost with the process, and the
    /// restart would send a funding reported as not made.
    #[tokio::test]
    async fn should_leave_no_row_on_disk_for_a_shielded_funding_abandoned_after_its_row_was_written(
    ) {
        let persistence = Arc::new(CapturingPersistence {
            records_shielded_recipients: true,
            overtaking_puts_back: std::sync::atomic::AtomicBool::new(true),
            ..Default::default()
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        *persistence.overtaken_by.lock().unwrap() = Some(manager.persister.clone());

        let refused = manager
            .broadcast_funded_asset_lock_tracked(
                super::AssetLockBuildAmount::Exact(1_000_000),
                &ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                Some(AssetLockRecipient::Shielded([7; 43])),
                &signer,
                None,
                None,
                |_| {},
            )
            .await;
        assert!(
            matches!(&refused, Err(PlatformWalletError::Persistence(message))
                if message.contains("shielded recipient was not made durable before dispatch")
                    && !message.contains("restart")),
            "{refused:?}"
        );
        assert!(
            persistence.overtaken_by.lock().unwrap().is_none(),
            "a flush was overtaken"
        );
        assert_eq!(broadcaster.calls(), 0, "nothing reached the wire");
        assert!(manager.list_tracked_locks().await.is_empty());
        assert_eq!(
            persistence.stored_outpoints().len(),
            1,
            "the row was stored, and its own flush wrote it"
        );
        assert!(
            persistence.rows_on_disk().is_empty(),
            "a restart finds no row to resume"
        );
    }

    /// An abandoned shielded funding whose row may be on disk, and whose
    /// removal is not known to be, is not reported as a failure: a restart
    /// may still resume and send it, so the outcome is unknown and the error
    /// says so. Its inputs stay reserved: released, the next funding would
    /// spend them, or be built from others beside a lock the restart sends.
    #[tokio::test]
    async fn should_keep_the_inputs_of_an_abandoned_shielded_funding_whose_removal_is_not_durable()
    {
        let persistence = Arc::new(CapturingPersistence {
            records_shielded_recipients: true,
            overtaking_puts_back: std::sync::atomic::AtomicBool::new(true),
            // The row's flush, the one that overtakes it, then the removal's.
            fail_on_flush: Some(3),
            ..Default::default()
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        let fund = || {
            manager.broadcast_funded_asset_lock_tracked(
                super::AssetLockBuildAmount::Exact(1_000_000),
                &ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                Some(AssetLockRecipient::Shielded([7; 43])),
                &signer,
                None,
                None,
                |_| {},
            )
        };
        *persistence.overtaken_by.lock().unwrap() = Some(manager.persister.clone());

        let refused = fund().await;
        assert!(
            matches!(&refused, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                if message.contains("a restart may still resume it")),
            "{refused:?}"
        );
        assert_eq!(broadcaster.calls(), 0, "nothing reached the wire");
        assert!(manager.list_tracked_locks().await.is_empty());
        assert_eq!(
            persistence.rows_on_disk().len(),
            1,
            "the row is on disk, its removal is not"
        );

        // The unsent transaction's inputs stay fenced as after a send that
        // may have gone out, so a sweep of the reservation does not free them.
        let unsent = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex")
            .iter()
            .filter_map(|cs| cs.asset_locks.as_ref())
            .flat_map(|locks| locks.asset_locks.values())
            .map(|row| row.transaction.clone())
            .next()
            .expect("the row was stored");
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            assert!(info.generation.in_broadcast_conflict(&unsent).is_some());
        }

        // The one coin the wallet holds stays reserved too, so the next
        // funding finds nothing to select.
        let rebuild = fund().await;
        assert!(
            matches!(
                rebuild,
                Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. })
            ),
            "{rebuild:?}"
        );
        assert_eq!(broadcaster.calls(), 0);
    }

    /// The same where the row waits in the buffer, its own flush having
    /// failed transiently, and the backend then refuses its removal: the next
    /// flush writes the row with nothing behind it, and a restart resumes it.
    /// So this outcome is unknown too, and the inputs stay reserved.
    #[tokio::test]
    async fn should_keep_the_inputs_of_an_abandoned_shielded_funding_whose_removal_was_refused() {
        let persistence = Arc::new(CapturingPersistence {
            records_shielded_recipients: true,
            flush_keeps_buffer: std::sync::atomic::AtomicBool::new(true),
            fail_remove_store: true,
            ..Default::default()
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        let fund = || {
            manager.broadcast_funded_asset_lock_tracked(
                super::AssetLockBuildAmount::Exact(1_000_000),
                &ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                Some(AssetLockRecipient::Shielded([7; 43])),
                &signer,
                None,
                None,
                |_| {},
            )
        };

        let refused = fund().await;
        assert!(
            matches!(&refused, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                if message.contains("a restart may still resume it")),
            "{refused:?}"
        );
        assert_eq!(broadcaster.calls(), 0, "nothing reached the wire");
        assert!(manager.list_tracked_locks().await.is_empty());
        assert!(persistence.removed_outpoints().is_empty());

        persistence
            .flush_keeps_buffer
            .store(false, std::sync::atomic::Ordering::SeqCst);
        manager.persister.flush().expect("the backend recovered");
        assert_eq!(
            persistence.rows_on_disk().len(),
            1,
            "the later flush wrote the row, and nothing removes it"
        );

        let rebuild = fund().await;
        assert!(
            matches!(
                rebuild,
                Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. })
            ),
            "{rebuild:?}"
        );
        assert_eq!(broadcaster.calls(), 0);
    }

    /// Funds a shielded recipient over a backend set up to fail, expecting
    /// the build to be abandoned with its outcome unknown and its inputs
    /// kept: nothing is sent or tracked, and the next funding finds the one
    /// coin the wallet holds still reserved.
    async fn expect_an_abandoned_shielded_funding_to_keep_its_inputs(
        manager: &AssetLockManager<CountingOkBroadcaster>,
        signer: &WalletSigner,
        broadcaster: &CountingOkBroadcaster,
    ) {
        let fund = || {
            manager.broadcast_funded_asset_lock_tracked(
                super::AssetLockBuildAmount::Exact(1_000_000),
                &ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                Some(AssetLockRecipient::Shielded([7; 43])),
                signer,
                None,
                None,
                |_| {},
            )
        };
        let refused = fund().await;
        assert!(
            matches!(&refused, Err(PlatformWalletError::TransactionBroadcastUnconfirmed(message))
                if message.contains("a restart may still resume it")),
            "{refused:?}"
        );
        assert!(manager.list_tracked_locks().await.is_empty());
        let rebuild = fund().await;
        assert!(
            matches!(
                rebuild,
                Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. })
            ),
            "{rebuild:?}"
        );
        assert_eq!(broadcaster.calls(), 0, "nothing reached the wire");
    }

    /// The same on a host that does not roll a failed round back: the row is
    /// written before a later part of its store fails, so the backend
    /// refuses the store with the row on disk, and refuses the removal too.
    /// A refused store is no proof that nothing was written.
    #[tokio::test]
    async fn should_keep_the_inputs_of_an_abandoned_shielded_funding_whose_refused_store_wrote_its_row(
    ) {
        let persistence = Arc::new(CapturingPersistence {
            records_shielded_recipients: true,
            commits_inline: true,
            lock_store_fails_after_writing: true,
            fail_remove_store: true,
            ..Default::default()
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;

        expect_an_abandoned_shielded_funding_to_keep_its_inputs(&manager, &signer, &broadcaster)
            .await;
        assert_eq!(
            persistence.rows_on_disk().len(),
            1,
            "the refused store left its row on disk"
        );
    }

    /// The same where another flush of the wallet's records wrote the row
    /// just before the row's own flush failed, and the removal's flush fails
    /// too: neither answer says the row is on disk, and it is.
    #[tokio::test]
    async fn should_keep_the_inputs_of_an_abandoned_shielded_funding_whose_row_another_flush_wrote()
    {
        let persistence = Arc::new(CapturingPersistence {
            records_shielded_recipients: true,
            overtaking_commits: std::sync::atomic::AtomicBool::new(true),
            // The row's flush, the one that overtakes it, then the removal's.
            fail_on_flush: Some(3),
            ..Default::default()
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        *persistence.overtaken_by.lock().unwrap() = Some(manager.persister.clone());

        expect_an_abandoned_shielded_funding_to_keep_its_inputs(&manager, &signer, &broadcaster)
            .await;
        assert!(
            persistence.overtaken_by.lock().unwrap().is_none(),
            "a flush was overtaken"
        );
        assert_eq!(
            persistence.rows_on_disk().len(),
            1,
            "the other flush wrote the row, and nothing wrote its removal"
        );
    }

    /// The same where a flush still in progress took the row: the removal
    /// stored behind it would be written first, and the row after it, by a
    /// flush that then returns without a failure for anything to see. The
    /// removal is not answered for while that flush is in progress, so the
    /// inputs stay reserved, and the flush writes the row in the end.
    #[tokio::test]
    async fn should_keep_the_inputs_of_an_abandoned_shielded_funding_whose_row_a_flush_in_progress_took(
    ) {
        let persistence = Arc::new(CapturingPersistence {
            records_shielded_recipients: true,
            ..Default::default()
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        *persistence.stalled_by.lock().unwrap() = Some(manager.persister.clone());

        expect_an_abandoned_shielded_funding_to_keep_its_inputs(&manager, &signer, &broadcaster)
            .await;
        assert!(
            persistence.stalled_by.lock().unwrap().is_none(),
            "a flush took the row and stalled"
        );
        assert!(
            persistence.rows_on_disk().is_empty(),
            "nothing is on disk while that flush is in progress"
        );

        persistence.let_stalled_flush_write();
        assert_eq!(
            persistence.rows_on_disk().len(),
            1,
            "the flush that took the row wrote it in the end"
        );
    }

    /// The broadcast half returns when the broadcaster accepts the transaction:
    /// the tracked row is `Broadcast` (recoverable/resumable) and the invitation
    /// funding pool was persisted AND flushed — all BEFORE any proof wait (the
    /// test completing at all proves no SPV wait ran), so a caller can durably
    /// record its own bookkeeping for the funded lock between the broadcast and
    /// the proof wait.
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
                None,
                &signer,
                None,
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
                    None,
                    &signer,
                    None,
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
        let persistence = Arc::new(CapturingPersistence::for_wallet(&wallet_manager, wallet_id));
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let generation = crate::test_support::wallet_generation(&wallet_manager, &wallet_id);
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
            generation,
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

    /// Broadcaster double that sequences the initial build's definite
    /// rejection against a resume parked in the transport-readiness wait.
    ///
    /// The first `broadcast` call is the build's: it announces that the
    /// `Built` row is tracked and the send is in flight, then waits for the
    /// test before returning the rejection that triggers the cleanup. Every
    /// later call is a resume's and receives the verdict selected by the
    /// test. `wait_until_ready` is the resume's park: it models a transport
    /// that comes up exactly when the test says so, which is the only way to
    /// hold a resume inside its dispatch window for the whole of the cleanup.
    struct RejectTheBuildAndParkTheResume {
        calls: std::sync::atomic::AtomicUsize,
        /// Every transaction handed to the broadcaster with the verdict it
        /// drew, in dispatch order — so a test can tell an attempt that was
        /// refused before dispatch from one that actually went out.
        dispatched: Mutex<Vec<(Txid, bool)>>,
        at_broadcast: Arc<tokio::sync::Barrier>,
        reject_gate: Arc<tokio::sync::Barrier>,
        resume_parked: Arc<tokio::sync::Barrier>,
        transport_gate: Arc<tokio::sync::Barrier>,
        resume_sent: Option<Arc<tokio::sync::Barrier>>,
        accept_resume: bool,
    }

    impl RejectTheBuildAndParkTheResume {
        fn dispatched(&self) -> Vec<(Txid, bool)> {
            self.dispatched.lock().expect("dispatch log mutex").clone()
        }
    }

    #[async_trait]
    impl TransactionBroadcaster for RejectTheBuildAndParkTheResume {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let accepted = call > 0 && self.accept_resume;
            self.dispatched
                .lock()
                .expect("dispatch log mutex")
                .push((transaction.txid(), accepted));
            if call == 0 {
                self.at_broadcast.wait().await;
                self.reject_gate.wait().await;
                return Err(BroadcastError::Rejected {
                    reason: "bad-txns-inputs-missingorspent".to_string(),
                });
            }
            if !accepted {
                return Err(BroadcastError::Rejected {
                    reason: "simulated pre-dispatch rejection".to_string(),
                });
            }
            if let Some(resume_sent) = &self.resume_sent {
                resume_sent.wait().await;
            }
            Ok(transaction.txid())
        }

        async fn wait_until_ready(&self, _timeout: Duration) -> bool {
            self.resume_parked.wait().await;
            self.transport_gate.wait().await;
            true
        }
    }

    /// Persistence double that supplies an InstantSend record from durable
    /// storage and pauses on the validation lookup made only after the resume
    /// has turned that record into a proof.
    struct ObservableLocalProofPersistence {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
        record: Mutex<Option<TransactionRecord>>,
        lookups: std::sync::atomic::AtomicUsize,
        proof_observed: Notify,
        proof_gate: std::sync::Barrier,
    }

    impl ObservableLocalProofPersistence {
        fn new() -> Self {
            Self {
                stored: Mutex::new(Vec::new()),
                record: Mutex::new(None),
                lookups: std::sync::atomic::AtomicUsize::new(0),
                proof_observed: Notify::new(),
                proof_gate: std::sync::Barrier::new(2),
            }
        }

        fn removed_outpoints(&self) -> Vec<OutPoint> {
            self.stored
                .lock()
                .expect("observable persistence mutex")
                .iter()
                .filter_map(|cs| cs.asset_locks.as_ref())
                .flat_map(|locks| locks.removed.iter().copied())
                .collect()
        }
    }

    impl PlatformWalletPersistence for ObservableLocalProofPersistence {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stored
                .lock()
                .expect("observable persistence mutex")
                .push(changeset);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }

        fn get_core_tx_record(
            &self,
            _wallet_id: WalletId,
            _txid: &Txid,
        ) -> Result<Option<TransactionRecord>, PersistenceError> {
            let lookup = self
                .lookups
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if lookup == 1 {
                self.proof_observed.notify_one();
                tokio::task::block_in_place(|| self.proof_gate.wait());
            }
            Ok(self.record.lock().expect("observable record mutex").clone())
        }
    }

    fn instant_send_record(transaction: Transaction) -> TransactionRecord {
        TransactionRecord::new(
            transaction,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InstantSend(
                dashcore::ephemerealdata::instant_lock::InstantLock::default(),
            ),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    /// The single tracked asset-lock outpoint on the fixture wallet.
    async fn the_only_tracked_outpoint(
        wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
    ) -> OutPoint {
        let wm = wallet_manager.read().await;
        let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
        assert_eq!(
            info.tracked_asset_locks.len(),
            1,
            "the build must have tracked its Built row before broadcasting"
        );
        *info
            .tracked_asset_locks
            .keys()
            .next()
            .expect("one tracked lock")
    }

    /// THE FENCE VIOLATION: a definite rejection must not release an asset
    /// lock's inputs while a resume is still on its way to broadcasting that
    /// very transaction.
    ///
    /// A resume reads the tracked row, its transaction and its status out of
    /// the map under one read guard, and only then waits for the broadcast
    /// transport, sends, and records the send by advancing the row. The
    /// initial build's cleanup runs against a snapshot that is already gone:
    /// its removal guard asks whether the row is still `Built`, which a
    /// resume that has not reached its status advance yet still is. So the
    /// cleanup used to remove the row and release both the funding
    /// reservation and the in-broadcast fence in the middle of the resume's
    /// dispatch window — and the resume then put the original transaction on
    /// the wire from inputs the definite-rejection contract had just told the
    /// host were free to rebuild from. The result is a live asset lock
    /// beside a replacement spending the same UTXOs.
    ///
    /// The interleaving is driven, not hoped for: the build suspends inside
    /// `broadcast` with its row tracked, the resume is held inside the
    /// transport-readiness wait, and only then is the rejection released. The
    /// resume's send is deliberately allowed to succeed afterwards — the fix
    /// is not to suppress it but to make sure nothing released its inputs
    /// first, which is what the rebuild refusal below asserts.
    ///
    /// The resume names a one-millisecond budget purely so its downstream
    /// proof wait cannot outlive the test; the transport wait is over long
    /// before that, since it ends on the test's own signal.
    #[tokio::test]
    async fn a_rejection_cleanup_cannot_release_inputs_under_a_parked_resume() {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let broadcaster = Arc::new(RejectTheBuildAndParkTheResume {
            calls: std::sync::atomic::AtomicUsize::new(0),
            dispatched: Mutex::new(Vec::new()),
            at_broadcast: Arc::new(tokio::sync::Barrier::new(2)),
            reject_gate: Arc::new(tokio::sync::Barrier::new(2)),
            resume_parked: Arc::new(tokio::sync::Barrier::new(2)),
            transport_gate: Arc::new(tokio::sync::Barrier::new(2)),
            resume_sent: None,
            accept_resume: true,
        });
        let (manager, persistence) = asset_lock_manager_over(
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&broadcaster),
        );

        let build = async {
            manager
                .create_funded_asset_lock_proof(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    0,
                    &signer,
                )
                .await
        };

        // Every rendezvous is bounded so that a regression which stops the
        // build or the resume from reaching its half of the interleaving
        // fails the test with the step it never got to, instead of hanging
        // the suite on a barrier nobody will arrive at.
        let reach = |step: &'static str, wait| async move {
            tokio::time::timeout(Duration::from_secs(30), wait)
                .await
                .unwrap_or_else(|_| panic!("the interleaving never reached: {step}"));
        };

        let coordinator = async {
            // The build is inside `broadcast`: signed, tracked at `Built`,
            // nothing decided yet.
            reach("the build's broadcast", broadcaster.at_broadcast.wait()).await;
            let out_point = the_only_tracked_outpoint(&wallet_manager, wallet_id).await;

            // A resume of that same row snapshots it and parks in the
            // transport wait, exactly as the launch catch-up does while the
            // SPV client is still starting.
            let resume = tokio::spawn({
                let manager = Arc::clone(&manager);
                async move {
                    manager
                        .resume_asset_lock(&out_point, Some(Duration::from_millis(1)))
                        .await
                }
            });
            reach(
                "the resume's transport wait",
                broadcaster.resume_parked.wait(),
            )
            .await;

            // Only now does the build's rejection — and its whole cleanup —
            // run.
            reach("the build's rejection", broadcaster.reject_gate.wait()).await;
            (out_point, resume)
        };

        let (build_result, (out_point, resume)) = tokio::join!(build, coordinator);

        let Err(PlatformWalletError::TransactionBroadcastUnconfirmed(reason)) = &build_result
        else {
            panic!(
                "with a resume holding the dispatch window the cleanup cannot run, so \
                 the definite-rejection contract — row gone, inputs released, rebuild \
                 safe — does not hold and must not be reported: {build_result:?}"
            );
        };
        assert!(
            reason.contains("resume"),
            "the unknown outcome must name the concurrent resume that kept the row, \
             so the verdict is not read as an ordinary ambiguous broadcast: {reason}"
        );
        {
            let wm = wallet_manager.read().await;
            let (_, info) = wm.get_wallet_and_info(&wallet_id).expect("wallet present");
            assert_eq!(
                info.tracked_asset_locks
                    .get(&out_point)
                    .map(|lock| lock.status.clone()),
                Some(AssetLockStatus::Built),
                "the row the parked resume is about to broadcast must survive the cleanup"
            );
        }
        assert!(
            persistence.removed_outpoints().is_empty(),
            "and no persisted-row deletion may be queued for it either, got {:?}",
            persistence.removed_outpoints()
        );

        // The assertion the whole test exists for: at this point the cleanup
        // has finished and the resume has not sent yet. If the inputs were
        // reusable here, the send that follows would land beside whatever the
        // host rebuilt from them.
        let rebuild = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        match rebuild {
            Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. }) => {}
            other => panic!(
                "the rejection cleanup released the inputs of a transaction a parked \
                 resume is still about to broadcast — a rebuild from them creates a \
                 second asset lock beside a live one: {other:?}"
            ),
        }

        // Bring the transport up and let the parked resume do exactly what it
        // was always going to do.
        tokio::time::timeout(Duration::from_secs(30), broadcaster.transport_gate.wait())
            .await
            .expect("the resume must still be parked in the transport wait");
        let _ = resume.await.expect("resume task");
        let dispatched = broadcaster.dispatched();
        assert_eq!(
            dispatched.len(),
            2,
            "the test proves nothing unless the parked resume really did reach \
             the broadcaster after the cleanup ran: {dispatched:?}"
        );
        assert!(
            dispatched[1].1,
            "and that send has to be one the transport accepted — a second \
             refusal would leave nothing on the wire and the interleaving \
             would be harmless for the wrong reason: {dispatched:?}"
        );
        assert_eq!(
            dispatched[0].0, dispatched[1].0,
            "what went out is the very transaction the cleanup judged rejected"
        );
    }

    /// A resume that proves its own attempt never left the device must
    /// release its cleanup exclusion. If the initial build's rejection is
    /// still waiting, that cleanup can then remove the dead row and release
    /// its inputs under the ordinary definite-rejection contract.
    #[tokio::test]
    async fn should_release_cleanup_exclusion_after_a_predispatch_resume_rejection() {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let broadcaster = Arc::new(RejectTheBuildAndParkTheResume {
            calls: std::sync::atomic::AtomicUsize::new(0),
            dispatched: Mutex::new(Vec::new()),
            at_broadcast: Arc::new(tokio::sync::Barrier::new(2)),
            reject_gate: Arc::new(tokio::sync::Barrier::new(2)),
            resume_parked: Arc::new(tokio::sync::Barrier::new(2)),
            transport_gate: Arc::new(tokio::sync::Barrier::new(2)),
            resume_sent: None,
            accept_resume: false,
        });
        let (manager, persistence) = asset_lock_manager_over(
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&broadcaster),
        );

        let build = async {
            manager
                .create_funded_asset_lock_proof(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    0,
                    &signer,
                )
                .await
        };
        let coordinator = async {
            tokio::time::timeout(Duration::from_secs(30), broadcaster.at_broadcast.wait())
                .await
                .expect("the build must reach its broadcast");
            let out_point = the_only_tracked_outpoint(&wallet_manager, wallet_id).await;
            let resume = tokio::spawn({
                let manager = Arc::clone(&manager);
                async move {
                    manager
                        .resume_asset_lock(&out_point, Some(Duration::from_millis(1)))
                        .await
                }
            });
            tokio::time::timeout(Duration::from_secs(30), broadcaster.resume_parked.wait())
                .await
                .expect("the resume must reach the transport wait");
            tokio::time::timeout(Duration::from_secs(30), broadcaster.transport_gate.wait())
                .await
                .expect("the resume transport must be released");
            let resume_result = tokio::time::timeout(Duration::from_secs(30), resume)
                .await
                .expect("the rejected resume must finish")
                .expect("the resume task must not panic");
            assert!(
                matches!(
                    resume_result,
                    Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
                ),
                "the resume must report its pre-dispatch rejection as an unknown original \
                 outcome: {resume_result:?}"
            );

            tokio::time::timeout(Duration::from_secs(30), broadcaster.reject_gate.wait())
                .await
                .expect("the initial rejection must be released after the resume exits");
            out_point
        };

        let (build_result, out_point) = tokio::join!(build, coordinator);
        assert!(
            matches!(
                build_result,
                Err(PlatformWalletError::TransactionBroadcast(_))
            ),
            "once the proven-predispatch resume releases its claim, the initial rejection can \
             restore the definite-rejection contract: {build_result:?}"
        );
        assert!(
            persistence.removed_outpoints().contains(&out_point),
            "the rejected Built row must be queued for deletion"
        );
        assert!(
            !wallet_manager
                .read()
                .await
                .get_wallet_and_info(&wallet_id)
                .expect("wallet present")
                .1
                .tracked_asset_locks
                .contains_key(&out_point),
            "the rejected Built row must be removed in memory"
        );
        assert!(
            manager
                .build_asset_lock_transaction(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    0,
                    &signer,
                )
                .await
                .is_ok(),
            "cleanup must make the definitely rejected transaction's inputs reusable"
        );
        assert_eq!(
            broadcaster
                .dispatched()
                .iter()
                .map(|(_, sent)| *sent)
                .collect::<Vec<_>>(),
            vec![false, false],
            "both attempts must be proven pre-dispatch for cleanup to be safe"
        );
    }

    /// Once a resumed send is observable, cancelling the future cannot make
    /// the original build's cleanup treat that transaction as never sent.
    #[tokio::test]
    async fn cancelling_after_an_observable_resume_send_keeps_cleanup_excluded() {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let broadcaster = Arc::new(RejectTheBuildAndParkTheResume {
            calls: std::sync::atomic::AtomicUsize::new(0),
            dispatched: Mutex::new(Vec::new()),
            at_broadcast: Arc::new(tokio::sync::Barrier::new(2)),
            reject_gate: Arc::new(tokio::sync::Barrier::new(2)),
            resume_parked: Arc::new(tokio::sync::Barrier::new(2)),
            transport_gate: Arc::new(tokio::sync::Barrier::new(2)),
            resume_sent: Some(Arc::new(tokio::sync::Barrier::new(2))),
            accept_resume: true,
        });
        let (manager, persistence) = asset_lock_manager_over(
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&broadcaster),
        );

        let build = async {
            manager
                .create_funded_asset_lock_proof(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    0,
                    &signer,
                )
                .await
        };
        let coordinator = async {
            tokio::time::timeout(Duration::from_secs(30), broadcaster.at_broadcast.wait())
                .await
                .expect("the build must reach its broadcast");
            let out_point = the_only_tracked_outpoint(&wallet_manager, wallet_id).await;
            let resume = tokio::spawn({
                let manager = Arc::clone(&manager);
                async move {
                    manager
                        .resume_asset_lock(&out_point, Some(Duration::from_millis(1)))
                        .await
                }
            });
            tokio::time::timeout(Duration::from_secs(30), broadcaster.resume_parked.wait())
                .await
                .expect("the resume must reach the transport wait");

            let wallet_write = wallet_manager.write().await;
            tokio::time::timeout(Duration::from_secs(30), broadcaster.transport_gate.wait())
                .await
                .expect("the resume transport must be released");
            tokio::time::timeout(
                Duration::from_secs(30),
                broadcaster
                    .resume_sent
                    .as_ref()
                    .expect("resume send gate")
                    .wait(),
            )
            .await
            .expect("the resumed send must become observable");
            resume.abort();
            assert!(
                resume
                    .await
                    .expect_err("the resume must be cancelled")
                    .is_cancelled(),
                "the resume must be aborted before it records Broadcast"
            );
            drop(wallet_write);
            tokio::time::timeout(Duration::from_secs(30), broadcaster.reject_gate.wait())
                .await
                .expect("the build rejection must be released");
            out_point
        };

        let (build_result, out_point) = tokio::join!(build, coordinator);
        assert!(
            matches!(
                build_result,
                Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
            ),
            "a cancellation after an observable send is an unknown outcome; cleanup must not \
             report that rebuilding is safe: {build_result:?}"
        );
        assert!(
            persistence.removed_outpoints().is_empty(),
            "the possibly-live transaction must not be queued for deletion"
        );
        assert_eq!(
            wallet_manager
                .read()
                .await
                .get_wallet_and_info(&wallet_id)
                .expect("wallet present")
                .1
                .tracked_asset_locks
                .get(&out_point)
                .map(|lock| lock.status.clone()),
            Some(AssetLockStatus::Built),
            "the cancelled resume did not record Broadcast, so the sticky exclusion must keep \
             the Built row"
        );
        assert!(
            matches!(
                manager
                    .build_asset_lock_transaction(
                        1_000_000,
                        0,
                        AssetLockFundingType::IdentityRegistration,
                        0,
                        &signer,
                    )
                    .await,
                Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. })
            ),
            "the cleanup must not release inputs of a transaction whose send was observable"
        );
    }

    /// A proof read from durable local state is equally conclusive: cancelling
    /// before the row can record it must keep cleanup from releasing its spend.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelling_after_observing_local_proof_keeps_cleanup_excluded() {
        let broadcaster = Arc::new(RejectTheBuildAndParkTheResume {
            calls: std::sync::atomic::AtomicUsize::new(0),
            dispatched: Mutex::new(Vec::new()),
            at_broadcast: Arc::new(tokio::sync::Barrier::new(2)),
            reject_gate: Arc::new(tokio::sync::Barrier::new(2)),
            resume_parked: Arc::new(tokio::sync::Barrier::new(2)),
            transport_gate: Arc::new(tokio::sync::Barrier::new(2)),
            resume_sent: None,
            accept_resume: true,
        });
        let persistence = Arc::new(ObservableLocalProofPersistence::new());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::clone(&broadcaster),
            Arc::clone(&persistence),
        )
        .await;
        let wallet_manager = Arc::clone(&manager.wallet_manager);
        let wallet_id = manager.wallet_id;

        let build = async {
            manager
                .create_funded_asset_lock_proof(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    0,
                    &signer,
                )
                .await
        };
        let coordinator = async {
            tokio::time::timeout(Duration::from_secs(30), broadcaster.at_broadcast.wait())
                .await
                .expect("the build must reach its broadcast");
            let (out_point, transaction) = {
                let mut wm = wallet_manager.write().await;
                let (_, info) = wm
                    .get_wallet_and_info_mut(&wallet_id)
                    .expect("wallet present");
                let lock = info
                    .tracked_asset_locks
                    .values()
                    .next()
                    .expect("one tracked lock");
                let out_point = lock.out_point;
                let transaction = lock.transaction.clone();
                info.core_wallet
                    .accounts
                    .standard_bip44_accounts
                    .get_mut(&0)
                    .expect("BIP44 account 0")
                    .transactions_mut()
                    .remove(&transaction.txid());
                (out_point, transaction)
            };
            *persistence.record.lock().expect("observable record mutex") =
                Some(instant_send_record(transaction));

            let resume = tokio::spawn({
                let manager = Arc::clone(&manager);
                async move { manager.resume_asset_lock(&out_point, None).await }
            });
            tokio::time::timeout(
                Duration::from_secs(30),
                persistence.proof_observed.notified(),
            )
            .await
            .expect("the resume must validate the local proof");
            let wallet_write = wallet_manager.write().await;
            tokio::task::block_in_place(|| persistence.proof_gate.wait());
            resume.abort();
            assert!(
                resume
                    .await
                    .expect_err("the resume must be cancelled")
                    .is_cancelled(),
                "the resume must be aborted before it records the proof"
            );
            drop(wallet_write);
            tokio::time::timeout(Duration::from_secs(30), broadcaster.reject_gate.wait())
                .await
                .expect("the build rejection must be released");
            out_point
        };

        let (build_result, out_point) = tokio::join!(build, coordinator);
        assert!(
            matches!(
                build_result,
                Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
            ),
            "a cancellation after observing local finality is an unknown outcome; cleanup must \
             not report that rebuilding is safe: {build_result:?}"
        );
        assert!(
            persistence.removed_outpoints().is_empty(),
            "the locally-final transaction must not be queued for deletion"
        );
        assert_eq!(
            wallet_manager
                .read()
                .await
                .get_wallet_and_info(&wallet_id)
                .expect("wallet present")
                .1
                .tracked_asset_locks
                .get(&out_point)
                .map(|lock| lock.status.clone()),
            Some(AssetLockStatus::Built),
            "the cancelled resume did not record its proof, so the sticky exclusion must keep \
             the Built row"
        );
        assert!(
            matches!(
                manager
                    .build_asset_lock_transaction(
                        1_000_000,
                        0,
                        AssetLockFundingType::IdentityRegistration,
                        0,
                        &signer,
                    )
                    .await,
                Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. })
            ),
            "the cleanup must not release inputs spent by a locally-final transaction"
        );
    }

    /// A claim that has not crossed the side-effect boundary is an RAII hold.
    ///
    /// `untrack_asset_lock` must refuse only while a resume is actually
    /// inside its pre-dispatch window, and must remove the row the moment the
    /// last releasable claim goes. Two claims are taken so the count itself is
    /// pinned: releasing one of them must not be enough.
    #[tokio::test]
    async fn a_released_dispatch_claim_lets_the_cleanup_remove_the_row() {
        let (manager, signer, _persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysMaybeSentBroadcaster)).await;

        // An ambiguous broadcast leaves the row tracked at `Built` — the
        // state every rejection cleanup and every resume starts from.
        let _ = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        let out_point = the_only_tracked_outpoint(&manager.wallet_manager, manager.wallet_id).await;

        let first = manager.claim_resume_dispatch(out_point);
        let second = manager.claim_resume_dispatch(out_point);
        assert!(
            manager
                .untrack_asset_lock(&out_point)
                .await
                .removed
                .is_empty(),
            "a claimed row must survive the cleanup"
        );

        drop(first);
        assert!(
            manager
                .untrack_asset_lock(&out_point)
                .await
                .removed
                .is_empty(),
            "one resume releasing its claim says nothing about the other: the \
             row must survive while any dispatch window is still open"
        );

        drop(second);
        assert!(
            manager
                .untrack_asset_lock(&out_point)
                .await
                .removed
                .contains(&out_point),
            "with the last claim gone the cleanup must be free again — a claim \
             that outlived its resume would fence these inputs for the rest of \
             the session"
        );
        let wm = manager.wallet_manager.read().await;
        let (_, info) = wm
            .get_wallet_and_info(&manager.wallet_id)
            .expect("wallet present");
        assert!(
            !info.tracked_asset_locks.contains_key(&out_point),
            "and the row itself is gone, not just reported as removed"
        );
    }

    #[tokio::test]
    async fn should_wallet_drain_refuse_missing_durable_wallet_snapshot() {
        let (wallet_manager, wallet_id, generation, _) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let manager = AssetLockManager::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster.clone(),
            // Admitted backend (inline commit) with no restored wallet, so
            // the refusal below comes from the durable read, not preflight.
            WalletPersister::new(
                wallet_id,
                Arc::new(CapturingPersistence {
                    commits_inline: true,
                    ..Default::default()
                }),
            ),
            Arc::clone(&generation),
        );
        // A globally successful load that omits this wallet must never be
        // interpreted as a definitive NotSent for an old persisted host UUID.
        assert!(matches!(
            manager.wallet_drain_operation_status([81; 16]).await,
            Err(PlatformWalletError::Persistence(message))
                if message.contains("wallet missing from durable snapshot")
        ));
        let operation = Arc::new(super::AssetLockOperationContext::new([81; 16]));
        let _registration = manager.register_wallet_drain_operation(operation.clone());
        assert!(matches!(
            manager.begin_wallet_drain(&operation).await,
            Err(PlatformWalletError::Persistence(message))
                if message.contains("wallet missing from durable snapshot")
        ));
        assert_eq!(
            operation.snapshot().state,
            super::AssetLockOperationState::Pending
        );
        assert_eq!(broadcaster.calls(), 0);
        assert!(manager.list_tracked_locks().await.is_empty());
    }

    #[tokio::test]
    async fn should_wallet_drain_release_only_the_last_matching_prebuild_registration() {
        let (wallet_manager, wallet_id, _, _) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let (manager, _) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );
        let operation = Arc::new(super::AssetLockOperationContext::new([82; 16]));
        let outer = manager.register_wallet_drain_operation(operation.clone());
        let inner = manager.register_wallet_drain_operation(operation.clone());
        manager.begin_wallet_drain(&operation).await.unwrap();
        let same_id = Arc::new(super::AssetLockOperationContext::new([82; 16]));
        let unrelated = manager.register_wallet_drain_operation(same_id.clone());
        drop(inner);
        assert_eq!(
            manager
                .wallet_drain_operation_status([82; 16])
                .await
                .unwrap()
                .state,
            super::AssetLockOperationState::Pending,
            "another Arc or a nested worker cannot clear the owner's registration"
        );
        drop(outer);
        assert_eq!(
            manager
                .wallet_drain_operation_status([82; 16])
                .await
                .unwrap()
                .state,
            super::AssetLockOperationState::Pending,
            "the queued same-UUID contender must remain visible after the owner exits"
        );
        assert!(manager.begin_wallet_drain(&same_id).await.is_err());
        drop(unrelated);
        assert_eq!(
            operation.snapshot().state,
            super::AssetLockOperationState::Pending
        );
        assert_eq!(
            manager
                .wallet_drain_operation_status([82; 16])
                .await
                .unwrap()
                .state,
            super::AssetLockOperationState::NotSent
        );
        let _retry = manager.register_wallet_drain_operation(operation.clone());
        assert_eq!(manager.begin_wallet_drain(&operation).await.unwrap(), None);
        assert_eq!(manager.broadcaster.calls(), 0);
    }

    #[tokio::test]
    async fn should_wallet_drain_retry_after_prebuild_load_failure() {
        let (wallet_manager, wallet_id, _, _) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let (manager, persistence) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );
        let operation = Arc::new(super::AssetLockOperationContext::new([83; 16]));
        persistence
            .fail_load
            .store(true, std::sync::atomic::Ordering::SeqCst);
        {
            let _registration = manager.register_wallet_drain_operation(operation.clone());
            assert!(matches!(
                manager.begin_wallet_drain(&operation).await,
                Err(PlatformWalletError::Persistence(_))
            ));
        }
        assert_eq!(
            operation.snapshot().state,
            super::AssetLockOperationState::Pending
        );
        persistence
            .fail_load
            .store(false, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            manager
                .wallet_drain_operation_status([83; 16])
                .await
                .unwrap()
                .state,
            super::AssetLockOperationState::NotSent
        );
        // Both a fresh context for the same UUID and a new explicit request
        // can proceed once durable loading works again.
        for id in [[83; 16], [84; 16]] {
            let retry = Arc::new(super::AssetLockOperationContext::new(id));
            let _registration = manager.register_wallet_drain_operation(retry.clone());
            assert_eq!(manager.begin_wallet_drain(&retry).await.unwrap(), None);
        }
        assert_eq!(manager.broadcaster.calls(), 0);
        assert!(manager.list_tracked_locks().await.is_empty());
    }

    #[tokio::test]
    async fn should_wallet_drain_pool_standard_families_without_change() {
        let (wallet_manager, wallet_id, _, signer) =
            funded_wallet_manager_dual_standard(&[700_000, 900_000], &[800_000]).await;
        let (manager, persistence) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );
        let operation = Arc::new(super::AssetLockOperationContext::new([11; 16]));
        let _registration = manager.register_wallet_drain_operation(operation.clone());
        assert_eq!(
            manager
                .wallet_drain_operation_status([11; 16])
                .await
                .unwrap()
                .state,
            super::AssetLockOperationState::Pending
        );
        assert_eq!(manager.begin_wallet_drain(&operation).await.unwrap(), None);
        let (_, outpoint) = manager
            .broadcast_funded_asset_lock_with_operation(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(1),
                },
                &crate::ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
                Some(&operation),
                None,
            )
            .await
            .unwrap();
        let locks = manager.list_tracked_locks().await;
        let lock = locks
            .iter()
            .find(|lock| lock.out_point == outpoint)
            .unwrap();
        assert_eq!(lock.transaction.input.len(), 3);
        assert_eq!(
            lock.transaction.output.len(),
            1,
            "only the burn output, no change"
        );
        assert!(lock.transaction.output[0].script_pubkey.is_op_return());
        assert_eq!(lock.transaction.output[0].value, lock.amount);
        assert!(lock.amount < 2_400_000);
        assert_eq!(lock.operation_id, Some([11; 16]));
        assert!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 2
        );
        assert_eq!(operation.snapshot().out_point, Some(outpoint));
        // UUID replay resolves the existing lock, and a fresh UUID cannot
        // bypass pending state even if the host lost its local marker.
        let replay = Arc::new(super::AssetLockOperationContext::new([11; 16]));
        let _replay_registration = manager.register_wallet_drain_operation(replay.clone());
        assert_eq!(
            manager.begin_wallet_drain(&replay).await.unwrap(),
            Some(outpoint)
        );
        let second = Arc::new(super::AssetLockOperationContext::new([12; 16]));
        let _second_registration = manager.register_wallet_drain_operation(second.clone());
        assert!(manager.begin_wallet_drain(&second).await.is_err());
        assert_eq!(
            second.snapshot().state,
            super::AssetLockOperationState::NotSent
        );
        assert_eq!(manager.broadcaster.calls(), 1);
        // Generic consumption cannot manufacture a proof-gated drain result.
        assert!(manager.consume_asset_lock(&outpoint).await.is_err());
        assert_eq!(
            manager
                .wallet_drain_operation_status([11; 16])
                .await
                .unwrap()
                .state,
            super::AssetLockOperationState::Pending
        );
    }

    #[tokio::test]
    async fn should_wallet_drain_include_dashpay_receiving_funds() {
        let (wallet_manager, wallet_id, _, signer, _) =
            funded_wallet_manager_with_contact(&[700_000], &[800_000]).await;
        let (manager, _) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );
        let operation = super::AssetLockOperationContext::new([21; 16]);
        let (_, outpoint) = manager
            .broadcast_funded_asset_lock_with_operation(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(1),
                },
                &crate::ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
                Some(&operation),
                None,
            )
            .await
            .unwrap();
        let locks = manager.list_tracked_locks().await;
        let lock = locks
            .iter()
            .find(|lock| lock.out_point == outpoint)
            .unwrap();
        assert_eq!(lock.transaction.input.len(), 2);
        assert_eq!(lock.transaction.output.len(), 1);
        assert!(lock.amount > 1_490_000 && lock.amount < 1_500_000);
    }

    #[tokio::test]
    async fn should_wallet_drain_refuse_101_inputs_before_dispatch_and_allow_100() {
        for count in [101, 100] {
            let (wallet_manager, wallet_id, _, signer) =
                crate::test_support::funded_wallet_manager_with_outputs(
                    StandardAccountType::BIP44Account,
                    &vec![10_000; count],
                )
                .await;
            let (manager, _) = asset_lock_manager_over(
                wallet_manager,
                wallet_id,
                Arc::new(CountingOkBroadcaster::default()),
            );
            let operation = super::AssetLockOperationContext::new([31; 16]);
            let result = manager
                .broadcast_funded_asset_lock_with_operation(
                    super::AssetLockBuildAmount::DrainAll {
                        minimum_lock_duffs: Some(1),
                    },
                    &crate::ASSET_LOCK_FUNDING_SOURCES,
                    0,
                    AssetLockFundingType::AssetLockShieldedAddressTopUp,
                    0,
                    &signer,
                    Some(&operation),
                    None,
                )
                .await;
            if count == 101 {
                assert!(matches!(
                    result,
                    Err(PlatformWalletError::AssetLockTooManyInputs {
                        count: 101,
                        max: 100
                    })
                ));
                assert_eq!(manager.broadcaster.calls(), 0);
                assert!(manager.list_tracked_locks().await.is_empty());
            } else {
                assert!(result.is_ok(), "{result:?}");
                assert_eq!(
                    manager.list_tracked_locks().await[0]
                        .transaction
                        .input
                        .len(),
                    100
                );
            }
        }
    }

    /// A wallet of `coins` coins of 10 000 duffs each, under an asset-lock
    /// manager whose broadcaster counts and accepts every transaction.
    async fn small_coin_asset_lock_manager(
        coins: usize,
    ) -> (Arc<AssetLockManager<CountingOkBroadcaster>>, WalletSigner) {
        let (wallet_manager, wallet_id, _, signer) = funded_wallet_manager_with_outputs(
            StandardAccountType::BIP44Account,
            &vec![10_000; coins],
        )
        .await;
        let (manager, _) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );
        (manager, signer)
    }

    /// Builds and broadcasts an identity-registration lock of exactly
    /// `amount_duffs`, funded from the default asset-lock sources.
    async fn broadcast_exact_registration_lock(
        manager: &AssetLockManager<CountingOkBroadcaster>,
        signer: &WalletSigner,
        amount_duffs: u64,
        operation_tag: u8,
    ) -> Result<(DerivationPath, OutPoint), PlatformWalletError> {
        let operation = AssetLockOperationContext::new([operation_tag; 16]);
        manager
            .broadcast_funded_asset_lock_with_operation(
                AssetLockBuildAmount::Exact(amount_duffs),
                &ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                signer,
                Some(&operation),
                None,
            )
            .await
    }

    /// A fixed amount held only in small coins selects more inputs than an
    /// asset lock may carry: the lock is refused before it is tracked or
    /// broadcast, as an oversized drain is, and its inputs stay spendable.
    #[tokio::test]
    async fn should_refuse_a_fixed_amount_lock_over_100_inputs_before_dispatch() {
        // 100 coins of 10 000 duffs cover 1 000 000 only before the fee.
        let (manager, signer) = small_coin_asset_lock_manager(120).await;

        let result = broadcast_exact_registration_lock(&manager, &signer, 1_000_000, 32).await;

        assert!(
            matches!(
                result,
                Err(PlatformWalletError::AssetLockTooManyInputs { count, max: 100 }) if count > 100
            ),
            "{result:?}"
        );
        assert_eq!(manager.broadcaster.calls(), 0);
        assert!(manager.list_tracked_locks().await.is_empty());
        // The refused build released its inputs: a smaller lock funds.
        let retry = broadcast_exact_registration_lock(&manager, &signer, 500_000, 33).await;
        assert!(retry.is_ok(), "{retry:?}");
        assert_eq!(manager.broadcaster.calls(), 1);
    }

    /// A refused build hands back the funding-key index it consumed. Refusing
    /// more often than the pool holds generated addresses must neither leave
    /// the funding type unable to build nor push the next lock past the
    /// addresses seed-only discovery covers: it reuses the first index.
    #[tokio::test]
    async fn should_reuse_the_funding_index_after_repeated_input_cap_refusals() {
        let (manager, signer) = small_coin_asset_lock_manager(120).await;

        for tag in 40..48 {
            let result = broadcast_exact_registration_lock(&manager, &signer, 1_000_000, tag).await;
            assert!(
                matches!(
                    result,
                    Err(PlatformWalletError::AssetLockTooManyInputs { .. })
                ),
                "refusal {tag}: {result:?}"
            );
        }

        let retry = broadcast_exact_registration_lock(&manager, &signer, 500_000, 48).await;
        let (path, _) = retry.expect("a lock that fits funds after the refusals");
        assert_eq!(manager.broadcaster.calls(), 1);
        assert_eq!(
            path.into_iter().last(),
            Some(&ChildNumber::from_normal_idx(0).expect("index 0")),
            "the lock reuses the funding index the refusals handed back"
        );
    }

    /// Handing back a refused build's index leaves every index a published
    /// lock used untouched: the published index stays used (an invitation's
    /// exported bearer key must never be reissued), and only the refused one
    /// is reused.
    #[tokio::test]
    async fn should_hand_back_only_the_refused_funding_index() {
        let (manager, signer) = small_coin_asset_lock_manager(120).await;

        let (published, _) = broadcast_exact_registration_lock(&manager, &signer, 100_000, 60)
            .await
            .expect("a small lock funds from a few coins");
        // About 109 coins are left: 1 000 000 needs more than 100 of them.
        let refused = broadcast_exact_registration_lock(&manager, &signer, 1_000_000, 61).await;
        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::AssetLockTooManyInputs { .. })
            ),
            "{refused:?}"
        );

        {
            let wm = manager.wallet_manager.read().await;
            let info = wm.get_wallet_info(&manager.wallet_id).expect("wallet");
            let account = info
                .core_wallet
                .accounts
                .identity_registration
                .as_ref()
                .expect("registration funding account");
            let pool = &account.managed_account_type().address_pools()[0];
            let used: Vec<u32> = (pool.addresses().values())
                .filter(|info| info.is_used())
                .map(|info| info.index)
                .collect();
            assert_eq!(used, [0]);
            assert_eq!(pool.highest_used(), Some(0));
        }

        let (next, _) = broadcast_exact_registration_lock(&manager, &signer, 300_000, 62)
            .await
            .expect("a lock that fits funds after the refusal");
        let index = |path: &DerivationPath| path.into_iter().last().copied();
        assert_eq!(
            index(&published),
            Some(ChildNumber::from_normal_idx(0).expect("index 0"))
        );
        assert_eq!(
            index(&next),
            Some(ChildNumber::from_normal_idx(1).expect("index 1"))
        );
    }

    /// A fixed amount that needs every one of 100 coins is exactly at the cap,
    /// so it is built and broadcast.
    #[tokio::test]
    async fn should_broadcast_a_fixed_amount_lock_of_exactly_100_inputs() {
        // 99 coins hold 990 000 duffs, short of 980 000 plus a 100-input fee.
        let (manager, signer) = small_coin_asset_lock_manager(100).await;

        let result = broadcast_exact_registration_lock(&manager, &signer, 980_000, 50).await;

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(manager.broadcaster.calls(), 1);
        let locks = manager.list_tracked_locks().await;
        assert_eq!(locks[0].transaction.input.len(), 100);
    }

    /// Stops signing while the builder owns the manager lock, allowing a
    /// competing writer to queue before the durability checkpoint.
    #[cfg(feature = "shielded")]
    struct PausedDrainSigner {
        inner: WalletSigner,
        resume: Notify,
    }

    #[cfg(feature = "shielded")]
    #[async_trait]
    impl key_wallet::signer::Signer for PausedDrainSigner {
        type Error = String;

        fn supported_methods(&self) -> &[key_wallet::signer::SignerMethod] {
            self.inner.supported_methods()
        }

        async fn sign_ecdsa(
            &self,
            path: &key_wallet::bip32::DerivationPath,
            sighash: [u8; 32],
        ) -> Result<
            (
                dashcore::crypto::ecdsa::EcdsaSignature,
                dashcore::crypto::ecdsa::EcdsaPublicKey,
            ),
            String,
        > {
            let signature = self.inner.sign_ecdsa(path, sighash).await?;
            self.resume.notified().await;
            Ok(signature)
        }

        async fn public_key(
            &self,
            path: &key_wallet::bip32::DerivationPath,
        ) -> Result<dashcore::crypto::ecdsa::EcdsaPublicKey, String> {
            self.inner.public_key(path).await
        }
    }

    #[cfg(feature = "shielded")]
    #[async_trait]
    impl key_wallet::signer::ExtendedPubKeySigner for PausedDrainSigner {
        async fn extended_public_key(
            &self,
            path: &key_wallet::bip32::DerivationPath,
        ) -> Result<key_wallet::bip32::ExtendedPubKey, String> {
            self.inner.extended_public_key(path).await
        }
    }

    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_checkpoint_drain_before_post_build_cancellation() {
        let (wallet_manager, wallet_id, _, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let (manager, persistence) = asset_lock_manager_over(
            wallet_manager.clone(),
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );
        let signer = PausedDrainSigner {
            inner: signer,
            resume: Notify::new(),
        };
        let operation = Arc::new(super::AssetLockOperationContext::new([42; 16]));
        let registration = manager.register_wallet_drain_operation(operation.clone());
        manager.begin_wallet_drain(&operation).await.unwrap();
        let mut future = Box::pin(manager.broadcast_funded_asset_lock_with_operation(
            super::AssetLockBuildAmount::DrainAll {
                minimum_lock_duffs: Some(1),
            },
            &crate::ASSET_LOCK_FUNDING_SOURCES,
            0,
            AssetLockFundingType::AssetLockShieldedAddressTopUp,
            0,
            &signer,
            Some(&operation),
            None,
        ));
        assert!(futures::poll!(&mut future).is_pending());
        let mut writer = Box::pin(wallet_manager.write());
        assert!(futures::poll!(&mut writer).is_pending());
        signer.resume.notify_one();
        // The queued writer blocks tracking, but cannot interrupt persistence.
        assert!(futures::poll!(&mut future).is_pending());
        let blocker = writer.await;
        let outpoint = operation
            .snapshot()
            .out_point
            .expect("checkpointed outpoint");
        {
            let stored = persistence.stored.lock().unwrap();
            assert!(
                stored
                    .iter()
                    .any(|cs| cs.asset_locks.as_ref().is_some_and(|locks| locks
                        .asset_locks
                        .get(&outpoint)
                        .is_some_and(|lock| lock.operation_id == Some([42; 16])))),
                "the first post-build suspension must have a durable recovery row"
            );
            assert!(
                stored
                    .iter()
                    .any(|cs| cs.account_address_pools.iter().any(|pool| pool
                        .addresses
                        .iter()
                        .any(|address| matches!(
                            address.state,
                            key_wallet::managed_account::address_pool::AddressState::Used
                        )))),
                "the burned funding key must be persisted too"
            );
        }
        assert_eq!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        assert_eq!(manager.broadcaster.calls(), 0);
        drop(future);
        drop(registration);
        drop(blocker);
        assert_eq!(
            operation.snapshot().state,
            super::AssetLockOperationState::Pending
        );
        assert_eq!(operation.snapshot().out_point, Some(outpoint));
        assert!(manager.list_tracked_locks().await.is_empty());
        persistence
            .restore_stored_locks
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let retry = Arc::new(super::AssetLockOperationContext::new([42; 16]));
        let _registration = manager.register_wallet_drain_operation(retry.clone());
        assert_eq!(
            manager.begin_wallet_drain(&retry).await.unwrap(),
            Some(outpoint)
        );
        assert_eq!(
            manager
                .tracked_resume_metadata(&outpoint)
                .await
                .unwrap()
                .funding_type,
            AssetLockFundingType::AssetLockShieldedAddressTopUp
        );
        let result = manager
            .resume_asset_lock(&outpoint, Some(Duration::ZERO))
            .await;
        assert!(
            !matches!(result, Err(PlatformWalletError::AssetLockNotTracked(_))),
            "{result:?}"
        );
        assert_eq!(
            manager.broadcaster.calls(),
            1,
            "resume dispatches the original checkpointed transaction"
        );
        let locks = manager.list_tracked_locks().await;
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].out_point, outpoint);
        assert_eq!(locks[0].status, AssetLockStatus::Broadcast);
        // Retrying hydration must not roll the live Broadcast row back to Built.
        assert_eq!(
            manager.begin_wallet_drain(&retry).await.unwrap(),
            Some(outpoint)
        );
        assert_eq!(
            manager.list_tracked_locks().await[0].status,
            AssetLockStatus::Broadcast
        );
    }

    #[tokio::test]
    async fn should_wallet_drain_pool_store_failure_release_inputs_for_retry() {
        assert_wallet_drain_pool_failure_can_retry(true).await;
    }

    #[tokio::test]
    async fn should_wallet_drain_pool_flush_failure_release_inputs_for_retry() {
        assert_wallet_drain_pool_failure_can_retry(false).await;
    }

    async fn assert_wallet_drain_pool_failure_can_retry(fail_store: bool) {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager_dual_standard(&[700_000, 900_000], &[800_000]).await;
        let persistence = Arc::new(CapturingPersistence {
            fail_next_pool_store: std::sync::atomic::AtomicBool::new(fail_store),
            fail_on_flush: (!fail_store).then_some(1),
            ..CapturingPersistence::for_wallet(&wallet_manager, wallet_id)
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let manager = AssetLockManager::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster.clone(),
            WalletPersister::new(wallet_id, persistence.clone()),
            Arc::clone(&generation),
        );
        let operation = Arc::new(super::AssetLockOperationContext::new([41; 16]));
        let registration = manager.register_wallet_drain_operation(operation.clone());
        manager.begin_wallet_drain(&operation).await.unwrap();
        let result = manager
            .broadcast_funded_asset_lock_with_operation(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(1),
                },
                &crate::ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
                Some(&operation),
                None,
            )
            .await;
        assert!(matches!(result, Err(PlatformWalletError::Persistence(_))));
        drop(registration);
        assert_eq!(broadcaster.calls(), 0);
        assert!(manager.list_tracked_locks().await.is_empty());
        assert_eq!(operation.snapshot().out_point, None);
        assert_eq!(
            operation.snapshot().state,
            super::AssetLockOperationState::NotSent
        );
        assert_eq!(
            manager
                .wallet_drain_operation_status([41; 16])
                .await
                .unwrap(),
            operation.snapshot()
        );

        // Retry immediately with the same operation ID and all three original
        // inputs, including the second account family. No TTL sweep is needed.
        let retry = Arc::new(super::AssetLockOperationContext::new([41; 16]));
        let _registration = manager.register_wallet_drain_operation(retry.clone());
        assert_eq!(manager.begin_wallet_drain(&retry).await.unwrap(), None);
        manager
            .broadcast_funded_asset_lock_with_operation(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(1),
                },
                &crate::ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
                Some(&retry),
                None,
            )
            .await
            .unwrap();
        assert_eq!(broadcaster.calls(), 1);
        let locks = manager.list_tracked_locks().await;
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].transaction.input.len(), 3);
        // The failed build consumed a key: successful retry must persist both
        // used indices rather than rolling the funding pool backwards.
        assert!(persistence.stored.lock().unwrap().iter().any(|cs| {
            cs.account_address_pools.iter().any(|pool| {
                matches!(
                    pool.account_type,
                    key_wallet::account::AccountType::AssetLockShieldedAddressTopUp
                ) && pool
                    .addresses
                    .iter()
                    .filter(|address| {
                        matches!(
                            address.state,
                            key_wallet::managed_account::address_pool::AddressState::Used
                        )
                    })
                    .count()
                    == 2
            })
        }));
    }

    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_publish_not_sent_before_cancelled_rejection_cleanup() {
        let (wallet_manager, wallet_id, _, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let (manager, persistence) = asset_lock_manager_over(
            wallet_manager.clone(),
            wallet_id,
            Arc::new(AlwaysRejectedBroadcaster),
        );
        let blocked_manager = Arc::new(Mutex::new(None));
        let blocker_slot = blocked_manager.clone();
        *persistence.before_remove.lock().unwrap() = Some(Box::new(move || {
            // Untracking is complete; delay only reservation cleanup.
            *blocker_slot.lock().unwrap() = Some(wallet_manager.clone().try_write_owned().unwrap());
        }));
        let operation = Arc::new(super::AssetLockOperationContext::new([43; 16]));
        let registration = manager.register_wallet_drain_operation(operation.clone());
        manager.begin_wallet_drain(&operation).await.unwrap();
        let mut future = Box::pin(manager.broadcast_funded_asset_lock_with_operation(
            super::AssetLockBuildAmount::DrainAll {
                minimum_lock_duffs: Some(1),
            },
            &crate::ASSET_LOCK_FUNDING_SOURCES,
            0,
            AssetLockFundingType::AssetLockShieldedAddressTopUp,
            0,
            &signer,
            Some(&operation),
            None,
        ));
        assert!(futures::poll!(&mut future).is_pending());
        assert_eq!(persistence.removed_outpoints().len(), 1);
        // The pool's flush and the row's; the removal is durable at its store
        // on this inline-committing backend, so it asks for no flush.
        assert_eq!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        drop(future);
        drop(registration);
        blocked_manager.lock().unwrap().take();
        assert_eq!(
            operation.snapshot().state,
            super::AssetLockOperationState::NotSent
        );
        assert!(operation.snapshot().out_point.is_none());
        assert_eq!(
            manager
                .wallet_drain_operation_status([43; 16])
                .await
                .unwrap(),
            operation.snapshot()
        );
        // Neither the same UUID nor another UUID is stranded by context_pending.
        for id in [[43; 16], [44; 16]] {
            let retry = Arc::new(super::AssetLockOperationContext::new(id));
            let _registration = manager.register_wallet_drain_operation(retry.clone());
            assert_eq!(manager.begin_wallet_drain(&retry).await.unwrap(), None);
        }
    }

    #[tokio::test]
    async fn should_wallet_drain_definite_rejection_clear_only_after_durable_cleanup() {
        let (wallet_manager, wallet_id, _, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let (manager, persistence) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(AlwaysRejectedBroadcaster),
        );
        let operation = Arc::new(super::AssetLockOperationContext::new([51; 16]));
        let _registration = manager.register_wallet_drain_operation(operation.clone());
        manager.begin_wallet_drain(&operation).await.unwrap();
        let result = manager
            .broadcast_funded_asset_lock_with_operation(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(1),
                },
                &crate::ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
                Some(&operation),
                None,
            )
            .await;
        assert!(matches!(
            result,
            Err(PlatformWalletError::TransactionBroadcast(_))
        ));
        assert!(manager.list_tracked_locks().await.is_empty());
        assert_eq!(persistence.removed_outpoints().len(), 1);
        // The pool's flush and the row's; the removal is durable at its store
        // on this inline-committing backend, so it asks for no flush.
        assert_eq!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        assert_eq!(
            operation.snapshot().state,
            super::AssetLockOperationState::NotSent
        );
        assert_eq!(
            manager
                .wallet_drain_operation_status([51; 16])
                .await
                .unwrap()
                .state,
            super::AssetLockOperationState::NotSent
        );
        let second = Arc::new(super::AssetLockOperationContext::new([52; 16]));
        let _second_registration = manager.register_wallet_drain_operation(second.clone());
        assert_eq!(manager.begin_wallet_drain(&second).await.unwrap(), None);
    }

    #[tokio::test]
    async fn should_release_rejected_drain_only_when_removal_committed() {
        for fail_remove_store in [false, true] {
            {
                let commits_inline = true;
                let (wallet_manager, wallet_id, generation, signer) =
                    funded_wallet_manager(StandardAccountType::BIP44Account).await;
                let persistence = Arc::new(CapturingPersistence {
                    commits_inline,
                    fail_remove_store,
                    fail_on_flush: Some(3),
                    restore_stored_locks: std::sync::atomic::AtomicBool::new(true),
                    ..CapturingPersistence::for_wallet(&wallet_manager, wallet_id)
                });
                let manager = AssetLockManager::new(
                    Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
                    wallet_manager,
                    wallet_id,
                    Arc::new(Notify::new()),
                    Arc::new(AlwaysRejectedBroadcaster),
                    WalletPersister::new(wallet_id, persistence.clone()),
                    Arc::clone(&generation),
                );
                let operation = Arc::new(super::AssetLockOperationContext::new([53; 16]));
                let registration = manager.register_wallet_drain_operation(operation.clone());
                manager.begin_wallet_drain(&operation).await.unwrap();
                let result = manager
                    .broadcast_funded_asset_lock_with_operation(
                        super::AssetLockBuildAmount::DrainAll {
                            minimum_lock_duffs: Some(1),
                        },
                        &crate::ASSET_LOCK_FUNDING_SOURCES,
                        0,
                        AssetLockFundingType::AssetLockShieldedAddressTopUp,
                        0,
                        &signer,
                        Some(&operation),
                        None,
                    )
                    .await;
                let released = commits_inline && !fail_remove_store;
                assert_eq!(
                    matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
                    released,
                    "inline={commits_inline}, store failure={fail_remove_store}: {result:?}"
                );
                if !released {
                    // The row may survive: the unknown outcome, not a
                    // retry-safe persistence failure.
                    assert!(
                        matches!(
                            result,
                            Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
                        ),
                        "{result:?}"
                    );
                }
                assert!(manager.list_tracked_locks().await.is_empty());
                let restored = persistence.load().unwrap();
                let persisted_count: usize = restored.wallets[&wallet_id]
                    .unused_asset_locks
                    .values()
                    .map(|locks| locks.len())
                    .sum();
                assert_eq!(persisted_count, usize::from(!released));
                assert_eq!(operation.snapshot().out_point.is_none(), released);
                drop(registration);
                assert_eq!(
                    manager
                        .wallet_drain_operation_status([53; 16])
                        .await
                        .unwrap()
                        .state,
                    if released {
                        super::AssetLockOperationState::NotSent
                    } else {
                        super::AssetLockOperationState::Pending
                    }
                );
                if released {
                    for id in [[53; 16], [54; 16]] {
                        let retry = Arc::new(super::AssetLockOperationContext::new(id));
                        let _registration = manager.register_wallet_drain_operation(retry.clone());
                        assert_eq!(manager.begin_wallet_drain(&retry).await.unwrap(), None);
                    }
                } else {
                    let other = Arc::new(super::AssetLockOperationContext::new([54; 16]));
                    assert!(manager.begin_wallet_drain(&other).await.is_err());
                }
                // Test actual input selection too: clearing only the operation
                // snapshot would still leave the non-expiring spend fence.
                let rebuild = manager
                    .build_asset_lock_transaction(
                        1_000_000,
                        0,
                        AssetLockFundingType::IdentityRegistration,
                        0,
                        &signer,
                    )
                    .await;
                assert_eq!(rebuild.is_ok(), released, "{rebuild:?}");
            }
        }
    }

    /// On the wallet-drain path the observer runs once, while the operation
    /// is `Pending` with the reported outpoint and before the lock's row is
    /// stored or the lock tracked. When that store then fails, the reported
    /// lock is recovered through the same operation UUID, not by outpoint.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_report_a_wallet_drain_lock_before_its_row_is_stored() {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let persistence = Arc::new(CapturingPersistence {
            commits_inline: true,
            fail_lock_store: std::sync::atomic::AtomicBool::new(true),
            restore_stored_locks: std::sync::atomic::AtomicBool::new(true),
            ..CapturingPersistence::for_wallet(&wallet_manager, wallet_id)
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let manager = AssetLockManager::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster.clone(),
            WalletPersister::new(wallet_id, persistence.clone()),
            Arc::clone(&generation),
        );
        let operation = Arc::new(super::AssetLockOperationContext::new([58; 16]));
        let registration = manager.register_wallet_drain_operation(operation.clone());
        manager.begin_wallet_drain(&operation).await.unwrap();

        let seen = Mutex::new(Vec::new());
        let observer = |out_point: OutPoint| {
            let tracked = manager
                .wallet_manager
                .try_read()
                .expect("the build holds no wallet-manager lock while observing")
                .get_wallet_and_info(&manager.wallet_id)
                .is_some_and(|(_, info)| info.tracked_asset_locks.contains_key(&out_point));
            let stored = persistence.stored_outpoints().contains(&out_point)
                || !persistence.failed_lock_stores.lock().unwrap().is_empty();
            let snapshot = operation.snapshot();
            seen.lock().unwrap().push((
                out_point,
                tracked,
                stored,
                snapshot.state,
                snapshot.out_point,
                broadcaster.calls(),
            ));
        };
        let result = manager
            .broadcast_funded_asset_lock_with_operation(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(1),
                },
                &crate::ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
                Some(&operation),
                Some(&observer),
            )
            .await;
        assert!(matches!(result, Err(PlatformWalletError::Persistence(_))));
        let point = operation.snapshot().out_point.unwrap();
        assert_eq!(
            seen.into_inner().unwrap(),
            vec![(
                point,
                false,
                false,
                super::AssetLockOperationState::Pending,
                Some(point),
                0
            )]
        );
        assert!(manager.list_tracked_locks().await.is_empty());
        drop(registration);
        drop(operation);

        let retry = Arc::new(super::AssetLockOperationContext::new([58; 16]));
        let _registration = manager.register_wallet_drain_operation(retry.clone());
        persistence
            .fail_lock_store
            .store(false, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            manager.begin_wallet_drain(&retry).await.unwrap(),
            Some(point)
        );
        let locks = manager.list_tracked_locks().await;
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].out_point, point);
        assert_eq!(broadcaster.calls(), 0);
    }

    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn should_retry_rolled_back_drain_checkpoint_without_rebuilding() {
        {
            let (wallet_manager, wallet_id, generation, signer) =
                funded_wallet_manager(StandardAccountType::BIP44Account).await;
            let persistence = Arc::new(CapturingPersistence {
                commits_inline: true,
                fail_lock_store: std::sync::atomic::AtomicBool::new(true),
                restore_stored_locks: std::sync::atomic::AtomicBool::new(true),
                ..CapturingPersistence::for_wallet(&wallet_manager, wallet_id)
            });
            let broadcaster = Arc::new(CountingOkBroadcaster::default());
            let manager = AssetLockManager::new(
                Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
                wallet_manager,
                wallet_id,
                Arc::new(Notify::new()),
                broadcaster.clone(),
                WalletPersister::new(wallet_id, persistence.clone()),
                Arc::clone(&generation),
            );
            let operation = Arc::new(super::AssetLockOperationContext::new([56; 16]));
            let registration = manager.register_wallet_drain_operation(operation.clone());
            manager.begin_wallet_drain(&operation).await.unwrap();
            let result = manager
                .broadcast_funded_asset_lock_with_operation(
                    super::AssetLockBuildAmount::DrainAll {
                        minimum_lock_duffs: Some(1),
                    },
                    &crate::ASSET_LOCK_FUNDING_SOURCES,
                    0,
                    AssetLockFundingType::AssetLockShieldedAddressTopUp,
                    0,
                    &signer,
                    Some(&operation),
                    None,
                )
                .await;
            assert!(matches!(result, Err(PlatformWalletError::Persistence(_))));
            let point = operation.snapshot().out_point.unwrap();
            let original_tx = persistence.failed_lock_stores.lock().unwrap()[0]
                .asset_locks
                .as_ref()
                .unwrap()
                .asset_locks[&point]
                .transaction
                .clone();
            drop(registration);
            drop(operation);
            assert!(persistence.load().unwrap().wallets[&wallet_id]
                .unused_asset_locks
                .is_empty());
            assert!(manager.list_tracked_locks().await.is_empty());
            assert_eq!(broadcaster.calls(), 0);
            assert_eq!(
                manager
                    .wallet_drain_operation_status([56; 16])
                    .await
                    .unwrap()
                    .state,
                super::AssetLockOperationState::Pending
            );
            let other = Arc::new(super::AssetLockOperationContext::new([57; 16]));
            let other_registration = manager.register_wallet_drain_operation(other.clone());
            assert!(manager.begin_wallet_drain(&other).await.is_err());
            drop(other_registration);
            // No ordinary spend or generic resume may race the unpublished row.
            assert!(manager
                .build_asset_lock_transaction(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    0,
                    &signer,
                )
                .await
                .is_err());
            assert!(matches!(
                manager
                    .resume_asset_lock(&point, Some(Duration::ZERO))
                    .await,
                Err(PlatformWalletError::AssetLockNotTracked(_))
            ));
            assert_eq!(broadcaster.calls(), 0);

            // A fresh caller/context for the same UUID finds the retained
            // original checkpoint. Another rollback must retain it again.
            let retry = Arc::new(super::AssetLockOperationContext::new([56; 16]));
            let _registration = manager.register_wallet_drain_operation(retry.clone());
            assert!(matches!(
                manager.begin_wallet_drain(&retry).await,
                Err(PlatformWalletError::Persistence(_))
            ));
            assert_eq!(persistence.failed_lock_stores.lock().unwrap().len(), 2);
            assert!(manager.list_tracked_locks().await.is_empty());
            persistence
                .fail_lock_store
                .store(false, std::sync::atomic::Ordering::SeqCst);
            assert_eq!(
                manager.begin_wallet_drain(&retry).await.unwrap(),
                Some(point)
            );
            let locks = manager.list_tracked_locks().await;
            assert_eq!(locks.len(), 1);
            assert_eq!(locks[0].transaction, original_tx);
            assert_eq!(locks[0].out_point, point);
            let restored = persistence.load().unwrap();
            let durable = restored.wallets[&wallet_id]
                .unused_asset_locks
                .values()
                .find_map(|locks| locks.get(&point))
                .unwrap();
            assert_eq!(durable.transaction, original_tx);
            assert_eq!(
                broadcaster.calls(),
                0,
                "checkpoint retry must not broadcast"
            );
            let _ = manager
                .resume_asset_lock(&point, Some(Duration::ZERO))
                .await;
            assert_eq!(
                broadcaster.calls(),
                1,
                "resume reuses the original fenced inputs"
            );
            assert_eq!(
                manager.list_tracked_locks().await[0].status,
                AssetLockStatus::Broadcast
            );
            assert_eq!(
                persistence
                    .stored
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|cs| !cs.account_address_pools.is_empty())
                    .count(),
                1,
                "retry must not allocate a second funding key or rebuild"
            );
        }
    }

    #[tokio::test]
    async fn should_wallet_drain_failed_row_flush_never_publish_resumable_lock() {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let persistence = Arc::new(CapturingPersistence {
            fail_on_flush: Some(2),
            restore_stored_locks: std::sync::atomic::AtomicBool::new(true),
            ..CapturingPersistence::for_wallet(&wallet_manager, wallet_id)
        });
        let broadcaster = Arc::new(CountingOkBroadcaster::default());
        let manager = AssetLockManager::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster.clone(),
            WalletPersister::new(wallet_id, persistence.clone()),
            Arc::clone(&generation),
        );
        let operation = Arc::new(super::AssetLockOperationContext::new([61; 16]));
        let _registration = manager.register_wallet_drain_operation(operation.clone());
        manager.begin_wallet_drain(&operation).await.unwrap();
        let result = manager
            .broadcast_funded_asset_lock_with_operation(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: Some(1),
                },
                &crate::ASSET_LOCK_FUNDING_SOURCES,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
                Some(&operation),
                None,
            )
            .await;
        assert!(matches!(result, Err(PlatformWalletError::Persistence(_))));
        assert_eq!(broadcaster.calls(), 0);
        assert!(manager.list_tracked_locks().await.is_empty());
        assert_eq!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        let point = operation.snapshot().out_point.unwrap();
        assert!(persistence
            .stored
            .lock()
            .unwrap()
            .iter()
            .filter_map(|cs| cs.asset_locks.as_ref())
            .any(|cs| cs
                .asset_locks
                .get(&point)
                .is_some_and(|lock| lock.operation_id == Some([61; 16]))));
        assert_eq!(
            manager
                .wallet_drain_operation_status([61; 16])
                .await
                .unwrap()
                .state,
            super::AssetLockOperationState::Pending
        );
        // On an inline-committing backend the row store already committed;
        // the failed flush notification only kept it unpublished. Explicit
        // retry re-establishes that exact row before tracking it.
        assert!(persistence.load().unwrap().wallets[&wallet_id]
            .unused_asset_locks
            .values()
            .any(|locks| locks.contains_key(&point)));
        assert_eq!(
            manager.begin_wallet_drain(&operation).await.unwrap(),
            Some(point)
        );
        assert_eq!(manager.list_tracked_locks().await[0].out_point, point);
        assert!(persistence.load().unwrap().wallets[&wallet_id]
            .unused_asset_locks
            .values()
            .any(|locks| locks.contains_key(&point)));
        assert_eq!(broadcaster.calls(), 0);
    }

    struct DrainPanicBroadcaster {
        persistence: Arc<CapturingPersistence>,
    }

    #[async_trait]
    impl TransactionBroadcaster for DrainPanicBroadcaster {
        async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, BroadcastError> {
            assert!(
                self.persistence
                    .flushes
                    .load(std::sync::atomic::Ordering::SeqCst)
                    >= 2,
                "row and pool state must already be flushed at dispatch"
            );
            panic!("simulated panic after dispatch may have started")
        }
    }

    #[tokio::test]
    async fn should_wallet_drain_panic_keep_exact_outpoint_and_pending_status() {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let persistence = Arc::new(CapturingPersistence::for_wallet(&wallet_manager, wallet_id));
        let manager = Arc::new(AssetLockManager::new(
            Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(DrainPanicBroadcaster {
                persistence: persistence.clone(),
            }),
            WalletPersister::new(wallet_id, persistence),
            Arc::clone(&generation),
        ));
        let operation = Arc::new(super::AssetLockOperationContext::new([71; 16]));
        let _registration = manager.register_wallet_drain_operation(operation.clone());
        manager.begin_wallet_drain(&operation).await.unwrap();
        let task_manager = manager.clone();
        let task_operation = operation.clone();
        let task = tokio::spawn(async move {
            task_manager
                .broadcast_funded_asset_lock_with_operation(
                    super::AssetLockBuildAmount::DrainAll {
                        minimum_lock_duffs: Some(1),
                    },
                    &crate::ASSET_LOCK_FUNDING_SOURCES,
                    0,
                    AssetLockFundingType::AssetLockShieldedAddressTopUp,
                    0,
                    &signer,
                    Some(&task_operation),
                    None,
                )
                .await
        });
        assert!(task.await.unwrap_err().is_panic());
        let snapshot = operation.snapshot();
        assert_eq!(snapshot.state, super::AssetLockOperationState::Pending);
        assert_eq!(
            Some(manager.list_tracked_locks().await[0].out_point),
            snapshot.out_point
        );
        assert_eq!(
            manager
                .wallet_drain_operation_status([71; 16])
                .await
                .unwrap(),
            snapshot
        );
    }

    /// An asset lock from a wallet holding a 1,000 DASH masternode collateral
    /// and a 5 DASH coin is funded from the 5 DASH coin: a lock only the
    /// collateral could fund is refused, and draining the account locks the
    /// spare coin alone.
    #[tokio::test]
    async fn should_fund_an_asset_lock_without_the_masternode_collateral() {
        const DASH: u64 = 100_000_000;
        let (wallet_manager, wallet_id, _generation, signer, _collateral, spare) =
            crate::test_support::wallet_manager_with_registered_collateral(
                StandardAccountType::BIP44Account,
                5 * DASH,
            )
            .await;
        let (manager, _persistence) = asset_lock_manager_over(
            wallet_manager,
            wallet_id,
            Arc::new(CountingOkBroadcaster::default()),
        );

        let shortfall = manager
            .build_asset_lock_transaction(
                6 * DASH,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                shortfall,
                Err(PlatformWalletError::AssetLockInsufficientFunds { .. })
            ),
            "a 6 DASH lock needs the collateral, got {shortfall:?}"
        );

        let (tx, _path, _token, _accounts) = manager
            .build_asset_lock_transaction_with_funding(
                super::AssetLockBuildAmount::DrainAll {
                    minimum_lock_duffs: None,
                },
                &[AccountTypePreference::BIP44],
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await
            .expect("draining the account locks the spare coin");
        let inputs: Vec<OutPoint> = tx.input.iter().map(|input| input.previous_output).collect();
        assert_eq!(
            inputs,
            vec![spare],
            "the drain must leave the collateral alone"
        );
    }
}
