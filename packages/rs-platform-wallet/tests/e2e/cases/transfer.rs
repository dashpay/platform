//! Self-transfer of credits between two platform-payment addresses
//! owned by the same test wallet.
//!
//! Runs by default (no `#[ignore]`). Operator setup lives in
//! `tests/.env` (template: `tests/.env.example`). A missing
//! `PLATFORM_WALLET_E2E_BANK_MNEMONIC` surfaces as a
//! [`FrameworkError::Bank`](crate::framework::FrameworkError::Bank)
//! during context init; an under-funded bank wallet panics with the
//! README's "top up at <address>" pointer so operators get an
//! actionable target.
//!
//! ```bash
//! cp packages/rs-platform-wallet/tests/.env.example \
//!    packages/rs-platform-wallet/tests/.env
//! # edit tests/.env to set PLATFORM_WALLET_E2E_BANK_MNEMONIC
//! cargo test --test e2e -- --nocapture
//! ```

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;

// Sized to dodge platform #3040 — AddressFundsTransferTransition's
// `calculate_min_required_fee` returns the static
// `state_transition_min_fees` floor (~6.5M for 1in/1out) but Drive's
// chain-time fee includes storage + processing costs that scale with
// the operation set (~14.94M empirically for the same shape). With
// `[ReduceOutput(0)]`, `output[0]` absorbs the fee at chain time;
// if it's smaller than the realistic fee the broadcast fails with
// `AddressesNotEnoughFundsError`. Picking output amounts well above
// the empirical chain-time ceiling sidesteps the bug until #3040
// lands at the dpp layer.

/// Gross credits the bank submits when funding `addr_1`. The bank
/// uses `[ReduceOutput(0)]`, so addr_1 actually receives
/// `FUNDING_CREDITS − bank_fee`. Sized well above the chain-time
/// fee (~15M empirically) so addr_1 retains enough headroom to
/// fund the test's own self-transfer (see #3040 comment above).
const FUNDING_CREDITS: u64 = 100_000_000;

/// Lower bound on what addr_1 must receive after the bank's fee
/// deduction before the test proceeds. Pinned well below the raw
/// gross so the wait isn't sensitive to fee fluctuations across
/// protocol versions.
const FUNDING_FLOOR: u64 = 70_000_000;

/// Gross credits the test wallet submits in its self-transfer to
/// `addr_2`. Same `[ReduceOutput(0)]` semantics — addr_2 receives
/// `TRANSFER_CREDITS − transfer_fee`. Sized well above the
/// empirical chain-time fee (~15M) to avoid #3040.
const TRANSFER_CREDITS: u64 = 50_000_000;

/// Lower bound on what addr_2 must receive before the assertions
/// run. A non-zero floor prevents an empty observation from
/// passing the wait.
const TRANSFER_FLOOR: u64 = 1_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn transfer_between_two_platform_addresses() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // `next_unused_receive_address` advances the pool only once an
    // address is observed used; derive `addr_2` AFTER `addr_1` is
    // funded so the cursor lands on a fresh slot.
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

    // Bank uses `[ReduceOutput(0)]`, so addr_1 receives
    // `FUNDING_CREDITS − bank_fee`. Wait on the post-fee floor.
    wait_for_balance(&s.test_wallet, &addr_1, FUNDING_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_1 funding never observed");

    let addr_2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_2");
    assert_ne!(
        addr_1, addr_2,
        "wallet must hand out a fresh address once addr_1 is observed used"
    );

    let outputs: BTreeMap<_, _> = std::iter::once((addr_2, TRANSFER_CREDITS)).collect();
    s.test_wallet
        .transfer(outputs)
        .await
        .expect("self-transfer");

    // addr_2 receives `TRANSFER_CREDITS − transfer_fee` (also
    // `[ReduceOutput(0)]`). Wait on the post-fee floor.
    wait_for_balance(&s.test_wallet, &addr_2, TRANSFER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_2 transfer never observed");

    // Re-sync so the cached view reflects post-transfer state across
    // BOTH addresses, then derive bank- and transfer-fee shares from
    // observed balances.
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync");
    let balances = s.test_wallet.balances().await;
    let received = balances.get(&addr_2).copied().unwrap_or(0);
    let remaining = balances.get(&addr_1).copied().unwrap_or(0);
    let observed_total = received.saturating_add(remaining);
    // Bank's `ReduceOutput(0)` charged its fee against addr_1's
    // funding output: the wallet's total post-transfer is
    // `FUNDING_CREDITS − bank_fee − transfer_fee`. Each fee is the
    // amount each ReduceOutput step trimmed off its respective
    // output; together they equal `FUNDING_CREDITS − observed_total`.
    let total_fees = FUNDING_CREDITS.saturating_sub(observed_total);
    // The transfer fee is the share TRANSFER_CREDITS lost while
    // crossing addr_1 -> addr_2.
    let transfer_fee = TRANSFER_CREDITS.saturating_sub(received);
    let bank_fee = total_fees.saturating_sub(transfer_fee);
    tracing::info!(
        target: "platform_wallet::e2e::cases::transfer",
        ?addr_1,
        ?addr_2,
        funded = FUNDING_CREDITS,
        received,
        remaining,
        bank_fee,
        transfer_fee,
        "post-transfer balance snapshot"
    );

    assert!(
        received >= TRANSFER_FLOOR,
        "addr_2 must hold at least TRANSFER_FLOOR ({TRANSFER_FLOOR}); observed {received}"
    );
    assert!(
        received < TRANSFER_CREDITS,
        "addr_2 must hold less than TRANSFER_CREDITS ({TRANSFER_CREDITS}) \
         after `ReduceOutput(0)` fee deduction; observed {received}"
    );
    assert!(
        transfer_fee > 0,
        "self-transfer must charge a non-zero fee (received={received})"
    );
    assert!(
        transfer_fee < TRANSFER_CREDITS,
        "transfer fee implausibly high: {transfer_fee} >= TRANSFER_CREDITS ({TRANSFER_CREDITS})"
    );
    assert!(
        bank_fee > 0,
        "bank funding must charge a non-zero fee (observed_total={observed_total})"
    );

    s.teardown().await.expect("teardown");
}
