// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! State-transition builder tests against the rs-dpp-generated fixtures in
//! `test_data/dpp_st_vectors.json` (Platform v4.0.0, protocol
//! version 12). The vectors carry full serialized transitions built from
//! deterministic inputs (fixed keys, entropy, RFC6979 ECDSA), so the
//! builders are checked byte-for-byte.
//!
//! The signing callback stands in for the C++ WalletSigner: it receives the
//! key id plus the double-SHA256 digest of the signable bytes and answers
//! with `dashcore::signer::sign_hash`, which is exactly what
//! `signer::sign(signable_bytes, key)` would produce.

use std::sync::Mutex;

use dash_platform_cxx::st::{self, AssetLockProofInput, NewIdentityKey, ASSET_LOCK_KEY_ID};
use dash_platform_cxx::types::KeyInfo;
use dpp::dashcore::hashes::{sha256, Hash};
use dpp::dashcore::signer as dash_signer;
use serde_json::Value;

fn hexv(hex: &str) -> Vec<u8> {
    hex::decode(hex).expect("bad hex in test vector")
}

fn hex32(hex: &str) -> [u8; 32] {
    hexv(hex).try_into().expect("expected 32 bytes")
}

fn vectors() -> Value {
    let doc: Value = serde_json::from_str(include_str!("../test_data/dpp_st_vectors.json"))
        .expect("parse dpp_st_vectors.json");
    assert_eq!(doc["platform_repo_tag"].as_str(), Some("v4.0.0"));
    doc
}

/// Test signer: routes key ids to the fixed vector keys and records every
/// digest it was asked to sign.
struct TestSigner {
    master_sk: [u8; 32],
    high_sk: [u8; 32],
    asset_lock_sk: [u8; 32],
    digests: Mutex<Vec<(u32, [u8; 32])>>,
}

impl TestSigner {
    fn from_vectors(doc: &Value) -> Self {
        TestSigner {
            master_sk: hex32(doc["keys"]["master"]["private_key_hex"].as_str().unwrap()),
            high_sk: hex32(doc["keys"]["high"]["private_key_hex"].as_str().unwrap()),
            asset_lock_sk: hex32(
                doc["keys"]["asset_lock"]["private_key_hex"]
                    .as_str()
                    .unwrap(),
            ),
            digests: Mutex::new(Vec::new()),
        }
    }

    fn sign(&self, key_id: u32, digest: [u8; 32]) -> Option<Vec<u8>> {
        self.digests.lock().unwrap().push((key_id, digest));
        let sk = match key_id {
            0 => self.master_sk,
            1 => self.high_sk,
            ASSET_LOCK_KEY_ID => self.asset_lock_sk,
            _ => return None,
        };
        dash_signer::sign_hash(&digest, &sk)
            .ok()
            .map(|s| s.to_vec())
    }
}

fn high_key(doc: &Value) -> KeyInfo {
    KeyInfo {
        id: 1,
        purpose: 0,        // AUTHENTICATION
        security_level: 2, // HIGH
        key_type: 0,       // ECDSA_SECP256K1
        read_only: false,
        data: hexv(doc["keys"]["high"]["public_key_hex"].as_str().unwrap()),
        disabled_at: None,
    }
}

fn check_built(
    built: &dash_platform_cxx::types::BuiltTransition,
    vector: &Value,
    signer: &TestSigner,
    expected_key_id: u32,
) {
    assert_eq!(
        hex::encode(&built.bytes),
        vector["serialized_hex"].as_str().unwrap(),
        "serialized transition bytes"
    );
    assert_eq!(
        built.hash,
        sha256::Hash::hash(&built.bytes).to_byte_array(),
        "hash must be single sha256 of the bytes"
    );
    let expected_digest = hex32(vector["digest_hex"].as_str().unwrap());
    let digests = signer.digests.lock().unwrap();
    assert!(
        digests.contains(&(expected_key_id, expected_digest)),
        "signer must be asked for the vector digest with key id {expected_key_id}; got {digests:?}"
    );
}

#[test]
fn dpns_preorder() {
    let doc = vectors();
    let vector = &doc["dpns_preorder"];
    let signer = TestSigner::from_vectors(&doc);
    let built = st::build_dpns_preorder(
        &hexv(vector["owner_id_hex"].as_str().unwrap()),
        vector["identity_contract_nonce"].as_u64().unwrap(),
        vector["label"].as_str().unwrap(),
        &hexv(vector["salt_hex"].as_str().unwrap()),
        &high_key(&doc),
        &|key_id, digest| signer.sign(key_id, digest),
    )
    .expect("build DPNS preorder");
    check_built(&built, vector, &signer, 1);
}

fn build_domain(
    doc: &Value,
    vector: &Value,
    signer: &TestSigner,
) -> dash_platform_cxx::types::BuiltTransition {
    st::build_dpns_domain(
        &hexv(vector["owner_id_hex"].as_str().unwrap()),
        vector["identity_contract_nonce"].as_u64().unwrap(),
        vector["label"].as_str().unwrap(),
        vector["normalized_label"].as_str().unwrap(),
        "dash",
        &hexv(vector["salt_hex"].as_str().unwrap()),
        &high_key(doc),
        &|key_id, digest| signer.sign(key_id, digest),
    )
    .expect("build DPNS domain")
}

