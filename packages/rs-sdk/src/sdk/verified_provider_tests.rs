//! A proved query verified end to end with a cold
//! [`VerifiedHttpContextProvider`]: the SDK proves the signing quorum's key
//! through the relay, verifies the response, and the provider counts the
//! response's Core height for its continuation.

use super::Sdk;
use crate::SdkBuilder;
use dapi_grpc::platform::v0::{
    get_epochs_info_request, GetEpochsInfoRequest, GetEpochsInfoResponse,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dapi_grpc::Message;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::types::ExtendedEpochInfos;
use drive_proof_verifier::FromProof;
use rs_dapi_client::AddressList;
use rs_sdk_trusted_context_provider::{
    Continuation, StartingPoint, VerifiedHttpContextProvider, VerifiedProviderOptions,
};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// One DKG interval, in blocks: the margin a continuation's anchor keeps
/// below the lowest verified Core height.
const DKG_INTERVAL_BLOCKS: u32 = 24;

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../rs-sdk-trusted-context-provider/tests/data")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(&path).expect("read fixture"))
        .expect("fixture JSON")
}

/// The testnet checkpoint at 1,574,768 the provider crate's proofs start
/// from.
fn checkpoint() -> dash_core_proof::State {
    serde_json::from_value(fixture("proof-testnet.json")["checkpoint"].clone())
        .expect("a checkpoint")
}

/// A captured, proved testnet `getEpochsInfo` response (epoch 0), its Core
/// height, and the relay's proof of its signing quorum from [`checkpoint`],
/// whose target is that height.
fn platform_fixture() -> (u32, GetEpochsInfoResponse, Vec<u8>) {
    let value = fixture("platform-epochs-testnet.json");
    let bytes = |key: &str| hex::decode(value[key].as_str().expect("hex")).expect("hex");
    (
        value["core_chain_locked_height"]
            .as_u64()
            .expect("a height")
            .try_into()
            .expect("a Core height"),
        GetEpochsInfoResponse::decode(bytes("response_hex").as_slice()).expect("a response"),
        bytes("quorum_proof_hex"),
    )
}

/// The request the fixture answers: the first epoch, ascending, proved.
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

/// A relay on loopback answering every `/proofs` request with `proof` and
/// recording each request's JSON body.
fn relay(proof: Vec<u8>) -> (String, Arc<Mutex<Vec<Value>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let address = listener.local_addr().expect("an address");
    let asked = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&asked);
    thread::spawn(move || {
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
            let (status, answer) = if path == "/proofs" {
                recorded
                    .lock()
                    .expect("relay log")
                    .push(serde_json::from_slice(&body).unwrap_or(Value::Null));
                (200, proof.clone())
            } else {
                (404, Vec::new())
            };
            let mut stream = reader.into_inner();
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                answer.len()
            );
            let _ = stream.write_all(&answer);
        }
    });
    (format!("http://{address}"), asked)
}

/// A network (not mock) testnet SDK whose proofs are verified with
/// `provider`, trusting Platform's block height from `response`: it was
/// captured long ago, so its signed time is not checked.
fn sdk_with(provider: &VerifiedHttpContextProvider, response: &GetEpochsInfoResponse) -> Sdk {
    SdkBuilder::new(AddressList::new())
        .with_network(Network::Testnet)
        .with_context_provider(provider.clone())
        .with_time_tolerance(None)
        .with_height_tolerance(Some(1))
        .with_trusted_initial_height(response.metadata().expect("metadata").height)
        .build()
        .expect("a verified testnet SDK")
}

/// Verify the fixture response the way every proved fetch does once the
/// transport returns it.
async fn verify_through_sdk(sdk: &Sdk, response: GetEpochsInfoResponse) -> ExtendedEpochInfos {
    let (epochs, metadata, _) = sdk
        .parse_proof_with_metadata_and_proof::<GetEpochsInfoRequest, ExtendedEpochInfos>(
            epochs_request(),
            response.clone(),
            "get_epochs_info",
        )
        .await
        .expect("the response verifies once the quorum key is proven");
    assert_eq!(
        metadata.core_chain_locked_height,
        response
            .metadata()
            .expect("metadata")
            .core_chain_locked_height
    );
    epochs.expect("epoch 0 exists")
}

