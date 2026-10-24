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
//! Why this bypasses the production `shielded_fund_from_asset_lock`
//! orchestration: that API builds a FRESH asset lock (new outpoint) on
//! every `FromWalletBalance` call and consumes the tracked lock on
//! success, so it is structurally incapable of resubmitting one proof
//! twice. This probe targets DRIVE's independent single-use outpoint
//! check, which requires submitting a duplicate proof directly — so it
//! builds each Type-18 transition through the `test-utils` seam
//! `operations::build_shield_from_asset_lock_st` and broadcasts it straight
//! through the SDK. The POSITIVE production-path coverage lives in SH-018
//! (`shielded_fund_from_asset_lock` end-to-end).
//!
//! Core-L1 gated — a RED on the proof-build documents the gate, not a
//! defect.

#![cfg(feature = "shielded")]

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::prelude::AssetLockProof;
use dpp::shielded::builder::OrchardProver;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use platform_wallet::wallet::shielded::operations::build_shield_from_asset_lock_st;
use platform_wallet::wallet::shielded::operations::test_utils::derive_asset_lock_private_key;
use platform_wallet::wallet::shielded::{AccountViewingKeys, OrchardKeySet};
use platform_wallet::{AssetLockFundingType, PlatformWalletError};

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, shielded_prover, teardown_sweep_shielded, unspent_notes,
    wait_for_shielded_balance,
};
use crate::framework::signer::SeedBackedCoreSigner;

// The lock is funded for the FULL `ASSET_LOCK_DUFFS`, but the shield asks
// for only `SHIELD_DUFFS` (< lock). Type 18 requires the lock to hold
// `shield_amount + asset-lock processing fee`; the production fund path
// derives `shield = lock_value - min_fee` for exactly this reason. The
// 350_000-duff (3.5e8-credit) gap exceeds Type 18's ~2.13e8-credit
// asset-lock shield fee — shielding the full lock value is rejected before
// the shield commits, so the REPLAY leg would never reach Drive's
// single-use outpoint check. Core funding covers the lock plus its L1 tx
// fee (+200_000 duffs headroom; an asset-lock tx fee is a few hundred duffs).
const TEST_WALLET_CORE_FUNDING: u64 = 1_950_000;
const ASSET_LOCK_DUFFS: u64 = 1_750_000;
const SHIELD_DUFFS: u64 = 1_400_000;
const SHIELDED_ACCOUNT: u32 = 0;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// Build a Type-18 transition consuming `proof` and broadcast it through the
/// SDK, waiting for Platform's consensus verdict. Bypasses the wallet's
/// asset-lock tracking on purpose: a replay must reach Drive's own
/// single-use outpoint check.
async fn shield_with_proof(
    sdk: &Arc<dash_sdk::Sdk>,
    keys: &AccountViewingKeys,
    proof: AssetLockProof,
    one_time_key: &[u8],
    credits: u64,
    prover: &impl OrchardProver,
) -> Result<StateTransitionProofResult, PlatformWalletError> {
    let state_transition = build_shield_from_asset_lock_st(
        sdk,
        keys,
        SHIELDED_ACCOUNT,
        proof,
        one_time_key,
        credits,
        prover,
    )?;
    state_transition
        .broadcast_and_wait_for_affected_state::<StateTransitionProofResult>(sdk.as_ref(), None)
        .await
        .map_err(PlatformWalletError::Sdk)
}

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
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL set to a falsy value — abuse case opted out (no-op pass)"
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
    let handle = bind_shielded(&s.test_wallet, &[SHIELDED_ACCOUNT], &s.ctx.workdir)
        .await
        .expect("bind_shielded");

    let keys = OrchardKeySet::from_seed(&seed_bytes, network, SHIELDED_ACCOUNT)
        .expect("derive Orchard keyset")
        .viewing_keys();
    let sdk = s.test_wallet.platform_wallet().sdk_arc();

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
    // Shield strictly less than the lock so the remainder covers Type 18's
    // asset-lock processing fee. The replay reuses this same `credits`/proof.
    let credits = dpp::balances::credits::CREDITS_PER_DUFF * SHIELD_DUFFS;

    // First shield must succeed (consumes the single-use proof).
    let first = shield_with_proof(&sdk, &keys, proof.clone(), &one_time_key, credits, &prover)
        .await
        .expect("first shield-from-asset-lock must succeed");
    assert!(
        matches!(
            first,
            StateTransitionProofResult::VerifiedAssetLockConsumed(
                StoredAssetLockInfo::FullyConsumed
            ) | StateTransitionProofResult::VerifiedAssetLockConsumedWithAddressInfos(
                StoredAssetLockInfo::FullyConsumed,
                _
            )
        ),
        "first shield must fully consume the asset lock; result={first:?}"
    );

    wait_for_shielded_balance(
        &s.test_wallet,
        &handle,
        SHIELDED_ACCOUNT,
        credits,
        STEP_TIMEOUT,
    )
    .await
    .expect("first asset lock shield note");
    let before = unspent_notes(&s.test_wallet, &handle, SHIELDED_ACCOUNT)
        .await
        .expect("first notes");
    assert_eq!(before.len(), 1, "exactly one note from the first shield");

    // Replay: resubmit the SAME proof. The outpoint is already consumed,
    // so the backend MUST reject.
    let replay = shield_with_proof(&sdk, &keys, proof, &one_time_key, credits, &prover)
        .await
        .expect_err("SH-035 FINDING (CRITICAL): the SAME asset-lock proof was consumed TWICE");
    assert!(
        matches!(
            replay.consensus_error(),
            Some(ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionOutPointAlreadyConsumedError(_)
                    | BasicError::IdentityAssetLockStateTransitionReplayError(_)
            ))
        ),
        "SH-035: replay must fail with an asset-lock consumption/replay consensus error; got {replay:?}"
    );
    tracing::info!(
        target: "platform_wallet::e2e::cases::sh_035",
        "ADV-VERDICT probe=SH-035 stage=consensus result=rejected detail=\"{replay}\""
    );

    handle.sync().await;
    let after = unspent_notes(&s.test_wallet, &handle, SHIELDED_ACCOUNT)
        .await
        .expect("post-replay notes");
    assert_eq!(after.len(), 1, "replay must not create a second note");
    assert_eq!(after[0].nullifier, before[0].nullifier);
    assert_eq!(after[0].value, before[0].value);
    let bank = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(network);
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank).await;
    s.teardown().await.expect("teardown");
}
