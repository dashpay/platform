use dapi_grpc::platform::v0::{
    GetIdentitiesByPublicKeyHashesRequest, GetIdentitiesByPublicKeyHashesResponse,
    GetIdentityRequest, GetIdentityResponse,
};
use dpp::prelude::Identity;
use drive_light_client::proof::from_proof::{FromProof, MockQuorumInfoProvider};
use std::{fs::File, path::PathBuf};
use tracing::Level;

#[test]
fn get_identities_by_hashes_notfound() {
    let (req, resp, provider): (
        GetIdentitiesByPublicKeyHashesRequest,
        GetIdentitiesByPublicKeyHashesResponse,
        MockQuorumInfoProvider,
    ) = load("vectors/get_identities_by_hashes_notfound.json");

    let ids = drive_light_client::proof::from_proof::IdentitiesByPublicKeyHashes::maybe_from_proof(
        &req,
        &resp,
        Box::new(provider),
    )
    .unwrap();
    assert!(ids.is_none())
    // Vec<Identity>::from_proof(req, resp, provider)
}

/// Given some test vectors dumped from a devnet, prove non-existence of identity with some hardcoded identifier
#[test]
fn identity_not_found() {
    enable_logs();

    let (request, response, provider) =
        load::<GetIdentityRequest, GetIdentityResponse>("vectors/identity_not_found.json");

    let identity = Identity::maybe_from_proof(&request, &response, Box::new(provider)).unwrap();
    assert!(identity.is_none())
}

#[derive(serde::Deserialize, Debug)]
struct QuorumInfo {
    #[serde(deserialize_with = "dapi_grpc::deserialization::from_hex")]
    quorum_public_key: Vec<u8>,
}

fn load<Req, Resp>(file: &str) -> (Req, Resp, MockQuorumInfoProvider)
where
    Req: dapi_grpc::Message + serde::de::DeserializeOwned,
    Resp: dapi_grpc::Message + serde::de::DeserializeOwned,
{
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(file);

    let f = File::open(path).unwrap();
    let (req, resp, quorum): (Req, Resp, QuorumInfo) = serde_json::from_reader(f).unwrap();

    println!("req: {:?}\nresp: {:?}\nquorum: {:?}\n", req, resp, quorum);

    let pubkey = quorum.quorum_public_key;
    let mut provider = MockQuorumInfoProvider::new();
    provider
        .expect_get_quorum_public_key()
        .return_once(|_, _| Ok(pubkey));
    (req, resp, provider)
}

fn enable_logs() {
    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_ansi(true)
        .with_max_level(Level::TRACE)
        .try_init()
        .ok();
}
