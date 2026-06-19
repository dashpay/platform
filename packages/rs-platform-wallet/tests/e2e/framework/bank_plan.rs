//! Cost-ordered setup-phase fund planner for the shared bank wallet.
//!
//! Policy lives here ([`plan`]); mechanism lives in
//! [`super::bank_rebalance`]. [`plan`] is pure and deterministic — it
//! maps measured per-account-type balances + minimums to an ordered list
//! of [`Move`]s, choosing the cheapest fund-movement edge that closes
//! each deficit (fast L2 transfers before the slow Platform→Core
//! withdrawal). [`execute`] dispatches each `Move` to the existing
//! `bank_rebalance` primitives. [`assert_all_floors`] is the unified
//! post-execution gate (subsumes the old `assert_floor` +
//! `assert_core_funded_for_one_pass`).
//!
//! Cost ordering (cheapest first): E3'/E3 fast L2 < E4 shield (prover) <
//! E5 Core→Platform asset-lock (one-time bootstrap) ≪ E1 Platform→Core
//! withdrawal (last resort, real Core deficit only). See the design at
//! the PR description for the full fund-movement graph.
//!
//! Idempotency: every edge is deficit-gated, so a re-run with balances
//! already at min emits an empty plan (all deficits zero). Amounts are
//! recomputed from fresh balances each run, so a resumed partial run
//! never double-spends.

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::fee::Credits;

use super::bank::BankWallet;
use super::bank_identity::BankIdentity;
use super::bank_rebalance::{
    self, bootstrap_lock_duff, bootstrap_lock_net_credits, BOOTSTRAP_ASSET_LOCK_FEE_RESERVE,
    CREDITS_PER_DUFF, PLATFORM_BOOTSTRAP_FEE_RESERVE,
};
use super::config::Config;
use super::{FrameworkError, FrameworkResult};

/// The four account types the bank maintains. Each is a single
/// bank-controlled holding; tests draw from these. The cost of *filling*
/// a type's deficit increases down the list, which is why
/// [`AccountType`] also encodes the planner's execution priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountType {
    /// L2 Platform credits — the hub. Funds every other type.
    Platform,
    /// Bank identity balance — fee headroom for the Platform→Core relay.
    Identity,
    /// Orchard shielded pool — prover-gated, opt-in via a positive min.
    Shielded,
    /// L1 Core duffs — source for asset-lock bootstrap; sink only via E1.
    Core,
}

/// Per-account-type balances, all in a common **credit** unit so surplus
/// math is uniform. Core duffs are converted via [`CREDITS_PER_DUFF`]
/// (1 duff = 1000 credits); the planner emits Core moves back in duffs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Balances {
    pub platform: Credits,
    pub identity: Credits,
    pub shielded: Credits,
    /// Confirmed Core balance, in **duffs** (not credits).
    pub core_duff: u64,
}

impl Balances {
    /// Core balance expressed in the common credit unit.
    fn core_credits(&self) -> Credits {
        (self.core_duff).saturating_mul(CREDITS_PER_DUFF)
    }

    fn get_credits(&self, ty: AccountType) -> Credits {
        match ty {
            AccountType::Platform => self.platform,
            AccountType::Identity => self.identity,
            AccountType::Shielded => self.shielded,
            AccountType::Core => self.core_credits(),
        }
    }
}

/// Per-account-type minimum balances. Platform / Identity / Shielded are
/// credit-denominated; Core is duff-denominated (matching the existing
/// `CORE_REFILL_*` knobs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mins {
    pub platform: Credits,
    pub identity: Credits,
    pub shielded: Credits,
    pub core_duff: u64,
    /// Target the E1 withdrawal aims for when Core is below `core_duff`.
    /// Must exceed `core_duff` (mirrors the refill threshold/target gate).
    pub core_target_duff: u64,
}

impl Mins {
    fn core_credits(&self) -> Credits {
        self.core_duff.saturating_mul(CREDITS_PER_DUFF)
    }

    fn get_credits(&self, ty: AccountType) -> Credits {
        match ty {
            AccountType::Platform => self.platform,
            AccountType::Identity => self.identity,
            AccountType::Shielded => self.shielded,
            AccountType::Core => self.core_credits(),
        }
    }
}

