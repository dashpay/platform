//! `dash_sdk::Sdk` construction for the e2e harness.
//!
//! [`build_sdk`] returns an `Arc<Sdk>` configured for the network
//! selected via [`super::config::Config`] (testnet by default;
//! `devnet` and `local` are accepted aliases for `Devnet` /
//! `Regtest`). DAPI addresses come from `Config::dapi_addresses` when
//! non-empty, otherwise the network's hard-coded testnet defaults are
//! used.
//!
//! # ContextProvider strategy
//!
//! The first iteration of the framework wires a [`NoopContextProvider`]
//! at SDK construction time. The first test (pure platform-address
//! transfers) doesn't need proof verification, so the no-op variant
//! is safe — any future call into proof verification would surface
//! an explicit error rather than silently returning fabricated keys.
//!
//! Once SPV is started ([`super::spv::start_spv`] +
//! [`super::spv::wait_for_mn_list_synced`]), the harness swaps in the
//! [`super::context_provider::SpvContextProvider`] via
//! [`dash_sdk::Sdk::set_context_provider`]. That method backs the
//! provider with `ArcSwap` (see `rs-sdk/src/sdk.rs`), so live swap
//! is supported and we do not need to rebuild the SDK once SPV is
//! ready. `harness.rs` (Wave 4) calls [`build_sdk`] exactly once
//! during init and then performs the swap in place.

use std::sync::Arc;

use dash_sdk::dapi_client::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use dashcore::Network;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

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

/// Build a fresh `Sdk` configured from `config`.
///
/// The returned SDK has a [`NoopContextProvider`] installed.
/// `harness.rs` calls [`Sdk::set_context_provider`] to upgrade to
/// [`super::context_provider::SpvContextProvider`] once SPV finishes
/// its initial masternode-list sync.
pub fn build_sdk(config: &Config) -> FrameworkResult<Arc<Sdk>> {
    let network = parse_network(&config.network)?;
    let address_list = build_address_list(config, network)?;

    let sdk = SdkBuilder::new(address_list)
        .with_network(network)
        .with_context_provider(NoopContextProvider)
        .build()
        .map_err(|e| {
            tracing::error!(target: "platform_wallet::e2e::sdk", "SdkBuilder::build failed: {e}");
            FrameworkError::NotImplemented("sdk::build_sdk — SdkBuilder::build failed (see logs)")
        })?;

    Ok(Arc::new(sdk))
}

/// Translate the string network selector from [`Config`] into a
/// `dashcore::Network` value. Accepts `testnet` (default in `Config`),
/// `mainnet`, `devnet`, `regtest`, and the `local` alias (mapped to
/// `Regtest` to match the convention used elsewhere in the workspace).
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

/// SDK [`ContextProvider`] that fails closed on quorum-key lookup
/// and returns `Ok(None)` for everything else.
///
/// Used as the bootstrap provider before SPV finishes its initial
/// sync. Tests that don't need proof verification (e.g. the
/// platform-address transfer happy path) never call
/// `get_quorum_public_key`, so the no-op variant is safe; tests that
/// do need it must wait for the harness to swap in the
/// [`super::context_provider::SpvContextProvider`] first.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopContextProvider;

impl dash_sdk::platform::ContextProvider for NoopContextProvider {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], dash_sdk::error::ContextProviderError> {
        Err(dash_sdk::error::ContextProviderError::Config(
            "NoopContextProvider: SPV-backed provider not yet wired".to_string(),
        ))
    }

    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, dash_sdk::error::ContextProviderError> {
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, dash_sdk::error::ContextProviderError> {
        Ok(None)
    }

    fn get_platform_activation_height(&self) -> Result<u32, dash_sdk::error::ContextProviderError> {
        Ok(0)
    }
}
