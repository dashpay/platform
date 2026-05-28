//! Demonstrates the boot-time protocol-version autodetect calibration.
//!
//! `SdkBuilder::new_testnet()` seeds PV 10 immediately and `build()` spawns a
//! best-effort proved fetch in the background that ratchets the per-instance
//! PV atomic to whatever the live testnet currently reports.
//!
//! Run with:
//! ```text
//! RUST_LOG=warn,dash_sdk::protocol_version=debug \
//!     cargo run -p dash-sdk --example version_autodetect
//! ```
//!
//! Set `DASH_TESTNET_QUORUMS_URL` to override the trusted-context provider's
//! root of trust if the built-in default is unreachable.

use std::{env, num::NonZeroUsize, time::Duration};

use dash_sdk::{
    platform::{fetch_current_no_parameters::FetchCurrent, types::epoch::Epoch},
    SdkBuilder,
};
use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dpp::dashcore::Network;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

#[tokio::main]
async fn main() {
    // `tracing::warn!` from the swallowed-calibration error path is visible
    // here; the calibration's own `debug!` lines need RUST_LOG=debug.
    tracing_subscriber::fmt::init();

    // Build a trusted context provider — the SDK refuses to fetch proved
    // responses without one, so the calibration would otherwise no-op.
    let quorums_url = env::var("DASH_TESTNET_QUORUMS_URL").ok();
    let provider = match quorums_url {
        Some(url) => TrustedHttpContextProvider::new_with_url(
            Network::Testnet,
            url,
            NonZeroUsize::new(64).unwrap(),
        ),
        None => {
            TrustedHttpContextProvider::new(Network::Testnet, None, NonZeroUsize::new(64).unwrap())
        }
    }
    .expect("trusted context provider");

    let sdk = SdkBuilder::new_testnet()
        .with_context_provider(provider)
        .build()
        .expect("build testnet SDK");

    // Immediate read: should be PV 10, the seeded testnet default. The
    // calibration spawn has not had a chance to run yet.
    println!("after build(): PV = {}", sdk.protocol_version_number());

    // Give the detached calibration task a moment to land. In a real app the
    // first user request would do this implicitly; here we sleep so the CLI
    // can show the "before / after calibration" delta.
    tokio::time::sleep(Duration::from_secs(2)).await;
    println!(
        "after 2s (calibration window): PV = {}",
        sdk.protocol_version_number()
    );

    // Independent confirmation via a user-driven proved fetch — same path
    // the calibration uses internally.
    match Epoch::fetch_current(&sdk).await {
        Ok(epoch) => println!(
            "epoch {} fetched; PV after user fetch = {}",
            epoch.index(),
            sdk.protocol_version_number()
        ),
        Err(err) => println!(
            "user fetch failed ({err}); PV stays at {}",
            sdk.protocol_version_number()
        ),
    }
}
