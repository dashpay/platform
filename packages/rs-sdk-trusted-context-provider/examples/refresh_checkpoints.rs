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
//! - the Core height Platform has committed, which stops while Platform
//!   halts. It is read from proved responses of evonodes in the network's
//!   seed list, each verified against a Platform quorum key proven from the
//!   current checkpoint, so (unless a Platform quorum signed a false block)
//!   no evonode can report one above Platform's own.
//!
//! `MAINNET_QUORUM_RELAY` or `TESTNET_QUORUM_RELAY` replaces a network's
//! public relay, which must serve both `/quorums` and `/proofs`. A network
//! that fails keeps its checkpoint, and the exit status is 1.
//! `--report` appends one Markdown line per network to a file, for the
//! release PR; if that write fails, the exit status is 2.

use dapi_grpc::platform::v0::{
    get_epochs_info_request, GetEpochsInfoRequest, GetEpochsInfoResponse,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dash_core_proof::{bootstrap, rpc_hex, QuorumProofChain, State, MAX_PROOF_BYTES};
use dash_network_seeds::{PlatformLiveness, PlatformStatus, Reachability, SslStatus};
use dpp::dashcore::sml::llmq_type::network::NetworkLLMQExt;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::types::ExtendedEpochInfos;
use drive_proof_verifier::FromProof;
use rs_dapi_client::{AddressList, DapiClient, DapiRequest, RequestSettings};
use rs_sdk_trusted_context_provider::types::QuorumsResponse;
use rs_sdk_trusted_context_provider::{get_quorum_base_url, VerifiedHttpContextProvider};
use serde_json::json;
use std::collections::BTreeMap;
use std::hash::{BuildHasher, RandomState};
use std::io::Write;
use std::net::IpAddr;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;
use std::time::Duration;

/// Largest `/quorums` body read. A real list is a few KiB.
const MAX_QUORUMS_BYTES: usize = 1 << 20;

/// Seed evonodes asked for Platform's Core height.
const PLATFORM_NODES: usize = 5;

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
    let (platform_height, answered, asked) =
        platform_core_height(network, &relay, &current).await?;
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
        "{message} ({answered} of {asked} seed evonodes gave a verified height)"
    ))
}

/// Platform's Core height, authenticated, with how many seed evonodes gave
/// one and how many were asked.
///
/// Each evonode is asked for a proved `getEpochsInfo`. Its metadata's Core
/// height is part of the state a Platform quorum signs, and the quorum's key
/// is proven from `current` through `relay`. Unless a threshold of a Platform
/// quorum signed a false block, the assumption behind every proof, a
/// verified height is one Platform committed, so no evonode can report one
/// above Platform's own. The highest verified height is used, and one is
/// enough.
async fn platform_core_height(
    network: Network,
    relay: &str,
    current: &State,
) -> Result<(u32, usize, usize), String> {
    let provider = VerifiedHttpContextProvider::with_checkpoint(
        network,
        current.clone(),
        vec![relay.to_string()],
        NonZeroUsize::new(16).expect("nonzero"),
    )
    .map_err(|error| error.to_string())?;
    let settings = RequestSettings {
        connect_timeout: Some(Duration::from_secs(5)),
        timeout: Some(Duration::from_secs(10)),
        retries: Some(0),
        ..RequestSettings::default()
    };
    sample_platform_height(platform_nodes(network), |uri| {
        let provider = provider.clone();
        // Each node gets its own task, so a panic while verifying one node's
        // answer only loses that answer.
        let ask = tokio::spawn({
            let uri = uri.clone();
            async move {
                let address = AddressList::from_str(&uri).map_err(|error| error.to_string())?;
                let client = DapiClient::new(address, settings);
                let request = epochs_request();
                let response = request
                    .execute(&client, settings)
                    .await
                    .map_err(|error| error.inner.to_string())?
                    .inner;
                authenticated_height(network, &provider, request, response).await
            }
        });
        async move {
            ask.await
                .map_err(|error| error.to_string())
                .and_then(|answer| answer)
                .map_err(|error| format!("{uri}: {error}"))
        }
    })
    .await
}

/// A small proved request: the first epoch, which always exists.
fn epochs_request() -> GetEpochsInfoRequest {
    GetEpochsInfoRequest {
        version: Some(get_epochs_info_request::Version::V0(
            get_epochs_info_request::GetEpochsInfoRequestV0 {
                start_epoch: Some(0),
                count: 1,
                ascending: true,
                prove: true,
            },
        )),
    }
}

