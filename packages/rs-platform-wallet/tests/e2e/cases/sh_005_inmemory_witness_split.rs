//! SH-005 — Spend against the in-memory store fails witness-unavailable;
//! the file-backed store succeeds (Found-027 pin).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-005.
//! Priority: P1. **RED-by-design** until Found-027 is fixed.
//!
//! `InMemoryShieldedStore::witness()` unconditionally returns `Err`, so
//! every spend (unshield/transfer/withdraw) is structurally
//! non-functional against it, while `FileBackedShieldedStore::witness()`
//! works — a silent backing-store-dependent capability split with no
//! type-level signal. Both implement the same `ShieldedStore` trait.
//!
//! This test seeds the SAME funded note into both stores and builds
//! identical unshields:
//!   * InMemory arm asserts `ShieldedMerkleWitnessUnavailable` (exact
//!     variant) — this documents the split.
//!   * FileBacked arm asserts `Ok(())`.
//!
//! The InMemory arm flips to a regression guard once Found-027 is
//! addressed (witness gains a real impl, or the type system forbids
//! spending against a store that cannot witness).

use std::time::Duration;

use platform_wallet::error::PlatformWalletError;
use platform_wallet::wallet::shielded::{operations, OrchardKeySet, SubwalletId};

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, in_memory_store, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 2_220_000_000;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_005_inmemory_witness_split() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();

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

    // FileBacked coordinator: shield + sync so the note is in the
    // commitment tree and witnessable.
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    s.test_wallet
        .platform_wallet()
        .shielded_shield_from_account(
            &handle.coordinator,
            0,
            0,
            SHIELD_AMOUNT,
            s.test_wallet.address_signer(),
            prover,
        )
        .await
        .expect("shield_from_account");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("shielded balance never reached SHIELD_AMOUNT");

    let pw = s.test_wallet.platform_wallet();
    let wallet_id = pw.wallet_id();
    let id = SubwalletId::new(wallet_id, 0);
    let keyset = OrchardKeySet::from_seed(&s.test_wallet.seed_bytes(), pw.sdk().network, 0)
        .expect("derive OrchardKeySet for account 0");

    // Copy the synced note out of the FileBacked store into a fresh
    // InMemory store, so note SELECTION succeeds on both — the only
    // difference is whether `witness()` can produce an auth path.
    let synced_notes = {
        use platform_wallet::wallet::shielded::ShieldedStore;
        let store = handle.coordinator.store().read().await;
        store
            .get_unspent_notes(id)
            .expect("get_unspent_notes from FileBacked store")
    };
    assert!(
        !synced_notes.is_empty(),
        "FileBacked store must hold the synced note before the split test"
    );

    let inmem = in_memory_store();
    {
        use platform_wallet::wallet::shielded::ShieldedStore;
        let mut store = inmem.write().await;
        for note in &synced_notes {
            store
                .save_note(id, note)
                .expect("seed InMemory store with note");
            store
                .append_commitment(&note.cmx, true)
                .expect("append commitment to InMemory store");
        }
    }

    // Destination address for both arms.
    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");

    // InMemory arm: note selection succeeds, but `witness()` is a hard
    // Err → mapped to `ShieldedMerkleWitnessUnavailable`. This is the
    // Found-027 pin.
    let inmem_result = operations::unshield(
        &pw.sdk_arc(),
        &inmem,
        None,
        wallet_id,
        &keyset,
        0,
        &addr_dst,
        UNSHIELD_AMOUNT,
        &prover,
    )
    .await;
    assert!(
        matches!(
            inmem_result,
            Err(PlatformWalletError::ShieldedMerkleWitnessUnavailable(_))
        ),
        "InMemory spend must fail with ShieldedMerkleWitnessUnavailable (Found-027); \
         observed {inmem_result:?}"
    );

    // FileBacked arm: the same unshield succeeds and the destination
    // balance arrives.
    let addr_dst_bech32m = addr_dst.to_bech32m_string(s.ctx.bank().network());
    s.test_wallet
        .platform_wallet()
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &addr_dst_bech32m,
            UNSHIELD_AMOUNT,
            prover,
        )
        .await
        .expect("FileBacked unshield must succeed");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_dst,
        UNSHIELD_AMOUNT,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("FileBacked unshield destination never observed");

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
