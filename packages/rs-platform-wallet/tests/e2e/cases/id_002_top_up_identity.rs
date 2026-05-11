//! ID-002 — Top-up identity from platform addresses.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Identity (ID) → ID-002).
//! Pinned status: Pass.
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
use crate::framework::wait::wait_for_identity_balance;

// REGISTER_FUNDING_CREDITS: REGISTRATION_FUNDING + 150M headroom for
// the chain-time IdentityCreateFromAddresses dynamic fee (~125M).
// Identity is committed exactly REGISTRATION_FUNDING (0.001 tDASH).
const REGISTRATION_FUNDING: u64 = 100_000;
const REGISTER_FUNDING_CREDITS: u64 = REGISTRATION_FUNDING + 150_000_000;
const REGISTER_FUNDING_FLOOR: u64 = REGISTER_FUNDING_CREDITS;

// TOP_UP_FUNDING_CREDITS: TOP_UP_AMOUNT + 15M headroom — the
// chain-time IdentityTopUp dynamic fee is ~13M and is paid from the
// address residual, NOT from the topped-up credits. Cannot drop
// below ~15M total or the chain rejects with insufficient-address-
// balance.
const TOP_UP_AMOUNT: Credits = 100_000;
const TOP_UP_FUNDING_CREDITS: u64 = TOP_UP_AMOUNT + 15_000_000;
const TOP_UP_FUNDING_FLOOR: u64 = TOP_UP_FUNDING_CREDITS;

// 60 s is too tight under `--test-threads=14` when ID-002 funds
// 45 000 000 duff on the top-up address while sibling cases broadcast
// concurrently — the funding broadcast lands but `wait_for_balance`'s
// chain-confirmed gate doesn't clear inside the default deadline.
// 120 s is plenty without softening the framework-wide default.
const STEP_TIMEOUT: Duration = Duration::from_secs(120);

#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with `cargo test -- --ignored`"]
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
    wait_for_balance(
        &s.test_wallet,
        &register_addr,
        REGISTER_FUNDING_FLOOR,
        STEP_TIMEOUT,
    )
    .await
    .expect("register funding never observed");

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
    assert_ne!(
        top_up_addr, register_addr,
        "top-up address must differ from the registration funding address"
    );
    s.ctx
        .bank()
        .fund_address(&top_up_addr, TOP_UP_FUNDING_CREDITS)
        .await
        .expect("bank.fund_address(top-up)");
    wait_for_balance(
        &s.test_wallet,
        &top_up_addr,
        TOP_UP_FUNDING_FLOOR,
        STEP_TIMEOUT,
    )
    .await
    .expect("top-up funding never observed");

    let inputs: BTreeMap<PlatformAddress, Credits> =
        std::iter::once((top_up_addr, TOP_UP_AMOUNT)).collect();
    let new_balance = s
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
