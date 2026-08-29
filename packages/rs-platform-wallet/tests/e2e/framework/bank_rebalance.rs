//! The operator maintains exactly one address balance: the bank's
//! Platform address (`tdash1kzz…` via
//! [`BankWallet::primary_receive_address`]). The harness auto-rebalances
//! Platform-to-Core internally as needed. Everything else is
//! implementation detail.
//!
//! Three helpers preserve this invariant at suite start:
//!
//! 1. [`provision_transfer_key_if_missing`] — ensures the bank identity
//!    advertises a `Purpose::TRANSFER` / `SecurityLevel::CRITICAL` key
//!    so [`drain_bank_identity_to_addresses`] can use the
//!    `IdentityCreditTransferToAddresses` primitive. Production bank
//!    identities registered before the bank-flow refactor only carry
//!    AUTHENTICATION keys (DPP rejected such drains with `missing key:
//!    no transfer public key`). Idempotent; the helper short-circuits
//!    once the key is present.
//!
//! 2. [`drain_bank_identity_to_addresses`] — any credits accumulated on
//!    the bank identity (legacy + transient mid-run sinks) are moved
//!    back to the Platform address via the fast Platform-only
//!    `transfer_credits_to_addresses_with_external_signer` primitive.
//!
//! 3. [`refill_core_from_platform_if_below_threshold`] — if the bank's
//!    L1 Core balance is below the configured threshold, refill it from
//!    the Platform address via a (slow) Platform→Core withdrawal,
//!    chained `top_up_from_addresses` → `withdraw_credits_with_external_signer`.
//!    Gated by the threshold so the slow path runs only when needed.
//!
//! After this refactor lands, the operator can ignore the Core address
//! and the bank identity. Only the Platform address needs external
//! top-ups.

use std::collections::BTreeMap;
use std::time::Duration;

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, Purpose, SecurityLevel};
use key_wallet::bip32::ExtendedPrivKey;

use super::bank::BankWallet;
use super::bank_identity::BankIdentity;
use super::config::Config;
use super::signer::{derive_identity_key, SeedBackedCoreSigner};
use super::wait::wait_for_identity_balance;
use super::wallet_factory::{default_fee_strategy, DEFAULT_ACCOUNT_INDEX_PUB};
use super::{FrameworkError, FrameworkResult};

/// Headroom kept on the bank identity after a Platform-side drain so a
/// follow-up `transfer_credits_to_addresses` (or core-refill chain) has
/// budget for its on-chain fee. Mirrors `IDENTITY_SWEEP_FEE_RESERVE` in
/// [`super::cleanup`] — empirically ~12-15M on testnet, 30M is generous.
const BANK_IDENTITY_DRAIN_FEE_RESERVE: Credits = 30_000_000;

/// 1 Core duff = 1000 Platform credits. Used by the core-refill chain to
/// translate duff thresholds / targets into credit-denominated transition
/// amounts, and by [`super::bank_plan`] for cross-type surplus math.
pub const CREDITS_PER_DUFF: u64 = 1_000;

/// Credit headroom kept on Platform beyond the bare deficit when an
/// asset-lock bootstrap funds it, so the immediately-following transition
/// fees don't re-underflow Platform. Shared by the planner's E5 sizing
/// ([`super::bank_plan`]) and the bank-identity bootstrap self-fund.
pub const PLATFORM_BOOTSTRAP_FEE_RESERVE: Credits = 100_000_000;

/// Credits the asset-lock address-funding transition itself burns, deducted
/// (`ReduceOutput(0)`) from the locked amount BEFORE it lands on the
/// recipient. Live paloma runs show this fee is ~93M credits, so the gross
/// lock must exceed the target by at least this much or the NET underflows
/// it. Shared by both sizing paths (planner E5 + bootstrap self-fund);
/// over-locking only leaves the bank more usable Platform balance.
pub const BOOTSTRAP_ASSET_LOCK_FEE_RESERVE: Credits = 150_000_000;

/// Core duffs to asset-lock so a Platform balance of `current_credits`
/// reaches `target_credits`. Ceil-divides the credit shortfall by
/// [`CREDITS_PER_DUFF`] so rounding never undershoots the target; returns
/// `0` when the balance already covers it. The single sizing rule both
/// the planner's E5 move and the bank-identity bootstrap use.
pub const fn bootstrap_lock_duff(current_credits: Credits, target_credits: Credits) -> u64 {
    target_credits
        .saturating_sub(current_credits)
        .div_ceil(CREDITS_PER_DUFF)
}

