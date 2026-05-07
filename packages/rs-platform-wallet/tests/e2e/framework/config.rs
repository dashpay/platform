//! Test framework configuration. Centralises every
//! `PLATFORM_WALLET_E2E_*` env var; loadable via [`Config::from_env`]
//! or constructed programmatically via [`Config::new`].
//!
//! Both constructors return a fully-resolved [`Config`]: every
//! defaultable field already carries its final value (no
//! `read-then-derive` lookups left for callers). `network` is parsed
//! once into [`Network`]; `p2p_port` is resolved against the
//! network-specific default at construction time.

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use dashcore::Network;

use super::{FrameworkError, FrameworkResult};

/// Environment variable names read by [`Config::from_env`].
pub mod vars {
    /// BIP-39 bank-wallet mnemonic. Required.
    pub const BANK_MNEMONIC: &str = "PLATFORM_WALLET_E2E_BANK_MNEMONIC";
    /// Network selector: `testnet` (default) / `mainnet` / `devnet` / `local`.
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
    /// the network default (mainnet 9999, testnet 19999); regtest and
    /// devnet have no default and require this var.
    pub const P2P_PORT: &str = "PLATFORM_WALLET_E2E_P2P_PORT";
    /// Optional 32-byte hex identifier of a pre-registered bank
    /// identity used as the destination of identity-credit sweeps.
    /// Unset falls back to "register a fresh bank identity from the
    /// bank's first platform address on first run and persist its id
    /// to the workdir slot".
    pub const BANK_IDENTITY_ID: &str = "PLATFORM_WALLET_E2E_BANK_IDENTITY_ID";
    /// Bank Core (Layer-1) funding gate. Controls how long the harness
    /// waits at init for the bank's confirmed Core balance to become
    /// non-zero — the SPV compact-filter scan must have walked past the
    /// bank's pre-funded UTXOs before tests like CR-* / ID-007 can
    /// observe them. Unset (default) enables the gate with a
    /// [`DEFAULT_BANK_CORE_GATE_TIMEOUT`] (900s) deadline; `0` /
    /// `disabled` / `false` / `off` opt out for Platform-only suites
    /// that don't need Core duffs; any positive integer overrides the
    /// timeout (in seconds).
    pub const BANK_CORE_GATE: &str = "PLATFORM_WALLET_E2E_BANK_CORE_GATE";
}

/// Default deadline for the bank Core funding gate when the env var is
/// unset. Sized to fit a cold-cache compact-filter scan from genesis on
/// testnet (~1.47M blocks ≈ 15 min); subsequent runs reuse the on-disk
/// cache and clear the gate in seconds.
pub const DEFAULT_BANK_CORE_GATE_TIMEOUT: Duration = Duration::from_secs(900);

/// Default minimum bank balance in credits.
///
/// Set at 5x the largest single-run cost (FUNDING_CREDITS=100M + ~15M chain-time
/// fee ≈ 115M per run) following DET's safety-factor pattern (dash-evo-tool#513).
/// Keeps the bank covering several consecutive runs even with the fee underestimate
/// from platform #3040 in play.
pub const DEFAULT_MIN_BANK_CREDITS: u64 = 500_000_000;

/// E2E framework configuration — fully resolved.
///
/// Every field carries its final value as of construction; callers
/// don't have to re-derive defaults. `network` is parsed; `p2p_port`
/// is the resolved port (override-or-default) — `None` only when the
/// network has no default and no override was supplied (regtest /
/// devnet without explicit configuration).
///
/// The `Debug` impl below is hand-written: a `derive(Debug)` would
/// print `bank_mnemonic` verbatim, which a stray
/// `tracing::info!("{config:?}")` or an `expect()` panic could leak
/// into CI logs.
#[derive(Clone)]
pub struct Config {
    /// BIP-39 bank mnemonic. Required.
    pub bank_mnemonic: String,
    /// Active network — parsed at construction.
    pub network: Network,
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
    /// SPV P2P port for the active network — resolved at construction
    /// time from the env override or the network default. `None` only
    /// when the network has no default and no override was provided
    /// (regtest / devnet without explicit configuration); the SPV
    /// peer-seeding path treats that as "skip and fall back to DNS
    /// discovery."
    pub p2p_port: Option<u16>,
    /// Optional pre-registered bank-identity id (32 bytes hex). When
    /// set, the harness loads it on init; when unset, the harness
    /// auto-registers a bank identity on first run and persists its
    /// id under the workdir slot.
    pub bank_identity_id: Option<String>,
    /// Bank Core (Layer-1) funding gate timeout. `Some(d)` waits up to
    /// `d` for the bank's confirmed Core balance to become non-zero
    /// before letting init proceed; `None` skips the gate entirely.
    /// Default is `Some(`[`DEFAULT_BANK_CORE_GATE_TIMEOUT`]`)` — opt
    /// out via `PLATFORM_WALLET_E2E_BANK_CORE_GATE=0` for Platform-
    /// only suites that don't need Core duffs.
    pub bank_core_gate_timeout: Option<Duration>,
    /// Source of [`bank_core_gate_timeout`]'s value, kept for the init
    /// log line so operators can tell defaulted-on from env-set.
    pub bank_core_gate_source: BankCoreGateSource,
}

