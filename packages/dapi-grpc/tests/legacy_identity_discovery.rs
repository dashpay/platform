use dapi_grpc::{platform::v0::GetIdentityByNonUniquePublicKeyHashResponse, Message};

#[test]
fn should_decode_legacy_non_unique_identity_proof() {
    // V0, result field 2, nested index Proof with an empty GroveDB byte field.
    let wire = [0x0a, 0x06, 0x12, 0x04, 0x0a, 0x02, 0x0a, 0x00];
    let response = GetIdentityByNonUniquePublicKeyHashResponse::decode(&wire[..]).unwrap();
    let Some(
        dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::Version::V0(
            v0,
        ),
    ) = response.version
    else {
        panic!("missing response version")
    };
    assert!(
        v0.result.is_some(),
        "legacy proof field 2 must survive decoding"
    );
}