/// Net Platform credits that land on the recipient after a gross asset-lock
/// of `lock_duff` duffs, once the funding transition's own
/// [`BOOTSTRAP_ASSET_LOCK_FEE_RESERVE`] fee is deducted. The single
/// net-credit model both the planner's E5 sizing and the bootstrap
/// self-fund use to reason about post-lock balances. Saturates to `0` when
/// the lock is too small to cover its own fee.
pub fn bootstrap_lock_net_credits(lock_duff: u64) -> Credits {
    lock_duff
        .saturating_mul(CREDITS_PER_DUFF)
        .saturating_sub(BOOTSTRAP_ASSET_LOCK_FEE_RESERVE)
}

/// Measured Core spend of one full e2e pass: ~13 tDASH ≈ 1.3e9 duffs
/// (1 DASH = 1e8 duffs). All refill sizing below is derived from this.
const CORE_BURN_PER_FULL_PASS_DUFF: u64 = 1_300_000_000;

/// Default trip line for the core-refill fallback. Below this many duffs
/// of confirmed Core balance the harness rebalances Platform→Core so
/// CR-* / ID-007 cases have working capital. Sized at one full pass plus
/// ~0.5-pass margin (~2e9 duffs ≈ 20 tDASH) so the bank is topped up
/// before it can run dry mid-pass. Overrideable via
/// [`super::config::vars::CORE_REFILL_THRESHOLD_DUFF`].
pub const DEFAULT_CORE_REFILL_THRESHOLD_DUFF: u64 = 2_000_000_000;

/// Default target balance (duffs) the core-refill chain aims to reach
/// when triggered. Sized at ~3.8 full passes (~5e9 duffs ≈ 50 tDASH) so
/// a single (slow) Platform→Core withdrawal buys several passes of
/// runway. Overrideable via
/// [`super::config::vars::CORE_REFILL_TARGET_DUFF`].
pub const DEFAULT_CORE_REFILL_TARGET_DUFF: u64 = 5_000_000_000;

/// Hard floor for the setup preflight: the minimum confirmed Core
/// balance needed to fund even a single full pass. If the bank is below
/// this *after* the setup refill attempt, the run is doomed and the
/// harness fails fast instead of burning a network slot on a guaranteed
/// mid-pass starvation.
pub const CORE_REFILL_OPERATIONAL_MIN_DUFF: u64 = CORE_BURN_PER_FULL_PASS_DUFF;

/// Identity-side fee reserve added on top of the desired core-refill
/// credit amount when topping up the bank identity. The withdrawal that
/// follows the top-up pays its own protocol fee out of the identity's
/// balance, so the top-up must overshoot the withdrawal target by at
/// least this much.
const CORE_REFILL_IDENTITY_FEE_RESERVE: Credits = 50_000_000;

/// Deadline for the post-top-up identity-balance visibility wait inside
/// [`refill_core_from_platform_if_below_threshold`]. Sized like the
/// bank-identity bootstrap path — generous, because the helper runs once
/// per suite.
const CORE_REFILL_TOPUP_TIMEOUT: Duration = Duration::from_secs(60);

/// BIP-32 master node for the bank wallet on its network.
///
/// The bank wallet is external-signable (keyless) after registration, so
/// `load_identity_by_index`'s resident-key derive fails with "External signable
/// wallet has no private key". Every bank-identity load in this module derives
/// its probe key from this master via `load_identity_by_index_from_master`
/// instead — mirrors [`super::cleanup::sweep_identities_with_seed`].
fn bank_master(bank: &BankWallet) -> FrameworkResult<ExtendedPrivKey> {
    ExtendedPrivKey::new_master(bank.network(), bank.seed_bytes())
        .map_err(|e| FrameworkError::Bank(format!("bank master-xprv derive: {e}")))
}

