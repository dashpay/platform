#[path = "fixtures/historical_hashes/vectors.rs"]
#[expect(
    dead_code,
    reason = "shared historical fixtures include other algorithms"
)]
mod vectors;

use dpp::address_funds::{AddressWitness, PlatformAddress};
use dpp::dashcore::hashes::{hash160, Hash};
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::serialization::PlatformMessageSignable;
use dpp::util::ecdsa::{
    parse_compact_signature, parse_public_key, sign, verify_data_signature, verify_hash_signature,
};
use dpp::ProtocolError;
use vectors::{ECDSA_PUBLIC_KEY, TRANSFER_ECDSA_SIGNATURE, TRANSFER_HASH, TRANSFER_SIGNABLE};

fn fixture() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        hex::decode(TRANSFER_SIGNABLE).unwrap(),
        hex::decode(TRANSFER_ECDSA_SIGNATURE).unwrap(),
        hex::decode(ECDSA_PUBLIC_KEY).unwrap(),
    )
}

#[test]
fn should_preserve_historical_signature_bytes_and_verification() {
    let (data, signature, public_key) = fixture();
    assert_eq!(sign(&data, &[1; 32]).unwrap().as_slice(), signature);
    verify_data_signature(&data, &signature, &public_key).unwrap();
    assert!(data
        .as_slice()
        .verify_signature(KeyType::ECDSA_SECP256K1, &public_key, &signature)
        .is_valid());
}

#[test]
fn should_preserve_all_compact_header_acceptance_rules() {
    let (_, mut signature, _) = fixture();
    for header in 0..=u8::MAX {
        signature[0] = header;
        let result = parse_compact_signature(&signature);
        if (27..=34).contains(&header) {
            let (id, compact) = result.unwrap().serialize_compact();
            assert_eq!(id.to_u8(), (header - 27) & 3);
            assert_eq!(compact.as_slice(), &signature[1..]);
        } else {
            assert_eq!(
                result.unwrap_err().to_string(),
                "invalid encoding",
                "header {header}"
            );
        }
    }
}

#[test]
fn should_preserve_compact_length_and_scalar_errors() {
    for len in [0, 1, 64, 66] {
        assert_eq!(
            parse_compact_signature(&vec![0; len])
                .unwrap_err()
                .to_string(),
            "length not 65 bytes"
        );
    }
    let order =
        hex::decode("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141").unwrap();
    for start in [1, 33] {
        for scalar in [order.as_slice(), &[0xff; 32]] {
            let (_, mut signature, _) = fixture();
            signature[start..start + 32].copy_from_slice(scalar);
            assert_eq!(
                parse_compact_signature(&signature).unwrap_err().to_string(),
                "invalid encoding"
            );
        }
    }
    // libsecp parses zero scalars, but neither verification nor recovery succeeds.
    let mut zero = [0; 65];
    zero[0] = 31;
    parse_compact_signature(&zero).unwrap();
    let (data, _, public_key) = fixture();
    assert_eq!(
        verify_data_signature(&data, &zero, &public_key)
            .unwrap_err()
            .to_string(),
        "signature failed verification"
    );
    assert_eq!(
        verify_hash_signature(&[0; 32], &zero, &[0; 20])
            .unwrap_err()
            .to_string(),
        "malformed signature"
    );
}

#[test]
fn should_preserve_public_key_encodings_and_error_text() {
    let (_, _, public_key) = fixture();
    let parsed = parse_public_key(&public_key).unwrap();
    let mut uncompressed = parsed.serialize_uncompressed();
    assert_eq!(parse_public_key(&uncompressed).unwrap(), parsed);
    for prefix in [0, 2, 3, 6, 7] {
        uncompressed[0] = prefix;
        assert_eq!(
            parse_public_key(&uncompressed).unwrap_err().to_string(),
            format!("key prefix invalid: {prefix}")
        );
    }
    for len in [0, 32, 34, 64, 66] {
        assert_eq!(
            parse_public_key(&vec![0; len]).unwrap_err().to_string(),
            format!("length {len} invalid for this base58 type")
        );
    }
    let mut invalid = [0xff; 33];
    invalid[0] = 2;
    assert_eq!(
        parse_public_key(&invalid).unwrap_err().to_string(),
        "key secp256k1 error"
    );
}

