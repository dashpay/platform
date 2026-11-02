//! `WasmVerifiedContext`: the browser binding of `VerifiedHttpContextProvider`.
//!
//! The provider proves every Platform quorum key it serves with a DIP 175
//! proof from a checkpoint, and treats the quorum service as an untrusted
//! relay. This module only marshals: checkpoints cross as plain objects in
//! Core's `getquorumproofchain` `target` form, errors become `WasmSdkError`s,
//! and every decision (which relays are acceptable, whether a checkpoint is
//! valid, when to fetch, how a proof is judged) stays in the Rust provider.
//! The builder attaches the provider itself, so contract registration, the
//! known-contract and token-configuration caches and quorum-key fetches are
//! the provider's own.

use std::num::NonZeroUsize;

use dash_core_proof::{ProofNetwork, State};
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::platform::ContextProvider;
use dash_sdk::Error as SdkError;
use js_sys::JSON;
use rs_sdk_trusted_context_provider::{
    get_quorum_base_url, VerifiedContextProviderError, VerifiedHttpContextProvider,
};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

use crate::error::WasmSdkError;

/// How many proven quorum keys a context keeps: above the 24 active Platform
/// quorums the provider requires, so a quorum that just rotated out is not
/// proven again at once.
const QUORUM_KEY_CACHE_SIZE: NonZeroUsize = NonZeroUsize::new(32).unwrap();

#[wasm_bindgen(typescript_custom_section)]
const QUORUM_PROOF_STATE_TS: &'static str = r#"
/**
 * A ChainLocked Core block and the masternode and quorum Merkle roots its
 * coinbase commits to: where a `WasmVerifiedContext` starts proving quorum
 * keys from. The form is Dash Core's: the `target` that
 * `getquorumproofchain` returns is a valid checkpoint, and so is the value
 * `WasmVerifiedContext.latestVerifiedState()` returns. Unknown fields are
 * rejected.
 */
export interface QuorumProofState {
  /** 0 for mainnet, 1 for testnet. */
  network: number;
  height: number;
  /** Block hash, hex in RPC display order. */
  block_hash: string;
  /** The coinbase's `merkleRootMNList`, hex in RPC display order. */
  masternode_root: string;
  /** The coinbase's `merkleRootQuorums`, hex in RPC display order. */
  quorum_root: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "QuorumProofState")]
    pub type QuorumProofStateJs;
}

/// A context whose Platform quorum keys are proven, not trusted.
///
/// Quorum keys are proven lazily with DIP 175 proofs from a checkpoint, the
/// first time the SDK meets each quorum, so nothing is fetched up front and
/// the factories are synchronous. Mainnet and testnet only.
///
/// Every factory takes optional `relays`: 1 to 8 distinct relay base URLs
/// whose `/proofs` endpoint serves the proofs, each `https` (plain `http`
/// only on a loopback host). Without them the context asks the network's
/// public quorum service. Relays are untrusted byte sources: a lying relay
/// can delay requests but cannot make the SDK accept a forged key.
///
/// The checkpoint is trust configuration: the one embedded in the SDK, a
/// state an earlier context verified (`latestVerifiedState()`), or a
/// ChainLocked block the app checked independently. A proof grows with the
/// age of the state it starts from, so long-lived apps should save
/// `latestVerifiedState()` and pass it back on their next start.
///
/// Attach it with `WasmSdkBuilder.withVerifiedContext(context)`. It discovers
/// no masternode addresses, so the builder keeps its preset or explicit ones.
///
/// The public mainnet relay does not serve proofs yet, so a mainnet context
/// without `relays` builds, but proofs fail until it does.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmVerifiedContext {
    inner: VerifiedHttpContextProvider,
}

#[wasm_bindgen]
impl WasmVerifiedContext {
    /// A mainnet context starting from the checkpoint embedded in the SDK.
    #[wasm_bindgen(js_name = "mainnet")]
    pub fn mainnet(relays: Option<Vec<String>>) -> Result<WasmVerifiedContext, WasmSdkError> {
        Self::embedded(Network::Mainnet, relays)
    }

