//! SH-035 — ADVERSARIAL: replayed Type-18 asset-lock proof — backend
//! MUST reject (single-use) [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-035. Priority: P1 (Core-L1
//! gated). CRITICAL-if-it-fails (double-shield from one L1 lock = value
//! forgery).
//!
//! Attack: shield-from-asset-lock (Type 18) with a valid proof, then
//! resubmit the SAME proof in a second Type-18 transition. An asset-lock
//! outpoint is single-use; the second consumption MUST fail.
//!
//! Uses the public `shielded_shield_from_asset_lock` wrapper (Gap-4) +
//! the one-time-key helper (Gap-5). Core-L1 gated — a RED on the
//! proof-build documents the gate, not a defect.

#![cfg(feature = "shielded")]

use std::time::Duration;

use platform_wallet::wallet::shielded::operations::test_utils::derive_asset_lock_private_key;
use platform_wallet::AssetLockFundingType;

use crate::framework::prelude::*;
use crate::framework::shielded::{adversarial_enabled, bind_shielded, shielded_prover};
use crate::framework::signer::SeedBackedCoreSigner;

const TEST_WALLET_CORE_FUNDING: u64 = 100_000;
const ASSET_LOCK_DUFFS: u64 = 50_000;
const SHIELDED_ACCOUNT: u32 = 0;
#[allow(dead_code)]
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_035_replayed_asset_lock_proof() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_035",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    // Core-L1 gate (panics RED if unavailable, documenting the gate).
    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet (Core-L1 gate)");

    let network = s.test_wallet.platform_wallet().sdk().network;
    let seed_bytes = s.test_wallet.seed_bytes();
    let prover = shielded_prover();
    let _handle = bind_shielded(&s.test_wallet, &[SHIELDED_ACCOUNT], &s.ctx.workdir)
        .await
        .expect("bind_shielded");

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
        .expect("create_funded_asset_lock_proof (Core-L1 seam — RED documents the gate)");

    let one_time_key = derive_asset_lock_private_key(&seed_bytes, network, &path)
        .expect("derive one-time asset-lock private key");
    let credits = dpp::balances::credits::CREDITS_PER_DUFF * ASSET_LOCK_DUFFS;

    // First shield must succeed (consumes the single-use proof).
    s.test_wallet
        .platform_wallet()
        .shielded_shield_from_asset_lock(
            SHIELDED_ACCOUNT,
            proof.clone(),
            &one_time_key,
            credits,
            prover,
        )
        .await
        .expect("first shield-from-asset-lock must succeed");

    // Replay: resubmit the SAME proof. The outpoint is already consumed,
    // so the backend MUST reject.
    let replay = s
        .test_wallet
        .platform_wallet()
        .shielded_shield_from_asset_lock(SHIELDED_ACCOUNT, proof, &one_time_key, credits, prover)
        .await;
    assert!(
        replay.is_err(),
        "SH-035 FINDING (CRITICAL): the SAME asset-lock proof was consumed TWICE — \
         double-shield from one L1 lock = value forgery. result={replay:?}"
    );
    tracing::info!(
        target: "platform_wallet::e2e::cases::sh_035",
        "replayed asset-lock proof correctly rejected (single-use enforced)"
    );

    s.teardown().await.expect("teardown");
}
