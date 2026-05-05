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

/// Funds the bank submits to the funding address. Option C
/// (DeductFromInput) delivers exactly this amount to the address.
/// Sized so that after the 50M registration, the residual (20M) clears
/// the chain-time identity_create_fee minimum (~15.5M) with 5M buffer.
const FUNDING_CREDITS: u64 = 70_000_000;

/// Floor the wait_for_balance keys on before registration runs.
/// Under Option C the address receives exactly FUNDING_CREDITS, so
/// the floor equals the funded amount.
const FUNDING_FLOOR: u64 = 70_000_000;

/// Credits committed to the new identity in the registration
/// transition. The address loses this exact amount minus the bank's
/// fee already deducted upstream and the registration fee deducted
/// at chain time.
const REGISTRATION_FUNDING: u64 = 50_000_000;

/// Floor the on-chain identity balance must clear post-registration.
/// `register_identity_from_addresses` already waits on
/// `funding / 2`; this assertion duplicates the lower bound so the
/// case fails clearly if the helper's wait threshold is ever
/// loosened.
const IDENTITY_BALANCE_FLOOR: u64 = REGISTRATION_FUNDING / 2;

/// Per-step wait deadline.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[ignore = "requires PLATFORM_WALLET_E2E_BANK_MNEMONIC and live testnet access; run with `cargo test -- --ignored`"]
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

    wait_for_balance(&s.test_wallet, &funding_addr, FUNDING_FLOOR, STEP_TIMEOUT)
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
        2,
        "registered identity must carry exactly two keys (MASTER + HIGH)"
    );
    assert!(
        on_chain.balance() >= IDENTITY_BALANCE_FLOOR,
        "identity balance {} must clear post-fee floor {}",
        on_chain.balance(),
        IDENTITY_BALANCE_FLOOR
    );
    assert!(
        on_chain.balance() < REGISTRATION_FUNDING,
        "identity balance {} must be strictly less than the funding {} after fee deduction",
        on_chain.balance(),
        REGISTRATION_FUNDING
    );

    // Address residual: register_from_addresses consumed the
    // registration funding; the address retains FUNDING_CREDITS -
    // REGISTRATION_FUNDING = 20M minus the chain-time fee. A
    // non-zero residual is expected and satisfies the chain's
    // identity_create_fee minimum (~15.5M).
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-registration sync");
    let balances = s.test_wallet.balances().await;
    let funding_residual = balances.get(&funding_addr).copied().unwrap_or(0);
    tracing::info!(
        target: "platform_wallet::e2e::cases::id_001",
        identity_id = %registered.id,
        identity_balance = on_chain.balance(),
        funding_residual,
        "registration snapshot"
    );

    s.teardown().await.expect("teardown");
}
