//! Sweep self-test — registers a fresh identity with a known
//! balance, runs `teardown` (which invokes
//! `cleanup::sweep_identities_with_seed`), and asserts the bank
//! identity's on-chain balance increases by the swept amount minus
//! the CreditTransfer fee.
//!
//! Pinned status: Pass.
//!
//! Distinct from the ID-NNN cohort: this exercises the cleanup
//! path's identity-credit recovery, not the production-wallet
//! identity APIs.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::framework::prelude::*;
use crate::framework::wait::wait_for_identity_balance;

/// Bank-funded credits the funding address starts with. Option C
/// (DeductFromInput) delivers exactly this amount. Sized so the
/// residual after 90M registration (150M) covers the chain-time
/// IdentityCreateFromAddresses dynamic fee (~125M; grew from ~110.86M
/// after QA-800 added a 4th CRITICAL key, +~550 bytes × 27_000
/// credits/byte ≈ +14.85M) with ~25M buffer for the sweep
/// teardown's combined-address-balance requirement.
const FUNDING_CREDITS: u64 = 240_000_000;
/// Under Option C the address receives exactly FUNDING_CREDITS.
const FUNDING_FLOOR: u64 = 240_000_000;

/// Credits committed to the swept identity. Sized comfortably above
/// `IDENTITY_SWEEP_FLOOR` (50M, hardcoded in `cleanup.rs`) so the
/// sweep actually broadcasts a CreditTransfer rather than skipping
/// the identity as below-floor.
const REGISTRATION_FUNDING: u64 = 90_000_000;

/// Lower bound on the bank-identity gain we must observe within
/// the wait window. The sweep transfers `balance -
/// IDENTITY_SWEEP_FEE_RESERVE` (30M reserve) which is bounded
/// below by `pre_balance - 30M - chain_time_fee`. Sized loosely so
/// chain-fee fluctuations don't flake the test.
const SWEEP_GAIN_FLOOR: u64 = 30_000_000;

const STEP_TIMEOUT: Duration = Duration::from_secs(120);

#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with `cargo test -- --ignored`"]
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn id_sweep_recovers_identity_credits() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    let bank_identity_id = s.ctx.bank_identity().id;
    let bank_pre_balance = Identity::fetch(s.ctx.sdk(), bank_identity_id)
        .await
        .expect("fetch bank pre")
        .expect("bank identity must be visible on chain")
        .balance();

    // Register a fresh identity with comfortable headroom.
    let funding_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive funding address");
    s.ctx
        .bank()
        .fund_address(&funding_addr, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    wait_for_balance(&s.test_wallet, &funding_addr, FUNDING_FLOOR, STEP_TIMEOUT)
        .await
        .expect("funding never observed");

    let registered = s
        .test_wallet
        .register_identity_from_addresses(funding_addr, REGISTRATION_FUNDING, 0)
        .await
        .expect("register_identity_from_addresses");

    let pre_sweep_balance = Identity::fetch(s.ctx.sdk(), registered.id)
        .await
        .expect("fetch identity pre-sweep")
        .expect("registered identity visible")
        .balance();
    tracing::info!(
        target: "platform_wallet::e2e::cases::id_sweep",
        identity_id = %registered.id,
        bank_identity_id = %bank_identity_id,
        bank_pre_balance,
        pre_sweep_balance,
        "snapshot before sweep"
    );

    // Teardown invokes `cleanup::teardown_one` which calls
    // `sweep_identities_with_seed` — the production sweep path.
    s.teardown().await.expect("teardown");

    // Wait for the bank identity's on-chain balance to reflect
    // the swept credits. The exact gain depends on the
    // `IDENTITY_SWEEP_FEE_RESERVE` headroom plus the chain-time
    // CreditTransfer fee — assert the looser lower bound.
    let bank_post_balance = wait_for_identity_balance(
        E2eContext::init().await.expect("ctx").sdk(),
        bank_identity_id,
        bank_pre_balance + SWEEP_GAIN_FLOOR,
        STEP_TIMEOUT,
    )
    .await
    .expect("bank identity balance never reflected swept credits");

    let bank_gain = bank_post_balance.saturating_sub(bank_pre_balance);
    assert!(
        bank_gain >= SWEEP_GAIN_FLOOR,
        "bank gain {bank_gain} must clear SWEEP_GAIN_FLOOR {SWEEP_GAIN_FLOOR} \
         (pre={bank_pre_balance} post={bank_post_balance})"
    );
    // The bank identity is process-shared, so under parallel test
    // execution (`--test-threads>1`) other tests' `teardown_one`
    // identity sweeps land on the same bank identity inside this
    // test's window. We therefore cannot assert `bank_gain <=
    // pre_sweep_balance` — sibling sweeps inflate `bank_post_balance`
    // legitimately. The lower bound above remains the meaningful
    // contract: OUR sweep DID move credits to the bank identity.

    tracing::info!(
        target: "platform_wallet::e2e::cases::id_sweep",
        bank_pre_balance,
        bank_post_balance,
        bank_gain,
        pre_sweep_balance,
        "sweep self-test snapshot"
    );
}
