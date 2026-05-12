//! Found-006 — `top_up_identity_with_funding` ignores caller-supplied
//! `topup_index`.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Found bugs → Found-006).
//! Pinned status: BUG-PIN, expected RED until upstream fix.
//!
//! ## Bug shape
//!
//! `wallet/identity/network/top_up.rs:60-106`. The doc says
//! `topup_index` is "An incrementing index distinguishing successive
//! top-ups for the same identity". The implementation prefixes the
//! parameter with `_` and the function body derives the funding key
//! path from `identity_index` alone (a `TODO(platform-wallet)` comment
//! confirms the parameter is unused). Two consecutive top-ups for the
//! same identity therefore derive from the same `(IdentityTopUp,
//! identity_index)` path — yielding the same one-time key address,
//! the same outpoint candidate, and a likely-duplicate asset-lock
//! transaction or nonce collision.
//!
//! ## What this test asserts
//!
//! Two consecutive `top_up_identity_with_funding` calls for the same
//! identity, with different `topup_index` values, must produce
//! DIFFERENT asset-lock txids (the doc-stated contract). Today they
//! produce the SAME funding-output address — the second call either
//! collides with the first's outpoint or builds a duplicate
//! transaction.
//!
//! ## FAILS UNTIL
//!
//! Found-006's upstream root cause is in `key_wallet`'s
//! `CreditOutputFunding` — the type plumbs only `identity_index` and
//! has no `topup_index` field. Downstream cannot fix Found-006
//! without an upstream API change first. So this test stays red
//! until the upstream `CreditOutputFunding` gains a `top_up_index`
//! field AND the downstream `top_up_identity_with_funding` wires it
//! through.
//!
//! Run with `cargo test -- --ignored` against a testnet bank with
//! Core funding to observe the failure mode.

use std::time::Duration;

use dpp::identity::Identity;
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet::wallet::identity::types::funding::TopUpFundingMethod;

use crate::framework::prelude::*;
use dash_sdk::platform::Fetch;

/// Core (Layer-1) duffs for the test wallet. Sized for two
/// back-to-back asset-lock top-ups (TOP_UP_AMOUNT × 2 + fee headroom).
const TEST_WALLET_CORE_FUNDING: u64 = 300_000_000;

/// Per-top-up asset-lock amount (duffs). Same shape as ID-002b.
const TOP_UP_AMOUNT: u64 = 100_000_000;

/// DIP-9 identity slot. Both top-ups target the same identity (this
/// IS the test).
const IDENTITY_INDEX: u32 = 0;

/// Registration funding via the cheaper address-funded path.
const REGISTRATION_FUNDING: u64 = 100_000_000;
const REGISTRATION_FUNDING_CREDITS: u64 = REGISTRATION_FUNDING + 150_000_000;

/// Per-step wait deadline.
const STEP_TIMEOUT: Duration = Duration::from_secs(180);

