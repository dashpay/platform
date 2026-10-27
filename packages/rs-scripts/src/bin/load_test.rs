//! Sustained document-throughput load generator for Dash Platform.
//!
//! Drives document-create state transitions from a single funded
//! identity, fanned out across N cloned data contracts so that
//! concurrency is not bounded by per-`(identity, contract)` nonce
//! contention. Intended for consensus stress measurement (block
//! interval / throughput / round stability), not as a max-TPS
//! benchmark: pick a `--rate` that keeps blocks consistently
//! non-empty without the client becoming the bottleneck, and hold it.
//!
//! Bootstrap (registering + funding the identity, an L1 asset-lock
//! step) is out of scope for this bin — it takes an already-funded
//! identity id + one of its private keys and only talks to DAPI.
//!
//! The base contract is cloned into `--contracts` on-chain variants;
//! each successive `DataContractCreate` gets a fresh identity nonce,
//! so the variants get distinct ids. Documents are then round-robined
//! across the variants (`--contracts >= --connections`). Per-contract
//! nonce *uniqueness* is guaranteed by the SDK's shared
//! per-`(identity, contract)` nonce cache, which hands out monotonic
//! nonces even to concurrent creates on the same variant; the fan-out
//! instead keeps each contract's in-flight, not-yet-committed nonces
//! well under the consensus missing-revisions window, so fire-and-forget
//! creates are not rejected for running too far ahead of the committed
//! nonce.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::Parser;
use dash_sdk::platform::transition::put_contract::PutContract;
use dash_sdk::platform::transition::put_document::PutDocument;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::Fetch;
use dash_sdk::{Sdk, SdkBuilder};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::document_type::DocumentType;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{Bytes32, Identifier};
use dpp::prelude::DataContract;
use platform_version::version::PlatformVersion;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rs_dapi_client::{Address, AddressList, RequestSettings};
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use simple_signer::single_key_signer::SingleKeySigner;
use std::num::NonZeroUsize;

#[derive(Parser, Debug)]
#[command(
    name = "load-test",
    about = "Sustained document-throughput load generator for Dash Platform.\n\
             Registers N contract variants owned by --identity, then broadcasts\n\
             random documents across them for --time seconds at up to --rate/sec."
)]
struct Args {
    /// Comma-separated DAPI address(es), e.g.
    /// https://1.2.3.4:1443,https://5.6.7.8:1443. Multiple addresses
    /// spread connections across nodes.
    #[arg(short = 'a', long = "address")]
    address: String,

    /// Network: mainnet | testnet | devnet | regtest.
    #[arg(short = 'n', long = "network", default_value = "testnet")]
    network: String,

    /// Devnet name (only when --network devnet), e.g. moutai.
    #[arg(long = "devnet")]
    devnet_name: Option<String>,

    /// Core RPC host — the SDK uses it to fetch quorum public keys for
    /// proof verification (this devnet has no HTTP quorum endpoint).
    #[arg(long = "core-host", default_value = "127.0.0.1")]
    core_host: String,

    /// Core RPC port.
    #[arg(long = "core-port", default_value = "20002")]
    core_port: u16,

    /// Core RPC username.
    #[arg(long = "core-user", default_value = "dashrpc")]
    core_user: String,

    /// Core RPC password. Prefer the CORE_RPC_PASSWORD env var.
    #[arg(long = "core-password")]
    core_password: Option<String>,

    /// External-client mode: fetch quorum public keys from the
    /// network's trusted HTTP endpoint (e.g. testnet/mainnet) instead
    /// of Core RPC, so no --core-* connection is needed.
    #[arg(long = "quorum-http")]
    quorum_http: bool,

    /// Identity id (base58) that owns the load. Must be registered
    /// and funded with enough credits for the whole run.
    #[arg(short = 'i', long = "identity")]
    identity_id: String,

    /// Private key for that identity — WIF or 64-char hex. Must be a
    /// CRITICAL + AUTHENTICATION + ECDSA_SECP256K1 key (required to
    /// sign contract creates; also valid for document creates).
    #[arg(short = 'k', long = "private-key")]
    private_key: String,

