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

/// Append `KEY=value` lines to the mode-600 credentials file. Creates
/// the file if absent; never truncates.
fn append_creds(path: &str, lines: &[(String, String)]) -> Result<(), String> {
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| format!("failed to open creds file {path}: {e}"))?;
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

/// Message for a proof-wait timeout that makes the failure NON-stranding:
/// the asset lock is already broadcast and on-chain, so the funds are not
/// lost — they are pending. It names the recoverable one-time key and the
/// exact command to resume without spending new funds.
fn strand_safe_timeout(txid: &Txid, out_env: &str, waiting_for: &str) -> String {
    format!(
        "timed out waiting for {waiting_for}.\n\
         The asset lock IS broadcast and on-chain — the funds are NOT lost, only pending.\n\
         The one-time key that owns it is saved in {out_env} (ASSET_LOCK_ONE_TIME_WIF).\n\
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

    if args.key_count < 3 {
        return Err("--key-count must be >= 3 (need MASTER + CRITICAL + HIGH keys)".to_string());
    }

    let core_password = std::env::var("CORE_RPC_PASSWORD")
        .ok()
        .or(args.core_rpc_password.clone())
        .ok_or_else(|| {
            "Core RPC password required: set CORE_RPC_PASSWORD or pass --core-rpc-password"
                .to_string()
        })?;

    let amount_duffs = (args.fund_dash * DUFFS_PER_DASH as f64) as u64;
    if amount_duffs == 0 {
        return Err("--fund-dash must be > 0".to_string());
    }

    // --- Platform SDK (DAPI + Core for quorum public keys) ---
    let addresses = args
        .address
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<DapiAddress>()
                .map_err(|e| format!("failed to parse address '{s}': {e}"))
        })
        .collect::<Result<Vec<DapiAddress>, String>>()?;
    let address_list = AddressList::from_iter(addresses);

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
        // The one-time key that owns the lock is read from the credentials
        // file written before the original broadcast; the freshly generated
        // identity keys are recorded now so the resumed identity is
        // recoverable too.
        let txid =
            Txid::from_str(resume.trim()).map_err(|e| format!("invalid --resume-txid: {e}"))?;
        let wif = read_creds_value(&args.out_env, "ASSET_LOCK_ONE_TIME_WIF")?.ok_or_else(|| {
            format!(
                "resume needs ASSET_LOCK_ONE_TIME_WIF in {} (the one-time key that owns the asset lock)",
                args.out_env
            )
        })?;
        let one_time_private_key = PrivateKey::from_wif(wif.trim())
            .map_err(|e| format!("failed to parse ASSET_LOCK_ONE_TIME_WIF: {e}"))?;
        append_creds(&args.out_env, &identity_creds)?;
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

        let tx = build_asset_lock_transaction(&core, amount_duffs, &one_time_key_hash)?;
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

        // Capture recovery credentials BEFORE broadcast: once on-chain, the
        // locked funds are recoverable only via the one-time key + outpoint.
        let mut pre_creds: Vec<(String, String)> = vec![
            ("ASSET_LOCK_TXID".to_string(), txid.to_string()),
            (
                "ASSET_LOCK_ONE_TIME_WIF".to_string(),
                one_time_private_key.to_wif(),
            ),
            (
                "ASSET_LOCK_ONE_TIME_ADDRESS".to_string(),
                one_time_address.to_string(),
            ),
            (
                "ASSET_LOCK_FUND_DASH".to_string(),
                args.fund_dash.to_string(),
            ),
        ];
        pre_creds.extend(identity_creds.iter().cloned());
        append_creds(&args.out_env, &pre_creds)?;
        eprintln!(
            "Recovery credentials written to {} before broadcast.",
            args.out_env
        );

        let broadcast_txid = core
            .send_raw_transaction(signed_hex.as_str())
            .map_err(|e| format!("sendrawtransaction failed: {e}"))?;
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
            return Err(strand_safe_timeout(
                &txid,
                &args.out_env,
                "the asset-lock tx to be ChainLocked",
            ));
        }
        let info = core
            .get_raw_transaction_info(&txid, None)
            .map_err(|e| format!("getrawtransaction failed: {e}"))?;
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
            return Err(strand_safe_timeout(
                &txid,
                &args.out_env,
                "platform to reach the ChainLocked core height",
            ));
        }
        let (_epoch, metadata) = Epoch::fetch_current_with_metadata(&sdk)
            .await
            .map_err(|e| format!("failed to fetch platform metadata: {e}"))?;
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

    // Persist final identity facts (key material already written pre-broadcast).
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
