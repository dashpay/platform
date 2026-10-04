//! Atomic Core transaction finalization.
//!
//! Funding and reservation deliberately happen in one synchronous critical
//! section under the wallet-manager write lock. Signing happens only after the
//! lock is dropped, so an external signer may call back into a host mnemonic
//! resolver without pinning wallet state.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use dashcore::{Address, OutPoint, PubkeyHash, ScriptBuf, Transaction};
use key_wallet::account::AccountType;
use key_wallet::managed_account::managed_account_collection::ManagedAccountCollection;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
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
use crate::error::ChosenInputProblem;
use crate::wallet::platform_wallet::PlatformWalletInfo;
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

/// The `(available, required)` of a build that failed for lack of funds, or
/// `None` when it failed for another reason. `NoUtxosAvailable` names no
/// requirement.
fn shortfall(error: &BuilderError) -> Option<(Option<u64>, Option<u64>)> {
    match error {
        BuilderError::InsufficientFunds {
            available,
            required,
        }
        | BuilderError::CoinSelection(SelectionError::InsufficientFunds {
            available,
            required,
        }) => Some((Some(*available), Some(*required))),
        BuilderError::CoinSelection(SelectionError::NoUtxosAvailable) => Some((Some(0), None)),
        _ => None,
    }
}

/// Whether a build failed for lack of funds.
/// Whether coins that are not final yet could change the outcome of a build
/// that failed with `error`: a shortfall, or too many inputs (a large waiting
/// coin may fund with a few what the final coins need hundreds for).
pub(crate) fn waiting_may_help(error: &BuilderError) -> bool {
    shortfall(error).is_some() || matches!(error, BuilderError::TooManyInputs { .. })
}

