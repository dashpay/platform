//! Process-global funding ledger — tracks bank outflows and sweep
//! recoveries per account type so we can calibrate funding floors
//! empirically rather than by guessing.
//!
//! The ledger is a singleton populated with `OnceLock`; its per-type
//! counters are `AtomicU64` so concurrent tests can record without a
//! `Mutex`. Ordering is `Relaxed` throughout — we need atomicity per
//! counter but no inter-counter ordering guarantees; totals are read
//! only at end-of-suite after all writers have quiesced.
//!
//! # Per-test attribution
//!
//! Use [`with_test_label`] to wrap a test body and attribute all
//! `record_platform_*` calls within that scope to the given test name.
//! [`CURRENT_TEST_LABEL`] is a `tokio::task_local!` variable: it
//! follows the async call chain (across `FUNDING_MUTEX.lock().await`
//! suspensions, etc.) regardless of which worker thread runs the future
//! at any given instant. Only platform credits are tracked per-test;
//! Core duffs remain global-only.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use dpp::fee::Credits;

// ── Task-local test label ────────────────────────────────────────────

tokio::task_local! {
    /// Current test label — set via [`with_test_label`] at test entry so
    /// per-test `record_*` helpers can attribute funding to the calling
    /// test. `None` inside the scope means "scope active but intentionally
    /// unlabeled"; absent scope means "not set" (external calls, harness
    /// setup, sweep path).
    pub static CURRENT_TEST_LABEL: Option<String>;
}

/// Scope the given future under test label `label`, attributing all
/// [`record_platform_requested`] / [`record_platform_recovered`] calls
/// within the future (and any `.await`-ed sub-futures on the same task)
/// to that label.
///
/// # Usage
/// ```ignore
/// #[tokio_shared_rt::test(shared)]
/// async fn my_test() {
///     with_test_label("my_test", async {
///         let s = setup().await.expect("setup");
///         // ... test body ...
///         s.teardown().await.expect("teardown");
///     })
///     .await;
/// }
/// ```
pub fn with_test_label<F: std::future::Future>(
    label: impl Into<String>,
    f: F,
) -> impl std::future::Future<Output = F::Output> {
    CURRENT_TEST_LABEL.scope(Some(label.into()), f)
}

/// Like [`with_test_label`] but only enters a scope when `label` is
/// `Some` and non-empty; otherwise runs `f` directly in the current
/// scope, **preserving** any outer [`CURRENT_TEST_LABEL`] value.
///
/// This is the preferred entry point inside the e2e setup helpers because
/// it avoids creating a `None`/blank scope that would shadow an enclosing
/// `Some` label.  The setup helpers use the "inherit-or-derive" pattern:
///
/// ```text
/// let existing = CURRENT_TEST_LABEL.try_with(|o| o.clone()).ok().flatten();
/// let label    = existing.or_else(|| label_from_file(location.file()));
/// maybe_with_test_label(label, async_body).await
/// ```
///
/// When called from inside an already-labeled scope (outer
/// `setup_with_*` already set the label), `existing` is `Some` and
/// `label` carries the outer label — the inner helper re-enters the same
/// scope, which is harmless. When called from a test file directly,
/// `existing` is `None` and the derived file-stem label is used.
pub async fn maybe_with_test_label<F: std::future::Future>(
    label: Option<String>,
    f: F,
) -> F::Output {
    if let Some(l) = label.filter(|l| !l.is_empty()) {
        CURRENT_TEST_LABEL.scope(Some(l), f).await
    } else {
        f.await
    }
}

// ── Per-test counters ────────────────────────────────────────────────

/// Platform-credit metrics for one test run. Values are plain `u64`
/// (not atomic) because they live behind a `Mutex` in [`PerTestMap`].
#[derive(Debug, Default, Clone)]
pub struct PerTestCounters {
    /// Gross credits requested from the bank for this test.
    pub requested: u64,
    /// Gross credits recovered (swept back to bank) for this test.
    pub recovered: u64,
    /// Number of outflow (`fund_address`) operations.
    pub op_count: u64,
}

impl PerTestCounters {
    /// Net cost: requested minus recovered, saturating at zero.
    pub fn net(&self) -> u64 {
        self.requested.saturating_sub(self.recovered)
    }
}

