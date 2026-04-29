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
}

/// Default minimum bank balance in credits.
pub const DEFAULT_MIN_BANK_CREDITS: u64 = 100_000_000;

/// E2E framework configuration.
#[derive(Debug, Clone)]
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

        Ok(Self {
            bank_mnemonic,
            network,
            dapi_addresses,
            min_bank_credits,
            workdir_base,
            trusted_context_url,
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
