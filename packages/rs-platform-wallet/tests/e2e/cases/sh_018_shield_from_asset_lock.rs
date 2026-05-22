//! SH-018 — Shield from a Core L1 asset lock (Type 18).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-018.
//! Priority: P1. (Wave H + Core-L1 gate.) MAY run RED until the Core-L1
//! asset-lock funding plumbing is complete — that is acceptable; a RED
//! here documents the Core-L1 gate, not a defect in the shield path.
//!
//! Uses the public `PlatformWallet::shielded_shield_from_asset_lock`
//! wrapper (Gap-4) + the `test-utils` one-time-key derivation helper
//! (Gap-5) + `AssetLockManager::create_funded_asset_lock_proof`:
//!   1. Fund the test wallet's Core (L1) account.
//!   2. Build an asset-lock proof over that UTXO (shielded funding type).
//!   3. Derive the one-time private key from (seed, path).
//!   4. `shielded_shield_from_asset_lock(account, proof, key, amount)`.
//!   5. Sync + assert the shielded balance reflects the amount.
//!
//! Do NOT weaken the assertions: if the Core-L1 funding seam isn't wired,
//! the proof-build (step 2) errors and the test goes RED documenting it.

#![cfg(feature = "shielded")]

use std::time::Duration;

use platform_wallet::wallet::shielded::operations::test_utils::derive_asset_lock_private_key;
use platform_wallet::AssetLockFundingType;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
};
use crate::framework::signer::SeedBackedCoreSigner;

/// Core (Layer-1) duffs to fund the test wallet with (gated behind
/// `PLATFORM_WALLET_E2E_BANK_CORE_GATE`).
const TEST_WALLET_CORE_FUNDING: u64 = 100_000;
/// Duffs locked into the asset lock (the shielded note value, modulo the
/// duff→credit conversion the protocol applies).
const ASSET_LOCK_DUFFS: u64 = 50_000;
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
    let seed_bytes = s.test_wallet.seed_bytes();
    let prover = shielded_prover();
    let handle = bind_shielded(&s.test_wallet, &[SHIELDED_ACCOUNT], &s.ctx.workdir)
        .await
        .expect("bind_shielded");

    // Build an asset-lock proof over the funded Core UTXO, for shielded
    // funding. Returns (proof, derivation_path, outpoint).
    let core_signer = SeedBackedCoreSigner::new(seed_bytes, network);
    let (proof, path, _outpoint) = s
        .test_wallet
        .platform_wallet()
        .asset_locks()
        .create_funded_asset_lock_proof(
            ASSET_LOCK_DUFFS,
            0,
            AssetLockFundingType::AssetLockShieldedAddressTopUp,
            SHIELDED_ACCOUNT,
            &core_signer,
        )
        .await
        .expect(
            "create_funded_asset_lock_proof (Core-L1 asset-lock seam — RED here documents the \
             gate, not a shield-path defect)",
        );

    // Derive the one-time asset-lock private key from (seed, path).
    let one_time_key = derive_asset_lock_private_key(&seed_bytes, network, &path)
        .expect("derive one-time asset-lock private key");

    // Shield from the asset lock via the public wrapper (Type 18).
    let credits = dpp::balances::credits::CREDITS_PER_DUFF * ASSET_LOCK_DUFFS;
    s.test_wallet
        .platform_wallet()
        .shielded_shield_from_asset_lock(SHIELDED_ACCOUNT, proof, &one_time_key, credits, prover)
        .await
        .expect("shielded_shield_from_asset_lock");

    let shielded = wait_for_shielded_balance(
        &s.test_wallet,
        &handle,
        SHIELDED_ACCOUNT,
        credits,
        STEP_TIMEOUT,
    )
    .await
    .expect("shielded balance never reached the asset-lock amount");
    assert_eq!(
        shielded, credits,
        "shielded_balances[{SHIELDED_ACCOUNT}] must equal the asset-lock credits exactly; \
         observed {shielded}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