    /// Path to the base contract JSON (its `id`/`ownerId` are
    /// overridden). Prefer a contract with a cheap, non-unique-index
    /// document type so documents are freely spammable.
    #[arg(short = 'c', long = "contract")]
    contract_file: PathBuf,

    /// Document type within the contract to broadcast. Defaults to the
    /// contract's first document type.
    #[arg(short = 'd', long = "doc-type")]
    doc_type: Option<String>,

    /// Number of contract variants to register = parallel nonce lanes.
    /// Must be >= --connections.
    #[arg(long = "contracts", default_value = "32")]
    contracts: u32,

    /// Target document broadcast rate (documents per second).
    #[arg(short = 'r', long = "rate", default_value = "5")]
    rate: f64,

    /// Maximum concurrent in-flight document requests.
    #[arg(long = "connections", default_value = "4")]
    connections: usize,

    /// Steady-state duration in seconds.
    #[arg(short = 't', long = "time", default_value = "300")]
    time: u64,

    /// Build the SDK, fetch the identity and base contract, then exit
    /// without registering contracts or broadcasting — a connectivity
    /// and credentials check.
    #[arg(long = "dry-run")]
    dry_run: bool,
}

fn parse_network(s: &str) -> Result<Network, String> {
    match s.to_ascii_lowercase().as_str() {
        "mainnet" | "main" => Ok(Network::Mainnet),
        "testnet" | "test" => Ok(Network::Testnet),
        "devnet" | "dev" => Ok(Network::Devnet),
        "regtest" => Ok(Network::Regtest),
        other => Err(format!(
            "unknown network '{other}' (expected mainnet | testnet | devnet | regtest)"
        )),
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let args = Args::parse();

    // Fail loud on degenerate inputs rather than hanging or panicking later,
    // after contract variants have already been registered (and paid for):
    // --connections 0 would block forever on a zero-permit semaphore; a
    // non-finite or too-high --rate produces a zero tick period that panics
    // tokio's interval; --time 0 would register contracts then broadcast
    // nothing and divide by zero when reporting.
    if args.connections < 1 {
        return Err("--connections must be >= 1".to_string());
    }
    if args.contracts < 1 {
        return Err("--contracts must be >= 1".to_string());
    }
    if args.time == 0 {
        return Err("--time must be >= 1 (seconds)".to_string());
    }
    // Guard against a --time so large that computing the run deadline
    // (Instant::now() + duration) would overflow the monotonic clock and panic.
    if Instant::now()
        .checked_add(Duration::from_secs(args.time))
        .is_none()
    {
        return Err("--time is too large to represent as a run deadline".to_string());
    }
    if !args.rate.is_finite() || args.rate <= 0.0 {
        return Err("--rate must be a positive, finite number".to_string());
    }
    // try_from_secs_f64 (unlike from_secs_f64) returns Err instead of panicking
    // when the interval is out of Duration's range, so this catches both a
    // too-high rate (interval rounds to zero) and a too-low rate (interval
    // overflows Duration) without the check itself ever panicking.
    match Duration::try_from_secs_f64(1.0 / args.rate) {
        Ok(period) if !period.is_zero() => {}
        _ => {
            return Err(format!(
                "--rate {} is outside the usable range (its inter-broadcast \
                 interval is not a positive, representable duration)",
                args.rate
            ))
        }
    }
    if args.contracts < args.connections as u32 {
        return Err(format!(
            "--contracts ({}) must be >= --connections ({}): with fewer contract \
             variants than concurrent requests, a variant's in-flight nonces pile \
             up and risk exceeding the consensus missing-revisions window",
            args.contracts, args.connections
        ));
    }

    let network = parse_network(&args.network)?;
    let platform_version = PlatformVersion::latest();

    let identity_id = Identifier::from_string(&args.identity_id, Encoding::Base58)
        .map_err(|e| format!("invalid --identity (expected base58): {e}"))?;

    let signer = SingleKeySigner::from_string(args.private_key.trim(), network)
        .map_err(|e| format!("invalid --private-key: {e}"))?;

    // Load and prepare the base contract. full_validation = false:
    // the fixture may carry an id/ownerId we're about to overwrite;
    // the on-chain state-transition validation still runs server-side.
    let json_bytes = std::fs::read(&args.contract_file)
        .map_err(|e| format!("failed to read {}: {e}", args.contract_file.display()))?;
    let json_value: serde_json::Value = serde_json::from_slice(&json_bytes).map_err(|e| {
        format!(
            "failed to parse {} as JSON: {e}",
            args.contract_file.display()
        )
    })?;
    let mut base_contract = DataContract::from_json(json_value, false, platform_version)
        .map_err(|e| format!("failed to build DataContract from JSON: {e}"))?;
    base_contract.set_owner_id(identity_id);
    // Old fixtures carry a v0 contract config, which Drive 4.x rejects
    // ("minimum version is 1"). Replace it with the current default.
    base_contract.set_config(
        DataContractConfig::default_for_version(platform_version)
            .map_err(|e| format!("failed to build contract config: {e}"))?,
    );

    let doc_type_name = match &args.doc_type {
        Some(name) => name.clone(),
        None => base_contract
            .document_types()
            .keys()
            .next()
            .cloned()
            .ok_or_else(|| "contract has no document types".to_string())?,
    };
    // Validate the chosen document type exists on the base contract.
    base_contract
        .document_type_cloned_for_name(&doc_type_name)
        .map_err(|e| format!("document type '{doc_type_name}' not found in contract: {e}"))?;

    // Build the SDK against one or more DAPI nodes.
    let addresses = args
        .address
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<Address>()
                .map_err(|e| format!("failed to parse address '{s}': {e}"))
        })
        .collect::<Result<Vec<Address>, String>>()?;
    if addresses.is_empty() {
        return Err("no DAPI addresses provided in --address".to_string());
    }
    let address_list = AddressList::from_iter(addresses);

    // Quorum public keys (for proof verification) come from one of two
    // sources. External-client mode (--quorum-http) fetches them from the
    // network's trusted HTTP endpoint (e.g. quorums.testnet.networks.dash.org),
    // needing no Core RPC. Otherwise (devnets with no such endpoint) they
    // come from Core RPC via --with-core.
    let sdk: Sdk = if args.quorum_http {
        let context_provider = TrustedHttpContextProvider::new(
            network,
            args.devnet_name.clone(),
            NonZeroUsize::new(100).expect("non-zero cache size"),
        )
        .map_err(|e| format!("failed to build HTTP context provider: {e}"))?;
        SdkBuilder::new(address_list)
            .with_network(network)
            .with_context_provider(context_provider)
            .build()
            .map_err(|e| format!("failed to build SDK: {e}"))?
    } else {
        let core_password = std::env::var("CORE_RPC_PASSWORD")
            .ok()
            .or(args.core_password.clone())
            .ok_or_else(|| {
                "Core RPC password required: set CORE_RPC_PASSWORD or pass --core-password \
                 (or use --quorum-http for an external client)"
                    .to_string()
            })?;
        SdkBuilder::new(address_list)
            .with_network(network)
            .with_core(
                &args.core_host,
                args.core_port,
                &args.core_user,
                &core_password,
            )
            .build()
            .map_err(|e| format!("failed to build SDK: {e}"))?
    };

    eprintln!("Fetching identity {}...", args.identity_id);
    let identity = Identity::fetch(&sdk, identity_id)
        .await
        .map_err(|e| format!("failed to fetch identity: {e}"))?
        .ok_or_else(|| format!("identity {} not found", args.identity_id))?;

    let signing_key = select_signing_key(&identity, &signer)?;
    eprintln!(
        "Using key id {} (purpose={:?}, security_level={:?}, key_type={:?}); identity balance {} credits",
        signing_key.id(),
        signing_key.purpose(),
        signing_key.security_level(),
        signing_key.key_type(),
        identity.balance(),
    );

    if args.dry_run {
        eprintln!(
            "Dry run OK: connected to DAPI, fetched identity, base contract '{}' parsed with \
             document type '{}'. Exiting without broadcasting.",
            args.contract_file.display(),
            doc_type_name,
        );
        return Ok(());
    }

    // Register the contract variants (the parallel nonce lanes).
    let doc_types = register_contract_variants(
        &sdk,
        &base_contract,
        &doc_type_name,
        &signing_key,
        &signer,
        args.contracts,
    )
    .await?;

    // Broadcast documents across the variants for the run duration.
    run_document_load(
        &sdk,
        identity_id,
        signing_key,
        &signer,
        doc_types,
        args.rate,
        args.connections,
        Duration::from_secs(args.time),
        platform_version,
    )
    .await;

    Ok(())
}

