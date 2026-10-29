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
//! The relay's quorum list only suggests which height to ask for. The new
//! checkpoint must be at least half a Platform quorum lifetime (about 12
//! hours) below two heights, so the Core heights Platform reports after a
//! release are above it:
//!
//! - the verified tip. A relay cannot prove a ChainLock above the real tip,
//!   and one that hides its tip only makes the checkpoint older;
//! - the Core height Platform reports, which stops while Platform halts. It
//!   is asked of evonodes from the network's seed list, not the relay, and
//!   the height used is the highest one that more than half of the answers
//!   are at or above.
//!
//! `MAINNET_QUORUM_RELAY` or `TESTNET_QUORUM_RELAY` replaces a network's
//! public relay, which must serve both `/quorums` and `/proofs`. A network
//! that fails keeps its checkpoint, and the exit status is 1.
//! `--report` appends one Markdown line per network to a file, for the
//! release PR; if that write fails, the exit status is 2.

use dapi_grpc::platform::v0::{
    get_status_request, get_status_response, GetStatusRequest, GetStatusResponse,
};
use dash_core_proof::{bootstrap, rpc_hex, QuorumProofChain, State, MAX_PROOF_BYTES};
use dash_network_seeds::{PlatformLiveness, PlatformStatus, Reachability, SslStatus};
use dpp::dashcore::sml::llmq_type::network::NetworkLLMQExt;
use dpp::dashcore::Network;
use rs_dapi_client::{AddressList, DapiClient, DapiRequest, RequestSettings};
use rs_sdk_trusted_context_provider::types::QuorumsResponse;
use rs_sdk_trusted_context_provider::{get_quorum_base_url, VerifiedHttpContextProvider};
use serde_json::json;
use std::collections::BTreeMap;
use std::hash::{BuildHasher, RandomState};
use std::io::Write;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;
use std::time::Duration;

/// Largest `/quorums` body read. A real list is a few KiB.
const MAX_QUORUMS_BYTES: usize = 1 << 20;

/// Seed evonodes asked for Platform's Core height, and how many must answer.
const STATUS_NODES: usize = 15;
const STATUS_ANSWERS: usize = 3;

