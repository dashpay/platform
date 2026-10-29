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
use std::path::{Path, PathBuf};
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
        let line = match refresh_embedded(&client, network).await {
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

/// Refresh this crate's embedded checkpoint for `network`.
async fn refresh_embedded(client: &reqwest::Client, network: Network) -> Result<String, String> {
    let current = VerifiedHttpContextProvider::embedded_checkpoint(network)
        .map_err(|error| error.to_string())?;
    let variable = format!("{}_QUORUM_RELAY", network.to_string().to_uppercase());
    let relay = match std::env::var(&variable) {
        Ok(relay) => relay.trim_end_matches('/').to_string(),
        Err(_) => get_quorum_base_url(network, None).map_err(|error| error.to_string())?,
    };
    refresh(
        client,
        network,
        &relay,
        &current,
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
    .await
}

/// Move `current` forward through `relay`, writing the new checkpoint and its
/// evidence under `crate_dir`.
async fn refresh(
    client: &reqwest::Client,
    network: Network,
    relay: &str,
    current: &State,
    crate_dir: &Path,
) -> Result<String, String> {
    let name = network.to_string();
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
    let (tip, _) = prove(client, relay, network, current, 0, &newest.quorum_hash).await?;
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
    let (target, chain) =
        prove(client, relay, network, current, height, &oldest.quorum_hash).await?;
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
                    .map_err(|error| {
                        // A failed write can leave a partial file behind.
                        let _ = std::fs::remove_file(&staging);
                        format!("{}: {error}", staging.display())
                    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use dash_core_proof::header_hash;
    use serde_json::Value;
    use std::io::{BufRead, BufReader, Read};
    use std::net::TcpListener;

    /// Two real testnet proofs from one checkpoint (1,574,768): one to
    /// 1,576,768, and one to the relay's tip about 5,000 blocks later.
    fn fixtures() -> (State, Vec<u8>, Vec<u8>) {
        let read = |json: &str| -> Value { serde_json::from_str(json).expect("fixture JSON") };
        let target = read(include_str!("../tests/data/proof-testnet.json"));
        let tip = read(include_str!("../tests/data/proof-testnet-tip.json"));
        assert_eq!(target["checkpoint"], tip["checkpoint"]);
        let proof = |value: &Value| hex::decode(value["bootstrap_hex"].as_str().expect("hex"));
        (
            serde_json::from_value(target["checkpoint"].clone()).expect("a checkpoint"),
            proof(&target).expect("hex"),
            proof(&tip).expect("hex"),
        )
    }

    fn proven_height(proof: &[u8], checkpoint: &State) -> u32 {
        bootstrap::verify(proof, checkpoint, 0)
            .expect("a valid proof")
            .state()
            .height
    }

    fn quorums(height: u32, success: bool) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "success": success,
            "data": [{
                "quorum_hash": "00".repeat(32),
                "key": "",
                "height": height,
                "valid_members_count": 0,
            }],
        }))
        .expect("JSON")
    }

    type Handler = Box<dyn Fn(&str, &Value) -> (u16, Vec<u8>) + Send>;

    /// A relay on loopback answering each request with `handler(path, body)`.
    fn relay(handler: Handler) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener.local_addr().expect("an address");
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    continue;
                }
                let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    if reader.read_line(&mut header).unwrap_or(0) == 0 || header.trim().is_empty() {
                        break;
                    }
                    if let Some((name, value)) = header.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse().unwrap_or(0);
                        }
                    }
                }
                let mut body = vec![0; length];
                if reader.read_exact(&mut body).is_err() {
                    continue;
                }
                let request = serde_json::from_slice(&body).unwrap_or(Value::Null);
                let (status, body) = handler(&path, &request);
                let mut stream = reader.into_inner();
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(&body);
            }
        });
        format!("http://{address}")
    }

    /// A crate directory holding placeholder testnet checkpoint files.
    fn crate_dir(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("refresh-checkpoints-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["checkpoints", "tests/data"] {
            std::fs::create_dir_all(dir.join(sub)).expect("a directory");
        }
        for file in [
            "checkpoints/testnet.json",
            "tests/data/checkpoint-evidence-testnet.json",
        ] {
            std::fs::write(dir.join(file), "old\n").expect("a placeholder");
        }
        dir
    }

    fn unchanged(dir: &Path) -> bool {
        [
            "checkpoints/testnet.json",
            "tests/data/checkpoint-evidence-testnet.json",
        ]
        .iter()
        .all(|file| std::fs::read_to_string(dir.join(file)).ok().as_deref() == Some("old\n"))
            && !dir.join("checkpoints/testnet.json.new").exists()
    }

    async fn run(handler: Handler, dir: &Path) -> Result<String, String> {
        let (checkpoint, _, _) = fixtures();
        refresh(
            &reqwest::Client::new(),
            Network::Testnet,
            &relay(handler),
            &checkpoint,
            dir,
        )
        .await
    }

    /// Answers `/quorums` with one quorum at `quorum_height`, the tip ask
    /// with `tip` and any other ask with `target`.
    fn honest(quorum_height: u32, tip: Vec<u8>, target: Vec<u8>) -> Handler {
        Box::new(move |path, request| match path {
            "/quorums" => (200, quorums(quorum_height, true)),
            _ if request["height"] == 0 => (200, tip.clone()),
            _ => (200, target.clone()),
        })
    }

    #[tokio::test]
    async fn should_write_a_proven_target_below_the_verified_tip_and_its_evidence() {
        let (checkpoint, target, tip) = fixtures();
        let target_height = proven_height(&target, &checkpoint);
        let dir = crate_dir("success");

        let message = run(honest(target_height - 24, tip, target.clone()), &dir)
            .await
            .expect("a refresh");
        assert!(
            message.starts_with(&format!("1574768 -> {target_height}")),
            "{message}"
        );

        let written: State = serde_json::from_str(
            &std::fs::read_to_string(dir.join("checkpoints/testnet.json")).expect("a checkpoint"),
        )
        .expect("a state");
        assert_eq!(
            written,
            *bootstrap::verify(&target, &checkpoint, 0)
                .expect("valid")
                .state()
        );
        let evidence: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("tests/data/checkpoint-evidence-testnet.json"))
                .expect("evidence"),
        )
        .expect("JSON");
        let header: [u8; 80] = hex::decode(evidence["header"].as_str().expect("hex"))
            .expect("hex")
            .try_into()
            .expect("80 bytes");
        assert_eq!(header_hash(&header), written.block_hash);
        assert!(!dir.join("checkpoints/testnet.json.new").exists());
    }

    #[tokio::test]
    async fn should_refuse_a_target_within_the_depth_of_the_verified_tip() {
        let (checkpoint, target, tip) = fixtures();
        let dir = crate_dir("near-tip-target");
        // The relay answers every ask at its tip.
        let error = run(
            honest(proven_height(&target, &checkpoint) - 24, tip.clone(), tip),
            &dir,
        )
        .await
        .expect_err("a target at the tip");
        assert!(
            error.contains("within 288 blocks of the verified tip"),
            "{error}"
        );
        assert!(unchanged(&dir));
    }

    #[tokio::test]
    async fn should_refuse_a_listed_quorum_near_the_verified_tip() {
        let (checkpoint, target, tip) = fixtures();
        let tip_height = proven_height(&tip, &checkpoint);
        let dir = crate_dir("near-tip-quorum");
        let error = run(honest(tip_height - 30, tip, target), &dir)
            .await
            .expect_err("a quorum the relay calls oldest near the tip");
        assert!(error.contains("oldest listed quorum"), "{error}");
        assert!(unchanged(&dir));
    }

    #[tokio::test]
    async fn should_bound_by_a_lower_tip_when_the_relay_hides_its_own() {
        let (checkpoint, target, _) = fixtures();
        let dir = crate_dir("hidden-tip");
        // The "tip" is the target itself, so it cannot be 288 blocks below.
        let error = run(
            honest(
                proven_height(&target, &checkpoint) - 24,
                target.clone(),
                target,
            ),
            &dir,
        )
        .await
        .expect_err("no target far enough below a hidden tip");
        assert!(error.contains("288 blocks"), "{error}");
        assert!(unchanged(&dir));
    }

    #[tokio::test]
    async fn should_refuse_an_unsuccessful_quorum_list() {
        let dir = crate_dir("unsuccessful");
        let error = run(Box::new(|_, _| (200, quorums(1_576_744, false))), &dir)
            .await
            .expect_err("an unsuccessful list");
        assert!(error.contains("reports failure"), "{error}");
        assert!(unchanged(&dir));
    }

    #[tokio::test]
    async fn should_refuse_an_oversized_proof_without_buffering_it() {
        let dir = crate_dir("oversized");
        let error = run(
            Box::new(|path, _| match path {
                "/quorums" => (200, quorums(1_576_744, true)),
                _ => (200, vec![0; MAX_PROOF_BYTES + 1]),
            }),
            &dir,
        )
        .await
        .expect_err("an oversized body");
        assert!(error.contains("over"), "{error}");
        assert!(unchanged(&dir));
    }

    #[tokio::test]
    async fn should_report_a_relay_error() {
        let dir = crate_dir("relay-error");
        let error = run(Box::new(|_, _| (403, b"Forbidden".to_vec())), &dir)
            .await
            .expect_err("a 403");
        assert!(error.contains("403"), "{error}");
        assert!(unchanged(&dir));
    }

    #[test]
    fn should_leave_every_file_unchanged_when_staging_one_fails() {
        let dir = crate_dir("staging");
        let error = write_together(&[
            (dir.join("checkpoints/testnet.json"), "new\n".to_string()),
            (dir.join("missing/testnet.json"), "new\n".to_string()),
        ])
        .expect_err("a missing destination");
        assert!(error.contains("missing"), "{error}");
        assert!(unchanged(&dir));
    }
}
