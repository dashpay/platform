//! Shared test helper for DPNS network tests.
//!
//! Reads the same `DASH_SDK_PLATFORM_HOST` / `DASH_SDK_PLATFORM_PORT` /
//! `DASH_SDK_PLATFORM_SSL` env vars used by `packages/rs-sdk/tests/.env`,
//! so vector regeneration against local devnets or SSH tunnels works
//! without editing source.

use rs_dapi_client::{Address, AddressList};

pub(super) fn test_address_list() -> AddressList {
    let env_path = format!("{}/tests/.env", env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(&env_path);

    let host = std::env::var("DASH_SDK_PLATFORM_HOST")
        .expect("DASH_SDK_PLATFORM_HOST must be set in tests/.env or environment");
    let port: u16 = std::env::var("DASH_SDK_PLATFORM_PORT")
        .expect("DASH_SDK_PLATFORM_PORT must be set in tests/.env or environment")
        .parse()
        .expect("DASH_SDK_PLATFORM_PORT must parse as u16");
    let ssl = std::env::var("DASH_SDK_PLATFORM_SSL")
        .ok()
        .map(|v| v == "true")
        .unwrap_or(true);
    let scheme = if ssl { "https" } else { "http" };
    let address: Address = format!("{scheme}://{host}:{port}")
        .parse()
        .expect("valid platform address");
    AddressList::from_iter([address])
}