fn map_builder_error(error: BuilderError, context: FundingContext<'_>) -> PlatformWalletError {
    if let Some((available, required)) = shortfall(&error) {
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

/// Whether a coin is final — InstantSend-locked or mined. Builds spend only
/// final coins ([`TransactionBuilder::require_final_inputs`]): the change of a
/// send that never reached the network never becomes final, so a payment
/// built on it is one no node accepts. The spendable and max figures count
/// the same coins, so they never offer what a build refuses.
pub(crate) fn is_final(utxo: &Utxo) -> bool {
    utxo.is_confirmed || utxo.is_instantlocked
}

/// Whether a coin the caller chose as an input can go into a build, in the
/// one order every entry point refuses in: unspendable first (an immature
/// coinbase output, a locked coin — confirmation alone would not change it),
/// then pinned by an in-flight broadcast (`pinned`; the build would refuse it
/// after selection anyway), then not final (code 59, the only refusal waiting
/// cures). Public for platform-wallet-ffi only.
#[doc(hidden)]
pub fn check_chosen_input(
    utxo: &Utxo,
    height: u32,
    pinned: &HashSet<OutPoint>,
) -> Result<(), PlatformWalletError> {
    if !utxo.is_spendable(height) {
        return Err(PlatformWalletError::ChosenInputUnavailable {
            outpoint: utxo.outpoint,
            problem: ChosenInputProblem::NotSpendable,
        });
    }
    if pinned.contains(&utxo.outpoint) {
        return Err(PlatformWalletError::InputMidBroadcast {
            outpoint: utxo.outpoint,
        });
    }
    if !is_final(utxo) {
        return Err(input_awaiting_network(utxo));
    }
    Ok(())
}

/// The coins in-flight broadcasts of this wallet pin, now — what
/// [`check_chosen_input`] takes. Public for platform-wallet-ffi only.
#[doc(hidden)]
pub fn in_broadcast_outpoints(info: &PlatformWalletInfo) -> HashSet<OutPoint> {
    info.generation.in_broadcast_outpoints()
}

/// Code 59 for a coin the caller chose as an input that is not final: it
/// names the coin, and `waiting` is its value.
pub(crate) fn input_awaiting_network(utxo: &Utxo) -> PlatformWalletError {
    PlatformWalletError::CoreFundsAwaitingNetwork {
        available: None,
        waiting: utxo.value(),
        required: None,
        outpoint: Some(utxo.outpoint),
    }
}

/// Makes a fresh copy of a build's configuration — the `make` of
/// [`CoreWallet::finalize_transaction_from`], whose doc states its contract.
pub(crate) type BuilderFactory<'a> =
    dyn Fn() -> Result<TransactionBuilder, PlatformWalletError> + Send + Sync + 'a;

/// The largest size key-wallet could be asked to price. Coin selection
/// prices candidate sets before it enforces the input limit, so the bound is
/// not a standard transaction but every coin a wallet could hold: `u32::MAX`
/// bytes is some 29 million 148-byte inputs.
const MAX_PRICED_BYTES: usize = u32::MAX as usize;

/// key-wallet multiplies the fee rate by the size unchecked: a rate whose fee
/// for the largest size it could price overflows is a typed error here, for a
/// caller to refuse before any build (or waiting-coins trial) prices with it
/// — not a panic, not a wrapped fee. Every rate up to about 42.9 DASH per kB
/// passes.
pub fn check_fee_rate(rate: FeeRate) -> Result<(), PlatformWalletError> {
    checked_fee(rate, MAX_PRICED_BYTES)
        .map(|_| ())
        .map_err(|_| {
            PlatformWalletError::TransactionBuild(format!(
                "fee rate {} sat/kB is above the maximum (about 42.9 DASH/kB)",
                rate.as_sat_per_kb()
            ))
        })
}

/// How a finalizer funds a build, beyond the builder itself. Made with
/// `FinalizeOptions::default()` and the setters below, so options can be added
/// without breaking callers.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct FinalizeOptions {
    /// Coins the build may spend, by outpoint. Each is looked up in the
    /// funding accounts under the wallet-manager write guard and the wallet's
    /// current copy is seeded (one they don't hold — another account's coin,
    /// or one spent since it was chosen — is refused by name, in both funding
    /// modes: the caller picked it), so it is judged on its finality now, not on a
    /// snapshot taken when the caller chose it: a coin InstantSend-locked
    /// since may be spent, one that is not final (any more) is refused by
    /// name with `CoreFundsAwaitingNetwork`, and one an in-flight broadcast
    /// pins with `InputMidBroadcast`. Inputs seeded on the builder itself
    /// (`TransactionBuilder::add_inputs`) stay the caller's snapshot: one not
    /// final in it is dropped by the final-inputs rule, and a selected one is
    /// re-checked before signing; one not final in that snapshot is not
    /// considered by the waiting-coins trial either. A chosen coin that
    /// another in-flight build has reserved is left out by key-wallet's
    /// reservation filter (the pinned key-wallet keeps reservations private),
    /// the one way a chosen coin can be left out without an error. Name a coin
    /// one way, not both: the builder
    /// cannot be read back, so a coin seeded both ways is a candidate twice,
    /// and a build that selects both copies is refused for spending it twice.
    pub inputs: Vec<OutPoint>,
    /// Fund through
    /// [`TransactionBuilder::add_funding_reservation_only`]: the sources take
    /// on their reservation bookkeeping but offer no candidates, so the build
    /// spends only the seeded inputs.
    pub reservation_only: bool,
}

impl FinalizeOptions {
    /// The coins the build may spend, by outpoint (see [`Self::inputs`]).
    pub fn with_inputs(mut self, inputs: impl IntoIterator<Item = OutPoint>) -> Self {
        self.inputs = inputs.into_iter().collect();
        self
    }

    /// Spend only the chosen inputs (see [`Self::reservation_only`]).
    pub fn with_reservation_only(mut self, reservation_only: bool) -> Self {
        self.reservation_only = reservation_only;
        self
    }
}

/// Whether coins that are not final yet would fund a build that just fell
/// short: key-wallet builds `make()` — a fresh copy of the build's
/// configuration, made only when a waiting coin exists — funded like the
/// build, with every not-yet-final coin of
/// the funding accounts treated as final. Returns the value of the
/// not-yet-final coins that trial spends, or `None` when it fails too
/// (confirmation would not help) or spends none of them; or the build's own
/// refusal when the trial would meet it — what stands in the way then is not
/// confirmation: a coin the configuration seeded that no funding account
/// holds (`ChosenInputUnavailable`), or one it seeded twice
/// (`TransactionBuild`, "spends … twice").
///
/// The trial is funded by the build's own fold ([`fund`]) from the same
/// `offered` accounts after the same `seeds`, so key-wallet applies its own
/// de-duplication, reservation filter and change address. An account holding
/// waiting coins is funded through a clone with those coins marked final, and
/// so is every standard account — key-wallet asks each account in turn for a
/// change address from its pool, and only a standard account has one — so no
/// account state moves; the others are funded as they are. Account coins an
/// in-flight broadcast pins (`pinned`, the caller's snapshot) are locked, so
/// selection passes them over; a pinned seed was refused before. A coin the
/// configuration seeds on the builder itself is not promoted: key-wallet keeps
/// the builder's copy over the clone's, so one not final there stays out of
/// the trial as it stays out of the build (choose it through
/// `FinalizeOptions::inputs` instead). The clones share the reservation sets, so what the trial
/// reserves is released before returning. Nothing is signed (a payload
/// finalizer on the configuration does run, as in the build). Runs
/// under the caller's wallet-manager write guard, on the failure path only.
pub(crate) fn trial_with_waiting_coins(
    make: &BuilderFactory<'_>,
    wallet: &Wallet,
    info: &mut PlatformWalletInfo,
    offered: &[AccountType],
    seeds: &[Utxo],
    pinned: Option<&HashSet<OutPoint>>,
    height: u32,
) -> Result<Option<u64>, PlatformWalletError> {
    let generation = Arc::clone(&info.generation);
    let accounts = &mut info.core_wallet.accounts;
    // One scan for the spendable coins that are not final, before anything
    // is snapshotted, made or cloned: a shortfall with none (the common case)
    // costs only this.
    let mut candidates: Vec<(AccountType, OutPoint, u64)> = Vec::new();
    for at in offered {
        let Some(managed) = accounts.funds_account(at) else {
            continue;
        };
        candidates.extend(
            managed
                .utxos
                .values()
                .filter(|utxo| utxo.is_spendable(height) && !is_final(utxo))
                .map(|utxo| (*at, utxo.outpoint, utxo.value())),
        );
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    // A pinned one is left out: the build refuses an input an in-flight
    // broadcast pins (`InputMidBroadcast`), so the trial must not count on it.
    // The caller's snapshot when it has one, else taken now.
    let snapshot;
    let pinned = match pinned {
        Some(pinned) => pinned,
        None => {
            snapshot = generation.in_broadcast_outpoints();
            &snapshot
        }
    };
    let mut waiting: HashMap<OutPoint, u64> = HashMap::new();
    let mut promote: HashMap<AccountType, Vec<OutPoint>> = HashMap::new();
    for (at, outpoint, value) in candidates {
        if !pinned.contains(&outpoint) {
            waiting.insert(outpoint, value);
            promote.entry(at).or_default().push(outpoint);
        }
    }
    if waiting.is_empty() {
        return Ok(None);
    }
    let Ok(trial) = make() else {
        return Ok(None);
    };
    // Funded through clones: an account with waiting coins (marked final) or
    // pinned ones (locked, so selection passes them over), and every standard
    // account — key-wallet asks each account in turn for a change address from
    // its pool until one gives it, and only a standard account has a pool.
    let mut clones: HashMap<AccountType, ManagedCoreFundsAccount> = HashMap::new();
    for at in offered {
        let Some(managed) = accounts.funds_account(at) else {
            continue;
        };
        let promoted = promote.remove(at).unwrap_or_default();
        let locked: Vec<OutPoint> = pinned
            .iter()
            .filter(|outpoint| managed.utxos.contains_key(*outpoint))
            .copied()
            .collect();
        if promoted.is_empty() && locked.is_empty() && !matches!(at, AccountType::Standard { .. }) {
            continue;
        }
        let mut clone = managed.clone();
        for outpoint in promoted {
            if let Some(utxo) = clone.utxos.get_mut(&outpoint) {
                utxo.is_instantlocked = true;
            }
        }
        for outpoint in locked {
            if let Some(utxo) = clone.utxos.get_mut(&outpoint) {
                utxo.is_locked = true;
            }
        }
        clones.insert(*at, clone);
    }
    // Seeds are the funding accounts' own coins, checked when chosen.
    let trial = fund(
        trial,
        wallet,
        accounts,
        &mut clones,
        offered,
        seeds.iter().cloned(),
        false,
        height,
    );
    let Ok((transaction, _, token)) = trial.build_unsigned_reserved() else {
        return Ok(None);
    };
    if let Some(token) = token {
        for at in offered {
            if let Some(managed) = accounts.funds_account(at) {
                managed.release_reservation_if_owner(&transaction, token);
            }
        }
    }
    // The build refuses a transaction that spends one coin twice (a coin the
    // configuration seeds more than once): a trial that does is no promise,
    // and it is the build's own refusal that stands in the way — decided
    // before anything else can return. (A build with no waiting coin gets no
    // trial and reports key-wallet's shortfall: the pinned builder has no
    // getters to inspect its seeds before building.)
    if let Some(error) = duplicate_prevout(&transaction) {
        return Err(error);
    }
    // Backstop: a pin taken since the caller's snapshot.
    if generation.in_broadcast_conflict(&transaction).is_some() {
        return Ok(None);
    }
    // The build refuses a selected input no funding account holds (a coin
    // the configuration seeded itself from elsewhere): a trial that needs one
    // is no promise, and that coin is what stands in the way.
    let owned = |outpoint: &OutPoint| {
        offered.iter().any(|at| {
            accounts
                .funds_account(at)
                .is_some_and(|managed| managed.utxos.contains_key(outpoint))
        })
    };
    // The trial knows the actual cause then: name the coin.
    if let Some(input) = transaction
        .input
        .iter()
        .find(|input| !owned(&input.previous_output))
    {
        return Err(PlatformWalletError::ChosenInputUnavailable {
            outpoint: input.previous_output,
            problem: ChosenInputProblem::NotInFundingAccounts,
        });
    }
    let spent = transaction
        .input
        .iter()
        .filter_map(|input| waiting.get(&input.previous_output))
        .fold(0u64, |sum, value| sum.saturating_add(*value));
    Ok((spent > 0).then_some(spent))
}

/// A transaction that spends one outpoint twice is invalid (Core rejects
/// it); the refusal a build and its waiting-coins trial share.
fn duplicate_prevout(transaction: &Transaction) -> Option<PlatformWalletError> {
    let mut prevouts: HashSet<OutPoint> = HashSet::new();
    transaction
        .input
        .iter()
        .find(|input| !prevouts.insert(input.previous_output))
        .map(|duplicate| {
            PlatformWalletError::TransactionBuild(format!(
                "built transaction spends {} twice",
                duplicate.previous_output
            ))
        })
}

/// The funding fold a build and its waiting-coins trial share: the wallet's
/// height, final coins only, the seeded inputs, then every offered account in
/// order — through its clone in `clones` where it has one — with
/// `add_funding`, or `add_funding_reservation_only` for a `reservation_only`
/// build.
#[allow(clippy::too_many_arguments)]
fn fund(
    builder: TransactionBuilder,
    wallet: &Wallet,
    accounts: &mut ManagedAccountCollection,
    clones: &mut HashMap<AccountType, ManagedCoreFundsAccount>,
    offered: &[AccountType],
    seeds: impl IntoIterator<Item = Utxo>,
    reservation_only: bool,
    height: u32,
) -> TransactionBuilder {
    let mut builder = builder
        .set_current_height(height)
        .require_final_inputs()
        .add_inputs(seeds);
    for at in offered {
        let Some(account) = wallet.accounts.account_of_type(*at) else {
            continue;
        };
        let managed = match clones.get_mut(at) {
            Some(clone) => clone,
            None => match accounts.funds_account_mut(at) {
                Some(managed) => managed,
                None => continue,
            },
        };
        builder = if reservation_only {
            builder.add_funding_reservation_only(managed, account)
        } else {
            builder.add_funding(managed, account)
        };
    }
    builder
}

/// A shortfall the not-yet-final coins would fund
/// ([`trial_with_waiting_coins`] returned the `waiting` value they bring)
/// becomes [`PlatformWalletError::CoreFundsAwaitingNetwork`], with the build's
/// own `available` / `required`; anything else is returned as it was mapped.
pub(crate) fn awaiting_network(
    error: PlatformWalletError,
    waiting: Option<u64>,
) -> PlatformWalletError {
    match (waiting, &error) {
        (
            Some(waiting),
            PlatformWalletError::CoreInsufficientFunds {
                available,
                required,
                ..
            }
            | PlatformWalletError::CorePooledInsufficientFunds {
                available,
                required,
                ..
            },
        ) => funds_awaiting_network(*available, *required, waiting),
        _ => error,
    }
}

/// Code 59 for a shortfall the waiting coins would fund.
fn funds_awaiting_network(
    available: Option<u64>,
    required: Option<u64>,
    waiting: u64,
) -> PlatformWalletError {
    PlatformWalletError::CoreFundsAwaitingNetwork {
        available,
        waiting,
        required,
        outpoint: None,
    }
}

/// The same rule for a build whose failure is otherwise reported as a plain
/// build error (the DashPay contact payment).
pub(crate) fn build_error_awaiting_network(
    error: BuilderError,
    waiting: Option<u64>,
) -> PlatformWalletError {
    match (shortfall(&error), waiting) {
        (Some((available, required)), Some(waiting)) => {
            funds_awaiting_network(available, required, waiting)
        }
        (None, Some(waiting)) if matches!(error, BuilderError::TooManyInputs { .. }) => {
            funds_awaiting_network(None, None, waiting)
        }
        _ => PlatformWalletError::TransactionBuild(error.to_string()),
    }
}

/// The fee one more input adds at `fee_rate`, from key-wallet's own estimator
/// by difference, so no size is re-declared here.
fn input_fee(fee_rate: FeeRate) -> Result<u64, PlatformWalletError> {
    checked_fee(
        fee_rate,
        estimate_tx_size(1, 0, false).saturating_sub(estimate_tx_size(0, 0, false)),
    )
}

/// key-wallet's `FeeRate::calculate_fee`, with the multiplication checked.
///
/// Upstream computes `(sat_per_kb * size_bytes).div_ceil(1000)` unchecked, and
/// the rate reaching [`CoreWallet::pooled_max_sendable`] is a `u64` the host
/// picks and hands across the FFI. An overflow there is not a wrong number:
/// the iOS profile builds with `panic = "abort"`, so it takes the host process
/// down instead of surfacing as an error the caller can show — and a profile
/// without overflow checks is worse still, wrapping silently into a fee that
/// makes the reported maximum nonsense. Both become a typed error here.
fn checked_fee(fee_rate: FeeRate, size_bytes: usize) -> Result<u64, PlatformWalletError> {
    fee_rate
        .as_sat_per_kb()
        .checked_mul(size_bytes as u64)
        .map(|total| total.div_ceil(1000))
        .ok_or_else(|| {
            PlatformWalletError::TransactionBuild(format!(
                "fee rate {} sat/kb overflows for a {size_bytes}-byte transaction",
                fee_rate.as_sat_per_kb()
            ))
        })
}

/// The output [`CoreWallet::pooled_max_sendable`] prices: one P2PKH, matching
/// the `estimate_tx_size(_, 1, false)` model it sizes the transaction with.
///
/// Only the script *shape* matters — `dust_value` reads the encoded length, not
/// the hash — so a zero hash stands in for the real destination, which the
/// caller has not chosen yet at send-max time.
fn modeled_output_script() -> ScriptBuf {
    use dashcore::hashes::Hash;
    ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0u8; 20]))
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
    /// build's inputs (the release/re-reserve double-spend window).
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
    /// cleanup while A's real reservation leaked until its TTL.
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
    /// the same held reservation. The transaction,
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
    // miss too: the caller asked for exactly those funds. Name the preference
    // itself, not the `Option` wrapping it — `strict` only ever arrives with a
    // one-element list, but formatting the `Option` would print `Some(BIP44)`.
    if strict && resolved.is_empty() {
        return Err(PlatformWalletError::WalletNotFound(match sources.first() {
            Some(preference) => {
                format!("wallet account {preference:?} #{source_index} not found")
            }
            None => format!("no funding source named for #{source_index}"),
        }));
    }
    Ok(resolved)
}