/// Thread-safe per-test funding breakdown. Keyed by test label string.
///
/// Lives inside [`FundingLedger`]; can also be instantiated standalone
/// for unit tests that don't want to interact with the process-global
/// singleton.
#[derive(Debug, Default)]
pub struct PerTestMap {
    inner: Mutex<HashMap<String, PerTestCounters>>,
}

impl PerTestMap {
    /// Record a credit outflow for `label`.
    pub fn record_requested(&self, label: &str, amount: u64) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.entry(label.to_string()).or_default();
        entry.requested = entry.requested.saturating_add(amount);
        entry.op_count += 1;
    }

    /// Record a credit recovery for `label`.
    pub fn record_recovered(&self, label: &str, amount: u64) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.entry(label.to_string()).or_default();
        entry.recovered = entry.recovered.saturating_add(amount);
    }

    /// Snapshot all per-test entries sorted by **NET cost descending**.
    pub fn snapshot_sorted(&self) -> Vec<(String, PerTestCounters)> {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<_> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        entries.sort_by(|a, b| b.1.net().cmp(&a.1.net()));
        entries
    }
}

// ── Global type counters ─────────────────────────────────────────────

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
///   - `per_test` — per-test platform-credit breakdown; populated only
///     for tests wrapped in [`with_test_label`].
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
    /// Per-test platform-credit breakdown. Populated when the calling
    /// test wraps its body in [`with_test_label`].
    pub per_test: PerTestMap,
}

static LEDGER: OnceLock<FundingLedger> = OnceLock::new();

/// Return the process-global ledger, initialising it on first call.
pub fn ledger() -> &'static FundingLedger {
    LEDGER.get_or_init(FundingLedger::default)
}

// ── Recording helpers ────────────────────────────────────────────────

