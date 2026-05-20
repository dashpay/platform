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
use dpp::version::PlatformVersion;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;

use super::config::Config;
use super::{FrameworkError, FrameworkResult};

/// Initial protocol version the e2e SDK seeds itself at via
/// `SdkBuilder::with_initial_version`. Deliberately set BELOW the
/// expected network version (Dash Platform v3.0 testnet = PV_11) so
/// every funded run also exercises #3483's upward-ratchet auto-detect
/// path: `maybe_update_protocol_version`'s `fetch_max` should bump
/// the per-instance atomic to the network's reported version after
/// the first proof-verified response. A clean run therefore proves
/// **both** that v3.0 wire compatibility holds (the SDK starts below
/// or at v3.0's PV so the V0/V1 dispatch picks V0) AND that the
/// auto-detect ratchet is doing its job (the atomic ends up >= the
/// network version). Hardcoded as a regression probe; once the
/// upward-ratchet shape stops needing live verification this can be
/// replaced with `PlatformVersion::get(11)` (or whatever PV the
/// shared testnet has rolled to).
const INITIAL_PROTOCOL_VERSION: dpp::util::deserializer::ProtocolVersion = 10;

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

    // Seed the SDK's protocol-version atomic to `INITIAL_PROTOCOL_VERSION`
    // (deliberately BELOW the testnet PV — see the const's doc). Auto-detect
    // stays on; `maybe_update_protocol_version`'s upward `fetch_max` ratchet
    // bumps it to the network's actual version after the first response
    // metadata round-trip. Pre-ratchet, the documents-query encoder reads
    // PV_10's `drive_abci.query.document_query.default_current_version` (= 0
    // via `DRIVE_ABCI_QUERY_VERSIONS_V0`) → V0 wire bytes, which v3.0 HPMNs
    // deserialize. Post-ratchet, future versioned features pick up the new
    // PV naturally.
    let initial_pv = PlatformVersion::get(INITIAL_PROTOCOL_VERSION).map_err(|e| {
        tracing::error!(
            target: "platform_wallet::e2e::sdk",
            "PlatformVersion::get({INITIAL_PROTOCOL_VERSION}) failed: {e}"
        );
        FrameworkError::Sdk(format!(
            "PlatformVersion::get({INITIAL_PROTOCOL_VERSION}) failed: {e}"
        ))
    })?;
    tracing::info!(
        target: "platform_wallet::e2e::sdk",
        initial_protocol_version = INITIAL_PROTOCOL_VERSION,
        "seeding SDK initial protocol version; auto-detect ratchet will adjust upward on first response"
    );

    // `TrustedHttpContextProvider: Clone` and its caches are `Arc<Mutex<...>>`,
    // so the clone passed into the SDK shares the `known_contracts` /
    // `known_token_configurations` maps with the `Arc` we hand back.
    let sdk = builder
        .with_initial_version(initial_pv)
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
