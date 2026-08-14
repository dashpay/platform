//! Bootstrap a funded Platform identity from a Dash Core faucet wallet,
//! using a **ChainLock** asset-lock proof.
//!
//! It generates a Platform identity plus a one-time asset-lock key,
//! builds an asset-lock funding transaction from the faucet wallet's
//! UTXOs, has Core sign and broadcast it, obtains a ChainLock-based
//! asset-lock proof, then registers the identity and reports its id.
//!
//! ## This supersedes an earlier InstantSend-based approach — and why that one loses money
//!
//! A previous version of this tool proved the asset lock with an InstantSend
//! lock instead of a ChainLock. **That does not work against rs-dapi and burns
//! the funds on every run.** The InstantSend proof is obtained by subscribing to
//! DAPI's transaction stream filtered on the one-time asset-lock address — but
//! that address exists only inside the asset-lock special-transaction payload,
//! which DAPI's stream matcher (`matches_transaction`) never inspects. So the
//! lock is never forwarded, the wait times out **after the funding transaction
//! is already broadcast and on-chain**, and the DASH is stranded on a one-time
//! key that is then discarded. Do not reintroduce the InstantSend path here;
//! getting this wrong is a silent, unrecoverable loss of funds. The mechanism
//! and the ChainLock alternative are detailed below.
//!
//! # Why ChainLock and not InstantSend
//!
//! An asset lock can be proven to Platform two ways: an InstantSend lock
//! on the funding transaction, or a ChainLock over the block that
//! contains it. This tool uses ChainLock, on purpose.
//!
//! The InstantSend path relies on subscribing to DAPI's transaction
//! stream filtered on the one-time asset-lock address. That address
//! exists only in the asset-lock *special-transaction payload*
//! (`credit_outputs`), and DAPI's stream bloom-matcher
//! (`matches_transaction`) inspects only the transaction's regular
//! inputs, output scripts, and txid — never the special-tx payload. So
//! the InstantSend lock for an asset-lock funding transaction is never
//! forwarded to the subscriber. The wait then blocks until it times out,
//! **after the funding transaction is already on-chain** — the DASH is
//! locked to a one-time key the caller then discards, and it is gone.
//! This is not hypothetical: it is the confirmed failure mode of the
//! older InstantSend-based funding bin against current rs-dapi, and it
//! burns the funds on every run.
//!
//! ChainLock sidesteps the stream entirely: after broadcast we poll Core
//! (`getrawtransaction`) until the funding tx is chain-locked, wait for
//! Platform's `coreChainLockedHeight` to reach that height, and build a
//! `ChainAssetLockProof` directly. On any network that chain-locks
//! promptly (devnets and testnet do, every block) this costs a couple of
//! minutes and cannot silently drop the proof.
//!
//! # Recoverability
//!
//! An asset lock is irreversible once broadcast. So all key material —
//! the one-time asset-lock WIF and the identity's private keys — is
//! written to the `--out-env` file **before** the transaction is
//! broadcast. If any later step fails, the locked funds are still
//! recoverable via the one-time key. Only the final identity id is ever
//! printed to stdout; key material never touches stdout or the logs.
//!
//! The ChainLock wait can legitimately take minutes (platform ingests a
//! fresh core ChainLock with some lag), so the proof timeout is generous.
//! If it is nonetheless exceeded, the tool does NOT strand the funds: it
//! reports that the asset lock is broadcast and on-chain, points at the
//! saved one-time WIF, and prints the exact command to finish the
//! registration against the existing lock — re-run with `--resume-txid
//! <txid>`, which reuses the lock and spends nothing new. Re-running
//! without `--resume-txid` would broadcast a second asset lock and strand
//! the first, so the resume path exists precisely to avoid that.
//!
//! # Endpoints
//!
//! It needs a Platform DAPI endpoint and a Dash Core RPC endpoint whose
//! wallet holds the faucet coins (reachable, e.g. over an SSH tunnel).
//! The SDK is built `with_core` so it fetches quorum public keys for
//! proof verification from that same Core RPC. The Core RPC password is
//! read from `CORE_RPC_PASSWORD` (falling back to `--core-rpc-password`)
//! so it never appears in the process arguments.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::process::ExitCode;
use std::str::FromStr;
use std::time::{Duration, Instant};

use clap::Parser;
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::types::epoch::Epoch;
use dash_sdk::{Sdk, SdkBuilder};
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::dashcore::consensus::encode::{deserialize, serialize};
use dpp::dashcore::secp256k1::Secp256k1;
use dpp::dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
use dpp::dashcore::transaction::special_transaction::TransactionPayload;
use dpp::dashcore::{
    Address, Network, OutPoint, PrivateKey, ScriptBuf, Transaction, TxIn, TxOut, Txid,
};
use dpp::dashcore_rpc::{Auth, Client, RpcApi};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::AssetLockProof;
use platform_version::version::PlatformVersion;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rs_dapi_client::{Address as DapiAddress, AddressList};
use simple_signer::signer::SimpleSigner;

