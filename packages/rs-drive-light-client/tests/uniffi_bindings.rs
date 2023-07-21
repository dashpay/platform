// use bincode::serde::{decode_borrowed_from_slice, decode_from_slice, encode_to_vec};
use bytes::Bytes;
use drive_light_client::uniffi_bindings::codec::{Codec, DEFAULT_CODEC};

include!("utils.rs");

#[test]
fn test_get_identity_proof() {
    use dapi_grpc::platform::v0::{GetIdentityRequest, GetIdentityResponse};

    use drive_light_client::Error;
    let (request, response, quorum_info_callback) =
        load::<GetIdentityRequest, GetIdentityResponse>("vectors/identity_not_found.json");

    let encoder = &DEFAULT_CODEC;

    let req = encoder.encode(&request).unwrap();
    assert_eq!(
        request,
        encoder.decode(&mut Bytes::from(req.clone())).unwrap()
    );

    let resp = encoder.encode(&response).unwrap();
    assert_eq!(
        response,
        encoder.decode(&mut Bytes::from(resp.clone())).unwrap()
    );

    let ret = drive_light_client::uniffi_bindings::proof::identity_proof_json(
        req,
        resp,
        Box::new(quorum_info_callback),
    );
    assert!(matches!(ret, Result::Err(Error::DocumentMissingInProof)));
}
