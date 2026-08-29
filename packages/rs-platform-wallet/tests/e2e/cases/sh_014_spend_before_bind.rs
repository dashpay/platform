//! SH-014 — Spend before bind → `ShieldedNotBound`; spend on an unbound
//! account → `ShieldedKeyDerivation`.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-014.
//! Priority: P2.
//!
//! Both failures must fire BEFORE any proof is built.
//!
//! Expected outcome: PASS.

use platform_wallet::error::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::shielded::{bind_shielded, new_file_backed_coordinator, shielded_prover};

const UNBOUND_ACCOUNT: u32 = 7;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_014_spend_before_bind() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();

    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");
    let addr_dst_bech32m = addr_dst.to_bech32m_string(s.ctx.bank().network());

    // Step 1: spend WITHOUT binding → ShieldedNotBound.
    let coordinator = new_file_backed_coordinator(&s.test_wallet, &s.ctx.workdir)
        .await
        .expect("coordinator");
    let before_bind = s
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
        matches!(before_bind, Err(PlatformWalletError::ShieldedNotBound)),
        "spend before bind must return ShieldedNotBound; observed {before_bind:?}"
    );

    // Step 2: bind only account 0, then spend on the unbound account 7 →
    // ShieldedKeyDerivation naming account 7.
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    let unbound = s
        .test_wallet
        .platform_wallet()
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            UNBOUND_ACCOUNT,
            &addr_dst_bech32m,
            1_000_000,
            prover,
        )
        .await;
    match unbound {
        Err(PlatformWalletError::ShieldedKeyDerivation(msg)) => assert!(
            msg.contains(&UNBOUND_ACCOUNT.to_string()),
            "error must name the unbound account {UNBOUND_ACCOUNT}; observed {msg:?}"
        ),
        other => panic!("expected ShieldedKeyDerivation naming account 7; observed {other:?}"),
    }

    s.teardown().await.expect("teardown");
}
