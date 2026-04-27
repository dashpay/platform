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
//! Flow (mirrors the plan's "First Test" section, with a Wave-8
//! tweak to addr_2's derivation point — see step 3):
//!
//! 1. `framework::setup()` — bank + SDK + SPV + registry init,
//!    plus a freshly-seeded `TestWallet` registered for cleanup.
//! 2. Bank funds `addr_1` with 50_000_000 credits and we wait for
//!    the test wallet to observe the inbound balance.
//! 3. ONLY THEN derive `addr_2`. The wallet's pool cursor only
//!    advances once an address is observed used, so calling
//!    `next_unused_address` twice back-to-back before any sync
//!    would return the same address. (Discovered live in Wave 8.)
//! 4. Test wallet self-transfers 10_000_000 credits to `addr_2`.
//! 5. Assert balances and derive the fee from the balance delta
//!    `FUNDING_CREDITS - received - remaining` (the production
//!    wallet does not surface a `fee_paid` accessor — keeping the
//!    test verification on observed balances mirrors what a real
//!    consumer would do on-chain).
//! 6. `setup_guard.teardown()` sweeps remaining funds back to the
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

// `flavor = "multi_thread"` is kept as defense-in-depth and parity
// with `dash-evo-tool/tests/backend-e2e/`. With the
// `dash_async::block_on` bridge in `framework/context_provider.rs`
// the framework now works on every tokio runtime flavor, so this
// attribute is no longer load-bearing — but multi-thread still
// gives the optimal `block_in_place + spawn` bridge path.
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

    // Step 1: derive `addr_1` and have the bank fund it. We do NOT
    // pre-allocate `addr_2` here: `next_unused_receive_address`
    // advances the address pool only once an address is observed
    // as used (i.e. an inbound balance is seen during sync).
    // Calling `next_unused_address` twice back-to-back before any
    // sync would return the SAME address — the cursor hasn't moved.
    // Deriving `addr_2` after `wait_for_balance(addr_1, ...)` lets
    // the BLAST sync inside `wait_for_balance` mark `addr_1` used
    // first, so the next derivation lands on a fresh slot. This
    // also exercises the wallet's "observe inbound funds + advance
    // address pool" property as a side benefit.
    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");

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

    // Step 3: derive `addr_2` AFTER the wallet has observed
    // `addr_1`'s inbound funding — only now does the address pool
    // cursor advance to a fresh slot.
    let addr_2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_2");
    assert_ne!(
        addr_1, addr_2,
        "wallet must hand out a fresh address once addr_1 is observed used"
    );

    // Step 4: self-transfer addr_1 -> addr_2.
    let outputs: BTreeMap<_, _> = std::iter::once((addr_2, TRANSFER_CREDITS)).collect();
    s.test_wallet
        .transfer(outputs)
        .await
        .expect("self-transfer");

    wait_for_balance(&s.test_wallet, &addr_2, TRANSFER_CREDITS, STEP_TIMEOUT)
        .await
        .expect("addr_2 transfer never observed");

    // Step 5: assert final balances. Re-sync once more so the
    // cached view reflects the post-transfer state across BOTH
    // addresses (the wait above only blocked on addr_2 reaching
    // its target). Then derive the fee from the balance delta
    // (FUNDING_CREDITS - received - remaining): the production
    // wallet does not surface a `fee_paid` accessor, so reading
    // it from observed balances keeps the assertion close to what
    // a real consumer would verify on-chain.
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
    // `remaining == FUNDING_CREDITS - TRANSFER_CREDITS - fee` falls
    // out of the fee derivation by construction once the two
    // assertions above hold; explicitly stating it would be a
    // tautology, so we don't.

    // Step 6: explicit teardown. Sweeps remaining funds back to the
    // bank and removes the registry entry.
    s.teardown().await.expect("teardown");
}