#[test]
fn dpns_domain() {
    let doc = vectors();
    let vector = &doc["dpns_domain"];
    assert_eq!(vector["contested"].as_bool(), Some(false));
    let signer = TestSigner::from_vectors(&doc);
    let built = build_domain(&doc, vector, &signer);
    check_built(&built, vector, &signer, 1);
}

// A contested label must automatically attach the prefunded voting balance
// (rs-dpp computes it from the contested unique index of the domain type).
#[test]
fn dpns_domain_contested() {
    let doc = vectors();
    let vector = &doc["dpns_domain_contested"];
    assert_eq!(vector["contested"].as_bool(), Some(true));
    let signer = TestSigner::from_vectors(&doc);
    let built = build_domain(&doc, vector, &signer);
    check_built(&built, vector, &signer, 1);
}

#[test]
fn dpns_domain_rejects_bad_normalization() {
    let doc = vectors();
    let signer = TestSigner::from_vectors(&doc);
    let err = st::build_dpns_domain(
        &[0x11; 32],
        1,
        "Alice",
        "alice", // wrong: o->0, l/i->1 normalization gives "a11ce"
        "dash",
        &[0x55; 32],
        &high_key(&doc),
        &|key_id, digest| signer.sign(key_id, digest),
    )
    .unwrap_err();
    assert!(err.contains("normalized label"), "{err}");
}

#[test]
fn dashpay_profile_create() {
    let doc = vectors();
    let vector = &doc["dashpay_profile_create"];
    let signer = TestSigner::from_vectors(&doc);
    let built = st::build_profile(
        &hexv(vector["owner_id_hex"].as_str().unwrap()),
        vector["identity_contract_nonce"].as_u64().unwrap(),
        vector["display_name"].as_str().unwrap(),
        vector["public_message"].as_str().unwrap(),
        vector["avatar_url"].as_str().unwrap(),
        &hexv(vector["avatar_hash_hex"].as_str().unwrap()),
        &hexv(vector["avatar_fingerprint_hex"].as_str().unwrap()),
        1,
        None,
        &hexv(vector["entropy_hex"].as_str().unwrap()),
        &high_key(&doc),
        &|key_id, digest| signer.sign(key_id, digest),
    )
    .expect("build profile create");
    check_built(&built, vector, &signer, 1);
}

#[test]
fn dashpay_profile_replace() {
    let doc = vectors();
    let create = &doc["dashpay_profile_create"];
    let vector = &doc["dashpay_profile_replace"];
    let signer = TestSigner::from_vectors(&doc);
    // The replace fixture carries the create fixture's profile fields with
    // an updated publicMessage (see the serialized property map).
    let built = st::build_profile(
        &hexv(vector["owner_id_hex"].as_str().unwrap()),
        vector["identity_contract_nonce"].as_u64().unwrap(),
        create["display_name"].as_str().unwrap(),
        vector["public_message"].as_str().unwrap(),
        create["avatar_url"].as_str().unwrap(),
        &hexv(create["avatar_hash_hex"].as_str().unwrap()),
        &hexv(create["avatar_fingerprint_hex"].as_str().unwrap()),
        vector["revision"].as_u64().unwrap(),
        Some(&hexv(vector["document_id_hex"].as_str().unwrap())),
        &[0u8; 32], // entropy is unused for replacements
        &high_key(&doc),
        &|key_id, digest| signer.sign(key_id, digest),
    )
    .expect("build profile replace");
    check_built(&built, vector, &signer, 1);
}

#[test]
fn dashpay_contact_request() {
    let doc = vectors();
    let vector = &doc["dashpay_contact_request"];
    let signer = TestSigner::from_vectors(&doc);
    let built = st::build_contact_request(
        &hexv(vector["owner_id_hex"].as_str().unwrap()),
        vector["identity_contract_nonce"].as_u64().unwrap(),
        &hexv(vector["to_user_id_hex"].as_str().unwrap()),
        &hexv(vector["encrypted_public_key_hex"].as_str().unwrap()),
        vector["sender_key_index"].as_u64().unwrap() as u32,
        vector["recipient_key_index"].as_u64().unwrap() as u32,
        vector["account_reference"].as_u64().unwrap() as u32,
        &hexv(vector["encrypted_account_label_hex"].as_str().unwrap()),
        &hexv(vector["entropy_hex"].as_str().unwrap()),
        &high_key(&doc),
        &|key_id, digest| signer.sign(key_id, digest),
    )
    .expect("build contact request");
    check_built(&built, vector, &signer, 1);
}

