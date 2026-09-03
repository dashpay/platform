//! Atomic Core transaction finalization.
//!
//! Funding and reservation deliberately happen in one synchronous critical
//! section under the wallet-manager write lock. Signing happens only after the
//! lock is dropped, so an external signer may call back into a host mnemonic
//! resolver without pinning wallet state.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use dashcore::{Address, OutPoint, Transaction};
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_collection::ManagedAccountCollection;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionError;
use key_wallet::wallet::managed_wallet_info::fee::{estimate_tx_size, FeeRate};
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    BuilderError, TransactionBuilder, TransactionSigner,
};
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::Wallet;
use key_wallet::{DerivationPath, ReservationToken, Utxo};

use super::{CoreWallet, WalletGeneration};
use crate::broadcaster::TransactionBroadcaster;
use crate::wallet::reservations::reservation_expired;
use crate::PlatformWalletError;

/// What funded (or failed to fund) a build, for attributing a shortfall.
///
/// A single-source build can name its account. A pooled build cannot: the
/// builder's `available`/`required` describe the UNION of every offered source,
/// so naming one of them would misreport the figures and could point at a
/// source that contributed nothing.
enum FundingContext<'a> {
    Single {
        preference: AccountTypePreference,
        index: u32,
    },
    Pooled(&'a [AccountTypePreference]),
}

fn map_builder_error(error: BuilderError, context: FundingContext<'_>) -> PlatformWalletError {
    let funds = match error {
        BuilderError::InsufficientFunds {
            available,
            required,
        }
        | BuilderError::CoinSelection(SelectionError::InsufficientFunds {
            available,
            required,
        }) => Some((Some(available), Some(required))),
        BuilderError::CoinSelection(SelectionError::NoUtxosAvailable) => Some((Some(0), None)),
        _ => None,
    };
    if let Some((available, required)) = funds {
        return match context {
            FundingContext::Single { preference, index } => {
                PlatformWalletError::CoreInsufficientFunds {
                    account_type: preference,
                    account_index: index,
                    available,
                    required,
                }
            }
            FundingContext::Pooled(sources) => PlatformWalletError::CorePooledInsufficientFunds {
                sources: sources.to_vec(),
                available,
                required,
            },
        };
    }
    PlatformWalletError::TransactionBuild(error.to_string())
}

/// A signed Core transaction whose selected inputs remain reserved until it is
/// broadcast, explicitly abandoned, observed by sync, or reclaimed by the
/// reservation TTL.
#[derive(Debug)]
pub struct SignedCoreTransaction {
    transaction: Transaction,
    fee: u64,
    /// Every concrete account that contributed funding inputs, in funding
    /// order (first supplies the change address). A pooled send spans the
    /// standard families and any DashPay receiving accounts, so the
    /// release/abandon paths must reconcile the reservation on EACH of them —
    /// key-wallet reserves per account, all stamped with the one build token.
    funding_accounts: Vec<AccountType>,
    /// The wallet's `last_processed_height` captured **inside** the funding
    /// critical section — the exact clock `set_current_height` stamped the
    /// selected inputs' reservation with, sampled *before* the (potentially
    /// slow, external) signer ran. The deferred-payment registry's age guard
    /// must baseline off this, not off a fresh `last_processed_height` sampled
    /// after signing: a slow external signer could otherwise let the wallet
    /// advance far enough that the token looks fresh while the reservation it
    /// covers has already aged toward key-wallet's TTL sweep.
    reservation_height: u32,
    /// The key-wallet [`ReservationToken`] stamped onto the selected inputs when
    /// `build_unsigned_reserved` reserved them, or `None` when the build took no
    /// reservation (no reservation set attached — not reached on the funded
    /// finalize path). Held so an abandoned or definitively-rejected send
    /// releases the reservation *owner-guarded*: after this build's inputs may
    /// have been swept by key-wallet's TTL and re-reserved by a concurrent build
    /// under a new token, releasing by outpoint alone would free that other
    /// build's inputs (the `dashpay/platform#4185` double-spend window).
    /// [`ManagedCoreFundsAccount::release_reservation_if_owner`] releases only
    /// inputs still owned by this token, closing that window.
    reservation_token: Option<ReservationToken>,
    /// The per-generation balance `Arc` of the wallet this payment was
    /// **finalized against** — captured from the originating `CoreWallet` inside
    /// `finalize_transaction`. It is the same unforgeable generation-identity
    /// marker [`CoreWallet::is_same_generation`] compares (a fresh `Arc` per
    /// wallet generation; two aliases of one generation share it, a
    /// remove-then-recreate under the same id gets a new one).
    ///
    /// The deferred-payment registry validates the wallet it is asked to bind
    /// this payment to against **this** marker before it mints a token
    /// ([`SignedPaymentRegistry::register`](crate::SignedPaymentRegistry::register)),
    /// so a caller cannot finalize through wallet A and then register/broadcast
    /// through an unrelated wallet B — the registry would otherwise treat B as
    /// the owner, submit A's transaction through B's broadcaster, and run B's
    /// cleanup while A's real reservation leaked until its TTL
    /// (`dashpay/platform#4185`).
    origin_generation: Arc<WalletGeneration>,
}

impl SignedCoreTransaction {
    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    pub fn fee(&self) -> u64 {
        self.fee
    }

    /// Every account that contributed funding inputs (funding order; the
    /// first supplied the change address). The release/abandon paths iterate
    /// these — the build's reservation lives per-account under one token.
    pub fn funding_accounts(&self) -> &[AccountType] {
        &self.funding_accounts
    }

    /// The `last_processed_height` the funding reservation was stamped with,
    /// captured in the funding critical section before signing. The deferred
    /// registry registers the token with this height so its age guard measures
    /// the reservation's true age rather than a post-signing sample.
    pub fn reservation_height(&self) -> u32 {
        self.reservation_height
    }

    /// The key-wallet [`ReservationToken`] the funding inputs were reserved
    /// under (`None` if the build reserved nothing). The broadcast/abandon
    /// release paths present it to
    /// [`ManagedCoreFundsAccount::release_reservation_if_owner`] so a rejected
    /// or abandoned send frees only reservations this build still owns.
    pub fn reservation_token(&self) -> Option<ReservationToken> {
        self.reservation_token
    }

    /// The per-generation balance `Arc` of the wallet this payment was finalized
    /// against — the unforgeable generation-identity marker the deferred-payment
    /// registry pointer-compares before binding the payment to a wallet (see
    /// [`origin_generation`](Self::origin_generation) field docs). Borrowed, not
    /// consumed, so the check can run before
    /// [`into_registered_parts`](Self::into_registered_parts) takes ownership.
    pub(crate) fn origin_generation(&self) -> &Arc<WalletGeneration> {
        &self.origin_generation
    }

