//! A context provider whose Platform quorum keys are proven, not trusted.
//!
//! [`VerifiedHttpContextProvider`] starts from a checkpoint: a ChainLocked
//! Core block and the quorum and masternode Merkle roots its coinbase commits
//! to. The checkpoint is trust configuration, embedded in this crate per
//! network or supplied by the application. When the SDK meets a Platform
//! quorum it has no key for, the provider asks a relay for a `DASHNC02` proof
//! (DIP 175) and believes the key only if the proof verifies from a state it
//! already trusts. Relays are untrusted byte sources; a lying relay can delay
//! requests but cannot make the SDK accept a forged key or ban honest nodes.

use crate::get_quorum_base_url;
use crate::proof_fetches::{ProofFetches, Timing};
use crate::proof_transport::ProofTransport;
use crate::system_contracts::{platform_activation_height, system_data_contract};
use dash_context_provider::{ContextProvider, ContextProviderError, QuorumKeyFuture};
use dash_core_proof::{ProofNetwork, State};
use dpp::dashcore::sml::llmq_type::network::NetworkLLMQExt;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use thiserror::Error;
use url::{Host, Url};

/// Most relays one provider asks.
const MAX_SOURCES: usize = 8;

/// The embedded mainnet checkpoint, Core `ProofState` JSON.
const MAINNET_CHECKPOINT: &str = include_str!("../checkpoints/mainnet.json");
/// The embedded testnet checkpoint, Core `ProofState` JSON.
const TESTNET_CHECKPOINT: &str = include_str!("../checkpoints/testnet.json");

/// Why a [`VerifiedHttpContextProvider`] could not be built.
#[derive(Debug, Error)]
pub enum VerifiedContextProviderError {
    /// Only mainnet and testnet have the fixed ChainLock quorum types the
    /// proof format relies on.
    #[error("unsupported network for verified quorum keys: {0:?}")]
    UnsupportedNetwork(Network),

    /// The checkpoint is malformed, outside the supported era or for another
    /// network.
    #[error("invalid checkpoint: {0}")]
    InvalidCheckpoint(String),

    /// A relay URL is not one the provider may ask.
    #[error("invalid proof source: {0}")]
    InvalidSource(String),

    /// The HTTP client could not be built.
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),
}

/// A [`ContextProvider`] that serves Platform quorum keys only after proving
/// them with DIP 175 proofs from a checkpoint.
///
/// Opt in with `SdkBuilder::with_context_provider` or
/// `Sdk::set_context_provider`. Clones share their caches.
///
/// Quorum keys:
/// * [`get_quorum_public_key`](ContextProvider::get_quorum_public_key) serves
///   a cached key if the quorum was proven active within a validity window of
///   the requested height (600 blocks on mainnet and testnet). Otherwise it
///   reports the source as unavailable, and the SDK calls
///   [`fetch_quorum_public_key`](ContextProvider::fetch_quorum_public_key).
/// * The fetch POSTs to each source's `/proofs`, verifies the proof from the
///   newest verified state, caches the key and returns it. Any failure
///   (timeouts, HTTP errors, oversize or invalid proofs, a proof of another
///   quorum) is reported as an unavailable source, so the SDK moves to
///   another node without banning the one that named the quorum.
/// * A quorum of a type other than the network's Platform type is rejected
///   as invalid and never fetched.
///
/// Fetches are shared per quorum, requests for at most two quorums are in
/// flight at once, relays that cannot serve a quorum are left out of its fetch,
/// failed quorums and failing relays cool down, and each fetch has a 70 s
/// deadline. Freshness of responses is the SDK's concern, not the provider's.
///
/// Keys are proven for the Core height Platform reports. If Platform halts,
/// that height stops while Core moves on, and a quorum not cached before the
/// halt may become unprovable once it leaves Core's quorum root.
///
/// A proof grows with the age of its starting state. Long-lived applications
/// should persist [`latest_verified_state`](Self::latest_verified_state) and
/// pass it back as the checkpoint on their next start.
///
/// The public mainnet relay currently answers `/proofs` with HTTP 403, so on
/// mainnet [`new`](Self::new) builds but fetches fail until a relay serves
/// proofs; pass working sources to [`with_checkpoint`](Self::with_checkpoint).
///
/// Data contracts and token configurations come from the known caches below
/// and the system contracts enabled by cargo features; nothing else is
/// fetched.
#[derive(Clone)]
pub struct VerifiedHttpContextProvider {
    inner: Arc<Inner>,
}

struct Inner {
    network: Network,
    sources: Vec<Url>,
    fetches: Arc<ProofFetches>,
    known_contracts: Mutex<HashMap<Identifier, Arc<DataContract>>>,
    known_token_configurations: Mutex<HashMap<Identifier, TokenConfiguration>>,
}

/// Lock a cache; every update under these locks completes before release.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn proof_network(network: Network) -> Result<ProofNetwork, VerifiedContextProviderError> {
    match network {
        Network::Mainnet => Ok(ProofNetwork::Mainnet),
        Network::Testnet => Ok(ProofNetwork::Testnet),
        other => Err(VerifiedContextProviderError::UnsupportedNetwork(other)),
    }
}

/// A checkpoint a proof can start from.
fn validated(checkpoint: State) -> Result<State, VerifiedContextProviderError> {
    checkpoint
        .validate()
        .map_err(|error| VerifiedContextProviderError::InvalidCheckpoint(error.to_string()))?;
    Ok(checkpoint)
}

/// Parse a relay base URL. It must be absolute `https`, or `http` to a
/// loopback host, and carry no credentials, query or fragment. The host is
/// not resolved.
fn parse_source(text: &str) -> Result<Url, VerifiedContextProviderError> {
    let invalid =
        |why: &str| VerifiedContextProviderError::InvalidSource(format!("'{text}' {why}"));
    let url = Url::parse(text).map_err(|error| invalid(&format!("is not a URL: {error}")))?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid("must not carry credentials"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(invalid("must not carry a query or fragment"));
    }
    let loopback = match url.host() {
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        None => return Err(invalid("has no host")),
    };
    match url.scheme() {
        "https" => Ok(url),
        "http" if loopback => Ok(url),
        "http" => Err(invalid("must use https unless it is a loopback address")),
        _ => Err(invalid("must use https")),
    }
}