/// A single fund-movement action. Maps 1:1 to the §2 edges. Amounts are
/// the planner's intent; the executor's underlying primitives keep their
/// own fee reserves and threshold gates, so a `Move` is always safe to
/// drop if a fresh balance read makes it unnecessary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// E3': identity → Platform drain (reclaim; runs first).
    DrainIdentity,
    /// E5: Core → Platform asset-lock bootstrap. `amount_duff` is locked
    /// on L1 and converted to L2 credits. One-time; SPV-gated.
    AssetLockCoreToPlatform { amount_duff: u64 },
    /// E3: Platform → bank identity top-up of `credits`.
    TopUpIdentity { credits: Credits },
    /// E4: Platform → shielded of `credits` (prover warm-up needed).
    ShieldFromPlatform { credits: Credits },
    /// E1: Platform → Core withdrawal, targeting `target_duff` (last
    /// resort; only on a real Core deficit).
    WithdrawToCore { target_duff: u64 },
}

/// Per-type funding shortfall surfaced by [`InsufficientFunds`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeShortfall {
    pub account_type: AccountType,
    /// Current balance in the type's native unit (credits, or duffs for Core).
    pub have: u64,
    /// Required minimum in the type's native unit.
    pub need: u64,
    /// `need - have` (0 when satisfied).
    pub short: u64,
}

/// Total-funds insufficiency: the bank cannot reach every min even after
/// reclaiming identity credits and asset-locking all Core surplus. One
/// operator-actionable error, all four types, with the two fixed top-up
/// addresses (filled in by [`execute`]/[`assert_all_floors`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsufficientFunds {
    pub shortfalls: Vec<TypeShortfall>,
}

/// Compute the cost-ordered plan from measured balances + minimums.
///
/// Pure and deterministic — no I/O. Returns the ordered [`Move`]s to run
/// (§3.4 execution order), or [`InsufficientFunds`] when the total
/// reachable funds (Platform + reclaimable identity + asset-lockable Core
/// surplus) cannot cover the sum of mins.
///
/// The plan models post-drain balances: identity credits above the
/// identity min are assumed reclaimed to Platform (the always-emitted
/// `DrainIdentity` move), so Platform's spendable surplus already
/// accounts for the drain when leaf deficits are sized.
pub fn plan(balances: Balances, mins: Mins) -> Result<Vec<Move>, InsufficientFunds> {
    let mut moves = Vec::new();

    // Step 1 (E3'): always reclaim identity surplus to Platform first.
    // Models maximal Platform surplus before sizing the leaf deficits.
    // The executor's drain keeps its own fee reserve, so this is a no-op
    // when the identity is at/below its reserve.
    moves.push(Move::DrainIdentity);
    let identity_surplus = balances.identity.saturating_sub(mins.identity);
    let mut platform_spendable = balances.platform.saturating_add(identity_surplus);

    // Step 2 (E5): if Platform is short of its min, bootstrap from Core
    // surplus via a one-time asset-lock. Platform is the hub — nothing
    // cheaper can fill it.
    let platform_deficit = mins.platform.saturating_sub(platform_spendable);
    if platform_deficit > 0 {
        let core_surplus_credits = balances.core_credits().saturating_sub(mins.core_credits());
        // The lock must fund the FULL downstream Platform outflow, not just
        // the hub min: Platform fans out to the identity / shielded / core
        // leaves (Steps 4-6), each drawn from the post-hub surplus. Size the
        // leaf deficits from the ORIGINAL balances so they match the later
        // per-step draws exactly.
        let leaf_outflow = leaf_platform_outflow(&balances, &mins);
        // Lock enough to NET (after the `ReduceOutput(0)` funding fee) the
        // hub deficit plus the leaf outflow, plus a reserve so the
        // transitions don't immediately re-underflow Platform.
        let want_credits = platform_deficit
            .saturating_add(leaf_outflow)
            .saturating_add(PLATFORM_BOOTSTRAP_FEE_RESERVE)
            .saturating_add(BOOTSTRAP_ASSET_LOCK_FEE_RESERVE);
        let lock_credits = want_credits.min(core_surplus_credits);
        if lock_credits > 0 {
            let amount_duff = bootstrap_lock_duff(0, lock_credits);
            // Credit only the NET (gross − funding fee) so the Step-3 hub
            // gate and the leaf-draw checks fire when a Core-constrained lock
            // can't net the full fan-out.
            let net_credits = bootstrap_lock_net_credits(amount_duff);
            if net_credits > 0 {
                moves.push(Move::AssetLockCoreToPlatform { amount_duff });
                platform_spendable = platform_spendable.saturating_add(net_credits);
            }
        }
    }

    // Step 3: hub must be funded before fan-out. If Platform still can't
    // reach its min, the run is doomed — collect the full insufficiency.
    if platform_spendable < mins.platform {
        return Err(insufficiency(balances, mins));
    }

    // Reserve Platform's own min; only the surplus funds leaf types.
    let mut platform_surplus = platform_spendable.saturating_sub(mins.platform);

    // Step 4 (E3): Platform → identity top-up on a real identity deficit.
    let identity_deficit = mins.identity.saturating_sub(balances.identity);
    if identity_deficit > 0 {
        if platform_surplus < identity_deficit {
            return Err(insufficiency(balances, mins));
        }
        platform_surplus -= identity_deficit;
        moves.push(Move::TopUpIdentity {
            credits: identity_deficit,
        });
    }

    // Step 5 (E4): Platform → shielded shield, only when min_shielded > 0
    // (opt-in). Prover warm-up is handled in `execute`.
    let shielded_deficit = mins.shielded.saturating_sub(balances.shielded);
    if mins.shielded > 0 && shielded_deficit > 0 {
        if platform_surplus < shielded_deficit {
            return Err(insufficiency(balances, mins));
        }
        platform_surplus -= shielded_deficit;
        moves.push(Move::ShieldFromPlatform {
            credits: shielded_deficit,
        });
    }

    // Step 6 (E1): Platform → Core withdrawal, last resort, real Core
    // deficit only. Sized to `core_target_duff`; the source is Platform
    // surplus expressed in credits.
    if balances.core_duff < mins.core_duff {
        let withdraw_credits = mins.core_target_duff.saturating_mul(CREDITS_PER_DUFF);
        if platform_surplus < withdraw_credits {
            return Err(insufficiency(balances, mins));
        }
        moves.push(Move::WithdrawToCore {
            target_duff: mins.core_target_duff,
        });
    }

    Ok(moves)
}