/// Ensure the bank identity advertises a `Purpose::TRANSFER` /
/// `SecurityLevel::CRITICAL` key so
/// [`drain_bank_identity_to_addresses`] (which broadcasts an
/// `IdentityCreditTransferToAddresses` transition) can satisfy DPP's
/// `purpose_requirement = [TRANSFER]` gate.
///
/// Production bank identities bootstrapped before the bank-flow
/// refactor were registered with only two AUTHENTICATION keys (a
/// MASTER for IdentityUpdate-signing and a HIGH for general auth);
/// the drain then failed with `Protocol error: missing key: no
/// transfer public key`, stranding ~9.58T credits on the bank
/// identity forever.
///
/// Flow:
///   - Fetch the identity from chain.
///   - If any TRANSFER-purpose key already exists, short-circuit (the
///     helper is idempotent on subsequent runs).
///   - Otherwise derive a fresh ECDSA keypair at DIP-9
///     `(identity_index, key_index = max_existing_key_id + 1)` — the
///     same derivation tree the bootstrap MASTER/HIGH keys live on,
///     so the existing [`BankIdentity::signer`] cache already holds
///     its private bytes (pre-derived up to `DEFAULT_GAP_LIMIT`).
///   - Broadcast an `IdentityUpdate` that adds the new key, signed by
///     the bank identity's MASTER auth key.
///
/// Returns the new key's `key_id` on a successful add, `Ok(None)`
/// when the helper short-circuited (existing TRANSFER key, fetch
/// failure, or broadcast failure). Best-effort: errors are logged at
/// WARN and surfaced to the caller as `Ok(None)` so harness init can
/// continue.
pub async fn provision_transfer_key_if_missing(
    bank: &BankWallet,
    bank_identity: &BankIdentity,
) -> FrameworkResult<Option<u32>> {
    let bank_wallet = bank.platform_wallet();
    let sdk = bank_wallet.sdk();

    // Snapshot the on-chain key set — the local IdentityManager
    // cache may be empty at this point in suite init (drain runs
    // before any `load_identity_by_index` call site).
    let identity = match Identity::fetch(sdk, bank_identity.id).await {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                "transfer-key provision skipped: chain reports bank identity absent"
            );
            return Ok(None);
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                error = %err,
                "transfer-key provision skipped: bank identity fetch failed"
            );
            return Ok(None);
        }
    };

    let already_present = identity
        .public_keys()
        .values()
        .any(|key| key.purpose() == Purpose::TRANSFER && key.disabled_at().is_none());
    if already_present {
        tracing::debug!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            "transfer-key provision no-op: bank identity already advertises a TRANSFER key"
        );
        return Ok(None);
    }

    let next_key_id: u32 = identity
        .public_keys()
        .keys()
        .copied()
        .max()
        .map(|max| max.saturating_add(1))
        .unwrap_or(0);

    let new_key = match derive_identity_key(
        bank.seed_bytes(),
        bank.network(),
        bank_identity.identity_index,
        next_key_id,
        Purpose::TRANSFER,
        SecurityLevel::CRITICAL,
    ) {
        Ok(key) => key,
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                next_key_id,
                error = %err,
                "transfer-key provision skipped: deriving the new key failed"
            );
            return Ok(None);
        }
    };

    // `update_identity_with_external_signer` looks the identity up in
    // the in-process IdentityManager (the same lookup the drain
    // primitive does later), so load it once here. Any failure means
    // the manager can't pick a MASTER key to sign the update —
    // surface as a skip rather than aborting harness init.
    let master = match bank_master(bank) {
        Ok(m) => m,
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                error = %err,
                "transfer-key provision skipped: bank master-xprv derive failed"
            );
            return Ok(None);
        }
    };
    if let Err(err) = bank_wallet
        .identity()
        .load_identity_by_index_from_master(bank_identity.identity_index, &master)
        .await
    {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            identity_index = bank_identity.identity_index,
            error = %err,
            "transfer-key provision skipped: failed to load bank identity into manager"
        );
        return Ok(None);
    }

    match bank_wallet
        .identity()
        .update_identity_with_external_signer(
            &bank_identity.id,
            vec![new_key],
            vec![],
            bank_identity.signer.as_ref(),
            None,
        )
        .await
    {
        Ok(()) => {
            tracing::info!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                key_id = next_key_id,
                identity_index = bank_identity.identity_index,
                "provisioned TRANSFER key on bank identity \
                 (drain helper will now succeed on subsequent runs)"
            );
            Ok(Some(next_key_id))
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                next_key_id,
                error = %err,
                "transfer-key provision broadcast failed; \
                 drain will continue to skip until the key lands"
            );
            Ok(None)
        }
    }
}

