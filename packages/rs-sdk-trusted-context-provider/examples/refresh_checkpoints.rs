//! Moves the embedded checkpoints forward. `yarn release` runs it.
//!
//! ```sh
//! cargo run -p rs-sdk-trusted-context-provider --example refresh_checkpoints [mainnet] [testnet]
//! ```
//!
//! For each network it asks the relay for a proof from the current
//! checkpoint, verifies it, and writes the proof's target as the new
//! checkpoint along with the evidence `tests/checkpoints.rs` checks. A new
//! checkpoint is proven from the one it replaces, so a refresh adds no trust
//! in the relay. `MAINNET_QUORUM_RELAY` or `TESTNET_QUORUM_RELAY` replaces a
//! network's public relay, which must serve both `/quorums` and `/proofs`.
//!
//! The target is about one Platform quorum lifetime (a day) below the tip,
//! so the Core heights Platform reports after a release are above it. A
//! network that fails keeps its checkpoint, and the exit status is nonzero.

use dash_core_proof::{bootstrap, rpc_hex, QuorumProofChain};
use dpp::dashcore::sml::llmq_type::network::NetworkLLMQExt;
use dpp::dashcore::Network;
use rs_sdk_trusted_context_provider::types::QuorumsResponse;
use rs_sdk_trusted_context_provider::{get_quorum_base_url, VerifiedHttpContextProvider};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

#[tokio::main]
async fn main() -> ExitCode {
    let mut networks = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "mainnet" => networks.push(Network::Mainnet),
            "testnet" => networks.push(Network::Testnet),
            _ => {
                eprintln!("usage: refresh_checkpoints [mainnet] [testnet]");
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
        match refresh(&client, network).await {
            Ok(message) => println!("{network}: {message}"),
            Err(error) => {
                eprintln!("{network}: checkpoint not refreshed: {error}");
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

    // The oldest active quorum is in every quorum root from shortly after
    // its DKG until about a day later, so a target just past its mining
    // can open it.
    let quorums: QuorumsResponse = client
        .get(format!("{relay}/quorums"))
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("{relay}/quorums: {error}"))?
        .json()
        .await
        .map_err(|error| format!("{relay}/quorums: {error}"))?;
    let oldest = quorums
        .data
        .iter()
        .min_by_key(|quorum| quorum.height)
        .ok_or_else(|| format!("{relay}/quorums lists no quorums"))?;
    // A quorum's height is where its DKG started; it is mined within the
    // next DKG interval.
    let interval = network.platform_type().params().dkg_params.interval;
    let height = u32::try_from(oldest.height)
        .ok()
        .and_then(|height| height.checked_add(interval))
        .ok_or_else(|| format!("quorum height {} is out of range", oldest.height))?;
    if height <= current.height {
        return Ok(format!(
            "kept {}; the relay's oldest quorum is not past it",
            current.height
        ));
    }

    let response = client
        .post(format!("{relay}/proofs"))
        .json(&json!({
            "checkpoint": rpc_hex(&current.block_hash),
            "height": height,
            "quorumHash": oldest.quorum_hash,
            "llmqType": u8::from(network.platform_type()),
            "nodeCount": 0,
        }))
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("{relay}/proofs: {error}"))?;
    let body = response
        .bytes()
        .await
        .map_err(|error| format!("{relay}/proofs: {error}"))?;

    let target = bootstrap::verify(&body, &current, height)
        .map_err(|error| format!("the proof does not verify: {}", error.message()))?
        .state()
        .clone();
    // The relay's height is a minimum. A target far above it, near the tip,
    // would leave heights Platform still reports unprovable.
    if target.height > height.saturating_add(interval) {
        return Err(format!(
            "the relay answered at {}, well above the {height} asked",
            target.height
        ));
    }
    // The envelope opens with the chain as a blob: a little-endian u32
    // length and the bytes. `verify` has already parsed it.
    let length = u32::from_le_bytes(body[..4].try_into().expect("4 bytes")) as usize;
    let chain = QuorumProofChain::decode(&body[4..4 + length])
        .map_err(|error| format!("the proof does not decode: {}", error.message()))?;
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
    write_together([
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
        "{} -> {} ({})",
        current.height,
        target.height,
        rpc_hex(&target.block_hash)
    ))
}

fn to_json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|text| text + "\n")
        .map_err(|error| error.to_string())
}

/// Writes every file beside its destination first, so a failed write leaves
/// the checkpoint and its evidence as they were.
fn write_together<const N: usize>(files: [(PathBuf, String); N]) -> Result<(), String> {
    let staged = files.map(|(path, text)| (path.with_extension("json.new"), path, text));
    for (staging, _, text) in &staged {
        std::fs::write(staging, text).map_err(|error| format!("{}: {error}", staging.display()))?;
    }
    for (staging, path, _) in &staged {
        std::fs::rename(staging, path).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}