/// The `/proofs` endpoint under a relay base URL.
fn proofs_endpoint(source: &Url) -> Result<Url, VerifiedContextProviderError> {
    let mut endpoint = source.clone();
    endpoint
        .path_segments_mut()
        .map_err(|()| {
            VerifiedContextProviderError::InvalidSource(format!("'{source}' cannot be a base"))
        })?
        .pop_if_empty()
        .push("proofs");
    Ok(endpoint)
}

impl VerifiedHttpContextProvider {
    /// A provider for mainnet or testnet that starts from the embedded
    /// checkpoint and asks the network's public quorum service. Makes no
    /// network request.
    ///
    /// `cache_size` is how many proven quorum keys are kept. It should be at
    /// least the number of active Platform quorums (24 on mainnet and
    /// testnet); 32 or more avoids proving keys again after eviction.
    pub fn new(
        network: Network,
        cache_size: NonZeroUsize,
    ) -> Result<Self, VerifiedContextProviderError> {
        let checkpoint = Self::embedded_checkpoint(network)?;
        let source = get_quorum_base_url(network, None)
            .map_err(|_| VerifiedContextProviderError::UnsupportedNetwork(network))?;
        Self::with_checkpoint(network, checkpoint, vec![source], cache_size)
    }

    /// A provider that starts from `checkpoint` and asks `sources`, 1 to 8
    /// relay base URLs whose `/proofs` endpoint serves Core's
    /// `getquorumproofchain` bootstrap proofs.
    ///
    /// The checkpoint is trust configuration: a ChainLocked block the
    /// application checked independently, or a state this provider verified
    /// earlier ([`latest_verified_state`](Self::latest_verified_state)).
    /// Sources must be `https`, except on loopback hosts, and distinct. Hosts
    /// are not resolved here. `cache_size` is as for [`new`](Self::new).
    pub fn with_checkpoint(
        network: Network,
        checkpoint: State,
        sources: Vec<String>,
        cache_size: NonZeroUsize,
    ) -> Result<Self, VerifiedContextProviderError> {
        Self::build(network, checkpoint, sources, cache_size, Timing::default())
    }

