include!("utils.rs");

#[test]
fn test_get_identity_proof_to_cbor() {
    use dapi_grpc::{
        platform::v0::{GetIdentityRequest, GetIdentityResponse},
        Message,
    };

    use drive_light_client::Error;
    let (request, response, quorum_info_callback) =
        load::<GetIdentityRequest, GetIdentityResponse>("vectors/identity_not_found.json");

    let req_proto = request.encode_to_vec();

    let resp_proto = response.encode_to_vec();

    let ret = drive_light_client::uniffi_bindings::proof::identity_proof_to_cbor(
        req_proto,
        resp_proto,
        Box::new(quorum_info_callback),
    );

    assert!(matches!(ret, Result::Err(Error::DocumentMissingInProof)));
}
