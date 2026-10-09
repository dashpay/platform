//! Emit deterministic compatibility fixtures from DPP's current BLS backend.
//! Run with `cargo run -p dpp --example generate_bls_compatibility_vectors`.

use dpp::bls::{PublicKey, SecretKey, Signature};
use dpp::core_types::validator::v0::ValidatorV0;
use dpp::core_types::validator_set::v0::ValidatorSetV0;
use dpp::dashcore::{hashes::Hash, ProTxHash, PubkeyHash, QuorumHash};
use serde_json::json;
use std::collections::BTreeMap;

fn main() {
    let key_generation: Vec<_> = [[0u8; 32], [1; 32], [255; 32]].iter().map(|ikm| {
        let key = SecretKey::from_ikm(ikm).unwrap();
        json!({"ikm": hex::encode(ikm), "secret_key": hex::encode(key.to_be_bytes()), "public_key": hex::encode(key.public_key().to_bytes())})
    }).collect();
    // Public test material: unit scalars, asymmetric bytes and the last canonical scalar.
    let keys: Vec<[u8; 32]> = [
        "0000000000000000000000000000000000000000000000000000000000000001",
        "0000000000000000000000000000000000000000000000000000000000000002",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        "73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000000",
    ]
    .iter()
    .map(|s| hex::decode(s).unwrap().try_into().unwrap())
    .collect();
    let mut basic = Vec::new();
    for bytes in &keys {
        let key = SecretKey::from_be_bytes(bytes).unwrap();
        for message in [
            vec![],
            b"Dash Platform BLS compatibility".to_vec(),
            vec![0; 32],
            (0..=255).collect(),
        ] {
            let signature = key.sign(&message).unwrap();
            basic.push(json!({
                "secret_key": hex::encode(bytes), "message": hex::encode(message),
                "public_key": hex::encode(key.public_key().to_bytes()),
                "legacy_public_key": hex::encode(key.public_key().to_legacy_bytes().unwrap()),
                "signature": hex::encode(signature.to_bytes())
            }));
        }
    }
    let mut secure_aggregation = Vec::new();
    for indexes in [
        vec![0],
        vec![0, 1],
        vec![0, 1, 2],
        vec![2, 0, 1],
        vec![0, 0],
    ] {
        let message = b"Dash Platform secure aggregation";
        let signers: Vec<_> = indexes
            .iter()
            .map(|&i| SecretKey::from_be_bytes(&keys[i]).unwrap())
            .collect();
        let public_keys: Vec<_> = signers.iter().map(|key| key.public_key()).collect();
        let signatures: Vec<_> = signers
            .iter()
            .map(|key| key.sign(message).unwrap())
            .collect();
        let point = Signature::aggregate_secure(&signatures, &public_keys).unwrap();
        secure_aggregation.push(json!({
            "secret_keys": indexes.iter().map(|&i| hex::encode(keys[i])).collect::<Vec<_>>(),
            "message": hex::encode(message),
            "public_keys": public_keys.iter().map(|key| hex::encode(key.to_bytes())).collect::<Vec<_>>(),
            "signature": hex::encode(point.to_bytes())
        }));
    }
    let mut scalars = Vec::new();
    for (name, bytes) in [
        ("zero", [0; 32]),
        (
            "order",
            hex::decode("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001")
                .unwrap()
                .try_into()
                .unwrap(),
        ),
        (
            "order_plus_one",
            hex::decode("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000002")
                .unwrap()
                .try_into()
                .unwrap(),
        ),
        ("max", [255; 32]),
    ] {
        let key = SecretKey::from_be_bytes(&bytes);
        scalars.push(json!({
            "name": name, "input": hex::encode(bytes),
            "normalized": key.as_ref().map(|key| hex::encode(key.to_be_bytes())),
            "public_key": key.as_ref().map(|key| hex::encode(key.public_key().to_bytes())),
            "signature": key.as_ref().map(|key| hex::encode(key.sign(b"scalar boundary").unwrap().to_bytes()))
        }));
    }
    let mut infinity = [0; 48];
    infinity[0] = 0xc0;
    let public_key = SecretKey::from_be_bytes(&keys[0]).unwrap().public_key();
    let mut storage = Vec::new();
    for (name, key) in [
        ("public_key", Some(public_key)),
        ("absent", None),
        (
            "infinity",
            Some(PublicKey::try_from(infinity.as_slice()).unwrap()),
        ),
    ] {
        let validator = ValidatorV0 {
            pro_tx_hash: ProTxHash::from_byte_array([0x11; 32]),
            public_key: key,
            node_ip: "127.0.0.1".into(),
            node_id: PubkeyHash::from_byte_array([0x22; 20]),
            core_port: 19999,
            platform_http_port: 1443,
            platform_p2p_port: 26656,
            is_banned: false,
        };
        let config = bincode::config::standard().with_big_endian();
        let set = ValidatorSetV0 {
            quorum_hash: QuorumHash::from_byte_array([0x33; 32]),
            quorum_index: Some(2),
            core_height: 123456,
            members: BTreeMap::from([(validator.pro_tx_hash, validator.clone())]),
            threshold_public_key: public_key,
        };
        storage.push(json!({
            "name": name, "validator": serde_json::to_value(&validator).unwrap(),
            "validator_bytes": hex::encode(bincode::encode_to_vec(&validator, config).unwrap()),
            "validator_set": serde_json::to_value(&set).unwrap(),
            "validator_set_bytes": hex::encode(bincode::encode_to_vec(&set, config).unwrap())
        }));
    }
    println!("{}", serde_json::to_string_pretty(&json!({
        "basic": basic, "secure_aggregation": secure_aggregation, "scalars": scalars, "storage": storage,
        "key_generation": key_generation
    })).unwrap());
}
