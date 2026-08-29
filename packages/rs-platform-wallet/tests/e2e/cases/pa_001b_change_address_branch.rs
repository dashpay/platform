//! PA-001b — Transfer with `output_change_address: None` vs `Some(addr)`.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-001b.
//! Priority: P2.
//!
//! Drives [`PlatformAddressWallet::transfer_with_change_address`], the
//! production accessor that surfaces the implicit "where does the
//! residual go?" decision as a first-class parameter. Two independent
//! tests pin the two override branches:
//!
//! - `pa_001b_change_address_branch_subcase_a` (`None`): residual stays
//!   implicitly on the input address (the pre-existing behaviour exposed
//!   by [`PlatformAddressWallet::transfer`]).
//! - `pa_001b_change_address_branch_subcase_b` (`Some(change_addr)`):
//!   every input is fully spent and `change_addr` absorbs
//!   `Σ inputs − Σ user_outputs`; the protocol's `Σ inputs == Σ outputs`
//!   invariant still holds.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};
use dpp::address_funds::PlatformAddress;
use platform_wallet::wallet::platform_addresses::transfer::FeeStrategyByAddress;
use platform_wallet::wallet::platform_addresses::{InputSelection, PlatformAddressWallet};

/// Bank fund per test address. Sized well above the chain-time fee
/// ceiling so the change branch's outputs both clear the fee target.
const FUNDING_CREDITS: u64 = 100_000_000;

/// Lower bound used by `wait_for_balance` to confirm bank funding
/// landed. Bank funds with `[DeductFromInput(0)]`, so the address
/// receives `FUNDING_CREDITS` exactly.
const FUNDING_FLOOR: u64 = 70_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// Gross credits routed to the user's destination output. Sized well
/// above the empirical chain-time fee (~15M) so the destination
/// output clears the `[ReduceOutput(0)]` fee target.
const TRANSFER_CREDITS: u64 = 30_000_000;

/// Lower bound used by `wait_for_balance` post-transfer.
const TRANSFER_FLOOR: u64 = 1_000_000;

