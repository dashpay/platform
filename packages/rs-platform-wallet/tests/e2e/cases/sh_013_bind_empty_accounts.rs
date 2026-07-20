//! SH-013 — `bind_shielded` with empty accounts → typed error (no panic).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-013.
//! Priority: P2.
//!
//! `bind_shielded(seed, &[], coordinator)` must return
//! `ShieldedKeyDerivation` naming the "at least one account" requirement,
//! not panic, and leave the wallet unbound (a subsequent spend returns
//! `ShieldedNotBound`).
//!
//! Expected outcome: PASS.

use platform_wallet::error::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::shielded::{new_file_backed_coordinator, shielded_prover};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_013_bind_empty_accounts() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();

    let coordinator = new_file_backed_coordinator(&s.test_wallet, &s.ctx.workdir)
        .await
        .expect("coordinator");

    let result = s
        .test_wallet
        .platform_wallet()
        .bind_shielded(&s.test_wallet.seed_bytes(), &[], &coordinator)
        .await;
    match result {
        Err(PlatformWalletError::ShieldedKeyDerivation(msg)) => {
            assert!(
                msg.contains("at least one account"),
                "error must name the 'at least one account' requirement; observed {msg:?}"
            );
        }
        other => panic!("expected ShieldedKeyDerivation; observed {other:?}"),
    }

    // The wallet must remain unbound: a spend returns ShieldedNotBound.
    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");
    let addr_dst_bech32m = addr_dst.to_bech32m_string(s.ctx.bank().network());
    let spend = s
        .test_wallet
        .platform_wallet()
        .shielded_unshield_to(
            &coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &addr_dst_bech32m,
            1_000_000,
            prover,
        )
        .await;
    assert!(
        matches!(spend, Err(PlatformWalletError::ShieldedNotBound)),
        "spend on an unbound wallet must return ShieldedNotBound; observed {spend:?}"
    );

    s.teardown().await.expect("teardown");
}