fn identity_keys(doc: &Value) -> Vec<NewIdentityKey> {
    vec![
        NewIdentityKey {
            id: 0,
            purpose: 0,        // AUTHENTICATION
            security_level: 0, // MASTER
            pubkey: hexv(doc["keys"]["master"]["public_key_hex"].as_str().unwrap()),
        },
        NewIdentityKey {
            id: 1,
            purpose: 0,
            security_level: 2, // HIGH
            pubkey: hexv(doc["keys"]["high"]["public_key_hex"].as_str().unwrap()),
        },
    ]
}

fn check_identity_create(
    built: &dash_platform_cxx::types::BuiltTransition,
    vector: &Value,
    signer: &TestSigner,
) {
    assert_eq!(
        hex::encode(&built.bytes),
        vector["serialized_hex"].as_str().unwrap(),
        "serialized identity create bytes"
    );
    let expected_digest = hex32(vector["digest_hex"].as_str().unwrap());
    let digests = signer.digests.lock().unwrap().clone();
    // Every identity key and the asset-lock key sign the same digest.
    for key_id in [0, 1, ASSET_LOCK_KEY_ID] {
        assert!(
            digests.contains(&(key_id, expected_digest)),
            "key {key_id} must sign the vector digest"
        );
    }

    // Round trip: the built bytes must deserialize back into an
    // IdentityCreate transition with signatures present.
    use dpp::serialization::PlatformDeserializable;
    use dpp::state_transition::StateTransition;
    let deserialized =
        StateTransition::deserialize_from_bytes(&built.bytes).expect("round-trip deserialize");
    match &deserialized {
        StateTransition::IdentityCreate(_) => {}
        other => panic!("expected IdentityCreate, got {other:?}"),
    }
    let reserialized = {
        use dpp::serialization::PlatformSerializable;
        deserialized.serialize_to_bytes().expect("re-serialize")
    };
    assert_eq!(reserialized, built.bytes, "round trip must be stable");
    assert!(
        deserialized
            .signature()
            .map(|signature| !signature.is_empty())
            .unwrap_or(false),
        "outer signature must be present"
    );
}

#[test]
fn identity_create_instant() {
    let doc = vectors();
    let vector = &doc["identity_create_instant"];
    let signer = TestSigner::from_vectors(&doc);
    let built = st::build_identity_create(
        AssetLockProofInput::Instant {
            transaction: hexv(vector["transaction_hex"].as_str().unwrap()),
            instant_lock: hexv(vector["instant_lock_hex"].as_str().unwrap()),
            output_index: vector["output_index"].as_u64().unwrap() as u32,
        },
        &identity_keys(&doc),
        &|key_id, digest| signer.sign(key_id, digest),
    )
    .expect("build identity create (instant)");
    check_identity_create(&built, vector, &signer);
}

#[test]
fn identity_create_chain() {
    let doc = vectors();
    let vector = &doc["identity_create_chain"];
    let signer = TestSigner::from_vectors(&doc);
    let built = st::build_identity_create(
        AssetLockProofInput::Chain {
            core_chain_locked_height: vector["core_chain_locked_height"].as_u64().unwrap() as u32,
            out_point: hexv(vector["out_point_hex"].as_str().unwrap())
                .try_into()
                .expect("36-byte outpoint"),
        },
        &identity_keys(&doc),
        &|key_id, digest| signer.sign(key_id, digest),
    )
    .expect("build identity create (chain)");
    check_identity_create(&built, vector, &signer);
}

// Keys must arrive at the wire sorted by id with no duplicates, regardless
// of input order.
#[test]
fn identity_create_sorts_and_rejects_duplicate_keys() {
    let doc = vectors();
    let vector = &doc["identity_create_chain"];
    let signer = TestSigner::from_vectors(&doc);
    let proof = || AssetLockProofInput::Chain {
        core_chain_locked_height: vector["core_chain_locked_height"].as_u64().unwrap() as u32,
        out_point: hexv(vector["out_point_hex"].as_str().unwrap())
            .try_into()
            .unwrap(),
    };

    let mut reversed = identity_keys(&doc);
    reversed.reverse();
    let built = st::build_identity_create(proof(), &reversed, &|key_id, digest| {
        signer.sign(key_id, digest)
    })
    .expect("reversed key order still builds");
    assert_eq!(
        hex::encode(&built.bytes),
        vector["serialized_hex"].as_str().unwrap()
    );

    let mut duplicated = identity_keys(&doc);
    duplicated[1].id = 0;
    let err = st::build_identity_create(proof(), &duplicated, &|key_id, digest| {
        signer.sign(key_id, digest)
    })
    .unwrap_err();
    assert!(err.contains("duplicate"), "{err}");
}

#[test]
fn signer_failure_is_reported() {
    let doc = vectors();
    let vector = &doc["dpns_preorder"];
    let err = st::build_dpns_preorder(
        &hexv(vector["owner_id_hex"].as_str().unwrap()),
        2,
        vector["label"].as_str().unwrap(),
        &hexv(vector["salt_hex"].as_str().unwrap()),
        &high_key(&doc),
        &|_key_id, _digest| None, // wallet refuses
    )
    .unwrap_err();
    assert!(
        err.contains("signing failed") || err.contains("locked"),
        "{err}"
    );
}
