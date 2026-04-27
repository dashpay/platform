//! Test framework configuration.
//!
//! Centralises every `PLATFORM_WALLET_E2E_*` env var used by the
//! harness (see plan: SDK & Network Wiring) so a future
//! standalone-crate extraction can swap [`Config::from_env`] out
//! without rewiring call sites. The same struct can be built
//! programmatically via [`Config::new`].
//!
//! Wave 2 stub: field shape only — Wave 3 adds the parser, default
//! resolution (network → DAPI URLs, workdir → `${TMPDIR}/...`), and
//! validation of required fields.

use std::path::PathBuf;

use super::FrameworkResult;

/// Names of environment variables read by [`Config::from_env`].
/// Centralised so future-crate extraction stays mechanical.
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
}

/// E2E framework configuration.
///
/// Wave 2 stub. Wave 3 populates the loader and adds `Network` /
/// `DapiUri` parsing. The shape here matches the plan's env-var
/// table so call sites land directly on real fields once Wave 3
/// fills them in.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// BIP-39 bank mnemonic. Required (validated by `from_env`).
    pub bank_mnemonic: String,
    /// Network selector. Defaults to `"testnet"` when unset.
    pub network: String,
    /// Optional DAPI address overrides. Empty means "use the
    /// network default list".
    pub dapi_addresses: Vec<String>,
    /// Minimum bank balance threshold. Defaults to `100_000_000`.
    pub min_bank_credits: u64,
    /// Workdir base path; slot fallback adds `-N` suffixes.
    /// Defaults to `${TMPDIR}/dash-platform-wallet-e2e`.
    pub workdir_base: PathBuf,
}

impl Config {
    /// Load configuration from environment variables and `.env`.
    ///
    /// Wave 2 stub. Wave 3 wires `dotenvy::dotenv()`, parses every
    /// var listed in [`vars`], and validates required fields
    /// (currently just `BANK_MNEMONIC`).
    pub fn from_env() -> FrameworkResult<Self> {
        Err(super::FrameworkError::NotImplemented(
            "Config::from_env — wired in Wave 3",
        ))
    }

    /// Programmatic-construction entry point for the future
    /// standalone-crate extraction. Mirrors [`Config::from_env`]
    /// shape so test harnesses outside this repo don't need to
    /// route through env vars.
    pub fn new(bank_mnemonic: String) -> Self {
        Self {
            bank_mnemonic,
            ..Self::default()
        }
    }
}
