use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_response::{
    get_identity_by_non_unique_public_key_hash_response_v0::Result as V0Result,
    get_identity_by_non_unique_public_key_hash_response_v1::Result as V1Result, Version,
};
use dapi_grpc::{platform::v0::GetIdentityByNonUniquePublicKeyHashResponse, Message};

#[test]
fn should_decode_the_two_proof_v0_answer_4_1_nodes_send() {
    // V0, result field 2: an index Proof whose GroveDB bytes are [1, 2], and identity
    // proof bytes [3, 4].
    let wire = [
        0x0a, 0x0c, 0x12, 0x0a, 0x0a, 0x04, 0x0a, 0x02, 1, 2, 0x12, 0x02, 3, 4,
    ];
    let response = GetIdentityByNonUniquePublicKeyHashResponse::decode(&wire[..]).unwrap();
    let Some(Version::V0(v0)) = &response.version else {
        panic!("response version 0")
    };
    let Some(V0Result::Proof(proved)) = &v0.result else {
        panic!("field 2 is the two-proof answer")
    };
    assert_eq!(
        proved
            .grovedb_identity_public_key_hash_proof
            .as_ref()
            .map(|proof| proof.grovedb_proof.as_slice()),
        Some(&[1, 2][..])
    );
    assert_eq!(proved.identity_proof_bytes.as_deref(), Some(&[3, 4][..]));
    assert_eq!(response.encode_to_vec(), wire);
}

#[test]
fn should_decode_the_single_proof_v1_answer() {
    // V1, result field 2, a Proof whose GroveDB bytes are [1, 2].
    let wire = [0x12, 0x06, 0x12, 0x04, 0x0a, 0x02, 1, 2];
    let response = GetIdentityByNonUniquePublicKeyHashResponse::decode(&wire[..]).unwrap();
    let Some(Version::V1(v1)) = &response.version else {
        panic!("response version 1")
    };
    let Some(V1Result::Proof(proof)) = &v1.result else {
        panic!("field 2 is the proof")
    };
    assert_eq!(proof.grovedb_proof, [1, 2]);
    assert_eq!(response.encode_to_vec(), wire);
}

#[test]
fn should_find_no_result_in_a_v0_answer_at_field_4() {
    // V0 with a Proof at field 4, as the 5.0 prereleases sent it.
    let wire = [0x0a, 0x06, 0x22, 0x04, 0x0a, 0x02, 1, 2];
    let response = GetIdentityByNonUniquePublicKeyHashResponse::decode(&wire[..]).unwrap();
    let Some(Version::V0(v0)) = &response.version else {
        panic!("response version 0")
    };
    assert!(v0.result.is_none());
}
