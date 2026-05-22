//! Test framework configuration. Centralises every
//! `PLATFORM_WALLET_E2E_*` env var; loadable via [`Config::from_env`]
//! or constructed programmatically via [`Config::new`].
//!
//! Both constructors return a fully-resolved [`Config`]: every
//! defaultable field already carries its final value (no
//! `read-then-derive` lookups left for callers). `network` is parsed
//! once into [`Network`]; `p2p_port` is resolved against the
//! network-specific default at construction time.

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use dashcore::Network;
use dpp::fee::Credits;

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
    /// Context-provider backend selector: `http` (trusted HTTP quorums
    /// host) or `spv` (quorum keys resolved from the local SPV runtime's
    /// masternode list, no hosted quorums service needed). Unset auto-
    /// selects per network (see [`ContextProviderKind::resolve`]).
    pub const CONTEXT_PROVIDER: &str = "PLATFORM_WALLET_E2E_CONTEXT_PROVIDER";
    /// Optional override for the SPV P2P port. Unset falls back to
    /// the network default (mainnet 9999, testnet 19999); regtest and
    /// devnet have no default and require this var.
    pub const P2P_PORT: &str = "PLATFORM_WALLET_E2E_P2P_PORT";
    /// Optional 32-byte hex identifier of a pre-registered bank
    /// identity used as the transient mid-run sink for the
    /// Platform→Core refill chain in [`super::bank_rebalance`].
    /// Identity-side test sweeps drain directly to the bank's Platform
    /// address; this identity exists for the refill buffer + legacy
    /// compatibility. Unset falls back to "register a fresh bank
    /// identity from the bank's first platform address on first run
    /// and persist its id to the workdir slot".
    pub const BANK_IDENTITY_ID: &str = "PLATFORM_WALLET_E2E_BANK_IDENTITY_ID";
    /// Bank Core (Layer-1) funding gate. Controls how long the harness
    /// waits at init for the bank's confirmed Core balance to become
    /// non-zero — the SPV compact-filter scan must have walked past the
    /// bank's pre-funded UTXOs before tests like CR-* / ID-007 can
    /// observe them. Unset (default) enables the gate with a
    /// [`DEFAULT_BANK_CORE_GATE_TIMEOUT`] (180s) deadline; `0` /
    /// `disabled` / `false` / `off` opt out for Platform-only suites
    /// that don't need Core duffs; any positive integer overrides the
    /// timeout (in seconds).
    pub const BANK_CORE_GATE: &str = "PLATFORM_WALLET_E2E_BANK_CORE_GATE";
    /// Operator escape hatch: when truthy (`1` / `true` / `yes` / `on`,
    /// case-insensitive), the harness skips starting the SPV runtime and
    /// the `wait_for_mn_list_synced` gate; SPV-gated case bodies (CR-001,
    /// anything asserting on `SpvRuntime` post-conditions) skip via
    /// [`super::spv_disabled_from_env`]. Use this to keep the suite making
    /// progress when testnet is in a ChainLock-cycle window blocking
    /// mn-list advance (rust-dashcore #470). Core-dependent tests
    /// (CR-003 funded-asset-lock, ID-007 Core-balance gates, any helper
    /// walking Core blocks) WILL fail when SPV is disabled.
    /// See `TEST_SPEC.md` CR-001 for the SPEC-level reference.
    pub const DISABLE_SPV: &str = "PLATFORM_WALLET_E2E_DISABLE_SPV";
    /// Period (seconds) between ticks of the harness's identity-state
    /// auto-sync. The loop calls
    /// [`refresh_identity`](platform_wallet::wallet::identity::IdentityWallet::refresh_identity)
    /// on every cached identity so `Identity::balance`,
    /// `Identity::revision`, and `Identity::public_keys` track chain
    /// reality during a test run. Unset uses
    /// [`DEFAULT_IDENTITY_SYNC_INTERVAL`] (15 s — matches production
    /// `PlatformAddressSync` / `IdentityTokenSync` / `ShieldedSync`).
    /// Non-positive / unparseable values fall back to the default with
    /// a warn.
    pub const IDENTITY_SYNC_INTERVAL_SECS: &str = "PLATFORM_WALLET_E2E_IDENTITY_SYNC_INTERVAL_SECS";
    /// Duff threshold below which the harness Platform→Core refill
    /// fallback fires at suite start (see
    /// [`super::bank_rebalance::refill_core_from_platform_if_below_threshold`]).
    /// Unset uses
    /// [`super::bank_rebalance::DEFAULT_CORE_REFILL_THRESHOLD_DUFF`].
    pub const CORE_REFILL_THRESHOLD_DUFF: &str = "PLATFORM_WALLET_E2E_CORE_REFILL_THRESHOLD_DUFF";
    /// Duff target the harness Platform→Core refill fallback aims to
    /// reach when triggered. Unset uses
    /// [`super::bank_rebalance::DEFAULT_CORE_REFILL_TARGET_DUFF`].
    pub const CORE_REFILL_TARGET_DUFF: &str = "PLATFORM_WALLET_E2E_CORE_REFILL_TARGET_DUFF";
}