#[ignore = "Found-006 — bug pin. EXPECTED to fail until upstream \
            `key_wallet::CreditOutputFunding` gains a `top_up_index` \
            field. Same PLATFORM_WALLET_E2E_BANK_CORE_GATE as CR-003. \
            Run with `cargo test -- --ignored`; the failure mode \
            today is either (a) duplicate-tx rejection at broadcast, \
            (b) duplicate txids across the two tracked locks, or (c) \
            both top-ups silently consuming the same outpoint."]
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn found_006_topup_index_ignored() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet failed");

    // Register an identity via the cheap address-funded path.
    let funding_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive register address");
    s.ctx
        .bank()
        .fund_address(&funding_addr, REGISTRATION_FUNDING_CREDITS)
        .await
        .expect("bank.fund_address(register)");
    wait_for_balance(
        &s.test_wallet,
        &funding_addr,
        REGISTRATION_FUNDING_CREDITS,
        STEP_TIMEOUT,
    )
    .await
    .expect("register funding never observed");
    let registered = s
        .test_wallet
        .register_identity_from_addresses(funding_addr, REGISTRATION_FUNDING, IDENTITY_INDEX)
        .await
        .expect("register_identity_from_addresses");
    let identity_id = registered.id;
    let _ = Identity::fetch(s.ctx.sdk(), identity_id)
        .await
        .expect("fetch pre")
        .expect("identity visible");

    // First top-up — topup_index = 0.
    s.test_wallet
        .platform_wallet()
        .identity()
        .top_up_identity_with_funding(
            &identity_id,
            TopUpFundingMethod::FundWithWallet {
                amount_duffs: TOP_UP_AMOUNT,
            },
            0,
            None,
        )
        .await
        .expect(
            "first top_up_identity_with_funding (topup_index=0) — \
             expected to succeed",
        );

    // Snapshot the tracked top-up locks BEFORE the second top-up so
    // we can isolate the txid the first call produced.
    let first_locks = s
        .test_wallet
        .platform_wallet()
        .asset_locks()
        .list_tracked_locks()
        .await;
    let first_topup_txids: Vec<_> = first_locks
        .iter()
        .filter(|l| {
            matches!(
                l.funding_type,
                key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityTopUp
            )
        })
        .map(|l| l.out_point.txid)
        .collect();
    assert_eq!(
        first_topup_txids.len(),
        1,
        "PRE-pin: first top-up must have produced exactly one \
         IdentityTopUp tracked lock; got {} entries",
        first_topup_txids.len()
    );
    let first_txid = first_topup_txids[0];

    // Second top-up — topup_index = 1. Per the doc this should
    // derive a fresh funding key and produce a NEW asset-lock tx.
    // Today this call collides with the first because the
    // implementation ignores `topup_index`.
    let second_call = s
        .test_wallet
        .platform_wallet()
        .identity()
        .top_up_identity_with_funding(
            &identity_id,
            TopUpFundingMethod::FundWithWallet {
                amount_duffs: TOP_UP_AMOUNT,
            },
            1,
            None,
        )
        .await;

    // The bug surfaces in one of three ways today:
    //   (a) `second_call` errors at broadcast with a duplicate-tx /
    //       no-spendable-input shape — `_topup_index` collision means
    //       the second build re-derives the same funding key, picks the
    //       same outpoint, and produces an identical asset-lock tx.
    //   (b) `second_call` succeeds but the new tracked lock has the
    //       same txid as the first (same outpoint reused).
    //   (c) `second_call` succeeds, the txids are different, BUT only
    //       because the first call already consumed the funding
    //       outpoint and the second falls through to a different
    //       UTXO by accident — not by design.
    //
    // The doc-stated contract is "different topup_index ⇒ different
    // derivation". Pin that as the hard assertion. Today (a)/(b) make
    // this assertion fail.
    second_call.expect(
        "Found-006: second top_up_identity_with_funding (topup_index=1) \
         was expected to succeed per the doc-stated contract — failure \
         here today is the pin of the bug. After upstream fix, the \
         call should succeed and produce a fresh derivation.",
    );

    let post_locks = s
        .test_wallet
        .platform_wallet()
        .asset_locks()
        .list_tracked_locks()
        .await;
    let post_topup_txids: Vec<_> = post_locks
        .iter()
        .filter(|l| {
            matches!(
                l.funding_type,
                key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityTopUp
            )
        })
        .filter(|l| {
            matches!(
                l.status,
                AssetLockStatus::InstantSendLocked | AssetLockStatus::ChainLocked
            )
        })
        .map(|l| l.out_point.txid)
        .collect();

    assert_eq!(
        post_topup_txids.len(),
        2,
        "Found-006 POST-pin: expected 2 distinct finalised IdentityTopUp \
         tracked locks after two top-ups, got {}. Today's likely failure \
         mode: the second call collided on the same derivation and either \
         didn't produce a new tracked lock or duplicated the first's \
         outpoint.",
        post_topup_txids.len()
    );

    let second_txid = post_topup_txids
        .iter()
        .find(|t| **t != first_txid)
        .copied()
        .unwrap_or_else(|| {
            panic!(
                "Found-006 POST-pin: no tracked lock with a distinct txid \
                 from the first call's txid {first_txid} — second top-up \
                 derived the same funding key as the first. This is the \
                 bug. After upstream fix, the second call must produce a \
                 different txid."
            )
        });
    assert_ne!(
        first_txid, second_txid,
        "Found-006 POST-pin violated: the two top-up calls produced the \
         same asset-lock txid {first_txid}. `topup_index` is being \
         ignored — see TEST_SPEC.md Found-006 for the upstream root cause."
    );

    s.teardown().await.expect("teardown");
}