/// Duffs per DASH.
const DUFFS_PER_DASH: u64 = 100_000_000;
/// Flat L1 fee for the asset-lock transaction (0.001 DASH — generous
/// for a handful of inputs on devnet/testnet).
const ASSET_LOCK_FEE_DUFFS: u64 = 100_000;
/// Minimum change worth returning; below this, fold into the fee.
const DUST_DUFFS: u64 = 10_000;
/// How long to wait for the asset-lock tx to be ChainLocked and for
/// platform to reach that core height before giving up. Generous, because
/// platform's ingestion of a fresh core ChainLock can lag by minutes; a
/// short budget here risks the tool giving up on a lock that is about to
/// become usable. On timeout the tool still prints a resume recipe rather
/// than stranding the funds (see the timeout handling in `run`).
const PROOF_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Parser, Debug)]
#[command(
    name = "register-identity",
    about = "Register a funded Platform identity using a ChainLock asset lock funded by a Core faucet wallet."
)]
struct Args {
    /// Comma-separated DAPI address(es), e.g. https://1.2.3.4:1443.
    #[arg(short = 'a', long = "address")]
    address: String,

    /// Network: mainnet | testnet | devnet | regtest.
    #[arg(short = 'n', long = "network", default_value = "testnet")]
    network: String,

    /// Core RPC host (reachable, e.g. over an SSH tunnel).
    #[arg(long = "core-host", default_value = "127.0.0.1")]
    core_host: String,

    /// Core RPC port.
    #[arg(long = "core-port", default_value = "20002")]
    core_port: u16,

    /// Core RPC username.
    #[arg(long = "core-rpc-user", default_value = "dashrpc")]
    core_rpc_user: String,

    /// Core RPC password. Prefer the CORE_RPC_PASSWORD env var.
    #[arg(long = "core-rpc-password")]
    core_rpc_password: Option<String>,

    /// Name of the Core wallet holding the faucet coins, e.g.
    /// dashd-wallet-1-faucet.
    #[arg(long = "faucet-wallet")]
    faucet_wallet: String,

    /// Amount of DASH to lock into the new identity's credits.
    #[arg(long = "fund-dash", default_value = "25")]
    fund_dash: f64,

    /// Number of identity keys to generate (>= 3 gives MASTER +
    /// CRITICAL + HIGH authentication keys).
    #[arg(long = "key-count", default_value = "3")]
    key_count: u32,

    /// Path to the mode-600 credentials file. Key material is appended
    /// here (never printed to stdout).
    #[arg(long = "out-env")]
    out_env: String,

    /// Resume a previously-broadcast asset lock instead of creating a new
    /// one. Pass the asset-lock txid; the one-time WIF is read from the
    /// `--out-env` file (`ASSET_LOCK_ONE_TIME_WIF`). Registers the identity
    /// against the existing chainlocked asset lock — NO new funds are spent.
    /// Use this to recover after a proof-wait timeout stranded a lock.
    #[arg(long = "resume-txid")]
    resume_txid: Option<String>,
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

/// Parse the comma-separated `--address` list into DAPI endpoints, rejecting
/// an EMPTY result (e.g. `""` or `",,"`). An empty endpoint list otherwise
/// flows through `AddressList::from_iter` and `SdkBuilder::build` unchecked and
/// fails only on the first Platform request — AFTER the asset-lock funding tx
/// is signed and broadcast, stranding the funds. This validation guards a
/// spend, so it must run before any funding work.
fn parse_dapi_addresses(arg: &str) -> Result<Vec<DapiAddress>, String> {
    let addresses = arg
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<DapiAddress>()
                .map_err(|e| format!("failed to parse address '{s}': {e}"))
        })
        .collect::<Result<Vec<DapiAddress>, String>>()?;
    if addresses.is_empty() {
        return Err(
            "--address resolved to an empty endpoint list; provide at least one DAPI address \
             such as https://1.2.3.4:1443. Refusing before any funding work."
                .to_string(),
        );
    }
    Ok(addresses)
}

/// Append `KEY=value` lines to the mode-600 credentials file. Creates
/// the file if absent; never truncates.
fn append_creds(path: &str, lines: &[(String, String)]) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| format!("failed to open creds file {path}: {e}"))?;
    // `.mode(0o600)` only applies to a freshly created inode. If the file
    // already existed with looser permissions, tighten it to 0600 BEFORE
    // writing any secret, so a pre-existing world-readable file cannot
    // silently keep exposing the keys we are about to append.
    f.set_permissions(std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("failed to enforce mode 600 on {path}: {e}"))?;
    for (k, v) in lines {
        writeln!(f, "{k}={v}").map_err(|e| format!("failed to write creds file: {e}"))?;
    }
    Ok(())
}