/// Drain the bank identity's Platform credits back to
/// [`BankWallet::primary_receive_address`] via the fast Platform-only
/// `transfer_credits_to_addresses_with_external_signer` primitive.
///
/// Leaves [`BANK_IDENTITY_DRAIN_FEE_RESERVE`] on the identity to cover
/// the transfer fee plus a small headroom for any follow-up cost (e.g.
/// the core-refill chain firing immediately afterwards). No-op when the
/// bank identity's balance is at or below that reserve.
///
/// Returns the amount drained (0 if no-op). Best-effort: failures are
/// logged at WARN and surfaced to the caller for context — the harness
/// init path treats them as non-fatal.
pub async fn drain_bank_identity_to_addresses(
    bank: &BankWallet,
    bank_identity: &BankIdentity,
) -> FrameworkResult<Credits> {
    let bank_wallet = bank.platform_wallet();
    let sdk = bank_wallet.sdk();

    // Ensure the bank identity is loaded into the bank wallet's
    // IdentityManager — `transfer_credits_to_addresses_with_external_signer`
    // looks it up there. On the persisted-id load path the manager
    // would otherwise be empty for the bank slot.
    let master = match bank_master(bank) {
        Ok(m) => m,
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                error = %err,
                "drain skipped: bank master-xprv derive failed"
            );
            return Ok(0);
        }
    };
    if let Err(err) = bank_wallet
        .identity()
        .load_identity_by_index_from_master(bank_identity.identity_index, &master)
        .await
    {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            identity_index = bank_identity.identity_index,
            error = %err,
            "drain skipped: failed to load bank identity into manager"
        );
        return Ok(0);
    }

    // Authoritative balance comes from chain, not from the local cache,
    // for the same reason as `cleanup::sweep_identities_with_seed` — a
    // stale cache flips this helper between "no-op" and "over-amount
    // transfer that the chain rejects".
    let pre: Credits = match IdentityBalance::fetch(sdk, bank_identity.id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                "drain skipped: chain reports bank identity absent"
            );
            return Ok(0);
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                error = %err,
                "drain skipped: bank identity balance refresh failed"
            );
            return Ok(0);
        }
    };

    if pre <= BANK_IDENTITY_DRAIN_FEE_RESERVE {
        tracing::debug!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            pre,
            reserve = BANK_IDENTITY_DRAIN_FEE_RESERVE,
            "drain no-op: bank identity at or below fee reserve"
        );
        return Ok(0);
    }

    let amount = pre - BANK_IDENTITY_DRAIN_FEE_RESERVE;
    let outputs: BTreeMap<_, _> =
        std::iter::once((*bank.primary_receive_address(), amount)).collect();

    match bank_wallet
        .identity()
        .transfer_credits_to_addresses_with_external_signer(
            &bank_identity.id,
            outputs,
            bank_identity.signer.as_ref(),
            None,
        )
        .await
    {
        Ok((_address_infos, post, _)) => {
            tracing::info!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                pre,
                post,
                drained = amount,
                "drained bank identity credits back to bank Platform address"
            );
            Ok(amount)
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                pre,
                attempted = amount,
                error = %err,
                "bank identity drain broadcast failed; continuing without drain. \
                 IdentityCreditTransferToAddresses requires a Purpose::TRANSFER / \
                 SecurityLevel::CRITICAL key on the bank identity. \
                 `provision_transfer_key_if_missing` runs at suite start to add one; \
                 if this WARN repeats, check that helper's log line for a broadcast \
                 failure and / or add a TRANSFER key manually via dash-evo-tool. \
                 See `framework::bank_rebalance` rustdoc for the operator invariant."
            );
            Ok(0)
        }
    }
}