/// Provenance of the resolved bank-Core-gate timeout — surfaced in the
/// harness init log so operators can tell "default kicked in" from
/// "operator set the var".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BankCoreGateSource {
    /// Env var unset — default-on with [`DEFAULT_BANK_CORE_GATE_TIMEOUT`].
    Default,
    /// Env var set to a value that disables the gate (`0`, `disabled`,
    /// `false`, `off`).
    EnvDisabled,
    /// Env var set to a positive integer — used as the timeout (seconds).
    EnvTimeout,
    /// Env var set to a value that didn't parse — fell back to the
    /// default timeout with a warning.
    EnvInvalidFallback,
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
            .field("bank_identity_id", &self.bank_identity_id)
            .field("bank_core_gate_timeout", &self.bank_core_gate_timeout)
            .field("bank_core_gate_source", &self.bank_core_gate_source)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        let network = Network::Testnet;
        Self {
            bank_mnemonic: String::new(),
            network,
            dapi_addresses: Vec::new(),
            min_bank_credits: DEFAULT_MIN_BANK_CREDITS,
            workdir_base: default_workdir_base(),
            trusted_context_url: None,
            p2p_port: default_p2p_port(network),
            bank_identity_id: None,
            bank_core_gate_timeout: Some(DEFAULT_BANK_CORE_GATE_TIMEOUT),
            bank_core_gate_source: BankCoreGateSource::Default,
        }
    }
}

impl Config {
    /// Load from environment variables, with `.env` at
    /// `${CARGO_MANIFEST_DIR}/tests/.env` as a CWD-independent
    /// fallback. `bank_mnemonic` is required; everything else
    /// resolves to its final value via the per-field defaults.
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

        let network = match std::env::var(vars::NETWORK) {
            Ok(raw) => parse_network(&raw)?,
            Err(_) => Network::Testnet,
        };

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
                    default_p2p_port(network)
                } else {
                    Some(trimmed.parse::<u16>().map_err(|err| {
                        FrameworkError::Config(format!(
                            "{} = {raw:?} is not a valid u16 port: {err}",
                            vars::P2P_PORT
                        ))
                    })?)
                }
            }
            Err(_) => default_p2p_port(network),
        };

        let bank_identity_id = std::env::var(vars::BANK_IDENTITY_ID)
            .ok()
            .map(|raw| raw.trim().to_string())
            .filter(|s| !s.is_empty());

        let (bank_core_gate_timeout, bank_core_gate_source) =
            parse_bank_core_gate(std::env::var(vars::BANK_CORE_GATE).ok().as_deref());

        Ok(Self {
            bank_mnemonic,
            network,
            dapi_addresses,
            min_bank_credits,
            workdir_base,
            trusted_context_url,
            p2p_port,
            bank_identity_id,
            bank_core_gate_timeout,
            bank_core_gate_source,
        })
    }

    /// Programmatic constructor — mirrors [`Config::from_env`] for
    /// test harnesses that don't route through env vars. Returns a
    /// fully-resolved config: `network` defaults to testnet and
    /// `p2p_port` to the testnet default (19999).
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
/// [`vars::P2P_PORT`]. Used only at [`Config`] construction; callers
/// read the resolved [`Config::p2p_port`] directly.
fn default_p2p_port(network: Network) -> Option<u16> {
    match network {
        Network::Mainnet => Some(9999),
        Network::Testnet => Some(19999),
        _ => None,
    }
}