/// Read the last value for `key` from a `KEY=value` credentials file.
/// Last-wins, matching how the file is sourced by a shell.
fn read_creds_value(path: &str, key: &str) -> Result<Option<String>, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read creds file {path}: {e}"))?;
    let prefix = format!("{key}=");
    Ok(contents
        .lines()
        .filter_map(|l| l.strip_prefix(&prefix))
        .next_back()
        .map(str::to_string))
}

/// Minimum asset-lock funding, in duffs, that an identity-create with
/// `key_count` keys needs to clear its required-balance validation. Mirrors
/// `IdentityCreateTransition::calculate_min_required_fee` (versioned): the
/// asset-lock processing-start floor plus, on fee-calc v1+, the base
/// identity-create cost and the per-key creation cost. Locking below this
/// broadcasts and mines a transaction whose identity-create then fails —
/// stranding the funds, the exact failure this tool exists to avoid.
fn min_asset_lock_duffs(key_count: u32, pv: &PlatformVersion) -> u64 {
    let identities = &pv.dpp.state_transitions.identities;
    let floor_credits = identities
        .asset_locks
        .required_asset_lock_duff_balance_for_processing_start_for_identity_create
        .saturating_mul(CREDITS_PER_DUFF);
    let required_credits = match identities.calculate_min_required_fee_on_identity_create_transition
    {
        0 => floor_credits,
        _ => {
            let min_fees = &pv.fee_version.state_transition_min_fees;
            min_fees.identity_create_base_cost.saturating_add(
                floor_credits.saturating_add(
                    min_fees
                        .identity_key_in_creation_cost
                        .saturating_mul(key_count as u64),
                ),
            )
        }
    };
    // Convert the required credits back to the minimum asset-lock duffs
    // (ceil, so rounding never lands us a duff short).
    required_credits.div_ceil(CREDITS_PER_DUFF)
}

