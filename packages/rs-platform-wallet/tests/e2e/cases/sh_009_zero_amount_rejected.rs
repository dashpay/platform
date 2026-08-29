//! SH-009 — Zero-amount shield / transfer / unshield rejected at the
//! boundary (no proof paid).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-009.
//! Priority: P2.
//!
//! Each call with `amount == 0` must return a typed `Err` (not a panic,
//! not `Ok`) synchronously — well under one ~30 s proof. The shield
//! zero-guard is confirmed in production (`platform_wallet.rs:733`); the
//! transfer/unshield guards are unconfirmed in the audit — **if either
//! lacks a zero-guard, this case goes RED and surfaces a
//! missing-validation finding** (mirrors PA-001c's contract framing).

use std::time::{Duration, Instant};

use crate::framework::prelude::*;
use crate::framework::shielded::{bind_shielded, shielded_default_address_43, shielded_prover};

/// Generous upper bound: a synchronous boundary rejection must return far
/// below one Halo-2 proof (~30 s). A few seconds covers lock acquisition
/// and address parsing without admitting a proof build.
const REJECT_CEILING: Duration = Duration::from_secs(5);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_009_zero_amount_rejected() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();

    let handle = bind_shielded(&s.test_wallet, &[0, 1], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    let pw = s.test_wallet.platform_wallet();

    // Shield with amount == 0.
    let t0 = Instant::now();
    let shield = pw
        .shielded_shield_from_account(
            &handle.coordinator,
            0,
            0,
            0,
            s.test_wallet.address_signer(),
            prover,
        )
        .await;
    assert!(
        shield.is_err(),
        "zero-amount shield must be rejected with a typed Err; observed {shield:?}"
    );
    assert!(
        t0.elapsed() < REJECT_CEILING,
        "zero-amount shield must reject synchronously (no proof build); took {:?}",
        t0.elapsed()
    );

    // Transfer with amount == 0 to account 1's address.
    let acct1_addr = shielded_default_address_43(&s.test_wallet, 1)
        .await
        .expect("account 1 default Orchard address");
    let t1 = Instant::now();
    let transfer = pw
        .shielded_transfer_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &acct1_addr,
            0,
            [0u8; 36],
            prover,
        )
        .await;
    assert!(
        transfer.is_err(),
        "zero-amount transfer must be rejected with a typed Err (RED if no guard exists); \
         observed {transfer:?}"
    );
    assert!(
        t1.elapsed() < REJECT_CEILING,
        "zero-amount transfer must reject synchronously; took {:?}",
        t1.elapsed()
    );

    // Unshield with amount == 0 to a transparent address.
    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");
    let addr_dst_bech32m = addr_dst.to_bech32m_string(s.ctx.bank().network());
    let t2 = Instant::now();
    let unshield = pw
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &addr_dst_bech32m,
            0,
            prover,
        )
        .await;
    assert!(
        unshield.is_err(),
        "zero-amount unshield must be rejected with a typed Err (RED if no guard exists); \
         observed {unshield:?}"
    );
    assert!(
        t2.elapsed() < REJECT_CEILING,
        "zero-amount unshield must reject synchronously; took {:?}",
        t2.elapsed()
    );

    // No funds were ever shielded, so the teardown sweep is a no-op.
    s.teardown().await.expect("teardown");
}