/// Default cadence for the harness's identity-state auto-sync (see
/// [`vars::IDENTITY_SYNC_INTERVAL_SECS`]). Matches the production
/// `PlatformAddressSync` / `IdentityTokenSync` / `ShieldedSync` cadence;
/// 3 s previously caused DAPI overload (v36 TK-005b/TK-011 regressions).
pub const DEFAULT_IDENTITY_SYNC_INTERVAL: Duration = Duration::from_secs(15);

/// Default deadline for the bank Core funding gate when the env var is
/// unset. 180 s gives ~1.8x margin over the worst observed cold-testnet
/// success (~100 s); subsequent runs reuse the on-disk cache and clear
/// the gate in seconds.
pub const DEFAULT_BANK_CORE_GATE_TIMEOUT: Duration = Duration::from_secs(180);

/// Default minimum bank balance in credits required to start the suite.
///
/// 500M is sufficient for non-token identity tests (ID-*, CR-*, PA-*).
/// Operators who observe the "Bank under-funded" panic should top up the
/// Platform address shown in the message to at least this value.
pub const DEFAULT_MIN_BANK_CREDITS: u64 = 500_000_000;

/// Informational floor for the token test suite.
///
/// Token tests (12+ cases, 1-3 identities each) cost ~35B credits per setup.
/// When the bank balance is below this value the harness emits a `warn!` so
/// operators know a token-suite run may exhaust funds mid-way, but this
/// threshold is NOT enforced as a panic — non-token tests are unaffected.
pub const EXPECTED_TOKEN_SUITE_FLOOR: Credits = 50_000_000_000;

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
    /// the per-network default; devnet requires this override when the
    /// `Http` context-provider backend is selected.
    pub trusted_context_url: Option<String>,
    /// Resolved context-provider backend (HTTP quorums host vs SPV-backed
    /// quorum lookups). Auto-selected per network when
    /// [`vars::CONTEXT_PROVIDER`] is unset — see
    /// [`ContextProviderKind::resolve`].
    pub context_provider: ContextProviderKind,
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
    /// Operator escape hatch: when `true`, the harness skips the SPV
    /// runtime spawn and the `wait_for_mn_list_synced` gate. The bank-
    /// Core gate is auto-disabled in tandem (it polls the SPV-fed
    /// confirmed-Core balance, which would never advance). Tests that
    /// rely on Core observation will fail; Platform-only flows still
    /// run. Set via [`vars::DISABLE_SPV`].
    pub disable_spv: bool,
    /// Cadence for the harness's identity-state auto-sync. See
    /// [`vars::IDENTITY_SYNC_INTERVAL_SECS`].
    pub identity_sync_interval: Duration,
    /// Trip line (duffs) for the harness Platform→Core refill fallback.
    /// Resolved from [`vars::CORE_REFILL_THRESHOLD_DUFF`] or the default.
    pub core_refill_threshold_duff: u64,
    /// Target (duffs) for the harness Platform→Core refill fallback.
    /// Resolved from [`vars::CORE_REFILL_TARGET_DUFF`] or the default.
    pub core_refill_target_duff: u64,
}

/// Which [`dash_sdk::platform::ContextProvider`] backend the harness
/// wires for proof verification.
///
/// `Http` uses [`rs_sdk_trusted_context_provider::TrustedHttpContextProvider`]
/// against a hosted quorums service; `Spv` resolves quorum public keys
/// from the local SPV runtime's masternode list, so no quorums HTTP host
/// is required (the porter devnet has none — QA-001). Both backends still
/// serve freshly-deployed contracts / token configurations from the
/// harness's known-contracts cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextProviderKind {
    /// Trusted HTTP quorums service (`TrustedHttpContextProvider`).
    Http,
    /// SPV-backed quorum lookups; no hosted quorums host needed.
    Spv,
}