#[tokio_shared_rt::test(shared)]
async fn pa_001b_change_address_branch_subcase_a() {
    init_tracing();

    // Sub-case A: output_change_address = None.
    // Residual stays implicitly on the input address — the wrapper
    // delegates straight to `transfer`, so addr_1 keeps the difference.
    let s = setup().await.expect("e2e setup failed (sub-case A)");
    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");
    s.ctx
        .bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address addr_1");
    // Funding precondition gated on the proof-verified chain view
    // (Found-025-immune): a stale local-map 0 would hang this before
    // the transfer that consumes addr_1 as an explicit input.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");

    let addr_2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_2");

    let user_outputs: BTreeMap<_, _> = std::iter::once((addr_2, TRANSFER_CREDITS)).collect();
    // QA-V19-002: Explicit declares "consume exactly this much from addr". Σ in must
    // match Σ out (no implicit change synthesis on None branch). Declaring the full
    // FUNDING_CREDITS would force a 100M-vs-30M mismatch — declare only what ships
    // (TRANSFER_CREDITS) and the un-declared residual stays on addr_1 implicitly.
    let inputs: BTreeMap<_, _> = std::iter::once((addr_1, TRANSFER_CREDITS)).collect();

    let platform: &PlatformAddressWallet = s.test_wallet.platform_wallet().platform();
    platform
        .transfer_with_change_address(
            default_account_index(),
            InputSelection::Explicit(inputs),
            user_outputs.into_iter().collect(),
            None, // implicit-change branch
            default_fee_strategy_for_test(addr_2),
            Some(dpp::version::PlatformVersion::latest()),
            s.test_wallet.address_signer(),
        )
        .await
        .expect("transfer_with_change_address(None)");

    wait_for_balance(&s.test_wallet, &addr_2, TRANSFER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_2 transfer never observed");

    s.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync (None branch)");
    let bal = s.test_wallet.balances().await;
    let addr_1_post = bal.get(&addr_1).copied().unwrap_or(0);
    let addr_2_post = bal.get(&addr_2).copied().unwrap_or(0);
    // None branch: Explicit({addr_1: TRANSFER_CREDITS}) declares only the shipped
    // amount. addr_2 receives TRANSFER_CREDITS; addr_1 keeps the undeclared
    // FUNDING_CREDITS − TRANSFER_CREDITS residual implicitly. Pin only the
    // qualitative outcome — exact post-balance numbers depend on chain-time fees.
    assert!(
        addr_1_post + addr_2_post >= FUNDING_CREDITS - 25_000_000,
        "Σ post-balances must be ≥ funding − fee ceiling; got addr_1={addr_1_post}, \
         addr_2={addr_2_post}"
    );
    assert!(
        addr_1_post >= FUNDING_CREDITS - TRANSFER_CREDITS - 25_000_000,
        "None branch: residual must still sit on addr_1; got addr_1={addr_1_post}"
    );
    s.teardown().await.expect("teardown sub-case A");
}

#[tokio_shared_rt::test(shared)]
async fn pa_001b_change_address_branch_subcase_b() {
    init_tracing();

    // Sub-case B: output_change_address = Some(change_addr).
    // Every input is fully spent; change_addr absorbs the residual.
    let s = setup().await.expect("e2e setup failed (sub-case B)");
    let src = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive src");
    s.ctx
        .bank()
        .fund_address(&src, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address src");
    // Funding precondition gated on the proof-verified chain view
    // (Found-025-immune): a stale local-map 0 would hang this before
    // the transfer that fully spends src as an explicit input.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &src,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("src funding never observed");

    // PA-001b's contract is "two distinct unused addresses" for the
    // transfer + change pair. `next_unused_receive_address` reserves the
    // index it hands out, so two back-to-back `next_unused_address()`
    // calls yield distinct indices from the existing 20-address gap
    // window (DIP-17 path `m/9'/coin'/17'/account'/key_class'/index` —
    // no BIP-44 change branch at this layer). Fresh-past-watermark
    // semantics belong to PA-005b, not here.
    let dest = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive dest");
    let PlatformAddress::P2pkh(_) = dest else {
        panic!("platform-payment account derives P2PKH only; got {dest:?}");
    };
    let change_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive change_addr");
    assert_ne!(src, dest);
    assert_ne!(src, change_addr);
    assert_ne!(dest, change_addr);

    let user_outputs: BTreeMap<_, _> = std::iter::once((dest, TRANSFER_CREDITS)).collect();
    let inputs: BTreeMap<_, _> = std::iter::once((src, FUNDING_CREDITS)).collect();

    let platform: &PlatformAddressWallet = s.test_wallet.platform_wallet().platform();
    platform
        .transfer_with_change_address(
            default_account_index(),
            InputSelection::Explicit(inputs),
            user_outputs.into_iter().collect(),
            Some(change_addr),
            default_fee_strategy_for_test(change_addr),
            Some(dpp::version::PlatformVersion::latest()),
            s.test_wallet.address_signer(),
        )
        .await
        .expect("transfer_with_change_address(Some(change_addr))");

    wait_for_balance(&s.test_wallet, &change_addr, TRANSFER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("change_addr never observed");

    s.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync (Some branch)");
    let bal = s.test_wallet.balances().await;
    let src_post = bal.get(&src).copied().unwrap_or(0);
    let dest_post = bal.get(&dest).copied().unwrap_or(0);
    let change_post = bal.get(&change_addr).copied().unwrap_or(0);

    assert_eq!(
        src_post, 0,
        "Some(change_addr) branch: src must be fully spent; got {src_post}"
    );
    assert!(
        change_post > 0,
        "change_addr must hold the residual; got {change_post}"
    );
    assert!(
        dest_post + change_post + 25_000_000 >= FUNDING_CREDITS,
        "dest + change must roughly equal Σ inputs minus fee; got dest={dest_post}, \
         change={change_post}"
    );

    s.teardown().await.expect("teardown sub-case B");
}

/// Idempotent tracing init shared across the split sub-cases. `try_init`
/// is a no-op if another test already installed a global subscriber.
fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();
}

/// DIP-17 default platform-payment account index (`0`). Inlined so
/// the test file stays self-contained — `wallet_factory` exposes
/// `DEFAULT_ACCOUNT_INDEX_PUB` but we keep the knob explicit here so
/// drift in the framework's choice surfaces locally.
fn default_account_index() -> u32 {
    0
}

/// `FeeStrategyByAddress::reduce_output(addr)` — the named output
/// absorbs the chain-time fee. Used by every transfer in this case so
/// the change-address branch can pin fee semantics on a known output.
fn default_fee_strategy_for_test(addr: PlatformAddress) -> FeeStrategyByAddress {
    FeeStrategyByAddress::reduce_output(addr)
}
