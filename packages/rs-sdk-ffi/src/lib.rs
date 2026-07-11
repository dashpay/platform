//! Dash Unified SDK FFI bindings
#![allow(clippy::result_large_err)]
#![allow(clippy::large_enum_variant)]
//!
//! This crate provides C-compatible FFI bindings for both Dash Core (SPV) and Platform SDKs,
//! enabling cross-platform applications to interact with the complete Dash ecosystem through C interfaces.

mod address;
mod address_sync;
mod contested_resource;
mod context_callbacks;
pub mod context_provider;
mod crypto;
mod data_contract;
mod document;
mod dpns;
mod error;
mod evonode;
mod group;
mod identity;
mod mnemonic_resolver;
mod mnemonic_resolver_core_signer;
mod protocol_version;
mod runtime;
mod sdk;
mod signer;
mod signer_simple;

pub use mnemonic_resolver::*;
pub use mnemonic_resolver_core_signer::*;
mod system;
mod token;
mod types;
mod utils;
mod voting;

#[cfg(test)]
mod test_utils;

pub use address::*;
pub use address_sync::*;
pub use contested_resource::*;
pub use context_callbacks::*;
pub use context_provider::*;
pub use crypto::*;
pub use data_contract::*;
pub use document::*;
pub use dpns::*;
pub use error::*;
pub use evonode::*;
pub use group::*;
pub use identity::*;
pub use protocol_version::*;
pub use sdk::*;
pub use signer::*;
pub use signer_simple::*;
pub use system::*;
pub use token::*;
pub use types::*;
pub use utils::*;
pub use voting::*;

/// Initialize the FFI library.
/// This should be called once at app startup before using any other functions.
#[no_mangle]
pub extern "C" fn dash_sdk_init() {
    // NOTE: Panic handler setup removed to avoid conflicts with dash-unified-ffi
    // The unified library sets its own panic handler in dash_unified_init()

    // Initialize context callbacks storage
    init_global_callbacks();

    // Initialize any other subsystems if needed
}

/// Enable logging with the specified level.
///
/// This function initializes a `tracing` subscriber with the given log level.
/// If the `RUST_LOG` environment variable is set, its directives take
/// precedence (useful for ad-hoc debugging); otherwise per-crate filter
/// directives derived from `level` are used.  The env var is only *read*,
/// never written, so the call is safe from any thread context (including
/// after a Tokio runtime has started).
///
/// The subscriber's built-in `tracing-log` bridge captures output from
/// crates that use the `log` facade, so a separate `env_logger::init()`
/// is not required.
///
/// If a global subscriber has already been set (e.g., by a previous call),
/// subsequent calls are a no-op and the original level is retained.
///
/// Level values: 0 = Error, 1 = Warn, 2 = Info, 3 = Debug, 4 = Trace
#[no_mangle]
pub extern "C" fn dash_sdk_enable_logging(level: u8) {
    let log_level = match level {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4 => "trace",
        _ => "info",
    };

    // Build the filter string with per-crate directives -- identical to what
    // was previously stored in RUST_LOG, but constructed in-process so there
    // is no data-race with concurrent `env::var` reads on other threads.
    // Includes `platform_wallet` + `platform_wallet_ffi` so wallet-side
    // traces (asset-lock catch-up, wait_for_proof, etc.) reach the
    // configured level instead of falling through to the EnvFilter
    // default (warn).
    let filter_string = format!(
        "dash_sdk={log_level},rs_sdk={log_level},rs_sdk_ffi={log_level},\
         platform_wallet={log_level},platform_wallet_ffi={log_level},\
         dapi_grpc={log_level},h2={log_level},tower={log_level},\
         hyper={log_level},tonic={log_level}"
    );

    // Honour RUST_LOG when present (read-only, no data-race); fall back
    // to the programmatic filter string otherwise.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter_string));

    // Initialize the global tracing subscriber.  `try_init` returns Err if a
    // subscriber is already installed; we intentionally ignore that so that
    // calling this function more than once is harmless.
    if tracing_subscriber::fmt::fmt()
        .with_env_filter(filter)
        .try_init()
        .is_ok()
    {
        tracing::info!(level = log_level, "logging enabled");
    }
}

/// Get the version of the Dash SDK FFI library
#[no_mangle]
pub extern "C" fn dash_sdk_version() -> *const std::os::raw::c_char {
    static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr() as *const std::os::raw::c_char
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `dash_sdk_enable_logging` does NOT set the RUST_LOG
    /// environment variable.  This is the core property that makes the
    /// function safe to call from a multi-threaded context.
    #[test]
    fn enable_logging_does_not_set_env_var() {
        // If RUST_LOG is already set by the test harness or environment,
        // we cannot reliably detect whether our function sets it, so skip.
        if std::env::var_os("RUST_LOG").is_some() {
            return;
        }

        // Call the function under test with each supported level.
        for level in 0..=4 {
            dash_sdk_enable_logging(level);
        }

        // The function must NOT have set RUST_LOG.
        assert!(
            std::env::var("RUST_LOG").is_err(),
            "RUST_LOG should not be set by dash_sdk_enable_logging; \
             env::set_var must not be used because it is a data race \
             in multi-threaded programs"
        );
    }

    /// Verify that the function can be called from multiple threads
    /// concurrently without panicking (i.e., no data race).
    #[test]
    fn enable_logging_is_thread_safe() {
        let handles: Vec<_> = (0..4)
            .map(|i| {
                std::thread::spawn(move || {
                    dash_sdk_enable_logging(i % 5);
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread should not panic");
        }
    }
}
