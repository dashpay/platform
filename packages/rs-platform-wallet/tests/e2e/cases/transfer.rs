//! Self-transfer of credits between two platform-payment addresses
//! owned by the same test wallet.
//!
//! Runs by default (no `#[ignore]`). Operator setup lives in
//! `tests/.env` (template: `tests/.env.example`); a missing
//! `PLATFORM_WALLET_E2E_BANK_MNEMONIC` panics with an actionable
//! "top up bank at <address>" message.
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

/// Initial credits the bank funds onto `addr_1`.
const FUNDING_CREDITS: u64 = 50_000_000;

/// Credits self-transferred from `addr_1` to `addr_2`.
const TRANSFER_CREDITS: u64 = 10_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
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

    wait_for_balance(&s.test_wallet, &addr_1, FUNDING_CREDITS, STEP_TIMEOUT)
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

    wait_for_balance(&s.test_wallet, &addr_2, TRANSFER_CREDITS, STEP_TIMEOUT)
        .await
        .expect("addr_2 transfer never observed");

    // Re-sync so the cached view reflects post-transfer state across
    // BOTH addresses; derive fee from the balance delta since the
    // wallet exposes no `fee_paid` accessor.
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync");
    let balances = s.test_wallet.balances().await;
    let received = balances.get(&addr_2).copied().unwrap_or(0);
    let remaining = balances.get(&addr_1).copied().unwrap_or(0);
    let fee = FUNDING_CREDITS
        .saturating_sub(received)
        .saturating_sub(remaining);
    tracing::info!(
        target: "platform_wallet::e2e::cases::transfer",
        ?addr_1,
        ?addr_2,
        funded = FUNDING_CREDITS,
        received,
        remaining,
        fee,
        "post-transfer balance snapshot"
    );

    assert_eq!(
        received, TRANSFER_CREDITS,
        "addr_2 must hold exactly the transferred amount"
    );
    assert!(
        fee > 0,
        "transfer must charge a non-zero fee (received={received}, remaining={remaining})"
    );
    assert!(
        fee < TRANSFER_CREDITS,
        "fee implausibly high: {fee} >= TRANSFER_CREDITS ({TRANSFER_CREDITS})"
    );

    s.teardown().await.expect("teardown");
}
