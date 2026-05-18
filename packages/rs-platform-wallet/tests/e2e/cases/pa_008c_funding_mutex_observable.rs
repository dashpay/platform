//! PA-008c — Observable serialisation of `FUNDING_MUTEX`.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-008c.
//! Priority: P2.
//!
//! ## What this test pins
//!
//! PA-008 / PA-008b prove that all concurrent fund calls *succeed*.
//! PA-008c is the stronger contract: prove the
//! [`crate::framework::bank::FUNDING_MUTEX`] is doing the
//! serialising. A future refactor that drops the mutex but happens to
//! win the race in CI would still pass PA-008/PA-008b but must fail
//! PA-008c.
//!
//! ## Mechanism
//!
//! The harness instruments
//! `BankWallet::fund_address` with a per-call `(seq, entry_ns,
//! exit_ns)` triple captured under the mutex (entry AFTER `lock().await`
//! resolves, exit BEFORE the guard drops). A drain accessor
//! [`BankWallet::funding_mutex_history`] returns the entries in
//! insertion order and clears the buffer.
//!
//! This file uses the instrumentation harness-side; production code
//! is unchanged.
//!
//! ## Flow
//!
//! 1. Drain any prior entries from sibling tests.
//! 2. Spawn three concurrent `bank.fund_address` tasks against three
//!    distinct receive addresses on the same wallet.
//! 3. Await all three.
//! 4. Drain the history. Assert:
//!    - Exactly three entries are present (one per spawned future).
//!    - Sorted by `seq`, the sequence numbers are strictly monotonic
//!      across the drain (mutex acquisition order is well-defined).
//!    - For every consecutive pair `(i, i+1)`,
//!      `entries[i].exit_ns <= entries[i+1].entry_ns`. The windows
//!      are pairwise non-overlapping — the mutex is actually
//!      serialising.
//!
//! ## Why three (not two)
//!
//! Two parallel funders is the minimum contention case; three
//! exercises the queueing contract that catches a hypothetical
//! "first-and-last" mutex implementation that drops the middle waiter.
//!
//! ## Parallel-safe assertions
//!
//! `FUNDING_MUTEX_HISTORY` is a process-global ring buffer that EVERY
//! `bank.fund_address` call writes to — including sibling tests running
//! in other worker threads under `--test-threads>1`. We therefore can
//! NOT assert strict cardinality (`history.len() == 3`); a sibling
//! test that funds during our fan-in window would inflate the count.
//!
//! Instead we check the contract that holds globally:
//!   - **At least 3** entries are present (our fan-in must have
//!     populated the buffer).
//!   - Sorted by `seq`, pairs are pairwise non-overlapping
//!     (`prev.exit_ns <= next.entry_ns`). This is the substance of
//!     the mutex's serialisation contract — it holds across ALL
//!     entries in the buffer, ours or anyone else's.
//!   - `FUNDING_MUTEX_SEQ` is strictly monotonic (atomic counter
//!     never reuses or decrements).
//!
//! Removing the strict-3 assertion is intentional: under serial
//! execution (`--test-threads=1`) sibling tests can't race in, so the
//! count would be 3 — but we don't gain signal by failing on a `≥ 3`
//! observation that's still consistent with the contract.

use std::time::Duration;

use crate::framework::bank::FundingMutexHistoryEntry;
use crate::framework::prelude::*;

/// Gross credits each fund call submits. Bank uses
/// `[ReduceOutput(0)]`; recipient receives `FUND_AMOUNT − bank_fee`.
const FUND_AMOUNT: u64 = 30_000_000;