/// Register `count` clones of the base contract, returning the
/// document type (with its real, on-chain contract id) for each.
///
/// Contract creates share the single per-identity nonce, so they are
/// issued serially — each waits for its confirmation before the next.
async fn register_contract_variants(
    sdk: &Sdk,
    base_contract: &DataContract,
    doc_type_name: &str,
    signing_key: &IdentityPublicKey,
    signer: &SingleKeySigner,
    count: u32,
) -> Result<Vec<DocumentType>, String> {
    eprintln!("Registering {count} contract variant(s)...");
    let mut doc_types = Vec::with_capacity(count as usize);

    for i in 0..count {
        // The chain can be transiently unavailable (a validator
        // restarting, or a momentary "Tenderdash is not available"),
        // which fails the broadcast without consuming the identity
        // nonce. Retry a few times so a single blip doesn't abort the
        // whole bootstrap.
        let mut attempt = 0;
        let confirmed = loop {
            attempt += 1;
            match base_contract
                .clone()
                .put_to_platform_and_wait_for_response(sdk, signing_key.clone(), signer, None)
                .await
            {
                Ok(contract) => break contract,
                Err(e) if attempt < 8 => {
                    eprintln!("  variant {i} attempt {attempt} failed ({e}); retrying in 4s");
                    tokio::time::sleep(Duration::from_secs(4)).await;
                }
                Err(e) => {
                    return Err(format!(
                        "failed to register contract variant {i} after {attempt} attempts: {e}"
                    ))
                }
            }
        };

        let doc_type = confirmed
            .document_type_cloned_for_name(doc_type_name)
            .map_err(|e| {
                format!(
                    "variant {i} ({}) lacks document type '{doc_type_name}': {e}",
                    confirmed.id()
                )
            })?;
        doc_types.push(doc_type);

        if (i + 1) % 8 == 0 || i + 1 == count {
            eprintln!("  registered {}/{} variants", i + 1, count);
        }
    }

    Ok(doc_types)
}