/// The concrete accounts `preference` resolves to at `source_index` — the
/// platform mirror of key-wallet's private `account_types_for`: the single
/// account at `source_index` for the standard families, and every DashPay
/// receiving account the selector picks (which span their own indices) for a
/// DashPay source. A set selector matching nothing resolves to an empty list,
/// not an error — a wallet with no contacts still sends from its standard
/// accounts.
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

/// The most inputs one standard transaction may carry. key-wallet enforces this
/// (`BuilderError::TooManyInputs`) but keeps its constant private, so the value
/// is mirrored here rather than imported.
///
/// A mirrored constant is exactly the hand-copy that drifts, so the direction
/// that matters is pinned by behaviour rather than by trust:
/// `pooled_max_sendable_respects_the_input_cap` builds a transaction filled to
/// this number, so a value ABOVE key-wallet's real limit turns red — that is
/// the direction that breaks the promise, since it names an amount no build can
/// reach. A value below it cannot be caught the same way (a test written in
/// terms of the mirror moves with it) and is merely conservative: the maximum
/// is under-reported and the last UTXOs stay unreachable in one send.
const MAX_STANDARD_TX_INPUTS: usize = 500;

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
                .filter(|utxo| is_final(utxo))
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
    /// A pool holding more than [`MAX_STANDARD_TX_INPUTS`] eligible UTXOs is
    /// capped at its largest that many: one transaction cannot carry the rest,
    /// so offering their value would name an amount no build could reach. Money
    /// beyond the cap is not lost, only unreachable in a single send.
    ///
    /// One limit is still NOT modelled, and it can only make the real ceiling
    /// lower, never higher: reservations held by another in-flight build (see
    /// [`Self::pooled_spendable_balance`]).
    pub async fn pooled_max_sendable(
        &self,
        sources: &[AccountTypePreference],
        source_index: u32,
        fee_rate: Option<FeeRate>,
    ) -> Result<u64, PlatformWalletError> {
        let fee_rate = fee_rate.unwrap_or_else(FeeRate::normal);
        // The one-output shell and the per-input cost ([`input_fee`]) come
        // from key-wallet's estimator, so this crate never re-declares a size
        // that could drift from the one the build charges.
        let empty = estimate_tx_size(0, 1, false);

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
        let input_cost = input_fee(fee_rate)?;
        let mut values: Vec<u64> = Vec::new();
        for at in resolved {
            let Some(managed) = info.core_wallet.accounts.funds_account(&at) else {
                continue;
            };
            values.extend(
                managed
                    .spendable_utxos(height)
                    .iter()
                    .filter(|utxo| is_final(utxo))
                    .map(|utxo| utxo.value())
                    .filter(|value| *value > input_cost),
            );
        }
        if values.is_empty() {
            return Ok(0);
        }

        // One transaction cannot carry more inputs than the relay cap, so a pool
        // above it can only offer its largest `MAX_STANDARD_TX_INPUTS` — asking
        // for more would need a build key-wallet refuses outright.
        if values.len() > MAX_STANDARD_TX_INPUTS {
            values.sort_unstable_by(|a, b| b.cmp(a));
            values.truncate(MAX_STANDARD_TX_INPUTS);
        }

        let selected: u64 = values.iter().sum();
        // Priced the way the default BranchAndBound selector prices it: one
        // rounding per input plus one for the rest of the transaction, never a
        // single rounding over the whole thing.
        //
        // `ceil(a) + n*ceil(b) >= ceil(a + n*b)`, so the selector always needs
        // at least as much as whole-transaction arithmetic suggests. Reporting
        // the cheaper figure names an amount those inputs cannot fund under the
        // selector's own acceptance test, and it closes the gap by reaching for
        // one more UTXO — the input a pool sitting on the cap has no room for,
        // so the build fails with "too many inputs" on the very amount this
        // getter promised. At a whole number of duffs per byte the two agree;
        // a fractional rate is where they part.
        let inputs_fee = input_cost.checked_mul(values.len() as u64).ok_or_else(|| {
            PlatformWalletError::TransactionBuild(format!(
                "fee rate {} sat/kb overflows for {} inputs",
                fee_rate.as_sat_per_kb(),
                values.len()
            ))
        })?;
        let fee = checked_fee(fee_rate, empty)?
            .checked_add(inputs_fee)
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild(format!(
                    "fee rate {} sat/kb overflows for a {}-input transaction",
                    fee_rate.as_sat_per_kb(),
                    values.len()
                ))
            })?;
        let net = selected.saturating_sub(fee);

        // Covering the fee is not enough: an output below the modeled script's
        // dust threshold is rejected by standard relay policy, so offering that
        // amount as a maximum names a payment that cannot be relayed. Report
        // nothing sendable instead — the same answer an empty pool gives.
        if net < modeled_output_script().dust_value().to_sat() {
            return Ok(0);
        }
        Ok(net)
    }

    /// Consume a configured builder, atomically fund and reserve its selected
    /// inputs, then sign without holding the wallet-manager lock.
    ///
    /// A builder cannot be read back or rebuilt (the pinned key-wallet has no
    /// getters and no `Clone`), so a shortfall is reported as insufficient
    /// funds even when coins that are not final yet would cover it. A caller
    /// that can make its configuration again uses
    /// [`Self::finalize_transaction_from`] to have such a shortfall reported
    /// as [`PlatformWalletError::CoreFundsAwaitingNetwork`].
    pub async fn finalize_transaction<S: TransactionSigner + ?Sized + Sync>(
        &self,
        builder: TransactionBuilder,
        sources: &[AccountTypePreference],
        source_index: u32,
        signer: &S,
    ) -> Result<SignedCoreTransaction, PlatformWalletError> {
        self.finalize(
            builder,
            None,
            FinalizeOptions::default(),
            sources,
            source_index,
            signer,
        )
        .await
    }

    /// [`Self::finalize_transaction`] with [`FinalizeOptions`]: inputs chosen
    /// by outpoint, and `reservation_only` funding.
    ///
    /// `reservation_only` is what a caller draining an account in batches
    /// under the standard-transaction input limit needs — ordinary funding
    /// offers the whole account on top of the batch, so every batch trips the
    /// cap and an account above it can never be drained.
    pub async fn finalize_transaction_with_options<S: TransactionSigner + ?Sized + Sync>(
        &self,
        builder: TransactionBuilder,
        options: FinalizeOptions,
        sources: &[AccountTypePreference],
        source_index: u32,
        signer: &S,
    ) -> Result<SignedCoreTransaction, PlatformWalletError> {
        self.finalize(builder, None, options, sources, source_index, signer)
            .await
    }

    /// [`Self::finalize_transaction_with_options`] for a build `make` can
    /// configure again: the build is `make()`, and when it falls short a
    /// second copy is tried with the coins that are not final yet treated as
    /// final ([`trial_with_waiting_coins`]). If that trial builds, the
    /// shortfall is [`PlatformWalletError::CoreFundsAwaitingNetwork`] —
    /// key-wallet's own verdict, at the build's fee rate, outputs, payload and
    /// strategy; otherwise it stays insufficient funds. A `reservation_only`
    /// build gets no trial: it spends only the chosen inputs, which are final
    /// by then (or refused), so nothing it could spend is waiting.
    ///
    /// `make` is called once before the wallet-manager lock is taken and, on a
    /// shortfall when the funding accounts hold a spendable coin that is not
    /// final and not pinned (never for `reservation_only`), once more while
    /// its write guard is held: it must not touch
    /// the wallet manager (or anything that waits on it), or that second call
    /// deadlocks. The trial may price coins the build never reached, so a fee
    /// rate it sets must pass [`check_fee_rate`] (key-wallet multiplies rate by
    /// size unchecked). It is dropped as soon as the lock is released, before
    /// the signer runs, so whatever it captures (a payload signing key, say)
    /// does not outlive the build.
    pub async fn finalize_transaction_from<F, S>(
        &self,
        make: F,
        options: FinalizeOptions,
        sources: &[AccountTypePreference],
        source_index: u32,
        signer: &S,
    ) -> Result<SignedCoreTransaction, PlatformWalletError>
    where
        F: Fn() -> Result<TransactionBuilder, PlatformWalletError> + Send + Sync,
        S: TransactionSigner + ?Sized + Sync,
    {
        let builder = make()?;
        self.finalize(
            builder,
            Some(Box::new(make)),
            options,
            sources,
            source_index,
            signer,
        )
        .await
    }

    async fn finalize<S: TransactionSigner + ?Sized + Sync>(
        &self,
        builder: TransactionBuilder,
        // Makes the build again for the waiting-coins trial; `None`: a
        // shortfall stays insufficient funds. Dropped before signing.
        make: Option<Box<BuilderFactory<'_>>>,
        options: FinalizeOptions,
        // The funding sources to POOL, in order — the first supplies the
        // change address. A plain send passes [`SEND_FUNDING_SOURCES`]
        // (BIP44 + BIP32 + every DashPay receiving account); a single-element
        // list reproduces the old one-account behavior, including its strict
        // account-not-found error. `source_index` addresses the standard
        // families; DashPay set selectors span their own indices.
        sources: &[AccountTypePreference],
        source_index: u32,
        signer: &S,
    ) -> Result<SignedCoreTransaction, PlatformWalletError> {
        let FinalizeOptions {
            inputs: seed_outpoints,
            reservation_only,
        } = options;
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
            // a new token meanwhile.
            // Only final coins (InstantSend-locked or mined) are spent; see
            // `is_final` and `fund`.
            let mut paths: HashMap<Address, DerivationPath> = HashMap::new();
            let resolved = resolved_funding_accounts(
                &info.core_wallet.accounts,
                wallet,
                sources,
                source_index,
                strict,
            )?;
            if resolved.is_empty() {
                return Err(PlatformWalletError::WalletNotFound(format!(
                    "no funding account of any named source at index {source_index}"
                )));
            }

            // Inputs chosen by outpoint are seeded from the wallet's current
            // copies, under this guard, before any funding call (which then
            // skips them as already present): the final-inputs rule judges
            // them as they are now. One that is not final is refused by name
            // instead of being dropped silently; nothing is reserved yet.
            // One snapshot of what in-flight broadcasts pin, taken only when
            // inputs were chosen; the waiting-coins trial reuses it, or takes
            // its own on a shortfall.
            let mut pinned: Option<HashSet<OutPoint>> = None;
            let mut seeds: Vec<Utxo> = Vec::new();
            let mut seeded: HashSet<OutPoint> = HashSet::new();
            for outpoint in &seed_outpoints {
                if !seeded.insert(*outpoint) {
                    continue;
                }
                let Some(utxo) = resolved.iter().find_map(|at| {
                    info.core_wallet
                        .accounts
                        .funds_account(at)
                        .and_then(|managed| managed.utxos.get(outpoint))
                }) else {
                    // Not a coin of the funding accounts (another account's,
                    // or spent since it was chosen): refused by name, in both
                    // funding modes. The caller picked this coin; leaving it
                    // out silently — under a drain, sweeping everything else
                    // without it — would be worse than refusing a seed the
                    // build might not have needed.
                    return Err(PlatformWalletError::ChosenInputUnavailable {
                        outpoint: *outpoint,
                        problem: ChosenInputProblem::NotInFundingAccounts,
                    });
                };
                // An unspendable coin would be dropped by the selector
                // silently: refused by name, like a pinned or waiting one.
                let pinned = pinned.get_or_insert_with(|| in_broadcast_outpoints(info));
                check_chosen_input(utxo, height, pinned)?;
                seeds.push(utxo.clone());
            }
            // The accounts that take on this build's reservation bookkeeping,
            // in funding order — the ones a failure path must release
            // (`resolved_funding_accounts` already dropped anything missing
            // from either half). NOT the accounts that end up contributing
            // inputs — selection may take nothing from most of them — which
            // are derived from the selected inputs below.
            let offered_accounts = resolved;
            for managed in offered_accounts
                .iter()
                .filter_map(|at| info.core_wallet.accounts.funds_account(at))
            {
                for utxo in managed.utxos.values() {
                    if let Some(path) = managed.address_derivation_path(&utxo.address) {
                        paths.insert(utxo.address.clone(), path);
                    }
                }
            }
            let builder = fund(
                builder,
                wallet,
                &mut info.core_wallet.accounts,
                &mut HashMap::new(),
                &offered_accounts,
                seeds.iter().cloned(),
                reservation_only,
                height,
            );

            let funding_context = if strict {
                FundingContext::Single {
                    preference: primary,
                    index: source_index,
                }
            } else {
                FundingContext::Pooled(sources)
            };
            // A shortfall the coins not final yet would fund is "waiting on
            // the network", not insufficient funds: key-wallet decides, on a
            // fresh copy of the build — on the failure path only, still under
            // this guard.
            let (unsigned, fee, reservation_token) =
                builder.build_unsigned_reserved().map_err(|error| {
                    // Only a shortfall or too many inputs can turn on coins
                    // that are not final, and a `reservation_only` build spends
                    // only its seeds, which are final by now.
                    let trial = make
                        .as_deref()
                        .filter(|_| waiting_may_help(&error) && !reservation_only);
                    let too_many = matches!(error, BuilderError::TooManyInputs { .. });
                    let error = map_builder_error(error, funding_context);
                    let Some(make) = trial else {
                        return error;
                    };
                    let waiting = match trial_with_waiting_coins(
                        make,
                        wallet,
                        info,
                        &offered_accounts,
                        &seeds,
                        pinned.as_ref(),
                        height,
                    ) {
                        Ok(waiting) => waiting,
                        Err(refused) => return refused,
                    };
                    match (too_many, waiting) {
                        (true, Some(waiting)) => funds_awaiting_network(None, None, waiting),
                        _ => awaiting_network(error, waiting),
                    }
                })?;

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
            // should not have to substring-match prose to tell it apart.
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
                        .ok_or(PlatformWalletError::ChosenInputUnavailable {
                            // Only a coin seeded on the builder itself can get
                            // here: the funding accounts did not offer it.
                            outpoint: input.previous_output,
                            problem: ChosenInputProblem::NotInFundingAccounts,
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
            // `require_final_inputs` judged the builder's copies of the coins.
            // An input the caller seeded on the builder itself is its
            // snapshot, and key-wallet keeps it over the resident entry, so a
            // coin a reorg has since demoted would pass on its old confirmed
            // flag. The resident coins are what gets signed: check those,
            // still under the write guard.
            if let Some(stale) = selected.iter().find(|utxo| !is_final(utxo)) {
                let error = input_awaiting_network(stale);
                release_all!(offered_accounts, info.core_wallet.accounts, &unsigned);
                return Err(error);
            }
            // Duplicate prevouts make a transaction invalid (Core rejects it),
            // and additive funding is the shape that can produce them — an
            // outpoint seeded by `add_inputs` that a funding account also
            // offers. `add_funding` filters those (rust-dashcore#931); this
            // asserts the invariant here too rather than handing a signer, and
            // then the network, a transaction that cannot confirm.
            if let Some(error) = duplicate_prevout(&unsigned) {
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
        // The factory, and whatever it captured, does not live across the
        // signer.
        drop(make);

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
            // wallet.
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
    /// untouched. When `None` (the build reserved
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
    use dashcore::{Address as DashAddress, Network, Transaction, TxIn};
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::signer::{Signer, SignerMethod};
    use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
    use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
    use key_wallet::DerivationPath;
    use tokio::sync::Barrier;

    use crate::broadcaster::TransactionBroadcaster;
    use crate::test_support::{
        funded_wallet_manager, funded_wallet_manager_dual_standard,
        funded_wallet_manager_with_contact, standard_account_coins, AlwaysMaybeSentBroadcaster,
        AlwaysOkBroadcaster, AlwaysRejectedBroadcaster, WalletSigner,
    };
    use key_wallet::wallet::managed_wallet_info::fee::{estimate_tx_size, FeeRate};

    use key_wallet::account::AccountType;
    use key_wallet::wallet::managed_wallet_info::transaction_builder::BuilderError;
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use crate::error::ChosenInputProblem;
    use crate::wallet::core::transaction::{
        build_error_awaiting_network, is_final, modeled_output_script, FinalizeOptions,
        MAX_STANDARD_TX_INPUTS,
    };
    use crate::wallet::core::CoreWallet;
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
    use crate::PlatformWalletError;
    use crate::SEND_FUNDING_SOURCES;

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
            .finalize_transaction(payment_builder(40), &SEND_FUNDING_SOURCES, 0, &signer)
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
            .finalize_transaction(payment_builder(41), &SEND_FUNDING_SOURCES, 0, &signer)
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
            .finalize_transaction(payment_builder(60), &SEND_FUNDING_SOURCES, 0, &signer)
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
            .finalize_transaction(payment_builder(61), &SEND_FUNDING_SOURCES, 0, &signer)
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
            core.pooled_spendable_balance(&SEND_FUNDING_SOURCES, 0)
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
            .pooled_spendable_balance(&SEND_FUNDING_SOURCES, 0)
            .await
            .expect("gross balance");
        let max = core
            .pooled_max_sendable(&SEND_FUNDING_SOURCES, 0, None)
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
            .finalize_transaction(spend(gross, 70), &SEND_FUNDING_SOURCES, 0, &signer)
            .await;
        assert!(
            over.is_err(),
            "the gross balance must not be offerable as an amount, got {over:?}"
        );

        // The net figure builds, and takes every UTXO with it.
        let finalized = core
            .finalize_transaction(spend(max, 71), &SEND_FUNDING_SOURCES, 0, &signer)
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
            .pooled_max_sendable(&SEND_FUNDING_SOURCES, 0, None)
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
                .pooled_spendable_balance(&SEND_FUNDING_SOURCES, 0)
                .await
                .expect("gross balance"),
            1_400_100,
            "the gross figure counts every spendable UTXO, dust included"
        );
        assert_eq!(
            with_dust
                .pooled_max_sendable(&SEND_FUNDING_SOURCES, 0, None)
                .await
                .expect("max sendable"),
            baseline,
            "a UTXO that cannot pay for its own input must not move the maximum"
        );
    }

    /// The same cap promise at a rate that is not a whole number of duffs per
    /// byte, where the selector's per-input rounding and a single rounding over
    /// the whole transaction disagree.
    ///
    /// The extra 575-duff UTXO is the trap: it survives the profitability
    /// filter, is too small to make the top-500 cut, and is therefore invisible
    /// to the estimate — but a maximum priced with whole-transaction rounding
    /// leaves the selector a few duffs short of its own acceptance test, and it
    /// closes that gap by taking this input as the 501st.
    #[tokio::test]
    async fn pooled_max_sendable_is_buildable_at_a_fractional_fee_rate() {
        let mut outputs = vec![10_000u64; MAX_STANDARD_TX_INPUTS];
        outputs.push(575);
        let (manager, wallet_id, generation, signer) =
            crate::test_support::funded_wallet_manager_with_outputs(
                StandardAccountType::BIP44Account,
                &outputs,
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

        let sources = &[AccountTypePreference::BIP44][..];
        let rate = FeeRate::new(1001);
        let max = core
            .pooled_max_sendable(sources, 0, Some(rate))
            .await
            .expect("max sendable");
        assert!(max > 0, "the pool is well above dust");

        let built = core
            .finalize_transaction(
                TransactionBuilder::new()
                    .set_fee_rate(rate)
                    .add_output(&DashAddress::dummy(Network::Testnet, 81), max),
                sources,
                0,
                &signer,
            )
            .await;
        assert!(
            built.is_ok(),
            "the reported maximum must build at the rate it was priced for: {:?}",
            built.err()
        );
    }

    /// The input cap, and the value of `MAX_STANDARD_TX_INPUTS` itself.
    ///
    /// A wallet holding one UTXO more than a transaction can carry cannot spend
    /// everything in one send, so a maximum computed from every eligible UTXO
    /// names an amount no build could reach. Both halves are asserted by
    /// building: the uncapped figure fails, the capped one succeeds with exactly
    /// the cap's worth of inputs.
    ///
    /// This is also what pins the mirrored constant, in the direction that can
    /// break the API's promise: the build filled to the cap only succeeds if
    /// key-wallet's private limit is at least the mirrored value, so raising the
    /// mirror above key-wallet's reds this test (verified with 600). Lowering it
    /// does not, and cannot — every assertion here is written in terms of the
    /// mirror and moves with it — but under-reporting is safe, only stingy.
    #[tokio::test]
    async fn pooled_max_sendable_respects_the_input_cap() {
        // One more than a standard transaction can carry, each well above the
        // ~296-duff cost of its own input so none is dropped as unprofitable.
        let outputs = vec![10_000u64; MAX_STANDARD_TX_INPUTS + 1];
        let (manager, wallet_id, generation, signer) =
            crate::test_support::funded_wallet_manager_with_outputs(
                StandardAccountType::BIP44Account,
                &outputs,
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

        let sources = &[AccountTypePreference::BIP44][..];
        let gross = core
            .pooled_spendable_balance(sources, 0)
            .await
            .expect("gross balance");
        assert_eq!(
            gross,
            10_000 * (MAX_STANDARD_TX_INPUTS as u64 + 1),
            "the gross figure counts every UTXO, cap or no cap"
        );

        let max = core
            .pooled_max_sendable(sources, 0, None)
            .await
            .expect("max sendable");

        // The capped total, minus the fee for a transaction that full.
        let capped_value = 10_000 * MAX_STANDARD_TX_INPUTS as u64;
        assert!(
            max < capped_value,
            "the fee for {MAX_STANDARD_TX_INPUTS} inputs has to come off: max {max}"
        );

        let spend = |amount: u64, tag: u8| {
            TransactionBuilder::new().add_output(
                &DashAddress::dummy(Network::Testnet, usize::from(tag)),
                amount,
            )
        };

        // Anything needing the UTXO beyond the cap is unbuildable. The amount
        // has to be the one an UNCAPPED maximum would have offered — the gross
        // total minus the fee a build spending every UTXO would pay — so that
        // funds are sufficient and the cap is the only thing left to refuse.
        // `capped_value + 1` looks like the same test but is not: it leaves too
        // little for the 501-input fee, fails on funds first, and would keep
        // passing if key-wallet dropped its cap entirely.
        let uncapped_max = gross
            - FeeRate::normal().calculate_fee(estimate_tx_size(
                MAX_STANDARD_TX_INPUTS + 1,
                1,
                false,
            ));
        // Note it is BELOW `capped_value`: the 501st input's fee costs more
        // than the 10,000 duffs it brings. What makes it need that input is
        // that it is beyond what a 500-input build can pay for — which is
        // exactly the reported maximum.
        assert!(
            uncapped_max > max,
            "the uncapped amount must be past what a capped build can pay: {uncapped_max} vs {max}"
        );
        let over = core
            .finalize_transaction(spend(uncapped_max, 80), sources, 0, &signer)
            .await;
        match over {
            Err(PlatformWalletError::TransactionBuild(message)) => assert!(
                message.contains("Too many inputs"),
                "the refusal must be the input cap, not something else: {message}"
            ),
            other => panic!("an amount requiring more than the cap must be refused for that reason, got {other:?}"),
        }

        // The reported maximum builds, and fills the transaction exactly to the
        // cap — which is only true if the mirrored constant matches key-wallet's.
        let finalized = core
            .finalize_transaction(spend(max, 81), sources, 0, &signer)
            .await
            .expect("the capped maximum must be an amount a build accepts");
        assert_eq!(
            finalized.transaction().input.len(),
            MAX_STANDARD_TX_INPUTS,
            "spending the capped maximum must fill the transaction to the cap"
        );
        core.abandon_transaction(&finalized).await;
    }

    /// A fee rate the host may pass but the fee arithmetic cannot hold.
    ///
    /// key-wallet multiplies `sat_per_kb * size_bytes` unchecked, and the rate
    /// arrives as a `u64` chosen host-side and forwarded verbatim by
    /// `core_wallet_pooled_max_sendable`. The iOS profile builds with
    /// `panic = "abort"`, so an overflow inside that multiplication ends the
    /// process rather than the call — it has to be refused before it happens,
    /// and the largest representable rate must still compute.
    #[tokio::test]
    async fn pooled_max_sendable_refuses_a_fee_rate_that_would_overflow() {
        let (manager, wallet_id, generation, _signer) =
            crate::test_support::funded_wallet_manager_with_outputs(
                StandardAccountType::BIP44Account,
                &[10_000_000],
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
        let sources = &[AccountTypePreference::BIP44][..];

        let refused = core
            .pooled_max_sendable(sources, 0, Some(FeeRate::new(u64::MAX)))
            .await;
        assert!(
            matches!(refused, Err(PlatformWalletError::TransactionBuild(_))),
            "an unrepresentable fee rate must be an error, not a panic: {refused:?}"
        );

        // The boundary itself: the highest rate whose per-input fee still fits.
        // Every UTXO is priced out at a rate this large, so the answer is zero —
        // but it is an ANSWER, which is the point.
        let per_input =
            estimate_tx_size(1, 1, false).saturating_sub(estimate_tx_size(0, 1, false)) as u64;
        let highest = u64::MAX / per_input;
        assert_eq!(
            core.pooled_max_sendable(sources, 0, Some(FeeRate::new(highest)))
                .await
                .expect("the largest representable rate must still compute"),
            0,
            "no UTXO can outearn its own input cost at that rate"
        );
    }

    /// Covering the fee is not the same as being spendable: an output under the
    /// dust threshold is refused by standard relay, so a maximum reported below
    /// it names a payment that cannot be made. Both sides of the boundary.
    #[tokio::test]
    async fn pooled_max_sendable_reports_nothing_when_the_net_output_would_be_dust() {
        let fee = FeeRate::normal().calculate_fee(estimate_tx_size(1, 1, false));
        let dust = modeled_output_script().dust_value().to_sat();

        let max_for = |funding: u64| async move {
            let (manager, wallet_id, generation, _signer) =
                crate::test_support::funded_wallet_manager_with_outputs(
                    StandardAccountType::BIP44Account,
                    &[funding],
                )
                .await;
            let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
            CoreWallet::new(
                sdk,
                manager,
                wallet_id,
                Arc::new(AlwaysOkBroadcaster),
                generation,
            )
            .pooled_max_sendable(&[AccountTypePreference::BIP44][..], 0, None)
            .await
            .expect("max sendable")
        };

        assert_eq!(
            max_for(dust + fee - 1).await,
            0,
            "one duff short of a relayable output must report nothing sendable"
        );
        assert_eq!(
            max_for(dust + fee).await,
            dust,
            "exactly at the dust threshold is still sendable"
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
            core.pooled_spendable_balance(&SEND_FUNDING_SOURCES, 0)
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
                .pooled_spendable_balance(&SEND_FUNDING_SOURCES, 0)
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

    /// A DRAIN THAT CARRIES A MEMO, through the production finalize path.
    ///
    /// The MAYAChain deposit shape: `SelectionStrategy::All` with the vault
    /// destination plus a zero-value OP_RETURN memo. key-wallet used to accept
    /// exactly one output under `All` ("requires exactly one output"), so the
    /// memo made every whole-balance swap unbuildable — a caller had to guess
    /// the fee and send an explicit amount instead. The engine now counts
    /// value carriers and lets zero-value data outputs ride along; this pins
    /// that the rust-dashcore revision THIS workspace builds against carries
    /// that behaviour, since a pin regression leaves every other test green.
    ///
    /// The amount given for the destination is 0: under `All` the engine sets
    /// it, and 0 is what the hosts pass. The assertions are the ones a host
    /// relies on: the build succeeds, the memo survives at value 0, and the
    /// sole spendable output — the deliverable amount the FFI reports —
    /// receives the entire balance minus the fee, with no change.
    #[tokio::test]
    async fn memo_bearing_drain_delivers_the_whole_balance_minus_fee() {
        let (core, signer) = core(
            StandardAccountType::BIP44Account,
            Arc::new(AlwaysOkBroadcaster),
        )
        .await;
        let destination = DashAddress::dummy(Network::Testnet, 7);
        let memo = b"=:MAYA.CACAO:maya1abc";
        let builder = TransactionBuilder::new()
            .set_selection_strategy(SelectionStrategy::All)
            .add_output(&destination, 0)
            .add_op_return(memo)
            .expect("memo is within the OP_RETURN limit");

        let finalized = core
            .finalize_transaction(builder, &[AccountTypePreference::BIP44], 0, &signer)
            .await
            .expect("a drain may carry a zero-value OP_RETURN memo beside its destination");

        let tx = finalized.transaction();
        let (memos, carriers): (Vec<_>, Vec<_>) = tx
            .output
            .iter()
            .partition(|out| out.script_pubkey.is_op_return());
        assert_eq!(
            carriers.len(),
            1,
            "exactly one spendable output: no change under a drain"
        );
        assert_eq!(memos.len(), 1, "the memo is on-chain");
        assert_eq!(
            memos[0].value, 0,
            "a data carrier claims none of the drained balance"
        );
        assert!(
            memos[0].script_pubkey.as_bytes().ends_with(memo),
            "the OP_RETURN carries the caller's memo bytes"
        );
        assert_eq!(carriers[0].script_pubkey, destination.script_pubkey());
        assert!(
            finalized.fee() > 0,
            "the memo bytes are priced into a real fee"
        );
        assert_eq!(
            carriers[0].value,
            10_000_000 - finalized.fee(),
            "the destination receives the fixture's whole balance minus the fee"
        );
    }

    /// `reservation_only` end to end IN THIS WORKSPACE. key-wallet covers
    /// `add_funding_reservation_only` on its own side, but only this proves
    /// the flag survives the crossing: it travels an FFI struct field, a
    /// finalizer bool, and a key-wallet call, and a refactor that drops it
    /// anywhere along that path leaves every other test in this repo green
    /// while silently readmitting the >500-input build the flag exists to
    /// prevent.
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

        let only_seeded = || FinalizeOptions {
            inputs: vec![seeded.outpoint],
            reservation_only: true,
        };

        // A payment neither UTXO covers alone. With the flag the unseeded one
        // is not a candidate, so selection must come up short.
        let err = core
            .finalize_transaction_with_options(
                payment_builder(60),
                only_seeded(),
                &[preference(account_type)],
                0,
                &signer,
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
                    .add_output(&DashAddress::dummy(Network::Testnet, 61), 500_000),
                only_seeded(),
                &[preference(account_type)],
                0,
                &signer,
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
                FinalizeOptions::default(),
                &[preference(account_type)],
                0,
                &signer,
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

    /// Mark every coin of BIP44 account 0 as the network sees it now: neither
    /// mined nor InstantSend-locked (`final_ = false`), or InstantSend-locked.
    async fn set_bip44_finality(
        manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: &WalletId,
        final_: bool,
    ) {
        let mut manager = manager.write().await;
        let (_, info) = manager.get_wallet_and_info_mut(wallet_id).expect("wallet");
        let account = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let managed = info
            .core_wallet
            .accounts
            .funds_account_mut(&account)
            .expect("bip44 account");
        for utxo in managed.utxos.values_mut() {
            utxo.is_confirmed = false;
            utxo.is_instantlocked = final_;
        }
    }

    async fn dual_core(
        bip44: &[u64],
        bip32: &[u64],
    ) -> (
        CoreWallet<AlwaysOkBroadcaster>,
        WalletSigner,
        Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        WalletId,
    ) {
        let (manager, wallet_id, generation, signer) =
            funded_wallet_manager_dual_standard(bip44, bip32).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            Arc::clone(&manager),
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );
        (core, signer, manager, wallet_id)
    }

    /// The change of a send that never reached the network never becomes
    /// final; a payment built on it is one no node accepts. Such coins are
    /// neither selected nor offered as spendable or as max.
    #[tokio::test]
    async fn should_not_offer_a_coin_the_network_has_not_confirmed() {
        let (core, _signer, manager, wallet_id) = dual_core(&[700_000], &[700_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;

        assert_eq!(
            core.pooled_spendable_balance(&SEND_FUNDING_SOURCES, 0)
                .await
                .expect("spendable"),
            700_000,
            "only the BIP32 coin is final"
        );
        let max = core
            .pooled_max_sendable(&SEND_FUNDING_SOURCES, 0, None)
            .await
            .expect("max");
        assert!(
            max < 700_000,
            "max must price only the final coin, got {max}"
        );
    }

    /// A payment only the not-yet-final coins would cover is reported as
    /// waiting on the network, not as a shortfall, and builds nothing — when
    /// the build can be made again for the trial. A plain builder passed to
    /// `finalize_transaction` cannot, so there it stays insufficient funds.
    #[tokio::test]
    async fn should_report_waiting_when_unconfirmed_coins_would_cover_the_payment() {
        let (core, signer, manager, wallet_id) = dual_core(&[700_000], &[700_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;

        let unpriced = core
            .finalize_transaction(payment_builder(69), &SEND_FUNDING_SOURCES, 0, &signer)
            .await;
        assert!(
            matches!(
                unpriced,
                Err(PlatformWalletError::CorePooledInsufficientFunds { .. })
            ),
            "got {unpriced:?}"
        );

        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(70)),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        match result {
            Err(PlatformWalletError::CoreFundsAwaitingNetwork {
                available, waiting, ..
            }) => {
                assert_eq!(available, Some(700_000));
                assert!(waiting > 600_000 && waiting <= 700_000, "waiting {waiting}");
            }
            other => panic!("expected CoreFundsAwaitingNetwork, got {other:?}"),
        }
    }

    /// Once its transaction is InstantSend-locked the coin is spendable again.
    #[tokio::test]
    async fn should_spend_the_coin_once_it_is_instant_locked() {
        let (core, signer, manager, wallet_id) = dual_core(&[700_000], &[700_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        assert!(core
            .finalize_transaction(payment_builder(71), &SEND_FUNDING_SOURCES, 0, &signer,)
            .await
            .is_err());

        set_bip44_finality(&manager, &wallet_id, true).await;
        let finalized = core
            .finalize_transaction(payment_builder(72), &SEND_FUNDING_SOURCES, 0, &signer)
            .await
            .expect("an InstantSend-locked coin is final");
        core.abandon_transaction(&finalized).await;
    }

    /// A coin the configuration seeds on the builder itself is one candidate
    /// in the trial, not two: 600,000 final (seeded) + 100,000 waiting cannot
    /// fund 1,000,000 even once confirmed, so it stays insufficient funds.
    #[tokio::test]
    async fn should_not_count_a_builder_seeded_coin_twice_in_the_trial() {
        let (core, signer, manager, wallet_id) = dual_core(&[100_000], &[600_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let seeded = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(91).add_inputs([seeded.clone()])),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CorePooledInsufficientFunds { .. })
            ),
            "got {result:?}"
        );
    }

    /// The build refuses an input an in-flight broadcast pins, so a trial
    /// that needs one promises nothing: 600,000 final (pinned) + 500,000
    /// waiting against 1,000,000 stays insufficient funds. With nothing
    /// pinned, the same wallet is waiting on the network.
    #[tokio::test]
    async fn should_not_promise_a_coin_an_inflight_broadcast_pins() {
        let (core, signer, manager, wallet_id) = dual_core(&[500_000], &[600_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let pinned = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        let dispatch = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: pinned.outpoint,
                ..TxIn::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let pin = core.test_generation_marker().pin_in_broadcast(&dispatch);
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(92)),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CorePooledInsufficientFunds { .. })
            ),
            "got {result:?}"
        );

        drop(pin);
        // Control: the same wallet shape with nothing pinned.
        let (core, signer, manager, wallet_id) = dual_core(&[500_000], &[600_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(93)),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ),
            "got {result:?}"
        );
    }

    /// A pinned coin in the account is passed over, not fatal: 600,000 final
    /// (pinned) + 1,500,000 waiting against 1,000,000 — the waiting coin alone
    /// funds it once final, so it is waiting on the network. Chosen as an
    /// input, the pinned coin is refused by name.
    #[tokio::test]
    async fn should_pass_over_a_pinned_coin_in_the_trial() {
        let (core, signer, manager, wallet_id) = dual_core(&[1_500_000], &[600_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let pinned = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        let dispatch = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: pinned.outpoint,
                ..TxIn::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let _pin = core.test_generation_marker().pin_in_broadcast(&dispatch);
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(94)),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork {
                    waiting: 1_500_000,
                    ..
                })
            ),
            "got {result:?}"
        );

        // Chosen by outpoint, the pinned coin is refused by name, as the
        // build would refuse it after selection.
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(95)),
                FinalizeOptions {
                    inputs: vec![pinned.outpoint],
                    reservation_only: false,
                },
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::InputMidBroadcast { outpoint }) if outpoint == pinned.outpoint
            ),
            "got {result:?}"
        );
    }

    /// A chosen input that an in-flight broadcast pins and a reorg has since
    /// demoted is refused as mid-broadcast, not as waiting: confirmation would
    /// not free it.
    #[tokio::test]
    async fn should_refuse_a_pinned_demoted_input_as_mid_broadcast() {
        let (core, signer, manager, wallet_id) = dual_core(&[1_500_000], &[600_000]).await;
        let chosen = bip44_coin(&core).await;
        let dispatch = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: chosen.outpoint,
                ..TxIn::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let _pin = core.test_generation_marker().pin_in_broadcast(&dispatch);
        set_bip44_finality(&manager, &wallet_id, false).await;
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(96)),
                FinalizeOptions {
                    inputs: vec![chosen.outpoint],
                    reservation_only: false,
                },
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::InputMidBroadcast { outpoint }) if outpoint == chosen.outpoint
            ),
            "got {result:?}"
        );
    }

    /// A chosen input the funding accounts don't hold is refused by name in
    /// both funding modes: another account's coin, and one spent between being
    /// chosen and the build.
    #[tokio::test]
    async fn should_refuse_a_chosen_input_the_funding_accounts_do_not_hold() {
        let (core, signer, manager, wallet_id) = dual_core(&[1_500_000], &[700_000]).await;
        let outside = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        let spent = bip44_coin(&core).await;
        {
            // Spent since it was chosen: gone from the account.
            let mut manager = manager.write().await;
            let (_, info) = manager.get_wallet_and_info_mut(&wallet_id).expect("wallet");
            info.core_wallet
                .first_bip44_managed_account_mut()
                .expect("bip44 account")
                .utxos
                .remove(&spent.outpoint);
        }
        let sources = [preference(StandardAccountType::BIP44Account)];
        for chosen in [outside.outpoint, spent.outpoint] {
            for reservation_only in [false, true] {
                let result = core
                    .finalize_transaction_from(
                        || Ok(payment_builder(97)),
                        FinalizeOptions::default()
                            .with_inputs([chosen])
                            .with_reservation_only(reservation_only),
                        &sources,
                        0,
                        &signer,
                    )
                    .await;
                assert!(
                    matches!(
                        result,
                        Err(PlatformWalletError::ChosenInputUnavailable {
                            outpoint,
                            problem: ChosenInputProblem::NotInFundingAccounts,
                        }) if outpoint == chosen
                    ),
                    "{chosen} reservation_only={reservation_only}: got {result:?}"
                );
            }
        }
    }

    /// The trial leaves nothing behind: after a code 59, every standard
    /// account's next change address is the one it was, and the final coin
    /// the trial selected is free at once — a payment it covers builds.
    #[tokio::test]
    async fn should_leave_no_reservation_or_change_address_behind_after_the_trial() {
        let (core, signer, manager, wallet_id) = dual_core(&[700_000], &[700_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let next_change = || async {
            let mut manager = manager.write().await;
            let (wallet, info) = manager.get_wallet_and_info_mut(&wallet_id).expect("wallet");
            let mut addresses = Vec::new();
            for standard_account_type in [
                StandardAccountType::BIP44Account,
                StandardAccountType::BIP32Account,
            ] {
                let at = AccountType::Standard {
                    index: 0,
                    standard_account_type,
                };
                let xpub = wallet
                    .accounts
                    .account_of_type(at)
                    .expect("account")
                    .account_xpub;
                let managed = info
                    .core_wallet
                    .accounts
                    .funds_account_mut(&at)
                    .expect("managed account");
                addresses.push(
                    managed
                        .next_change_address(Some(&xpub), false)
                        .expect("change address"),
                );
            }
            addresses
        };
        let before = next_change().await;

        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(99)),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ),
            "got {result:?}"
        );
        assert_eq!(next_change().await, before, "no change pool moved");

        let built = core
            .finalize_transaction(
                TransactionBuilder::new()
                    .add_output(&DashAddress::dummy(Network::Testnet, 100), 500_000),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await
            .expect("the final coin is not left reserved by the trial");
        core.abandon_transaction(&built).await;
    }

    /// The trial costs nothing it does not need: a shortfall with no waiting
    /// coin, or a `reservation_only` one, makes the configuration once (the
    /// build); one with a waiting coin makes it twice (the build and the
    /// trial), never more.
    #[tokio::test]
    async fn should_make_the_trial_only_when_a_coin_is_waiting() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let count = |core: &CoreWallet<AlwaysOkBroadcaster>,
                     signer: &WalletSigner,
                     options: FinalizeOptions,
                     sources: Vec<AccountTypePreference>| {
            let made = AtomicUsize::new(0);
            let core = core.clone();
            let signer = signer.clone();
            async move {
                let result = core
                    .finalize_transaction_from(
                        || {
                            made.fetch_add(1, Ordering::SeqCst);
                            Ok(payment_builder(101))
                        },
                        options,
                        &sources,
                        0,
                        &signer,
                    )
                    .await;
                assert!(result.is_err(), "a shortfall: {result:?}");
                made.load(Ordering::SeqCst)
            }
        };

        // Nothing waiting: 600,000 final against 1,000,000.
        let (core, signer, _manager, _wallet_id) = dual_core(&[100], &[600_000]).await;
        assert_eq!(
            count(
                &core,
                &signer,
                FinalizeOptions::default(),
                SEND_FUNDING_SOURCES.to_vec()
            )
            .await,
            1
        );

        // A waiting coin: built, then tried.
        let (core, signer, manager, wallet_id) = dual_core(&[700_000], &[700_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        assert_eq!(
            count(
                &core,
                &signer,
                FinalizeOptions::default(),
                SEND_FUNDING_SOURCES.to_vec()
            )
            .await,
            2
        );

        // `reservation_only`: no trial even with a coin waiting.
        let seed = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        assert_eq!(
            count(
                &core,
                &signer,
                FinalizeOptions::default()
                    .with_inputs([seed.outpoint])
                    .with_reservation_only(true),
                SEND_FUNDING_SOURCES.to_vec(),
            )
            .await,
            1
        );
    }

    /// Too many inputs is waiting on the network when a coin that is not final
    /// yet would fund it with few: 600 final coins of 2,000 duffs need more
    /// than 500 inputs for 1,100,000, but one waiting coin of 1,500,000 pays it
    /// alone once final.
    #[tokio::test]
    async fn should_report_waiting_when_too_many_inputs_would_shrink_once_a_coin_is_final() {
        let (core, signer, manager, wallet_id) = dual_core(&[1_500_000], &vec![2_000; 600]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let make = || {
            Ok(TransactionBuilder::new()
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .add_output(&DashAddress::dummy(Network::Testnet, 102), 1_100_000))
        };

        let plain = core
            .finalize_transaction(make().expect("builder"), &SEND_FUNDING_SOURCES, 0, &signer)
            .await;
        assert!(
            matches!(plain, Err(PlatformWalletError::TransactionBuild(ref message))
                if message.contains("inputs")),
            "without the trial it is the input limit: got {plain:?}"
        );

        let result = core
            .finalize_transaction_from(
                make,
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork {
                    waiting: 1_500_000,
                    ..
                })
            ),
            "got {result:?}"
        );
    }

    /// A trial that needs a coin no funding account holds promises nothing:
    /// the factory seeds a final 700,000-duff BIP32 coin while only BIP44
    /// (one waiting 700,000-duff coin) funds a 1,000,000 payment. The trial
    /// would spend both, but the build refuses the BIP32 input, so
    /// confirmation would not help: not code 59, but the typed refusal naming
    /// the seeded coin.
    #[tokio::test]
    async fn should_not_report_waiting_on_a_trial_input_outside_the_funding_accounts() {
        let (core, signer, manager, wallet_id) = dual_core(&[700_000], &[700_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let outside = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(103).add_inputs([outside.clone()])),
                FinalizeOptions::default(),
                &[preference(StandardAccountType::BIP44Account)],
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::ChosenInputUnavailable {
                    outpoint,
                    problem: ChosenInputProblem::NotInFundingAccounts,
                }) if outpoint == outside.outpoint
            ),
            "the seeded coin is what stands in the way: got {result:?}"
        );
    }

    /// A coin seeded on the builder that no funding account holds, once
    /// selected (a drain takes every candidate), is the typed refusal naming
    /// it — and the build's reservation is released: the same drain without
    /// it builds at once.
    #[tokio::test]
    async fn should_refuse_a_selected_builder_seed_outside_the_funding_accounts() {
        let (core, signer, _manager, _wallet_id) = dual_core(&[1_500_000], &[700_000]).await;
        let outside = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        let drain = |tag| {
            TransactionBuilder::new()
                .set_selection_strategy(SelectionStrategy::All)
                .add_output(&DashAddress::dummy(Network::Testnet, tag), 0)
        };
        let sources = [preference(StandardAccountType::BIP44Account)];

        let result = core
            .finalize_transaction(
                drain(104).add_inputs([outside.clone()]),
                &sources,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::ChosenInputUnavailable {
                    outpoint,
                    problem: ChosenInputProblem::NotInFundingAccounts,
                }) if outpoint == outside.outpoint
            ),
            "got {result:?}"
        );

        let built = core
            .finalize_transaction(drain(105), &sources, 0, &signer)
            .await
            .expect("nothing of the refused build stays reserved");
        core.abandon_transaction(&built).await;
    }

    /// A factory that seeds the same final coin twice: 2 × 400,000 duffs
    /// (BIP32) plus one waiting 300,000-duff BIP44 coin would look like
    /// 1,100,000 to a trial, but the wallet holds 700,000 and the build
    /// refuses the duplicate input — the trial does too, instead of code 59.
    #[tokio::test]
    async fn should_not_report_waiting_on_a_trial_that_spends_a_seed_twice() {
        let (core, signer, manager, wallet_id) = dual_core(&[300_000], &[400_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let seeded = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        let result = core
            .finalize_transaction_from(
                || {
                    Ok(payment_builder(108)
                        .set_selection_strategy(SelectionStrategy::LargestFirst)
                        .add_inputs([seeded.clone(), seeded.clone()]))
                },
                FinalizeOptions::default(),
                &[AccountTypePreference::BIP44, AccountTypePreference::BIP32],
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBuild(ref message))
                if message.contains("twice")),
            "got {result:?}"
        );
    }

    /// The same refusals when the build pools several sources (BIP44 and
    /// DashPay receiving funds, BIP32 left out): the trial that needs the
    /// outside seed, and the drain that selects it.
    #[tokio::test]
    async fn should_refuse_an_outside_builder_seed_in_a_pooled_build() {
        let pooled = [
            AccountTypePreference::BIP44,
            AccountTypePreference::AllDashpayReceivingFunds,
        ];
        let refused = |result: &Result<_, PlatformWalletError>, seeded: dashcore::OutPoint| {
            matches!(
                result,
                Err(PlatformWalletError::ChosenInputUnavailable {
                    outpoint,
                    problem: ChosenInputProblem::NotInFundingAccounts,
                }) if *outpoint == seeded
            )
        };

        let (core, signer, manager, wallet_id) = dual_core(&[700_000], &[700_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let outside = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(106).add_inputs([outside.clone()])),
                FinalizeOptions::default(),
                &pooled,
                0,
                &signer,
            )
            .await;
        assert!(refused(&result, outside.outpoint), "trial: got {result:?}");

        let (core, signer, _manager, _wallet_id) = dual_core(&[1_500_000], &[700_000]).await;
        let outside = standard_account_coins(&core, StandardAccountType::BIP32Account)
            .await
            .into_iter()
            .next()
            .expect("bip32 coin");
        let result = core
            .finalize_transaction(
                TransactionBuilder::new()
                    .set_selection_strategy(SelectionStrategy::All)
                    .add_output(&DashAddress::dummy(Network::Testnet, 107), 0)
                    .add_inputs([outside.clone()]),
                &pooled,
                0,
                &signer,
            )
            .await;
        assert!(refused(&result, outside.outpoint), "drain: got {result:?}");
    }

    /// A payment the unconfirmed coins would not cover either stays a plain
    /// shortfall: waiting would not help.
    #[tokio::test]
    async fn should_keep_insufficient_funds_when_waiting_would_not_help() {
        let (core, signer, manager, wallet_id) = dual_core(&[700_000], &[700_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let builder = || {
            TransactionBuilder::new()
                .add_output(&DashAddress::dummy(Network::Testnet, 73), 5_000_000)
        };
        let result = core
            .finalize_transaction_from(
                || Ok(builder()),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CorePooledInsufficientFunds { .. })
            ),
            "got {result:?}"
        );
    }

    /// key-wallet can report a shortfall before pricing fees (`required` is
    /// then only the outputs). 900,000 final + 100,148 waiting do not cover a
    /// 1,000,000 payment once the two inputs and the fee are paid: waiting
    /// would not help, so it stays insufficient funds.
    #[tokio::test]
    async fn should_count_the_fee_before_calling_a_shortfall_waiting() {
        let (core, signer, manager, wallet_id) = dual_core(&[100_148], &[900_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let builder = || {
            TransactionBuilder::new()
                .add_output(&DashAddress::dummy(Network::Testnet, 75), 1_000_000)
        };
        let result = core
            .finalize_transaction_from(
                || Ok(builder()),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CorePooledInsufficientFunds { .. })
            ),
            "got {result:?}"
        );
    }

    /// With nothing final at all key-wallet names no requirement, and the
    /// trial over the waiting coin funds the payment: code 59, with the coin
    /// the trial spends as waiting and no invented requirement.
    #[tokio::test]
    async fn should_report_waiting_when_no_coin_is_final() {
        let (manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            Arc::clone(&manager),
            wallet_id,
            Arc::new(AlwaysOkBroadcaster),
            generation,
        );
        set_bip44_finality(&manager, &wallet_id, false).await;
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(74)),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork {
                    available: Some(0),
                    waiting: 10_000_000,
                    required: None,
                    ..
                })
            ),
            "got {result:?}"
        );
    }

    /// A high fee rate: 900,000 final + 101,000 waiting against 1,000,000 at
    /// 10,000 duffs/kB. Confirmation could not fund it (1,001,000 is short of
    /// the payment plus even the 3,400-duff no-change fee): insufficient funds,
    /// not code 59 — through a plain builder (no trial) and through the trial
    /// alike. At the default rate the trial builds: code 59.
    #[tokio::test]
    async fn should_not_report_waiting_for_a_build_it_cannot_price() {
        let (core, signer, manager, wallet_id) = dual_core(&[101_000], &[900_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let high_rate = |tag| payment_builder(tag).set_fee_rate(FeeRate::new(10_000));
        let is_short = |result: &Result<_, PlatformWalletError>| {
            matches!(
                result,
                Err(PlatformWalletError::CorePooledInsufficientFunds { .. })
            )
        };

        let opaque = core
            .finalize_transaction(high_rate(86), &SEND_FUNDING_SOURCES, 0, &signer)
            .await;
        assert!(is_short(&opaque), "got {opaque:?}");

        let described = core
            .finalize_transaction_from(
                || Ok(high_rate(87)),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(is_short(&described), "got {described:?}");

        let default_rate = core
            .finalize_transaction_from(
                || Ok(payment_builder(88)),
                FinalizeOptions::default(),
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                default_rate,
                Err(PlatformWalletError::CoreFundsAwaitingNetwork { .. })
            ),
            "got {default_rate:?}"
        );
    }

    /// The BIP44 coin of a `dual_core` wallet, as the wallet holds it now.
    async fn bip44_coin<B: TransactionBroadcaster>(core: &CoreWallet<B>) -> key_wallet::Utxo {
        standard_account_coins(core, StandardAccountType::BIP44Account)
            .await
            .into_iter()
            .next()
            .expect("bip44 coin")
    }

    /// A coin chosen by outpoint that is not final is refused by name, with
    /// ordinary and with `reservation_only` funding — not dropped silently.
    /// Not chosen, a `reservation_only` build has nothing waiting to spend:
    /// insufficient funds.
    #[tokio::test]
    async fn should_refuse_an_input_chosen_by_outpoint_that_is_not_final_yet() {
        let (core, signer, manager, wallet_id) = dual_core(&[1_500_000], &[700_000]).await;
        set_bip44_finality(&manager, &wallet_id, false).await;
        let seeded = bip44_coin(&core).await;
        let sources = [preference(StandardAccountType::BIP44Account)];

        for reservation_only in [false, true] {
            let result = core
                .finalize_transaction_from(
                    || Ok(payment_builder(76)),
                    FinalizeOptions {
                        inputs: vec![seeded.outpoint],
                        reservation_only,
                    },
                    &sources,
                    0,
                    &signer,
                )
                .await;
            assert!(
                matches!(
                    result,
                    Err(PlatformWalletError::CoreFundsAwaitingNetwork {
                        outpoint: Some(outpoint),
                        waiting: 1_500_000,
                        ..
                    }) if outpoint == seeded.outpoint
                ),
                "reservation_only={reservation_only}: got {result:?}"
            );
        }

        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(77)),
                FinalizeOptions {
                    reservation_only: true,
                    ..FinalizeOptions::default()
                },
                &sources,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::CoreInsufficientFunds { .. })
            ),
            "got {result:?}"
        );
    }

    /// A coin the caller chose while it was not final, InstantSend-locked
    /// before the build is finalized, is spent: finalization judges the
    /// wallet's copy as it is now, not the caller's snapshot — with ordinary
    /// and with `reservation_only` funding.
    #[tokio::test]
    async fn should_spend_an_input_chosen_before_it_became_final() {
        for reservation_only in [false, true] {
            let (core, signer, manager, wallet_id) = dual_core(&[1_500_000], &[700_000]).await;
            set_bip44_finality(&manager, &wallet_id, false).await;
            let chosen = bip44_coin(&core).await;
            assert!(!is_final(&chosen), "chosen while not final");
            set_bip44_finality(&manager, &wallet_id, true).await;

            let finalized = core
                .finalize_transaction_from(
                    || Ok(payment_builder(85)),
                    FinalizeOptions {
                        inputs: vec![chosen.outpoint],
                        reservation_only,
                    },
                    &[preference(StandardAccountType::BIP44Account)],
                    0,
                    &signer,
                )
                .await
                .unwrap_or_else(|error| panic!("reservation_only={reservation_only}: {error:?}"));
            let inputs = &finalized.transaction().input;
            assert_eq!(inputs.len(), 1, "reservation_only={reservation_only}");
            assert_eq!(inputs[0].previous_output, chosen.outpoint);
            core.abandon_transaction(&finalized).await;
        }
    }

    /// A coin seeded while final and demoted before finalization (a reorg) is
    /// refused on its current status by name, before anything is signed, and
    /// leaves no reservation behind — with ordinary and with
    /// `reservation_only` funding, whether it was chosen by outpoint or seeded
    /// on the builder as the caller's snapshot (caught after selection).
    #[tokio::test]
    async fn should_refuse_a_seeded_coin_demoted_before_finalization() {
        for (reservation_only, by_outpoint) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let case = format!("reservation_only={reservation_only} by_outpoint={by_outpoint}");
            let (core, signer, manager, wallet_id) = dual_core(&[1_500_000], &[700_000]).await;
            let seeded = bip44_coin(&core).await;
            assert!(is_final(&seeded), "seeded while final");
            let build = |tag| {
                let builder = payment_builder(tag);
                if by_outpoint {
                    (
                        builder,
                        FinalizeOptions {
                            inputs: vec![seeded.outpoint],
                            reservation_only,
                        },
                    )
                } else {
                    (
                        builder.add_inputs([seeded.clone()]),
                        FinalizeOptions {
                            reservation_only,
                            ..FinalizeOptions::default()
                        },
                    )
                }
            };
            set_bip44_finality(&manager, &wallet_id, false).await;
            let sources = [preference(StandardAccountType::BIP44Account)];

            // The failing signer proves signing never ran: reaching it would
            // turn the result into a build error.
            let (builder, options) = build(78);
            let result = core
                .finalize_transaction_with_options(builder, options, &sources, 0, &FailingSigner)
                .await;
            assert!(
                matches!(
                    result,
                    Err(PlatformWalletError::CoreFundsAwaitingNetwork { outpoint: Some(outpoint), .. })
                        if outpoint == seeded.outpoint
                ),
                "{case}: got {result:?}"
            );

            // Nothing stays reserved: once final again the same coin builds.
            set_bip44_finality(&manager, &wallet_id, true).await;
            let (builder, options) = build(79);
            core.finalize_transaction_with_options(builder, options, &sources, 0, &signer)
                .await
                .unwrap_or_else(|error| panic!("{case}: {error:?}"));
        }
    }

    /// A final coin chosen by outpoint that is not spendable (locked here; an
    /// immature coinbase output alike) is refused by name, not dropped.
    #[tokio::test]
    async fn should_refuse_an_input_chosen_by_outpoint_that_is_not_spendable() {
        let (core, signer, manager, wallet_id) = dual_core(&[1_500_000], &[700_000]).await;
        let chosen = bip44_coin(&core).await;
        {
            let mut manager = manager.write().await;
            let (_, info) = manager.get_wallet_and_info_mut(&wallet_id).expect("wallet");
            let managed = info
                .core_wallet
                .first_bip44_managed_account_mut()
                .expect("bip44 account");
            managed
                .utxos
                .get_mut(&chosen.outpoint)
                .expect("chosen coin")
                .is_locked = true;
        }
        let result = core
            .finalize_transaction_from(
                || Ok(payment_builder(90)),
                FinalizeOptions {
                    inputs: vec![chosen.outpoint],
                    ..FinalizeOptions::default()
                },
                &SEND_FUNDING_SOURCES,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::ChosenInputUnavailable {
                    outpoint,
                    problem: ChosenInputProblem::NotSpendable,
                }) if outpoint == chosen.outpoint
            ),
            "got {result:?}"
        );
    }

    /// The contact payment reports its shortfall through the same rule.
    #[test]
    fn should_map_a_contact_shortfall_the_waiting_coins_cover() {
        let covered = build_error_awaiting_network(
            BuilderError::InsufficientFunds {
                available: 100_000,
                required: 500_000,
            },
            Some(600_000),
        );
        assert!(matches!(
            covered,
            PlatformWalletError::CoreFundsAwaitingNetwork {
                available: Some(100_000),
                required: Some(500_000),
                ..
            }
        ));
        let short = build_error_awaiting_network(
            BuilderError::InsufficientFunds {
                available: 100_000,
                required: 5_000_000,
            },
            None,
        );
        assert!(matches!(short, PlatformWalletError::TransactionBuild(_)));
    }

    /// The message a host may still surface reads in DASH, with no Rust
    /// `Option` debug text.
    #[test]
    fn should_word_the_waiting_error_in_dash() {
        let message = PlatformWalletError::CoreFundsAwaitingNetwork {
            available: Some(0),
            waiting: 85_998_722,
            required: Some(1_000_078),
            outpoint: None,
        }
        .to_string();
        assert_eq!(
            message,
            "Core funds are waiting for network confirmation: 0.85998722 DASH not yet confirmed, \
             available 0 DASH, needed 0.01000078 DASH"
        );
        assert!(!message.contains("Some("));
    }
}