#[test]
fn should_report_invalid_signature_before_invalid_key_or_hash() {
    let (data, mut signature, _) = fixture();
    signature[1..33].fill(0xff);
    assert_eq!(
        verify_data_signature(&data, &signature, &[])
            .unwrap_err()
            .to_string(),
        "invalid encoding"
    );
    assert_eq!(
        verify_hash_signature(&[], &signature, &[])
            .unwrap_err()
            .to_string(),
        "invalid encoding"
    );
    let (_, valid_signature, _) = fixture();
    assert_eq!(
        verify_hash_signature(&[], &valid_signature, &[])
            .unwrap_err()
            .to_string(),
        "Invalid hash length"
    );
}

#[test]
fn should_recover_compressed_key_hash_for_both_header_compression_flags() {
    let (_, mut signature, public_key) = fixture();
    let hash = hex::decode(TRANSFER_HASH).unwrap();
    let key_hash = hash160::Hash::hash(&public_key).to_byte_array();
    for header in [27, 31] {
        signature[0] = header;
        verify_hash_signature(&hash, &signature, &key_hash).unwrap();
        assert_eq!(
            verify_hash_signature(&hash, &signature, &[0; 20])
                .unwrap_err()
                .to_string(),
            "the signature isn't valid"
        );
    }
}

#[test]
fn should_reject_high_s_verification_but_accept_high_s_recovery() {
    let (data, mut signature, public_key) = fixture();
    // n minus the historical signature's S, with the recovery parity flipped.
    signature[33..].copy_from_slice(
        &hex::decode("e952933fd7484a943559327bf88e9c0261fda84544e0b2f4736f786a1ad2c20b").unwrap(),
    );
    signature[0] = 32;
    assert_eq!(
        verify_data_signature(&data, &signature, &public_key)
            .unwrap_err()
            .to_string(),
        "signature failed verification"
    );
    verify_hash_signature(
        &hex::decode(TRANSFER_HASH).unwrap(),
        &signature,
        hash160::Hash::hash(&public_key).as_byte_array(),
    )
    .unwrap();
}

#[test]
fn should_preserve_multisig_malformed_signature_error() {
    let (data, mut signature, public_key) = fixture();
    let mut script = vec![0x51, 33];
    script.extend_from_slice(&public_key);
    script.extend_from_slice(&[0x51, 0xae]);
    let address = PlatformAddress::P2sh(hash160::Hash::hash(&script).to_byte_array());
    signature[0] = 35;
    let witness = AddressWitness::P2sh {
        signatures: vec![signature.into()],
        redeem_script: script.into(),
    };
    let error = address
        .verify_bytes_against_witness(&witness, &data)
        .unwrap_err();
    match error {
        ProtocolError::AddressWitnessError(message) => {
            assert_eq!(message, "Invalid signature format: invalid encoding")
        }
        error => panic!("unexpected error: {error}"),
    }
}

#[test]
fn should_preserve_multisig_malformed_public_key_error() {
    let (data, signature, _) = fixture();
    let mut script = vec![0x51, 33, 2];
    script.extend_from_slice(&[0xff; 32]);
    script.extend_from_slice(&[0x51, 0xae]);
    let address = PlatformAddress::P2sh(hash160::Hash::hash(&script).to_byte_array());
    let witness = AddressWitness::P2sh {
        signatures: vec![signature.into()],
        redeem_script: script.into(),
    };
    let error = address
        .verify_bytes_against_witness(&witness, &data)
        .unwrap_err();
    match error {
        ProtocolError::AddressWitnessError(message) => {
            assert_eq!(message, "Invalid public key: key secp256k1 error")
        }
        error => panic!("unexpected error: {error}"),
    }
}

#[test]
fn should_preserve_identity_public_key_hash_errors_and_encoding() {
    let (_, _, public_key) = fixture();
    let parsed = parse_public_key(&public_key).unwrap();
    let identity_key = |bytes: Vec<u8>| {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            data: bytes.into(),
            read_only: false,
            disabled_at: None,
        })
    };
    for bytes in [public_key, parsed.serialize_uncompressed().to_vec()] {
        assert_eq!(
            identity_key(bytes.clone()).public_key_hash().unwrap(),
            hash160::Hash::hash(&bytes).to_byte_array()
        );
    }
    let mut malformed = vec![0xff; 33];
    malformed[0] = 2;
    assert_eq!(
        identity_key(malformed)
            .public_key_hash()
            .unwrap_err()
            .to_string(),
        "unable to create pub key - key secp256k1 error"
    );
    let mut hybrid = parsed.serialize_uncompressed().to_vec();
    hybrid[0] = 6;
    assert_eq!(
        identity_key(hybrid)
            .public_key_hash()
            .unwrap_err()
            .to_string(),
        "unable to create pub key - key prefix invalid: 6"
    );
}
