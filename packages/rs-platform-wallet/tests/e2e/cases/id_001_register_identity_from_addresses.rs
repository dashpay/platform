//! ID-001 — Register identity funded from platform addresses.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Identity (ID) → ID-001).
//! Pinned status: Pass.
//!
//! Exercises `IdentityWallet::register_from_addresses` end-to-end via
//! the `TestWallet::register_identity_from_addresses` helper. The
//! helper itself is also exercised by `ID-flow-001` in the entry
//! tier; this case adds a direct on-chain assertion that the
//! registered key set matches the placeholder, plus the address
//! residual / fee accounting that the entry-tier flow does not pin.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Funds the bank submits to the funding address. Sized at
/// `REGISTRATION_FUNDING + 150M`: the 150M residual covers the
/// chain-time IdentityCreateFromAddresses dynamic fee (~125.71M
/// observed) with buffer for protocol-version drift. Mirrors the
/// `setup_with_n_identities` `REGISTRATION_HEADROOM` constant.
const FUNDING_CREDITS: u64 = REGISTRATION_FUNDING + 150_000_000;

/// Floor the wait_for_balance keys on before registration runs.
/// Under Option C the address receives exactly FUNDING_CREDITS.
const FUNDING_FLOOR: u64 = FUNDING_CREDITS;

/// Credits committed to the new identity. KEPT LARGER than
/// 0.001 tDASH: must stay above `IDENTITY_SWEEP_FLOOR` (50M,
/// hardcoded in `cleanup.rs`) so the teardown sweep recovers
/// credits back to the bank identity instead of silently skipping
/// (~30M IDENTITY_SWEEP_FEE_RESERVE gets paid; the rest comes back).
/// 100M provides 50M margin above floor + the sweep fee reserve
/// (sweep transfer fee is ~6.5M per `state_transition_min_fees`).
/// Up from 50M (which was AT-floor — sweep ran but barely).
const REGISTRATION_FUNDING: u64 = 100_000_000;

/// Floor the on-chain identity balance must clear post-registration.
/// `register_identity_from_addresses` already waits on
/// `funding / 2`; this assertion duplicates the lower bound so the
/// case fails clearly if the helper's wait threshold is ever
/// loosened.
const IDENTITY_BALANCE_FLOOR: u64 = REGISTRATION_FUNDING / 2;

/// Per-step wait deadline.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn id_001_register_identity_from_addresses() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

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
        &funding_addr,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("funding never observed");

    let registered = s
        .test_wallet
        .register_identity_from_addresses(funding_addr, REGISTRATION_FUNDING, 0)
        .await
        .expect("register_identity_from_addresses");

    assert_ne!(
        registered.id.to_buffer(),
        [0u8; 32],
        "registered id must be non-default"
    );

    // Fetch the on-chain identity to pin (a) it actually exists and
    // (b) its key set matches what the helper submitted.
    let on_chain = Identity::fetch(s.ctx.sdk(), registered.id)
        .await
        .expect("Identity::fetch")
        .expect("identity must be visible on chain");

    assert_eq!(
        on_chain.id(),
        registered.id,
        "fetched identity id must match the registered id"
    );
    assert_eq!(
        on_chain.public_keys().len(),
        4,
        "registered identity must carry exactly four keys (MASTER + HIGH + TRANSFER + CRITICAL)"
    );
    assert!(
        on_chain.balance() >= IDENTITY_BALANCE_FLOOR,
        "identity balance {} must clear post-fee floor {}",
        on_chain.balance(),
        IDENTITY_BALANCE_FLOOR
    );
    // The chain-time IdentityCreateFromAddresses fee is paid from the
    // address residual (the FUNDING_CREDITS - REGISTRATION_FUNDING
    // headroom), NOT from the credits committed to the identity. So
    // the identity ends up with exactly REGISTRATION_FUNDING.
    assert_eq!(
        on_chain.balance(),
        REGISTRATION_FUNDING,
        "identity balance should equal REGISTRATION_FUNDING — fee comes from address residual, not identity balance"
    );

    // Address residual: register_from_addresses consumed
    // REGISTRATION_FUNDING from the address AND the chain-time
    // dynamic fee (~125.71M observed). After both, residual <
    // FUNDING_CREDITS - REGISTRATION_FUNDING (the headroom).
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-registration sync");
    let balances = s.test_wallet.balances().await;
    let funding_residual = balances.get(&funding_addr).copied().unwrap_or(0);
    assert!(
        funding_residual < FUNDING_CREDITS - REGISTRATION_FUNDING,
        "funding addr residual {funding_residual} must be less than headroom {} (chain fee should have been deducted from the residual)",
        FUNDING_CREDITS - REGISTRATION_FUNDING,
    );
    tracing::info!(
        target: "platform_wallet::e2e::cases::id_001",
        identity_id = %registered.id,
        identity_balance = on_chain.balance(),
        funding_residual,
        "registration snapshot"
    );

    s.teardown().await.expect("teardown");
}
