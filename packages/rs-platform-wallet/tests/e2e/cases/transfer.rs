// TODO(qa-wave5): live happy-path run pending operator bank pre-funding.
//   Marvin's QA pass could not execute the funded scenario because no
//   testnet bank wallet with `>= PLATFORM_WALLET_E2E_MIN_BANK_CREDITS`
//   credits is available in this environment. Once an operator
//   provisions one and exports `PLATFORM_WALLET_E2E_BANK_MNEMONIC`, run:
//     cargo test --test e2e -- --ignored --nocapture \
//       transfer_between_two_platform_addresses
//   See `tests/e2e/README.md` "Bank pre-funding" for the procedure.

//! First end-to-end test — credits transfer between two
//! platform-payment addresses owned by the same test wallet.
//!
//! Flow (mirrors the plan's "First Test" section):
//!
//! 1. `framework::setup()` — bank + SDK + SPV + registry init,
//!    plus a freshly-seeded `TestWallet` registered for cleanup.
//! 2. Bank funds `addr_1` with 50_000_000 credits.
//! 3. Test wallet self-transfers 10_000_000 credits to `addr_2`.
//! 4. Assert balances against the changeset's reported `fee_paid`
//!    (the public accessor added in Wave 1, commit `b5ed6e45d7`).
//! 5. `setup_guard.teardown()` sweeps remaining funds back to the
//!    bank and removes the registry entry.
//!
//! Marked `#[ignore]` because it requires a live testnet + a
//! pre-funded bank wallet (see `tests/e2e/README.md` for operator
//! setup). Run with:
//!
//! ```bash
//! PLATFORM_WALLET_E2E_BANK_MNEMONIC="..." \
//!   cargo test --test e2e -- --ignored --nocapture
//! ```

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;

/// Initial credits the bank funds onto `addr_1`. Large enough to
/// cover the self-transfer plus the inevitable fee, small enough
/// not to drain a modest bank.
const FUNDING_CREDITS: u64 = 50_000_000;

/// Credits self-transferred from `addr_1` to `addr_2`.
const TRANSFER_CREDITS: u64 = 10_000_000;

/// Per-step deadline for balance observations. 60s comfortably
/// covers BLAST-sync round-trip plus Drive block time on testnet.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

// `flavor = "multi_thread"` is REQUIRED — `SpvContextProvider`'s
// `block_in_place` bridge (framework/context_provider.rs) panics on a
// current-thread runtime, which is the `tokio_shared_rt::test`
// default. Mirrors `dash-evo-tool/tests/backend-e2e/` precedent.
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access"]
async fn transfer_between_two_platform_addresses() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // Step 1: derive two receive addresses on the test wallet.
    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");
    let addr_2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_2");
    assert_ne!(
        addr_1, addr_2,
        "wallet must hand out two distinct addresses"
    );

    // Step 2: bank funds addr_1 — submission only; we wait on the
    // recipient's view of the balance below.
    s.ctx
        .bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");

    wait_for_balance(&s.test_wallet, &addr_1, FUNDING_CREDITS, STEP_TIMEOUT)
        .await
        .expect("addr_1 funding never observed");

    // Step 3: self-transfer addr_1 -> addr_2.
    let outputs: BTreeMap<_, _> = std::iter::once((addr_2, TRANSFER_CREDITS)).collect();
    let cs = s
        .test_wallet
        .transfer(outputs)
        .await
        .expect("self-transfer");

    let fee = cs.fee_paid();
    assert!(fee > 0, "transfer should report a non-zero fee (got {fee})");

    wait_for_balance(&s.test_wallet, &addr_2, TRANSFER_CREDITS, STEP_TIMEOUT)
        .await
        .expect("addr_2 transfer never observed");

    // Step 4: assert final balances. Re-sync once more so the
    // cached view reflects the post-transfer state across BOTH
    // addresses (the wait above only blocked on addr_2 reaching
    // its target).
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync");
    let balances = s.test_wallet.balances().await;
    let addr_2_balance = balances.get(&addr_2).copied().unwrap_or(0);
    let addr_1_balance = balances.get(&addr_1).copied().unwrap_or(0);

    assert_eq!(
        addr_2_balance, TRANSFER_CREDITS,
        "addr_2 must hold exactly the transferred amount"
    );
    assert_eq!(
        addr_1_balance,
        FUNDING_CREDITS - TRANSFER_CREDITS - fee,
        "addr_1 must equal funded - transferred - fee (fee={fee})"
    );

    // Step 5: explicit teardown. Sweeps remaining funds back to the
    // bank and removes the registry entry.
    s.teardown().await.expect("teardown");
}