impl ContextProviderKind {
    /// Resolve the effective backend from the env-var raw value
    /// (`None` = unset) and the active network.
    ///
    /// Explicit `spv` / `http` (case-insensitive, trimmed) wins.
    /// Unset / empty / unrecognised auto-selects: networks with a
    /// hosted quorums endpoint and either a built-in URL (mainnet /
    /// testnet) or an operator-supplied `trusted_context_url` use
    /// `Http`; everything else (devnet / regtest without a trusted URL,
    /// e.g. porter) uses `Spv`, because there is no quorums host to
    /// point at and constructing `TrustedHttpContextProvider` would die
    /// on the bogus host (QA-001).
    fn resolve(raw: Option<&str>, network: Network, has_trusted_url: bool) -> Self {
        if let Some(raw) = raw {
            match raw.trim().to_ascii_lowercase().as_str() {
                "spv" => return Self::Spv,
                "http" => return Self::Http,
                "" => {}
                other => tracing::warn!(
                    target: "platform_wallet::e2e::config",
                    var = vars::CONTEXT_PROVIDER,
                    value = %other,
                    "unrecognised context-provider selector; auto-selecting per network"
                ),
            }
        }
        match network {
            Network::Mainnet | Network::Testnet => Self::Http,
            _ if has_trusted_url => Self::Http,
            _ => Self::Spv,
        }
    }
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
            .field("context_provider", &self.context_provider)
            .field("p2p_port", &self.p2p_port)
            .field("bank_identity_id", &self.bank_identity_id)
            .field("bank_core_gate_timeout", &self.bank_core_gate_timeout)
            .field("bank_core_gate_source", &self.bank_core_gate_source)
            .field("disable_spv", &self.disable_spv)
            .field("identity_sync_interval", &self.identity_sync_interval)
            .field(
                "core_refill_threshold_duff",
                &self.core_refill_threshold_duff,
            )
            .field("core_refill_target_duff", &self.core_refill_target_duff)
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
            context_provider: ContextProviderKind::resolve(None, network, false),
            p2p_port: default_p2p_port(network),
            bank_identity_id: None,
            bank_core_gate_timeout: Some(DEFAULT_BANK_CORE_GATE_TIMEOUT),
            bank_core_gate_source: BankCoreGateSource::Default,
            disable_spv: false,
            identity_sync_interval: DEFAULT_IDENTITY_SYNC_INTERVAL,
            core_refill_threshold_duff: super::bank_rebalance::DEFAULT_CORE_REFILL_THRESHOLD_DUFF,
            core_refill_target_duff: super::bank_rebalance::DEFAULT_CORE_REFILL_TARGET_DUFF,
        }
    }
}

