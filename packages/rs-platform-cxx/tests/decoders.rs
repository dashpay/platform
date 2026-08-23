// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Decoder tests against the rs-dpp-generated vectors in
//! `test_data/dpp_identity_vectors.json` (identities) and the
//! stored-document fixtures in dpp_st_vectors.json (DPNS domain, DashPay
//! profile and contactRequest documents as Drive stores/returns them).

use dash_platform_cxx::decode;
use serde_json::Value;

fn hexv(hex: &str) -> Vec<u8> {
    hex::decode(hex).expect("bad hex in test vector")
}

fn identity_vectors() -> Value {
    serde_json::from_str(include_str!("../test_data/dpp_identity_vectors.json"))
        .expect("parse dpp_identity_vectors.json")
}

fn st_vectors() -> Value {
    serde_json::from_str(include_str!("../test_data/dpp_st_vectors.json"))
        .expect("parse dpp_st_vectors.json")
}

#[test]
fn decode_identity_vector() {
    let doc = identity_vectors();
    let vector = &doc["identity"];
    let identity = decode::decode_identity(&hexv(vector["serialized_hex"].as_str().unwrap()))
        .expect("decode identity");

    assert_eq!(hex::encode(identity.id), vector["id"].as_str().unwrap());
    assert_eq!(Some(identity.balance), vector["balance"].as_u64());
    assert_eq!(Some(identity.revision), vector["revision"].as_u64());

    let expected_keys = vector["public_keys"].as_array().unwrap();
    assert_eq!(identity.keys.len(), expected_keys.len());
    for (key, expected) in identity.keys.iter().zip(expected_keys) {
        assert_eq!(Some(u64::from(key.id)), expected["id"].as_u64());
        assert_eq!(Some(u64::from(key.purpose)), expected["purpose"].as_u64());
        assert_eq!(
            Some(u64::from(key.security_level)),
            expected["security_level"].as_u64()
        );
        assert_eq!(Some(u64::from(key.key_type)), expected["key_type"].as_u64());
        assert_eq!(Some(key.read_only), expected["read_only"].as_bool());
        assert_eq!(hex::encode(&key.data), expected["data"].as_str().unwrap());
        assert_eq!(key.disabled_at, expected["disabled_at"].as_u64());
    }
}

#[test]
fn decode_identity_rejects_garbage() {
    assert!(decode::decode_identity(&[0xff; 16]).is_err());
}

#[test]
fn decode_stored_dpns_domain() {
    let doc = st_vectors();
    let vector = &doc["stored_documents"]["domain"];
    let name = decode::decode_dpns_domain(&hexv(vector["serialized_hex"].as_str().unwrap()))
        .expect("decode DPNS domain");
    assert_eq!(
        hex::encode(name.document_id),
        vector["id_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(name.owner_id),
        vector["owner_id_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(name.identity),
        vector["identity_id_hex"].as_str().unwrap()
    );
    assert_eq!(name.label, vector["label"].as_str().unwrap());
    assert_eq!(
        name.normalized_label,
        vector["normalized_label"].as_str().unwrap()
    );
    assert_eq!(name.parent_domain, "dash");
}

#[test]
fn decode_stored_dashpay_profile() {
    let doc = st_vectors();
    let vector = &doc["stored_documents"]["profile"];
    let profile = decode::decode_dashpay_profile(&hexv(vector["serialized_hex"].as_str().unwrap()))
        .expect("decode DashPay profile");
    assert_eq!(
        hex::encode(profile.document_id),
        vector["id_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(profile.owner_id),
        vector["owner_id_hex"].as_str().unwrap()
    );
    assert_eq!(Some(profile.revision), vector["revision"].as_u64());
    assert_eq!(
        profile.display_name,
        vector["display_name"].as_str().unwrap()
    );
    assert_eq!(
        profile.public_message,
        vector["public_message"].as_str().unwrap()
    );
    assert_eq!(profile.avatar_url, vector["avatar_url"].as_str().unwrap());
    assert_eq!(Some(profile.created_at), vector["created_at"].as_u64());
    assert_eq!(Some(profile.updated_at), vector["updated_at"].as_u64());
    assert_eq!(profile.avatar_hash, vec![0x88; 32]);
    assert_eq!(profile.avatar_fingerprint, vec![0x99; 8]);
}

#[test]
fn decode_stored_contact_request() {
    let doc = st_vectors();
    let vector = &doc["stored_documents"]["contact"];
    let request = decode::decode_contact_request(&hexv(vector["serialized_hex"].as_str().unwrap()))
        .expect("decode contact request");
    assert_eq!(
        hex::encode(request.document_id),
        vector["id_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(request.owner_id),
        vector["owner_id_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(request.to_user_id),
        vector["to_user_id_hex"].as_str().unwrap()
    );
    assert_eq!(
        Some(u64::from(request.sender_key_index)),
        vector["sender_key_index"].as_u64()
    );
    assert_eq!(
        Some(u64::from(request.recipient_key_index)),
        vector["recipient_key_index"].as_u64()
    );
    assert_eq!(
        Some(u64::from(request.account_reference)),
        vector["account_reference"].as_u64()
    );
    assert_eq!(Some(request.created_at), vector["created_at"].as_u64());
    assert_eq!(
        Some(u64::from(request.core_height_created_at)),
        vector["core_height_created_at"].as_u64()
    );
    assert_eq!(request.encrypted_public_key, vec![0xab; 96]);
    assert_eq!(request.encrypted_account_label, vec![0xcd; 48]);
}

// Documents proven by the drive query vectors are placeholder items, not
// real documents; decoding them must fail cleanly rather than panic.
#[test]
fn decode_rejects_placeholder_documents() {
    assert!(decode::decode_dpns_domain(b"proved-dpns-document").is_err());
    assert!(decode::decode_dashpay_profile(b"proved-profile-document").is_err());
    assert!(decode::decode_contact_request(b"proved-contact-document").is_err());
}
