//! Moves the embedded checkpoints forward. `yarn release` runs it.
//!
//! ```sh
//! cargo run -p rs-sdk-trusted-context-provider --example refresh_checkpoints -- \
//!     [--report <file>] [mainnet] [testnet]
//! ```
//!
//! For each network it asks the relay for two proofs from the current
//! checkpoint, one to the relay's tip and one to a block about a day below
//! it, verifies both, and writes the lower target as the new checkpoint along
//! with the evidence `tests/checkpoints.rs` checks. A new checkpoint is proven
//! from the one it replaces, so a refresh adds no trust in the relay.
//!
//! The relay's quorum list only suggests which height to ask for. What bounds
//! the new checkpoint is the verified tip: a relay cannot prove a ChainLock
//! above the real tip, and the checkpoint must be at least half a Platform
//! quorum lifetime (about 12 hours) below it, so the Core heights Platform
//! reports after a release are above it. A relay that hides its tip only
//! makes the checkpoint older.
//!
//! `MAINNET_QUORUM_RELAY` or `TESTNET_QUORUM_RELAY` replaces a network's
//! public relay, which must serve both `/quorums` and `/proofs`. A network
//! that fails keeps its checkpoint, and the exit status is nonzero.
//! `--report` appends one Markdown line per network to a file, for the
//! release PR.

use dash_core_proof::{bootstrap, rpc_hex, QuorumProofChain, State, MAX_PROOF_BYTES};
use dpp::dashcore::sml::llmq_type::network::NetworkLLMQExt;
use dpp::dashcore::Network;
use rs_sdk_trusted_context_provider::types::QuorumsResponse;
use rs_sdk_trusted_context_provider::{get_quorum_base_url, VerifiedHttpContextProvider};
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

/// Largest `/quorums` body read. A real list is a few KiB.
const MAX_QUORUMS_BYTES: usize = 1 << 20;

const USAGE: &str = "usage: refresh_checkpoints [--report <file>] [mainnet] [testnet]";

#[tokio::main]
async fn main() -> ExitCode {
    let mut networks = Vec::new();
    let mut report = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "mainnet" => networks.push(Network::Mainnet),
            "testnet" => networks.push(Network::Testnet),
            "--report" => match args.next() {
                Some(path) => report = Some(PathBuf::from(path)),
                None => {
                    eprintln!("{USAGE}");
                    return ExitCode::FAILURE;
                }
            },
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }
    if networks.is_empty() {
        networks = vec![Network::Mainnet, Network::Testnet];
    }

    let client = match reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            eprintln!("cannot build an HTTP client: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut status = ExitCode::SUCCESS;
    for network in networks {
        let line = match refresh(&client, network).await {
            Ok(message) => {
                println!("{network}: {message}");
                format!("- {network}: {message}")
            }
            Err(error) => {
                eprintln!("{network}: checkpoint not refreshed: {error}");
                status = ExitCode::FAILURE;
                // Relay text stays in a code span, so it cannot pose as
                // Markdown in the release PR.
                format!(
                    "- {network}: **not refreshed**, kept the previous checkpoint: `{}`",
                    error.replace('`', "'")
                )
            }
        };
        // Written as each network finishes, so the report matches the files
        // even if a later network's run is cut short.
        if let Some(path) = &report {
            let written = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| writeln!(file, "{line}"));
            if let Err(error) = written {
                eprintln!("{}: {error}", path.display());
                status = ExitCode::FAILURE;
            }
        }
    }
    status
}