    /// A testnet context starting from the checkpoint embedded in the SDK.
    #[wasm_bindgen(js_name = "testnet")]
    pub fn testnet(relays: Option<Vec<String>>) -> Result<WasmVerifiedContext, WasmSdkError> {
        Self::embedded(Network::Testnet, relays)
    }

    /// A mainnet context starting from `checkpoint`, a `QuorumProofState`.
    #[wasm_bindgen(js_name = "mainnetWithCheckpoint")]
    pub fn mainnet_with_checkpoint(
        checkpoint: QuorumProofStateJs,
        relays: Option<Vec<String>>,
    ) -> Result<WasmVerifiedContext, WasmSdkError> {
        Self::with_checkpoint(Network::Mainnet, &state_from_js(&checkpoint)?, relays)
    }

    /// A testnet context starting from `checkpoint`, a `QuorumProofState`.
    #[wasm_bindgen(js_name = "testnetWithCheckpoint")]
    pub fn testnet_with_checkpoint(
        checkpoint: QuorumProofStateJs,
        relays: Option<Vec<String>>,
    ) -> Result<WasmVerifiedContext, WasmSdkError> {
        Self::with_checkpoint(Network::Testnet, &state_from_js(&checkpoint)?, relays)
    }

    /// The checkpoint this context started from.
    #[wasm_bindgen(js_name = "checkpoint")]
    pub fn checkpoint_js(&self) -> Result<QuorumProofStateJs, WasmSdkError> {
        state_to_js(self.inner.checkpoint())
    }

    /// The highest state a proof has verified, or the checkpoint before any
    /// has. Save it and pass it back as the checkpoint on the next start, so
    /// later proofs are shorter.
    #[wasm_bindgen(js_name = "latestVerifiedState")]
    pub fn latest_verified_state_js(&self) -> Result<QuorumProofStateJs, WasmSdkError> {
        state_to_js(&self.inner.latest_verified_state())
    }

    /// The relay base URLs this context asks.
    #[wasm_bindgen(js_name = "relays")]
    pub fn relays(&self) -> Vec<String> {
        self.inner
            .sources()
            .iter()
            .map(|url| url.to_string())
            .collect()
    }

    /// Prove a Platform quorum's public key for a Core ChainLocked height, as
    /// the SDK does before it verifies a response that quorum signed, and
    /// return the 48-byte BLS key. A key already proven for the height is
    /// returned without a request.
    ///
    /// `quorumHash` is hex in RPC display order, as Core's `quorum list`
    /// shows it.
    #[wasm_bindgen(js_name = "fetchQuorumPublicKey")]
    pub async fn fetch_quorum_public_key_js(
        &self,
        #[wasm_bindgen(js_name = "quorumType")] quorum_type: u32,
        #[wasm_bindgen(js_name = "quorumHash")] quorum_hash: String,
        #[wasm_bindgen(js_name = "coreChainLockedHeight")] core_chain_locked_height: u32,
    ) -> Result<Vec<u8>, WasmSdkError> {
        let quorum_hash: [u8; 32] = hex::decode(&quorum_hash)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| {
                WasmSdkError::invalid_argument(format!(
                    "quorumHash must be 32 bytes of hex, got '{quorum_hash}'"
                ))
            })?;
        self.prove_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
            .await
            .map(Vec::from)
    }
}

impl WasmVerifiedContext {
    /// The provider the builder attaches and the SDK caches into.
    pub(crate) fn provider(&self) -> &VerifiedHttpContextProvider {
        &self.inner
    }

    /// The network whose quorums this context proves.
    pub(crate) fn network(&self) -> Network {
        match self.inner.checkpoint().network {
            ProofNetwork::Mainnet => Network::Mainnet,
            ProofNetwork::Testnet => Network::Testnet,
        }
    }

    fn embedded(network: Network, relays: Option<Vec<String>>) -> Result<Self, WasmSdkError> {
        let checkpoint = VerifiedHttpContextProvider::embedded_checkpoint(network)
            .map_err(construction_error)?;
        Self::with_checkpoint(network, &checkpoint, relays)
    }

