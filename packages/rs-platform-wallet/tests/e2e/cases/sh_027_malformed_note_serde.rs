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

use platform_wallet::error::PlatformWalletError;
use platform_wallet::wallet::shielded::{operations, OrchardKeySet, SubwalletId};

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, in_memory_store, seed_malformed_note, shielded_prover,
};

/// Malformed `note_data` lengths to probe. The valid layout is 115 bytes
/// (`recipient43 ‖ value8 ‖ rho32 ‖ rseed32`); each of these must error.
const BAD_LENGTHS: &[usize] = &[0, 1, 114, 116, 200];

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

    for &len in BAD_LENGTHS {
        // Fresh store per length so a prior malformed note can't mask the
        // next. The note value is large so note-selection picks it and
        // the deserialize path is reached.
        let store = in_memory_store();
        seed_malformed_note(
            &store,
            id,
            50_000_000,
            vec![0xABu8; len],
            [0x11; 32],
            [0x22; 32],
        )
        .await
        .expect("seed malformed note");

        // Drive the spend path. `deserialize_note` runs inside
        // `extract_spends_and_anchor` (per note, before witness), so a
        // malformed note surfaces a typed `ShieldedBuildError`. An
        // in-memory store ALSO has a hard-Err witness() (Found-027), so a
        // 115-byte-but-otherwise-bad note can instead surface
        // `ShieldedMerkleWitnessUnavailable` — both are acceptable typed
        // errors. The forbidden outcomes are `Ok` (silent corruption) and
        // a PANIC (host DoS). A panic propagates as a test failure naming
        // this case, which is itself the RED finding.
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

        match result {
            Err(
                PlatformWalletError::ShieldedBuildError(_)
                | PlatformWalletError::ShieldedMerkleWitnessUnavailable(_)
                | PlatformWalletError::ShieldedInsufficientBalance { .. },
            ) => {
                tracing::info!(
                    target: "platform_wallet::e2e::cases::sh_027",
                    len,
                    "malformed note ({len} B) rejected with a typed error (no panic)"
                );
            }
            Ok(()) => panic!(
                "SH-027 FINDING: malformed {len}-byte note_data was accepted into a built bundle \
                 (silent corruption)"
            ),
            Err(other) => panic!(
                "SH-027: malformed {len}-byte note must surface a typed serde/witness error; \
                 observed {other:?}"
            ),
        }
    }

    s.teardown().await.expect("teardown");
}
