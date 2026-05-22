//! DPNS-001 — Register and resolve a `.dash` name.
//! Spec: `tests/e2e/TEST_SPEC.md` §"DPNS" → DPNS-001.
//! Priority: P0.
//!
//! Bank funds a fresh platform address, the wallet registers an
//! identity at DIP-9 slot 0 via the Wave A
//! [`TestWallet::register_identity_from_addresses`] helper, then a
//! uniquely-labelled `e2e-<8 hex>.dash` name is registered against that
//! identity through
//! [`IdentityWallet::register_name_with_external_signer`]. The
//! assertion side waits on
//! [`wait_for_dpns_name_visible`](crate::framework::wait::wait_for_dpns_name_visible)
//! so we observe end-to-end resolver propagation, not just the
//! state-transition broadcast acknowledgement.
//!
//! gated behind the `e2e` cargo feature like the rest of the live-testnet harness; pair
//! with `PLATFORM_WALLET_E2E_BANK_MNEMONIC` and run via
//! `cargo test -- --ignored`.

use std::time::Duration;

use rand::RngCore;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, wait_for_dpns_name_visible,
    CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Pre-fee credits committed to the new identity. KEPT LARGER than
/// 0.001 tDASH for two reasons: (a) DPNS name registration runs a
/// preorder + register document pair which consumes ~50M from the
/// identity balance, and (b) the post-DPNS residual must stay above
/// `IDENTITY_SWEEP_FLOOR` (50M, `cleanup.rs`) so the teardown sweep
/// recovers credits back to the bank identity instead of silently
/// skipping (Marvin v32 forensics). 150M = 50M DPNS + 100M sweep
/// margin (50M floor + sweep transfer fee ~6.5M + buffer).
const REGISTRATION_FUNDING: u64 = 150_000_000;

/// Headroom carried on the funding address residual so the chain-time
/// `IdentityCreateFromAddresses` dynamic fee (~110.86M observed on
/// testnet — `validate_fees_of_event_v0 PaidFromAddressInputs`
/// baseline plus the slot-2 TRANSFER key's storage cost) clears with
/// buffer for protocol-version drift. Mirrors the
/// `setup_with_n_identities` `REGISTRATION_HEADROOM` constant in
/// `framework/mod.rs` — the residual must absorb the dynamic fee
/// after registration consumes `REGISTRATION_FUNDING`, otherwise the
/// chain returns
/// `AddressesNotEnoughFundsError(required=110_862_220)` (QA-701-B).
const REGISTRATION_HEADROOM: u64 = 150_000_000;

/// Bank → funding-address gross. Funds the registration transition
/// (`REGISTRATION_FUNDING`) plus the dynamic-fee residual headroom
/// (`REGISTRATION_HEADROOM`). Earlier sizings (~200M) left only ~70M
/// after the registration consumed `REGISTRATION_FUNDING`, which fell
/// short of the ~110.86M dynamic fee — DPNS-001 then panicked with
/// "Insufficient combined address balances: total available is less
/// than required 110862220". Reuses the same arithmetic as
/// `setup_with_n_identities`'s funding policy.
const FUNDING_CREDITS: u64 = REGISTRATION_FUNDING + REGISTRATION_HEADROOM;

/// Floor `wait_for_balance` keys on before registration runs. Under
/// Option C (DeductFromInput) the address receives exactly
/// `FUNDING_CREDITS`, so the floor equals the funded amount.
const FUNDING_FLOOR: u64 = FUNDING_CREDITS;

/// Per-step deadline: bank funding observation, identity visibility,
/// DPNS resolver visibility.
const STEP_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn dpns_001_register_and_resolve_name() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // 1. Bank-fund a fresh address.
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

    // 2. Register identity at DIP-9 slot 0 (Wave A helper does the
    //    placeholder identity + key derivation + on-chain wait).
    let identity = s
        .test_wallet
        .register_identity_from_addresses(funding_addr, REGISTRATION_FUNDING, 0)
        .await
        .expect("register_identity_from_addresses");

    // 3. Generate a unique `.dash` label per run. 8 hex chars =
    //    32 bits of entropy — collision-safe for a CI worker fleet
    //    while keeping the label well inside DPNS's 3..=63 char
    //    range.
    let label = format!("e2e-{}", random_hex_label(8));
    let name = format!("{label}.dash");

    // 4. Register the DPNS name. The wallet's external-signer path
    //    looks the identity up by id and signs the document state
    //    transition with the supplied identity signer (Wave A's
    //    `SeedBackedIdentitySigner`, pre-derived for slot 0). The
    //    label parameter is the prefix only — DPP appends ".dash" and
    //    returns the full domain name.
    let full_name = s
        .test_wallet
        .platform_wallet()
        .identity()
        .register_name_with_external_signer(&identity.id, &label, identity.signer.as_ref())
        .await
        .expect("register_name_with_external_signer");
    assert_eq!(
        full_name, name,
        "register_name_with_external_signer must return the full domain name"
    );

    // 5. Wait for resolver visibility — observable propagation, not
    //    just the broadcast ack.
    let resolved = wait_for_dpns_name_visible(s.ctx.sdk(), &name, STEP_TIMEOUT)
        .await
        .expect("DPNS name never resolved");

    // 6. Resolution must return the registering identity.
    assert_eq!(
        resolved, identity.id,
        "DPNS resolver must return the registering identity's id"
    );

    s.teardown().await.expect("teardown");
}

/// Generate a lower-case hex string of length `n` from
/// [`rand::rngs::OsRng`]. Used to pin a unique DPNS label per run so
/// concurrent CI workers don't contest each other and a re-run never
/// collides with a prior round's already-registered name.
fn random_hex_label(n: usize) -> String {
    let mut bytes = vec![0u8; n.div_ceil(2)];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut s = hex::encode(&bytes);
    s.truncate(n);
    s
}