    fn build(
        network: Network,
        checkpoint: State,
        sources: Vec<String>,
        cache_size: NonZeroUsize,
        timing: Timing,
    ) -> Result<Self, VerifiedContextProviderError> {
        let expected = proof_network(network)?;
        let checkpoint = validated(checkpoint)?;
        if checkpoint.network != expected {
            return Err(VerifiedContextProviderError::InvalidCheckpoint(format!(
                "checkpoint is for {:?}, provider for {network:?}",
                checkpoint.network
            )));
        }
        if !(1..=MAX_SOURCES).contains(&sources.len()) {
            return Err(VerifiedContextProviderError::InvalidSource(format!(
                "{} sources given, expected 1 to {MAX_SOURCES}",
                sources.len()
            )));
        }
        let sources = sources
            .iter()
            .map(|source| parse_source(source))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(repeated) = sources
            .iter()
            .enumerate()
            .find_map(|(index, source)| sources[..index].contains(source).then_some(source))
        {
            return Err(VerifiedContextProviderError::InvalidSource(format!(
                "'{repeated}' is given twice"
            )));
        }
        let endpoints = sources
            .iter()
            .map(proofs_endpoint)
            .collect::<Result<Vec<_>, _>>()?;

        let platform_type = network.platform_type();
        // A Platform quorum signs while it is among the newest
        // `signing_active_quorum_count` quorums, and one is formed every DKG
        // interval; one more interval covers a quorum mined late in it.
        let params = platform_type.params();
        let window = params
            .dkg_params
            .interval
            .saturating_mul(params.signing_active_quorum_count.saturating_add(1));
        let fetches = ProofFetches::new(
            u32::from(u8::from(platform_type)),
            window,
            checkpoint,
            endpoints,
            cache_size,
            ProofTransport::new()?,
            timing,
        );
        Ok(Self {
            inner: Arc::new(Inner {
                network,
                sources,
                fetches: Arc::new(fetches),
                known_contracts: Mutex::new(HashMap::new()),
                known_token_configurations: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// The checkpoint this crate embeds for `network`, validated.
    pub fn embedded_checkpoint(network: Network) -> Result<State, VerifiedContextProviderError> {
        let json = match proof_network(network)? {
            ProofNetwork::Mainnet => MAINNET_CHECKPOINT,
            ProofNetwork::Testnet => TESTNET_CHECKPOINT,
        };
        serde_json::from_str(json)
            .map_err(|error| VerifiedContextProviderError::InvalidCheckpoint(error.to_string()))
            .and_then(validated)
    }

    /// The checkpoint the provider was built with.
    pub fn checkpoint(&self) -> &State {
        self.inner.fetches.checkpoint()
    }

    /// The highest state a proof has verified, or the checkpoint. An
    /// application may persist it and pass it back as the checkpoint, so
    /// later proofs are shorter.
    pub fn latest_verified_state(&self) -> State {
        self.inner.fetches.latest_state()
    }

    /// The relay base URLs the provider asks.
    pub fn sources(&self) -> &[Url] {
        &self.inner.sources
    }

    /// How many proven quorum keys are cached.
    pub fn cached_quorum_count(&self) -> usize {
        self.inner.fetches.cached_quorum_count()
    }

    /// Set known contracts that will be served immediately.
    pub fn with_known_contracts(self, contracts: Vec<DataContract>) -> Self {
        self.add_known_contracts(contracts);
        self
    }

    /// Add a data contract to the known contracts cache.
    pub fn add_known_contract(&self, contract: DataContract) {
        lock(&self.inner.known_contracts).insert(contract.id(), Arc::new(contract));
    }

    /// Add data contracts to the known contracts cache.
    pub fn add_known_contracts(&self, contracts: Vec<DataContract>) {
        let mut known = lock(&self.inner.known_contracts);
        for contract in contracts {
            known.insert(contract.id(), Arc::new(contract));
        }
    }

    /// Insert a contract only when its version is strictly newer, returning
    /// the retained entry. Equal versions preserve the existing body.
    pub fn add_known_contract_if_newer(&self, contract: DataContract) -> Arc<DataContract> {
        let mut known = lock(&self.inner.known_contracts);
        let id = contract.id();
        if let Some(existing) = known.get(&id) {
            if existing.version() >= contract.version() {
                return Arc::clone(existing);
            }
        }
        let contract = Arc::new(contract);
        known.insert(id, Arc::clone(&contract));
        contract
    }

    /// A data contract from the known contracts cache.
    pub fn get_known_contract(&self, id: &Identifier) -> Option<Arc<DataContract>> {
        lock(&self.inner.known_contracts).get(id).cloned()
    }

    /// Remove a data contract from the known contracts cache, returning
    /// whether it was there.
    pub fn remove_known_contract(&self, id: &Identifier) -> bool {
        lock(&self.inner.known_contracts).remove(id).is_some()
    }

    /// Add a token configuration to the known token configurations cache.
    pub fn add_known_token_configuration(&self, token_id: Identifier, config: TokenConfiguration) {
        lock(&self.inner.known_token_configurations).insert(token_id, config);
    }

    /// Add token configurations to the known token configurations cache.
    pub fn add_known_token_configurations(&self, configs: Vec<(Identifier, TokenConfiguration)>) {
        lock(&self.inner.known_token_configurations).extend(configs);
    }
}

impl ContextProvider for VerifiedHttpContextProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        if let Some(contract) = self.get_known_contract(id) {
            return Ok(Some(contract));
        }
        system_data_contract(id, platform_version)
    }

    fn register_data_contract(&self, contract: Arc<DataContract>) {
        lock(&self.inner.known_contracts).insert(contract.id(), contract);
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(lock(&self.inner.known_token_configurations)
            .get(token_id)
            .cloned())
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        self.inner
            .fetches
            .lookup(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn fetch_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Option<QuorumKeyFuture> {
        if !self
            .inner
            .fetches
            .provable(quorum_type, core_chain_locked_height)
        {
            return None;
        }
        let fetches = Arc::clone(&self.inner.fetches);
        // Never `Ok(None)`: a proof can only show that a quorum exists, so a
        // failure to prove one says nothing about the node that named it.
        Some(Box::pin(async move {
            fetches
                .fetch(quorum_type, quorum_hash, core_chain_locked_height)
                .await
                .map(Some)
        }))
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(platform_activation_height(self.inner.network))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use dash_core_proof::bootstrap;
    use dash_core_proof::{parse_commitment, rpc_hex, MAX_PROOF_BYTES};
    use futures::future::join_all;
    use serde_json::{json, Value};
    use std::collections::VecDeque;
    use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::thread;
    use std::time::{Duration, Instant};

    /// A real `getquorumproofchain` result and the quorum it opens.
    struct Fixture {
        network: Network,
        checkpoint: State,
        bootstrap: Vec<u8>,
        target: State,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        key: [u8; 48],
    }

    fn fixture(network: Network) -> Fixture {
        let json = match network {
            Network::Mainnet => include_str!("../tests/data/proof-mainnet.json"),
            _ => include_str!("../tests/data/proof-testnet.json"),
        };
        let value: Value = serde_json::from_str(json).expect("fixture JSON");
        let state = |key: &str| serde_json::from_value::<State>(value[key].clone()).expect(key);
        let bootstrap = hex::decode(value["bootstrap_hex"].as_str().expect("bootstrap_hex"))
            .expect("bootstrap hex");
        let checkpoint = state("checkpoint");
        let verified = bootstrap::verify(&bootstrap, &checkpoint, 0).expect("fixture verifies");
        let commitment = parse_commitment(verified.records()[0].leaf).expect("a commitment");
        let mut quorum_hash = commitment.quorum_hash;
        quorum_hash.reverse();
        Fixture {
            network,
            checkpoint,
            target: state("target"),
            bootstrap,
            quorum_type: u32::from(commitment.quorum_type),
            quorum_hash,
            key: commitment.public_key,
        }
    }

    /// What the mock relay does with one connection.
    enum Answer {
        Reply {
            status: u16,
            headers: Vec<(&'static str, String)>,
            body: Vec<u8>,
        },
        /// A chunked body of `chunks` chunks of `size` bytes.
        Chunked { size: usize, chunks: usize },
        /// Read the request, then hold the connection open without answering.
        Hang(Duration),
        /// Answer with this status and body after a delay, without holding
        /// up later connections.
        Late(Duration, u16, Vec<u8>),
    }

    fn reply(status: u16, body: Vec<u8>) -> Answer {
        Answer::Reply {
            status,
            headers: Vec::new(),
            body,
        }
    }

    /// A relay on a loopback port that answers connections in order and
    /// records each request body. It stops after its answers, or when no
    /// connection comes for a while.
    struct MockRelay {
        url: String,
        requests: Arc<Mutex<Vec<(Instant, Value)>>>,
        most_open: Arc<AtomicUsize>,
    }

    /// Counts a connection as open until dropped.
    struct Open(Arc<AtomicUsize>);

    impl Drop for Open {
        fn drop(&mut self) {
            self.0.fetch_sub(1, AtomicOrdering::SeqCst);
        }
    }

    impl MockRelay {
        /// Answer connections in the order they arrive.
        fn spawn(answers: Vec<Answer>) -> Self {
            let count = answers.len();
            let mut answers = answers.into_iter();
            Self::serve(count, move |_| answers.next())
        }

        /// Answer each request from the queue of the quorum it names.
        fn spawn_by_quorum(queues: Vec<([u8; 32], Vec<Answer>)>) -> Self {
            let count = queues.iter().map(|(_, answers)| answers.len()).sum();
            let mut queues: HashMap<String, VecDeque<Answer>> = queues
                .into_iter()
                .map(|(hash, answers)| (hex::encode(hash), answers.into()))
                .collect();
            Self::serve(count, move |request| {
                let hash = request["quorumHash"].as_str()?;
                queues.get_mut(hash)?.pop_front()
            })
        }

        fn serve(
            count: usize,
            mut answer_for: impl FnMut(&Value) -> Option<Answer> + Send + 'static,
        ) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock relay");
            listener
                .set_nonblocking(true)
                .expect("make the mock relay bounded");
            let url = format!("http://{}", listener.local_addr().expect("relay address"));
            let requests = Arc::new(Mutex::new(Vec::new()));
            let recorded = Arc::clone(&requests);
            let open = Arc::new(AtomicUsize::new(0));
            let most_open = Arc::new(AtomicUsize::new(0));
            let most = Arc::clone(&most_open);
            thread::spawn(move || {
                for _ in 0..count {
                    let Some(mut stream) = accept(&listener) else {
                        return;
                    };
                    let now_open = open.fetch_add(1, AtomicOrdering::SeqCst) + 1;
                    most.fetch_max(now_open, AtomicOrdering::SeqCst);
                    let guard = Open(Arc::clone(&open));
                    let body = read_request(&stream);
                    let answer = answer_for(&body);
                    lock(&recorded).push((Instant::now(), body));
                    let Some(answer) = answer else {
                        continue;
                    };
                    match answer {
                        Answer::Reply {
                            status,
                            headers,
                            body,
                        } => {
                            let mut head = format!(
                                "HTTP/1.1 {status} Mock\r\nConnection: close\r\n\
                                 Content-Type: application/octet-stream\r\n"
                            );
                            if !headers.iter().any(|(name, _)| *name == "Content-Length") {
                                head.push_str(&format!("Content-Length: {}\r\n", body.len()));
                            }
                            for (name, value) in headers {
                                head.push_str(&format!("{name}: {value}\r\n"));
                            }
                            head.push_str("\r\n");
                            // The client may hang up early; that is its right.
                            let _ = stream.write_all(head.as_bytes());
                            let _ = stream.write_all(&body);
                        }
                        Answer::Chunked { size, chunks } => {
                            let _ = stream.write_all(
                                b"HTTP/1.1 200 OK\r\nConnection: close\r\n\
                                  Transfer-Encoding: chunked\r\n\r\n",
                            );
                            let chunk = vec![0u8; size];
                            for _ in 0..chunks {
                                let written = stream
                                    .write_all(format!("{size:x}\r\n").as_bytes())
                                    .and_then(|()| stream.write_all(&chunk))
                                    .and_then(|()| stream.write_all(b"\r\n"));
                                if written.is_err() {
                                    break;
                                }
                            }
                            let _ = stream.write_all(b"0\r\n\r\n");
                        }
                        Answer::Hang(duration) => {
                            thread::spawn(move || {
                                thread::sleep(duration);
                                drop(stream);
                                drop(guard);
                            });
                        }
                        Answer::Late(delay, status, body) => {
                            thread::spawn(move || {
                                thread::sleep(delay);
                                let head = format!(
                                    "HTTP/1.1 {status} Mock\r\nConnection: close\r\n\
                                     Content-Length: {}\r\n\r\n",
                                    body.len()
                                );
                                let _ = stream.write_all(head.as_bytes());
                                let _ = stream.write_all(&body);
                                drop(stream);
                                drop(guard);
                            });
                        }
                    }
                }
            });
            Self {
                url,
                requests,
                most_open,
            }
        }

        fn requests(&self) -> Vec<Value> {
            lock(&self.requests)
                .iter()
                .map(|(_, body)| body.clone())
                .collect()
        }

        /// When each request for `quorum_hash` arrived.
        fn arrivals(&self, quorum_hash: [u8; 32]) -> Vec<Instant> {
            let hash = hex::encode(quorum_hash);
            lock(&self.requests)
                .iter()
                .filter(|(_, body)| body["quorumHash"] == hash.as_str())
                .map(|(at, _)| *at)
                .collect()
        }

        /// The `height` of each request, in arrival order.
        fn heights(&self) -> Vec<Value> {
            self.requests()
                .iter()
                .map(|body| body["height"].clone())
                .collect()
        }

        fn most_open(&self) -> usize {
            self.most_open.load(AtomicOrdering::SeqCst)
        }
    }

    fn accept(listener: &TcpListener) -> Option<TcpStream> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    // An accepted socket inherits non-blocking mode on BSDs.
                    stream
                        .set_nonblocking(false)
                        .expect("make the request blocking");
                    return Some(stream);
                }
                Err(error)
                    if error.kind() == ErrorKind::WouldBlock && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => return None,
            }
        }
    }

    /// Read one request and return its JSON body.
    fn read_request(stream: &TcpStream) -> Value {
        let mut reader = BufReader::new(stream.try_clone().expect("clone the request stream"));
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read a request line");
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().expect("a content length");
                }
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).expect("read the request body");
        serde_json::from_slice(&body).expect("a JSON request body")
    }