    /// `None` relays means the network's public quorum service.
    fn with_checkpoint(
        network: Network,
        checkpoint: &State,
        relays: Option<Vec<String>>,
    ) -> Result<Self, WasmSdkError> {
        let relays = match relays {
            Some(relays) => relays,
            None => vec![get_quorum_base_url(network, None)
                .map_err(|error| WasmSdkError::invalid_argument(error.to_string()))?],
        };
        VerifiedHttpContextProvider::with_checkpoint(
            network,
            checkpoint.clone(),
            relays,
            QUORUM_KEY_CACHE_SIZE,
        )
        .map(|inner| Self { inner })
        .map_err(construction_error)
    }

    /// The fetch the SDK runs for a quorum it has no key for. When the
    /// provider declines to fetch (a quorum type or height it never proves),
    /// the synchronous lookup says why. Failures keep the SDK's error kind,
    /// so an unavailable relay stays retriable.
    async fn prove_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], WasmSdkError> {
        let proven = match self.inner.fetch_quorum_public_key(
            quorum_type,
            quorum_hash,
            core_chain_locked_height,
        ) {
            Some(fetch) => fetch.await,
            None => self
                .inner
                .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
                .map(Some),
        };
        match proven {
            Ok(Some(key)) => Ok(key),
            Ok(None) => Err(WasmSdkError::not_found(format!(
                "quorum {} of type {quorum_type} was not found",
                hex::encode(quorum_hash)
            ))),
            Err(error) => Err(WasmSdkError::with_context(
                "Failed to prove the quorum public key",
                SdkError::from(error),
            )),
        }
    }
}

/// Every construction failure is the caller's input (network, checkpoint,
/// relays) except an HTTP client that cannot be built.
fn construction_error(error: VerifiedContextProviderError) -> WasmSdkError {
    let message = format!("Failed to create verified context: {error}");
    match error {
        VerifiedContextProviderError::Http(_) => WasmSdkError::generic(message),
        _ => WasmSdkError::invalid_argument(message),
    }
}

/// A checkpoint from its JSON text, as Core writes it.
fn state_from_json(json: &str) -> Result<State, WasmSdkError> {
    serde_json::from_str(json).map_err(|error| {
        WasmSdkError::invalid_argument(format!("Invalid quorum proof state: {error}"))
    })
}

/// A state as JSON text, in the form `state_from_json` reads.
fn state_to_json(state: &State) -> Result<String, WasmSdkError> {
    serde_json::to_string(state).map_err(|error| {
        WasmSdkError::serialization(format!("Failed to serialize quorum proof state: {error}"))
    })
}

/// Crosses through `JSON`, so the object takes exactly the form Core and
/// `State`'s serde use.
fn state_from_js(value: &JsValue) -> Result<State, WasmSdkError> {
    if !value.is_object() {
        return Err(WasmSdkError::invalid_argument(
            "checkpoint must be a QuorumProofState object",
        ));
    }
    let json = JSON::stringify(value)
        .ok()
        .and_then(|json| json.as_string())
        .ok_or_else(|| WasmSdkError::invalid_argument("checkpoint cannot be converted to JSON"))?;
    state_from_json(&json)
}