/// The Core height in `response`'s metadata, once its proof verifies against
/// a quorum key `provider` proves.
async fn authenticated_height(
    network: Network,
    provider: &VerifiedHttpContextProvider,
    request: GetEpochsInfoRequest,
    response: GetEpochsInfoResponse,
) -> Result<u32, String> {
    let proof = response.proof().map_err(|error| error.to_string())?;
    let metadata = response.metadata().map_err(|error| error.to_string())?;
    let height = metadata.core_chain_locked_height;
    let quorum_hash: [u8; 32] = proof
        .quorum_hash
        .as_slice()
        .try_into()
        .map_err(|_| "a quorum hash of the wrong length".to_string())?;
    // The verifier only reads the provider's cache, so prove the key first.
    if let Some(fetch) = provider.fetch_quorum_public_key(proof.quorum_type, quorum_hash, height) {
        fetch.await.map_err(|error| error.to_string())?;
    }
    // The version only picks how the GroveDB proof is read; the quorum signs
    // the version and the root hash, so a wrong pick fails the signature.
    let version = PlatformVersion::get(metadata.protocol_version)
        .unwrap_or_else(|_| PlatformVersion::latest());
    ExtendedEpochInfos::maybe_from_proof_with_metadata(
        request, response, network, version, provider,
    )
    .map_err(|error| error.to_string())?;
    Ok(height)
}

/// Ask every node with `ask` and take the highest height. Each is
/// authenticated, so any answer is safe, and one is enough.
async fn sample_platform_height<Ask, Answer>(
    nodes: Vec<String>,
    ask: Ask,
) -> Result<(u32, usize, usize), String>
where
    Ask: Fn(String) -> Answer,
    Answer: std::future::Future<Output = Result<u32, String>>,
{
    let asked = nodes.len();
    let (heights, errors): (Vec<_>, Vec<_>) = futures::future::join_all(nodes.into_iter().map(ask))
        .await
        .into_iter()
        .partition(Result::is_ok);
    let answered = heights.len();
    match heights.into_iter().flatten().max() {
        Some(height) => Ok((height, answered, asked)),
        None => Err(format!(
            "none of {asked} seed evonodes gave an authenticated Platform Core height{}",
            errors
                .into_iter()
                .find_map(Result::err)
                .map(|error| format!(", for example {error}"))
                .unwrap_or_default()
        )),
    }
}

