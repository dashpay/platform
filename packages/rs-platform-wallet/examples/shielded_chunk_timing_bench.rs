//! Focused per-chunk timing benchmark against a live SDK_TEST_DATA devnet.
//!
//! Issues N sequential single-chunk `ShieldedEncryptedNotes` fetches against
//! paloma and reports the distribution of per-chunk wall-clock. With the
//! `info!`-level "shielded chunk fetched" telemetry that landed in
//! `rs-sdk/src/platform/fetch.rs::fetch_with_metadata_and_proof`, each call
//! also logs its own `net_ms` (gRPC roundtrip) / `verify_ms` (proof
//! verification) split — so we get both the aggregate distribution and the
//! per-call breakdown without further instrumentation.
//!
//! This is an opt-in example (NOT a test) because it performs real network
//! I/O against a live devnet; cargo examples are compiled but never run by
//! `cargo test`.
//!
//! Run:
//!   cargo run -p platform-wallet --release --features shielded \
//!     --example shielded_chunk_timing_bench
//!
//! Env overrides (all optional):
//! - `PALOMA_QUORUM_URL`     defaults to http://44.238.203.84:8080
//! - `PALOMA_DAPI_ADDRESSES` comma-separated https://<ip>:1443 list; otherwise
//!   13 paloma defaults are used.
//! - `BENCH_CHUNKS`           how many sequential chunks to fetch (default 20)
//! - `BENCH_CHUNK_SIZE`       per-chunk size (default 2048; max enforced
//!   server-side via the protocol version)
//! - `BENCH_START_INDEX`      where to start fetching (default 0)
//!
//! Why sequential (not concurrent): the question this bench answers is
//! "what's the per-chunk cost?", not "what's the network throughput?". 16-way
//! concurrency is already in the production cold-sync code; this bench
//! isolates one chunk at a time so distribution stats reflect true per-call
//! cost (network + server proof gen + client verify) without contention
//! between chunks.

#![cfg(feature = "shielded")]

use std::sync::Arc;
use std::time::Instant;

use dash_sdk::platform::Fetch;
use dash_sdk::sdk::{Address, AddressList};
use dash_sdk::SdkBuilder;
use dashcore::Network;
use drive_proof_verifier::types::{ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery};
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

const DEFAULT_QUORUM_URL: &str = "http://44.238.203.84:8080";
const DEFAULT_DAPI_ADDRESSES: &[&str] = &[
    "https://68.67.122.85:1443",
    "https://68.67.122.86:1443",
    "https://68.67.122.87:1443",
    "https://68.67.122.88:1443",
    "https://68.67.122.192:1443",
    "https://68.67.122.193:1443",
    "https://68.67.122.195:1443",
    "https://68.67.122.196:1443",
    "https://68.67.122.197:1443",
    "https://68.67.122.198:1443",
    "https://68.67.122.199:1443",
    "https://68.67.122.206:1443",
    "https://68.67.122.207:1443",
];

fn dapi_addresses() -> AddressList {
    let raw = std::env::var("PALOMA_DAPI_ADDRESSES").unwrap_or_default();
    let addrs: Vec<Address> = if raw.trim().is_empty() {
        DEFAULT_DAPI_ADDRESSES
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect()
    } else {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    };
    assert!(!addrs.is_empty(), "no DAPI addresses configured");
    AddressList::from_iter(addrs)
}

fn quorum_url() -> String {
    std::env::var("PALOMA_QUORUM_URL").unwrap_or_else(|_| DEFAULT_QUORUM_URL.to_string())
}

