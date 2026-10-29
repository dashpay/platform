//! ID-002 — Top-up identity from platform addresses.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Identity (ID) → ID-002).
//! Pinned status: red-by-design (concurrency-only) — single-thread
//! PASS; deterministic FAIL under the documented 14-thread v-run on
//! a Found-026-family `next_unused_address()` duplicate-derivation
//! race (see the RED-by-design pin at the `assert_ne!` below).
//!
//! Registers an identity (ID-001 helper), funds a second platform
//! address from the bank, then drives `top_up_from_addresses` and
//! pins the post-top-up balance delta against the topped-up amount.

use std::collections::BTreeMap;
use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, wait_for_identity_balance,
    CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// REGISTRATION_FUNDING: KEPT LARGER than 0.001 tDASH so the
// post-top-up identity balance stays above `IDENTITY_SWEEP_FLOOR`
// (50M in `cleanup.rs`) — without that, teardown silently skips the
// sweep and the credits stay stranded (Marvin v32 forensics).
// 100M sits 50M above the floor with margin for the chain-time
// sweep transfer fee (~6.5M per `state_transition_min_fees`).
// REGISTER_FUNDING_CREDITS = REGISTRATION_FUNDING + 150M headroom
// for the chain-time IdentityCreateFromAddresses fee (~125M).
const REGISTRATION_FUNDING: u64 = 100_000_000;
const REGISTER_FUNDING_CREDITS: u64 = REGISTRATION_FUNDING + 150_000_000;
const REGISTER_FUNDING_FLOOR: u64 = REGISTER_FUNDING_CREDITS;

// TOP_UP_FUNDING_CREDITS: TOP_UP_AMOUNT + 15M headroom — the
// chain-time IdentityTopUp dynamic fee (~13M) is paid from the
// address residual, NOT from the topped-up credits.
// >= 200_000 protocol minimum for asset-lock top-up
// (input_sum - output_sum >= minimum_difference=200_000).
// See dashpay/platform DPP top-up state-transition validation.
const TOP_UP_AMOUNT: Credits = 1_000_000;
const TOP_UP_FUNDING_CREDITS: u64 = 16_000_000; // 1M top-up + 15M fee headroom
const TOP_UP_FUNDING_FLOOR: u64 = TOP_UP_FUNDING_CREDITS;