/// Refill the bank's Core (Layer-1) confirmed balance from the Platform
/// address pool when it dips below `threshold_duff`, targeting
/// `target_duff` afterwards. Best-effort; never fails harness init.
///
/// Chain: `top_up_from_addresses` (bank address → bank identity)
/// followed by `withdraw_credits_with_external_signer` (bank identity →
/// bank Core address). The withdrawal is the canonical slow path — it
/// rides the Core withdrawal pool — so it's gated behind the
/// `threshold_duff` check to avoid the cost on every run.
///
/// Returns the duff amount the withdrawal was issued for (0 when the
/// helper short-circuited because the balance was above the threshold or
/// because any sub-step failed).
pub async fn refill_core_from_platform_if_below_threshold(
    bank: &BankWallet,
    bank_identity: &BankIdentity,
    threshold_duff: u64,
    target_duff: u64,
) -> FrameworkResult<u64> {
    if target_duff <= threshold_duff {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            threshold_duff,
            target_duff,
            "core-refill skipped: misconfigured (target must exceed threshold)"
        );
        return Ok(0);
    }

    let core_balance = bank.core_balance_confirmed();
    if core_balance >= threshold_duff {
        tracing::debug!(
            target: "platform_wallet::e2e::bank_rebalance",
            core_balance,
            threshold_duff,
            "core-refill no-op: Core balance above threshold"
        );
        return Ok(0);
    }

    let bank_wallet = bank.platform_wallet();

    // Ensure the bank identity is loaded into the manager — both the
    // top-up and the withdrawal look it up there.
    let master = match bank_master(bank) {
        Ok(m) => m,
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                error = %err,
                "core-refill skipped: bank master-xprv derive failed"
            );
            return Ok(0);
        }
    };
    if let Err(err) = bank_wallet
        .identity()
        .load_identity_by_index_from_master(bank_identity.identity_index, &master)
        .await
    {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            error = %err,
            "core-refill skipped: failed to load bank identity into manager"
        );
        return Ok(0);
    }

    let withdraw_credits: Credits = target_duff.saturating_mul(CREDITS_PER_DUFF);
    let topup_credits: Credits = withdraw_credits.saturating_add(CORE_REFILL_IDENTITY_FEE_RESERVE);

    let inputs: BTreeMap<_, _> =
        std::iter::once((*bank.primary_receive_address(), topup_credits)).collect();
    let identity_balance_after_topup = match bank_wallet
        .identity()
        .top_up_from_addresses(&bank_identity.id, inputs, bank.address_signer(), None)
        .await
    {
        Ok((_address_infos, new_balance, _)) => new_balance,
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                topup_credits,
                error = %err,
                "core-refill skipped: top_up_from_addresses failed"
            );
            return Ok(0);
        }
    };

    // Wait for the new identity balance to be visible on chain before
    // issuing the withdrawal — the SDK's `WithdrawFromIdentity` rebuilds
    // the transition from `Identity::fetch`, so a stale view rejects
    // with `InsufficientIdentityBalance`.
    if let Err(err) = wait_for_identity_balance(
        bank_wallet.sdk(),
        bank_identity.id,
        identity_balance_after_topup,
        CORE_REFILL_TOPUP_TIMEOUT,
    )
    .await
    {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            expected = identity_balance_after_topup,
            error = %err,
            "core-refill skipped: post-top-up identity balance never \
             converged; abandoning withdrawal"
        );
        return Ok(0);
    }

    let core_addr = match bank.primary_core_receive_address().await {
        Ok(addr) => addr,
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                error = %err,
                "core-refill skipped: bank Core receive-address resolution failed"
            );
            return Ok(0);
        }
    };

    match bank_wallet
        .identity()
        .withdraw_credits_with_external_signer(
            &bank_identity.id,
            withdraw_credits,
            &core_addr,
            bank_identity.signer.as_ref(),
            None,
        )
        .await
    {
        Ok(()) => {
            tracing::info!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                core_balance_before = core_balance,
                target_duff,
                withdrew_credits = withdraw_credits,
                bank_core_addr = %core_addr,
                "Platform→Core refill issued (will settle through the Core \
                 withdrawal pool; the bank's confirmed balance updates after \
                 SPV observes the unlock)"
            );
            Ok(target_duff)
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                withdraw_credits,
                error = %err,
                "core-refill withdrawal failed; Core balance unchanged"
            );
            Ok(0)
        }
    }
}