/// Total credits Platform must pay out to the leaf types AFTER reaching its
/// own min — the identity top-up (Step 4), shielded shield (Step 5, opt-in),
/// and core withdrawal (Step 6). Computed from the original `balances`/`mins`
/// so the E5 lock target (Step 2) and the later per-step draws agree.
fn leaf_platform_outflow(balances: &Balances, mins: &Mins) -> Credits {
    let identity_deficit = mins.identity.saturating_sub(balances.identity);
    let shielded_deficit = if mins.shielded > 0 {
        mins.shielded.saturating_sub(balances.shielded)
    } else {
        0
    };
    let withdraw_credits = if balances.core_duff < mins.core_duff {
        mins.core_target_duff.saturating_mul(CREDITS_PER_DUFF)
    } else {
        0
    };
    identity_deficit
        .saturating_add(shielded_deficit)
        .saturating_add(withdraw_credits)
}

/// Build the full per-type shortfall report (native units) for the
/// insufficiency error. Includes types that ARE satisfied (short = 0) so
/// the operator sees the whole picture.
fn insufficiency(balances: Balances, mins: Mins) -> InsufficientFunds {
    let credit_short = |ty: AccountType| TypeShortfall {
        account_type: ty,
        have: balances.get_credits(ty),
        need: mins.get_credits(ty),
        short: mins
            .get_credits(ty)
            .saturating_sub(balances.get_credits(ty)),
    };
    InsufficientFunds {
        shortfalls: vec![
            credit_short(AccountType::Platform),
            credit_short(AccountType::Identity),
            credit_short(AccountType::Shielded),
            // Core reported in duffs (its native unit).
            TypeShortfall {
                account_type: AccountType::Core,
                have: balances.core_duff,
                need: mins.core_duff,
                short: mins.core_duff.saturating_sub(balances.core_duff),
            },
        ],
    }
}

