//! `dash_sdk::Sdk` construction for the e2e harness.
//!
//! [`build_sdk`] returns an `Arc<Sdk>` configured for the network
//! selected via [`super::config::Config`] (testnet by default;
//! `devnet` and `local` are accepted aliases for `Devnet` /
//! `Regtest`). DAPI addresses come from `Config::dapi_addresses`
//! when non-empty, otherwise the network's hard-coded testnet
//! defaults are used.
//!
//! # Context provider
//!
//! The harness wires
//! [`rs_sdk_trusted_context_provider::TrustedHttpContextProvider`]
//! as the SDK's [`ContextProvider`] directly at construction time.
//! That provider answers quorum public-key lookups over a trusted
//! HTTP endpoint (testnet / mainnet defaults are baked into the
//! crate); the harness does NOT spin up an SPV client to seed
//! quorum state. The SPV-based provider plumbing lives in
//! `framework/spv.rs` and `framework/context_provider.rs` for
//! future re-enablement (Task #15) but is currently disabled —
//! see `harness.rs` for the commented-out wiring.
//!
//! Operators can override the provider URL via
//! `PLATFORM_WALLET_E2E_TRUSTED_CONTEXT_URL` ([`Config::trusted_context_url`]).

use std::num::NonZeroUsize;
use std::sync::Arc;

use dash_sdk::dapi_client::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use dashcore::Network;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

use super::config::Config;
use super::{FrameworkError, FrameworkResult};

/// Default DAPI addresses used when `Config::dapi_addresses` is
/// empty. Mirrors the constant from `tests/spv_sync.rs` so both
/// integration test binaries point at the same well-known testnet
/// masternodes that are known to support compact block filters.
pub const TESTNET_DAPI_ADDRESSES: &[&str] = &[
    "https://68.67.122.1:1443",
    "https://68.67.122.2:1443",
    "https://68.67.122.3:1443",
];

/// Cache size for [`TrustedHttpContextProvider`]'s LRU quorum cache.
/// 256 entries comfortably covers the working set for a single
/// e2e test run; the provider only allocates an entry on a cache
/// miss and the bound is `NonZeroUsize` for the constructor.
const TRUSTED_CONTEXT_CACHE_SIZE: usize = 256;

/// Build a fresh `Sdk` configured from `config`.
///
/// Installs [`TrustedHttpContextProvider`] as the SDK's
/// [`ContextProvider`] using either the network-builtin endpoint
/// or the override at [`Config::trusted_context_url`] when set.
pub fn build_sdk(config: &Config) -> FrameworkResult<Arc<Sdk>> {
    let network = parse_network(&config.network)?;
    let address_list = build_address_list(config, network)?;

    let cache_size = NonZeroUsize::new(TRUSTED_CONTEXT_CACHE_SIZE).expect("cache size > 0");
    let context_provider = build_trusted_context_provider(network, config, cache_size)?;

    let sdk = SdkBuilder::new(address_list)
        .with_network(network)
        .with_context_provider(context_provider)
        .build()
        .map_err(|e| {
            tracing::error!(target: "platform_wallet::e2e::sdk", "SdkBuilder::build failed: {e}");
            FrameworkError::NotImplemented("sdk::build_sdk — SdkBuilder::build failed (see logs)")
        })?;

    Ok(Arc::new(sdk))
}

/// Build the trusted HTTP context provider for `network`, honoring
/// the optional `trusted_context_url` override.
fn build_trusted_context_provider(
    network: Network,
    config: &Config,
    cache_size: NonZeroUsize,
) -> FrameworkResult<TrustedHttpContextProvider> {
    let result = match &config.trusted_context_url {
        Some(url) => {
            tracing::info!(
                target: "platform_wallet::e2e::sdk",
                %url,
                "using TrustedHttpContextProvider with operator-supplied URL"
            );
            TrustedHttpContextProvider::new_with_url(network, url.clone(), cache_size)
        }
        None => {
            tracing::info!(
                target: "platform_wallet::e2e::sdk",
                ?network,
                "using TrustedHttpContextProvider with network-builtin URL"
            );
            TrustedHttpContextProvider::new(network, None, cache_size)
        }
    };
    result.map_err(|e| {
        tracing::error!(
            target: "platform_wallet::e2e::sdk",
            "TrustedHttpContextProvider construction failed: {e}"
        );
        FrameworkError::NotImplemented(
            "sdk::build_trusted_context_provider — TrustedHttpContextProvider failed (see logs)",
        )
    })
}

/// Translate the string network selector from [`Config`] into a
/// `dashcore::Network` value. Accepts `testnet` (default in
/// `Config`), `mainnet`, `devnet`, `regtest`, and the `local`
/// alias (mapped to `Regtest` to match the convention used
/// elsewhere in the workspace).
fn parse_network(name: &str) -> FrameworkResult<Network> {
    match name.trim().to_ascii_lowercase().as_str() {
        "" | "testnet" => Ok(Network::Testnet),
        "mainnet" => Ok(Network::Mainnet),
        "devnet" => Ok(Network::Devnet),
        "regtest" | "local" => Ok(Network::Regtest),
        other => {
            tracing::error!(
                target: "platform_wallet::e2e::sdk",
                "unknown network selector {other:?} (expected testnet/mainnet/devnet/regtest/local)"
            );
            Err(FrameworkError::NotImplemented(
                "sdk::parse_network — unknown network selector (see logs)",
            ))
        }
    }
}

/// Resolve the DAPI [`AddressList`] used by the SDK.
///
/// Honours [`Config::dapi_addresses`] when populated; otherwise falls
/// back to [`TESTNET_DAPI_ADDRESSES`] for testnet runs. For
/// non-testnet networks without explicit addresses we surface a
/// configuration error rather than guessing — devnet/local require
/// operator-provided endpoints.
fn build_address_list(config: &Config, network: Network) -> FrameworkResult<AddressList> {
    if !config.dapi_addresses.is_empty() {
        return parse_addresses(config.dapi_addresses.iter().map(String::as_str));
    }

    match network {
        Network::Testnet => parse_addresses(TESTNET_DAPI_ADDRESSES.iter().copied()),
        other => {
            tracing::error!(
                target: "platform_wallet::e2e::sdk",
                "no DAPI addresses configured for {other:?} — set {} to a comma-separated list of DAPI URLs",
                super::config::vars::DAPI_ADDRESSES,
            );
            Err(FrameworkError::NotImplemented(
                "sdk::build_address_list — no DAPI addresses configured (see logs)",
            ))
        }
    }
}

fn parse_addresses<'a, I>(iter: I) -> FrameworkResult<AddressList>
where
    I: IntoIterator<Item = &'a str>,
{
    iter.into_iter()
        .map(|s| {
            s.parse().map_err(|e| {
                tracing::error!(
                    target: "platform_wallet::e2e::sdk",
                    "invalid DAPI address {s:?}: {e}"
                );
                FrameworkError::NotImplemented(
                    "sdk::parse_addresses — invalid DAPI address (see logs)",
                )
            })
        })
        .collect()
}
