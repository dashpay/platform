use dapi_grpc::platform::v0::{
    GetIdentitiesByPublicKeyHashesRequest, GetIdentitiesByPublicKeyHashesResponse,
    GetIdentityRequest, GetIdentityResponse,
};
use dpp::prelude::Identity;
use drive_light_client::proof::from_proof::{FromProof, MockQuorumInfoProvider};

include!("utils.rs");

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