/// Broadcast random documents round-robin across the contract variants
/// for `duration`, paced to `rate` per second and capped at
/// `connections` concurrent in-flight requests.
#[allow(clippy::too_many_arguments)]
async fn run_document_load(
    sdk: &Sdk,
    owner_id: Identifier,
    signing_key: IdentityPublicKey,
    signer: &SingleKeySigner,
    doc_types: Vec<DocumentType>,
    rate: f64,
    connections: usize,
    duration: Duration,
    platform_version: &'static PlatformVersion,
) {
    let deadline = Instant::now() + duration;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(connections));

    let oks = Arc::new(AtomicUsize::new(0));
    let errs = Arc::new(AtomicUsize::new(0));
    let last_report_secs = Arc::new(AtomicU64::new(0));
    let start = Instant::now();

    // Fire-and-forget: broadcast to mempool (do not wait for block
    // proof) so throughput isn't gated on block time; retries off so
    // the measured rate reflects real acceptance, not client retrying.
    let put_settings = PutSettings {
        request_settings: RequestSettings {
            connect_timeout: Some(Duration::from_secs(30)),
            timeout: Some(Duration::from_secs(30)),
            retries: Some(0),
            ban_failed_address: Some(false),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut ticker = tokio::time::interval(Duration::from_secs_f64(1.0 / rate));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    eprintln!(
        "Broadcasting up to {rate:.2} docs/s across {} variants, {} concurrent, for {} s...",
        doc_types.len(),
        connections,
        duration.as_secs()
    );

    let mut lane: usize = 0;
    while Instant::now() < deadline {
        ticker.tick().await;

        // Cap in-flight requests; this also backpressures the tick loop
        // when the network is slower than the target rate, so the
        // effective rate honestly reflects the bottleneck.
        let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();

        let doc_type = doc_types[lane % doc_types.len()].clone();
        lane = lane.wrapping_add(1);

        let sdk = sdk.clone();
        let signer = signer.clone();
        let signing_key = signing_key.clone();
        let oks = Arc::clone(&oks);
        let errs = Arc::clone(&errs);
        let last_report_secs = Arc::clone(&last_report_secs);

        tokio::spawn(async move {
            let _permit = permit;
            let mut rng = StdRng::from_entropy();
            let entropy: [u8; 32] = rng.gen();
            let time_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let document = match doc_type.random_document_with_params(
                owner_id,
                Bytes32::from(entropy),
                Some(time_ms),
                None,
                None,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                &mut rng,
                platform_version,
            ) {
                Ok(doc) => doc,
                Err(e) => {
                    eprintln!("failed to generate random document: {e}");
                    errs.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            let result = document
                .put_to_platform(
                    &sdk,
                    doc_type,
                    Some(entropy),
                    signing_key,
                    None,
                    &signer,
                    Some(put_settings),
                )
                .await;

            match result {
                Ok(_) => {
                    oks.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    errs.fetch_add(1, Ordering::Relaxed);
                    eprintln!("broadcast failed: {e}");
                }
            }

            // Roughly every 10 s, one task prints a progress line.
            let elapsed = start.elapsed().as_secs();
            if elapsed.is_multiple_of(10)
                && last_report_secs.swap(elapsed, Ordering::Relaxed) != elapsed
            {
                eprintln!(
                    "{elapsed}s: {} ok, {} err",
                    oks.load(Ordering::Relaxed),
                    errs.load(Ordering::Relaxed),
                );
            }
        });
    }

    // Drain: acquire every permit, which is only possible once all
    // in-flight requests have completed and released theirs.
    let _ = semaphore
        .acquire_many_owned(connections as u32)
        .await
        .unwrap();

    let ok = oks.load(Ordering::Relaxed);
    let err = errs.load(Ordering::Relaxed);
    // Report measured wall-clock (includes the drain past the deadline), and
    // base accepted throughput on successful acceptances only — failed
    // broadcasts are not landed documents and must not inflate the headline.
    let elapsed = start.elapsed().as_secs_f64();
    eprintln!(
        "Done: {} attempted ({} ok, {} err) over {:.0}s = {:.2} docs/s accepted \
         (broadcast/mempool acceptance, not on-chain commit; target {:.2}).",
        ok + err,
        ok,
        err,
        elapsed,
        ok as f64 / elapsed,
        rate,
    );
}

/// Find the identity's first key that (1) the supplied private key can
/// sign with and (2) meets the contract-create requirement of
/// AUTHENTICATION + CRITICAL + ECDSA_SECP256K1. A CRITICAL key also
/// satisfies document creates (which accept CRITICAL/HIGH/MEDIUM).
fn select_signing_key(
    identity: &Identity,
    signer: &SingleKeySigner,
) -> Result<IdentityPublicKey, String> {
    let mut matched_but_unusable: Vec<&IdentityPublicKey> = Vec::new();

    for public_key in identity.public_keys().values() {
        if !signer.can_sign_with(public_key) {
            continue;
        }
        if public_key.is_disabled() {
            matched_but_unusable.push(public_key);
            continue;
        }
        if public_key.purpose() == Purpose::AUTHENTICATION
            && public_key.security_level() == SecurityLevel::CRITICAL
            && public_key.key_type() == KeyType::ECDSA_SECP256K1
        {
            return Ok(public_key.clone());
        }
        matched_but_unusable.push(public_key);
    }

    if matched_but_unusable.is_empty() {
        Err(format!(
            "private key does not match any public key on identity {}",
            identity.id()
        ))
    } else {
        let details = matched_but_unusable
            .iter()
            .map(|pk| {
                format!(
                    "  id={} purpose={:?} security_level={:?} key_type={:?} disabled={}",
                    pk.id(),
                    pk.purpose(),
                    pk.security_level(),
                    pk.key_type(),
                    pk.is_disabled()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        Err(format!(
            "private key matched a key on identity {} but no key meets the\n\
             requirement (AUTHENTICATION + CRITICAL + ECDSA_SECP256K1).\n\
             Matched keys:\n{}",
            identity.id(),
            details
        ))
    }
}