// 60 s is too tight under `--test-threads=14` when ID-002 funds
// 45 000 000 duff on the top-up address while sibling cases broadcast
// concurrently — the funding broadcast lands but `wait_for_balance`'s
// chain-confirmed gate doesn't clear inside the default deadline.
// 120 s is plenty without softening the framework-wide default.
const STEP_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn id_002_top_up_identity_from_addresses() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    let register_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive register address");
    s.ctx
        .bank()
        .fund_address(&register_addr, REGISTER_FUNDING_CREDITS)
        .await
        .expect("bank.fund_address(register)");
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
        &register_addr,
        REGISTER_FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("register funding never observed");

    // Chain-confirmed gate is sdk-only and never warms the wallet's local
    // balance map; refresh it before register consumes register_addr.
    s.test_wallet.sync_balances().await.expect("pre-tx sync");

    let registered = s
        .test_wallet
        .register_identity_from_addresses(register_addr, REGISTRATION_FUNDING, 0)
        .await
        .expect("register_identity_from_addresses");

    let pre_balance = Identity::fetch(s.ctx.sdk(), registered.id)
        .await
        .expect("fetch pre")
        .expect("identity visible")
        .balance();
    assert!(
        pre_balance > 0,
        "post-registration identity balance must be non-zero (got {pre_balance})"
    );

    // Fund a second address dedicated to the top-up.
    let top_up_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive top-up address");
    // RED-by-design pin (QA-502, Found-026 family). Under the
    // documented 14-thread v-run this `assert_ne!` deterministically
    // panics with left == right: `next_unused_address()` returns a
    // DUPLICATE of `register_addr` under concurrent BLAST-sync churn
    // on the `PlatformAddressWallet` pool cursor. The Found-025
    // chain-confirmed funding gate (this branch) clears first, so
    // this is the *downstream* production cursor race it unmasked —
    // NOT a regression. The panic is the proof; the assertion stays
    // genuine. Do not weaken / skip via the `e2e` gate — fix the production race
    // upstream, then this goes green. See TEST_SPEC ID-002.
    assert_ne!(
        top_up_addr, register_addr,
        "top-up address must differ from the registration funding address"
    );
    s.ctx
        .bank()
        .fund_address(&top_up_addr, TOP_UP_FUNDING_CREDITS)
        .await
        .expect("bank.fund_address(top-up)");
    // Found-025: same poisoned-map hazard as the register-funding gate
    // above — `top_up_from_addresses` re-fetches this address's
    // balance + nonce from a round-robin DAPI replica, so gate on the
    // proof-verified chain view rather than the local sync map.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &top_up_addr,
        TOP_UP_FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("top-up funding never observed");

    // Chain-confirmed gate is sdk-only and never warms the wallet's local
    // balance map; refresh it before top-up consumes top_up_addr.
    s.test_wallet.sync_balances().await.expect("pre-tx sync");

    let inputs: BTreeMap<PlatformAddress, Credits> =
        std::iter::once((top_up_addr, TOP_UP_AMOUNT)).collect();
    let (_address_infos, new_balance, _) = s
        .test_wallet
        .platform_wallet()
        .identity()
        .top_up_from_addresses(&registered.id, inputs, s.test_wallet.address_signer(), None)
        .await
        .expect("top_up_from_addresses");

    // The wallet returns the post-fee balance. Cross-check against
    // an on-chain fetch so we trust both surfaces.
    //
    // The wallet credits its local view as soon as the top-up
    // state transition is broadcast and acknowledged. The
    // proof-verified `Identity::fetch` path can briefly trail that
    // — DAPI nodes apply the new block at slightly different
    // wall-clock times, and the next request may land on the
    // lagging replica (Marvin v7 QA-702: wallet 75M, fetch 50M).
    // Poll on the chain side until it agrees with the wallet
    // view, then pin the equality.
    let on_chain_post =
        wait_for_identity_balance(s.ctx.sdk(), registered.id, new_balance, STEP_TIMEOUT)
            .await
            .expect("on-chain identity balance never reached wallet-returned value");
    assert_eq!(
        on_chain_post, new_balance,
        "wallet-returned balance {new_balance} must match on-chain fetch {on_chain_post}"
    );

    let delta = on_chain_post.saturating_sub(pre_balance);
    // Top-up fee is paid from the address residual (the
    // TOP_UP_FUNDING_CREDITS - TOP_UP_AMOUNT headroom), NOT from the
    // credits committed to the identity. So the identity balance
    // delta equals TOP_UP_AMOUNT exactly.
    assert_eq!(
        delta, TOP_UP_AMOUNT,
        "balance delta {delta} should equal TOP_UP_AMOUNT {TOP_UP_AMOUNT} — \
         top-up fee comes from address residual, not the topped-up credits"
    );

    // Address residual: top_up consumed `TOP_UP_AMOUNT` AND the
    // chain-time top-up fee from `top_up_addr`. So the residual
    // ends up below the headroom (TOP_UP_FUNDING_CREDITS -
    // TOP_UP_AMOUNT).
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-top-up sync");
    let balances = s.test_wallet.balances().await;
    let top_up_residual = balances.get(&top_up_addr).copied().unwrap_or(0);
    assert!(
        top_up_residual < TOP_UP_FUNDING_CREDITS - TOP_UP_AMOUNT,
        "top-up addr residual {top_up_residual} must be less than headroom {} (chain fee should have been deducted from the residual)",
        TOP_UP_FUNDING_CREDITS - TOP_UP_AMOUNT,
    );
    tracing::info!(
        target: "platform_wallet::e2e::cases::id_002",
        identity_id = %registered.id,
        pre_balance,
        post_balance = on_chain_post,
        delta,
        top_up_residual,
        "top-up snapshot"
    );

    s.teardown().await.expect("teardown");
}