/// Record a platform-credit outflow. Also attributes to the current
/// test label when inside a [`with_test_label`] scope.
pub fn record_platform_requested(credits: Credits) {
    let l = ledger();
    l.platform.add_requested(credits);
    // Per-test attribution: fires only when a test label is in scope.
    let _ = CURRENT_TEST_LABEL.try_with(|opt| {
        if let Some(label) = opt {
            l.per_test.record_requested(label, credits);
        }
    });
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

/// Record a platform-credit recovery (sweep back to bank). Also
/// attributes to the current test label when inside a
/// [`with_test_label`] scope.
pub fn record_platform_recovered(credits: Credits) {
    let l = ledger();
    l.platform.add_recovered(credits);
    let _ = CURRENT_TEST_LABEL.try_with(|opt| {
        if let Some(label) = opt {
            l.per_test.record_recovered(label, credits);
        }
    });
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

/// 1 DASH = 1e8 duffs × 1000 credits/duff = 100_000_000_000 credits.
pub const CREDITS_PER_DASH: u64 = 100_000_000_000;

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

    // ── Per-test breakdown ───────────────────────────────────────────
    // Only populated for tests that wrap their body in with_test_label().
    let per_test = l.per_test.snapshot_sorted();
    if per_test.is_empty() {
        eprintln!();
        eprintln!("  (no per-test attribution — wrap test bodies in with_test_label() to enable)");
    } else {
        eprintln!();
        eprintln!("──────────────────────────────────────────────────────────────");
        eprintln!("  PER-TEST FUNDING BREAKDOWN (platform credits, net desc.)");
        eprintln!("  Tests NOT wrapped in with_test_label() are not shown.");
        eprintln!("──────────────────────────────────────────────────────────────");
        eprintln!(
            "  {:<42}  {:>18}  {:>18}  {:>12}  {:>4}",
            "Test", "Requested(©)", "Net(©)", "Net(DASH)", "Ops"
        );
        eprintln!("  {}", "─".repeat(100));
        for (label, counters) in &per_test {
            let net = counters.net();
            let net_dash = credits_to_dash(net);
            // Truncate long test names gracefully.
            let display_name = if label.len() > 42 {
                format!("{}…", &label[..41])
            } else {
                label.clone()
            };
            eprintln!(
                "  {:<42}  {:>18}  {:>18}  {:>12.6}  {:>4}",
                display_name,
                fmt_credits(counters.requested),
                fmt_credits(net),
                net_dash,
                counters.op_count
            );
        }
        eprintln!("  {}", "─".repeat(100));
        let total_test_req: u64 = per_test.iter().map(|(_, c)| c.requested).sum();
        let total_test_net: u64 = per_test.iter().map(|(_, c)| c.net()).sum();
        eprintln!(
            "  {:<42}  {:>18}  {:>18}  {:>12.6}  {:>4}",
            format!("TOTAL ({} tests)", per_test.len()),
            fmt_credits(total_test_req),
            fmt_credits(total_test_net),
            credits_to_dash(total_test_net),
            per_test.iter().map(|(_, c)| c.op_count).sum::<u64>()
        );
        eprintln!("──────────────────────────────────────────────────────────────");
    }
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

    // ── Per-test breakdown tests ─────────────────────────────────────

    /// `PerTestMap` accumulates requested / recovered / op_count per label.
    #[test]
    fn per_test_map_accumulates_per_label() {
        let map = PerTestMap::default();
        map.record_requested("alpha", 100_000);
        map.record_requested("alpha", 200_000);
        map.record_recovered("alpha", 50_000);
        map.record_requested("beta", 400_000);

        let entries = map.snapshot_sorted();
        assert_eq!(entries.len(), 2, "two distinct labels");

        // beta net = 400_000, alpha net = 250_000 → beta sorts first
        assert_eq!(entries[0].0, "beta");
        assert_eq!(entries[0].1.requested, 400_000);
        assert_eq!(entries[0].1.recovered, 0);
        assert_eq!(entries[0].1.op_count, 1);
        assert_eq!(entries[0].1.net(), 400_000);

        assert_eq!(entries[1].0, "alpha");
        assert_eq!(entries[1].1.requested, 300_000);
        assert_eq!(entries[1].1.recovered, 50_000);
        assert_eq!(entries[1].1.op_count, 2);
        assert_eq!(entries[1].1.net(), 250_000);
    }

    /// Net saturates at zero when recovered > requested.
    #[test]
    fn per_test_map_net_saturates_at_zero() {
        let map = PerTestMap::default();
        map.record_requested("t", 100);
        map.record_recovered("t", 500);
        let entries = map.snapshot_sorted();
        assert_eq!(entries[0].1.net(), 0, "net must saturate at 0");
    }

    /// `snapshot_sorted` returns entries in descending NET order.
    #[test]
    fn per_test_map_snapshot_sorted_by_net_descending() {
        let map = PerTestMap::default();
        // Intentionally insert in ascending order.
        map.record_requested("cheap", 10_000_000);
        map.record_requested("medium", 50_000_000);
        map.record_requested("expensive", 200_000_000);

        let entries = map.snapshot_sorted();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].0, "expensive");
        assert_eq!(entries[1].0, "medium");
        assert_eq!(entries[2].0, "cheap");
    }

    /// `CURRENT_TEST_LABEL` task-local survives across `yield_now` await
    /// points (confirms the value is task-scoped, not thread-scoped).
    #[tokio::test]
    async fn per_test_task_local_survives_yield() {
        let map = PerTestMap::default();

        // Helper: read the label from task-local scope and record on `map`.
        let record = |map: &PerTestMap, credits: u64| {
            let _ = CURRENT_TEST_LABEL.try_with(|opt| {
                if let Some(label) = opt {
                    map.record_requested(label, credits);
                }
            });
        };

        CURRENT_TEST_LABEL
            .scope(Some("gamma".to_string()), async {
                record(&map, 70_000);
                // Yield the current tokio task — the label must survive.
                tokio::task::yield_now().await;
                record(&map, 30_000);
            })
            .await;

        // Calls outside any scope must not produce entries.
        record(&map, 99_999);

        let entries = map.snapshot_sorted();
        assert_eq!(entries.len(), 1, "unlabeled calls must not appear");
        assert_eq!(entries[0].0, "gamma");
        assert_eq!(entries[0].1.requested, 100_000);
        assert_eq!(entries[0].1.op_count, 2);
    }

    /// Two concurrent tasks each carry their own independent label and
    /// their accounting does not bleed across task boundaries.
    #[tokio::test]
    async fn per_test_task_local_isolated_across_tasks() {
        use std::sync::Arc;

        let map = Arc::new(PerTestMap::default());

        let map_a = Arc::clone(&map);
        let task_a = tokio::spawn(CURRENT_TEST_LABEL.scope(
            Some("task_a".to_string()),
            async move {
                tokio::task::yield_now().await;
                let _ = CURRENT_TEST_LABEL.try_with(|opt| {
                    if let Some(label) = opt {
                        map_a.record_requested(label, 111_000);
                    }
                });
            },
        ));

        let map_b = Arc::clone(&map);
        let task_b = tokio::spawn(CURRENT_TEST_LABEL.scope(
            Some("task_b".to_string()),
            async move {
                tokio::task::yield_now().await;
                let _ = CURRENT_TEST_LABEL.try_with(|opt| {
                    if let Some(label) = opt {
                        map_b.record_requested(label, 222_000);
                    }
                });
            },
        ));

        task_a.await.expect("task_a");
        task_b.await.expect("task_b");

        // task_b net > task_a net → task_b sorts first.
        let entries = map.snapshot_sorted();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "task_b");
        assert_eq!(entries[0].1.requested, 222_000);
        assert_eq!(entries[1].0, "task_a");
        assert_eq!(entries[1].1.requested, 111_000);
    }

    /// `PerTestCounters::net` uses `CREDITS_PER_DASH` for DASH conversion.
    #[test]
    fn credits_per_dash_constant_is_correct() {
        // 1 DASH = 100B credits
        assert_eq!(CREDITS_PER_DASH, 100_000_000_000);
        // Validate round-trip with credits_to_dash
        assert!((credits_to_dash(CREDITS_PER_DASH) - 1.0).abs() < 1e-9);
    }

    // ── maybe_with_test_label wiring tests ──────────────────────────

    /// `maybe_with_test_label` with a real label attributes both
    /// setup-side (fund_address) and teardown-side (sweep) credits to
    /// the same per-test entry — verifying the wiring end-to-end without
    /// a live DAPI connection.
    #[tokio::test]
    async fn maybe_with_test_label_attributes_setup_and_teardown() {
        use std::sync::Arc;
        let map = Arc::new(PerTestMap::default());

        // Setup side: fund_address → record_platform_requested reads label
        let m = Arc::clone(&map);
        maybe_with_test_label(Some("wired_test".into()), async move {
            let _ = CURRENT_TEST_LABEL.try_with(|opt| {
                if let Some(label) = opt {
                    m.record_requested(label, 300_000_000);
                }
            });
            tokio::task::yield_now().await;
            let _ = CURRENT_TEST_LABEL.try_with(|opt| {
                if let Some(label) = opt {
                    m.record_requested(label, 200_000_000);
                }
            });
        })
        .await;

        // Teardown side: SetupGuard::teardown() enters same label for sweep
        let m = Arc::clone(&map);
        maybe_with_test_label(Some("wired_test".into()), async move {
            let _ = CURRENT_TEST_LABEL.try_with(|opt| {
                if let Some(label) = opt {
                    m.record_recovered(label, 450_000_000);
                }
            });
        })
        .await;

        let entries = map.snapshot_sorted();
        assert_eq!(entries.len(), 1, "exactly one entry: 'wired_test'");
        let (name, c) = &entries[0];
        assert_eq!(name, "wired_test");
        assert_eq!(c.requested, 500_000_000, "300M + 200M setup funding");
        assert_eq!(c.recovered, 450_000_000, "teardown sweep");
        assert_eq!(c.net(), 50_000_000, "net = 500M − 450M");
        assert_eq!(c.op_count, 2, "two fund_address ops");
    }

    /// `maybe_with_test_label(None, f)` runs `f` in the existing scope,
    /// preserving the outer label — the inner helper must NOT shadow an
    /// enclosing `Some` with a blank / `None` scope.
    #[tokio::test]
    async fn maybe_with_test_label_none_preserves_outer_scope() {
        use std::sync::Arc;
        let map = Arc::new(PerTestMap::default());

        // Outer scope has a real label (set by e.g. setup_with_n_identities).
        let m = Arc::clone(&map);
        maybe_with_test_label(Some("outer_label".into()), async move {
            // Inner call with None label (setup_with_per_identity_funding when
            // caller is a framework file and no outer scope — here we simulate
            // the framework-file case by passing None explicitly).
            maybe_with_test_label(None, async move {
                // fund_address path must still see "outer_label".
                let _ = CURRENT_TEST_LABEL.try_with(|opt| {
                    if let Some(label) = opt {
                        m.record_requested(label, 100_000_000);
                    }
                });
            })
            .await;
        })
        .await;

        let entries = map.snapshot_sorted();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].0, "outer_label",
            "outer label must survive a None inner call"
        );
        assert_eq!(entries[0].1.requested, 100_000_000);
    }
}