fn state_to_js(state: &State) -> Result<QuorumProofStateJs, WasmSdkError> {
    JSON::parse(&state_to_json(state)?)
        .map(JsCast::unchecked_into)
        .map_err(|_| WasmSdkError::serialization("Failed to convert the quorum proof state"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::WasmSdkErrorKind;
    use crate::sdk::WasmSdkBuilder;
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};

    const PROOF_TESTNET: &str =
        include_str!("../../rs-sdk-trusted-context-provider/tests/data/proof-testnet.json");

    /// The quorum the testnet fixture proof opens, its hash in RPC display order, and its
    /// key; `tests/unit/verified-context.spec.ts` uses the same values.
    pub(super) const FIXTURE_QUORUM: (u8, &str, &str) = (
        6,
        "0000001b381f694e98ab6efcf1f380aba750bd6b4f70f48f9e8818cff2a64af6",
        "b813d560b9a73db7b01cfe80ee7ab20a4af7ccda4b94a0cd8ceda8f5cb0fa604760430ef8c644e0f823e089aa7839f93",
    );

    /// A state of the testnet fixture: `checkpoint` or `target`.
    pub(super) fn fixture_state(key: &str) -> State {
        let fixture: serde_json::Value = serde_json::from_str(PROOF_TESTNET).expect("fixture");
        state_from_json(&fixture[key].to_string()).expect(key)
    }

    /// The testnet fixture's proof, from `fixture_state("checkpoint")` to
    /// `fixture_state("target")`, as `getquorumproofchain` returned it.
    pub(super) fn fixture_proof() -> Vec<u8> {
        let fixture: serde_json::Value = serde_json::from_str(PROOF_TESTNET).expect("fixture");
        hex::decode(fixture["bootstrap_hex"].as_str().expect("bootstrap_hex")).expect("hex")
    }

    /// The fixture's quorum, read from the proof itself.
    #[test]
    fn should_name_the_quorum_the_fixture_proof_opens() {
        let proof = fixture_proof();
        let verified =
            dash_core_proof::bootstrap::verify(&proof, &fixture_state("checkpoint"), 0).unwrap();
        let commitment = dash_core_proof::parse_commitment(verified.records()[0].leaf).unwrap();
        let mut quorum_hash = commitment.quorum_hash;
        quorum_hash.reverse();
        assert_eq!(
            (
                commitment.quorum_type,
                hex::encode(quorum_hash).as_str(),
                hex::encode(commitment.public_key).as_str()
            ),
            FIXTURE_QUORUM
        );
    }

    #[test]
    fn should_start_from_the_embedded_checkpoint_before_any_proof() {
        for (network, context) in [
            (Network::Mainnet, WasmVerifiedContext::mainnet(None)),
            (Network::Testnet, WasmVerifiedContext::testnet(None)),
        ] {
            let context = context.expect("the embedded checkpoint builds");
            let embedded =
                VerifiedHttpContextProvider::embedded_checkpoint(network).expect("embedded");
            assert_eq!(context.network(), network);
            assert_eq!(context.provider().checkpoint(), &embedded);
            assert_eq!(context.provider().latest_verified_state(), embedded);
            assert_eq!(
                context.relays(),
                vec![format!("{}/", get_quorum_base_url(network, None).unwrap())]
            );
        }
    }

    #[test]
    fn should_round_trip_a_state_through_its_json_form() {
        let target = fixture_state("target");
        let json = state_to_json(&target).expect("serializes");
        assert_eq!(state_from_json(&json).expect("parses"), target);
        let value: serde_json::Value = serde_json::from_str(&json).expect("JSON");
        assert_eq!(value["network"], 1);
        assert_eq!(value["height"], 1_576_768);
        assert_eq!(
            value["block_hash"],
            "00000021a51acdce0ba4c5f1bff3fd121143e7ab958f9f30bd1de4856b449285"
        );
    }

    #[test]
    fn should_reject_a_malformed_checkpoint_as_an_invalid_argument() {
        for json in [
            r#"{"network":1}"#,
            r#"{"network":1,"height":1576768,"block_hash":"zz","masternode_root":"00","quorum_root":"00"}"#,
        ] {
            let error = state_from_json(json).expect_err(json);
            assert_eq!(error.kind(), WasmSdkErrorKind::InvalidArgument, "{json}");
        }
        let mut extra: serde_json::Value = serde_json::from_str(PROOF_TESTNET).unwrap();
        extra["target"]["extra"] = 1.into();
        assert!(state_from_json(&extra["target"].to_string()).is_err());
    }

    #[test]
    fn should_reject_a_checkpoint_of_the_other_network() {
        let testnet = fixture_state("checkpoint");
        let error = WasmVerifiedContext::with_checkpoint(Network::Mainnet, &testnet, None)
            .err()
            .expect("a testnet checkpoint cannot start a mainnet context");
        assert_eq!(error.kind(), WasmSdkErrorKind::InvalidArgument);
        assert!(
            error.message().contains("checkpoint"),
            "{}",
            error.message()
        );
    }

    #[test]
    fn should_reject_relays_the_provider_may_not_ask() {
        let checkpoint = fixture_state("checkpoint");
        for relays in [
            vec![],
            vec!["http://relay.example".to_string()],
            vec!["not a url".to_string()],
            vec![
                "https://relay.example".to_string(),
                "https://relay.example/".to_string(),
            ],
            (0..9)
                .map(|i| format!("https://relay{i}.example"))
                .collect(),
        ] {
            let error =
                WasmVerifiedContext::with_checkpoint(Network::Testnet, &checkpoint, Some(relays))
                    .err()
                    .expect("refused");
            assert_eq!(error.kind(), WasmSdkErrorKind::InvalidArgument);
            assert!(
                error.message().contains("proof source"),
                "{}",
                error.message()
            );
        }
    }

    #[test]
    fn should_use_a_custom_checkpoint_and_relays() {
        let checkpoint = fixture_state("checkpoint");
        let context = WasmVerifiedContext::with_checkpoint(
            Network::Testnet,
            &checkpoint,
            Some(vec![
                "https://relay.example".to_string(),
                "http://127.0.0.1:8080/base".to_string(),
            ]),
        )
        .expect("builds");
        assert_eq!(context.provider().latest_verified_state(), checkpoint);
        assert_eq!(
            context.relays(),
            vec!["https://relay.example/", "http://127.0.0.1:8080/base"]
        );
        let default_relay =
            WasmVerifiedContext::with_checkpoint(Network::Testnet, &checkpoint, None)
                .expect("builds with the public relay");
        assert_eq!(
            default_relay.relays(),
            vec!["https://quorums.testnet.networks.dash.org/"]
        );
    }

    #[tokio::test]
    async fn should_report_a_quorum_type_it_never_proves_without_a_request() {
        let context = WasmVerifiedContext::testnet(None).expect("builds");
        // Type 1 is not testnet's Platform quorum type, so nothing is fetched.
        let error = context
            .prove_quorum_public_key(1, [7; 32], 1_600_000)
            .await
            .expect_err("never proven");
        assert_eq!(error.kind(), WasmSdkErrorKind::ContextProviderError);
        assert!(error.message().contains("prove"), "{}", error.message());
    }

    /// A relay that cannot be reached is the SDK's retriable "source
    /// unavailable", not a final failure.
    #[tokio::test]
    async fn should_report_an_unreachable_relay_as_retriable() {
        // Bound and dropped, so nothing listens on the port.
        let port = TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .expect("a free port")
            .port();
        let context = WasmVerifiedContext::with_checkpoint(
            Network::Testnet,
            &fixture_state("checkpoint"),
            Some(vec![format!("http://127.0.0.1:{port}")]),
        )
        .expect("builds");
        let (quorum_type, quorum_hash, _) = FIXTURE_QUORUM;
        let quorum_hash: [u8; 32] = hex::decode(quorum_hash).unwrap().try_into().unwrap();

        let error = context
            .prove_quorum_public_key(
                u32::from(quorum_type),
                quorum_hash,
                fixture_state("target").height,
            )
            .await
            .expect_err("nothing answers");

        assert_eq!(error.kind(), WasmSdkErrorKind::ContextProviderError);
        assert!(error.is_retriable(), "{}", error.message());
    }

    /// Answer one `POST /proofs` on a loopback port with `body`, and hand back
    /// the request's JSON.
    fn spawn_relay(body: Vec<u8>) -> (String, JoinHandle<serde_json::Value>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the mock relay");
        let url = format!("http://{}", listener.local_addr().expect("address"));
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the provider connects");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut line = String::new();
            reader.read_line(&mut line).expect("request line");
            assert!(line.starts_with("POST /proofs "), "{line}");
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).expect("header");
                if header == "\r\n" {
                    break;
                }
                if let Some((name, value)) = header.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().expect("length");
                    }
                }
            }
            let mut request = vec![0; length];
            reader.read_exact(&mut request).expect("request body");
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write the head");
            stream.write_all(&body).expect("write the proof");
            serde_json::from_slice(&request).expect("a JSON request")
        });
        (url, handle)
    }

    /// Through the SDK it is attached to, the verified context asks a relay
    /// for the proof of a quorum it has no key for and serves the key the
    /// proof opens. The relay is a loopback mock serving the testnet fixture.
    #[tokio::test]
    async fn should_prove_a_key_from_a_relay_through_the_sdk() {
        let (relay, server) = spawn_relay(fixture_proof());
        let target = fixture_state("target");
        let context = WasmVerifiedContext::with_checkpoint(
            Network::Testnet,
            &fixture_state("checkpoint"),
            Some(vec![relay]),
        )
        .expect("builds");
        let sdk = WasmSdkBuilder::new_testnet()
            .with_verified_context(&context)
            .expect("attach")
            .build()
            .expect("build");
        let provider = sdk
            .inner_sdk()
            .context_provider()
            .expect("the verified context is the SDK's provider");
        let (quorum_type, quorum_hash, key) = FIXTURE_QUORUM;
        let quorum_hash: [u8; 32] = hex::decode(quorum_hash).unwrap().try_into().unwrap();

        let proven = provider
            .fetch_quorum_public_key(u32::from(quorum_type), quorum_hash, target.height)
            .expect("a Platform quorum is fetched")
            .await
            .expect("the proof verifies");

        assert_eq!(proven.map(hex::encode).as_deref(), Some(key));
        assert_eq!(context.provider().latest_verified_state(), target);
        let request = server.join().expect("the relay answered");
        assert_eq!(request["height"], target.height);
        assert_eq!(request["llmqType"], u32::from(quorum_type));
    }

    /// A contract the SDK registers replaces the one the known-contracts cache
    /// held, as with the trusted context.
    #[test]
    fn should_forward_a_registered_contract_to_the_known_contracts_cache() {
        let context = WasmVerifiedContext::testnet(None).expect("builds");
        let sdk = WasmSdkBuilder::new_testnet()
            .with_verified_context(&context)
            .expect("attach")
            .build()
            .expect("build");
        let provider = sdk
            .inner_sdk()
            .context_provider()
            .expect("the verified context is the SDK's provider");
        let dpns_v2 = load_system_data_contract(
            SystemDataContract::DPNS,
            PlatformVersion::get(13).expect("protocol version 13"),
        )
        .expect("DPNS v2 loads");
        let dpns_v3 =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("DPNS v3 loads");
        context.provider().add_known_contract(dpns_v2);

        provider.register_data_contract(Arc::new(dpns_v3.clone()));

        let resolved = provider
            .get_data_contract(&dpns_v3.id(), PlatformVersion::latest())
            .expect("the context answers")
            .expect("the context holds DPNS");
        assert_eq!(*resolved, dpns_v3);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::tests::{fixture_proof, fixture_state, FIXTURE_QUORUM};
    use super::*;
    use js_sys::{Function, Reflect, Uint8Array};
    use wasm_bindgen_test::wasm_bindgen_test;

    /// The browser's `fetch` carries the proof request to a relay, and the
    /// context returns the key the proof opens. The relay is the global
    /// `fetch`, replaced to serve the testnet fixture proof, so no network is
    /// touched.
    #[wasm_bindgen_test]
    async fn should_prove_a_key_through_the_browser_fetch_transport() {
        let global = js_sys::global();
        Reflect::set(
            &global,
            &JsValue::from_str("__proof"),
            &Uint8Array::from(fixture_proof().as_slice()),
        )
        .expect("hand the proof to the stub");
        let fetch = Function::new_with_args(
            "request",
            "globalThis.__relayed = request.url; \
             return Promise.resolve(new Response(globalThis.__proof, { status: 200 }));",
        );
        let original = Reflect::get(&global, &JsValue::from_str("fetch")).expect("the fetch");
        Reflect::set(&global, &JsValue::from_str("fetch"), &fetch).expect("replace the fetch");

        let target = fixture_state("target");
        let context = WasmVerifiedContext::with_checkpoint(
            Network::Testnet,
            &fixture_state("checkpoint"),
            Some(vec!["https://relay.test".to_string()]),
        )
        .expect("builds");
        let (quorum_type, quorum_hash, key) = FIXTURE_QUORUM;
        let proven = context
            .fetch_quorum_public_key_js(
                u32::from(quorum_type),
                quorum_hash.to_string(),
                target.height,
            )
            .await;
        // Later tests in the same wasm module use the real fetch.
        Reflect::set(&global, &JsValue::from_str("fetch"), &original).expect("restore the fetch");

        assert_eq!(hex::encode(proven.expect("the proof verifies")), key);
        assert_eq!(context.provider().latest_verified_state(), target);
        let relayed = Reflect::get(&global, &JsValue::from_str("__relayed")).expect("asked");
        assert_eq!(
            relayed.as_string().as_deref(),
            Some("https://relay.test/proofs")
        );
    }
}
