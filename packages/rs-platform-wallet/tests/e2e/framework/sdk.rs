//! `dash_sdk::Sdk` construction. [`build_sdk`] wires
//! [`TrustedHttpContextProvider`] (the SPV-backed alternative is
//! deferred — Task #15) and resolves DAPI addresses from
//! [`Config::dapi_addresses`] or the testnet defaults.
//! Provider URL override: `PLATFORM_WALLET_E2E_TRUSTED_CONTEXT_URL`.

use std::num::NonZeroUsize;
use std::sync::Arc;

use dash_sdk::dapi_client::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use dashcore::Network;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

use super::config::{parse_network, Config};
use super::{FrameworkError, FrameworkResult};

/// Default DAPI addresses for testnet — mirrors `tests/spv_sync.rs`
/// so both binaries hit the same masternodes that support compact
/// block filters.
pub const TESTNET_DAPI_ADDRESSES: &[&str] = &[
    "https://68.67.122.1:1443",
    "https://68.67.122.2:1443",
    "https://68.67.122.3:1443",
];

/// LRU quorum-cache size for [`TrustedHttpContextProvider`].
const TRUSTED_CONTEXT_CACHE_SIZE: usize = 256;

/// Build a fresh `Sdk` with [`TrustedHttpContextProvider`] wired
/// (network-builtin URL, or [`Config::trusted_context_url`] override).
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

/// Build the trusted HTTP context provider, honoring the optional
/// `trusted_context_url` override.
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

/// Resolve the DAPI [`AddressList`]. Honours
/// [`Config::dapi_addresses`]; otherwise testnet falls back to
/// [`TESTNET_DAPI_ADDRESSES`]. Devnet/local without explicit
/// addresses surfaces an error rather than guessing.
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