/// Setup preflight: assert the bank can fund at least one full e2e
/// pass. Call AFTER [`refill_core_from_platform_if_below_threshold`] at
/// suite start — if the confirmed Core balance is still below
/// [`CORE_REFILL_OPERATIONAL_MIN_DUFF`] the refill chain couldn't (or
/// didn't) deliver, so abort instead of entering a run guaranteed to
/// starve mid-pass.
///
/// Returns [`FrameworkError::Bank`] naming the fixed index-0 Core
/// top-up address and the exact shortfall (needed vs available), in the
/// same operator-actionable shape as [`BankWallet::send_core_to`]'s
/// under-funded error.
pub async fn assert_core_funded_for_one_pass(bank: &BankWallet) -> FrameworkResult<()> {
    let confirmed = bank.core_balance_confirmed();
    if confirmed >= CORE_REFILL_OPERATIONAL_MIN_DUFF {
        return Ok(());
    }

    let top_up_addr = match bank.primary_core_receive_address().await {
        Ok(addr) => addr.to_string(),
        Err(err) => format!("<unresolved: {err}>"),
    };
    let short = CORE_REFILL_OPERATIONAL_MIN_DUFF - confirmed;
    Err(FrameworkError::Bank(format!(
        "Bank Core under-funded for e2e run (preflight).\n  \
         confirmed : {confirmed} duffs\n  \
         required  : {CORE_REFILL_OPERATIONAL_MIN_DUFF} duffs (one full pass burns ~{burn})\n  \
         short by  : {short} duffs\n  \
         top up at : {top_up_addr}\n\
         \n\
         The Platform→Core auto-refill could not raise the bank above the \
         one-pass floor (Platform side likely empty too). Send testnet Core \
         duffs to the fixed address above, then re-run.",
        burn = CORE_BURN_PER_FULL_PASS_DUFF,
    )))
}

/// E5 — bootstrap Platform credits from the bank's Core balance via a
/// one-time asset-lock. The crux of "fund only Core, the framework
/// handles the rest" (Core-only seed scenario).
///
/// Flow (test-only; the harness holds the seed, so it can materialise
/// the credit-output private key the production no-raw-key signer path
/// deliberately avoids):
///   1. Build + broadcast the L1 asset-lock tx and wait for its proof via
///      [`AssetLockManager::create_funded_asset_lock_proof`] using the
///      bank's seed-backed Core signer. Returns `(proof, path, _)`.
///   2. Materialise the credit-output [`PrivateKey`] from the seed at the
///      returned derivation path.
///   3. Convert the locked Dash to Platform credits on the bank's primary
///      receive address via [`PlatformAddressWallet::fund_from_asset_lock`].
///
/// Requires SPV: the proof needs a ChainLocked (or IS-locked) funding tx,
/// so this hard-errors when `disable_spv` is set — Core-only bootstrap
/// genuinely cannot run without SPV. All other failures surface as
/// [`FrameworkError::Bank`] so the unified floor check reports them.
pub async fn asset_lock_core_to_platform(
    bank: &BankWallet,
    amount_duff: u64,
    disable_spv: bool,
) -> FrameworkResult<()> {
    use dpp::address_funds::PlatformAddress;
    use platform_wallet::AssetLockFunding;

    if disable_spv {
        return Err(FrameworkError::Bank(
            "Core-only bootstrap (asset-lock Core→Platform) requires SPV for the \
             ChainLocked funding proof, but PLATFORM_WALLET_E2E_DISABLE_SPV is set. \
             Either enable SPV or fund the bank's Platform address directly."
                .to_string(),
        ));
    }
    if amount_duff == 0 {
        return Ok(());
    }

    let network = bank.network();
    let wallet = bank.platform_wallet();
    let core_signer = SeedBackedCoreSigner::new(*bank.seed_bytes(), network);

    // Fund the bank's primary Platform address from a wallet-balance
    // asset lock. The unified `fund_from_asset_lock` flow builds and
    // broadcasts the Core asset-lock tx, waits for its proof (IS → CL
    // fallback inside), and submits the platform-address top-up — the
    // bank harness no longer manages proof derivation by hand.
    let recipient: PlatformAddress = *bank.primary_receive_address();
    let mut addresses: BTreeMap<PlatformAddress, Option<Credits>> = BTreeMap::new();
    addresses.insert(recipient, None);

    // The asset-lock-funded transition has NO address inputs (the value
    // source is the lock itself), so `DeductFromInput(0)` is out of bounds.
    // Take the fee from the single recipient output instead.
    wallet
        .platform()
        .fund_from_asset_lock(
            AssetLockFunding::FromWalletBalance {
                amount_duffs: amount_duff,
                account_index: DEFAULT_ACCOUNT_INDEX_PUB,
            },
            DEFAULT_ACCOUNT_INDEX_PUB,
            addresses,
            default_fee_strategy(),
            bank.address_signer(),
            &core_signer,
            None,
        )
        .await
        .map_err(|e| {
            FrameworkError::Bank(format!(
                "asset-lock bootstrap: fund_from_asset_lock failed: {e}"
            ))
        })?;

    super::funding_ledger::record_e5_lock(amount_duff);
    tracing::info!(
        target: "platform_wallet::e2e::bank_rebalance",
        amount_duff,
        recipient = %recipient.to_bech32m_string(network),
        "E5 bootstrap: asset-locked Core → Platform"
    );
    Ok(())
}