    /// Consume this finalized transaction into the owned parts the deferred
    /// [`SignedPaymentRegistry`](crate::SignedPaymentRegistry) stores.
    ///
    /// Consuming (rather than cloning) is what enforces unique reservation
    /// ownership: `SignedCoreTransaction` is deliberately not `Clone`, so a
    /// finalize yields exactly one ownership object and the registry can be
    /// handed it exactly once — a caller cannot mint two live tokens that name
    /// the same held reservation (`dashpay/platform#4185`). The transaction,
    /// funding account, and reservation height are derived here, not supplied
    /// independently by the caller.
    pub(crate) fn into_registered_parts(self) -> RegisteredPaymentParts {
        RegisteredPaymentParts {
            transaction: self.transaction,
            funding_accounts: self.funding_accounts,
            reservation_height: self.reservation_height,
            reservation_token: self.reservation_token,
        }
    }
}

/// The owned facts the deferred-payment registry takes over when it registers a
/// finalized transaction. Produced only by
/// [`SignedCoreTransaction::into_registered_parts`], which consumes the
/// non-`Clone` ownership object exactly once.
pub(crate) struct RegisteredPaymentParts {
    pub(crate) transaction: Transaction,
    pub(crate) funding_accounts: Vec<AccountType>,
    pub(crate) reservation_height: u32,
    pub(crate) reservation_token: Option<ReservationToken>,
}

#[cfg(any(test, feature = "test-utils"))]
impl SignedCoreTransaction {
    /// Build a `SignedCoreTransaction` directly, for tests that need a finalized
    /// ownership object without running the full funding + signing pipeline
    /// (e.g. the registry and FFI destroy/lifecycle tests).
    ///
    /// `origin_generation` is the per-generation balance `Arc` the payment is to
    /// be treated as finalized against — a test that registers it must hand the
    /// registry the SAME generation
    /// ([`CoreWallet::test_generation_marker`](crate::CoreWallet::test_generation_marker)),
    /// exactly as the production path binds a token to the finalizing wallet.
    pub fn new_for_test(
        transaction: Transaction,
        fee: u64,
        funding_accounts: Vec<AccountType>,
        reservation_height: u32,
        reservation_token: Option<ReservationToken>,
        origin_generation: Arc<WalletGeneration>,
    ) -> Self {
        Self {
            transaction,
            fee,
            funding_accounts,
            reservation_height,
            reservation_token,
            origin_generation,
        }
    }
}

/// The funding sources a plain send pools by default: both standard families
/// plus every DashPay contact-receiving account, in this order — the FIRST
/// source (BIP44) supplies the change address, so change from a pooled send
/// always returns to the transparent primary account.
///
/// CoinJoin is deliberately absent (spending mixed outputs alongside
/// transparent ones links them and undoes the mixing — the same reasoning as
/// upstream `AccountTypePreference::DEFAULT`), and so are a contact's
/// watch-only `DashpayExternalAccount` coins, which
/// `AllDashpayReceivingFunds` excludes by construction upstream (it selects
/// only the receiving side the local seed can sign).
pub const SEND_FUNDING_SOURCES: [AccountTypePreference; 3] = [
    AccountTypePreference::BIP44,
    AccountTypePreference::BIP32,
    AccountTypePreference::AllDashpayReceivingFunds,
];

/// The funding sources an **asset lock** pools by default — the same set, and
/// for the same reasons, as [`SEND_FUNDING_SOURCES`]: an invitation, identity
/// registration or top-up draws from the union of the standard families and
/// every DashPay contact-receiving account, with change returning to BIP44.
///
/// Before this, an asset lock could only be funded from one BIP44 account, so a
/// wallet holding its balance across several accounts had to sweep them into
/// that account first and lock out of it — an extra on-chain hop, an extra fee,
/// and a transparent address reused for the privilege. Pooling ends that
/// sweep-then-lock shape.
///
/// CoinJoin is absent here for a second reason on top of the linkage one:
/// upstream rejects a CoinJoin source pooled with any other, and CoinJoin
/// asset-lock funding is drain-only. That flow keeps naming its single account,
/// through `AssetLockBuildAmount::DrainAll`.
pub const ASSET_LOCK_FUNDING_SOURCES: [AccountTypePreference; 3] = SEND_FUNDING_SOURCES;

/// The concrete accounts `preference` resolves to at `source_index` — the
/// platform mirror of key-wallet's private `account_types_for`: the single
/// account at `source_index` for the standard families, and every DashPay
/// receiving account the selector picks (which span their own indices) for a
/// DashPay source. A set selector matching nothing resolves to an empty list,
/// not an error — a wallet with no contacts still sends from its standard
/// accounts.
/// The accounts a pooled build will actually fund from, in funding order and
/// deduplicated: those `resolve_source_accounts` names AND that resolve on both
/// halves — the keys side (`wallet.accounts`) and the managed side
/// (`info.core_wallet.accounts`). An account present in only one is skipped,
/// because funding needs both.
///
/// `strict` reproduces the single-source contract: naming ONE account is an
/// explicit request for it, so a miss is an error rather than a silent skip. A
/// pooled call skips instead — a wallet without a BIP32 account or without
/// DashPay contacts still funds from what it has.
///
/// Shared by `finalize_transaction_with_options` and
/// `pooled_spendable_balance` so the set one reports can never drift from the
/// set the other funds.
pub(crate) fn resolved_funding_accounts(
    accounts: &ManagedAccountCollection,
    wallet: &Wallet,
    sources: &[AccountTypePreference],
    source_index: u32,
    strict: bool,
) -> Result<Vec<AccountType>, PlatformWalletError> {
    let mut seen: HashSet<AccountType> = HashSet::new();
    let mut resolved: Vec<AccountType> = Vec::new();
    for &preference in sources {
        for at in resolve_source_accounts(accounts, preference, source_index) {
            if !seen.insert(at) {
                continue;
            }
            if wallet.accounts.account_of_type(at).is_none()
                || accounts.funds_account(&at).is_none()
            {
                if strict {
                    return Err(PlatformWalletError::WalletNotFound(format!(
                        "wallet account {preference:?} #{source_index} not found"
                    )));
                }
                continue;
            }
            resolved.push(at);
        }
    }
    // A strict SET selector (a DashPay preference naming zero accounts) is a
    // miss too: the caller asked for exactly those funds.
    if strict && resolved.is_empty() {
        return Err(PlatformWalletError::WalletNotFound(format!(
            "wallet account {:?} #{source_index} not found",
            sources.first()
        )));
    }
    Ok(resolved)
}

pub(crate) fn resolve_source_accounts(
    accounts: &key_wallet::account::ManagedAccountCollection,
    preference: AccountTypePreference,
    source_index: u32,
) -> Vec<AccountType> {
    let (identity, friend) = match preference {
        AccountTypePreference::AllDashpayReceivingFunds => (None, None),
        AccountTypePreference::DashpayIdentityReceivingFunds { user_identity_id } => {
            (Some(user_identity_id), None)
        }
        AccountTypePreference::DashpayFriendshipReceivingFunds {
            user_identity_id,
            friend_identity_id,
        } => (Some(user_identity_id), Some(friend_identity_id)),
        _ => return preference.account_type(source_index).into_iter().collect(),
    };
    accounts
        .dashpay_receival_accounts
        .keys()
        .filter(|key| identity.is_none_or(|id| key.user_identity_id == id))
        .filter(|key| friend.is_none_or(|id| key.friend_identity_id == id))
        .map(|key| AccountType::DashpayReceivingFunds {
            index: key.index,
            user_identity_id: key.user_identity_id,
            friend_identity_id: key.friend_identity_id,
        })
        .collect()
}

