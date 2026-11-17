//! SH-027 — ADVERSARIAL: malformed note serde (note_data ≠ 115 bytes,
//! corrupted cmx/nullifier) — error SAFELY, no panic.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-027. Priority: P1. HIGH-if-fails
//! (panic = host DoS; silent corruption = fund loss).
//!
//! Attack: seed the store with a `ShieldedNote` whose `note_data` is
//! truncated (114 B), oversized (116 B), empty, and bit-corrupted, then
//! drive the spend path that calls `extract_spends_and_anchor` →
//! `deserialize_note` (strict `SERIALIZED_NOTE_LEN = 115`).
//!
//! Correct behavior: every malformed length returns a typed
//! `ShieldedBuildError` (`deserialize_note` returns `None`) — NEVER a
//! panic, NEVER a silently-truncated note in a built bundle.
//!
//! This case is ACHIEVABLE without a production-seam change: the
//! `ShieldedStore` trait (`save_note` + `append_commitment`) is public,
//! so `seed_malformed_note` injects the bad note and `operations::unshield`
//! drives the deserialize path against an in-memory store.

#![cfg(feature = "shielded")]

use std::time::Duration;

use dpp::shielded::compute_shielded_unshield_fee;
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::state_transition::StateTransition;
use platform_wallet::error::PlatformWalletError;
use platform_wallet::wallet::shielded::{operations, OrchardKeySet, SubwalletId};

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, bind_shielded, in_memory_store, seed_malformed_note, shielded_prover,
    teardown_sweep_shielded, unspent_notes, wait_for_shielded_balance, NoProofProver,
};

use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// All-ones payloads: wrong lengths plus invalid field encodings at 115 bytes.
const PAYLOAD_LENGTHS: &[usize] = &[0, 1, 114, 115, 116, 200];

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_027_malformed_note_serde() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_027",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL set to a falsy value — abuse case opted out (no-op pass)"
        );
        return;
    }

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();
    let pw = s.test_wallet.platform_wallet();
    let wallet_id = pw.wallet_id();
    let network = pw.sdk().network;
    let keyset =
        OrchardKeySet::from_seed(&s.test_wallet.seed_bytes(), network, 0).expect("derive keyset");
    let id = SubwalletId::new(wallet_id, 0);

    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");

    for &len in PAYLOAD_LENGTHS {
        // Fresh store per length so a prior malformed note can't mask the
        // next. The note value is large so note-selection picks it and
        // the deserialize path is reached.
        let store = in_memory_store();
        seed_malformed_note(
            &store,
            id,
            1_120_000_000,
            vec![0xFFu8; len],
            [0x11; 32],
            [0x22; 32],
        )
        .await
        .expect("seed malformed note");

        // 115 all-ones bytes also has an invalid recipient/rho; all probes
        // must reach deserialization, before the in-memory witness limitation.
        let result = operations::unshield(
            &pw.sdk_arc(),
            &store,
            None,
            wallet_id,
            &keyset,
            0,
            &addr_dst,
            10_000_000,
            &prover,
        )
        .await;

        assert!(
            matches!(&result, Err(PlatformWalletError::ShieldedBuildError(message))
            if message == "Failed to deserialize note at position 0"),
            "malformed {len}-byte note must reach deserialization: {result:?}"
        );
    }

    // Use a genuine serialized note for content/metadata probes so a malformed
    // recipient cannot hide the field being tested.
    const NOTE_VALUE: u64 = 1_120_000_000;
    const FUNDING: u64 = 2_400_000_000;
    let timeout = Duration::from_secs(60);
    s.ctx
        .bank()
        .fund_address(&addr_dst, FUNDING)
        .await
        .expect("fund content probes");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_dst,
        FUNDING,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        timeout,
    )
    .await
    .expect("funding observed");
    s.test_wallet.sync_balances().await.expect("funding sync");
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind");
    pw.shielded_shield_from_account(
        &handle.coordinator,
        0,
        0,
        NOTE_VALUE,
        s.test_wallet.address_signer(),
        prover,
    )
    .await
    .expect("shield real note");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, NOTE_VALUE, timeout)
        .await
        .expect("real note observed");
    let notes = unspent_notes(&s.test_wallet, &handle, 0)
        .await
        .expect("real notes");
    assert_eq!(notes.len(), 1);
    let fee = compute_shielded_unshield_fee(2, s.ctx.sdk().version()).expect("fee");
    for corrupt_cmx in [false, true] {
        let mut note = notes[0].clone();
        let expected = if corrupt_cmx {
            note.cmx = [0xFF; 32];
            format!("invalid stored cmx for note at position {}", note.position)
        } else {
            // A single high-bit flip makes rho non-canonical without changing length.
            note.note_data[82] ^= 0x80;
            format!("Failed to deserialize note at position {}", note.position)
        };
        let result = operations::build_unshield_st(
            &pw.sdk_arc(),
            handle.coordinator.store(),
            &keyset,
            &addr_dst,
            10_000_000,
            fee,
            &[note],
            &&NoProofProver,
        )
        .await;
        assert!(
            matches!(&result, Err(PlatformWalletError::ShieldedBuildError(message))
            if message == &expected),
            "corruption must fail before proving: {result:?}"
        );
    }
    // Stored nullifiers are selection metadata. The proof builder derives the
    // actual nullifier from the note and FVK; corrupt metadata must not enter it.
    let mut corrupt = notes[0].clone();
    corrupt.nullifier[0] ^= 0xFF;
    let corrupt_nullifier = corrupt.nullifier;
    let built = operations::build_unshield_st(
        &pw.sdk_arc(),
        handle.coordinator.store(),
        &keyset,
        &addr_dst,
        10_000_000,
        fee,
        &[corrupt],
        &prover,
    )
    .await
    .expect("build from real note");
    let StateTransition::Unshield(UnshieldTransition::V0(built)) = built else {
        panic!("expected unshield")
    };
    assert!(built
        .actions
        .iter()
        .any(|action| action.nullifier == notes[0].nullifier));
    assert!(
        built
            .actions
            .iter()
            .all(|action| action.nullifier != corrupt_nullifier),
        "untrusted metadata must not become an on-wire nullifier"
    );
    let bank = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(network);
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank).await;

    s.teardown().await.expect("teardown");
}
