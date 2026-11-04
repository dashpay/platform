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

use dash_sdk::dpp::dashcore::Network;
use dash_sdk::platform::ContextProvider;
use dash_sdk::Error as SdkError;
use js_sys::JSON;
use rs_sdk_trusted_context_provider::{
    StartingPoint, VerifiedContextProviderError, VerifiedHttpContextProvider,
    VerifiedProviderOptions,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

use crate::error::WasmSdkError;

#[wasm_bindgen(typescript_custom_section)]
const QUORUM_PROOF_STATE_TS: &'static str = r#"
/**
 * A ChainLocked Core block and the masternode and quorum Merkle roots its
 * coinbase commits to: where a `WasmVerifiedContext` can start proving quorum
 * keys from. The form is Dash Core's: the `target` that
 * `getquorumproofchain` returns is a valid checkpoint. Unknown fields are
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

/**
 * Where a `WasmVerifiedContext` got to, for the next session to start from:
 * what `WasmVerifiedContext.continuation()` returns and the
 * `*FromContinuation` factories take. `anchor` is the newest verified
 * state more than one DKG interval (24 blocks) below the lowest Core height
 * of a response the SDK verified with this context's keys, a height a
 * Platform quorum signed (the context's starting point if none was
 * verified, or if no kept state is that far below it, as with a recent
 * custom checkpoint), so the next session can prove the heights Platform reports
 * from it, including a key verified from evidence up to that interval below
 * its height, for a quorum that has just retired; `states` are the newer
 * verified states, lowest first, so its proofs stay short. Every state is
 * trust configuration, as a checkpoint is: only pass back a continuation a
 * context produced. Unknown fields are rejected.
 */
export interface QuorumProofContinuation {
  anchor: QuorumProofState;
  states: QuorumProofState[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "QuorumProofState")]
    pub type QuorumProofStateJs;

    #[wasm_bindgen(typescript_type = "QuorumProofContinuation")]
    pub type QuorumProofContinuationJs;
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
/// Where proofs start is trust configuration: the checkpoint embedded in the
/// SDK, a ChainLocked block the app checked independently, or the
/// `continuation()` of an earlier context. A proof grows with the age of the
/// state it starts from, so long-lived apps should save `continuation()` and
/// pass it to `mainnetFromContinuation` / `testnetFromContinuation` on their
/// next start. Do not restart from `latestVerifiedState()` alone: it is often
/// above the Core height Platform reports, and no key at or below it could be
/// proven from it.
///
/// Attach it with `WasmSdkBuilder.withVerifiedContext(context)`. It discovers
/// no masternode addresses, so the builder keeps the ones it has.
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
        Self::build(Network::Mainnet, None, relays)
    }

    /// A testnet context starting from the checkpoint embedded in the SDK.
    #[wasm_bindgen(js_name = "testnet")]
    pub fn testnet(relays: Option<Vec<String>>) -> Result<WasmVerifiedContext, WasmSdkError> {
        Self::build(Network::Testnet, None, relays)
    }

    /// A mainnet context starting from `checkpoint`, a `QuorumProofState`
    /// the app checked independently.
    #[wasm_bindgen(js_name = "mainnetWithCheckpoint")]
    pub fn mainnet_with_checkpoint(
        checkpoint: QuorumProofStateJs,
        relays: Option<Vec<String>>,
    ) -> Result<WasmVerifiedContext, WasmSdkError> {
        Self::with_checkpoint(Network::Mainnet, &checkpoint, relays)
    }

    /// A testnet context starting from `checkpoint`, a `QuorumProofState`
    /// the app checked independently.
    #[wasm_bindgen(js_name = "testnetWithCheckpoint")]
    pub fn testnet_with_checkpoint(
        checkpoint: QuorumProofStateJs,
        relays: Option<Vec<String>>,
    ) -> Result<WasmVerifiedContext, WasmSdkError> {
        Self::with_checkpoint(Network::Testnet, &checkpoint, relays)
    }

    /// A mainnet context resuming from `continuation`, what an earlier
    /// context's `continuation()` returned.
    #[wasm_bindgen(js_name = "mainnetFromContinuation")]
    pub fn mainnet_from_continuation(
        continuation: QuorumProofContinuationJs,
        relays: Option<Vec<String>>,
    ) -> Result<WasmVerifiedContext, WasmSdkError> {
        Self::from_continuation(Network::Mainnet, &continuation, relays)
    }

    /// A testnet context resuming from `continuation`, what an earlier
    /// context's `continuation()` returned.
    #[wasm_bindgen(js_name = "testnetFromContinuation")]
    pub fn testnet_from_continuation(
        continuation: QuorumProofContinuationJs,
        relays: Option<Vec<String>>,
    ) -> Result<WasmVerifiedContext, WasmSdkError> {
        Self::from_continuation(Network::Testnet, &continuation, relays)
    }

    /// The checkpoint this context started from: the anchor, when it
    /// resumed from a continuation.
    #[wasm_bindgen(js_name = "checkpoint")]
    pub fn checkpoint_js(&self) -> Result<QuorumProofStateJs, WasmSdkError> {
        to_js(self.inner.checkpoint())
    }

    /// The highest state a proof has verified, or the checkpoint before any
    /// has. For information; save `continuation()` to resume from instead.
    #[wasm_bindgen(js_name = "latestVerifiedState")]
    pub fn latest_verified_state_js(&self) -> Result<QuorumProofStateJs, WasmSdkError> {
        to_js(&self.inner.latest_verified_state())
    }

    /// Where this context got to, a `QuorumProofContinuation`: save it once
    /// queries have run and pass it to `mainnetFromContinuation` /
    /// `testnetFromContinuation` on the next start.
    #[wasm_bindgen(js_name = "continuation")]
    pub fn continuation_js(&self) -> Result<QuorumProofContinuationJs, WasmSdkError> {
        to_js(&self.inner.continuation())
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
    /// returned without a request. The height is not a verified response's,
    /// so it does not move `continuation()`'s anchor.
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

    fn with_checkpoint(
        network: Network,
        checkpoint: &JsValue,
        relays: Option<Vec<String>>,
    ) -> Result<Self, WasmSdkError> {
        let checkpoint = from_js(checkpoint, "checkpoint", "a QuorumProofState")?;
        Self::build(network, Some(StartingPoint::Checkpoint(checkpoint)), relays)
    }

    fn from_continuation(
        network: Network,
        continuation: &JsValue,
        relays: Option<Vec<String>>,
    ) -> Result<Self, WasmSdkError> {
        let continuation = from_js(continuation, "continuation", "a QuorumProofContinuation")?;
        Self::build(
            network,
            Some(StartingPoint::Continuation(continuation)),
            relays,
        )
    }

    /// The provider supplies every default left `None`: the embedded
    /// checkpoint, the network's public quorum service and the key cache.
    fn build(
        network: Network,
        start: Option<StartingPoint>,
        relays: Option<Vec<String>>,
    ) -> Result<Self, WasmSdkError> {
        VerifiedHttpContextProvider::with_options(
            network,
            VerifiedProviderOptions {
                start,
                sources: relays,
                cache_size: None,
            },
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

/// Every construction failure is the caller's input (network, starting
/// point, relays) except an HTTP client that cannot be built.
fn construction_error(error: VerifiedContextProviderError) -> WasmSdkError {
    let message = format!("Failed to create verified context: {error}");
    match error {
        VerifiedContextProviderError::Http(_) => WasmSdkError::generic(message),
        _ => WasmSdkError::invalid_argument(message),
    }
}

/// A value from its JSON text, in the form its serde defines: Core's for a
/// state, and `{ anchor, states }` of those for a continuation.
fn from_json<T: DeserializeOwned>(json: &str, what: &str) -> Result<T, WasmSdkError> {
    serde_json::from_str(json)
        .map_err(|error| WasmSdkError::invalid_argument(format!("Invalid {what}: {error}")))
}

/// A value as JSON text, in the form `from_json` reads.
fn to_json<T: Serialize>(value: &T) -> Result<String, WasmSdkError> {
    serde_json::to_string(value).map_err(|error| {
        WasmSdkError::serialization(format!("Failed to serialize quorum proof data: {error}"))
    })
}

/// Crosses through `JSON`, so the object takes exactly the form serde uses.
fn from_js<T: DeserializeOwned>(
    value: &JsValue,
    what: &str,
    expected: &str,
) -> Result<T, WasmSdkError> {
    if !value.is_object() {
        return Err(WasmSdkError::invalid_argument(format!(
            "{what} must be {expected} object"
        )));
    }
    let json = JSON::stringify(value)
        .ok()
        .and_then(|json| json.as_string())
        .ok_or_else(|| {
            WasmSdkError::invalid_argument(format!("{what} cannot be converted to JSON"))
        })?;
    from_json(&json, what)
}

fn to_js<T: Serialize, J: JsCast>(value: &T) -> Result<J, WasmSdkError> {
    JSON::parse(&to_json(value)?)
        .map(JsCast::unchecked_into)
        .map_err(|_| WasmSdkError::serialization("Failed to convert quorum proof data"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::WasmSdkErrorKind;
    use crate::sdk::WasmSdkBuilder;
    use dash_core_proof::State;
    use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use rs_sdk_trusted_context_provider::{get_quorum_base_url, Continuation};
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
        from_json(&fixture[key].to_string(), key).expect(key)
    }

    /// A context starting from `checkpoint`, as `*WithCheckpoint` builds one.
    pub(super) fn with_checkpoint(
        network: Network,
        checkpoint: &State,
        relays: Option<Vec<String>>,
    ) -> Result<WasmVerifiedContext, WasmSdkError> {
        WasmVerifiedContext::build(
            network,
            Some(StartingPoint::Checkpoint(checkpoint.clone())),
            relays,
        )
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
            assert_eq!(ContextProvider::network(context.provider()), Some(network));
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
        let json = to_json(&target).expect("serializes");
        assert_eq!(from_json::<State>(&json, "state").expect("parses"), target);
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
            let error = from_json::<State>(json, "checkpoint").expect_err(json);
            assert_eq!(error.kind(), WasmSdkErrorKind::InvalidArgument, "{json}");
        }
        let mut extra: serde_json::Value = serde_json::from_str(PROOF_TESTNET).unwrap();
        extra["target"]["extra"] = 1.into();
        assert!(from_json::<State>(&extra["target"].to_string(), "checkpoint").is_err());
    }

    #[test]
    fn should_reject_a_checkpoint_of_the_other_network() {
        let testnet = fixture_state("checkpoint");
        let error = with_checkpoint(Network::Mainnet, &testnet, None)
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
            let error = with_checkpoint(Network::Testnet, &checkpoint, Some(relays))
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
        let context = with_checkpoint(
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
        let default_relay = with_checkpoint(Network::Testnet, &checkpoint, None)
            .expect("builds with the public relay");
        assert_eq!(
            default_relay.relays(),
            vec!["https://quorums.testnet.networks.dash.org/"]
        );
    }

    #[tokio::test]
    async fn should_report_a_quorum_type_it_never_proves_without_a_request() {
        // Nothing listens on the relay's port, so a request would fail as
        // retriable rather than as the provider's refusal.
        let port = TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .expect("a free port")
            .port();
        let context = with_checkpoint(
            Network::Testnet,
            &fixture_state("checkpoint"),
            Some(vec![format!("http://127.0.0.1:{port}")]),
        )
        .expect("builds");
        // Type 1 is not testnet's Platform quorum type, so nothing is fetched.
        let error = context
            .prove_quorum_public_key(1, [7; 32], fixture_state("target").height)
            .await
            .expect_err("never proven");
        assert_eq!(error.kind(), WasmSdkErrorKind::ContextProviderError);
        assert!(!error.is_retriable(), "{}", error.message());
        assert!(
            error
                .message()
                .contains("is not a Platform quorum this provider verifies"),
            "{}",
            error.message()
        );
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
        let context = with_checkpoint(
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
        let context = with_checkpoint(
            Network::Testnet,
            &fixture_state("checkpoint"),
            Some(vec![relay]),
        )
        .expect("builds");
        let sdk = WasmSdkBuilder::new_testnet()
            .with_verified_context(&context)
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

    /// The saved continuation crosses through its JSON form and resumes where
    /// the last session got to: the next context proves the same key at the
    /// same height through the relay.
    #[tokio::test]
    async fn should_prove_a_key_at_its_height_again_from_a_saved_continuation() {
        let (quorum_type, quorum_hash, key) = FIXTURE_QUORUM;
        let quorum_hash: [u8; 32] = hex::decode(quorum_hash).unwrap().try_into().unwrap();
        let target = fixture_state("target");
        let (relay, server) = spawn_relay(fixture_proof());
        let first = with_checkpoint(
            Network::Testnet,
            &fixture_state("checkpoint"),
            Some(vec![relay]),
        )
        .expect("builds");
        first
            .prove_quorum_public_key(u32::from(quorum_type), quorum_hash, target.height)
            .await
            .expect("the proof verifies");
        server.join().expect("the relay answered");

        // What an app saves: the continuation's JSON form.
        let saved = to_json(&first.provider().continuation()).expect("serializes");
        let value: serde_json::Value = serde_json::from_str(&saved).expect("JSON");
        assert_eq!(
            value["anchor"]["height"],
            fixture_state("checkpoint").height
        );
        assert_eq!(value["states"][0]["height"], target.height);
        let continuation: Continuation = from_json(&saved, "continuation").expect("parses");

        let (relay, server) = spawn_relay(fixture_proof());
        let next = WasmVerifiedContext::build(
            Network::Testnet,
            Some(StartingPoint::Continuation(continuation)),
            Some(vec![relay]),
        )
        .expect("resumes");
        let proven = next
            .prove_quorum_public_key(u32::from(quorum_type), quorum_hash, target.height)
            .await
            .expect("proven again");

        assert_eq!(hex::encode(proven), key);
        assert_eq!(server.join().expect("asked")["height"], target.height);
        assert_eq!(next.provider().latest_verified_state(), target);
    }

    #[test]
    fn should_reject_a_continuation_it_cannot_resume_from() {
        let checkpoint = fixture_state("checkpoint");
        // The provider checks the states; the binding reports its refusal as
        // the caller's input.
        let error = WasmVerifiedContext::build(
            Network::Testnet,
            Some(StartingPoint::Continuation(Continuation {
                anchor: fixture_state("target"),
                states: vec![checkpoint],
            })),
            None,
        )
        .err()
        .expect("a state below the anchor is refused");
        assert_eq!(error.kind(), WasmSdkErrorKind::InvalidArgument);
        for json in [
            r#"{"anchor":{}}"#,
            r#"{"states":[]}"#,
            r#"{"anchor":null,"states":[],"extra":1}"#,
        ] {
            let error = from_json::<Continuation>(json, "continuation").expect_err(json);
            assert_eq!(error.kind(), WasmSdkErrorKind::InvalidArgument, "{json}");
        }
    }

    /// A contract the SDK registers replaces the one the known-contracts cache
    /// held, as with the trusted context.
    #[test]
    fn should_forward_a_registered_contract_to_the_known_contracts_cache() {
        let context = WasmVerifiedContext::testnet(None).expect("builds");
        let sdk = WasmSdkBuilder::new_testnet()
            .with_verified_context(&context)
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
    use super::tests::{fixture_proof, fixture_state, with_checkpoint, FIXTURE_QUORUM};
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
        let context = with_checkpoint(
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