/// The exit status when the `--report` file could not be written.
const REPORT_FAILED: u8 = 2;

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
    let mut report_failed = false;
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
                report_failed = true;
            }
        }
    }
    if report_failed {
        // `release.sh` stops on this, since the report no longer matches
        // the files.
        return ExitCode::from(REPORT_FAILED);
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
    let (platform_height, answered, asked) = platform_core_height(network).await?;
    let message = refresh(
        client,
        network,
        &relay,
        &current,
        platform_height,
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
    .await?;
    Ok(format!(
        "{message} ({answered} of {asked} seed evonodes answered)"
    ))
}

/// The Core height Platform reports, as more than half of the seed evonodes
/// asked report it, with how many answered and how many were asked.
async fn platform_core_height(network: Network) -> Result<(u32, usize, usize), String> {
    let settings = RequestSettings {
        connect_timeout: Some(Duration::from_secs(5)),
        timeout: Some(Duration::from_secs(10)),
        retries: Some(0),
        ..RequestSettings::default()
    };
    sample_platform_height(status_nodes(network), |uri| async move {
        let request = GetStatusRequest {
            version: Some(get_status_request::Version::V0(
                get_status_request::GetStatusRequestV0 {},
            )),
        };
        let client = DapiClient::new(AddressList::from_str(&uri).ok()?, settings);
        status_height(request.execute(&client, settings).await.ok()?.inner)
    })
    .await
}

/// Ask every node with `ask` and take the height more than half of them
/// support. A node that does not answer supports no height.
async fn sample_platform_height<Ask, Answer>(
    nodes: Vec<String>,
    ask: Ask,
) -> Result<(u32, usize, usize), String>
where
    Ask: Fn(String) -> Answer,
    Answer: std::future::Future<Output = Option<u32>>,
{
    let asked = nodes.len();
    let heights: Vec<u32> = futures::future::join_all(nodes.into_iter().map(ask))
        .await
        .into_iter()
        .flatten()
        .collect();
    let answered = heights.len();
    majority_height(heights, asked)
        .map(|height| (height, answered, asked))
        .ok_or_else(|| {
            format!(
                "only {answered} of {asked} seed evonodes reported Platform's Core height; \
                 more than half must agree"
            )
        })
}

/// Platform's Core height from a `getStatus` response, if it has one.
fn status_height(response: GetStatusResponse) -> Option<u32> {
    match response.version? {
        get_status_response::Version::V0(status) => status.chain?.core_chain_locked_height,
    }
}

/// Up to [`STATUS_NODES`] seed evonodes whose Platform probes passed, taken
/// in turn from each /24 network in random order, so that one operator's
/// block makes up as little of the sample as the network allows.
fn status_nodes(network: Network) -> Vec<String> {
    let random = RandomState::new();
    let mut networks: BTreeMap<u64, Vec<(u64, String)>> = BTreeMap::new();
    for seed in dash_network_seeds::evo_seeds(network) {
        let (Some(port), Some(platform)) = (seed.platform_http_port, &seed.platform) else {
            continue;
        };
        let IpAddr::V4(ip) = seed.address.ip() else {
            continue;
        };
        if !probe_passed(platform) {
            continue;
        }
        let [a, b, c, _] = ip.octets();
        networks
            .entry(random.hash_one([a, b, c]))
            .or_default()
            .push((random.hash_one(ip), format!("https://{ip}:{port}")));
    }
    let mut networks: Vec<_> = networks
        .into_values()
        .map(|mut nodes| {
            nodes.sort_unstable_by_key(|(order, _)| *order);
            nodes.into_iter().map(|(_, uri)| uri)
        })
        .collect();
    let mut chosen = Vec::new();
    while chosen.len() < STATUS_NODES {
        let before = chosen.len();
        for nodes in &mut networks {
            if chosen.len() < STATUS_NODES {
                chosen.extend(nodes.next());
            }
        }
        if chosen.len() == before {
            break;
        }
    }
    chosen
}

/// Whether a seed's Platform probe connected, got an HTTP answer and saw a
/// valid certificate.
fn probe_passed(platform: &PlatformStatus) -> bool {
    platform.reachable == Reachability::Ok
        && platform.live == PlatformLiveness::Ok
        && platform.ssl == SslStatus::Valid
}

/// The highest height that more than half of the `asked` nodes, and at
/// least [`STATUS_ANSWERS`], report at or above. Raising it takes more than
/// half the nodes asked, whatever the others do; a lagging node only lowers
/// it, and a silent one supports nothing.
fn majority_height(mut heights: Vec<u32>, asked: usize) -> Option<u32> {
    let needed = (asked / 2 + 1).max(STATUS_ANSWERS);
    heights.sort_unstable_by(|a, b| b.cmp(a));
    heights.get(needed - 1).copied()
}

/// Move `current` forward through `relay`, below the Core height Platform
/// reports, writing the new checkpoint and its evidence under `crate_dir`.
async fn refresh(
    client: &reqwest::Client,
    network: Network,
    relay: &str,
    current: &State,
    platform_height: u32,
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
    let bound = tip.height.min(platform_height).saturating_sub(depth);
    let below = format!(
        "{depth} blocks below the verified tip {} and Platform's Core height {platform_height}",
        tip.height
    );
    if bound <= current.height {
        return Ok(format!("kept {}; nothing newer is {below}", current.height));
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
            "the oldest listed quorum ({}) is not {below}",
            oldest.height
        ));
    }
    let (target, chain) =
        prove(client, relay, network, current, height, &oldest.quorum_hash).await?;
    // The target and tip are proven, so this holds whatever the relay listed.
    if target.height > bound {
        return Err(format!(
            "the relay answered at {}, not {below}",
            target.height
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

    write_together(
        &[
            (
                crate_dir.join(format!("tests/data/checkpoint-evidence-{name}.json")),
                to_json(&evidence)?,
            ),
            (
                crate_dir.join(format!("checkpoints/{name}.json")),
                to_json(&target)?,
            ),
        ],
        |from, to| std::fs::rename(from, to),
    )?;
    Ok(format!(
        "{} -> {} (`{}`), verified tip {}, Platform at {platform_height}",
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
/// `rename` is `std::fs::rename` outside tests.
fn write_together(
    files: &[(PathBuf, String)],
    mut rename: impl FnMut(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), String> {
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
        if let Err(error) = rename(staging, path) {
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

    /// Both files hold their placeholders and no staging file is left.
    fn unchanged(dir: &Path) -> bool {
        [
            "checkpoints/testnet.json",
            "tests/data/checkpoint-evidence-testnet.json",
        ]
        .iter()
        .all(|file| {
            std::fs::read_to_string(dir.join(file)).ok().as_deref() == Some("old\n")
                && !dir.join(format!("{file}.new")).exists()
        })
    }

    /// Refresh through `handler` while Platform is not the limit.
    async fn run(handler: Handler, dir: &Path) -> Result<String, String> {
        run_at(handler, dir, u32::MAX).await
    }

    /// Refresh through `handler` with Platform at `platform_height`.
    async fn run_at(handler: Handler, dir: &Path, platform_height: u32) -> Result<String, String> {
        let (checkpoint, _, _) = fixtures();
        refresh(
            &reqwest::Client::new(),
            Network::Testnet,
            &relay(handler),
            &checkpoint,
            platform_height,
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
            error.contains("not 288 blocks below the verified tip"),
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

    #[tokio::test]
    async fn should_stay_below_the_core_height_of_a_halted_platform() {
        let (checkpoint, target, tip) = fixtures();
        let target_height = proven_height(&target, &checkpoint);
        let dir = crate_dir("halted");
        // Core went on to the tip, but Platform stopped at the target.
        let error = run_at(honest(target_height - 24, tip, target), &dir, target_height)
            .await
            .expect_err("a target at Platform's Core height");
        assert!(
            error.contains(&format!("Platform's Core height {target_height}")),
            "{error}"
        );
        assert!(unchanged(&dir));
    }

    #[tokio::test]
    async fn should_refuse_a_proven_target_above_the_bound_set_by_platform() {
        let (checkpoint, target, tip) = fixtures();
        let target_height = proven_height(&target, &checkpoint);
        let dir = crate_dir("platform-bound");
        // The ask passes the bound, but the relay answers one block above it.
        let error = run_at(
            honest(target_height - 100, tip, target),
            &dir,
            target_height + 287,
        )
        .await
        .expect_err("a target above Platform's bound");
        assert!(
            error.contains(&format!("answered at {target_height}")),
            "{error}"
        );
        assert!(unchanged(&dir));
    }

    #[tokio::test]
    async fn should_accept_a_target_exactly_at_the_bound_set_by_platform() {
        let (checkpoint, target, tip) = fixtures();
        let target_height = proven_height(&target, &checkpoint);
        let dir = crate_dir("platform-boundary");
        let message = run_at(
            honest(target_height - 24, tip, target),
            &dir,
            target_height + 288,
        )
        .await
        .expect("a refresh");
        assert!(
            message.contains(&format!("-> {target_height}")),
            "{message}"
        );
    }

    #[tokio::test]
    async fn should_keep_the_checkpoint_while_platform_is_not_past_it() {
        let (checkpoint, target, tip) = fixtures();
        let dir = crate_dir("kept");
        let message = run_at(
            honest(proven_height(&target, &checkpoint) - 24, tip, target),
            &dir,
            checkpoint.height + 288,
        )
        .await
        .expect("kept");
        assert!(message.starts_with("kept 1574768"), "{message}");
        assert!(unchanged(&dir));
    }

    #[test]
    fn should_take_the_height_more_than_half_the_nodes_asked_reach() {
        assert_eq!(majority_height(vec![], 15), None);
        // Two raising it while one honest node answers: 3 of 15 is no majority.
        assert_eq!(majority_height(vec![900, 900, 100], 15), None);
        assert_eq!(majority_height(vec![100; 7], 15), None);
        assert_eq!(majority_height(vec![100; 8], 15), Some(100));
        // Seven of fifteen raising it cannot move it; eight can.
        let mut heights = vec![900; 7];
        heights.extend([100; 8]);
        assert_eq!(majority_height(heights.clone(), 15), Some(100));
        heights[7] = 900;
        assert_eq!(majority_height(heights, 15), Some(900));
        // Lagging nodes lower it only once they leave no majority above;
        // half of an even count cannot raise it.
        assert_eq!(majority_height(vec![100, 100, 1, 100, 100], 5), Some(100));
        assert_eq!(majority_height(vec![100, 1, 1, 1, 100], 5), Some(1));
        assert_eq!(majority_height(vec![900, 900, 100, 100], 4), Some(100));
        // Never fewer than three answers.
        assert_eq!(majority_height(vec![100, 100], 2), None);
        assert_eq!(majority_height(vec![100, 100, 100], 3), Some(100));
    }

    fn status(chain: Option<Option<u32>>) -> GetStatusResponse {
        use get_status_response::get_status_response_v0::Chain;
        GetStatusResponse {
            version: Some(get_status_response::Version::V0(
                get_status_response::GetStatusResponseV0 {
                    chain: chain.map(|core_chain_locked_height| Chain {
                        core_chain_locked_height,
                        ..Chain::default()
                    }),
                    ..Default::default()
                },
            )),
        }
    }

    #[test]
    fn should_read_platforms_core_height_from_a_status_response() {
        assert_eq!(
            status_height(status(Some(Some(1_581_918)))),
            Some(1_581_918)
        );
        // A node whose Drive is down reports its chain without the height.
        assert_eq!(status_height(status(Some(None))), None);
        assert_eq!(status_height(status(None)), None);
        assert_eq!(status_height(GetStatusResponse { version: None }), None);
    }

    #[tokio::test]
    async fn should_count_silent_nodes_as_supporting_no_height() {
        let nodes = |count: usize| (0..count).map(|node| format!("node{node}")).collect();
        // Eight of fifteen answer, two of them high: the honest six still
        // leave no majority for the high value.
        let answers = |uri: String| async move {
            match uri
                .trim_start_matches("node")
                .parse::<u32>()
                .expect("an index")
            {
                0 | 1 => Some(900),
                2..=7 => Some(100),
                _ => None,
            }
        };
        assert_eq!(
            sample_platform_height(nodes(15), answers).await,
            Ok((100, 8, 15))
        );
        // Three answers of fifteen, two of them high, decide nothing.
        let error = sample_platform_height(nodes(15), |uri: String| async move {
            match uri.as_str() {
                "node0" | "node1" => Some(900),
                "node2" => Some(100),
                _ => None,
            }
        })
        .await
        .expect_err("no majority");
        assert!(error.contains("only 3 of 15"), "{error}");
        let error = sample_platform_height(Vec::new(), |_| async { Some(100) })
            .await
            .expect_err("no nodes");
        assert!(error.contains("only 0 of 0"), "{error}");
    }

    #[test]
    fn should_ask_probed_seed_evonodes_from_every_network_block() {
        let subnet = |uri: &str| {
            let host = uri.trim_start_matches("https://");
            host.rsplit_once('.').map(|(subnet, _)| subnet.to_string())
        };
        for network in [Network::Mainnet, Network::Testnet] {
            let nodes = status_nodes(network);
            assert_eq!(nodes.len(), STATUS_NODES, "{network}");
            let mut unique = nodes.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), nodes.len(), "{network}");
            // Every /24 with a probed node is asked before any is asked twice.
            let blocks: std::collections::BTreeSet<_> = dash_network_seeds::evo_seeds(network)
                .iter()
                .filter(|seed| {
                    seed.platform_http_port.is_some()
                        && seed.platform.as_ref().is_some_and(probe_passed)
                })
                .filter_map(|seed| subnet(&seed.address.ip().to_string()))
                .collect();
            let asked: std::collections::BTreeSet<_> =
                nodes.iter().filter_map(|uri| subnet(uri)).collect();
            assert_eq!(asked.len(), blocks.len().min(STATUS_NODES), "{network}");
        }
    }

    fn files(dir: &Path) -> [(PathBuf, String); 2] {
        [
            (
                dir.join("tests/data/checkpoint-evidence-testnet.json"),
                "new\n".to_string(),
            ),
            (dir.join("checkpoints/testnet.json"), "new\n".to_string()),
        ]
    }

    #[test]
    fn should_replace_both_files() {
        let dir = crate_dir("replace");
        write_together(&files(&dir), |from, to| std::fs::rename(from, to)).expect("written");
        for (path, _) in files(&dir) {
            assert_eq!(std::fs::read_to_string(&path).expect("a file"), "new\n");
            assert!(!path.with_extension("json.new").exists());
        }
    }

    #[test]
    fn should_leave_every_file_unchanged_when_staging_one_fails() {
        let dir = crate_dir("staging");
        let [first, _] = files(&dir);
        let error = write_together(
            &[
                first,
                (dir.join("missing/testnet.json"), "new\n".to_string()),
            ],
            |from, to| std::fs::rename(from, to),
        )
        .expect_err("a missing destination");
        assert!(error.contains("missing"), "{error}");
        assert!(unchanged(&dir));
    }

    #[test]
    fn should_restore_a_replaced_file_when_a_later_rename_fails() {
        let dir = crate_dir("rollback");
        let mut renames = 0;
        let error = write_together(&files(&dir), |from, to| {
            renames += 1;
            if renames == 2 {
                return Err(std::io::Error::other("refused"));
            }
            std::fs::rename(from, to)
        })
        .expect_err("a failed rename");
        assert!(error.contains("refused"), "{error}");
        assert_eq!(renames, 2);
        assert!(unchanged(&dir));
    }
}
