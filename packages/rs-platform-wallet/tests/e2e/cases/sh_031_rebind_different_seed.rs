//! SH-031 — ADVERSARIAL: reject rebinding a different seed and preserve
//! the existing account's binding, notes, balance, and persisted viewing key.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-031. Priority: P1. HIGH-if-fails.
//!
//! Attack: `bind_shielded(seed_A, &[0])`, shield + sync some notes, then
//! `bind_shielded(seed_B, &[0])` with a DIFFERENT seed on the same
//! wallet/coordinator.
//!
//! Correct behavior: return `ShieldedKeyDerivation` before changing any
//! seed-A state. The guard from #4237 is intentional: durable state is
//! keyed by wallet/account, so replacing its viewing key would misattribute
//! existing notes and hide the new key's history.
//!
//! Achievable through the public API (`bind_shielded` twice) — no
//! production-seam change needed.

#![cfg(feature = "shielded")]

use std::collections::BTreeMap;
use std::time::Duration;

use platform_wallet::wallet::shielded::{OrchardKeySet, ShieldedNote, SubwalletId};
use platform_wallet::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, shielded_prover, teardown_sweep_shielded, unspent_notes,
    wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// Clears `SHIELD_AMOUNT + 1e9 reserve + ~1.63e8 shield fee` (see `sh_010`),
// with 1e8 headroom.
const FUNDING_CREDITS: u64 = 1_312_851_200;
const SHIELD_AMOUNT: u64 = 50_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_031_reject_rebind_different_seed_preserves_state() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_031",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL set to a falsy value — abuse case opted out (no-op pass)"
        );
        return;
    }

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();
    let pw = s.test_wallet.platform_wallet();

    // Bind with seed_A (the wallet's real seed) and shield a note.
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind seed_A");
    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");
    s.ctx
        .bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync");
    pw.shielded_shield_from_account(
        &handle.coordinator,
        0,
        0,
        SHIELD_AMOUNT,
        s.test_wallet.address_signer(),
        prover,
    )
    .await
    .expect("shield under seed_A");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("seed_A note never synced");

    let id = SubwalletId::new(pw.wallet_id(), 0);
    let address_before = pw.shielded_default_address(0).await.expect("seed_A bound");
    let balances_before = handle
        .balances(&s.test_wallet)
        .await
        .expect("seed_A balances");
    assert_eq!(balances_before.get(&0), Some(&SHIELD_AMOUNT));
    let note_snapshot = |notes: Vec<ShieldedNote>| {
        notes
            .into_iter()
            .map(|note| {
                (
                    note.position,
                    (
                        note.cmx,
                        note.nullifier,
                        note.block_height,
                        note.is_spent,
                        note.value,
                        note.note_data,
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
    let notes_before = note_snapshot(
        unspent_notes(&s.test_wallet, &handle, 0)
            .await
            .expect("seed_A notes"),
    );
    assert!(
        !notes_before.is_empty(),
        "seed_A must have a note to preserve"
    );
    pw.flush_persist().expect("flush seed_A state");
    let key_before = pw
        .load_persisted()
        .expect("load seed_A state")
        .shielded
        .viewing_keys
        .get(&id)
        .cloned()
        .expect("persisted seed_A viewing key");
    let expected_key = OrchardKeySet::from_seed(&s.test_wallet.seed_bytes(), pw.sdk().network, 0)
        .expect("derive seed_A viewing key")
        .viewing_keys()
        .to_fvk_bytes()
        .to_vec();
    assert_eq!(key_before, expected_key);
    assert!(handle
        .coordinator
        .registered_subwallets()
        .await
        .contains(&id));

    // Refuse a different seed before replacing any part of seed_A's state.
    let (seed_b, _hex) = crate::framework::wallet_factory::fresh_seed();
    let error = pw
        .bind_shielded(&seed_b, &[0], &handle.coordinator)
        .await
        .expect_err("rebinding seed_B over seed_A must fail");
    assert!(
        matches!(error, PlatformWalletError::ShieldedKeyDerivation(_)),
        "expected ShieldedKeyDerivation for a conflicting viewing key; got {error:?}"
    );
    assert!(pw.is_shielded_bound().await, "seed_A must remain bound");
    assert_eq!(pw.shielded_default_address(0).await, Some(address_before));
    assert!(handle
        .coordinator
        .registered_subwallets()
        .await
        .contains(&id));
    assert_eq!(
        handle
            .balances(&s.test_wallet)
            .await
            .expect("balances after rejection"),
        balances_before,
        "rejected rebind must preserve seed_A's balance"
    );
    assert_eq!(
        note_snapshot(
            unspent_notes(&s.test_wallet, &handle, 0)
                .await
                .expect("notes after rejection")
        ),
        notes_before,
        "rejected rebind must preserve seed_A's notes"
    );
    pw.flush_persist().expect("flush after rejected rebind");
    assert_eq!(
        pw.load_persisted()
            .expect("load after rejection")
            .shielded
            .viewing_keys
            .get(&id),
        Some(&key_before),
        "rejected rebind must preserve seed_A's persisted viewing key"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