/// Lower bound on each recipient's balance after the bank's fee
/// deduction. Same shape as PA-008's floor.
const FUND_FLOOR: u64 = 1_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared)]
async fn pa_008c_funding_mutex_serialisation_observable() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // ---- Derive three distinct receive addresses by funding+marking each. ----
    // Each `next_unused_address` hand-out is reserved (Found-026
    // `bc87e4dec9`); mirror PA-008's marker pattern (marks retained as
    // no-op documentation). Sequential funds here are NOT what the assertion
    // pins — we only care about the post-marker concurrent fan-in
    // below. We DRAIN the history after the markers so the assertion
    // sees only the three concurrent entries.
    const MARKER_AMOUNT: u64 = 30_000_000;
    const MARKER_FLOOR: u64 = 1_000_000;

    let addr_a = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_a");
    s.ctx
        .bank()
        .fund_address(&addr_a, MARKER_AMOUNT)
        .await
        .expect("bank.fund_address marker a");
    wait_for_balance(&s.test_wallet, &addr_a, MARKER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_a marker never observed");

    let addr_b = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_b");
    assert_ne!(
        addr_a, addr_b,
        "addr_b must differ from addr_a — each hand-out is reserved (Found-026)"
    );
    s.ctx
        .bank()
        .fund_address(&addr_b, MARKER_AMOUNT)
        .await
        .expect("bank.fund_address marker b");
    wait_for_balance(&s.test_wallet, &addr_b, MARKER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_b marker never observed");

    let addr_c = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_c");
    assert_ne!(addr_a, addr_c);
    assert_ne!(addr_b, addr_c);

    // Drain whatever the markers + sibling tests recorded so the
    // post-fan-in drain contains ONLY our three concurrent entries.
    let _pre = s.ctx.bank().funding_mutex_history();

    // ---- Concurrent funds. PA-008's contract — but here we drain the
    // history afterwards and assert observable serialisation. ----
    let bank = s.ctx.bank();
    let (r_a, r_b, r_c) = tokio::join!(
        bank.fund_address(&addr_a, FUND_AMOUNT),
        bank.fund_address(&addr_b, FUND_AMOUNT),
        bank.fund_address(&addr_c, FUND_AMOUNT),
    );
    r_a.expect("concurrent fund a");
    r_b.expect("concurrent fund b");
    r_c.expect("concurrent fund c");

    // Wait for each address to observe its concurrent fund so any
    // sibling test that piggy-backs on FUNDING_MUTEX between the
    // join and the drain doesn't pollute our window. wait_for_balance
    // doesn't acquire FUNDING_MUTEX itself, so this is safe.
    wait_for_balance(&s.test_wallet, &addr_a, FUND_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_a never observed concurrent fund");
    wait_for_balance(&s.test_wallet, &addr_b, FUND_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_b never observed concurrent fund");
    wait_for_balance(&s.test_wallet, &addr_c, FUND_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_c never observed concurrent fund");

    // ---- Assertions on the drained history. ----
    let history = s.ctx.bank().funding_mutex_history();

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_008c",
        entries = ?history,
        "FUNDING_MUTEX observed history"
    );

    // (1) Cardinality lower bound: our three concurrent funds must
    // have populated the buffer. Strict equality (`== 3`) would fail
    // under `--test-threads>1` if a sibling test funds during our
    // fan-in window — `FUNDING_MUTEX_HISTORY` is process-global and
    // every `bank.fund_address` writes to it. Loosening to `>= 3`
    // keeps the contract honest under parallel execution; the
    // serialisation property checked in (3) holds across ALL entries
    // regardless of who recorded them.
    assert!(
        history.len() >= 3,
        "PA-008c: expected at least 3 FUNDING_MUTEX entries from the \
         concurrent fan-in, observed {}: {history:?}",
        history.len()
    );

    // (2) Sequence: strictly monotonic. The instrumentation increments
    // FUNDING_MUTEX_SEQ atomically per acquisition, so a non-monotonic
    // sequence here would mean the atomic counter is broken — not a
    // contract failure of the mutex itself, but worth pinning.
    let mut by_seq: Vec<FundingMutexHistoryEntry> = history.clone();
    by_seq.sort_by_key(|e| e.seq);
    for w in by_seq.windows(2) {
        assert!(
            w[0].seq < w[1].seq,
            "PA-008c: FUNDING_MUTEX_SEQ must be strictly monotonic; \
             got prev={prev:?} next={next:?} (full history: {history:?})",
            prev = w[0],
            next = w[1],
        );
    }

    // (3) Pairwise non-overlap, sorted by acquisition sequence. This is
    // the *substance* of the contract: serialisation means the i-th
    // critical section completes before the (i+1)-th begins.
    //
    // entry_ns / exit_ns are sampled inside the lock in fund_address;
    // exit_ns is captured BEFORE the guard drops, so a strict
    // `prev.exit_ns <= next.entry_ns` is the right relation. Equality
    // is allowed for back-to-back acquisitions where the next waiter
    // wakes in the same nanosecond — extremely rare on real hardware
    // but legal under the contract.
    for w in by_seq.windows(2) {
        assert!(
            w[0].exit_ns <= w[1].entry_ns,
            "PA-008c: FUNDING_MUTEX critical sections overlapped — \
             prev (seq={pseq}) exit_ns={pexit}, \
             next (seq={nseq}) entry_ns={nentry}; \
             a removal of FUNDING_MUTEX would surface here. \
             Full history (seq-sorted): {by_seq:?}",
            pseq = w[0].seq,
            pexit = w[0].exit_ns,
            nseq = w[1].seq,
            nentry = w[1].entry_ns,
        );
    }

    // (4) Each individual window is well-formed: exit_ns >= entry_ns.
    // Defensive check — instrumentation samples the same monotonic
    // anchor on both sides, so a violation here would indicate either
    // a clock anomaly or an instrumentation bug. The contract is
    // observable serialisation, but a single-window violation would
    // invalidate the cross-window assertion above.
    for entry in &by_seq {
        assert!(
            entry.exit_ns >= entry.entry_ns,
            "PA-008c: malformed entry (exit_ns < entry_ns): {entry:?}"
        );
    }

    s.teardown().await.expect("teardown");
}