    /// The URL of a loopback port nothing listens on.
    fn refused_url() -> String {
        let closed = TcpListener::bind("127.0.0.1:0").expect("bind");
        format!("http://{}", closed.local_addr().expect("address"))
    }

    fn cache_size() -> NonZeroUsize {
        NonZeroUsize::new(8).expect("non-zero")
    }

    /// Short retry delays; everything else as in production.
    fn quick() -> Timing {
        Timing {
            first_retry: Duration::from_millis(5),
            ..Timing::default()
        }
    }

    fn provider_for(
        f: &Fixture,
        sources: Vec<String>,
        timing: Timing,
    ) -> VerifiedHttpContextProvider {
        VerifiedHttpContextProvider::build(
            f.network,
            f.checkpoint.clone(),
            sources,
            cache_size(),
            timing,
        )
        .expect("a provider")
    }

    async fn fetch(
        provider: &VerifiedHttpContextProvider,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        height: u32,
    ) -> Result<Option<[u8; 48]>, ContextProviderError> {
        provider
            .fetch_quorum_public_key(quorum_type, quorum_hash, height)
            .expect("a Platform quorum is fetched")
            .await
    }

    fn unavailable_reason(result: Result<Option<[u8; 48]>, ContextProviderError>) -> String {
        match result {
            Err(ContextProviderError::QuorumSourceUnavailable(reason)) => reason,
            other => panic!("expected an unavailable source, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_prove_cache_and_return_keys_from_real_relay_proofs() {
        for network in [Network::Mainnet, Network::Testnet] {
            let f = fixture(network);
            let relay = MockRelay::spawn(vec![reply(200, f.bootstrap.clone())]);
            let provider = provider_for(&f, vec![relay.url.clone()], quick());
            let height = f.target.height;
            assert!(matches!(
                provider.get_quorum_public_key(f.quorum_type, f.quorum_hash, height),
                Err(ContextProviderError::QuorumSourceUnavailable(_))
            ));

            let key = fetch(&provider, f.quorum_type, f.quorum_hash, height).await;
            assert_eq!(key.expect("a proven key"), Some(f.key), "{network:?}");
            assert_eq!(
                provider
                    .get_quorum_public_key(f.quorum_type, f.quorum_hash, height)
                    .expect("cached"),
                f.key
            );
            assert_eq!(provider.cached_quorum_count(), 1);
            assert_eq!(provider.latest_verified_state(), f.target);
            assert_eq!(
                relay.requests(),
                [json!({
                    "checkpoint": rpc_hex(&f.checkpoint.block_hash),
                    "height": height,
                    "quorumHash": hex::encode(f.quorum_hash),
                    "llmqType": f.quorum_type,
                    "nodeCount": 0,
                })]
            );
        }
    }

    #[test]
    fn should_open_the_mainnet_platform_quorum_named_in_rpc_order() {
        let f = fixture(Network::Mainnet);
        assert_eq!(f.quorum_type, 4);
        assert_eq!(
            hex::encode(f.quorum_hash),
            "000000000000001e3234155b7645fdd5af10c59bd1507ee6d75ded0ae4eab979"
        );
        assert_eq!(fixture(Network::Testnet).quorum_type, 6);
    }

    #[tokio::test]
    async fn should_serve_a_key_inside_the_window_and_prove_it_again_past_it() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![
            reply(200, f.bootstrap.clone()),
            reply(500, b"later".to_vec()),
        ]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let proven = f.target.height;
        fetch(&provider, f.quorum_type, f.quorum_hash, proven)
            .await
            .expect("proven");

        assert!(provider
            .get_quorum_public_key(f.quorum_type, f.quorum_hash, proven + 600)
            .is_ok());
        assert!(provider
            .get_quorum_public_key(f.quorum_type, f.quorum_hash, 0)
            .is_ok());
        let past = proven + 601;
        assert!(matches!(
            provider.get_quorum_public_key(f.quorum_type, f.quorum_hash, past),
            Err(ContextProviderError::QuorumSourceUnavailable(_))
        ));

        // The new proof starts from the verified target, not the checkpoint.
        let reason = unavailable_reason(fetch(&provider, f.quorum_type, f.quorum_hash, past).await);
        assert!(reason.contains("HTTP 500: later"), "{reason}");
        let requests = relay.requests();
        assert_eq!(requests[1]["checkpoint"], rpc_hex(&f.target.block_hash));
        assert_eq!(requests[1]["height"], past);
        assert!(
            provider
                .get_quorum_public_key(f.quorum_type, f.quorum_hash, proven + 600)
                .is_ok(),
            "a failed proof does not take back an earlier one"
        );
    }

    #[tokio::test]
    async fn should_send_one_request_for_concurrent_fetches_of_a_quorum() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![reply(200, f.bootstrap.clone())]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let fetches =
            (0..16).map(|_| fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height));
        for key in join_all(fetches).await {
            assert_eq!(key.expect("a proven key"), Some(f.key));
        }
        assert_eq!(relay.requests().len(), 1);
    }

    #[tokio::test]
    async fn should_give_up_on_a_relay_that_hangs_before_the_deadline() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![Answer::Hang(Duration::from_secs(5))]);
        let timing = Timing {
            deadline: Duration::from_millis(800),
            request_timeout: Duration::from_millis(400),
            ..quick()
        };
        let provider = provider_for(&f, vec![relay.url.clone()], timing);
        let started = Instant::now();
        let reason = unavailable_reason(
            fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await,
        );
        assert!(started.elapsed() < Duration::from_secs(5), "{reason}");
        assert!(reason.contains(&relay.url), "names the source: {reason}");
        assert_eq!(provider.cached_quorum_count(), 0);
    }

    #[tokio::test]
    async fn should_cool_down_a_refused_quorum_and_a_failing_source() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![reply(403, b"forbidden".to_vec())]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let reason = unavailable_reason(
            fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await,
        );
        assert!(reason.contains("HTTP 403: forbidden"), "{reason}");

        let again = unavailable_reason(
            fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await,
        );
        assert!(again.contains("retry in 5 s"), "{again}");
        let other =
            unavailable_reason(fetch(&provider, f.quorum_type, [0x11; 32], f.target.height).await);
        assert!(other.contains("cooling down"), "{other}");
        assert_eq!(relay.requests().len(), 1, "cooling down sends nothing");
    }

    #[tokio::test]
    async fn should_report_a_refused_connection_as_an_unavailable_source() {
        let f = fixture(Network::Testnet);
        let provider = provider_for(&f, vec![refused_url()], quick());
        let started = Instant::now();
        unavailable_reason(fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await);
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn should_ask_for_the_tip_after_a_bad_gateway() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![
            reply(502, b"Requested quorum is not in the target root".to_vec()),
            reply(200, f.bootstrap.clone()),
        ]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let height = f.target.height;
        let key = fetch(&provider, f.quorum_type, f.quorum_hash, height).await;
        assert_eq!(key.expect("a proven key"), Some(f.key));
        let heights = relay.heights();
        assert_eq!(heights, [json!(height), json!(0)]);
    }

    #[tokio::test]
    async fn should_not_let_a_bogus_height_hold_back_an_honest_one() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![
            reply(502, b"height beyond the tip".to_vec()),
            reply(200, f.bootstrap.clone()),
            reply(200, f.bootstrap.clone()),
        ]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let bogus = i32::MAX as u32;
        let reason =
            unavailable_reason(fetch(&provider, f.quorum_type, f.quorum_hash, bogus).await);
        assert!(reason.contains("latest ChainLock"), "{reason}");

        // Neither the quorum nor the relay cools down for the bogus height.
        let honest = f.target.height;
        let key = fetch(&provider, f.quorum_type, f.quorum_hash, honest).await;
        assert_eq!(key.expect("the honest height is proven"), Some(f.key));
        let heights = relay.heights();
        assert_eq!(heights, [json!(bogus), json!(0), json!(honest)]);
    }

    #[tokio::test]
    async fn should_prove_an_honest_height_while_a_bogus_one_is_in_flight() {
        let f = fixture(Network::Testnet);
        let bogus = i32::MAX as u32;
        let bootstrap = f.bootstrap.clone();
        let relay = MockRelay::serve(2, move |request| {
            Some(if request["height"] == bogus {
                Answer::Hang(Duration::from_secs(20))
            } else {
                reply(200, bootstrap.clone())
            })
        });
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let stuck = provider
            .fetch_quorum_public_key(f.quorum_type, f.quorum_hash, bogus)
            .expect("a Platform quorum is fetched");
        let stuck = tokio::spawn(stuck);
        let asked = Instant::now();
        while relay.requests().is_empty() && asked.elapsed() < Duration::from_secs(10) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(relay.heights(), [json!(bogus)]);

        let key = fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await;
        assert_eq!(key.expect("the honest height is proven"), Some(f.key));
        assert!(!stuck.is_finished(), "the bogus fetch is still waiting");
        stuck.abort();
    }

    #[tokio::test]
    async fn should_release_the_provider_when_every_caller_of_a_pending_fetch_leaves() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![Answer::Hang(Duration::from_secs(20))]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let inner = Arc::downgrade(&provider.inner);
        let fetches = Arc::downgrade(&provider.inner.fetches);
        let mut pending = provider
            .fetch_quorum_public_key(f.quorum_type, f.quorum_hash, f.target.height)
            .expect("a Platform quorum is fetched");
        let asked = Instant::now();
        while relay.requests().is_empty() && asked.elapsed() < Duration::from_secs(10) {
            let polled = tokio::time::timeout(Duration::from_millis(20), &mut pending).await;
            assert!(polled.is_err(), "the relay hangs");
        }
        assert_eq!(relay.requests().len(), 1, "the request is in flight");

        drop(pending);
        drop(provider);
        assert!(fetches.upgrade().is_none(), "the fetch state is released");
        assert!(inner.upgrade().is_none(), "the provider is released");
    }

    #[tokio::test]
    async fn should_stop_asking_relays_that_cannot_serve_a_fake_quorum() {
        let f = fixture(Network::Testnet);
        let gateway = || (0..6).map(|_| reply(502, Vec::new())).collect::<Vec<_>>();
        let relays = [MockRelay::spawn(gateway()), MockRelay::spawn(gateway())];
        let sources = relays.iter().map(|relay| relay.url.clone()).collect();
        let provider = provider_for(&f, sources, quick());
        let started = Instant::now();
        let reason =
            unavailable_reason(fetch(&provider, f.quorum_type, [0x11; 32], f.target.height).await);
        assert!(reason.contains("not asked again"), "{reason}");
        assert!(started.elapsed() < Duration::from_secs(20), "{reason}");
        for relay in &relays {
            let heights = relay.heights();
            assert_eq!(heights, [json!(f.target.height), json!(0)], "{reason}");
        }
    }

    #[tokio::test]
    async fn should_hedge_to_another_source_while_one_hangs() {
        let f = fixture(Network::Testnet);
        let hung = MockRelay::spawn(vec![Answer::Hang(Duration::from_secs(20))]);
        let answering = MockRelay::spawn(vec![reply(200, f.bootstrap.clone())]);
        let timing = Timing {
            hedge_after: Duration::from_millis(200),
            ..quick()
        };
        let sources = vec![hung.url.clone(), answering.url.clone()];
        let provider = provider_for(&f, sources, timing);
        let key = fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await;
        assert_eq!(key.expect("the second source proves it"), Some(f.key));
        assert_eq!(hung.requests().len(), 1);
        assert_eq!(answering.requests().len(), 1);
    }

    #[tokio::test]
    async fn should_prove_with_another_source_after_one_fails() {
        let f = fixture(Network::Testnet);
        let failing = MockRelay::spawn(vec![reply(403, Vec::new())]);
        let answering = MockRelay::spawn(vec![reply(200, f.bootstrap.clone())]);
        let sources = vec![failing.url.clone(), answering.url.clone()];
        let provider = provider_for(&f, sources, quick());
        let key = fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await;
        assert_eq!(key.expect("the second source proves it"), Some(f.key));
        assert_eq!(failing.requests().len(), 1);
        assert_eq!(answering.requests().len(), 1);
    }

    #[tokio::test]
    async fn should_leave_out_only_the_source_that_is_behind() {
        let f = fixture(Network::Testnet);
        // A proof from another checkpoint whose target is two blocks below
        // the fixture's: its height is judged before anything else.
        let behind: Value =
            serde_json::from_str(include_str!("../tests/data/proof-testnet-no-links.json"))
                .expect("fixture JSON");
        let behind = hex::decode(behind["bootstrap_hex"].as_str().expect("bootstrap_hex"))
            .expect("bootstrap hex");
        let lagging = MockRelay::spawn(vec![reply(502, Vec::new()), reply(200, behind)]);
        let proving = MockRelay::spawn(vec![Answer::Late(
            Duration::from_millis(500),
            200,
            f.bootstrap.clone(),
        )]);
        let timing = Timing {
            hedge_after: Duration::from_millis(50),
            ..quick()
        };
        let provider = provider_for(&f, vec![lagging.url.clone(), proving.url.clone()], timing);
        // The fixture's target is exactly the lowest one this height accepts.
        let height = f.target.height + 600;
        let key = fetch(&provider, f.quorum_type, f.quorum_hash, height).await;
        assert_eq!(key.expect("the other source proves it"), Some(f.key));
        let heights = lagging.heights();
        assert_eq!(heights, [json!(height), json!(0)]);
        assert_eq!(proving.requests().len(), 1);
    }

    #[tokio::test]
    async fn should_end_a_fetch_once_every_source_is_behind() {
        let f = fixture(Network::Testnet);
        let behind = || vec![reply(502, Vec::new()), reply(200, f.bootstrap.clone())];
        let relays = [MockRelay::spawn(behind()), MockRelay::spawn(behind())];
        let sources = relays.iter().map(|relay| relay.url.clone()).collect();
        let provider = provider_for(&f, sources, quick());
        let reason = unavailable_reason(
            fetch(&provider, f.quorum_type, f.quorum_hash, i32::MAX as u32).await,
        );
        assert_eq!(reason.matches("latest ChainLock").count(), 2, "{reason}");
        for relay in &relays {
            assert_eq!(relay.requests().len(), 2, "{reason}");
        }
        assert_eq!(provider.inner.fetches.cooling(), [false, false]);
    }

    #[tokio::test]
    async fn should_wait_for_a_place_before_asking_again_after_a_backoff() {
        let f = fixture(Network::Testnet);
        let (b, c) = ([0x21; 32], [0x22; 32]);
        let slow_gateway = || {
            vec![
                Answer::Late(Duration::from_millis(1500), 502, Vec::new()),
                reply(502, Vec::new()),
            ]
        };
        let relay = MockRelay::spawn_by_quorum(vec![
            (
                f.quorum_hash,
                vec![reply(503, Vec::new()), reply(200, f.bootstrap.clone())],
            ),
            (b, slow_gateway()),
            (c, slow_gateway()),
        ]);
        let timing = Timing {
            first_retry: Duration::from_millis(300),
            deadline: Duration::from_secs(10),
            ..Timing::default()
        };
        let provider = provider_for(&f, vec![relay.url.clone()], timing);
        let height = f.target.height;
        let first = fetch(&provider, f.quorum_type, f.quorum_hash, height);
        let later = async {
            // B and C start while the first quorum waits out the 503.
            tokio::time::sleep(Duration::from_millis(100)).await;
            futures::join!(
                fetch(&provider, f.quorum_type, b, height),
                fetch(&provider, f.quorum_type, c, height)
            )
        };
        let (key, (b_result, c_result)) = futures::join!(first, later);
        assert_eq!(key.expect("proven once a place is free"), Some(f.key));
        assert!(b_result.is_err() && c_result.is_err());

        let first_asked = relay.arrivals(f.quorum_hash);
        let b_asked = relay.arrivals(b);
        assert_eq!(first_asked.len(), 2);
        // Without the limit the retry would follow the 503 after 300 ms; it
        // came only once B and C had their answers, 1.5 s after asking.
        assert!(
            first_asked[1] >= b_asked[0] + Duration::from_millis(1400),
            "asked again too early"
        );
        assert!(
            relay.most_open() <= 2,
            "{} requests open at once",
            relay.most_open()
        );
    }

    #[tokio::test]
    async fn should_cool_down_a_source_that_fails_early_on_a_request_the_deadline_shortened() {
        let f = fixture(Network::Testnet);
        let hung = MockRelay::spawn(vec![Answer::Hang(Duration::from_secs(20))]);
        let timing = Timing {
            deadline: Duration::from_secs(2),
            hedge_after: Duration::from_millis(100),
            ..quick()
        };
        let provider = provider_for(&f, vec![hung.url.clone(), refused_url()], timing);
        unavailable_reason(fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await);
        // The hung request was cut short by the deadline; the refusal was not.
        assert_eq!(provider.inner.fetches.cooling(), [false, true]);
    }

    #[tokio::test]
    async fn should_retry_a_busy_relay_without_cooling_it_down() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![
            reply(503, b"busy".to_vec()),
            reply(200, f.bootstrap.clone()),
        ]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let key = fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await;
        assert_eq!(key.expect("proven after the retry"), Some(f.key));
        assert_eq!(relay.requests().len(), 2);
    }

    #[tokio::test]
    async fn should_cool_down_a_relay_that_times_out_upstream() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![reply(504, Vec::new())]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let reason = unavailable_reason(
            fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await,
        );
        assert!(reason.contains("HTTP 504"), "{reason}");
        let other =
            unavailable_reason(fetch(&provider, f.quorum_type, [0x11; 32], f.target.height).await);
        assert!(other.contains("cooling down"), "{other}");
        assert_eq!(relay.requests().len(), 1);
    }

    #[tokio::test]
    async fn should_keep_the_first_key_when_a_proof_opens_another() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![reply(200, f.bootstrap.clone())]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let quorum = (f.quorum_type, f.quorum_hash);
        // A key proven long ago, too long for the requested height.
        provider
            .inner
            .fetches
            .publish(quorum, [1; 48], &f.checkpoint)
            .expect("an earlier proof");
        let reason = unavailable_reason(
            fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await,
        );
        assert!(reason.contains("different key"), "{reason}");
        let cached =
            provider.get_quorum_public_key(f.quorum_type, f.quorum_hash, f.checkpoint.height);
        assert_eq!(cached.expect("still cached"), [1; 48]);
        assert!(provider
            .get_quorum_public_key(f.quorum_type, f.quorum_hash, f.target.height)
            .is_err());
    }

    #[tokio::test]
    async fn should_refuse_oversized_answers() {
        let f = fixture(Network::Testnet);
        let declared = Answer::Reply {
            status: 200,
            headers: vec![("Content-Length", (MAX_PROOF_BYTES + 1).to_string())],
            body: Vec::new(),
        };
        let chunked = Answer::Chunked {
            size: 64 * 1024,
            chunks: MAX_PROOF_BYTES / (64 * 1024) + 2,
        };
        for answer in [declared, chunked] {
            let relay = MockRelay::spawn(vec![answer]);
            let provider = provider_for(&f, vec![relay.url.clone()], quick());
            let reason = unavailable_reason(
                fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await,
            );
            assert!(reason.contains("more than a proof can be"), "{reason}");
            assert_eq!(provider.cached_quorum_count(), 0);
        }
    }

    #[tokio::test]
    async fn should_reject_a_proof_with_a_flipped_byte() {
        let f = fixture(Network::Testnet);
        let mut corrupt = f.bootstrap.clone();
        let middle = corrupt.len() / 2;
        corrupt[middle] ^= 1;
        let relay = MockRelay::spawn(vec![reply(200, corrupt)]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let reason = unavailable_reason(
            fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await,
        );
        assert!(reason.contains("invalid proof"), "{reason}");
        assert_eq!(provider.cached_quorum_count(), 0);
        assert_eq!(provider.latest_verified_state(), f.checkpoint);
    }

    #[tokio::test]
    async fn should_cache_nothing_from_a_proof_of_another_quorum() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![reply(200, f.bootstrap.clone())]);
        let provider = provider_for(&f, vec![relay.url.clone()], quick());
        let reason =
            unavailable_reason(fetch(&provider, f.quorum_type, [0x11; 32], f.target.height).await);
        assert!(
            reason.contains("opens the requested quorum 0 times"),
            "{reason}"
        );
        assert_eq!(provider.cached_quorum_count(), 0);
        assert_eq!(provider.latest_verified_state(), f.checkpoint);
    }

    #[tokio::test]
    async fn should_not_follow_redirects() {
        let f = fixture(Network::Testnet);
        let target = MockRelay::spawn(vec![reply(200, f.bootstrap.clone())]);
        let redirect = MockRelay::spawn(vec![Answer::Reply {
            status: 302,
            headers: vec![("Location", format!("{}/proofs", target.url))],
            body: Vec::new(),
        }]);
        let provider = provider_for(&f, vec![redirect.url.clone()], quick());
        let reason = unavailable_reason(
            fetch(&provider, f.quorum_type, f.quorum_hash, f.target.height).await,
        );
        assert!(reason.contains("HTTP 302"), "{reason}");
        assert_eq!(redirect.requests().len(), 1);
        assert!(target.requests().is_empty(), "the redirect was followed");
        assert_eq!(provider.cached_quorum_count(), 0);
    }

    #[tokio::test]
    async fn should_not_start_proving_a_third_quorum_at_once() {
        let f = fixture(Network::Testnet);
        let relay = MockRelay::spawn(vec![
            Answer::Hang(Duration::from_secs(5)),
            Answer::Hang(Duration::from_secs(5)),
        ]);
        let timing = Timing {
            deadline: Duration::from_millis(500),
            request_timeout: Duration::from_millis(500),
            ..quick()
        };
        let provider = provider_for(&f, vec![relay.url.clone()], timing);
        let height = f.target.height;
        let results =
            join_all([1u8, 2, 3].map(|seed| fetch(&provider, f.quorum_type, [seed; 32], height)))
                .await;
        let reasons: Vec<_> = results.into_iter().map(unavailable_reason).collect();
        assert!(
            reasons[2].contains("other quorums are being proven"),
            "{reasons:?}"
        );
    }

    #[test]
    fn should_reject_quorums_it_never_proves() {
        let f = fixture(Network::Testnet);
        let provider = provider_for(&f, vec!["https://relay.example".to_string()], quick());
        for (quorum_type, height) in [(4, 1), (1, 1), (f.quorum_type, i32::MAX as u32 + 1)] {
            assert!(provider
                .fetch_quorum_public_key(quorum_type, [1; 32], height)
                .is_none());
            assert!(matches!(
                provider.get_quorum_public_key(quorum_type, [1; 32], height),
                Err(ContextProviderError::InvalidQuorum(_))
            ));
        }
    }

    fn with_sources(
        sources: &[&str],
    ) -> Result<VerifiedHttpContextProvider, VerifiedContextProviderError> {
        let checkpoint = VerifiedHttpContextProvider::embedded_checkpoint(Network::Testnet)
            .expect("embedded checkpoint");
        VerifiedHttpContextProvider::with_checkpoint(
            Network::Testnet,
            checkpoint,
            sources.iter().map(|source| source.to_string()).collect(),
            cache_size(),
        )
    }

    #[test]
    fn should_accept_only_https_sources_or_plain_http_on_loopback() {
        for good in [
            "https://quorums.testnet.networks.dash.org",
            "https://relay.example/base/",
            "http://127.0.0.1:8080",
            "http://127.1.2.3",
            "http://[::1]:9000",
            "http://localhost:1234",
            // Hosts are not resolved when the provider is built.
            "https://does-not-resolve.invalid",
        ] {
            assert!(with_sources(&[good]).is_ok(), "{good}");
        }
        for bad in [
            "http://relay.example",
            "http://10.0.0.1",
            "ftp://relay.example",
            "https://user:secret@relay.example",
            "https://relay.example/?key=1",
            "https://relay.example/#top",
            "relay.example",
            "data:text/plain,proof",
        ] {
            assert!(
                matches!(
                    with_sources(&[bad]),
                    Err(VerifiedContextProviderError::InvalidSource(_))
                ),
                "{bad}"
            );
        }
        assert!(with_sources(&[]).is_err());
        let many: Vec<_> = (0..9)
            .map(|n| format!("https://relay{n}.example"))
            .collect();
        let many: Vec<_> = many.iter().map(String::as_str).collect();
        assert!(with_sources(&many).is_err());
        assert!(with_sources(&many[..8]).is_ok());
        assert!(
            with_sources(&["https://relay.example", "https://relay.example/"]).is_err(),
            "the same relay twice"
        );
    }

    #[test]
    fn should_post_to_proofs_under_each_source() {
        for (source, endpoint) in [
            ("https://relay.example", "https://relay.example/proofs"),
            ("https://relay.example/", "https://relay.example/proofs"),
            (
                "https://relay.example/a/b",
                "https://relay.example/a/b/proofs",
            ),
            ("https://relay.example/a/", "https://relay.example/a/proofs"),
        ] {
            let url = parse_source(source).expect("a source");
            assert_eq!(
                proofs_endpoint(&url).expect("an endpoint").as_str(),
                endpoint
            );
        }
    }

    #[test]
    fn should_build_only_for_mainnet_and_testnet_with_a_matching_checkpoint() {
        for network in [Network::Devnet, Network::Regtest] {
            assert!(matches!(
                VerifiedHttpContextProvider::new(network, cache_size()),
                Err(VerifiedContextProviderError::UnsupportedNetwork(_))
            ));
        }
        let mainnet = VerifiedHttpContextProvider::embedded_checkpoint(Network::Mainnet)
            .expect("mainnet checkpoint");
        assert_eq!(mainnet.height, 2_550_000);
        let mismatched = VerifiedHttpContextProvider::with_checkpoint(
            Network::Testnet,
            mainnet.clone(),
            vec!["https://relay.example".to_string()],
            cache_size(),
        );
        assert!(matches!(
            mismatched,
            Err(VerifiedContextProviderError::InvalidCheckpoint(_))
        ));
        let mut empty = mainnet;
        empty.quorum_root = [0; 32];
        assert!(matches!(
            VerifiedHttpContextProvider::with_checkpoint(
                Network::Mainnet,
                empty,
                vec!["https://relay.example".to_string()],
                cache_size(),
            ),
            Err(VerifiedContextProviderError::InvalidCheckpoint(_))
        ));

        let testnet = VerifiedHttpContextProvider::new(Network::Testnet, cache_size())
            .expect("a testnet provider without network access");
        assert_eq!(testnet.checkpoint().height, 1_560_000);
        assert_eq!(testnet.latest_verified_state(), *testnet.checkpoint());
        assert_eq!(
            testnet.sources()[0].as_str(),
            "https://quorums.testnet.networks.dash.org/"
        );
    }

    #[test]
    fn should_serve_known_contracts_and_token_configurations_only() {
        let f = fixture(Network::Testnet);
        let provider = provider_for(&f, vec!["https://relay.example".to_string()], quick());
        let id = Identifier::from([7; 32]);
        assert!(provider
            .get_data_contract(&id, PlatformVersion::latest())
            .expect("a lookup")
            .is_none());
        assert!(provider
            .get_token_configuration(&id)
            .expect("a lookup")
            .is_none());
        assert_eq!(
            provider.get_platform_activation_height().expect("a height"),
            1_090_319
        );
    }
}