impl<B: TransactionBroadcaster + ?Sized> CoreWallet<B> {
    /// The balance a pooled build could actually select from — the same
    /// accounts [`Self::finalize_transaction`] funds from, counting only UTXOs
    /// coin selection would accept.
    ///
    /// Hosts gate their amount entry on this. The wallet-level balance is a
    /// strict superset: it sums every funding account, CoinJoin included, and
    /// never consults a reservation set, so gating on it offers money the build
    /// then refuses — the shortfall surfacing as
    /// [`CorePooledInsufficientFunds`](PlatformWalletError::CorePooledInsufficientFunds)
    /// after the user has already committed to an amount.
    ///
    /// Missing sources are skipped, as in a pooled build: a wallet without a
    /// BIP32 account or without DashPay contacts still has a spendable balance.
    ///
    /// **Gross, not net of fee** — the sum a build may draw on, which is what an
    /// "available balance" line should show. A build needs `amount + fee`, so
    /// this is NOT the number to put behind a max/"send all" control: use
    /// [`Self::pooled_max_sendable`], which prices the fee off the inputs that
    /// spending it all would take.
    ///
    /// Reservations are NOT subtracted: key-wallet keeps each account's
    /// `ReservationSet` private, so reading it needs an accessor there and a pin
    /// bump. The figure is therefore optimistic by whatever another in-flight
    /// build currently holds — transient by construction, since a reservation is
    /// released when its spend is processed, on a definitive broadcast
    /// rejection, at the TTL, or on restart. The account-set mismatch this fixes
    /// is permanent, and was the whole of the shortfall in the report that
    /// prompted it (support ticket 32081: 0.0054 DASH offered as spendable
    /// against a 94 DASH balance, all of it CoinJoin).
    pub async fn pooled_spendable_balance(
        &self,
        sources: &[AccountTypePreference],
        source_index: u32,
    ) -> Result<u64, PlatformWalletError> {
        let manager = self.wallet_manager.read().await;
        let (wallet, info) = manager
            .get_wallet_and_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound("wallet not found".into()))?;
        let height = info.core_wallet.last_processed_height();

        // Same resolver, same strictness rule, as the funding path.
        let resolved = resolved_funding_accounts(
            &info.core_wallet.accounts,
            wallet,
            sources,
            source_index,
            sources.len() == 1,
        )?;