/// Resolve the bank Core funding gate timeout from the env-var raw
/// value (`None` = unset).
///
/// Mapping:
/// - unset (default) → on, [`DEFAULT_BANK_CORE_GATE_TIMEOUT`]
/// - `0` / `disabled` / `false` / `off` (case-insensitive) → off
/// - positive integer → on, that many seconds
/// - non-empty unparseable → on, default timeout, with a warning
/// - empty string → on, default timeout (treated as unset)
pub(crate) fn parse_bank_core_gate(raw: Option<&str>) -> (Option<Duration>, BankCoreGateSource) {
    let Some(raw) = raw else {
        return (
            Some(DEFAULT_BANK_CORE_GATE_TIMEOUT),
            BankCoreGateSource::Default,
        );
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (
            Some(DEFAULT_BANK_CORE_GATE_TIMEOUT),
            BankCoreGateSource::Default,
        );
    }

    if trimmed == "0"
        || trimmed.eq_ignore_ascii_case("disabled")
        || trimmed.eq_ignore_ascii_case("false")
        || trimmed.eq_ignore_ascii_case("off")
    {
        return (None, BankCoreGateSource::EnvDisabled);
    }

    match trimmed.parse::<u64>() {
        Ok(secs) => (
            Some(Duration::from_secs(secs)),
            BankCoreGateSource::EnvTimeout,
        ),
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::config",
                var = vars::BANK_CORE_GATE,
                value = %raw,
                ?err,
                default_secs = DEFAULT_BANK_CORE_GATE_TIMEOUT.as_secs(),
                "could not parse bank Core gate value; falling back to default timeout"
            );
            (
                Some(DEFAULT_BANK_CORE_GATE_TIMEOUT),
                BankCoreGateSource::EnvInvalidFallback,
            )
        }
    }
}

/// Parse a network string supporting the canonical dashcore names
/// plus the test-harness `local` alias for regtest and an empty
/// shorthand for testnet. Used only at [`Config`] construction;
/// callers read the resolved [`Config::network`] directly.
fn parse_network(s: &str) -> FrameworkResult<Network> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_core_gate_unset_defaults_to_900s() {
        let (timeout, src) = parse_bank_core_gate(None);
        assert_eq!(timeout, Some(DEFAULT_BANK_CORE_GATE_TIMEOUT));
        assert_eq!(src, BankCoreGateSource::Default);
    }

    #[test]
    fn bank_core_gate_empty_string_defaults_to_900s() {
        let (timeout, src) = parse_bank_core_gate(Some(""));
        assert_eq!(timeout, Some(DEFAULT_BANK_CORE_GATE_TIMEOUT));
        assert_eq!(src, BankCoreGateSource::Default);

        let (timeout, src) = parse_bank_core_gate(Some("   "));
        assert_eq!(timeout, Some(DEFAULT_BANK_CORE_GATE_TIMEOUT));
        assert_eq!(src, BankCoreGateSource::Default);
    }

    #[test]
    fn bank_core_gate_zero_disables() {
        let (timeout, src) = parse_bank_core_gate(Some("0"));
        assert_eq!(timeout, None);
        assert_eq!(src, BankCoreGateSource::EnvDisabled);
    }

    #[test]
    fn bank_core_gate_aliases_disable() {
        for raw in ["disabled", "DISABLED", "false", "False", "off", "OFF"] {
            let (timeout, src) = parse_bank_core_gate(Some(raw));
            assert_eq!(timeout, None, "{raw}");
            assert_eq!(src, BankCoreGateSource::EnvDisabled, "{raw}");
        }
    }

    #[test]
    fn bank_core_gate_positive_integer_overrides_timeout() {
        let (timeout, src) = parse_bank_core_gate(Some("60"));
        assert_eq!(timeout, Some(Duration::from_secs(60)));
        assert_eq!(src, BankCoreGateSource::EnvTimeout);

        let (timeout, src) = parse_bank_core_gate(Some("  120  "));
        assert_eq!(timeout, Some(Duration::from_secs(120)));
        assert_eq!(src, BankCoreGateSource::EnvTimeout);
    }

    #[test]
    fn bank_core_gate_invalid_falls_back_to_default() {
        let (timeout, src) = parse_bank_core_gate(Some("abc"));
        assert_eq!(timeout, Some(DEFAULT_BANK_CORE_GATE_TIMEOUT));
        assert_eq!(src, BankCoreGateSource::EnvInvalidFallback);

        let (timeout, src) = parse_bank_core_gate(Some("-1"));
        assert_eq!(timeout, Some(DEFAULT_BANK_CORE_GATE_TIMEOUT));
        assert_eq!(src, BankCoreGateSource::EnvInvalidFallback);
    }
}