/// Message for a proof-wait timeout that makes the failure NON-stranding.
/// The funds are recoverable via the saved one-time key either way, but the
/// finality claim must match what Core actually reported: `chainlocked=false`
/// means the tx may still be unconfirmed/in the mempool (verify in Core
/// first), while `chainlocked=true` means only Platform's catch-up timed out.
fn strand_safe_timeout(txid: &Txid, out_env: &str, chainlocked: bool) -> String {
    let state = if chainlocked {
        "The asset lock IS broadcast and ChainLocked on L1 — only Platform's catch-up to that \
         core height timed out. The funds are NOT lost; re-running will finish once Platform \
         has caught up."
            .to_string()
    } else {
        format!(
            "The asset lock was broadcast but Core has NOT yet reported it as ChainLocked — it may \
             still be unconfirmed or in the mempool. The funds are NOT lost. First verify in Core \
             (`getrawtransaction {txid} 1` → chainlock:true); resume only once it is chainlocked."
        )
    };
    format!(
        "timed out waiting for the ChainLock proof.\n\
         {state}\n\
         The one-time key that owns the lock is saved in {out_env} (ASSET_LOCK_ONE_TIME_WIF__{txid}).\n\
         Resume WITHOUT spending new funds by re-running the same command plus:\n\
         \x20   --resume-txid {txid}\n\
         Do NOT re-run without --resume-txid — that broadcasts a second asset lock and strands this one."
    )
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
    let network = parse_network(&args.network)?;
    let platform_version = PlatformVersion::latest();

    // Key count: lower bound (need MASTER + HIGH + CRITICAL) and Platform's
    // versioned upper bound. Exceeding the max would sign and broadcast the
    // irreversible funding tx before the SDK rejects the over-large transition.
    let max_key_count = platform_version
        .dpp
        .state_transitions
        .identities
        .max_public_keys_in_creation;
    if args.key_count < 3 {
        return Err("--key-count must be >= 3 (need MASTER + CRITICAL + HIGH keys)".to_string());
    }
    if args.key_count > max_key_count as u32 {
        return Err(format!(
            "--key-count {} exceeds Platform's identity-create limit of {} \
             (max_public_keys_in_creation); the transition would be rejected AFTER the funding \
             transaction is broadcast",
            args.key_count, max_key_count
        ));
    }

    let core_password = std::env::var("CORE_RPC_PASSWORD")
        .ok()
        // An exported-but-empty CORE_RPC_PASSWORD must NOT shadow
        // --core-rpc-password; treat it as unset.
        .filter(|p| !p.is_empty())
        .or(args.core_rpc_password.clone())
        .ok_or_else(|| {
            "Core RPC password required: set CORE_RPC_PASSWORD or pass --core-rpc-password"
                .to_string()
        })?;

    let amount_duffs = (args.fund_dash * DUFFS_PER_DASH as f64) as u64;
    if amount_duffs == 0 {
        return Err("--fund-dash must be > 0".to_string());
    }
    // Reject an asset lock below the identity-create minimum BEFORE locking
    // funds: a smaller lock is broadcast and mined, then its identity-create
    // fails required-balance validation and the funds are stranded. Only the
    // fresh path spends; a resume reuses an existing lock.
    if args.resume_txid.is_none() {
        let min_duffs = min_asset_lock_duffs(args.key_count, platform_version);
        if amount_duffs < min_duffs {
            return Err(format!(
                "--fund-dash {} = {} duffs is below the identity-create minimum of {} duffs for {} \
                 keys (asset-lock floor + base + per-key create fee); a smaller lock would be \
                 broadcast and then stranded when registration fails its required-balance check",
                args.fund_dash, amount_duffs, min_duffs, args.key_count
            ));
        }
    }

    // --- Platform SDK (DAPI + Core for quorum public keys) ---
    // Reject an empty endpoint list BEFORE building the SDK or doing any
    // funding work (an empty list otherwise reaches the fresh path and the
    // asset lock is broadcast before the first Platform request fails).
    let address_list = AddressList::from_iter(parse_dapi_addresses(&args.address)?);

    // No HTTP quorum endpoint exists for this devnet, so the SDK fetches
    // quorum public keys (for proof verification) from Core RPC.
    let sdk: Sdk = SdkBuilder::new(address_list)
        .with_network(network)
        .with_core(
            &args.core_host,
            args.core_port,
            &args.core_rpc_user,
            &core_password,
        )
        .build()
        .map_err(|e| format!("failed to build SDK: {e}"))?;

    // --- Core RPC (faucet wallet) for listunspent / sign / send ---
    let wallet_url = format!(
        "http://{}:{}/wallet/{}",
        args.core_host, args.core_port, args.faucet_wallet
    );
    let core = Client::new(
        &wallet_url,
        Auth::UserPass(args.core_rpc_user.clone(), core_password.clone()),
    )
    .map_err(|e| format!("failed to connect to Core RPC: {e}"))?;

    // --- Generate the identity keys ---
    // The identity id derives from the asset-lock outpoint, not from these
    // keys, so the key set is generated the same way in both the fresh and
    // resume paths.
    let mut rng = StdRng::from_entropy();
    let (identity, key_material): (Identity, Vec<(IdentityPublicKey, [u8; 32])>) =
        Identity::random_identity_with_main_keys_with_private_key(
            args.key_count,
            &mut rng,
            platform_version,
        )
        .map_err(|e| format!("failed to generate identity: {e}"))?;

    // Encode the identity key material for the credentials file.
    let mut identity_creds: Vec<(String, String)> = Vec::new();
    for (pk, secret) in &key_material {
        let wif = PrivateKey::from_byte_array(secret, network)
            .map(|k| k.to_wif())
            .map_err(|e| format!("failed to encode identity key {} as WIF: {e}", pk.id()))?;
        identity_creds.push((format!("IDENTITY_KEY_{}_WIF", pk.id()), wif));
    }
    if let Some((critical_key, critical_secret)) = find_critical_auth_key(&identity, &key_material)
    {
        let wif = PrivateKey::from_byte_array(&critical_secret, network)
            .map(|k| k.to_wif())
            .map_err(|e| format!("failed to encode critical key as WIF: {e}"))?;
        identity_creds.push((
            "IDENTITY_CRITICAL_AUTH_KEY_ID".to_string(),
            critical_key.id().to_string(),
        ));
        identity_creds.push(("IDENTITY_CRITICAL_AUTH_KEY_WIF".to_string(), wif));
    }

    // --- Obtain the asset lock: resume an existing one, or create+broadcast ---
    let (txid, one_time_private_key): (Txid, PrivateKey) = if let Some(resume) = &args.resume_txid {
        // RESUME: reuse an already-broadcast asset lock — NO new funds spent.
        // Read the one-time key SCOPED TO THIS txid, so a credentials file that
        // accumulated several asset locks cannot pair the requested txid with a
        // different lock's one-time key. Identity keys are written on success
        // below, not here.
        let txid =
            Txid::from_str(resume.trim()).map_err(|e| format!("invalid --resume-txid: {e}"))?;
        let wif_key = format!("ASSET_LOCK_ONE_TIME_WIF__{txid}");
        let wif = read_creds_value(&args.out_env, &wif_key)?.ok_or_else(|| {
            format!(
                "resume found no {wif_key} in {} — the one-time key for txid {txid} is not recorded \
                 there; point --out-env at the credentials file written when this lock was created",
                args.out_env
            )
        })?;
        let one_time_private_key = PrivateKey::from_wif(wif.trim())
            .map_err(|e| format!("failed to parse {wif_key}: {e}"))?;
        eprintln!(
            "Resuming existing asset-lock tx {txid} (no new broadcast; no new funds spent)..."
        );
        (txid, one_time_private_key)
    } else {
        // FRESH: generate a one-time key, build + Core-sign + broadcast the
        // asset lock, capturing all recovery credentials BEFORE broadcast.
        let secp = Secp256k1::new();
        let mut secp_rng = dpp::dashcore::secp256k1::rand::thread_rng();
        let one_time_secret = dpp::dashcore::secp256k1::SecretKey::new(&mut secp_rng);
        let one_time_private_key = PrivateKey::new(one_time_secret, network);
        let one_time_public_key = one_time_private_key.public_key(&secp);
        let one_time_key_hash = one_time_public_key.pubkey_hash();
        let one_time_address = Address::p2pkh(&one_time_public_key, network);

        let tx = build_asset_lock_transaction(
            &core,
            amount_duffs,
            &one_time_key_hash,
            platform_version
                .dpp
                .state_transitions
                .identities
                .asset_locks
                .max_asset_lock_transaction_inputs,
        )?;
        let unsigned_hex = hex::encode(serialize(&tx));

        eprintln!(
            "Signing asset-lock tx ({} inputs, locking {} DASH) via Core wallet...",
            tx.input.len(),
            args.fund_dash
        );
        let signed = core
            .sign_raw_transaction_with_wallet(unsigned_hex.as_str(), None, None)
            .map_err(|e| format!("signrawtransactionwithwallet failed: {e}"))?;
        if !signed.complete {
            return Err(
                "Core could not fully sign the asset-lock tx (signrawtransactionwithwallet \
                 returned incomplete — is the faucet wallet loaded and are the inputs spendable?)"
                    .to_string(),
            );
        }
        let signed_hex = hex::encode(&signed.hex);
        let signed_tx: Transaction =
            deserialize(&signed.hex).map_err(|e| format!("failed to parse signed tx: {e}"))?;
        let txid = signed_tx.txid();

        // Capture recovery credentials BEFORE broadcast, SCOPED TO THIS txid so
        // multiple asset locks in one file never cross-contaminate. Identity
        // keys are written only on success (below), so the file never advertises
        // keys that do not control the registered identity.
        append_creds(
            &args.out_env,
            &[
                ("ASSET_LOCK_TXID".to_string(), txid.to_string()),
                (
                    format!("ASSET_LOCK_ONE_TIME_WIF__{txid}"),
                    one_time_private_key.to_wif(),
                ),
                (
                    format!("ASSET_LOCK_ONE_TIME_ADDRESS__{txid}"),
                    one_time_address.to_string(),
                ),
                (
                    format!("ASSET_LOCK_FUND_DASH__{txid}"),
                    args.fund_dash.to_string(),
                ),
            ],
        )?;
        eprintln!(
            "Recovery credentials (scoped to {txid}) written to {} before broadcast.",
            args.out_env
        );

        // sendrawtransaction can return a transport error AFTER Core already
        // accepted the tx. Do not treat that as a clean failure: check Core,
        // proceed if the tx is there, otherwise report the ambiguity and the
        // resume path rather than inviting a blind re-run (a second lock).
        let broadcast_txid = match core.send_raw_transaction(signed_hex.as_str()) {
            Ok(t) => t,
            Err(e) => match core.get_raw_transaction_info(&txid, None) {
                Ok(_) => {
                    eprintln!(
                        "sendrawtransaction returned an error ({e}) but tx {txid} is present in \
                         Core — treating as broadcast."
                    );
                    txid
                }
                Err(_) => {
                    return Err(format!(
                        "sendrawtransaction failed ({e}) and tx {txid} is NOT yet visible in Core \
                         — acceptance is ambiguous. Recovery credentials (scoped to {txid}) are \
                         saved in {}. Check Core: `getrawtransaction {txid} 1`. If it appears, \
                         resume with --resume-txid {txid}; do NOT blindly re-run, which could \
                         broadcast a second asset lock.",
                        args.out_env
                    ));
                }
            },
        };
        if broadcast_txid != txid {
            return Err(format!(
                "broadcast txid {broadcast_txid} does not match signed txid {txid}"
            ));
        }
        eprintln!("Broadcast asset-lock tx {txid}; waiting for ChainLock proof...");
        (txid, one_time_private_key)
    };

    // --- ChainLock asset-lock proof (no DAPI stream) ---
    let started = Instant::now();
    // 1. Wait for Core to report the tx as ChainLocked and give its height.
    let core_chain_locked_height: u32 = loop {
        if started.elapsed() > PROOF_TIMEOUT {
            // Not yet chainlocked when we gave up.
            return Err(strand_safe_timeout(&txid, &args.out_env, false));
        }
        // A transient Core RPC error after broadcast is retryable — the asset
        // lock is already on-chain, so keep polling until PROOF_TIMEOUT rather
        // than forcing manual recovery over a brief transport blip.
        let info = match core.get_raw_transaction_info(&txid, None) {
            Ok(info) => info,
            Err(e) => {
                eprintln!("transient: getrawtransaction failed ({e}); retrying in 2s...");
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        if info.chainlock {
            let h = info
                .height
                .ok_or("ChainLocked tx has no height in getrawtransaction result")?;
            break u32::try_from(h).map_err(|_| format!("invalid tx height {h}"))?;
        }
        eprintln!("asset-lock tx not yet ChainLocked; retrying in 2s...");
        tokio::time::sleep(Duration::from_secs(2)).await;
    };
    eprintln!(
        "asset-lock tx ChainLocked at core height {core_chain_locked_height}; waiting for platform to reach it..."
    );

    // 2. Wait until platform's core-chain-locked height has caught up, so
    //    drive will accept the proof.
    loop {
        if started.elapsed() > PROOF_TIMEOUT {
            // Chainlocked on L1; only Platform's catch-up timed out.
            return Err(strand_safe_timeout(&txid, &args.out_env, true));
        }
        // Transient DAPI metadata-fetch errors are retryable for the same
        // reason: the lock is chainlocked and recoverable, so retry until the
        // timeout instead of bailing to manual recovery.
        let metadata = match Epoch::fetch_current_with_metadata(&sdk).await {
            Ok((_epoch, metadata)) => metadata,
            Err(e) => {
                eprintln!("transient: platform metadata fetch failed ({e}); retrying in 2s...");
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        if metadata.core_chain_locked_height >= core_chain_locked_height {
            break;
        }
        eprintln!(
            "platform core-chain-locked height {} < {}; retrying in 2s...",
            metadata.core_chain_locked_height, core_chain_locked_height
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let asset_lock_proof = AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height,
        out_point: OutPoint { txid, vout: 0 },
    });
    let proof_elapsed = started.elapsed();
    eprintln!(
        "Got ChainLock asset-lock proof in {:.1}s. Registering identity...",
        proof_elapsed.as_secs_f64()
    );

    // --- Register the identity ---
    let mut signer = SimpleSigner::default();
    signer.add_identity_public_keys(key_material.iter().cloned());

    let reg_started = Instant::now();
    let registered = identity
        .put_to_platform_and_wait_for_response_with_private_key(
            &sdk,
            asset_lock_proof,
            &one_time_private_key,
            &signer,
            None,
        )
        .await
        .map_err(|e| format!("failed to register identity: {e}"))?;
    let reg_elapsed = reg_started.elapsed();

    let identity_id = registered.id();
    let identity_id_b58 = identity_id.to_string(Encoding::Base58);
    let balance = registered.balance();
    let total_elapsed = started.elapsed();

    // Write the identity key material ONLY now that registration succeeded, and
    // ONLY if the on-chain identity actually carries the keys we generated. If
    // the asset lock had already produced an identity in a prior run,
    // put_to_platform resolves the AlreadyExists response by fetching that
    // identity — whose keys are NOT ours. Advertising ours would hand out keys
    // that cannot sign for it.
    let our_key_data: std::collections::BTreeSet<Vec<u8>> = key_material
        .iter()
        .map(|(pk, _)| pk.data().as_slice().to_vec())
        .collect();
    let onchain_key_data: std::collections::BTreeSet<Vec<u8>> = registered
        .public_keys()
        .values()
        .map(|pk| pk.data().as_slice().to_vec())
        .collect();
    if our_key_data == onchain_key_data {
        append_creds(&args.out_env, &identity_creds)?;
    } else {
        eprintln!(
            "WARNING: identity {identity_id_b58} already existed with different keys (from a prior \
             run); this run's freshly generated keys do NOT control it and were NOT written to {}. \
             The controlling keys are whatever the original run saved.",
            args.out_env
        );
    }

    // Persist final identity facts.
    append_creds(
        &args.out_env,
        &[
            ("IDENTITY_ID".to_string(), identity_id_b58.clone()),
            ("IDENTITY_BALANCE_CREDITS".to_string(), balance.to_string()),
            (
                "IDENTITY_ASSET_LOCK_PROOF_TYPE".to_string(),
                "ChainLock".to_string(),
            ),
            (
                "IDENTITY_REGISTER_ELAPSED_SEC".to_string(),
                format!("{:.1}", total_elapsed.as_secs_f64()),
            ),
        ],
    )?;

    eprintln!(
        "REGISTERED id={identity_id_b58} proof=ChainLock credits={balance} \
         proof_wait={:.1}s register={:.1}s total={:.1}s",
        proof_elapsed.as_secs_f64(),
        reg_elapsed.as_secs_f64(),
        total_elapsed.as_secs_f64()
    );
    // stdout: identity id ONLY (no key material, ever).
    println!("{identity_id_b58}");

    Ok(())
}

/// Build an unsigned asset-lock funding transaction: select faucet
/// UTXOs to cover `amount_duffs` + fee, burn `amount_duffs` via an
/// OP_RETURN output, return change, and credit `amount_duffs` to the
/// one-time key hash in the asset-lock payload.
fn build_asset_lock_transaction(
    core: &Client,
    amount_duffs: u64,
    one_time_key_hash: &dpp::dashcore::PubkeyHash,
    max_inputs: u16,
) -> Result<Transaction, String> {
    let target = amount_duffs
        .checked_add(ASSET_LOCK_FEE_DUFFS)
        .ok_or("amount overflow")?;

    let mut utxos = core
        .list_unspent(Some(1), None, None, None, None)
        .map_err(|e| format!("listunspent failed: {e}"))?;
    // Largest first, so we cover the target with as few inputs as possible.
    utxos.sort_by(|a, b| b.amount.to_sat().cmp(&a.amount.to_sat()));

    let mut inputs = Vec::new();
    let mut change_script: Option<ScriptBuf> = None;
    let mut selected: u64 = 0;
    for entry in utxos {
        if change_script.is_none() {
            change_script = Some(entry.script_pub_key.clone());
        }
        inputs.push(TxIn {
            previous_output: OutPoint::new(entry.txid, entry.vout),
            script_sig: ScriptBuf::new(),
            sequence: 0xFFFF_FFFF,
            witness: Default::default(),
        });
        selected = selected.saturating_add(entry.amount.to_sat());
        if selected >= target {
            break;
        }
    }
    if selected < target {
        return Err(format!(
            "faucet wallet has insufficient spendable funds: need {target} duffs, have {selected}"
        ));
    }
    // Platform rejects an asset-lock proof whose transaction has more than
    // `max_asset_lock_transaction_inputs` inputs. Core would still sign, mine,
    // and chainlock such a tx, but every Platform use of its proof would be
    // invalid — the locked output permanently unusable. Refuse before signing.
    if inputs.len() > max_inputs as usize {
        return Err(format!(
            "funding this amount needs {} inputs, but Platform's asset-lock cap is {} \
             (max_asset_lock_transaction_inputs); the lock would be unusable. Consolidate the \
             faucet wallet into fewer, larger UTXOs or lock a smaller amount",
            inputs.len(),
            max_inputs
        ));
    }
    let change_script = change_script.expect("at least one input selected");

    // Regular outputs: burn (locks the value on L1) + change.
    let mut outputs = vec![TxOut {
        value: amount_duffs,
        script_pubkey: ScriptBuf::new_op_return(&[]),
    }];
    let change = selected - target;
    if change > DUST_DUFFS {
        outputs.push(TxOut {
            value: change,
            script_pubkey: change_script,
        });
    }

    // Asset-lock payload: credit the burned value to the one-time key.
    let payload = TransactionPayload::AssetLockPayloadType(AssetLockPayload {
        version: AssetLockPayload::CURRENT_VERSION,
        credit_outputs: vec![TxOut {
            value: amount_duffs,
            script_pubkey: ScriptBuf::new_p2pkh(one_time_key_hash),
        }],
    });

    Ok(Transaction {
        version: 3,
        lock_time: 0,
        input: inputs,
        output: outputs,
        special_transaction_payload: Some(payload),
    })
}

/// Find the identity's CRITICAL AUTHENTICATION ECDSA key and its
/// matching private key from the generated key material.
fn find_critical_auth_key(
    identity: &Identity,
    key_material: &[(IdentityPublicKey, [u8; 32])],
) -> Option<(IdentityPublicKey, [u8; 32])> {
    let public_key = identity.get_first_public_key_matching(
        Purpose::AUTHENTICATION,
        [SecurityLevel::CRITICAL].into_iter().collect(),
        [KeyType::ECDSA_SECP256K1].into_iter().collect(),
        false,
    )?;
    key_material
        .iter()
        .find(|(pk, _)| pk.id() == public_key.id())
        .map(|(pk, secret)| (pk.clone(), *secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_network_accepts_known_and_rejects_unknown() {
        assert!(matches!(parse_network("devnet"), Ok(Network::Devnet)));
        assert!(matches!(parse_network("DevNet"), Ok(Network::Devnet)));
        assert!(matches!(parse_network("testnet"), Ok(Network::Testnet)));
        assert!(matches!(parse_network("mainnet"), Ok(Network::Mainnet)));
        assert!(matches!(parse_network("regtest"), Ok(Network::Regtest)));
        assert!(parse_network("bogus").is_err());
    }

    #[test]
    fn dapi_addresses_reject_empty_before_any_spend() {
        // An empty endpoint list must be a hard error — otherwise it reaches the
        // funding path and the asset lock is broadcast before the SDK fails.
        assert!(parse_dapi_addresses("").is_err());
        assert!(parse_dapi_addresses(",,").is_err());
        assert!(parse_dapi_addresses("  , ,  ").is_err());
        let ok = parse_dapi_addresses("https://1.2.3.4:1443,https://5.6.7.8:1443").unwrap();
        assert_eq!(ok.len(), 2);
    }

    #[test]
    fn creds_roundtrip_returns_last_value_and_file_is_mode_600() {
        // A resume run appends fresh identity keys after an earlier attempt's,
        // so read_creds_value MUST return the LAST value — otherwise the caller
        // would sign with a stale key that does not match the live identity.
        use std::os::unix::fs::PermissionsExt;
        let path =
            std::env::temp_dir().join(format!("register_identity_test_{}.env", std::process::id()));
        let p = path.to_str().unwrap();
        let _ = std::fs::remove_file(p);

        append_creds(p, &[("K".to_string(), "first".to_string())]).unwrap();
        append_creds(
            p,
            &[
                ("OTHER".to_string(), "x".to_string()),
                ("K".to_string(), "second".to_string()),
            ],
        )
        .unwrap();

        assert_eq!(read_creds_value(p, "K").unwrap().as_deref(), Some("second"));
        assert_eq!(read_creds_value(p, "OTHER").unwrap().as_deref(), Some("x"));
        assert_eq!(read_creds_value(p, "MISSING").unwrap(), None);

        let mode = std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "credentials file must be created mode 600");
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn strand_message_is_non_stranding_and_actionable() {
        let txid =
            Txid::from_str("c913da3655688c10c79e0d8b8e059c94625b939cfa99e848f0f24dc48ec4f685")
                .unwrap();
        // Post-ChainLock timeout message.
        let msg = strand_safe_timeout(&txid, "/tmp/creds.env", true);
        // Pre-ChainLock message must NOT assert L1 finality it hasn't established.
        let pre = strand_safe_timeout(&txid, "/tmp/creds.env", false);
        assert!(pre.contains("--resume-txid") && pre.to_lowercase().contains("not lost"));
        assert!(msg.contains("ChainLocked on L1"));
        // Must name the txid, the creds path, and the exact resume flag, and must
        // reassure the funds are not lost — that is the whole point of the change.
        assert!(msg.contains("c913da3655688c10c79e0d8b8e059c94625b939cfa99e848f0f24dc48ec4f685"));
        assert!(msg.contains("/tmp/creds.env"));
        assert!(msg.contains("--resume-txid"));
        assert!(msg.to_lowercase().contains("not lost"));
    }

    #[test]
    fn min_asset_lock_clears_the_floor_and_grows_with_keys() {
        // The minimum guard exists so a lock below the identity-create
        // requirement is never broadcast (which would strand it): the computed
        // minimum must be at least the versioned processing-start floor and must
        // increase as more keys are added.
        let pv = PlatformVersion::latest();
        let floor = pv
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .required_asset_lock_duff_balance_for_processing_start_for_identity_create;
        let min3 = min_asset_lock_duffs(3, pv);
        let min6 = min_asset_lock_duffs(6, pv);
        assert!(
            min3 >= floor,
            "minimum must clear the processing-start floor"
        );
        assert!(min6 > min3, "more keys must require a larger minimum lock");
    }

    #[test]
    fn finds_the_critical_authentication_signing_key_and_its_secret() {
        // The whole tool is useless if it cannot hand back a CRITICAL-level
        // authentication key (the one that can sign both contracts and
        // documents) together with the private key that matches it.
        let platform_version = PlatformVersion::latest();
        let mut rng = StdRng::seed_from_u64(1);
        let (identity, key_material): (Identity, Vec<(IdentityPublicKey, [u8; 32])>) =
            Identity::random_identity_with_main_keys_with_private_key(
                3,
                &mut rng,
                platform_version,
            )
            .expect("generate identity");

        let (pk, secret) = find_critical_auth_key(&identity, &key_material)
            .expect("a 3-key identity must expose a CRITICAL AUTHENTICATION ECDSA key");

        assert_eq!(pk.purpose(), Purpose::AUTHENTICATION);
        assert_eq!(pk.security_level(), SecurityLevel::CRITICAL);
        assert_eq!(pk.key_type(), KeyType::ECDSA_SECP256K1);

        // The returned secret is really THIS key's material, not some other key's.
        let expected = key_material
            .iter()
            .find(|(k, _)| k.id() == pk.id())
            .map(|(_, s)| *s)
            .unwrap();
        assert_eq!(secret, expected);
    }
}
