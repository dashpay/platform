//! PA-001b — Transfer with `output_change_address: None` vs `Some(addr)`.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-001b.
//! Priority: P2.
//!
//! Drives [`PlatformAddressWallet::transfer_with_change_address`], the
//! production accessor that surfaces the implicit "where does the
//! residual go?" decision as a first-class parameter. The two
//! sub-cases pin the two override branches:
//!
//! - `None`: residual stays implicitly on the input address (the
//!   pre-existing behaviour exposed by [`PlatformAddressWallet::transfer`]).
//! - `Some(change_addr)`: every input is fully spent and `change_addr`
//!   absorbs `Σ inputs − Σ user_outputs`; the protocol's
//!   `Σ inputs == Σ outputs` invariant still holds.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;
use dpp::address_funds::PlatformAddress;
use key_wallet::managed_account::platform_address::PlatformP2PKHAddress;
use platform_wallet::wallet::platform_addresses::InputSelection;

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
async fn pa_001b_change_address_branch() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    use platform_wallet::wallet::platform_addresses::PlatformAddressWallet;

    // ---- Sub-case A: output_change_address = None -----------------
    // Residual stays implicitly on the input address — the wrapper
    // delegates straight to `transfer`, so addr_1 keeps the
    // difference.
    let s_a = setup().await.expect("e2e setup failed (sub-case A)");
    let addr_1 = s_a
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");
    s_a.ctx
        .bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address addr_1");
    wait_for_balance(&s_a.test_wallet, &addr_1, FUNDING_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_1 funding never observed");

    let addr_2 = s_a
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

    let platform_a: &PlatformAddressWallet = s_a.test_wallet.platform_wallet().platform();
    platform_a
        .transfer_with_change_address(
            default_account_index(),
            InputSelection::Explicit(inputs),
            user_outputs,
            None, // implicit-change branch
            default_fee_strategy_for_test(),
            Some(dpp::version::PlatformVersion::latest()),
            s_a.test_wallet.address_signer(),
        )
        .await
        .expect("transfer_with_change_address(None)");

    wait_for_balance(&s_a.test_wallet, &addr_2, TRANSFER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_2 transfer never observed");

    s_a.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync (None branch)");
    let bal_a = s_a.test_wallet.balances().await;
    let addr_1_post = bal_a.get(&addr_1).copied().unwrap_or(0);
    let addr_2_post = bal_a.get(&addr_2).copied().unwrap_or(0);
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
    s_a.teardown().await.expect("teardown sub-case A");

    // ---- Sub-case B: output_change_address = Some(change_addr) ----
    // Every input is fully spent; change_addr absorbs the residual.
    let s_b = setup().await.expect("e2e setup failed (sub-case B)");
    let src = s_b
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive src");
    s_b.ctx
        .bank()
        .fund_address(&src, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address src");
    wait_for_balance(&s_b.test_wallet, &src, FUNDING_FLOOR, STEP_TIMEOUT)
        .await
        .expect("src funding never observed");

    // QA-V25-003 — `next_unused_receive_address` parks on the lowest
    // unused index until something marks it used (PA-005 invariant,
    // pinned by `key_wallet::AddressPool::next_unused`). Two sequential
    // `next_unused_address()` calls without an intervening mark would
    // return the SAME index — exactly the "change_addr == receive_addr"
    // symptom Marvin v25 reported.
    //
    // QA-V27-006 — the prior fix used `next_unused_receive_addresses`
    // (the batch-fresh helper that always extends past
    // `highest_generated`) to dodge the cursor-park. But by this point
    // `src`'s funding sync has already invoked `mark_and_maintain_gap_limit`
    // and pushed the pool to `highest_used + gap_limit = 21`, leaving
    // zero headroom for a fresh-past-watermark derivation. The batch
    // call hits `GapLimitExceeded` deterministically once sync has
    // observed `src` (reliably under threads=8, racy at threads=1).
    //
    // PA-001b's contract is just "two distinct unused addresses" — it
    // does not need fresh-past-watermark semantics (those belong to
    // PA-005b). Derive `dest` from the existing 20-address gap window
    // via `next_unused_address()`, mark it used to advance the cursor,
    // then derive `change_addr` the same way. Marking `dest` used early
    // is harmless: the funds-arrival sync will mark it used anyway.
    // (DIP-17 path: `m/9'/coin'/17'/account'/key_class'/index` — there
    // is no BIP-44 change branch at this layer; the symptom is purely
    // a cursor-parking artefact, not a derivation collapse.)
    let dest = s_b
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive dest");
    let PlatformAddress::P2pkh(dest_hash) = dest else {
        panic!("platform-payment account derives P2PKH only; got {dest:?}");
    };
    {
        let wallet_id = s_b.test_wallet.platform_wallet().wallet_id();
        let mut wm = s_b
            .test_wallet
            .platform_wallet()
            .wallet_manager()
            .write()
            .await;
        let info = wm
            .get_wallet_info_mut(&wallet_id)
            .expect("test wallet present in manager");
        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index_mut(default_account_index())
            .expect("default platform-payment account present");
        let dest_p2pkh = PlatformP2PKHAddress::new(dest_hash);
        assert!(
            account.mark_platform_address_used(&dest_p2pkh),
            "mark_platform_address_used(dest) returned false: dest missing from pool"
        );
    }
    let change_addr = s_b
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive change_addr");
    assert_ne!(src, dest);
    assert_ne!(src, change_addr);
    assert_ne!(dest, change_addr);

    let user_outputs: BTreeMap<_, _> = std::iter::once((dest, TRANSFER_CREDITS)).collect();
    let inputs: BTreeMap<_, _> = std::iter::once((src, FUNDING_CREDITS)).collect();

    let platform_b: &PlatformAddressWallet = s_b.test_wallet.platform_wallet().platform();
    platform_b
        .transfer_with_change_address(
            default_account_index(),
            InputSelection::Explicit(inputs),
            user_outputs,
            Some(change_addr),
            default_fee_strategy_for_test(),
            Some(dpp::version::PlatformVersion::latest()),
            s_b.test_wallet.address_signer(),
        )
        .await
        .expect("transfer_with_change_address(Some(change_addr))");

    wait_for_balance(&s_b.test_wallet, &change_addr, TRANSFER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("change_addr never observed");

    s_b.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync (Some branch)");
    let bal_b = s_b.test_wallet.balances().await;
    let src_post = bal_b.get(&src).copied().unwrap_or(0);
    let dest_post = bal_b.get(&dest).copied().unwrap_or(0);
    let change_post = bal_b.get(&change_addr).copied().unwrap_or(0);

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

    s_b.teardown().await.expect("teardown sub-case B");
}

/// DIP-17 default platform-payment account index (`0`). Inlined so
/// the test file stays self-contained — `wallet_factory` exposes
/// `DEFAULT_ACCOUNT_INDEX_PUB` but we keep the knob explicit here so
/// drift in the framework's choice surfaces locally.
fn default_account_index() -> u32 {
    0
}

/// `[ReduceOutput(0)]` — output 0 absorbs the chain-time fee. Used by
/// every transfer in this case so the change-address branch can pin
/// fee semantics on the BTreeMap-lex-smallest output.
fn default_fee_strategy_for_test() -> dpp::address_funds::AddressFundsFeeStrategy {
    vec![dpp::address_funds::AddressFundsFeeStrategyStep::ReduceOutput(0)]
}
