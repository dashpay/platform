//! Process-global funding ledger — tracks bank outflows and sweep
//! recoveries per account type so we can calibrate funding floors
//! empirically rather than by guessing.
//!
//! The ledger is a singleton populated with `OnceLock`; its per-type
//! counters are `AtomicU64` so concurrent tests can record without a
//! `Mutex`. Ordering is `Relaxed` throughout — we need atomicity per
//! counter but no inter-counter ordering guarantees; totals are read
//! only at end-of-suite after all writers have quiesced.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use dpp::fee::Credits;

/// Per-type accounting, all values in the type's native unit
/// (credits for Platform/Identity/Shielded; duffs for Core).
#[derive(Debug, Default)]
pub struct TypeCounters {
    /// Gross outflow from bank (credits or duffs).
    pub requested: AtomicU64,
    /// Gross recovered from test wallets back to bank (credits or duffs).
    pub recovered: AtomicU64,
    /// Number of individual outflow operations.
    pub op_count: AtomicU64,
}

impl TypeCounters {
    pub fn add_requested(&self, amount: u64) {
        self.requested.fetch_add(amount, Ordering::Relaxed);
        self.op_count.fetch_add(1, Ordering::Relaxed);
    }
    pub fn add_recovered(&self, amount: u64) {
        self.recovered.fetch_add(amount, Ordering::Relaxed);
    }
    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.requested.load(Ordering::Relaxed),
            self.recovered.load(Ordering::Relaxed),
            self.op_count.load(Ordering::Relaxed),
        )
    }
}

/// Process-global ledger. One instance lives in `LEDGER`; every bank
/// outflow and sweep inflow records here.
///
/// Type taxonomy mirrors `bank_plan::Balances`:
///   - `platform` — Credits from bank Platform-address pool (`fund_address`).
///   - `identity` — Credits from bank Platform to bank identity (E3 top-up).
///   - `shielded` — Credits from bank Platform to shielded pool (E4).
///   - `core`     — L1 duffs from bank Core addresses (`send_core_to`).
///
/// Additionally:
///   - `e5_core_locked_duff` — duffs consumed by the E5 asset-lock
///     bootstrap (bank-internal, not test-wallet outflow).
///   - `dust_abandoned_credits` — credits abandoned because residual
///     was below `min_input_amount`.
#[derive(Debug, Default)]
pub struct FundingLedger {
    pub platform: TypeCounters,
    pub identity: TypeCounters,
    pub shielded: TypeCounters,
    pub core: TypeCounters,
    /// E5 asset-lock: duffs locked to bootstrap Platform credits.
    pub e5_core_locked_duff: AtomicU64,
    /// Credits abandoned as dust (below `min_input_amount`).
    pub dust_abandoned_credits: AtomicU64,
}

static LEDGER: OnceLock<FundingLedger> = OnceLock::new();

/// Return the process-global ledger, initialising it on first call.
pub fn ledger() -> &'static FundingLedger {
    LEDGER.get_or_init(FundingLedger::default)
}

// ── Recording helpers ────────────────────────────────────────────────

pub fn record_platform_requested(credits: Credits) {
    ledger().platform.add_requested(credits);
}

pub fn record_core_requested(duffs: u64) {
    ledger().core.add_requested(duffs);
}

pub fn record_identity_requested(credits: Credits) {
    ledger().identity.add_requested(credits);
}

pub fn record_shielded_requested(credits: Credits) {
    ledger().shielded.add_requested(credits);
}

pub fn record_e5_lock(duffs: u64) {
    ledger()
        .e5_core_locked_duff
        .fetch_add(duffs, Ordering::Relaxed);
}

pub fn record_platform_recovered(credits: Credits) {
    ledger().platform.add_recovered(credits);
}

pub fn record_identity_recovered(credits: Credits) {
    ledger().identity.add_recovered(credits);
}

pub fn record_core_recovered(duffs: u64) {
    ledger().core.add_recovered(duffs);
}