/// Up to [`PLATFORM_NODES`] seed evonodes whose Platform probes passed, taken
/// in turn from each /24 network in random order. Their answers are
/// authenticated, so the spread is only so one operator's outage leaves
/// others to ask.
fn platform_nodes(network: Network) -> Vec<String> {
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
    while chosen.len() < PLATFORM_NODES {
        let before = chosen.len();
        for nodes in &mut networks {
            if chosen.len() < PLATFORM_NODES {
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

    /// A real proved testnet `getEpochsInfo` response, its Core height, and
    /// the relay proof of its signing quorum from the fixtures' checkpoint.
    fn platform_fixture() -> (u32, GetEpochsInfoResponse, Vec<u8>) {
        use dapi_grpc::Message;
        let value: Value =
            serde_json::from_str(include_str!("../tests/data/platform-epochs-testnet.json"))
                .expect("fixture JSON");
        let bytes = |key: &str| hex::decode(value[key].as_str().expect("hex")).expect("hex");
        (
            value["core_chain_locked_height"]
                .as_u64()
                .expect("a height") as u32,
            GetEpochsInfoResponse::decode(bytes("response_hex").as_slice()).expect("a response"),
            bytes("quorum_proof_hex"),
        )
    }

    /// Authenticate `response` with a provider whose relay serves the
    /// fixture's quorum proof.
    async fn verify_platform_height(response: GetEpochsInfoResponse) -> Result<u32, String> {
        let (checkpoint, _, _) = fixtures();
        let (_, _, quorum_proof) = platform_fixture();
        let provider = VerifiedHttpContextProvider::with_checkpoint(
            Network::Testnet,
            checkpoint,
            vec![relay(Box::new(move |_, _| (200, quorum_proof.clone())))],
            NonZeroUsize::new(4).expect("nonzero"),
        )
        .expect("a provider");
        authenticated_height(Network::Testnet, &provider, epochs_request(), response).await
    }

    fn metadata(
        response: &mut GetEpochsInfoResponse,
    ) -> &mut dapi_grpc::platform::v0::ResponseMetadata {
        match response.version.as_mut().expect("a version") {
            dapi_grpc::platform::v0::get_epochs_info_response::Version::V0(v0) => {
                v0.metadata.as_mut().expect("metadata")
            }
        }
    }

    #[tokio::test]
    async fn should_authenticate_platforms_core_height_from_a_proved_response() {
        let (height, response, _) = platform_fixture();
        assert_eq!(verify_platform_height(response).await, Ok(height));
    }

    #[tokio::test]
    async fn should_refuse_a_proved_response_whose_core_height_was_changed() {
        let (height, mut response, _) = platform_fixture();
        // One block higher still gets the quorum's key, but the quorum
        // signed the real height.
        metadata(&mut response).core_chain_locked_height = height + 1;
        let error = verify_platform_height(response)
            .await
            .expect_err("a raised height");
        assert!(error.contains("invalid signature"), "{error}");
    }

    #[tokio::test]
    async fn should_refuse_a_proof_signed_by_a_quorum_of_another_type() {
        use dapi_grpc::platform::v0::get_epochs_info_response::{
            get_epochs_info_response_v0, Version,
        };
        let (_, mut response, _) = platform_fixture();
        let Some(Version::V0(v0)) = response.version.as_mut() else {
            panic!("a version");
        };
        let Some(get_epochs_info_response_v0::Result::Proof(proof)) = v0.result.as_mut() else {
            panic!("a proof");
        };
        proof.quorum_type = 4;
        let error = verify_platform_height(response)
            .await
            .expect_err("another quorum type");
        assert!(error.contains("is not a Platform quorum"), "{error}");
    }

    #[tokio::test]
    async fn should_refuse_epochs_returned_instead_of_a_proof() {
        use dapi_grpc::platform::v0::get_epochs_info_response::{
            get_epochs_info_response_v0, Version,
        };
        let (_, mut response, _) = platform_fixture();
        let Some(Version::V0(v0)) = response.version.as_mut() else {
            panic!("a version");
        };
        v0.result = Some(get_epochs_info_response_v0::Result::Epochs(
            get_epochs_info_response_v0::EpochInfos::default(),
        ));
        let error = verify_platform_height(response)
            .await
            .expect_err("no proof");
        assert!(error.contains("proof"), "{error}");
    }

    #[tokio::test]
    async fn should_refuse_a_response_without_a_proof() {
        let error = verify_platform_height(GetEpochsInfoResponse { version: None })
            .await
            .expect_err("no proof");
        assert!(error.contains("version"), "{error}");
    }

    #[tokio::test]
    async fn should_take_the_highest_authenticated_height_and_need_only_one() {
        let nodes = |count: usize| (0..count).map(|node| format!("node{node}")).collect();
        let answers = |uri: String| async move {
            match uri.as_str() {
                "node0" => Ok(100),
                "node1" => Ok(105),
                _ => Err(format!("{uri}: unreachable")),
            }
        };
        assert_eq!(
            sample_platform_height(nodes(5), answers).await,
            Ok((105, 2, 5))
        );
        let error = sample_platform_height(nodes(5), |uri: String| async move {
            Err::<u32, _>(format!("{uri}: bad proof"))
        })
        .await
        .expect_err("no authenticated height");
        assert!(error.contains("none of 5"), "{error}");
        assert!(error.contains("node0: bad proof"), "{error}");
        let error = sample_platform_height(Vec::new(), |_| async { Ok(100) })
            .await
            .expect_err("no nodes");
        assert!(error.contains("none of 0"), "{error}");
    }

    #[test]
    fn should_ask_probed_seed_evonodes_from_every_network_block() {
        let subnet = |uri: &str| {
            let host = uri.trim_start_matches("https://");
            host.rsplit_once('.').map(|(subnet, _)| subnet.to_string())
        };
        for network in [Network::Mainnet, Network::Testnet] {
            let nodes = platform_nodes(network);
            assert!(
                !nodes.is_empty() && nodes.len() <= PLATFORM_NODES,
                "{network}"
            );
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
            assert_eq!(asked.len(), blocks.len().min(PLATFORM_NODES), "{network}");
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
