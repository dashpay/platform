//! SH-007 — A pre-bind note is witnessable/spendable (Found-029
//! regression guard, #3603 FIXED).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-007.
//! Priority: P1. **GREEN regression guard** (NOT red-by-design).
//!
//! Before #3603 the coordinator marked only positions a currently-
//! registered IVK decrypted, so a note for wallet B landing while B was
//! unbound had its auth path discarded — B's later bind discovered the
//! balance but the position was unwitnessable. #3603's `sync.rs` rewrite
//! marks EVERY commitment position so the shared tree is witness-complete
//! regardless of bind ordering. This case guards that fix: a regression
//! to mark-only-owned flips the spend to `ShieldedMerkleWitnessUnavailable`
//! and the test goes RED.
//!
//! Coupling: the spend leg MUST use the FileBacked store (Found-027 is
//! independent of #3603 and would mask this guard with a false RED). The
//! harness `bind_shielded` always uses FileBacked.

use std::sync::Arc;
use std::time::Duration;

use platform_wallet::wallet::shielded::OrchardKeySet;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    new_file_backed_coordinator, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance, ShieldedHandle,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// A's funding clears `SHIELD_AMOUNT + 1e9 reserve + ~1.63e8 shield fee`
// (see `sh_010`), with 1e8 headroom.
const FUNDING_CREDITS: u64 = 2_382_851_200;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
// B spends this pre-bind note via an unshield, so it must exceed
// `B_UNSHIELD + the ~1.63e8 unshield fee`; 2e8 clears it with headroom.
const NOTE_TO_B: u64 = 200_000_000;
const B_UNSHIELD: u64 = 8_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_007_pre_bind_note_witnessable() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Two wallets sharing ONE FileBacked coordinator: A is the sync
    // driver, B receives a note before binding.
    let a = setup().await.expect("setup wallet A");
    let b = setup().await.expect("setup wallet B");
    let prover = shielded_prover();

    // Single shared coordinator (built off A's manager/SDK).
    let coordinator = new_file_backed_coordinator(&a.test_wallet, &a.ctx.workdir)
        .await
        .expect("shared coordinator");

    // Bind A on the shared coordinator.
    a.test_wallet
        .platform_wallet()
        .bind_shielded(&a.test_wallet.seed_bytes(), &[0], &coordinator)
        .await
        .expect("bind A");
    let a_handle = ShieldedHandle {
        coordinator: Arc::clone(&coordinator),
        accounts: vec![0],
    };

    // Fund + shield into A so A has a spendable note to pay B with.
    let addr_1 = a
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");
    a.ctx
        .bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    wait_for_address_balance_chain_confirmed_n(
        a.ctx.sdk(),
        &addr_1,
        FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");
    a.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync");
    a.test_wallet
        .platform_wallet()
        .shielded_shield_from_account(
            &coordinator,
            0,
            0,
            SHIELD_AMOUNT,
            a.test_wallet.address_signer(),
            prover,
        )
        .await
        .expect("A shield_from_account");
    wait_for_shielded_balance(&a.test_wallet, &a_handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("A shielded balance never reached SHIELD_AMOUNT");

    // Derive B's default Orchard address WITHOUT binding B (so its note
    // lands while B is unbound — the pre-bind condition #3603 fixes).
    let b_keyset = OrchardKeySet::from_seed(
        &b.test_wallet.seed_bytes(),
        b.test_wallet.platform_wallet().sdk().network,
        0,
    )
    .expect("derive B OrchardKeySet");
    let b_addr_43 = b_keyset.default_address.to_raw_address_bytes();

    // A pays a private note to B while B is UNBOUND, then A drives a sync
    // (still B-unbound) so B's position is appended under the
    // mark-every-position policy.
    a.test_wallet
        .platform_wallet()
        .shielded_transfer_to(
            &coordinator,
            &a.test_wallet.seed_bytes(),
            0,
            &b_addr_43,
            NOTE_TO_B,
            [0u8; 36],
            prover,
        )
        .await
        .expect("A → B private transfer");
    let _ = coordinator.sync(true).await;

    // NOW bind B on the same coordinator and sync.
    b.test_wallet
        .platform_wallet()
        .bind_shielded(&b.test_wallet.seed_bytes(), &[0], &coordinator)
        .await
        .expect("bind B");
    let b_handle = ShieldedHandle {
        coordinator: Arc::clone(&coordinator),
        accounts: vec![0],
    };

    // B's balance is discoverable.
    let b_bal = wait_for_shielded_balance(&b.test_wallet, &b_handle, 0, NOTE_TO_B, STEP_TIMEOUT)
        .await
        .expect("B never discovered its pre-bind note");
    assert_eq!(
        b_bal, NOTE_TO_B,
        "B's pre-bind note balance must equal the note value; observed {b_bal}"
    );

    // GREEN guard: the pre-bind note IS witnessable, so B can spend it. A
    // regression to mark-only-owned flips this to
    // ShieldedMerkleWitnessUnavailable and the test goes RED.
    let b_dst = b
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive B dst");
    let b_dst_bech32m = b_dst.to_bech32m_string(b.ctx.bank().network());
    b.test_wallet
        .platform_wallet()
        .shielded_unshield_to(
            &coordinator,
            &b.test_wallet.seed_bytes(),
            0,
            &b_dst_bech32m,
            B_UNSHIELD,
            prover,
        )
        .await
        .unwrap_or_else(|e| {
            panic!(
                "SH-007: B's pre-bind note unshield failed: {e}. If this is a \
                 ShieldedMerkleWitnessUnavailable / anchor error, the \
                 mark-every-position witness policy (#3603, Found-029) regressed."
            )
        });
    wait_for_address_balance_chain_confirmed_n(
        b.ctx.sdk(),
        &b_dst,
        B_UNSHIELD,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("B unshield destination never observed");

    let bank_addr = a
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(a.ctx.bank().network());
    teardown_sweep_shielded(&b.test_wallet, &b_handle, &bank_addr).await;
    teardown_sweep_shielded(&a.test_wallet, &a_handle, &bank_addr).await;
    b.teardown().await.expect("teardown B");
    a.teardown().await.expect("teardown A");
}
