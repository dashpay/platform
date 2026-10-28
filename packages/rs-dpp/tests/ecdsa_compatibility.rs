#[path = "fixtures/historical_hashes/vectors.rs"]
#[expect(
    dead_code,
    reason = "shared historical fixtures include other algorithms"
)]
mod vectors;

use dpp::address_funds::{AddressWitness, AddressWitnessVerificationOperations, PlatformAddress};
use dpp::consensus::codes::ErrorWithCode;
use dpp::dashcore::crypto::ecdsa::EcdsaError;
use dpp::dashcore::hashes::{hash160, Hash};
use dpp::dashcore::key::Error as KeyError;
use dpp::dashcore::sign_message::MessageSignatureError;
use dpp::dashcore::signer::{sign, verify_data_signature, verify_hash_signature};
use dpp::dashcore::{sign_message, PublicKey};
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::secp256k1::{ecdsa::RecoverableSignature, ecdsa::RecoveryId, Message};
use dpp::serialization::PlatformMessageSignable;
use dpp::signing::{is_low_s, verify_low_s_data_signature};
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
fn should_match_historical_verification_for_all_headers_and_scalar_boundaries() {
    let (data, signature, public_key) = fixture();
    let message =
        Message::from_digest(dpp::dashcore::hashes::sha256d::Hash::hash(&data).to_byte_array());
    let raw_key = dpp::secp256k1::PublicKey::from_slice(&public_key).unwrap();
    let order =
        hex::decode("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141").unwrap();
    let mut signatures = vec![signature.clone()];
    for start in [1, 33] {
        for scalar in [&[0; 32][..], order.as_slice(), &[0xff; 32][..]] {
            let mut candidate = signature.clone();
            candidate[start..start + 32].copy_from_slice(scalar);
            signatures.push(candidate);
        }
    }
    for mut candidate in signatures {
        for header in 0..=u8::MAX {
            candidate[0] = header;
            // Baseline Core parsed recovery metadata before verifying with libsecp256k1.
            let historical_valid = (27..=34).contains(&header) && {
                let id = RecoveryId::try_from(i32::from((header - 27) & 3)).unwrap();
                RecoverableSignature::from_compact(&candidate[1..], id)
                    .and_then(|sig| sig.to_standard().verify(message, &raw_key))
                    .is_ok()
            };
            let result =
                data.as_slice()
                    .verify_signature(KeyType::ECDSA_SECP256K1, &public_key, &candidate);
            assert_eq!(result.is_valid(), historical_valid, "header {header}");
            if !historical_valid {
                assert_eq!(result.errors[0].code(), 20009);
            }
        }
    }
}

#[test]
fn should_preserve_witness_fee_operations_for_recovery_and_multisig_search() {
    let (data, signature, public_key) = fixture();
    let address = PlatformAddress::P2pkh(hash160::Hash::hash(&public_key).to_byte_array());
    let witness = AddressWitness::P2pkh {
        signature: signature.clone().into(),
    };
    assert_eq!(
        address
            .verify_bytes_against_witness(&witness, &data)
            .unwrap(),
        AddressWitnessVerificationOperations::for_p2pkh(data.len())
    );

    let other = dpp::secp256k1::SecretKey::from_secret_bytes([2; 32]).unwrap();
    let other_key = dpp::secp256k1::PublicKey::from_secret_key(&other).serialize();
    let mut script = vec![0x51, 33];
    script.extend_from_slice(&other_key);
    script.push(33);
    script.extend_from_slice(&public_key);
    script.extend_from_slice(&[0x52, 0xae]);
    let address = PlatformAddress::P2sh(hash160::Hash::hash(&script).to_byte_array());
    let witness = AddressWitness::P2sh {
        signatures: vec![signature.into()],
        redeem_script: script.into(),
    };
    assert_eq!(
        address
            .verify_bytes_against_witness(&witness, &data)
            .unwrap(),
        AddressWitnessVerificationOperations::for_p2sh_multisig(2, data.len())
    );
    assert!(address
        .verify_bytes_against_witness(&witness, b"different message")
        .is_err());
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
        let result = sign_message::from_slice(&signature).unwrap().signature();
        if (27..=34).contains(&header) {
            let parsed = sign_message::from_slice(&signature).unwrap();
            assert_eq!(parsed.recovery_id().unwrap(), (header - 27) & 3);
            assert_eq!(result.unwrap().to_bytes().as_slice(), &signature[1..]);
        } else {
            assert_eq!(
                result.unwrap_err(),
                EcdsaError::InvalidRecoveryId,
                "header {header}"
            );
        }
    }
}

#[test]
fn should_preserve_compact_length_and_scalar_errors() {
    for len in [0, 1, 64, 66] {
        assert_eq!(
            sign_message::from_slice(&vec![0; len]).unwrap_err(),
            MessageSignatureError::InvalidLength
        );
    }
    let order =
        hex::decode("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141").unwrap();
    for start in [1, 33] {
        for scalar in [order.as_slice(), &[0xff; 32]] {
            let (_, mut signature, _) = fixture();
            signature[start..start + 32].copy_from_slice(scalar);
            assert_eq!(
                sign_message::from_slice(&signature)
                    .unwrap()
                    .signature()
                    .unwrap_err(),
                EcdsaError::InvalidSignature
            );
        }
    }
    // libsecp parses zero scalars, but neither verification nor recovery succeeds.
    let mut zero = [0; 65];
    zero[0] = 31;
    sign_message::from_slice(&zero)
        .unwrap()
        .signature()
        .unwrap();
    let (data, _, public_key) = fixture();
    assert_eq!(
        verify_data_signature(&data, &zero, &public_key)
            .unwrap_err()
            .downcast_ref::<EcdsaError>(),
        Some(&EcdsaError::VerifyFailed)
    );
    assert_eq!(
        verify_hash_signature(&[0; 32], &zero, &[0; 20])
            .unwrap_err()
            .downcast_ref::<EcdsaError>(),
        Some(&EcdsaError::RecoveryFailed)
    );
}