fn percentile(sorted: &[u128], pct: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * pct / 100.0).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let _ = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .try_init();

    let chunks: u64 = std::env::var("BENCH_CHUNKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let chunk_size: u32 = std::env::var("BENCH_CHUNK_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2048);
    let start_index: u64 = std::env::var("BENCH_START_INDEX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let addresses = dapi_addresses();
    let quorum = quorum_url();
    let live_count = addresses.get_live_addresses().len();
    println!("== Shielded chunk timing bench ==");
    println!("Quorum URL:        {}", quorum);
    println!("DAPI nodes:        {}", live_count);
    println!("Chunks to fetch:   {}", chunks);
    println!("Chunk size:        {}", chunk_size);
    println!("Start index:       {}", start_index);

    let provider = TrustedHttpContextProvider::new_with_url(
        Network::Devnet,
        quorum.clone(),
        std::num::NonZeroUsize::new(100).unwrap(),
    )
    .expect("build TrustedHttpContextProvider");

    let sdk = SdkBuilder::new(addresses)
        .with_network(Network::Devnet)
        .with_context_provider(provider)
        .build()
        .expect("build sdk");
    let sdk = Arc::new(sdk);

    // Warm-up: prefetch_quorums should already have happened in build();
    // do one throwaway fetch to amortize any first-call connection setup.
    println!("Warm-up fetch...");
    let _ = ShieldedEncryptedNotes::fetch_with_metadata(
        sdk.as_ref(),
        ShieldedEncryptedNotesQuery {
            start_index,
            count: chunk_size,
        },
        None,
    )
    .await;

    let mut totals_ms: Vec<u128> = Vec::with_capacity(chunks as usize);
    let mut total_notes: usize = 0;

    // Server requires start_index to be a multiple of the protocol's
    // server-side chunk size (2048 today). We step by that constant so
    // every request is alignment-valid even when the user asks for a
    // smaller `count` — that lets us isolate per-request overhead vs
    // bandwidth without tripping the alignment check.
    const SERVER_CHUNK_ALIGNMENT: u64 = 2048;

    let bench_start = Instant::now();
    for i in 0..chunks {
        let idx = start_index + i * SERVER_CHUNK_ALIGNMENT;
        let q = ShieldedEncryptedNotesQuery {
            start_index: idx,
            count: chunk_size,
        };
        let call_start = Instant::now();
        let result = ShieldedEncryptedNotes::fetch_with_metadata(sdk.as_ref(), q, None).await;
        let call_ms = call_start.elapsed().as_millis();
        match result {
            Ok((Some(ShieldedEncryptedNotes { notes, .. }), _)) => {
                total_notes += notes.len();
                println!(
                    "  chunk {:>3} @ idx {:>8} -> {:>6.0} ms  ({} notes)",
                    i,
                    idx,
                    call_ms as f64,
                    notes.len()
                );
            }
            Ok((None, _)) => {
                println!(
                    "  chunk {:>3} @ idx {:>8} -> {:>6.0} ms  (empty)",
                    i, idx, call_ms as f64
                );
            }
            Err(e) => {
                println!("  chunk {:>3} @ idx {:>8} -> ERROR: {}", i, idx, e);
                continue;
            }
        }
        totals_ms.push(call_ms);
    }
    let bench_elapsed = bench_start.elapsed();

    if totals_ms.is_empty() {
        println!("No chunks succeeded.");
        return;
    }

    let mut sorted = totals_ms.clone();
    sorted.sort_unstable();
    let sum_ms: u128 = sorted.iter().sum();
    let avg_ms = sum_ms as f64 / sorted.len() as f64;

    println!();
    println!("== Per-chunk distribution (ms, sequential fetches) ==");
    println!("Count:     {}", sorted.len());
    println!("Min:       {:>7.0}", *sorted.first().unwrap() as f64);
    println!("p50:       {:>7.0}", percentile(&sorted, 50.0) as f64);
    println!("p90:       {:>7.0}", percentile(&sorted, 90.0) as f64);
    println!("p95:       {:>7.0}", percentile(&sorted, 95.0) as f64);
    println!("p99:       {:>7.0}", percentile(&sorted, 99.0) as f64);
    println!("Max:       {:>7.0}", *sorted.last().unwrap() as f64);
    println!("Avg:       {:>7.1}", avg_ms);
    println!();
    println!(
        "Total wall-clock: {:.2} s   Notes fetched: {}   Rate: {:.0} notes/s (sequential)",
        bench_elapsed.as_secs_f64(),
        total_notes,
        total_notes as f64 / bench_elapsed.as_secs_f64()
    );
    println!();
    println!(
        "NOTE: Each successful chunk also logs `shielded chunk fetched net_ms=N \
         verify_ms=N total_ms=N address=...` at info level — that's the network/\
         verify split for the same calls. Filter for `shielded chunk` in the bench \
         output above to read them in order."
    );
}