/// E3 — top up the bank identity from the bank's Platform address pool by
/// `credits`. Thin wrapper over `top_up_from_addresses` that loads the
/// identity into the manager first (the primitive looks it up there).
pub async fn top_up_identity_from_platform(
    bank: &BankWallet,
    bank_identity: &BankIdentity,
    credits: Credits,
) -> FrameworkResult<()> {
    if credits == 0 {
        return Ok(());
    }
    let bank_wallet = bank.platform_wallet();
    let master = bank_master(bank)?;
    bank_wallet
        .identity()
        .load_identity_by_index_from_master(bank_identity.identity_index, &master)
        .await
        .map_err(|e| FrameworkError::Bank(format!("E3 top-up: load bank identity failed: {e}")))?;

    let inputs: BTreeMap<_, _> =
        std::iter::once((*bank.primary_receive_address(), credits)).collect();
    bank_wallet
        .identity()
        .top_up_from_addresses(&bank_identity.id, inputs, bank.address_signer(), None)
        .await
        .map(|_new_balance| {
            super::funding_ledger::record_identity_requested(credits);
        })
        .map_err(|e| FrameworkError::Bank(format!("E3 top-up: top_up_from_addresses failed: {e}")))
}

/// E4 — shield `credits` from the bank's Platform address into its
/// shielded pool. Prover-gated: if shielded support isn't configured on
/// the manager (no `configure_shielded` ran, so the coordinator is
/// `None`), this WARNs and skips rather than hanging on proof generation.
/// Best-effort like the other slow edges.
pub async fn shield_from_platform(bank: &BankWallet, credits: Credits, config: &Config) {
    if credits == 0 {
        return;
    }
    if config.min_shielded_credits == 0 {
        return;
    }
    if !shielded_is_ready(bank).await {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            credits,
            "E4 shield skipped: the bank does not bind a shielded pool at \
             setup (unimplemented — SH cases self-fund their own per-test \
             shielded pool). Set PLATFORM_WALLET_E2E_MIN_SHIELDED_CREDITS=0 \
             to silence this floor check."
        );
        return;
    }

    tracing::warn!(
        target: "platform_wallet::e2e::bank_rebalance",
        credits,
        "E4 shield requested but the harness does not yet bind the shielded \
         pool / warm the Orchard prover at setup; skipping. Tracked as a \
         follow-up — shielded setup-funding lands with the shielded case suite."
    );
}

/// Total bank shielded balance (sum across shielded accounts), or `0`
/// when shielded support isn't configured/bound yet. Best-effort.
pub async fn shielded_total_balance(bank: &BankWallet) -> Credits {
    if !shielded_is_ready(bank).await {
        return 0;
    }
    // When a coordinator is wired, sum the per-account balances. Until the
    // harness binds shielded at setup this returns 0 (not-ready above).
    0
}

