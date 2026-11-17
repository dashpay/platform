//! SH-020 — ADVERSARIAL: double-spend the same note across two
//! transitions (Type 17) — backend MUST reject the second [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-020. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails.
//!
//! Attack: build two distinct, individually-valid unshield transitions
//! that both spend the SAME shielded note (same nullifier), bypassing the
//! wallet's `reserve_unspent_notes` via the build-against-note seam, and
//! broadcast both. Exactly ONE must COMMIT; the second must be rejected
//! because its Orchard nullifier is already in Drive's spent set
//! (`NullifierAlreadySpentError`, code 40901).
//!
//! The verdict is read at CONSENSUS, not at `check_tx` (SD-002): both
//! transitions can pass mempool admission, so the case broadcasts both
//! and then waits for each one's COMMIT outcome. A transition counts as
//! committed only if it both passed `check_tx` AND `wait_commit_raw`
//! returned a verified proof result.
//!
//! The loser must carry consensus code 40901. One exception: a loser that
//! CheckTx admitted and whose result then timed out (mempool recheck can evict
//! it once the winner commits) passes only on chain evidence of no effect —
//! the winner committed the shared nullifier and the loser's destination is
//! unchanged. Any other loser error fails.
//!
//! RED if the backend commits both (double-spend — CRITICAL fund forgery)
//! or commits neither (liveness bug).

#![cfg(feature = "shielded")]

use std::time::Duration;

use dpp::shielded::compute_minimum_shielded_fee;
use dpp::version::PlatformVersion;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, assert_consensus_rejection, bind_shielded, broadcast_raw,
    build_unshield_st_against_notes, shielded_prover, teardown_sweep_shielded, unspent_notes,
    wait_commit_raw, wait_for_shielded_balance, wait_for_unspent_notes,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::AddressInfo;
use dpp::address_funds::PlatformAddress;

const FUNDING_CREDITS: u64 = 1_400_000_000;
const SHIELD_AMOUNT: u64 = 200_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Consensus commit needs block production + proof — longer than the
/// per-step funding/sync gate.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_020_double_spend_two_transitions() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_020",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL set to a falsy value — abuse case opted out (no-op pass)"
        );
        return;
    }

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");

    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");
    s.ctx
        .bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync");
    s.test_wallet
        .platform_wallet()
        .shielded_shield_from_account(
            &handle.coordinator,
            0,
            0,
            SHIELD_AMOUNT,
            s.test_wallet.address_signer(),
            prover,
        )
        .await
        .expect("shield_from_account");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("shielded balance never reached SHIELD_AMOUNT");

    // Capture the single synced note; build TWO unshields against it.
    let notes = unspent_notes(&s.test_wallet, &handle, 0)
        .await
        .expect("capture unspent notes");
    assert!(
        !notes.is_empty(),
        "expected one synced note to double-spend"
    );
    let one_note = vec![notes[0].clone()];

    let exact_fee = compute_minimum_shielded_fee(1, s.ctx.sdk().version())
        .expect("compute_minimum_shielded_fee");
    let dst_a = s.test_wallet.next_unused_address().await.expect("dst_a");
    let dst_b = s.test_wallet.next_unused_address().await.expect("dst_b");

    let st_a = build_unshield_st_against_notes(
        &s.test_wallet,
        &handle,
        0,
        &dst_a,
        UNSHIELD_AMOUNT,
        exact_fee,
        &one_note,
    )
    .await
    .expect("build first unshield against note");
    let st_b = build_unshield_st_against_notes(
        &s.test_wallet,
        &handle,
        0,
        &dst_b,
        UNSHIELD_AMOUNT,
        exact_fee,
        &one_note,
    )
    .await
    .expect("build second unshield against the SAME note");

    // BEFORE state: both unshield destinations are fresh (0 credits). Read
    // them via the proof-verified on-chain path so the verdict rests on a
    // real before/after delta, not an assumption.
    let before_a = fetch_credits(s.ctx.sdk(), &dst_a).await;
    let before_b = fetch_credits(s.ctx.sdk(), &dst_b).await;

    // Broadcast BOTH first (check_tx / mempool admission) so the two
    // same-nullifier spends are in flight before either is processed.
    let bcast_a = broadcast_raw(s.ctx.sdk(), &st_a).await;
    let bcast_b = broadcast_raw(s.ctx.sdk(), &st_b).await;

    let admitted_a = bcast_a.is_ok();
    let admitted_b = bcast_b.is_ok();
    let commit_a = match bcast_a {
        Ok(()) => wait_commit_raw(s.ctx.sdk(), &st_a, COMMIT_TIMEOUT).await,
        Err(error) => Err(error),
    };
    let commit_b = match bcast_b {
        Ok(()) => wait_commit_raw(s.ctx.sdk(), &st_b, COMMIT_TIMEOUT).await,
        Err(error) => Err(error),
    };
    assert_eq!(
        [commit_a.is_ok(), commit_b.is_ok()]
            .into_iter()
            .filter(|ok| *ok)
            .count(),
        1,
        "exactly one spend must commit; a={commit_a:?}, b={commit_b:?}"
    );
    let (winner, loser, before_winner, before_loser, loser_admitted, rejection) =
        match (&commit_a, &commit_b) {
            (Ok(_), Err(error)) => (&dst_a, &dst_b, before_a, before_b, admitted_b, error),
            (Err(error), Ok(_)) => (&dst_b, &dst_a, before_b, before_a, admitted_a, error),
            _ => unreachable!("one successful commit asserted above"),
        };
    let credited = wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        winner,
        before_winner + UNSHIELD_AMOUNT,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("winning spend must credit at least the amount sent");
    assert!(credited - before_winner >= UNSHIELD_AMOUNT);
    assert_eq!(
        fetch_credits(s.ctx.sdk(), loser).await,
        before_loser,
        "rejected same-note spend must not credit its destination"
    );
    match rejection {
        // Admitted at CheckTx, then evicted or never reported before the
        // deadline. The winner committed the shared nullifier, so the loser can
        // never execute, and its destination is proven unchanged above.
        dash_sdk::Error::TimeoutReached(..) if loser_admitted => tracing::warn!(
            target: "platform_wallet::e2e::cases::sh_020",
            ?rejection,
            "ADV-VERDICT probe=SH-020 stage=consensus result=unobserved; \
             no effect proven by the committed nullifier and unchanged destination"
        ),
        _ => assert_consensus_rejection(rejection, &[40901]),
    }

    // Corroborate: the note's value left the pool exactly ONCE. The seam
    // bypassed local reservation, so wait until sync marks the original spent
    // and scans the winner's change: one unspent note, a different nullifier.
    let residual =
        wait_for_unspent_notes(
            &s.test_wallet,
            &handle,
            0,
            STEP_TIMEOUT,
            |notes| match notes {
                [change] if change.nullifier != one_note[0].nullifier => Some(change.value),
                _ => None,
            },
        )
        .await
        .expect("original note spent and winner change scanned");
    assert!(
        residual <= SHIELD_AMOUNT - UNSHIELD_AMOUNT,
        "SH-020: change must exclude the committed spend; residual={residual}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}

/// Proof-verified balance; an absent address is zero, a failed read is fatal.
async fn fetch_credits(sdk: &dash_sdk::Sdk, addr: &PlatformAddress) -> u64 {
    AddressInfo::fetch(sdk, *addr)
        .await
        .expect("proof-verified address balance")
        .map_or(0, |info| info.balance)
}