/// Execute a plan against the live bank. Each `Move` dispatches to a
/// `bank_rebalance` primitive (the mechanism layer). Best-effort per the
/// existing helpers' contract: a failed slow edge logs a WARN and
/// continues so [`assert_all_floors`] surfaces the single, unified
/// operator error rather than a mid-plan abort. The asset-lock bootstrap
/// (E5) is the one edge that hard-errors when its SPV prerequisite is
/// absent, because Core-only bootstrap genuinely cannot proceed.
pub async fn execute(
    plan: &[Move],
    bank: &BankWallet,
    bank_identity: &BankIdentity,
    config: &Config,
) -> FrameworkResult<()> {
    for mv in plan {
        match mv {
            Move::DrainIdentity => {
                match bank_rebalance::drain_bank_identity_to_addresses(bank, bank_identity).await {
                    Ok(0) => {}
                    Ok(drained) => tracing::info!(
                        target: "platform_wallet::e2e::bank_plan",
                        drained,
                        "E3' drain: identity → Platform"
                    ),
                    Err(err) => tracing::warn!(
                        target: "platform_wallet::e2e::bank_plan",
                        error = %err,
                        "E3' drain failed; continuing"
                    ),
                }
            }
            Move::AssetLockCoreToPlatform { amount_duff } => {
                bank_rebalance::asset_lock_core_to_platform(bank, *amount_duff, config.disable_spv)
                    .await?;
            }
            Move::TopUpIdentity { credits } => {
                match bank_rebalance::top_up_identity_from_platform(bank, bank_identity, *credits)
                    .await
                {
                    Ok(()) => tracing::info!(
                        target: "platform_wallet::e2e::bank_plan",
                        credits,
                        "E3 top-up: Platform → identity"
                    ),
                    Err(err) => tracing::warn!(
                        target: "platform_wallet::e2e::bank_plan",
                        error = %err,
                        "E3 top-up failed; continuing"
                    ),
                }
            }
            Move::ShieldFromPlatform { credits } => {
                bank_rebalance::shield_from_platform(bank, *credits, config).await;
            }
            Move::WithdrawToCore { target_duff } => {
                match bank_rebalance::refill_core_from_platform_if_below_threshold(
                    bank,
                    bank_identity,
                    config.core_refill_threshold_duff,
                    *target_duff,
                )
                .await
                {
                    Ok(0) => {}
                    Ok(refilled) => tracing::info!(
                        target: "platform_wallet::e2e::bank_plan",
                        refilled_duff = refilled,
                        "E1 withdrawal: Platform → Core"
                    ),
                    Err(err) => tracing::warn!(
                        target: "platform_wallet::e2e::bank_plan",
                        error = %err,
                        "E1 withdrawal failed; continuing"
                    ),
                }
            }
        }
    }
    Ok(())
}

/// Unified post-execution funding gate. Re-reads live balances and
/// asserts every type meets its min. Subsumes the old
/// `bank.assert_floor` (Platform) and
/// `bank_rebalance::assert_core_funded_for_one_pass` (Core), preserving
/// their operator-actionable shapes: a Platform shortfall panics (init is
/// `OnceCell`-driven, so a panic is the established "abort the suite"
/// signal), and a Core shortfall returns [`FrameworkError::Bank`].
///
/// Both messages name the relevant fixed top-up address (Platform DIP-17
/// idx0 / Core BIP-44 idx0). Shielded/identity shortfalls are surfaced as
/// WARNs (their mins are soft — leaf types degrade rather than block the
/// whole suite), except where a hard min is configured.
pub async fn assert_all_floors(
    bank: &BankWallet,
    bank_identity: &BankIdentity,
    config: &Config,
    sweep_recovered: usize,
    registry_total: usize,
    registry_failed: usize,
) -> FrameworkResult<()> {
    // Platform floor: panic shape preserved from `bank.assert_floor`.
    bank.assert_floor(config, sweep_recovered, registry_total, registry_failed)
        .await;

    // Identity / shielded are soft — WARN only, never block the suite.
    let identity_balance = fetch_identity_balance(bank, bank_identity).await;
    if identity_balance < config.min_identity_credits {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_plan",
            have = identity_balance,
            need = config.min_identity_credits,
            "bank identity below its credit floor; the Platform→Core relay may \
             starve on fees. Identity-heavy cases could under-fund."
        );
    }
    if config.min_shielded_credits > 0 {
        let shielded_balance = fetch_shielded_balance(bank).await;
        if shielded_balance < config.min_shielded_credits {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_plan",
                have = shielded_balance,
                need = config.min_shielded_credits,
                "bank shielded pool below its configured floor; shielded cases \
                 may skip (prover unconfigured or shield did not settle)."
            );
        }
    }

    // Core floor: error shape preserved from `assert_core_funded_for_one_pass`.
    bank_rebalance::assert_core_funded_for_one_pass(bank).await
}