/// Walk up from `start` looking for a `.claude` path component; if found,
/// the parent of that component is the parent-repo root. Returns the
/// `tests/.env` path under `packages/rs-platform-wallet/` in that root,
/// or `/dev/null` (which never passes `.exists()`) when not found.
fn find_parent_repo_env(start: &std::path::Path) -> PathBuf {
    for ancestor in start.ancestors() {
        let components: Vec<_> = ancestor.components().collect();
        if let Some(idx) = components.iter().position(|c| c.as_os_str() == ".claude") {
            let parent_root: PathBuf = components[..idx].iter().collect();
            let candidate = parent_root.join("packages/rs-platform-wallet/tests/.env");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("/dev/null")
}

/// Try each candidate path in order; load the first one that exists.
fn load_e2e_env() {
    let manifest_env = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/.env");
    let parent_env = find_parent_repo_env(Path::new(env!("CARGO_MANIFEST_DIR")));

    for candidate in [&manifest_env, &parent_env] {
        if candidate.exists() {
            match dotenvy::from_path(candidate) {
                Ok(()) => {
                    tracing::info!(
                        target: "platform_wallet::e2e::config",
                        path = %candidate.display(),
                        "loaded e2e .env"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        target: "platform_wallet::e2e::config",
                        path = %candidate.display(),
                        ?err,
                        "failed to load e2e .env (process env vars still apply)"
                    );
                }
            }
            return;
        }
    }

    tracing::warn!(
        target: "platform_wallet::e2e::config",
        "no e2e .env found in any candidate location (process env vars still apply)"
    );
}

impl Config {
    /// Load from environment variables, with `.env` at
    /// `${CARGO_MANIFEST_DIR}/tests/.env` as a CWD-independent
    /// fallback. `bank_mnemonic` is required; everything else
    /// resolves to its final value via the per-field defaults.
    pub fn from_env() -> FrameworkResult<Self> {
        load_e2e_env();

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

        let context_provider = ContextProviderKind::resolve(
            std::env::var(vars::CONTEXT_PROVIDER).ok().as_deref(),
            network,
            trusted_context_url.is_some(),
        );

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

        let disable_spv = parse_truthy(std::env::var(vars::DISABLE_SPV).ok().as_deref());

        let identity_sync_interval = parse_identity_sync_interval(
            std::env::var(vars::IDENTITY_SYNC_INTERVAL_SECS)
                .ok()
                .as_deref(),
        );

        let core_refill_threshold_duff = parse_u64_duff_var(
            vars::CORE_REFILL_THRESHOLD_DUFF,
            super::bank_rebalance::DEFAULT_CORE_REFILL_THRESHOLD_DUFF,
        );
        let core_refill_target_duff = parse_u64_duff_var(
            vars::CORE_REFILL_TARGET_DUFF,
            super::bank_rebalance::DEFAULT_CORE_REFILL_TARGET_DUFF,
        );

        Ok(Self {
            bank_mnemonic,
            network,
            dapi_addresses,
            min_bank_credits,
            workdir_base,
            trusted_context_url,
            context_provider,
            p2p_port,
            bank_identity_id,
            bank_core_gate_timeout,
            bank_core_gate_source,
            disable_spv,
            identity_sync_interval,
            core_refill_threshold_duff,
            core_refill_target_duff,
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

/// Resolve a u64 (duff-denominated) env var with a fallback default.
/// Unset / empty / unparseable values fall back to `default` with a
/// `warn` so an operator's fat-fingered override isn't silently
/// ignored.
pub(crate) fn parse_u64_duff_var(var: &'static str, default: u64) -> u64 {
    match std::env::var(var) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return default;
            }
            match trimmed.parse::<u64>() {
                Ok(value) => value,
                Err(err) => {
                    tracing::warn!(
                        target: "platform_wallet::e2e::config",
                        var = var,
                        value = %raw,
                        ?err,
                        default,
                        "could not parse duff env var as u64; falling back to default"
                    );
                    default
                }
            }
        }
        Err(_) => default,
    }
}

/// Parse a boolean opt-in flag from a raw env-var value (`None` = unset).
///
/// Truthy: `1`, `true`, `yes`, `on` (case-insensitive, trimmed).
/// Everything else — including empty / unset / unparseable — is `false`.
/// Used by [`vars::DISABLE_SPV`].
pub(crate) fn parse_truthy(raw: Option<&str>) -> bool {
    let Some(raw) = raw else { return false };
    let trimmed = raw.trim();
    trimmed == "1"
        || trimmed.eq_ignore_ascii_case("true")
        || trimmed.eq_ignore_ascii_case("yes")
        || trimmed.eq_ignore_ascii_case("on")
}

/// Resolve the identity-sync interval from a raw env-var value.
///
/// - unset / empty / whitespace → [`DEFAULT_IDENTITY_SYNC_INTERVAL`]
/// - positive integer → `Duration::from_secs(n)`
/// - `0` / negative / unparseable → default, with a `warn` so operators
///   know their override was ignored. Zero would tight-loop the sync;
///   forcing a positive minimum keeps a fat-finger from melting CI.
pub(crate) fn parse_identity_sync_interval(raw: Option<&str>) -> Duration {
    let Some(raw) = raw else {
        return DEFAULT_IDENTITY_SYNC_INTERVAL;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return DEFAULT_IDENTITY_SYNC_INTERVAL;
    }
    match trimmed.parse::<u64>() {
        Ok(0) => {
            tracing::warn!(
                target: "platform_wallet::e2e::config",
                var = vars::IDENTITY_SYNC_INTERVAL_SECS,
                value = %raw,
                default_secs = DEFAULT_IDENTITY_SYNC_INTERVAL.as_secs(),
                "identity-sync interval of 0 would tight-loop the sync; using default"
            );
            DEFAULT_IDENTITY_SYNC_INTERVAL
        }
        Ok(secs) => Duration::from_secs(secs),
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::config",
                var = vars::IDENTITY_SYNC_INTERVAL_SECS,
                value = %raw,
                ?err,
                default_secs = DEFAULT_IDENTITY_SYNC_INTERVAL.as_secs(),
                "could not parse identity-sync interval; falling back to default"
            );
            DEFAULT_IDENTITY_SYNC_INTERVAL
        }
    }
}

/// Returns `true` when [`vars::DISABLE_SPV`] is set to a truthy value
/// (`1` / `true` / `yes` / `on`, case-insensitive, surrounding
/// whitespace ignored). Any other value — including unset, empty, or
/// unrecognised — returns `false`.
///
/// SPV-gated cases (e.g. CR-001) call this at the top of the test body
/// and `return` early when it reports `true`, so the operator can opt
/// out of SPV-only assertions without burning the cold-cache timeout.
/// The harness reads the same flag in `E2eContext::build` to skip
/// starting the SPV runtime altogether.
pub fn spv_disabled_from_env() -> bool {
    is_truthy_env(vars::DISABLE_SPV)
}

/// Truthy-env helper shared by SPV-style boolean flags. Reads `key`
/// from the process environment and returns `true` for `1` / `true` /
/// `yes` / `on` (case-insensitive, trimmed); everything else — unset,
/// empty, or unrecognised — returns `false`.
fn is_truthy_env(key: &str) -> bool {
    matches!(
        std::env::var(key).ok().as_deref().map(str::trim),
        Some(v) if v == "1"
            || v.eq_ignore_ascii_case("true")
            || v.eq_ignore_ascii_case("yes")
            || v.eq_ignore_ascii_case("on")
    )
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
    fn bank_core_gate_unset_defaults_to_180s() {
        let (timeout, src) = parse_bank_core_gate(None);
        assert_eq!(timeout, Some(DEFAULT_BANK_CORE_GATE_TIMEOUT));
        assert_eq!(src, BankCoreGateSource::Default);
    }

    #[test]
    fn bank_core_gate_empty_string_defaults_to_180s() {
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
    fn context_provider_explicit_selectors_win() {
        for net in [Network::Testnet, Network::Devnet, Network::Regtest] {
            assert_eq!(
                ContextProviderKind::resolve(Some("spv"), net, true),
                ContextProviderKind::Spv,
                "explicit spv on {net:?}"
            );
            assert_eq!(
                ContextProviderKind::resolve(Some("  HTTP "), net, false),
                ContextProviderKind::Http,
                "explicit http on {net:?}"
            );
        }
    }

    #[test]
    fn context_provider_auto_select_per_network() {
        // Built-in-quorum networks default to HTTP regardless of URL.
        assert_eq!(
            ContextProviderKind::resolve(None, Network::Testnet, false),
            ContextProviderKind::Http
        );
        assert_eq!(
            ContextProviderKind::resolve(None, Network::Mainnet, false),
            ContextProviderKind::Http
        );
        // Devnet with a trusted URL → HTTP; without one → SPV (porter).
        assert_eq!(
            ContextProviderKind::resolve(None, Network::Devnet, true),
            ContextProviderKind::Http
        );
        assert_eq!(
            ContextProviderKind::resolve(None, Network::Devnet, false),
            ContextProviderKind::Spv
        );
        assert_eq!(
            ContextProviderKind::resolve(None, Network::Regtest, false),
            ContextProviderKind::Spv
        );
    }

    #[test]
    fn context_provider_unrecognised_falls_back_to_auto() {
        // Garbage selector behaves as "unset" → per-network auto-select.
        assert_eq!(
            ContextProviderKind::resolve(Some("nonsense"), Network::Devnet, false),
            ContextProviderKind::Spv
        );
        assert_eq!(
            ContextProviderKind::resolve(Some(""), Network::Testnet, false),
            ContextProviderKind::Http
        );
    }

    #[test]
    fn disable_spv_unset_is_false() {
        assert!(!parse_truthy(None));
    }

    #[test]
    fn disable_spv_truthy_aliases() {
        for raw in [
            "1", "true", "TRUE", "True", "yes", "YES", "on", "ON", "  true  ",
        ] {
            assert!(parse_truthy(Some(raw)), "{raw}");
        }
    }

    #[test]
    fn disable_spv_falsy_or_unparseable_is_false() {
        for raw in ["", "  ", "0", "false", "no", "off", "disabled", "abc"] {
            assert!(!parse_truthy(Some(raw)), "{raw}");
        }
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

    #[test]
    fn find_parent_repo_env_no_claude_component_returns_dev_null() {
        let result = find_parent_repo_env(std::path::Path::new("/usr/local/bin"));
        assert_eq!(result, PathBuf::from("/dev/null"));
    }

    #[test]
    fn find_parent_repo_env_with_claude_in_path_returns_candidate() {
        use std::io::Write;

        let tmp = tempfile::tempdir().expect("tempdir");
        // Build a fake parent-repo tree under tmp: .claude/worktrees/agent-X/packages/...
        let worktree_pkg = tmp
            .path()
            .join(".claude/worktrees/agent-test/packages/rs-platform-wallet");
        std::fs::create_dir_all(&worktree_pkg).expect("create dirs");

        // Create the parent-repo tests/.env that the function should find.
        let parent_tests_env = tmp.path().join("packages/rs-platform-wallet/tests/.env");
        std::fs::create_dir_all(parent_tests_env.parent().unwrap()).expect("create dirs");
        std::fs::File::create(&parent_tests_env)
            .expect("create .env")
            .write_all(b"TEST=1\n")
            .expect("write .env");

        let result = find_parent_repo_env(&worktree_pkg);
        assert_eq!(result, parent_tests_env);
    }

    #[test]
    fn find_parent_repo_env_claude_present_but_no_env_file_returns_dev_null() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let worktree_pkg = tmp
            .path()
            .join(".claude/worktrees/agent-test/packages/rs-platform-wallet");
        std::fs::create_dir_all(&worktree_pkg).expect("create dirs");
        // No .env file created — should fall through to /dev/null.

        let result = find_parent_repo_env(&worktree_pkg);
        assert_eq!(result, PathBuf::from("/dev/null"));
    }

    /// Process-wide env-var flag used to exercise [`is_truthy_env`].
    /// Distinct from any production var so cargo-test parallelism with
    /// the `from_env` callers can never collide. The truthy/falsy
    /// matrix is exercised in a single test so the two halves don't
    /// race over the same key under parallel cargo-test execution.
    const TRUTHY_PROBE_VAR: &str = "PLATFORM_WALLET_E2E_TEST_TRUTHY_PROBE";

    #[test]
    fn identity_sync_unset_defaults_to_15s() {
        assert_eq!(
            parse_identity_sync_interval(None),
            DEFAULT_IDENTITY_SYNC_INTERVAL
        );
    }

    #[test]
    fn identity_sync_positive_integer_overrides() {
        assert_eq!(
            parse_identity_sync_interval(Some("10")),
            Duration::from_secs(10)
        );
        assert_eq!(
            parse_identity_sync_interval(Some("  60  ")),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn identity_sync_zero_falls_back_to_default() {
        assert_eq!(
            parse_identity_sync_interval(Some("0")),
            DEFAULT_IDENTITY_SYNC_INTERVAL
        );
    }

    #[test]
    fn identity_sync_invalid_falls_back_to_default() {
        for raw in ["", "  ", "abc", "-1", "1.5"] {
            assert_eq!(
                parse_identity_sync_interval(Some(raw)),
                DEFAULT_IDENTITY_SYNC_INTERVAL,
                "{raw}"
            );
        }
    }

    #[test]
    fn is_truthy_env_matrix() {
        // SAFETY: single-threaded — the probe key is unique to this
        // test, so no parallel test can mutate it underneath us.
        std::env::remove_var(TRUTHY_PROBE_VAR);
        assert!(!is_truthy_env(TRUTHY_PROBE_VAR), "unset must be falsy");

        for raw in [
            "1", "true", "TRUE", "True", "yes", "Yes", "YES", "on", "ON", " on ", "  1\t",
        ] {
            std::env::set_var(TRUTHY_PROBE_VAR, raw);
            assert!(
                is_truthy_env(TRUTHY_PROBE_VAR),
                "{raw:?} should be recognised as truthy"
            );
        }

        for raw in ["", " ", "0", "false", "no", "off", "disabled", "abc"] {
            std::env::set_var(TRUTHY_PROBE_VAR, raw);
            assert!(
                !is_truthy_env(TRUTHY_PROBE_VAR),
                "{raw:?} must NOT be recognised as truthy"
            );
        }
        std::env::remove_var(TRUTHY_PROBE_VAR);
    }
}
