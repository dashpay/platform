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

const REGISTER_FUNDING_CREDITS: u64 = 60_000_000;
const REGISTER_FUNDING_FLOOR: u64 = 50_000_000;
const REGISTRATION_FUNDING: u64 = 50_000_000;

const TOP_UP_FUNDING_CREDITS: u64 = 30_000_000;
const TOP_UP_FUNDING_FLOOR: u64 = 25_000_000;

/// Credits the top-up commits to the identity. Below
/// `TOP_UP_FUNDING_CREDITS` so the second address keeps a non-zero
/// residual the test can assert on.
const TOP_UP_AMOUNT: Credits = 25_000_000;

const STEP_TIMEOUT: Duration = Duration::from_secs(60);

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
    let on_chain_post = Identity::fetch(s.ctx.sdk(), registered.id)
        .await
        .expect("fetch post")
        .expect("identity visible")
        .balance();
    assert_eq!(
        on_chain_post, new_balance,
        "wallet-returned balance {new_balance} must match on-chain fetch {on_chain_post}"
    );

    let delta = on_chain_post.saturating_sub(pre_balance);
    assert!(
        delta > 0,
        "top-up must raise the identity balance: pre={pre_balance} post={on_chain_post}"
    );
    assert!(
        delta < TOP_UP_AMOUNT,
        "balance delta {delta} must be strictly less than the topped-up amount {TOP_UP_AMOUNT} \
         (the difference is the on-chain top-up fee)"
    );
    let top_up_fee = TOP_UP_AMOUNT.saturating_sub(delta);
    assert!(
        top_up_fee > 0,
        "top-up fee must be non-zero (delta={delta} amount={TOP_UP_AMOUNT})"
    );

    // Address residual: top_up consumed `TOP_UP_AMOUNT` from
    // `top_up_addr`; the rest stays as residual modulo top-up fee
    // mechanics.
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-top-up sync");
    let balances = s.test_wallet.balances().await;
    let top_up_residual = balances.get(&top_up_addr).copied().unwrap_or(0);
    tracing::info!(
        target: "platform_wallet::e2e::cases::id_002",
        identity_id = %registered.id,
        pre_balance,
        post_balance = on_chain_post,
        delta,
        top_up_fee,
        top_up_residual,
        "top-up snapshot"
    );

    s.teardown().await.expect("teardown");
}
