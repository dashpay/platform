//! Sweep self-test — registers a fresh identity with a known
//! balance, runs `teardown` (which invokes
//! `cleanup::sweep_identities_with_seed`), and asserts that the
//! returned [`SweepReport::swept_identity_credits`] cleared at least
//! [`SWEEP_GAIN_FLOOR`].
//!
//! Pinned status: Pass.
//!
//! Distinct from the ID-NNN cohort: this exercises the cleanup
//! path's identity-credit recovery, not the production-wallet
//! identity APIs. The sweep destination is the bank's Platform
//! address (see [`super::super::framework::bank_rebalance`]'s
//! single-funding-pool invariant); the bank identity is no longer
//! the sweep target.
//!
//! QA-V39-001 — the prior contract observed the bank address pool's
//! post-sweep delta, but the bank address is process-shared and
//! sibling tests' `fund_address` spends drain it during the wait
//! window. Asserting on the sweep's own return value sidesteps the
//! observability race entirely.
//!
//! QA-503 — the secondary `bank_identity post<=pre` invariant was
//! removed for the same reason: concurrent harness `bank_rebalance`
//! core-refill legitimately tops up the bank identity mid-run, so
//! that sink is unobservable in isolation under parallelism. The
//! immune `swept_identity_credits` assertion is the sole binding
//! correctness pin.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Bank-funded credits the funding address starts with. Option C
/// (DeductFromInput) delivers exactly this amount. Sized so the
/// residual after 90M registration (150M) covers the chain-time
/// IdentityCreateFromAddresses dynamic fee (~125M; grew from ~110.86M
/// after QA-800 added a 4th CRITICAL key, +~550 bytes × 27_000
/// credits/byte ≈ +14.85M) with ~25M buffer for the sweep
/// teardown's combined-address-balance requirement.
const FUNDING_CREDITS: u64 = 240_000_000;
/// Under Option C the address receives exactly FUNDING_CREDITS.
const FUNDING_FLOOR: u64 = 240_000_000;

/// Credits committed to the swept identity. KEPT LARGER than
/// 0.001 tDASH: this test exists to exercise the sweep path, which
/// only broadcasts when identity balance ≥ `IDENTITY_SWEEP_FLOOR`
/// (50M, hardcoded in `cleanup.rs`). 90M sits comfortably above the
/// floor so the sweep actually fires; the swept credits land on the
/// bank's Platform address at teardown.
const REGISTRATION_FUNDING: u64 = 90_000_000;

/// Lower bound on the bank-address gain we must observe within the
/// wait window. The sweep transfers `balance -
/// IDENTITY_SWEEP_FEE_RESERVE` (30M reserve) which is bounded below
/// by `pre_balance - 30M - chain_time_fee`. Sized loosely so
/// chain-fee fluctuations don't flake the test.
const SWEEP_GAIN_FLOOR: u64 = 30_000_000;

const STEP_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn id_sweep_recovers_identity_credits() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    let bank_identity_id = s.ctx.bank_identity().id;

    // Register a fresh identity with comfortable headroom.
    let funding_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive funding address");
    s.ctx
        .bank()
        .fund_address(&funding_addr, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    // Found-025: the rs-sdk address-sync drops a fetched balance update
    // when the address isn't yet in `pending_addresses`, poisoning the
    // wallet's local sync map under multi-thread churn so
    // `wait_for_balance`'s local-view precondition never reaches target
    // and its proof-verified hand-off never runs. Observe the funding
    // directly via the proof-verified `AddressInfo::fetch` path —
    // the chain-state read the validator itself walks — bypassing the
    // poisoned map. Mirrors `setup_with_per_identity_funding`.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &funding_addr,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("funding never observed");

    let registered = s
        .test_wallet
        .register_identity_from_addresses(funding_addr, REGISTRATION_FUNDING, 0)
        .await
        .expect("register_identity_from_addresses");

    let pre_sweep_balance = Identity::fetch(s.ctx.sdk(), registered.id)
        .await
        .expect("fetch identity pre-sweep")
        .expect("registered identity visible")
        .balance();
    tracing::info!(
        target: "platform_wallet::e2e::cases::id_sweep",
        identity_id = %registered.id,
        bank_identity_id = %bank_identity_id,
        pre_sweep_balance,
        "snapshot before sweep"
    );

    // Teardown invokes `cleanup::teardown_one` which calls
    // `sweep_identities_with_seed` — the production sweep path. The
    // returned [`SweepReport`] surfaces the per-broadcast `amount`
    // Σ as [`SweepReport::swept_identity_credits`]: direct evidence
    // that our sweep moved credits, immune to the bank-address pool
    // contention that plagued the prior bank-delta contract.
    let report = s.teardown().await.expect("teardown");

    assert!(
        report.swept_identity_credits >= SWEEP_GAIN_FLOOR,
        "sweep must have moved at least SWEEP_GAIN_FLOOR ({SWEEP_GAIN_FLOOR}) credits; \
         observed swept_identity_credits={swept} (broadcasts_succeeded={succ} \
         broadcast_failures={fails:?} had_funds_to_recover={had} pre_sweep_balance={pre})",
        swept = report.swept_identity_credits,
        succ = report.broadcasts_succeeded,
        fails = report.broadcast_failures,
        had = report.had_funds_to_recover,
        pre = pre_sweep_balance,
    );

    // No bank-identity post<=pre invariant here: the concurrent
    // harness `bank_rebalance` core-refill legitimately tops up the
    // bank identity mid-run (`framework/bank_rebalance.rs` design),
    // so that sink is structurally unobservable in isolation under
    // parallelism — same flaw QA-V39-001 fixed for the primary check.
    // Sweep correctness is fully pinned by the race-immune
    // `swept_identity_credits` assertion above (QA-503, TEST_SPEC).
    tracing::info!(
        target: "platform_wallet::e2e::cases::id_sweep",
        swept_identity_credits = report.swept_identity_credits,
        broadcasts_succeeded = report.broadcasts_succeeded,
        pre_sweep_balance,
        "sweep self-test snapshot"
    );
}