        let mut total: u64 = 0;
        for at in resolved {
            let Some(managed) = info.core_wallet.accounts.funds_account(&at) else {
                continue;
            };
            total += managed
                .spendable_utxos(height)
                .iter()
                .map(|utxo| utxo.value())
                .sum::<u64>();
        }
        Ok(total)
    }

    /// The largest amount a pooled build could actually pay out, net of the fee
    /// that spending it costs — the figure a "send max" control must use.
    ///
    /// [`Self::pooled_spendable_balance`] is the gross sum a build may draw on.
    /// Entering that verbatim as an amount fails: coin selection clears its
    /// `total_available >= amount` check and then cannot cover `amount + fee`,
    /// so the shortfall this API exists to remove simply moves from the CoinJoin
    /// edge to the max-amount edge. The fee is not a rounding concern here —
    /// with `use_only_added_inputs` draining accounts hundreds of UTXOs at a
    /// time, the input count, and so the fee, runs far above dust.
    ///
    /// Computed the way "spend everything" actually builds: one output, no
    /// change, sizes from key-wallet's own [`estimate_tx_size`] and
    /// [`FeeRate`] rather than sizes re-declared here. A UTXO whose value does
    /// not cover the fee its own input adds is left out — including it would
    /// lower the answer — so this is a true maximum, not a subtraction from the
    /// gross figure.
    ///
    /// `fee_rate` defaults to [`FeeRate::normal()`], which is what
    /// `TransactionBuilder::new` starts from; pass the host's rate if it sets
    /// one, or the answer will not match the build.
    ///
    /// Two limits are deliberately NOT modelled, both of which can only make the
    /// real ceiling lower: reservations held by another in-flight build (see
    /// [`Self::pooled_spendable_balance`]), and the standard-transaction input
    /// cap, which key-wallet keeps private and which a pool above it must reach
    /// through batched sends rather than a larger amount.
    pub async fn pooled_max_sendable(
        &self,
        sources: &[AccountTypePreference],
        source_index: u32,
        fee_rate: Option<FeeRate>,
    ) -> Result<u64, PlatformWalletError> {
        let fee_rate = fee_rate.unwrap_or_else(FeeRate::normal);
        // Per-input and per-output costs derived from key-wallet's estimator by
        // difference, so this crate never re-declares a size that could drift
        // from the one the build charges.
        let empty = estimate_tx_size(0, 1, false);
        let per_input = estimate_tx_size(1, 1, false).saturating_sub(empty);

        let manager = self.wallet_manager.read().await;
        let (wallet, info) = manager
            .get_wallet_and_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound("wallet not found".into()))?;
        let height = info.core_wallet.last_processed_height();

        let resolved = resolved_funding_accounts(
            &info.core_wallet.accounts,
            wallet,
            sources,
            source_index,
            sources.len() == 1,
        )?;

        // A UTXO earns its place only if it brings in more than its own input
        // costs at this rate; the rest are dead weight and are dropped.
        let input_cost = fee_rate.calculate_fee(per_input);
        let mut selected: u64 = 0;
        let mut count: usize = 0;
        for at in resolved {
            let Some(managed) = info.core_wallet.accounts.funds_account(&at) else {
                continue;
            };
            for utxo in managed.spendable_utxos(height) {
                if utxo.value() > input_cost {
                    selected += utxo.value();
                    count += 1;
                }
            }
        }
        if count == 0 {
            return Ok(0);
        }

        let fee = fee_rate.calculate_fee(estimate_tx_size(count, 1, false));
        Ok(selected.saturating_sub(fee))
    }

    /// Consume a configured builder, atomically fund and reserve its selected
    /// inputs, then sign without holding the wallet-manager lock.
    pub async fn finalize_transaction<S: TransactionSigner + ?Sized + Sync>(
        &self,
        builder: TransactionBuilder,
        sources: &[AccountTypePreference],
        source_index: u32,
        signer: &S,
    ) -> Result<SignedCoreTransaction, PlatformWalletError> {
        self.finalize_transaction_with_options(builder, sources, source_index, signer, false)
            .await
    }

    /// `reservation_only` funds through
    /// [`TransactionBuilder::add_funding_reservation_only`]: the sources take on
    /// their reservation bookkeeping but offer no candidates, so the build spends
    /// only the inputs already seeded on the builder.
    ///
    /// A caller draining an account in batches under the standard-transaction
    /// input limit needs it — ordinary funding offers the whole account on top of
    /// the batch, so every batch trips the cap and an account above it can never
    /// be drained.
    pub async fn finalize_transaction_with_options<S: TransactionSigner + ?Sized + Sync>(
        &self,
        builder: TransactionBuilder,
        // The funding sources to POOL, in order — the first supplies the
        // change address. A plain send passes [`SEND_FUNDING_SOURCES`]
        // (BIP44 + BIP32 + every DashPay receiving account); a single-element
        // list reproduces the old one-account behavior, including its strict
        // account-not-found error. `source_index` addresses the standard
        // families; DashPay set selectors span their own indices.
        sources: &[AccountTypePreference],
        source_index: u32,
        signer: &S,
        reservation_only: bool,
    ) -> Result<SignedCoreTransaction, PlatformWalletError> {
        let primary = *sources.first().ok_or_else(|| {
            PlatformWalletError::TransactionBuild("no funding sources named".into())
        })?;
        // A single-source call is an explicit request for THAT account: keep
        // the strict not-found error the one-account API had. A pooled call
        // skips missing sources (a wallet without a BIP32 account or without
        // DashPay contacts still sends) and errors only if NOTHING funds.
        let strict = sources.len() == 1;

        let (unsigned, fee, selected, paths, height, reservation_token, funding_accounts) = {
            let mut manager = self.wallet_manager.write().await;
            let (wallet, info) = manager
                .get_wallet_and_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound("wallet not found".into()))?;

            let height = info.core_wallet.last_processed_height();

            // Fund from every resolved account, mirroring key-wallet's own
            // multi-source fold (`transaction_building::fund`): dedup overlapping
            // sources (funding an account twice would offer its UTXOs to
            // selection twice), collect the address→path map per contributing
            // account for the external signer, and let the FIRST source's first
            // account supply the change address. `add_funding` observes each
            // account's ReservationSet and `build_unsigned_reserved` records the
            // pooled selection AND returns the ONE token stamped onto every
            // reserved input across accounts. There is no await in this section
            // and the manager write guard prevents another finalizer
            // interleaving. The token rides in `SignedCoreTransaction` so a
            // later abandon or rejected broadcast releases *only* the inputs
            // this build still owns, even if a TTL sweep re-reserved them under
            // a new token meanwhile (`dashpay/platform#4185`).
            let mut builder = builder.set_current_height(height);
            // Accounts that took on this build's reservation bookkeeping, in
            // funding order — i.e. the ones a failure path must release. Under
            // `reservation_only` nothing is offered to selection at all; without
            // it these are also the accounts whose UTXOs were offered. Either
            // way this is NOT the list of accounts that end up contributing
            // inputs — selection may take nothing from most of them — so it
            // drives build-time cleanup only, and the contributor list stored on
            // the transaction is derived from the selected inputs below.
            let mut offered_accounts: Vec<AccountType> = Vec::new();
            let mut paths: HashMap<Address, DerivationPath> = HashMap::new();
            let resolved = resolved_funding_accounts(
                &info.core_wallet.accounts,
                wallet,
                sources,
                source_index,
                strict,
            )?;
            {
                for at in resolved {
                    let (Some(account), Some(managed)) = (
                        wallet.accounts.account_of_type(at),
                        info.core_wallet.accounts.funds_account_mut(&at),
                    ) else {
                        continue;
                    };
                    for utxo in managed.utxos.values() {
                        if let Some(path) = managed.address_derivation_path(&utxo.address) {
                            paths.insert(utxo.address.clone(), path);
                        }
                    }
                    builder = if reservation_only {
                        builder.add_funding_reservation_only(managed, account)
                    } else {
                        builder.add_funding(managed, account)
                    };
                    offered_accounts.push(at);
                }
            }
            if offered_accounts.is_empty() {
                return Err(PlatformWalletError::WalletNotFound(format!(
                    "no funding account of any named source at index {source_index}"
                )));
            }

            let funding_context = if strict {
                FundingContext::Single {
                    preference: primary,
                    index: source_index,
                }
            } else {
                FundingContext::Pooled(sources)
            };
            let (unsigned, fee, reservation_token) = builder
                .build_unsigned_reserved()
                .map_err(|error| map_builder_error(error, funding_context))?;

            // Release across every contributing account on the error paths
            // below: the pooled reservation lives per account under the one
            // token, and we are still inside the write guard (no sweep can
            // interleave), so the plain by-outpoint release is exact.
            macro_rules! release_all {
                ($accounts:expr, $collection:expr, $unsigned:expr) => {
                    for at in $accounts.iter() {
                        if let Some(managed) = $collection.funds_account_mut(at) {
                            managed.release_reservation($unsigned);
                        }
                    }
                };
            }

            // Refuse a selection that picked an input pinned by an IN-FLIGHT
            // BROADCAST. A pinned input is normally still reserved and never
            // reaches selection; getting here means this build's own
            // selection swept that dispatch's aged reservation (catch-up
            // advanced the clock past key-wallet's TTL while the dispatch
            // was suspended pre-submission) and re-reserved the input under
            // our token. Completing this build would race the pinned,
            // already-signed transaction on the wire — the double-spend the
            // dispatch-side age guard exists to prevent
            // (`WalletGeneration::pin_in_broadcast`). Still under the write
            // guard, so the check is atomic with our reservation and the
            // release is exact.
            //
            // The refusal is TYPED (`InputMidBroadcast`, carrying the
            // conflicting outpoint) rather than a build-failure string: the
            // request itself is sound and can be re-attempted once the fenced
            // dispatch's outcome is reconciled (see the variant docs for the
            // duplicate-payment hazard in "retry unchanged"), and callers
            // should not have to substring-match prose to tell it apart
            // (`dashpay/platform#4309`).
            if let Some(outpoint) = info.generation.in_broadcast_conflict(&unsigned) {
                release_all!(offered_accounts, info.core_wallet.accounts, &unsigned);
                return Err(PlatformWalletError::InputMidBroadcast { outpoint });
            }

            // Map every selected input back to the account that owns it. That
            // mapping — not the offered list — is what the transaction carries:
            // selection routinely takes nothing from most offered sources, and
            // a `funding_accounts` naming every contact would make release and
            // registry bookkeeping scale with the address book while claiming
            // contributions that never happened.
            let mut contributors: Vec<AccountType> = Vec::new();
            let selected: Vec<Utxo> = match unsigned
                .input
                .iter()
                .map(|input| {
                    offered_accounts
                        .iter()
                        .find_map(|at| {
                            let utxo =
                                info.core_wallet.accounts.funds_account(at).and_then(
                                    |managed| managed.utxos.get(&input.previous_output),
                                )?;
                            Some((*at, utxo.clone()))
                        })
                        .map(|(at, utxo)| {
                            if !contributors.contains(&at) {
                                contributors.push(at);
                            }
                            utxo
                        })
                        .ok_or_else(|| {
                            PlatformWalletError::TransactionBuild(format!(
                                "selected input {} is no longer in any funding account",
                                input.previous_output
                            ))
                        })
                })
                .collect::<Result<_, _>>()
            {
                Ok(selected) => selected,
                Err(error) => {
                    release_all!(offered_accounts, info.core_wallet.accounts, &unsigned);
                    return Err(error);
                }
            };
            // Duplicate prevouts make a transaction invalid (Core rejects it),
            // and additive funding is the shape that can produce them — an
            // outpoint seeded by `add_inputs` that a funding account also
            // offers. `add_funding` filters those (rust-dashcore#931); this
            // asserts the invariant here too rather than handing a signer, and
            // then the network, a transaction that cannot confirm.
            let mut prevouts: HashSet<OutPoint> = HashSet::new();
            if let Some(duplicate) = unsigned
                .input
                .iter()
                .find(|input| !prevouts.insert(input.previous_output))
            {
                let error = PlatformWalletError::TransactionBuild(format!(
                    "built transaction spends {} twice",
                    duplicate.previous_output
                ));
                release_all!(offered_accounts, info.core_wallet.accounts, &unsigned);
                return Err(error);
            }
            // The per-account path collection above covered every UTXO offered
            // to selection, so every selected input's address must be present.
            if let Some(missing) = selected
                .iter()
                .find(|utxo| !paths.contains_key(&utxo.address))
            {
                let error = PlatformWalletError::TransactionBuild(format!(
                    "no derivation path for selected input address {}",
                    missing.address
                ));
                release_all!(offered_accounts, info.core_wallet.accounts, &unsigned);
                return Err(error);
            }

            (
                unsigned,
                fee,
                selected,
                paths,
                height,
                reservation_token,
                contributors,
            )
        };

        let signed = match signer
            .sign_tx(unsigned.clone(), selected, move |address| {
                paths.get(&address).cloned()
            })
            .await
        {
            Ok(signed) => signed,
            Err(error) => {
                // Signing awaited an (external) signer with the manager lock
                // dropped, so key-wallet's TTL sweep could have reclaimed this
                // build's reservation and a concurrent build re-taken the same
                // inputs under a new token. Release owner-guarded so we free
                // only what this build still owns.
                self.release_transaction_reservation(
                    &funding_accounts,
                    &unsigned,
                    reservation_token,
                )
                .await;
                return Err(PlatformWalletError::TransactionBuild(error.to_string()));
            }
        };

        Ok(SignedCoreTransaction {
            transaction: signed,
            fee,
            funding_accounts,
            reservation_height: height,
            reservation_token,
            // Capture the finalizing wallet's generation identity so the
            // deferred registry can refuse to bind this payment to any other
            // wallet (`dashpay/platform#4185`).
            origin_generation: Arc::clone(self.generation()),
        })
    }

    /// Release a finalized transaction that the caller has chosen not to send.
    ///
    /// # Reservation age guard
    ///
    /// This is the abandon/free arm of the finalized-transaction handle —
    /// including the FFI broadcast/abandon *failure* paths (invalid or
    /// wrong-generation wallet handle) that route their cleanup here, and the
    /// host-language deinit/GC backstop
    /// (`core_wallet_signed_transaction_free`). A pinned handle can reach it
    /// long after `finalize`, so it honors the **same** age bound as
    /// [`broadcast_finalized_transaction`](Self::broadcast_finalized_transaction),
    /// off the same shared [`reservation_expired`] predicate and the same
    /// `last_processed_height` clock.
    ///
    /// With the build's owner token present the release is owner-guarded
    /// (`release_reservation_if_owner`), which is safe at ANY age: it frees the
    /// inputs only while this build still owns them and no-ops once key-wallet's
    /// TTL sweep or a re-reservation transferred ownership. Between
    /// [`RESERVATION_MAX_AGE_BLOCKS`](crate::wallet::reservations::RESERVATION_MAX_AGE_BLOCKS)
    /// and the TTL the reservation is typically STILL this build's, so an aged
    /// abandon must still release — skipping would strand the inputs for
    /// several more blocks while the host has already discarded the payment.
    /// Only a token-less build (never reached on the funded finalize path)
    /// honours the age bound and skips: its only release primitive is the
    /// unguarded by-outpoint form, which after a sweep could free a newer
    /// build's reservation. This mirrors the deferred registry's
    /// `reconcile_removed_entry` policy exactly.
    pub async fn abandon_transaction(&self, transaction: &SignedCoreTransaction) {
        if transaction.reservation_token.is_none()
            && reservation_expired(
                transaction.reservation_height,
                self.last_processed_height().await,
            )
        {
            // Aged, and no owner token to guard the release: the outpoint may
            // have been swept and re-reserved by an unrelated build. Leave it
            // for key-wallet's TTL; releasing by outpoint could free that newer
            // reservation.
            return;
        }
        self.release_transaction_reservation(
            &transaction.funding_accounts,
            &transaction.transaction,
            transaction.reservation_token,
        )
        .await;
    }

    /// Release the funding reservation `transaction` holds, bound to this
    /// handle's own wallet *generation*.
    ///
    /// `token` is the [`ReservationToken`] the build stamped onto the inputs
    /// (`SignedCoreTransaction::reservation_token`). When present the release is
    /// *owner-guarded* — it frees only inputs still owned by that token, so a
    /// reservation key-wallet's TTL swept and a concurrent build re-took is left
    /// untouched (`dashpay/platform#4185`). When `None` (the build reserved
    /// nothing) it falls back to the unconditional by-outpoint release; that
    /// path is never reached for a funded finalize, which always reserves.
    pub(crate) async fn release_transaction_reservation(
        &self,
        // Every account the build funded from — the pooled reservation lives
        // per account under the one `token`, so each must reconcile.
        accounts: &[AccountType],
        transaction: &Transaction,
        token: Option<ReservationToken>,
    ) {
        // Validate the generation AND mutate the `ReservationSet` under one
        // manager-lock hold. `ReservationSet::release` removes an outpoint
        // unconditionally, and it is reached via `wallet_id` — an identity that a
        // remove-then-recreate under the same id preserves. Between a token's
        // generation validation and this cleanup the wallet could therefore have
        // been re-created, and an unguarded release-by-outpoint could then free
        // the NEW generation's reservation on the same input.
        //
        // Binding the release to this handle's own generation closes that
        // window: the wallet registered under `wallet_id` is the same generation
        // as `self` iff their per-generation balance `Arc`s are pointer-equal
        // (`wallet_id` + the shared manager `Arc` are both preserved across a
        // recreation; only the balance `Arc` is fresh — the same identity
        // `is_same_generation` uses). A read lock is enough and makes this atomic
        // against recreation: a recreate needs the manager *write* lock, so it
        // cannot interleave between the pointer check and the release below.
        let manager = self.wallet_manager.read().await;
        let Some(info) = manager.get_wallet_info(&self.wallet_id) else {
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id),
                ?accounts,
                "could not release finalized Core transaction reservation: wallet not found"
            );
            return;
        };
        if !Arc::ptr_eq(&info.generation, self.generation()) {
            // The wallet under this id is a different (re-created) generation:
            // releasing by outpoint could free ITS reservation. Leave it — the
            // original generation's reservation ceased to exist with it.
            tracing::warn!(
                wallet_id = %hex::encode(self.wallet_id),
                ?accounts,
                "skipping reservation release: wallet was re-created under the same id \
                 (different generation) since the token was minted"
            );
            return;
        }
        for at in accounts {
            match info.core_wallet.accounts.funds_account(at) {
                // Owner-guarded when the build stamped a token: even within this
                // generation, a TTL sweep between build and release could have
                // re-reserved the same outpoints under a new token, and an
                // unconditional release would free that newer reservation. With
                // the token key-wallet frees only inputs this build still owns
                // in THIS account's set. `None` (no reservation taken) falls
                // back to the unconditional release.
                Some(managed) => match token {
                    Some(token) => managed.release_reservation_if_owner(transaction, token),
                    None => managed.release_reservation(transaction),
                },
                None => tracing::warn!(
                    wallet_id = %hex::encode(self.wallet_id),
                    account = %at,
                    "could not release finalized Core transaction reservation: account not found"
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use dashcore::secp256k1::{ecdsa, PublicKey};
    use dashcore::{Address as DashAddress, Network};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::signer::{Signer, SignerMethod};
    use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
    use key_wallet::DerivationPath;
    use tokio::sync::Barrier;

    use crate::broadcaster::TransactionBroadcaster;
    use crate::test_support::{
        funded_wallet_manager, funded_wallet_manager_dual_standard,
        funded_wallet_manager_with_contact, AlwaysMaybeSentBroadcaster, AlwaysOkBroadcaster,
        AlwaysRejectedBroadcaster, WalletSigner,
    };
    use crate::wallet::core::CoreWallet;
    use crate::PlatformWalletError;

    fn preference(account_type: StandardAccountType) -> AccountTypePreference {
        match account_type {
            StandardAccountType::BIP44Account => AccountTypePreference::BIP44,
            StandardAccountType::BIP32Account => AccountTypePreference::BIP32,
        }
    }

    async fn core<B: TransactionBroadcaster>(
        account_type: StandardAccountType,
        broadcaster: Arc<B>,
    ) -> (CoreWallet<B>, WalletSigner) {
        let (manager, wallet_id, balance, signer) = funded_wallet_manager(account_type).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        (
            CoreWallet::new(sdk, manager, wallet_id, broadcaster, balance),
            signer,
        )
    }

    /// THE POOLED SEND (rust-dashcore#925/#929): `SEND_FUNDING_SOURCES` draws
    /// from BOTH standard families when neither alone covers the payment,
    /// records every contributing account on the ownership object, tolerates
    /// the wallet having no DashPay accounts (the `AllDashpayReceivingFunds`
    /// selector contributes nothing rather than erroring), and an abandon
    /// releases the reservation on EVERY contributing account so an immediate
    /// identical rebuild succeeds.
    #[tokio::test]
    async fn pooled_send_spans_families_and_abandon_releases_all() {
        let (manager, wallet_id, generation, signer) =
            funded_wallet_manager_dual_standard(&[700_000], &[700_000]).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            manager,
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );

        // 1_000_000 exceeds either family's 700_000, so selection must pool.
        let finalized = core
            .finalize_transaction(
                payment_builder(40),
                &crate::SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await
            .expect("pooled finalize must fund from both standard families");
        assert_eq!(
            finalized.funding_accounts().len(),
            2,
            "both standard families must contribute (and be recorded for release)"
        );
        assert!(
            finalized.transaction().input.len() >= 2,
            "a payment above either family's balance needs inputs from both"
        );

        // Abandon must release BOTH accounts' reservations: an identical
        // rebuild can only succeed if every pooled input returned to the pool.
        core.abandon_transaction(&finalized).await;
        let rebuilt = core
            .finalize_transaction(
                payment_builder(41),
                &crate::SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await
            .expect("abandon must release every contributing account's reservation");
        core.abandon_transaction(&rebuilt).await;
    }

    /// The DashPay half of `SEND_FUNDING_SOURCES`, end to end: a payment larger
    /// than BIP44 alone holds must reach into a real `DashpayReceivingFunds`
    /// contact account, sign its inputs (DIP-15 `Normal256` derivation path),
    /// and record that account for release. Without this, every lookup in the
    /// pooled path could resolve `None` for contact accounts and the feature
    /// would silently degrade to a BIP44+BIP32 send.
    #[tokio::test]
    async fn pooled_send_spends_dashpay_contact_funds() {
        let (manager, wallet_id, generation, signer, contact_account) =
            funded_wallet_manager_with_contact(&[700_000], &[700_000]).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            manager,
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );

        let finalized = core
            .finalize_transaction(
                payment_builder(60),
                &crate::SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await
            .expect("pooled finalize must reach contact funds");
        assert!(
            finalized.funding_accounts().contains(&contact_account),
            "the contact account must be recorded as a contributor, got {:?}",
            finalized.funding_accounts()
        );
        assert!(
            finalized
                .transaction()
                .input
                .iter()
                .all(|input| !input.script_sig.is_empty()),
            "every pooled input must be signed, including the DIP-15 contact input"
        );
        // BIP32 account 0 exists on the test wallet but holds nothing, so it is
        // OFFERED to selection and contributes no input. Contributors are
        // derived from the selected prevouts, so it must not be recorded —
        // otherwise release and registry bookkeeping would claim accounts that
        // never funded anything and scale with the address book.
        assert!(
            !finalized
                .funding_accounts()
                .contains(&key_wallet::account::AccountType::Standard {
                    index: 0,
                    standard_account_type: StandardAccountType::BIP32Account,
                }),
            "an offered-but-unselected account must not be recorded as a contributor, got {:?}",
            finalized.funding_accounts()
        );

        // And releasing must reach the contact account too.
        core.abandon_transaction(&finalized).await;
        let rebuilt = core
            .finalize_transaction(
                payment_builder(61),
                &crate::SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await
            .expect("abandon must release the contact account's reservation");
        core.abandon_transaction(&rebuilt).await;
    }

    /// A single-source call keeps the strict one-account contract: naming a
    /// family with no funded account at the index errors instead of silently
    /// funding from elsewhere.
    #[tokio::test]
    async fn single_source_missing_account_still_errors() {
        let (core, signer) = core(
            StandardAccountType::BIP44Account,
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        let result = core
            .finalize_transaction(
                payment_builder(50),
                &[AccountTypePreference::BIP44],
                7, // no account at index 7
                &signer,
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::WalletNotFound(_))),
            "explicit single-source misses must stay strict, got {result:?}"
        );
    }

    /// The coupling this getter exists to enforce: the total must come from the
    /// same accounts `finalize_transaction` funds from, and the strictness rule
    /// must match too. Nothing else pins that — the two are separate code paths
    /// over the same rule, which is exactly how the host's hand-copy drifted in
    /// the first place (support ticket 32081).
    #[tokio::test]
    async fn pooled_spendable_balance_reports_the_funding_set_and_stays_strict() {
        let (manager, wallet_id, generation, _signer) =
            funded_wallet_manager_dual_standard(&[700_000], &[700_000]).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            manager,
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );

        // The pooled selector sums BOTH standard families — the same set
        // `pooled_send_spans_families_and_abandon_releases_all` proves a build
        // draws on. An account counted here but skipped by the build (or the
        // reverse) breaks this number.
        assert_eq!(
            core.pooled_spendable_balance(&crate::SEND_FUNDING_SOURCES, 0)
                .await
                .expect("pooled balance"),
            1_400_000,
            "the pooled figure must be both families, and nothing else"
        );

        // A single-family selector sees only its own family.
        assert_eq!(
            core.pooled_spendable_balance(&[AccountTypePreference::BIP44], 0)
                .await
                .expect("bip44 balance"),
            700_000
        );

        // Strictness matches `single_source_missing_account_still_errors`: a
        // single-source miss is an error here too, never a silent `Ok(0)` that
        // the host would render as "insufficient funds" while the matching
        // `finalize` call says "no such account".
        let missed = core
            .pooled_spendable_balance(&[AccountTypePreference::BIP44], 7)
            .await;
        assert!(
            matches!(missed, Err(PlatformWalletError::WalletNotFound(_))),
            "a single-source miss must error as it does in finalize, got {missed:?}"
        );
    }

    /// The fee question, settled by building rather than by arithmetic: the
    /// gross figure is NOT an amount a build accepts, and the net one is.
    ///
    /// This is the failure review described — a host wiring "send max" to the
    /// gross balance and passing it through verbatim relocates the shortfall
    /// from the CoinJoin edge to the max-amount edge. Asserting only that
    /// `max < gross` would not catch an estimate that is merely close; running
    /// both numbers through `finalize_transaction` does.
    #[tokio::test]
    async fn pooled_max_sendable_is_an_amount_a_build_accepts() {
        let (manager, wallet_id, generation, signer) =
            funded_wallet_manager_dual_standard(&[700_000], &[700_000]).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            manager,
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );

        let gross = core
            .pooled_spendable_balance(&crate::SEND_FUNDING_SOURCES, 0)
            .await
            .expect("gross balance");
        let max = core
            .pooled_max_sendable(&crate::SEND_FUNDING_SOURCES, 0, None)
            .await
            .expect("max sendable");
        assert!(
            max < gross,
            "the fee has to come off somewhere: gross {gross}, max {max}"
        );

        let spend = |amount: u64, tag: u8| {
            TransactionBuilder::new().add_output(
                &DashAddress::dummy(Network::Testnet, usize::from(tag)),
                amount,
            )
        };

        // The gross figure is unbuildable — the whole of review's point.
        let over = core
            .finalize_transaction(spend(gross, 70), &crate::SEND_FUNDING_SOURCES, 0, &signer)
            .await;
        assert!(
            over.is_err(),
            "the gross balance must not be offerable as an amount, got {over:?}"
        );

        // The net figure builds, and takes every UTXO with it.
        let finalized = core
            .finalize_transaction(spend(max, 71), &crate::SEND_FUNDING_SOURCES, 0, &signer)
            .await
            .expect("max sendable must be an amount a build accepts");
        assert_eq!(
            finalized.transaction().input.len(),
            2,
            "spending the maximum must draw on both funded accounts"
        );
        core.abandon_transaction(&finalized).await;
    }

    /// A UTXO that does not cover the fee its own input adds must be left out:
    /// including it would LOWER the answer, so the maximum is a selection
    /// problem, not a subtraction from the gross figure. Without this the doc's
    /// "true maximum" claim is untested.
    #[tokio::test]
    async fn pooled_max_sendable_drops_utxos_that_cost_more_than_they_bring() {
        let sdk = || Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));

        let (manager, wallet_id, generation, _signer) =
            funded_wallet_manager_dual_standard(&[700_000], &[700_000]).await;
        let plain = CoreWallet::new(
            sdk(),
            manager,
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );
        let baseline = plain
            .pooled_max_sendable(&crate::SEND_FUNDING_SOURCES, 0, None)
            .await
            .expect("max sendable");

        // The same wallet plus one UTXO worth far less than the ~296 duffs its
        // input costs at the default rate.
        let (manager, wallet_id, generation, _signer) =
            funded_wallet_manager_dual_standard(&[700_000, 100], &[700_000]).await;
        let with_dust = CoreWallet::new(
            sdk(),
            manager,
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );

        assert_eq!(
            with_dust
                .pooled_spendable_balance(&crate::SEND_FUNDING_SOURCES, 0)
                .await
                .expect("gross balance"),
            1_400_100,
            "the gross figure counts every spendable UTXO, dust included"
        );
        assert_eq!(
            with_dust
                .pooled_max_sendable(&crate::SEND_FUNDING_SOURCES, 0, None)
                .await
                .expect("max sendable"),
            baseline,
            "a UTXO that cannot pay for its own input must not move the maximum"
        );
    }

    /// The DashPay leg, and the exclusion the ticket turned on: contact funds
    /// count toward the pooled figure, CoinJoin does not.
    #[tokio::test]
    async fn pooled_spendable_balance_counts_contacts_and_excludes_coinjoin() {
        let (manager, wallet_id, generation, _signer, _contact) =
            funded_wallet_manager_with_contact(&[700_000], &[700_000]).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            manager,
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );
        assert_eq!(
            core.pooled_spendable_balance(&crate::SEND_FUNDING_SOURCES, 0)
                .await
                .expect("pooled balance"),
            1_400_000,
            "AllDashpayReceivingFunds must contribute the contact account's funds"
        );

        // A wallet whose money is ALL in CoinJoin: the wallet-level balance the
        // host used to gate on reports the full 10_000_000, the pooled figure
        // reports nothing, and the build agrees with the pooled figure. That
        // gap, on a 94 DASH wallet, is the whole of ticket 32081.
        let (manager, wallet_id, generation, _signer) =
            crate::test_support::funded_coinjoin_wallet_manager().await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let coinjoin_only = CoreWallet::new(
            sdk,
            manager,
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );
        assert_eq!(
            coinjoin_only
                .pooled_spendable_balance(&crate::SEND_FUNDING_SOURCES, 0)
                .await
                .expect("pooled balance"),
            0,
            "CoinJoin is excluded from the send pool, so it must not be offered as spendable"
        );
    }

    fn payment_builder(tag: u8) -> TransactionBuilder {
        TransactionBuilder::new().add_output(
            &DashAddress::dummy(Network::Testnet, usize::from(tag)),
            1_000_000,
        )
    }

    /// `reservation_only` end to end IN THIS WORKSPACE. key-wallet #994 covers
    /// `add_funding_reservation_only` on its own side, but nothing here proved
    /// the flag survives the crossing: it travels an FFI struct field, a
    /// finalizer bool, and a key-wallet call, and a refactor that drops it
    /// anywhere along that path leaves every test in this repo green while
    /// silently reintroducing the >500-input build this branch exists to fix.
    ///
    /// The account holds two UTXOs; the builder is seeded with exactly one.
    /// Under the flag the build must spend that one and nothing else — and a
    /// payment only the pair could cover must FAIL, which is the assertion that
    /// actually proves the second UTXO was never offered to selection. The
    /// unflagged control shows the same wallet funds it happily.
    #[tokio::test]
    async fn reservation_only_finalize_spends_only_the_seeded_inputs() {
        let account_type = StandardAccountType::BIP44Account;
        let (manager, wallet_id, generation, signer) =
            crate::test_support::funded_wallet_manager_with_outputs(
                account_type,
                &[900_000, 900_000],
            )
            .await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            manager,
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );

        // One of the account's two spendable UTXOs, chosen by outpoint so the
        // pick is deterministic across runs.
        let seeded = {
            use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

            let wm = core.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&core.wallet_id())
                .expect("wallet present in manager");
            let height = info.core_wallet.last_processed_height();
            let account = info
                .core_wallet
                .first_bip44_managed_account()
                .expect("bip44 managed account");
            let mut utxos: Vec<key_wallet::Utxo> = account
                .spendable_utxos(height)
                .into_iter()
                .cloned()
                .collect();
            assert_eq!(utxos.len(), 2, "fixture must fund two separate UTXOs");
            utxos.sort_by_key(|u| u.outpoint);
            utxos.remove(0)
        };

        // A payment neither UTXO covers alone. With the flag the unseeded one
        // is not a candidate, so selection must come up short.
        let err = core
            .finalize_transaction_with_options(
                payment_builder(60).add_inputs([seeded.clone()]),
                &[preference(account_type)],
                0,
                &signer,
                true,
            )
            .await
            .expect_err("the account's other UTXO must not be reachable under the flag");
        assert!(
            matches!(err, PlatformWalletError::CoreInsufficientFunds { .. }),
            "expected a funding shortfall, got {err:?}"
        );

        // Within what the seeded input alone covers: exactly that input, and
        // the account's own change address still comes from its bookkeeping.
        let finalized = core
            .finalize_transaction_with_options(
                TransactionBuilder::new()
                    .add_output(&DashAddress::dummy(Network::Testnet, 61), 500_000)
                    .add_inputs([seeded.clone()]),
                &[preference(account_type)],
                0,
                &signer,
                true,
            )
            .await
            .expect("the seeded input alone covers this payment");
        let inputs = &finalized.transaction().input;
        assert_eq!(inputs.len(), 1, "only the seeded input may be spent");
        assert_eq!(
            inputs[0].previous_output, seeded.outpoint,
            "the spent input must be the seeded one"
        );
        core.abandon_transaction(&finalized).await;

        // Control: the same wallet, the same payment, no flag — proof the
        // shortfall above came from the restriction and not from the fixture.
        let pooled = core
            .finalize_transaction_with_options(
                payment_builder(62),
                &[preference(account_type)],
                0,
                &signer,
                false,
            )
            .await
            .expect("without the flag both UTXOs fund the payment");
        assert_eq!(
            pooled.transaction().input.len(),
            2,
            "the unflagged build must reach for both UTXOs"
        );
        core.abandon_transaction(&pooled).await;
    }

    #[tokio::test]
    async fn concurrent_same_account_finalizers_cannot_reserve_the_same_input() {
        let account_type = StandardAccountType::BIP44Account;
        let (core, signer) = core(account_type, Arc::new(AlwaysOkBroadcaster)).await;
        let core = Arc::new(core);
        let barrier = Arc::new(Barrier::new(3));
        let mut tasks = Vec::new();
        for tag in [10, 11] {
            let core = Arc::clone(&core);
            let signer = signer.clone();
            let barrier = Arc::clone(&barrier);
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                core.finalize_transaction(
                    payment_builder(tag),
                    &[preference(account_type)],
                    0,
                    &signer,
                )
                .await
            }));
        }
        barrier.wait().await;
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await);
        }
        let successes = results
            .iter()
            .filter(|result| matches!(result, Ok(Ok(_))))
            .count();
        let build_failures = results
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    Ok(Err(PlatformWalletError::CoreInsufficientFunds { .. }))
                )
            })
            .count();
        assert_eq!((successes, build_failures), (1, 1));
    }

    struct FailingSigner;

    #[async_trait]
    impl Signer for FailingSigner {
        type Error = &'static str;

        fn supported_methods(&self) -> &[SignerMethod] {
            &[SignerMethod::Digest]
        }

        async fn sign_ecdsa(
            &self,
            _path: &DerivationPath,
            _sighash: [u8; 32],
        ) -> Result<(ecdsa::Signature, PublicKey), Self::Error> {
            Err("intentional signer failure")
        }

        async fn public_key(&self, _path: &DerivationPath) -> Result<PublicKey, Self::Error> {
            Err("intentional signer failure")
        }
    }

    #[tokio::test]
    async fn validation_and_signing_failures_do_not_strand_reservations() {
        let account_type = StandardAccountType::BIP44Account;
        let (core, signer) = core(account_type, Arc::new(AlwaysOkBroadcaster)).await;

        let validation = core
            .finalize_transaction(
                TransactionBuilder::new(),
                &[preference(account_type)],
                0,
                &signer,
            )
            .await;
        assert!(matches!(
            validation,
            Err(PlatformWalletError::TransactionBuild(_))
        ));

        let signing = core
            .finalize_transaction(
                payment_builder(20),
                &[preference(account_type)],
                0,
                &FailingSigner,
            )
            .await;
        assert!(matches!(
            signing,
            Err(PlatformWalletError::TransactionBuild(_))
        ));

        assert!(core
            .finalize_transaction(payment_builder(21), &[preference(account_type)], 0, &signer)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn abandon_and_definitive_rejection_release_but_maybe_sent_retains() {
        let account_type = StandardAccountType::BIP44Account;
        let (rejection_core, signer) =
            core(account_type, Arc::new(AlwaysRejectedBroadcaster)).await;
        let abandoned = rejection_core
            .finalize_transaction(payment_builder(30), &[preference(account_type)], 0, &signer)
            .await
            .expect("finalize for abandon");
        rejection_core.abandon_transaction(&abandoned).await;

        let rejected = rejection_core
            .finalize_transaction(payment_builder(31), &[preference(account_type)], 0, &signer)
            .await
            .expect("reservation released by abandon");
        assert!(matches!(
            rejection_core
                .broadcast_finalized_transaction(&rejected)
                .await,
            Err(PlatformWalletError::TransactionBroadcast(_))
        ));
        assert!(rejection_core
            .finalize_transaction(payment_builder(32), &[preference(account_type)], 0, &signer)
            .await
            .is_ok());

        let (ambiguous_core, ambiguous_signer) =
            core(account_type, Arc::new(AlwaysMaybeSentBroadcaster)).await;
        let ambiguous = ambiguous_core
            .finalize_transaction(
                payment_builder(33),
                &[preference(account_type)],
                0,
                &ambiguous_signer,
            )
            .await
            .expect("finalize ambiguous send");
        assert!(matches!(
            ambiguous_core
                .broadcast_finalized_transaction(&ambiguous)
                .await,
            Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
        ));
        assert!(matches!(
            ambiguous_core
                .finalize_transaction(
                    payment_builder(34),
                    &[preference(account_type)],
                    0,
                    &ambiguous_signer,
                )
                .await,
            Err(PlatformWalletError::CoreInsufficientFunds { .. })
        ));
    }
}