/// Whether shielded support is configured + bound enough to read a
/// balance / build a shield. Currently always `false` because the harness
/// does not call `configure_shielded` at setup; the gate is here so E4 /
/// the balance read fail-soft (WARN + skip) instead of hanging once
/// shielded setup lands.
async fn shielded_is_ready(_bank: &BankWallet) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the CREDITS_PER_DUFF constant — getting this wrong silently
    /// converts 1 DASH into either 1000 DASH or 0.001 DASH downstream.
    #[test]
    fn credits_per_duff_is_one_thousand() {
        assert_eq!(CREDITS_PER_DUFF, 1_000);
    }

    /// Pin the shared bootstrap fee reserve — both the planner's E5 sizing
    /// and the bank-identity bootstrap depend on this value.
    #[test]
    fn platform_bootstrap_fee_reserve_is_pinned() {
        assert_eq!(PLATFORM_BOOTSTRAP_FEE_RESERVE, 100_000_000);
    }

    /// Pin the asset-lock funding-fee reserve — it must stay above the
    /// live-observed ~93M funding fee or both sizing paths under-net.
    #[test]
    fn bootstrap_asset_lock_fee_reserve_is_pinned() {
        assert_eq!(BOOTSTRAP_ASSET_LOCK_FEE_RESERVE, 150_000_000);
    }

    #[test]
    fn bootstrap_lock_duff_ceils_the_shortfall() {
        // Already covered → 0 duffs.
        assert_eq!(bootstrap_lock_duff(1_000, 1_000), 0);
        assert_eq!(bootstrap_lock_duff(2_000, 1_000), 0);
        // Exact multiple of CREDITS_PER_DUFF.
        assert_eq!(bootstrap_lock_duff(0, 2_000), 2);
        // Sub-duff shortfall rounds UP, never down to 0.
        assert_eq!(bootstrap_lock_duff(0, 1), 1);
        assert_eq!(bootstrap_lock_duff(0, 1_001), 2);
    }

    #[test]
    fn bootstrap_lock_net_credits_subtracts_the_funding_fee() {
        // Gross 450M − 150M fee reserve = 300M net.
        assert_eq!(bootstrap_lock_net_credits(450_000), 300_000_000);
        // A lock too small to cover its own fee nets 0 (saturating).
        assert_eq!(bootstrap_lock_net_credits(100_000), 0);
    }

    /// 1 duff round-trips through the duff→credits cast used by the
    /// core-refill helper at the 1000x ratio. Mirrors what
    /// `refill_core_from_platform_if_below_threshold` does to compute
    /// `withdraw_credits` from `target_duff`.
    #[test]
    fn duff_to_credits_conversion_round_trips() {
        let duff: u64 = 1;
        let credits: Credits = duff.saturating_mul(CREDITS_PER_DUFF);
        assert_eq!(credits, 1_000);
        let duff_back: u64 = (credits / CREDITS_PER_DUFF) as u64;
        assert_eq!(duff_back, duff);
    }

    /// Misconfigured (target ≤ threshold) is caught before any chain
    /// contact — pinned as a guard so a future "swap the args" edit
    /// can't silently waste a slow withdrawal.
    #[test]
    fn refill_misconfig_target_must_exceed_threshold() {
        let threshold = DEFAULT_CORE_REFILL_THRESHOLD_DUFF;
        let target = DEFAULT_CORE_REFILL_TARGET_DUFF;
        assert!(
            target > threshold,
            "defaults must obey target > threshold (target={target} threshold={threshold})"
        );
    }

    /// The refill defaults must stay anchored to the measured per-pass
    /// burn: the trip line covers ≥1 full pass and the target buys ≥3.
    /// A future tweak that drops these below the burn would let the
    /// bank starve mid-run again.
    #[test]
    fn refill_defaults_cover_measured_burn() {
        assert!(
            DEFAULT_CORE_REFILL_THRESHOLD_DUFF >= CORE_BURN_PER_FULL_PASS_DUFF,
            "threshold must cover ≥1 full pass (threshold={DEFAULT_CORE_REFILL_THRESHOLD_DUFF} \
             burn/pass={CORE_BURN_PER_FULL_PASS_DUFF})"
        );
        assert!(
            DEFAULT_CORE_REFILL_TARGET_DUFF >= CORE_BURN_PER_FULL_PASS_DUFF * 3,
            "target must buy ≥3 full passes (target={DEFAULT_CORE_REFILL_TARGET_DUFF} \
             burn/pass={CORE_BURN_PER_FULL_PASS_DUFF})"
        );
        // Preflight floor is exactly one pass: below it a run cannot
        // finish, so failing fast is the only correct behaviour.
        assert_eq!(
            CORE_REFILL_OPERATIONAL_MIN_DUFF, CORE_BURN_PER_FULL_PASS_DUFF,
            "preflight floor must equal one full pass of burn"
        );
        assert!(
            DEFAULT_CORE_REFILL_THRESHOLD_DUFF >= CORE_REFILL_OPERATIONAL_MIN_DUFF,
            "auto-refill trip line must sit at or above the hard preflight floor so \
             a healthy run never lands in the fail-fast window"
        );
    }
}