/// Build a [`Balances`] snapshot from the live bank. Identity/shielded
/// reads are best-effort: a failed fetch reads as 0 so the planner errs
/// toward funding rather than skipping a type it can't measure.
///
/// Uses [`BankWallet::effective_platform_credits`] (i.e. `max(cache,
/// adopted)`) so the planner sees the proof-verified independent balance
/// when the wallet cache is known to be stale from a lagging DAPI replica
/// (#3611). Under normal operation this is identical to `total_credits()`.
pub async fn snapshot_balances(bank: &BankWallet, bank_identity: &BankIdentity) -> Balances {
    Balances {
        platform: bank.effective_platform_credits().await,
        identity: fetch_identity_balance(bank, bank_identity).await,
        shielded: fetch_shielded_balance(bank).await,
        core_duff: bank.core_balance_confirmed(),
    }
}

/// Bank-identity balance from chain (authoritative; the local cache can
/// be stale at init). Reads `0` on absent identity / fetch error.
async fn fetch_identity_balance(bank: &BankWallet, bank_identity: &BankIdentity) -> Credits {
    match IdentityBalance::fetch(bank.platform_wallet().sdk(), bank_identity.id).await {
        Ok(Some(balance)) => balance,
        Ok(None) => 0,
        Err(err) => {
            tracing::debug!(
                target: "platform_wallet::e2e::bank_plan",
                bank_identity_id = %bank_identity.id,
                error = %err,
                "identity balance fetch failed; treating as 0 for planning"
            );
            0
        }
    }
}

/// Total bank shielded balance (sum across accounts) from the manager's
/// coordinator. Reads `0` when shielded support isn't configured/bound
/// yet — the planner then sizes the shield from the configured min.
async fn fetch_shielded_balance(bank: &BankWallet) -> Credits {
    bank_rebalance::shielded_total_balance(bank).await
}

/// Build the [`Mins`] from config knobs.
pub fn mins_from_config(config: &Config) -> Mins {
    Mins {
        platform: config.min_bank_credits,
        identity: config.min_identity_credits,
        shielded: config.min_shielded_credits,
        core_duff: config.core_refill_threshold_duff,
        core_target_duff: config.core_refill_target_duff,
    }
}

