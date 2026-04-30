//! Test framework configuration. Centralises every
//! `PLATFORM_WALLET_E2E_*` env var; loadable via [`Config::from_env`]
//! or constructed programmatically via [`Config::new`].

use std::path::PathBuf;
use std::str::FromStr;

use dashcore::Network;

use super::{FrameworkError, FrameworkResult};

/// Environment variable names read by [`Config::from_env`].
pub mod vars {
    /// BIP-39 bank-wallet mnemonic. Required.
    pub const BANK_MNEMONIC: &str = "PLATFORM_WALLET_E2E_BANK_MNEMONIC";
    /// Network selector: `testnet` (default) / `devnet` / `local`.
    pub const NETWORK: &str = "PLATFORM_WALLET_E2E_NETWORK";
    /// Comma-separated list of DAPI addresses overriding the
    /// network default.
    pub const DAPI_ADDRESSES: &str = "PLATFORM_WALLET_E2E_DAPI_ADDRESSES";
    /// Minimum bank balance (credits) required at startup.
    pub const MIN_BANK_CREDITS: &str = "PLATFORM_WALLET_E2E_MIN_BANK_CREDITS";
    /// Workdir base path; slot fallback adds `-N` suffixes.
    pub const WORKDIR: &str = "PLATFORM_WALLET_E2E_WORKDIR";
    /// Optional override for the trusted HTTP context provider URL.
    /// Defaults to the network-builtin endpoint when unset.
    pub const TRUSTED_CONTEXT_URL: &str = "PLATFORM_WALLET_E2E_TRUSTED_CONTEXT_URL";
    /// Optional override for the SPV P2P port. Unset falls back to
    /// the network-default ([`super::default_p2p_port`]).
    pub const P2P_PORT: &str = "PLATFORM_WALLET_E2E_P2P_PORT";
}

/// Default minimum bank balance in credits.
///
/// Set at 5x the largest single-run cost (FUNDING_CREDITS=100M + ~15M chain-time
/// fee ≈ 115M per run) following DET's safety-factor pattern (dash-evo-tool#513).
/// Keeps the bank covering several consecutive runs even with the fee underestimate
/// from platform #3040 in play.
pub const DEFAULT_MIN_BANK_CREDITS: u64 = 500_000_000;

/// E2E framework configuration.
///
/// The `Debug` impl below is hand-written: a `derive(Debug)` would print
/// `bank_mnemonic` verbatim, which a stray `tracing::info!("{config:?}")`
/// or an `expect()` panic could leak into CI logs.
#[derive(Clone)]
pub struct Config {
    /// BIP-39 bank mnemonic. Required.
    pub bank_mnemonic: String,
    /// Network selector. Defaults to `"testnet"`.
    pub network: String,
    /// Optional DAPI address overrides; empty means use the
    /// network default list.
    pub dapi_addresses: Vec<String>,
    /// Minimum bank balance threshold (credits).
    pub min_bank_credits: u64,
    /// Workdir base path; slot fallback adds `-N` suffixes.
    pub workdir_base: PathBuf,
    /// Optional trusted-context-provider URL override. `None` uses
    /// the per-network default; devnet requires this override.
    pub trusted_context_url: Option<String>,
    /// Optional SPV P2P port override. `None` falls back to
    /// [`default_p2p_port`] for the active network. Custom-port
    /// devnets / `local` always require this override (or the
    /// SPV path skips peer-seeding).
    pub p2p_port: Option<u16>,
}

impl std::fmt::Debug for Config {
    /// Redacts `bank_mnemonic`. Logs and panic backtraces would
    /// otherwise leak the shared funding seed into CI artifacts.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("bank_mnemonic", &"<redacted>")
            .field("network", &self.network)
            .field("dapi_addresses", &self.dapi_addresses)
            .field("min_bank_credits", &self.min_bank_credits)
            .field("workdir_base", &self.workdir_base)
            .field("trusted_context_url", &self.trusted_context_url)
            .field("p2p_port", &self.p2p_port)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bank_mnemonic: String::new(),
            network: "testnet".into(),
            dapi_addresses: Vec::new(),
            min_bank_credits: DEFAULT_MIN_BANK_CREDITS,
            workdir_base: default_workdir_base(),
            trusted_context_url: None,
            p2p_port: None,
        }
    }
}