async fn refresh(client: &reqwest::Client, network: Network) -> Result<String, String> {
    let current = VerifiedHttpContextProvider::embedded_checkpoint(network)
        .map_err(|error| error.to_string())?;
    let name = network.to_string();
    let variable = format!("{}_QUORUM_RELAY", name.to_uppercase());
    let relay = match std::env::var(&variable) {
        Ok(relay) => relay.trim_end_matches('/').to_string(),
        Err(_) => get_quorum_base_url(network, None).map_err(|error| error.to_string())?,
    };
    let quorum_type = network.platform_type();
    let params = quorum_type.params();
    let interval = params.dkg_params.interval;
    // Half a quorum lifetime: Platform's Core height trails the tip by a few
    // blocks unless it halts.
    let depth = interval * params.signing_active_quorum_count / 2;

    let url = format!("{relay}/quorums");
    let response = client
        .get(&url)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("{url}: {error}"))?;
    let quorums: QuorumsResponse =
        serde_json::from_slice(&read_capped(response, MAX_QUORUMS_BYTES, &url).await?)
            .map_err(|error| format!("{url}: {error}"))?;
    if !quorums.success {
        return Err(format!("{url} reports failure"));
    }
    let (Some(oldest), Some(newest)) = (
        quorums.data.iter().min_by_key(|quorum| quorum.height),
        quorums.data.iter().max_by_key(|quorum| quorum.height),
    ) else {
        return Err(format!("{url} lists no quorums"));
    };

    // Height 0 asks for the relay's tip; the newest quorum is active there.
    let (tip, _) = prove(client, &relay, network, &current, 0, &newest.quorum_hash).await?;
    let bound = tip.height.saturating_sub(depth);
    if bound <= current.height {
        return Ok(format!(
            "kept {}; the verified tip {} is not {depth} blocks past it",
            current.height, tip.height
        ));
    }

    // The oldest active quorum is in every quorum root from shortly after
    // its DKG until about a day later, so a target just past its mining can
    // open it. A quorum's height is where its DKG started; it is mined within
    // the next DKG interval.
    let height = u32::try_from(oldest.height)
        .ok()
        .and_then(|height| height.checked_add(interval))
        .ok_or_else(|| format!("quorum height {} is out of range", oldest.height))?;
    if height <= current.height {
        return Ok(format!(
            "kept {}; the oldest listed quorum ({}) is not past it",
            current.height, oldest.height
        ));
    }
    if height > bound {
        return Err(format!(
            "the oldest listed quorum ({}) is within {depth} blocks of the verified tip {}",
            oldest.height, tip.height
        ));
    }
    let (target, chain) = prove(
        client,
        &relay,
        network,
        &current,
        height,
        &oldest.quorum_hash,
    )
    .await?;
    // Both heights are proven, so this holds whatever the relay listed.
    if target.height > bound {
        return Err(format!(
            "the relay answered at {}, within {depth} blocks of the verified tip {}",
            target.height, tip.height
        ));
    }
    let evidence = json!({
        "header": hex::encode(chain.target.header),
        "coinbase": hex::encode(&chain.coinbase.transaction),
        "coinbase_path": {
            "index": chain.coinbase.path.index,
            "count": chain.coinbase.path.count,
            "siblings": chain.coinbase.path.siblings.iter().map(hex::encode).collect::<Vec<_>>(),
        },
    });

    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    write_together(&[
        (
            crate_dir.join(format!("tests/data/checkpoint-evidence-{name}.json")),
            to_json(&evidence)?,
        ),
        (
            crate_dir.join(format!("checkpoints/{name}.json")),
            to_json(&target)?,
        ),
    ])?;
    Ok(format!(
        "{} -> {} (`{}`), verified tip {}",
        current.height,
        target.height,
        rpc_hex(&target.block_hash),
        tip.height
    ))
}

/// Ask the relay for a proof from `checkpoint` to a target at or above
/// `height`, and verify it. The quorum only lets the relay build the proof;
/// its opening is not checked.
async fn prove(
    client: &reqwest::Client,
    relay: &str,
    network: Network,
    checkpoint: &State,
    height: u32,
    quorum_hash: &str,
) -> Result<(State, QuorumProofChain), String> {
    let url = format!("{relay}/proofs");
    let response = client
        .post(&url)
        .json(&json!({
            "checkpoint": rpc_hex(&checkpoint.block_hash),
            "height": height,
            "quorumHash": quorum_hash,
            "llmqType": u8::from(network.platform_type()),
            "nodeCount": 0,
        }))
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("{url}: {error}"))?;
    let body = read_capped(response, MAX_PROOF_BYTES, &url).await?;
    let state = bootstrap::verify(&body, checkpoint, height)
        .map_err(|error| format!("the proof does not verify: {}", error.message()))?
        .state()
        .clone();
    // The envelope opens with the chain as a blob: a little-endian u32
    // length and the bytes. `verify` has already parsed it.
    let length = u32::from_le_bytes(body[..4].try_into().expect("4 bytes")) as usize;
    let chain = QuorumProofChain::decode(&body[4..4 + length])
        .map_err(|error| format!("the proof does not decode: {}", error.message()))?;
    Ok((state, chain))
}

/// Read a body, refusing one over `limit` before buffering past it.
async fn read_capped(
    mut response: reqwest::Response,
    limit: usize,
    url: &str,
) -> Result<Vec<u8>, String> {
    let too_large = || format!("{url}: the body is over {limit} bytes");
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(too_large());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("{url}: {error}"))?
    {
        if chunk.len() > limit - body.len() {
            return Err(too_large());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn to_json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|text| text + "\n")
        .map_err(|error| error.to_string())
}

/// Replaces the files as one generation: each is written beside its
/// destination first, and if a rename fails the files already renamed get
/// their old contents back. A crash between two renames can still leave a
/// mixed pair, which the `checkpoints` test then fails.
fn write_together(files: &[(PathBuf, String)]) -> Result<(), String> {
    let mut staged: Vec<(PathBuf, &PathBuf, Vec<u8>)> = Vec::with_capacity(files.len());
    for (path, text) in files {
        let staging = path.with_extension("json.new");
        let written = std::fs::read(path)
            .map_err(|error| format!("{}: {error}", path.display()))
            .and_then(|old| {
                std::fs::write(&staging, text)
                    .map(|()| old)
                    .map_err(|error| format!("{}: {error}", staging.display()))
            });
        match written {
            Ok(old) => staged.push((staging, path, old)),
            Err(error) => {
                for (staging, _, _) in &staged {
                    let _ = std::fs::remove_file(staging);
                }
                return Err(error);
            }
        }
    }
    for (index, (staging, path, _)) in staged.iter().enumerate() {
        if let Err(error) = std::fs::rename(staging, path) {
            for (_, done, old) in &staged[..index] {
                if let Err(error) = std::fs::write(done, old) {
                    eprintln!("{}: cannot restore: {error}", done.display());
                }
            }
            for (staging, _, _) in &staged[index..] {
                let _ = std::fs::remove_file(staging);
            }
            return Err(format!("{}: {error}", path.display()));
        }
    }
    Ok(())
}