#[test]
fn should_preserve_public_key_encodings() {
    let (_, _, public_key) = fixture();
    let parsed = PublicKey::from_slice(&public_key).unwrap();
    let mut uncompressed = parsed.inner.to_uncompressed();
    assert_eq!(
        PublicKey::from_slice(&uncompressed).unwrap().inner,
        parsed.inner
    );
    for prefix in [0, 2, 3, 6, 7] {
        uncompressed[0] = prefix;
        assert_eq!(
            PublicKey::from_slice(&uncompressed).unwrap_err(),
            KeyError::InvalidKeyPrefix(prefix)
        );
    }
    for len in [0, 32, 34, 64, 66] {
        assert_eq!(
            PublicKey::from_slice(&vec![0; len]).unwrap_err(),
            KeyError::InvalidBase58PayloadLength(len)
        );
    }
    let mut invalid = [0xff; 33];
    invalid[0] = 2;
    assert_eq!(
        PublicKey::from_slice(&invalid).unwrap_err(),
        KeyError::Ecdsa(EcdsaError::InvalidPublicKey)
    );
}

#[test]
fn should_reject_malformed_inputs_with_stable_consensus_codes() {
    let (data, mut signature, public_key) = fixture();
    signature[1..33].fill(0xff);
    for key in [public_key.as_slice(), &[]] {
        let result = data
            .as_slice()
            .verify_signature(KeyType::ECDSA_SECP256K1, key, &signature);
        assert!(!result.is_valid());
        assert_eq!(result.errors[0].code(), 20009);
    }
    assert!(verify_hash_signature(&[], &signature, &[]).is_err());
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

/// The historical signature with `n - S` in place of S and the recovery parity flipped:
/// the same statement, signed with the malleable twin.
fn high_s_fixture() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (data, mut signature, public_key) = fixture();
    signature[33..].copy_from_slice(
        &hex::decode("e952933fd7484a943559327bf88e9c0261fda84544e0b2f4736f786a1ad2c20b").unwrap(),
    );
    signature[0] = 32;
    (data, signature, public_key)
}

#[test]
fn should_reject_high_s_verification_but_accept_high_s_recovery() {
    let (data, signature, public_key) = high_s_fixture();
    // The rule itself, independent of what the curve backend does with high-S: on the
    // current pin the backend refuses it, from rust-dashcore#1214 on it normalizes it.
    let plain = |bytes: &[u8]| {
        sign_message::from_slice(bytes)
            .unwrap()
            .signature()
            .unwrap()
    };
    assert!(is_low_s(&plain(&fixture().1)));
    assert!(!is_low_s(&plain(&signature)));
    // Platform's rule, stated in dpp: `dashcore::signer::verify_data_signature` itself
    // normalizes S from rust-dashcore#1214 on, so it is no longer the place to assert this.
    assert_eq!(
        verify_low_s_data_signature(&data, &signature, &public_key)
            .unwrap_err()
            .downcast_ref::<EcdsaError>(),
        Some(&EcdsaError::VerifyFailed)
    );
    let result =
        data.as_slice()
            .verify_signature(KeyType::ECDSA_SECP256K1, &public_key, &signature);
    assert!(!result.is_valid());
    assert_eq!(result.errors[0].code(), 20009);
    verify_hash_signature(
        &hex::decode(TRANSFER_HASH).unwrap(),
        &signature,
        hash160::Hash::hash(&public_key).as_byte_array(),
    )
    .unwrap();
}

#[test]
fn should_reject_high_s_signature_in_multisig_witness() {
    let (data, low_s, public_key) = fixture();
    let (_, high_s, _) = high_s_fixture();
    let mut script = vec![0x51, 33];
    script.extend_from_slice(&public_key);
    script.extend_from_slice(&[0x51, 0xae]);
    let address = PlatformAddress::P2sh(hash160::Hash::hash(&script).to_byte_array());
    let witness = |signature: Vec<u8>| AddressWitness::P2sh {
        signatures: vec![signature.into()],
        redeem_script: script.clone().into(),
    };
    address
        .verify_bytes_against_witness(&witness(low_s), &data)
        .unwrap();
    match address
        .verify_bytes_against_witness(&witness(high_s), &data)
        .unwrap_err()
    {
        ProtocolError::AddressWitnessError(message) => {
            assert_eq!(message, "Not enough valid signatures: verified 0, need 1")
        }
        error => panic!("unexpected error: {error}"),
    }
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
            assert_eq!(message, "Invalid public key: key ECDSA error")
        }
        error => panic!("unexpected error: {error}"),
    }
}

#[test]
fn should_use_core_identity_public_key_errors_and_preserve_encoding() {
    let (_, _, public_key) = fixture();
    let parsed = PublicKey::from_slice(&public_key).unwrap();
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
    for bytes in [public_key, parsed.inner.to_uncompressed().to_vec()] {
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
        "unable to create pub key - key ECDSA error"
    );
    let mut hybrid = parsed.inner.to_uncompressed().to_vec();
    hybrid[0] = 6;
    assert_eq!(
        identity_key(hybrid)
            .public_key_hash()
            .unwrap_err()
            .to_string(),
        "unable to create pub key - key prefix invalid: 6"
    );
}