pub fn record_dust_abandoned(credits: Credits) {
    ledger()
        .dust_abandoned_credits
        .fetch_add(credits, Ordering::Relaxed);
}

// ── Unit conversion ──────────────────────────────────────────────────

const CREDITS_PER_DUFF: u64 = 1_000;
const DUFFS_PER_DASH: u64 = 100_000_000; // 1 DASH = 1e8 duffs

fn credits_to_dash(credits: u64) -> f64 {
    credits as f64 / (CREDITS_PER_DUFF as f64 * DUFFS_PER_DASH as f64)
}

fn duffs_to_dash(duffs: u64) -> f64 {
    duffs as f64 / DUFFS_PER_DASH as f64
}

// ── Report ───────────────────────────────────────────────────────────

/// Env var that gates verbose stderr output. When set to a truthy value
/// (`1`/`true`/`yes`/`on`), the full tabular report is written to stderr.
/// When unset/falsy, only a compact `tracing::info!` line is emitted.
pub const FUNDING_REPORT_VAR: &str = "PLATFORM_WALLET_E2E_FUNDING_REPORT";

/// Render and emit the end-of-suite funding summary.
///
/// Always fires at suite end (from `SetupGuard::Drop` when `prev == 1`).
/// Full tabular output is gated on `PLATFORM_WALLET_E2E_FUNDING_REPORT=1`
/// to avoid noise in non-calibration runs; the compact `tracing::info!`
/// fires unconditionally.
pub fn print_report() {
    let l = ledger();

    let (plat_req, plat_rec, plat_ops) = l.platform.snapshot();
    let (id_req, id_rec, id_ops) = l.identity.snapshot();
    let (sh_req, sh_rec, sh_ops) = l.shielded.snapshot();
    let (core_req, core_rec, core_ops) = l.core.snapshot();
    let e5_lock = l.e5_core_locked_duff.load(Ordering::Relaxed);
    let dust = l.dust_abandoned_credits.load(Ordering::Relaxed);

    let plat_net = plat_req.saturating_sub(plat_rec);
    let id_net = id_req.saturating_sub(id_rec);
    let sh_net = sh_req.saturating_sub(sh_rec);
    let core_net = core_req.saturating_sub(core_rec);

    // DASH-normalised grand total: all credit types → duffs → DASH,
    // then add Core duffs directly.
    let total_credits_net = plat_net.saturating_add(id_net).saturating_add(sh_net);
    let total_dash = credits_to_dash(total_credits_net) + duffs_to_dash(core_net);

    tracing::info!(
        target: "platform_wallet::e2e::funding_ledger",
        platform_requested = plat_req,
        platform_recovered = plat_rec,
        platform_net = plat_net,
        identity_requested = id_req,
        identity_net = id_net,
        shielded_requested = sh_req,
        shielded_net = sh_net,
        core_duff_requested = core_req,
        core_duff_net = core_net,
        e5_core_locked_duff = e5_lock,
        dust_abandoned_credits = dust,
        total_dash = format!("{total_dash:.6}"),
        "═══ FUNDING LEDGER SUMMARY (end of suite) ═══"
    );

    let verbose = is_truthy(std::env::var(FUNDING_REPORT_VAR).ok().as_deref());
    if !verbose {
        return;
    }

    // Full tabular report to stderr (grep for E2E-FUNDING-REPORT).
    eprintln!();
    eprintln!("══════════════════════════════════════════════════════════════");
    eprintln!("  E2E-FUNDING-REPORT  (set {FUNDING_REPORT_VAR}=0 to silence)");
    eprintln!("══════════════════════════════════════════════════════════════");
    eprintln!(
        "  {:12}  {:>18}  {:>18}  {:>18}  {:>6}",
        "Type", "Requested", "Recovered", "Net", "Ops"
    );
    eprintln!("  {}", "─".repeat(78));
    eprintln!(
        "  {:12}  {:>18}  {:>18}  {:>18}  {:>6}",
        "Platform©",
        fmt_credits(plat_req),
        fmt_credits(plat_rec),
        fmt_credits(plat_net),
        plat_ops
    );
    eprintln!(
        "  {:12}  {:>18}  {:>18}  {:>18}  {:>6}",
        "Identity©",
        fmt_credits(id_req),
        fmt_credits(id_rec),
        fmt_credits(id_net),
        id_ops
    );
    eprintln!(
        "  {:12}  {:>18}  {:>18}  {:>18}  {:>6}",
        "Shielded©",
        fmt_credits(sh_req),
        fmt_credits(sh_rec),
        fmt_credits(sh_net),
        sh_ops
    );
    eprintln!(
        "  {:12}  {:>18}  {:>18}  {:>18}  {:>6}",
        "Core(duff)",
        fmt_u64(core_req),
        fmt_u64(core_rec),
        fmt_u64(core_net),
        core_ops
    );
    eprintln!("  {}", "─".repeat(78));
    eprintln!("  {:12}  {:>55.6} DASH", "GRAND NET", total_dash);
    eprintln!(
        "  {:12}  {:>18}  (E5 bootstrap, bank-internal)",
        "E5 lock©",
        fmt_u64(e5_lock)
    );
    eprintln!(
        "  {:12}  {:>18}  (below min_input_amount)",
        "Dust aband©",
        fmt_credits(dust)
    );
    eprintln!("══════════════════════════════════════════════════════════════");
    eprintln!("  © = credits   duff = Layer-1 duffs");
    eprintln!("  Recovered amounts are gross (fee not subtracted).");
    eprintln!("  Net = Requested − Recovered per type.");
    eprintln!();
}