/// The key of the quorum that signed the fixture is not cached.
fn assert_key_unavailable(provider: &VerifiedHttpContextProvider, response: GetEpochsInfoResponse) {
    let version = PlatformVersion::get(response.metadata().expect("metadata").protocol_version)
        .unwrap_or(PlatformVersion::latest());
    let error = ExtendedEpochInfos::maybe_from_proof_with_metadata(
        epochs_request(),
        response,
        Network::Testnet,
        version,
        provider,
    )
    .expect_err("a cold provider has no key");
    assert!(
        matches!(
            error,
            drive_proof_verifier::Error::QuorumKeyUnavailable { .. }
        ),
        "got {error:?}"
    );
}

/// A cold verified provider behind a network SDK: the first verification of
/// a captured testnet response misses the quorum key, the SDK has the
/// provider prove it through the relay and verifies the same response
/// again, and the verified Core height reaches the provider. A session
/// restored from its continuation proves the key again and verifies the
/// same response at the same height.
#[tokio::test]
async fn should_verify_a_proved_query_with_a_cold_verified_provider_and_resume_from_its_continuation(
) {
    let (height, response, quorum_proof) = platform_fixture();
    let checkpoint = checkpoint();
    let cache_size = NonZeroUsize::new(32).expect("nonzero");

    // First session, from the checkpoint.
    let (url, asked) = relay(quorum_proof.clone());
    let provider = VerifiedHttpContextProvider::with_checkpoint(
        Network::Testnet,
        checkpoint.clone(),
        vec![url],
        cache_size,
    )
    .expect("a provider");
    assert_key_unavailable(&provider, response.clone());
    assert_eq!(provider.lowest_verified_core_height(), None);
    let sdk = sdk_with(&provider, &response);

    let epochs = verify_through_sdk(&sdk, response.clone()).await;

    assert_eq!(epochs.len(), 1, "epoch 0");
    {
        let asked = asked.lock().expect("relay log");
        assert_eq!(asked.len(), 1, "one proof, for the quorum: {asked:?}");
        assert_eq!(
            asked[0]["height"], height,
            "asked for the response's height"
        );
    }
    assert_eq!(provider.cached_quorum_count(), 1);
    // The verifier reported the response's Core height, after its signature
    // verified, to the provider the SDK holds.
    assert_eq!(provider.lowest_verified_core_height(), Some(height));
    let continuation = provider.continuation();
    assert!(
        continuation.anchor.height < height - DKG_INTERVAL_BLOCKS,
        "anchor {} is not more than one DKG interval below {height}",
        continuation.anchor.height
    );
    assert_eq!(continuation.anchor, checkpoint);
    assert_eq!(
        continuation
            .states
            .iter()
            .map(|state| state.height)
            .collect::<Vec<_>>(),
        vec![height],
        "the proven target is kept above the anchor"
    );

    // Next session, restored from the continuation, its cache cold again.
    let saved = serde_json::to_string(&continuation).expect("serialize the continuation");
    let restored: Continuation = serde_json::from_str(&saved).expect("restore it");
    let (url, asked) = relay(quorum_proof);
    let resumed = VerifiedHttpContextProvider::with_options(
        Network::Testnet,
        VerifiedProviderOptions {
            start: Some(StartingPoint::Continuation(restored)),
            sources: Some(vec![url]),
            cache_size: Some(cache_size),
        },
    )
    .expect("a resumed provider");
    assert_eq!(resumed.checkpoint(), &checkpoint);
    assert_key_unavailable(&resumed, response.clone());
    let sdk = sdk_with(&resumed, &response);

    let epochs = verify_through_sdk(&sdk, response).await;

    assert_eq!(epochs.len(), 1, "epoch 0");
    {
        let asked = asked.lock().expect("relay log");
        assert_eq!(asked.len(), 1, "proved again: {asked:?}");
        assert_eq!(asked[0]["height"], height);
        assert_eq!(
            asked[0]["checkpoint"],
            dash_core_proof::rpc_hex(&checkpoint.block_hash).as_str(),
            "from the anchor, the only state below the height"
        );
    }
    assert_eq!(resumed.lowest_verified_core_height(), Some(height));
    assert_eq!(resumed.continuation(), continuation);
}
