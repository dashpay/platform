//! The fixture proofs verify on wasm32, the target the browser SDK runs on,
//! and how long `bootstrap::verify` takes there is logged, since that time
//! runs on the browser's event loop.
#![cfg(target_arch = "wasm32")]

use dash_core_proof::{bootstrap, State};
use serde_json::Value;
use wasm_bindgen_test::{console_log, wasm_bindgen_test};
use web_time::Instant;

/// How many times each proof is verified, so one slow run does not decide the
/// figure.
const RUNS: u32 = 5;

#[wasm_bindgen_test]
fn should_verify_the_fixture_proofs_on_wasm_and_log_the_time() {
    // The chain lengths are those of the fixtures Core built: 4 links over
    // 2,000 testnet blocks, 2 over 1,500 mainnet blocks.
    for (name, json, links) in [
        ("testnet", include_str!("data/proof-testnet.json"), 4),
        ("mainnet", include_str!("data/proof-mainnet.json"), 2),
    ] {
        let fixture: Value = serde_json::from_str(json).expect("fixture JSON");
        let state = |key: &str| serde_json::from_value::<State>(fixture[key].clone()).expect(key);
        let checkpoint = state("checkpoint");
        let target = state("target");
        let proof = hex::decode(fixture["bootstrap_hex"].as_str().expect("bootstrap_hex"))
            .expect("bootstrap hex");

        let mut fastest = f64::MAX;
        let mut total = 0.0;
        for _ in 0..RUNS {
            let started = Instant::now();
            let verified = bootstrap::verify(&proof, &checkpoint, target.height)
                .expect("the fixture proof verifies");
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            assert_eq!(verified.state(), &target, "{name}");
            fastest = fastest.min(ms);
            total += ms;
        }
        let mean = total / f64::from(RUNS);
        console_log!(
            "{name}: {} byte proof, {links} links: fastest {fastest:.1} ms ({:.1} ms per link), \
             mean {mean:.1} ms over {RUNS} runs",
            proof.len(),
            fastest / f64::from(links),
        );
    }
}