fn fmt_credits(c: u64) -> String {
    format!("{c}©")
}

fn fmt_u64(v: u64) -> String {
    format!("{v}")
}

fn is_truthy(raw: Option<&str>) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    let t = raw.trim();
    t == "1"
        || t.eq_ignore_ascii_case("true")
        || t.eq_ignore_ascii_case("yes")
        || t.eq_ignore_ascii_case("on")
}

// ── Unit tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_counters_accumulate_correctly() {
        let c = TypeCounters::default();
        c.add_requested(100);
        c.add_requested(200);
        c.add_recovered(50);
        let (req, rec, ops) = c.snapshot();
        assert_eq!(req, 300);
        assert_eq!(rec, 50);
        assert_eq!(ops, 2);
    }

    #[test]
    fn net_saturates_at_zero_when_recovered_exceeds_requested() {
        let c = TypeCounters::default();
        c.add_requested(50);
        c.add_recovered(200); // sweep returns more than we recorded (e.g. prior run)
        let (req, rec, _) = c.snapshot();
        // net via saturating_sub
        assert_eq!(req.saturating_sub(rec), 0);
    }

    #[test]
    fn credits_to_dash_conversion_1_dash() {
        // 1 DASH = 1e8 duffs × 1000 credits/duff = 1e11 credits
        assert!((credits_to_dash(100_000_000_000) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn duffs_to_dash_conversion_1_dash() {
        assert!((duffs_to_dash(100_000_000) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn grand_total_dash_sums_credit_and_core_types() {
        // 100B credits (= 1 DASH) + 1e8 duffs (= 1 DASH) → 2.0 DASH net
        let net_credits: u64 = 100_000_000_000;
        let net_core_duff: u64 = 100_000_000;
        let total = credits_to_dash(net_credits) + duffs_to_dash(net_core_duff);
        assert!((total - 2.0).abs() < 1e-9);
    }

    #[test]
    fn is_truthy_recognises_variants() {
        for v in ["1", "true", "TRUE", "yes", "on", "  1\t"] {
            assert!(is_truthy(Some(v)), "{v}");
        }
        for v in ["0", "false", "no", "off", "", "abc"] {
            assert!(!is_truthy(Some(v)), "{v}");
        }
        assert!(!is_truthy(None));
    }
}