impl Config {
    /// Load from environment variables, with `.env` at
    /// `${CARGO_MANIFEST_DIR}/tests/.env` as a CWD-independent
    /// fallback. `bank_mnemonic` is required; everything else
    /// uses the per-field defaults.
    pub fn from_env() -> FrameworkResult<Self> {
        // Anchor the `.env` path at the crate's manifest dir so
        // CWD doesn't change behaviour; a missing file is expected.
        let path: String = env!("CARGO_MANIFEST_DIR").to_owned() + "/tests/.env";
        if let Err(err) = dotenvy::from_path(&path) {
            tracing::warn!(
                target: "platform_wallet::e2e::config",
                path = %path,
                ?err,
                "failed to load e2e .env (process env vars still apply)"
            );
        }

        let bank_mnemonic = std::env::var(vars::BANK_MNEMONIC).map_err(|_| {
            FrameworkError::Bank(format!(
                "{} not set — point it at a BIP-39 testnet mnemonic with at least \
                 {} pre-funded credits and re-run",
                vars::BANK_MNEMONIC,
                DEFAULT_MIN_BANK_CREDITS
            ))
        })?;

        let network = std::env::var(vars::NETWORK).unwrap_or_else(|_| "testnet".into());

        let dapi_addresses = std::env::var(vars::DAPI_ADDRESSES)
            .ok()
            .map(|raw| {
                raw.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let min_bank_credits = match std::env::var(vars::MIN_BANK_CREDITS) {
            Ok(raw) => raw.trim().parse::<u64>().map_err(|err| {
                FrameworkError::Bank(format!(
                    "{} = {raw:?} is not a valid u64: {err}",
                    vars::MIN_BANK_CREDITS
                ))
            })?,
            Err(_) => DEFAULT_MIN_BANK_CREDITS,
        };

        let workdir_base = std::env::var(vars::WORKDIR)
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_workdir_base());

        let trusted_context_url = std::env::var(vars::TRUSTED_CONTEXT_URL)
            .ok()
            .map(|raw| raw.trim().to_string())
            .filter(|s| !s.is_empty());

        let p2p_port = match std::env::var(vars::P2P_PORT) {
            Ok(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.parse::<u16>().map_err(|err| {
                        FrameworkError::Config(format!(
                            "{} = {raw:?} is not a valid u16 port: {err}",
                            vars::P2P_PORT
                        ))
                    })?)
                }
            }
            Err(_) => None,
        };

        Ok(Self {
            bank_mnemonic,
            network,
            dapi_addresses,
            min_bank_credits,
            workdir_base,
            trusted_context_url,
            p2p_port,
        })
    }

    /// Programmatic constructor — mirrors [`Config::from_env`] for
    /// test harnesses that don't route through env vars.
    pub fn new(bank_mnemonic: String) -> Self {
        Self {
            bank_mnemonic,
            ..Self::default()
        }
    }
}

/// `${TMPDIR}/dash-platform-wallet-e2e` — default workdir base
/// before slot-fallback.
fn default_workdir_base() -> PathBuf {
    std::env::temp_dir().join("dash-platform-wallet-e2e")
}

/// Network-default SPV P2P port. Mirrors the canonical mainnet (9999)
/// and testnet (19999) ports. Returns `None` for regtest / devnet —
/// those have site-specific ports and must be supplied via
/// [`Config::p2p_port`].
pub(super) fn default_p2p_port(network: Network) -> Option<u16> {
    match network {
        Network::Mainnet => Some(9999),
        Network::Testnet => Some(19999),
        _ => None,
    }
}

/// Resolve the effective SPV P2P port: explicit [`Config::p2p_port`]
/// override wins; otherwise fall back to [`default_p2p_port`].
pub(super) fn effective_p2p_port(config: &Config, network: Network) -> Option<u16> {
    config.p2p_port.or_else(|| default_p2p_port(network))
}

/// Parse a network string supporting the canonical dashcore names
/// plus the test-harness `local` alias for regtest and an empty
/// shorthand for testnet. Delegates the rest to `<Network as FromStr>`.
pub(super) fn parse_network(s: &str) -> FrameworkResult<Network> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(Network::Testnet);
    }
    if trimmed.eq_ignore_ascii_case("local") {
        return Ok(Network::Regtest);
    }
    Network::from_str(trimmed)
        .map_err(|e| FrameworkError::Config(format!("invalid network {trimmed:?}: {e}")))
}
