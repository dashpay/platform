//! Microbenchmark for `dash_sdk::platform::shielded::try_decrypt_note`.
//!
//! Run:
//!   cargo test -p platform-wallet --release --features shielded \
//!     shielded_decrypt_bench -- --ignored --nocapture
//!
//! Why this benchmark exists: a cold sync of the SDK_TEST_DATA chain (1M
//! filler notes) takes ~3 minutes on iPhone with ~50% CPU. The SDK's
//! sequential `for chunk in chunks { for note in chunk { try_decrypt_note(...)
//! } }` loop runs single-threaded after all chunks are fetched. This bench
//! isolates the CPU-bound decrypt path from network I/O so we can measure
//! single-threaded vs rayon-parallel throughput independently of DAPI
//! roundtrips.
//!
//! Note generation mirrors the on-chain SDK_TEST_DATA layout: random `rho`,
//! random encrypted payload of the canonical 216-byte wire length. The decrypt
//! path's early-out for non-Pallas `nullifier` bytes (~1/4 of random 32-byte
//! values) makes filler notes cheap to reject — but on a real chain every
//! recorded nullifier IS a valid Pallas element, so the realistic per-note
//! cost is somewhat higher. Treat these numbers as a lower bound on
//! production-chain decrypt time.

#![cfg(feature = "shielded")]

use std::time::Instant;

use dash_sdk::platform::shielded::try_decrypt_note;
use dashcore::Network;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use rayon::prelude::*;

use platform_wallet::wallet::shielded::keys::OrchardKeySet;

const ENCRYPTED_NOTE_WIRE_LEN: usize = 216;
const SEED_BENCH: [u8; 32] = [0x73; 32]; // matches SEED_A in drive-abci's seeder

/// Generate `count` filler `ShieldedEncryptedNote`s with random bytes
/// matching the on-chain wire layout. Deterministic given `rng_seed`.
fn generate_filler_notes(count: usize, rng_seed: u64) -> Vec<ShieldedEncryptedNote> {
    let mut rng = StdRng::seed_from_u64(rng_seed);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let mut cmx = vec![0u8; 32];
        let mut nullifier = vec![0u8; 32];
        let mut cv_net = vec![0u8; 32];
        let mut encrypted_note = vec![0u8; ENCRYPTED_NOTE_WIRE_LEN];
        rng.fill_bytes(&mut cmx);
        rng.fill_bytes(&mut nullifier);
        rng.fill_bytes(&mut cv_net);
        rng.fill_bytes(&mut encrypted_note);
        out.push(ShieldedEncryptedNote {
            cmx,
            nullifier,
            cv_net,
            encrypted_note,
        });
    }
    out
}

#[test]
#[ignore = "Microbenchmark; opt in via --ignored --nocapture"]
fn shielded_decrypt_bench() {
    // 100k is enough for stable rates while staying ~1-10s per run on dev
    // hardware. Bumping to 1M gives wall-clock that matches the iPhone cold
    // sync; useful for cross-check but iterates slowly.
    let count: usize = std::env::var("BENCH_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000);

    println!("== Shielded decrypt benchmark ==");
    println!("Note count:        {}", count);
    println!("Rayon threads:     {}", rayon::current_num_threads());

    let key_set =
        OrchardKeySet::from_seed(&SEED_BENCH, Network::Regtest, 0).expect("derive Orchard keys");
    let prepared_ivk =
        grovedb_commitment_tree::PreparedIncomingViewingKey::new(&key_set.incoming_viewing_key);

    print!("Generating {} notes...", count);
    let gen_start = Instant::now();
    let notes = generate_filler_notes(count, 0xDEAD_BEEF);
    let gen_ms = gen_start.elapsed().as_millis();
    println!(" {} ms", gen_ms);

    // Warm-up: run a small batch to amortize any first-call setup.
    for n in notes.iter().take(1024) {
        let _ = try_decrypt_note(&prepared_ivk, n);
    }

    // --- Single-threaded ---
    let st_start = Instant::now();
    let mut st_hits = 0usize;
    for n in &notes {
        if try_decrypt_note(&prepared_ivk, n).is_some() {
            st_hits += 1;
        }
    }
    let st_elapsed = st_start.elapsed();
    let st_rate = count as f64 / st_elapsed.as_secs_f64();
    println!(
        "Single-threaded:   {:>8.2} ms total, {:>9.0} notes/sec, hits={}",
        st_elapsed.as_secs_f64() * 1000.0,
        st_rate,
        st_hits
    );

    // --- Rayon parallel ---
    let par_start = Instant::now();
    let par_hits: usize = notes
        .par_iter()
        .filter(|n| try_decrypt_note(&prepared_ivk, n).is_some())
        .count();
    let par_elapsed = par_start.elapsed();
    let par_rate = count as f64 / par_elapsed.as_secs_f64();
    println!(
        "Rayon parallel:    {:>8.2} ms total, {:>9.0} notes/sec, hits={}",
        par_elapsed.as_secs_f64() * 1000.0,
        par_rate,
        par_hits
    );

    let speedup = par_rate / st_rate;
    println!("Speedup:           {:.2}×", speedup);

    // Project to 1M-note cold sync wall-clock for the decrypt phase only
    // (fetch / append / save are separate).
    let proj_st_s = 1_000_000.0 / st_rate;
    let proj_par_s = 1_000_000.0 / par_rate;
    println!("Projected for 1M notes:");
    println!("  Single-threaded: {:>6.1} s", proj_st_s);
    println!("  Rayon parallel:  {:>6.1} s", proj_par_s);
}
