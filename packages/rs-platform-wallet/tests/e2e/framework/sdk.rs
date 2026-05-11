//! `dash_sdk::Sdk` construction. [`build_sdk`] wires
//! [`TrustedHttpContextProvider`] (the SPV-backed alternative is
//! deferred — Task #15) and resolves DAPI addresses from
//! [`Config::dapi_addresses`] or — for mainnet/testnet — delegates to
//! `SdkBuilder::new_testnet()` / `new_mainnet()` (PR #3570 wires those
//! up against `dash_network_seeds::evo_seeds(network)` upstream).
//! Provider URL override: `PLATFORM_WALLET_E2E_TRUSTED_CONTEXT_URL`.

use std::num::NonZeroUsize;
use std::sync::Arc;

use dash_sdk::dapi_client::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use dashcore::Network;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

use super::config::Config;
use super::{FrameworkError, FrameworkResult};

/// LRU quorum-cache size for [`TrustedHttpContextProvider`].
const TRUSTED_CONTEXT_CACHE_SIZE: usize = 256;

/// Build a fresh `Sdk` with [`TrustedHttpContextProvider`] wired
/// (network-builtin URL, or [`Config::trusted_context_url`] override).
///
/// Returns the SDK plus a shared handle to the trusted context
/// provider so test helpers can call `add_known_contract` /
/// `add_known_token_configuration` after deploying contracts at
/// runtime — the SDK's proof verifier reads back through the same
/// provider, so dynamically-registered contracts must land in its
/// `known_contracts` cache before any state transition that touches
/// them is broadcast (otherwise `DriveProofError(UnknownContract)`).
///
/// The provider is `Clone` and its inner caches are `Arc<Mutex<...>>`,
/// so the clone handed to `SdkBuilder::with_context_provider` shares
/// state with the [`Arc`]-wrapped handle returned alongside the SDK —
/// any `add_known_*` call on the returned `Arc` is visible to the
/// SDK's verifier immediately. (QA-900)
pub fn build_sdk(config: &Config) -> FrameworkResult<(Arc<Sdk>, Arc<TrustedHttpContextProvider>)> {
    let network = config.network;
    let builder = build_sdk_builder(config, network)?;

    let cache_size = NonZeroUsize::new(TRUSTED_CONTEXT_CACHE_SIZE).expect("cache size > 0");
    let context_provider = build_trusted_context_provider(network, config, cache_size)?;

    // `TrustedHttpContextProvider: Clone` and its caches are `Arc<Mutex<...>>`,
    // so the clone passed into the SDK shares the `known_contracts` /
    // `known_token_configurations` maps with the `Arc` we hand back.
    let sdk = builder
        .with_context_provider(context_provider.clone())
        .build()
        .map_err(|e| {
            tracing::error!(target: "platform_wallet::e2e::sdk", "SdkBuilder::build failed: {e}");
            FrameworkError::Sdk(format!("SdkBuilder::build failed: {e}"))
        })?;

    Ok((Arc::new(sdk), Arc::new(context_provider)))
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
        FrameworkError::Sdk(format!(
            "TrustedHttpContextProvider construction failed: {e}"
        ))
    })
}

/// Pick the right [`SdkBuilder`] constructor based on [`Config::dapi_addresses`]
/// and `network`. Honours an explicit operator-supplied address list first;
/// otherwise mainnet/testnet delegate to `SdkBuilder::new_testnet()` /
/// `new_mainnet()` (PR #3570) which derive their bootstrap list from
/// `dash_network_seeds::evo_seeds(network)`. Devnet/local without an explicit
/// address list surfaces an error rather than guessing.
fn build_sdk_builder(config: &Config, network: Network) -> FrameworkResult<SdkBuilder> {
    if !config.dapi_addresses.is_empty() {
        let addresses = parse_addresses(config.dapi_addresses.iter().map(String::as_str))?;
        return Ok(SdkBuilder::new(addresses).with_network(network));
    }

    match network {
        Network::Testnet => Ok(SdkBuilder::new_testnet()),
        Network::Mainnet => Ok(SdkBuilder::new_mainnet()),
        other => {
            tracing::error!(
                target: "platform_wallet::e2e::sdk",
                "no DAPI addresses configured for {other:?} — set {} to a comma-separated list of DAPI URLs",
                super::config::vars::DAPI_ADDRESSES,
            );
            Err(FrameworkError::Config(format!(
                "no DAPI addresses configured for {other:?} — set {} to a comma-separated list of DAPI URLs",
                super::config::vars::DAPI_ADDRESSES,
            )))
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
                FrameworkError::Config(format!("invalid DAPI address {s:?}: {e}"))
            })
        })
        .collect()
}