/// Render an [`InsufficientFunds`] into the single operator-actionable
/// [`FrameworkError::Bank`], appending the two fixed top-up addresses.
pub async fn insufficiency_to_error(
    insufficiency: &InsufficientFunds,
    bank: &BankWallet,
) -> FrameworkError {
    let platform_addr = bank
        .primary_receive_address()
        .to_bech32m_string(bank.network());
    let core_addr = match bank.primary_core_receive_address().await {
        Ok(addr) => addr.to_string(),
        Err(err) => format!("<unresolved: {err}>"),
    };

    let mut lines = String::from("Bank under-funded for e2e run (planner).\n");
    for s in &insufficiency.shortfalls {
        let unit = if s.account_type == AccountType::Core {
            "duffs"
        } else {
            "credits"
        };
        lines.push_str(&format!(
            "  {:?}: have {} {unit}, need {} {unit}, short {} {unit}\n",
            s.account_type, s.have, s.need, s.short,
        ));
    }
    lines.push_str(&format!(
        "\nTop up the relevant fixed address(es), then re-run:\n  \
         Platform (DIP-17 idx0): {platform_addr}\n  \
         Core     (BIP-44 idx0): {core_addr}\n\
         Core surplus is asset-locked into Platform automatically; fund Core \
         directly only on a fresh network with no Platform balance."
    ));
    FrameworkError::Bank(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Behaviour-preserving defaults matching today's effective floors:
    /// Platform 500M, identity reserve 30M, shielded off (0), Core
    /// threshold 2e9 / target 5e9 duffs.
    fn legacy_mins() -> Mins {
        Mins {
            platform: 500_000_000,
            identity: 30_000_000,
            shielded: 0,
            core_duff: 2_000_000_000,
            core_target_duff: 5_000_000_000,
        }
    }

    /// A bank that already meets every min — re-run must be a no-op
    /// (only the always-emitted, self-gating drain).
    #[test]
    fn idempotent_rerun_at_min_is_noop() {
        let bal = Balances {
            platform: 500_000_000,
            identity: 30_000_000,
            shielded: 0,
            core_duff: 2_000_000_000,
        };
        let plan = plan(bal, legacy_mins()).expect("plan");
        assert_eq!(
            plan,
            vec![Move::DrainIdentity],
            "at-min balances must yield only the self-gating drain"
        );
    }

    /// Core-only seed: operator funded ONLY the Core address. Platform=0,
    /// identity=0, shielded=0, Core well above its min. Plan must
    /// asset-lock Core→Platform (E5) to fund the hub, then top up identity.
    #[test]
    fn core_only_bootstrap_emits_asset_lock() {
        let bal = Balances {
            platform: 0,
            identity: 0,
            shielded: 0,
            // 60 tDASH in duffs — plenty above the 2e9 Core min.
            core_duff: 6_000_000_000,
        };
        let plan = plan(bal, legacy_mins()).expect("plan");
        assert_eq!(plan[0], Move::DrainIdentity);
        // E5 must appear and lock at least the Platform deficit's worth.
        let asset_lock = plan
            .iter()
            .find_map(|m| match m {
                Move::AssetLockCoreToPlatform { amount_duff } => Some(*amount_duff),
                _ => None,
            })
            .expect("Core-only bootstrap must emit an asset-lock move");
        // Locked duffs * 1000 credits must cover the 500M Platform deficit.
        assert!(
            asset_lock.saturating_mul(CREDITS_PER_DUFF) >= 500_000_000,
            "asset-lock must cover the Platform deficit (locked {asset_lock} duffs)"
        );
        // Identity deficit (30M) must be topped up after the hub is funded.
        assert!(
            plan.contains(&Move::TopUpIdentity {
                credits: 30_000_000
            }),
            "identity deficit must be topped up post-bootstrap; plan={plan:?}"
        );
        // No withdrawal — Core is above its min.
        assert!(
            !plan
                .iter()
                .any(|m| matches!(m, Move::WithdrawToCore { .. })),
            "no E1 withdrawal when Core is above min; plan={plan:?}"
        );
    }

    /// Partial deficit: Platform funded, identity funded, but Core below
    /// its min. Only the last-resort E1 withdrawal should fire (Platform
    /// has the surplus to cover the target).
    #[test]
    fn core_deficit_only_emits_withdrawal_last() {
        let bal = Balances {
            // Enough Platform surplus to cover the 5e9-duff (=5e12 credits)
            // withdrawal target plus the 500M min.
            platform: 5_000_000_000_000 + 500_000_000,
            identity: 30_000_000,
            shielded: 0,
            core_duff: 0,
        };
        let plan = plan(bal, legacy_mins()).expect("plan");
        assert_eq!(plan.first(), Some(&Move::DrainIdentity));
        assert_eq!(
            plan.last(),
            Some(&Move::WithdrawToCore {
                target_duff: 5_000_000_000
            }),
            "Core deficit must emit E1 withdrawal as the last move; plan={plan:?}"
        );
        assert!(
            !plan
                .iter()
                .any(|m| matches!(m, Move::AssetLockCoreToPlatform { .. })),
            "no E5 bootstrap when Platform is already funded; plan={plan:?}"
        );
    }

    /// Shielded opt-in: a positive shielded min with Platform surplus
    /// emits the shield move; a zero min never does.
    #[test]
    fn shielded_move_is_opt_in() {
        let bal = Balances {
            platform: 1_000_000_000,
            identity: 30_000_000,
            shielded: 0,
            core_duff: 2_000_000_000,
        };

        // min_shielded = 0 → no shield move.
        let plan_off = plan(bal, legacy_mins()).expect("plan off");
        assert!(
            !plan_off
                .iter()
                .any(|m| matches!(m, Move::ShieldFromPlatform { .. })),
            "shielded min 0 must never emit a shield; plan={plan_off:?}"
        );

        // min_shielded > 0 with Platform surplus → shield move.
        let mut mins_on = legacy_mins();
        mins_on.shielded = 100_000_000;
        let plan_on = plan(bal, mins_on).expect("plan on");
        assert!(
            plan_on.contains(&Move::ShieldFromPlatform {
                credits: 100_000_000
            }),
            "shielded min>0 with surplus must emit shield; plan={plan_on:?}"
        );
    }

    /// Insufficient total: Platform=0 and Core has no surplus above its
    /// own min, so the hub can't be bootstrapped → hard failure listing
    /// all four types.
    #[test]
    fn insufficient_total_fails_with_all_types() {
        let bal = Balances {
            platform: 0,
            identity: 0,
            shielded: 0,
            // Exactly at the Core min → zero surplus to asset-lock.
            core_duff: 2_000_000_000,
        };
        let err = plan(bal, legacy_mins()).expect_err("must be insufficient");
        assert_eq!(
            err.shortfalls.len(),
            4,
            "insufficiency must report all four account types"
        );
        let platform = err
            .shortfalls
            .iter()
            .find(|s| s.account_type == AccountType::Platform)
            .expect("platform shortfall present");
        assert_eq!(platform.have, 0);
        assert_eq!(platform.need, 500_000_000);
        assert_eq!(platform.short, 500_000_000);
    }

    /// The drain reclaims identity surplus into Platform's spendable pool
    /// BEFORE leaf deficits are sized, so a bank with all its credits
    /// stranded on the identity can still fund Platform without a bootstrap.
    #[test]
    fn identity_surplus_funds_platform_via_drain_no_bootstrap() {
        let bal = Balances {
            platform: 0,
            // 600M on the identity: 30M reserve + 570M reclaimable.
            identity: 600_000_000,
            shielded: 0,
            core_duff: 2_000_000_000,
        };
        let plan = plan(bal, legacy_mins()).expect("plan");
        // Drain present, NO asset-lock (reclaimed identity credits cover
        // the 500M Platform min).
        assert_eq!(plan.first(), Some(&Move::DrainIdentity));
        assert!(
            !plan
                .iter()
                .any(|m| matches!(m, Move::AssetLockCoreToPlatform { .. })),
            "reclaimed identity surplus must avoid the slow bootstrap; plan={plan:?}"
        );
    }

    /// QA-008 regression: a Core-constrained bank whose lockable surplus
    /// covers the bare Platform deficit GROSS but not after the asset-lock
    /// funding fee. The planner must NOT false-pass on phantom (un-netted)
    /// credits — it has to fire `InsufficientFunds` because the NET landing
    /// on Platform underflows its min.
    ///
    /// deficit = 500M; Core surplus = 600M credits — above the 500M deficit
    /// but below deficit + 150M fee, so the capped lock nets only
    /// 600M − 150M = 450M < 500M.
    #[test]
    fn core_constrained_lock_under_nets_deficit_fails() {
        let core_surplus_credits = 600_000_000u64;
        let mins = legacy_mins();
        let bal = Balances {
            platform: 0,
            identity: 0,
            shielded: 0,
            core_duff: mins.core_duff + core_surplus_credits / CREDITS_PER_DUFF,
        };
        let err = plan(bal, mins).expect_err(
            "Core-constrained lock that can't NET the Platform deficit must be insufficient",
        );
        let platform = err
            .shortfalls
            .iter()
            .find(|s| s.account_type == AccountType::Platform)
            .expect("platform shortfall present");
        assert_eq!(platform.need, 500_000_000);
    }

    /// QA-008: a well-funded bank sizes the E5 lock to NET the deficit after
    /// the funding fee. With `legacy_mins` (identity min 30M, balances all 0)
    /// the lock also covers the 30M identity leaf deficit (see
    /// `e5_lock_covers_leaf_deficits_not_just_platform_min`).
    #[test]
    fn well_funded_e5_lock_includes_the_funding_fee() {
        let bal = Balances {
            platform: 0,
            identity: 0,
            shielded: 0,
            core_duff: 6_000_000_000,
        };
        let plan = plan(bal, legacy_mins()).expect("plan");
        let asset_lock = plan
            .iter()
            .find_map(|m| match m {
                Move::AssetLockCoreToPlatform { amount_duff } => Some(*amount_duff),
                _ => None,
            })
            .expect("well-funded bank must emit an asset-lock move");
        // Gross must cover platform deficit (500M) + identity leaf deficit
        // (30M) + leaf reserve (100M) + funding fee (150M) = 780M.
        let gross = asset_lock.saturating_mul(CREDITS_PER_DUFF);
        assert_eq!(
            gross,
            500_000_000
                + 30_000_000
                + PLATFORM_BOOTSTRAP_FEE_RESERVE
                + BOOTSTRAP_ASSET_LOCK_FEE_RESERVE
        );
        assert!(
            bootstrap_lock_net_credits(asset_lock) >= 500_000_000,
            "net after funding fee must clear the Platform min; net={}",
            bootstrap_lock_net_credits(asset_lock)
        );
    }

    /// QA-010 regression (the live pa_002 blocker): an abundant-Core bank
    /// with BOTH a Platform deficit and a Shielded floor. The E5 lock must
    /// be sized to fund the full Platform fan-out (hub min + shielded leaf),
    /// not just the hub min — otherwise the post-hub surplus can't cover the
    /// shield and `plan()` false-fails with `InsufficientFunds` despite the
    /// Core being there.
    ///
    /// Mirrors the live state: Platform ~162M / need 500M, Shielded 0 / need
    /// 500M, identity funded, Core huge.
    #[test]
    fn e5_lock_covers_leaf_deficits_not_just_platform_min() {
        let mins = Mins {
            platform: 500_000_000,
            identity: 30_000_000,
            shielded: 500_000_000,
            core_duff: 2_000_000_000,
            core_target_duff: 5_000_000_000,
        };
        let bal = Balances {
            platform: 162_146_560,
            identity: 184_004_720,
            shielded: 0,
            // ~102 tDASH — far above the Core min, so the lock isn't capped.
            core_duff: 102_000_000_000,
        };
        let plan = plan(bal, mins).expect("abundant Core must fund platform + shielded leaf");

        // The hub gets topped up AND the shielded floor is funded — no
        // false InsufficientFunds.
        assert!(plan.contains(&Move::ShieldFromPlatform {
            credits: 500_000_000
        }));
        let asset_lock = plan
            .iter()
            .find_map(|m| match m {
                Move::AssetLockCoreToPlatform { amount_duff } => Some(*amount_duff),
                _ => None,
            })
            .expect("E5 asset-lock must be emitted");

        // Net-after-fee, on top of the Platform balance + the drained
        // identity surplus (184M − 30M = 154M reclaimed in Step 1), must
        // cover the hub min (500M) PLUS the shielded leaf (500M) = 1B.
        let net = bootstrap_lock_net_credits(asset_lock);
        let drained_identity_surplus = 184_004_720 - 30_000_000;
        let platform_after = 162_146_560 + drained_identity_surplus + net;
        assert!(
            platform_after >= 500_000_000 + 500_000_000,
            "platform after E5 ({platform_after}) must cover hub min + shielded leaf"
        );
    }

    /// QA-010: when Core IS constrained, folding leaf deficits into the lock
    /// target must still surface `InsufficientFunds` (no false-pass) if the
    /// capped net can't cover the full fan-out.
    #[test]
    fn e5_leaf_sizing_still_fails_when_core_capped_below_total() {
        let mins = Mins {
            platform: 500_000_000,
            identity: 30_000_000,
            shielded: 500_000_000,
            core_duff: 2_000_000_000,
            core_target_duff: 5_000_000_000,
        };
        // Core surplus 700M credits: enough to net the 500M hub min after the
        // 150M fee (700−150=550 ≥ 500) but NOT the additional 500M shielded
        // leaf — must fail, not false-pass.
        let core_surplus_credits = 700_000_000u64;
        let bal = Balances {
            platform: 0,
            identity: 30_000_000,
            shielded: 0,
            core_duff: mins.core_duff + core_surplus_credits / CREDITS_PER_DUFF,
        };
        let err = plan(bal, mins).expect_err("capped lock can't fund the shielded leaf");
        let shielded = err
            .shortfalls
            .iter()
            .find(|s| s.account_type == AccountType::Shielded)
            .expect("shielded shortfall present");
        assert_eq!(shielded.need, 500_000_000);
    }
}
