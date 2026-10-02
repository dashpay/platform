//! SH-018 — Shield from a Core L1 asset lock (Type 18).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-018.
//! Priority: P1. (Wave H + Core-L1 gate.) MAY run RED until the Core-L1
//! asset-lock funding plumbing is complete — that is acceptable; a RED
//! here documents the Core-L1 gate, not a defect in the shield path.
//!
//! Exercises the SAME path production funds shielded asset-locks through:
//! `PlatformWallet::shielded_fund_from_asset_lock` with
//! `AssetLockFunding::FromWalletBalance` (the FFI
//! `platform_wallet_manager_shielded_fund_from_asset_lock` and the
//! seed-pool batch funder both drive this exact API). The orchestrator
//! builds the asset lock from the wallet's Core balance, resolves the
//! proof with IS→CL fallback, derives `shield_amount = lock_value −
//! pool_fee` itself, submits the Type-18 transition, and consumes the
//! tracked lock:
//!   1. Fund the test wallet's Core (L1) account.
//!   2. `shielded_fund_from_asset_lock(FromWalletBalance { amount_duffs })`.
//!   3. Sync + assert the shielded balance reflects the credited value.
//!
//! Because production self-derives the shielded amount from the on-chain
//! lock value, the assertion is a fee-tolerant range (`lock − fee` lands
//! between half the lock and the full lock), mirroring CR-003 / ID-002b.
//!
//! Do NOT weaken the assertions: if the Core-L1 funding seam isn't wired,
//! the orchestrated fund (step 2) errors and the test goes RED.

#![cfg(feature = "shielded")]

use std::time::Duration;

use dpp::address_funds::OrchardAddress;
use dpp::balances::credits::CREDITS_PER_DUFF;
use platform_wallet::AssetLockFunding;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_default_address_43, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance,
};
use crate::framework::signer::SeedBackedCoreSigner;

/// Core (Layer-1) duffs to fund the test wallet with (gated behind
/// `PLATFORM_WALLET_E2E_BANK_CORE_GATE`). Must cover the asset lock plus
/// its L1 tx fee.
const TEST_WALLET_CORE_FUNDING: u64 = 1_750_000;
/// Duffs locked into the asset lock. The orchestrator shields
/// `lock_value − pool_fee`; the remainder covers Type 18's ~2.13e8-credit
/// asset-lock processing fee.
const ASSET_LOCK_DUFFS: u64 = 1_500_000;
/// BIP-44 standard account the orchestrator draws the lock's duffs from
/// (account 0 is the SPV-visible funded account).
const FUNDING_ACCOUNT_INDEX: u32 = 0;
const SHIELDED_ACCOUNT: u32 = 0;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_018_shield_from_asset_lock() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Core-L1 gate: panics (RED) if SPV / Core funding isn't available,
    // documenting the gate. Mirrors CR-003.
    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet (Core-L1 gate)");

    let network = s.test_wallet.platform_wallet().sdk().network;
    let prover = shielded_prover();
    let handle = bind_shielded(&s.test_wallet, &[SHIELDED_ACCOUNT], &s.ctx.workdir)
        .await
        .expect("bind_shielded");

    // The note must land on this wallet's own bound Orchard account so the
    // shielded sync can recover it — same self-fund shape the FFI uses.
    let recipient_raw = shielded_default_address_43(&s.test_wallet, SHIELDED_ACCOUNT)
        .await
        .expect("shielded default address for bound account");
    let recipient = OrchardAddress::from_raw_bytes(&recipient_raw)
        .expect("valid Orchard recipient address from bound account");

    // Drive the production orchestrated path: build the asset lock from the
    // wallet's Core balance, resolve via IS→CL fallback, self-derive the
    // shield amount, submit Type 18, and consume the tracked lock. `None`
    // credits => the recipient receives `lock_value − pool_fee`.
    let core_signer = SeedBackedCoreSigner::new(s.test_wallet.seed_bytes(), network);
    s.test_wallet
        .platform_wallet()
        .shielded_fund_from_asset_lock(
            &handle.coordinator,
            AssetLockFunding::FromWalletBalance {
                amount_duffs: ASSET_LOCK_DUFFS,
                account_index: FUNDING_ACCOUNT_INDEX,
            },
            vec![(recipient, None)],
            &core_signer,
            prover,
            None,
            // Single real note, no anonymity-set fillers.
            0,
            None,
            None,
        )
        .await
        .expect("shielded_fund_from_asset_lock (Core-L1 asset-lock seam — RED here documents the gate, not a shield-path defect)");

    // Production self-derives `shield_amount = lock_value − pool_fee`, so
    // accept a fee-tolerant range: at least half the lock (post-fee floor,
    // mirrors CR-003 / ID-002b) and at most the full lock value (the fee is
    // subtracted, never added).
    let lock_credits = ASSET_LOCK_DUFFS.saturating_mul(CREDITS_PER_DUFF);
    let expected_min = lock_credits / 2;
    let shielded = wait_for_shielded_balance(
        &s.test_wallet,
        &handle,
        SHIELDED_ACCOUNT,
        expected_min,
        STEP_TIMEOUT,
    )
    .await
    .expect("shielded balance never reached the post-fee floor");
    assert!(
        shielded >= expected_min && shielded <= lock_credits,
        "shielded_balances[{SHIELDED_ACCOUNT}] = {shielded} must land in the post-fee range \
         [{expected_min}, {lock_credits}] (production shields lock_value − pool_fee)"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
