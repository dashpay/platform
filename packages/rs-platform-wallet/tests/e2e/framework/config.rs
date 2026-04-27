//! Test framework configuration.
//!
//! Centralises every `PLATFORM_WALLET_E2E_*` env var used by the
//! harness (see plan: SDK & Network Wiring) so a future
//! standalone-crate extraction can swap [`Config::from_env`] out
//! without rewiring call sites. The same struct can be built
//! programmatically via [`Config::new`].

use std::path::PathBuf;

use super::{FrameworkError, FrameworkResult};

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
    /// Optional override URL for the trusted HTTP context provider.
    /// Defaults to the network-builtin endpoint baked into
    /// `rs-sdk-trusted-context-provider` when unset.
    pub const TRUSTED_CONTEXT_URL: &str = "PLATFORM_WALLET_E2E_TRUSTED_CONTEXT_URL";
}

/// Default minimum bank balance in credits — `100_000_000` matches
/// the plan's env-var table.
pub const DEFAULT_MIN_BANK_CREDITS: u64 = 100_000_000;

/// E2E framework configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// BIP-39 bank mnemonic. Required (validated by `from_env`).
    pub bank_mnemonic: String,
    /// Network selector. Defaults to `"testnet"` when unset.
    pub network: String,
    /// Optional DAPI address overrides. Empty means "use the
    /// network default list".
    pub dapi_addresses: Vec<String>,
    /// Minimum bank balance threshold (credits). Defaults to
    /// [`DEFAULT_MIN_BANK_CREDITS`].
    pub min_bank_credits: u64,
    /// Workdir base path; slot fallback adds `-N` suffixes.
    /// Defaults to `${TMPDIR}/dash-platform-wallet-e2e`.
    pub workdir_base: PathBuf,
    /// Optional override for the trusted HTTP context provider URL.
    /// `None` means "use the per-network default baked into the
    /// `rs-sdk-trusted-context-provider` crate" (testnet / mainnet
    /// have built-in endpoints; devnet requires this override).
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
    /// Load configuration from environment variables and
    /// `${CARGO_MANIFEST_DIR}/tests/.env`.
    ///
    /// The `.env` path is anchored at the crate's manifest dir
    /// (mirrors the convention from
    /// `packages/rs-sdk/tests/fetch/config.rs` and
    /// `packages/rs-sdk-ffi/tests/integration_tests/config.rs`),
    /// so loading is deterministic regardless of the caller's CWD.
    /// A missing `.env` is fine — process env vars stay the
    /// source of truth — but if the file exists and fails to
    /// parse, the warning surfaces in test logs.
    ///
    /// The bank mnemonic is required; everything else falls back
    /// to the defaults documented on each [`Config`] field.
    pub fn from_env() -> FrameworkResult<Self> {
        // Best-effort `.env` load anchored at the crate's manifest
        // dir — matches workspace convention. A missing file is
        // expected (CI rarely ships one); other failures (parse
        // error, permissions) get logged but don't abort init.
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

/// `${TMPDIR}/dash-platform-wallet-e2e` — the default workdir base
/// before slot-fallback. Matches the plan's "Workdir &
/// Cross-Process Coordination" section.
fn default_workdir_base() -> PathBuf {
    std::env::temp_dir().join("dash-platform-wallet-e2e")
}
