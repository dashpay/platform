use crate::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::KeyType;
use crate::util::hash::ripemd160_sha256;
use crate::ProtocolError;
use anyhow::anyhow;
#[cfg(feature = "ed25519-dalek")]
use dashcore::ed25519_dalek;
use dashcore::hashes::Hash;
use dashcore::key::Secp256k1;
use dashcore::secp256k1::SecretKey;
use dashcore::{Network, PublicKey as ECDSAPublicKey};
use platform_value::Bytes20;
#[cfg(feature = "bls-signatures")]
use {crate::bls_signatures, dashcore::blsful::Bls12381G2Impl};
impl IdentityPublicKeyHashMethodsV0 for IdentityPublicKeyV0 {
    /// Get the original public key hash
    fn public_key_hash(&self) -> Result<[u8; 20], ProtocolError> {
        if self.data.is_empty() {
            return Err(ProtocolError::EmptyPublicKeyDataError);
        }

        match self.key_type {
            KeyType::ECDSA_SECP256K1 => {
                let key = match self.data.len() {
                    // TODO: We need to update schema and tests for 65 len keys
                    65 | 33 => ECDSAPublicKey::from_slice(self.data.as_slice())
                        .map_err(|e| anyhow!("unable to create pub key - {}", e))?,
                    _ => {
                        return Err(ProtocolError::ParsingError(format!(
                            "the key length is invalid: {} Allowed sizes: 33 or 65 bytes for ecdsa key",
                            self.data.len()
                        )));
                    }
                };
                Ok(key.pubkey_hash().to_byte_array())
            }
            KeyType::BLS12_381 => {
                if self.data.len() != 48 {
                    Err(ProtocolError::ParsingError(format!(
                        "the key length is invalid: {} Allowed sizes: 48 bytes for bls key",
                        self.data.len()
                    )))
                } else {
                    Ok(ripemd160_sha256(self.data.as_slice()))
                }
            }
            KeyType::ECDSA_HASH160 | KeyType::BIP13_SCRIPT_HASH | KeyType::EDDSA_25519_HASH160 => {
                Ok(Bytes20::from_vec(self.data.to_vec())?.into_buffer())
            }
        }
    }

    fn validate_private_key_bytes(
        &self,
        private_key_bytes: &[u8; 32],
        network: Network,
    ) -> Result<bool, ProtocolError> {
        match self.key_type {
            KeyType::ECDSA_SECP256K1 => {
                let secp = Secp256k1::new();
                let secret_key = match SecretKey::from_byte_array(private_key_bytes) {
                    Ok(secret_key) => secret_key,
                    Err(_) => return Ok(false),
                };
                let private_key = dashcore::PrivateKey::new(secret_key, network);

                Ok(private_key.public_key(&secp).to_bytes() == self.data.as_slice())
            }
            KeyType::BLS12_381 => {
                #[cfg(feature = "bls-signatures")]
                {
                    let private_key: Option<bls_signatures::SecretKey<Bls12381G2Impl>> =
                        bls_signatures::SecretKey::<Bls12381G2Impl>::from_be_bytes(
                            private_key_bytes,
                        )
                        .into();
                    if private_key.is_none() {
                        return Ok(false);
                    }
                    let private_key = private_key.expect("expected private key");

                    Ok(private_key.public_key().0.to_compressed() == self.data.as_slice())
                }
                #[cfg(not(feature = "bls-signatures"))]
                return Err(ProtocolError::NotSupported(
                    "Converting a private key to a bls public key is not supported without the bls-signatures feature".to_string(),
                ));
            }
            KeyType::ECDSA_HASH160 => {
                let secp = Secp256k1::new();
                let secret_key = match SecretKey::from_byte_array(private_key_bytes) {
                    Ok(secret_key) => secret_key,
                    Err(_) => return Ok(false),
                };
                let private_key = dashcore::PrivateKey::new(secret_key, network);

                Ok(
                    ripemd160_sha256(private_key.public_key(&secp).to_bytes().as_slice())
                        .as_slice()
                        == self.data.as_slice(),
                )
            }
            KeyType::EDDSA_25519_HASH160 => {
                #[cfg(feature = "ed25519-dalek")]
                {
                    let key_pair = ed25519_dalek::SigningKey::from_bytes(private_key_bytes);
                    Ok(
                        ripemd160_sha256(key_pair.verifying_key().to_bytes().as_slice()).as_slice()
                            == self.data.as_slice(),
                    )
                }
                #[cfg(not(feature = "ed25519-dalek"))]
                return Err(ProtocolError::NotSupported(
                    "Converting a private key to a eddsa hash 160 is not supported without the ed25519-dalek feature".to_string(),
                ));
            }
            KeyType::BIP13_SCRIPT_HASH => Err(ProtocolError::NotSupported(
                "Converting a private key to a script hash is not supported".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{Purpose, SecurityLevel};
    use dashcore::blsful::{Bls12381G2Impl, Pairing, Signature, SignatureSchemes};
    use dashcore::Network;
    use dpp::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_bls_serialization_deserialization() {
        let mut rng = StdRng::seed_from_u64(5);
        let (public_key_data, secret_key) = KeyType::BLS12_381
            .random_public_and_private_key_data(&mut rng, PlatformVersion::latest())
            .expect("expected to get keys");
        let decoded_secret_key =
            dashcore::blsful::SecretKey::<Bls12381G2Impl>::from_be_bytes(&secret_key)
                .expect("expected to get secret key");
        let public_key = decoded_secret_key.public_key();
        let decoded_public_key_data = public_key.0.to_compressed();
        assert_eq!(
            public_key_data.as_slice(),
            decoded_public_key_data.as_slice()
        )
    }

    #[test]
    fn test_bls_serialization_deserialization_signature() {
        let mut rng = StdRng::seed_from_u64(5);
        let (_, secret_key) = KeyType::BLS12_381
            .random_public_and_private_key_data(&mut rng, PlatformVersion::latest())
            .expect("expected to get keys");
        let decoded_secret_key =
            dashcore::blsful::SecretKey::<Bls12381G2Impl>::from_be_bytes(&secret_key)
                .expect("expected to get secret key");
        let signature = decoded_secret_key
            .sign(SignatureSchemes::Basic, b"hello")
            .expect("expected to sign");
        let compressed = signature.as_raw_value().to_compressed();
        let g2 = <Bls12381G2Impl as Pairing>::Signature::from_compressed(&compressed)
            .expect("G2 projective");
        let decoded_signature = Signature::<Bls12381G2Impl>::Basic(g2);
        assert_eq!(
            compressed.as_slice(),
            decoded_signature.as_raw_value().to_compressed().as_slice()
        )
    }

    #[cfg(feature = "random-public-keys")]
    #[test]
    fn test_validate_private_key_bytes_with_random_keys() {
        let platform_version = PlatformVersion::latest();
        let mut rng = StdRng::from_entropy();

        // Test for ECDSA_SECP256K1
        let key_type = KeyType::ECDSA_SECP256K1;
        let (public_key_data, private_key_data) = key_type
            .random_public_and_private_key_data(&mut rng, platform_version)
            .expect("expected to generate random keys");

        let identity_public_key = IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type,
            data: public_key_data.into(),
            read_only: false,
            disabled_at: None,
        };

        // Validate that the private key matches the public key
        assert!(identity_public_key
            .validate_private_key_bytes(&private_key_data, Network::Testnet)
            .unwrap(),);

        // Test with an invalid private key
        let invalid_private_key_bytes = [0u8; 32];
        assert!(!identity_public_key
            .validate_private_key_bytes(&invalid_private_key_bytes, Network::Testnet)
            .unwrap());
    }

    #[cfg(all(feature = "random-public-keys", feature = "bls-signatures"))]
    #[test]
    fn test_validate_private_key_bytes_with_random_keys_bls12_381() {
        let platform_version = PlatformVersion::latest();
        let mut rng = StdRng::from_entropy();

        // Test for BLS12_381
        let key_type = KeyType::BLS12_381;
        let (public_key_data, private_key_data) = key_type
            .random_public_and_private_key_data(&mut rng, platform_version)
            .expect("expected to generate random keys");

        let identity_public_key = IdentityPublicKeyV0 {
            id: 2,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type,
            data: public_key_data.into(),
            read_only: false,
            disabled_at: None,
        };

        // Validate that the private key matches the public key
        assert!(identity_public_key
            .validate_private_key_bytes(&private_key_data, Network::Testnet)
            .unwrap());

        // Test with an invalid private key
        let invalid_private_key_bytes = [0u8; 32];
        assert!(!identity_public_key
            .validate_private_key_bytes(&invalid_private_key_bytes, Network::Testnet)
            .unwrap());
    }

    // -- public_key_hash error paths --

    #[test]
    fn test_public_key_hash_empty_data_errors() {
        use platform_value::BinaryData;
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            data: BinaryData::new(vec![]),
            read_only: false,
            disabled_at: None,
        };
        let err = key.public_key_hash().unwrap_err();
        assert!(matches!(err, ProtocolError::EmptyPublicKeyDataError));
    }

    #[test]
    fn test_public_key_hash_ecdsa_wrong_length_errors() {
        use platform_value::BinaryData;
        // ECDSA_SECP256K1 accepts only 33 or 65 bytes. 32 should fail with ParsingError.
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            data: BinaryData::new(vec![1u8; 32]),
            read_only: false,
            disabled_at: None,
        };
        let err = key.public_key_hash().unwrap_err();
        match err {
            ProtocolError::ParsingError(msg) => assert!(msg.contains("key length is invalid")),
            other => panic!("expected ParsingError, got {:?}", other),
        }
    }

    #[test]
    fn test_public_key_hash_bls_wrong_length_errors() {
        use platform_value::BinaryData;
        // BLS12_381 expects exactly 48 bytes.
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::BLS12_381,
            data: BinaryData::new(vec![1u8; 40]),
            read_only: false,
            disabled_at: None,
        };
        let err = key.public_key_hash().unwrap_err();
        match err {
            ProtocolError::ParsingError(msg) => assert!(msg.contains("48 bytes for bls key")),
            other => panic!("expected ParsingError, got {:?}", other),
        }
    }

    #[test]
    fn test_public_key_hash_bls_returns_ripemd160_sha256_of_data() {
        use crate::util::hash::ripemd160_sha256;
        use platform_value::BinaryData;
        let data = vec![7u8; 48];
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::BLS12_381,
            data: BinaryData::new(data.clone()),
            read_only: false,
            disabled_at: None,
        };
        let hash = key
            .public_key_hash()
            .expect("expected hash for 48-byte bls");
        assert_eq!(hash, ripemd160_sha256(data.as_slice()));
    }

    #[test]
    fn test_public_key_hash_ecdsa_hash160_returns_data_itself() {
        use platform_value::BinaryData;
        let data = vec![9u8; 20];
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            data: BinaryData::new(data.clone()),
            read_only: false,
            disabled_at: None,
        };
        let hash = key.public_key_hash().expect("expected hash");
        assert_eq!(hash.as_slice(), data.as_slice());
    }

    #[test]
    fn test_public_key_hash_bip13_script_hash_returns_data_itself() {
        use platform_value::BinaryData;
        let data = vec![3u8; 20];
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::BIP13_SCRIPT_HASH,
            data: BinaryData::new(data.clone()),
            read_only: false,
            disabled_at: None,
        };
        let hash = key.public_key_hash().expect("expected hash");
        assert_eq!(hash.as_slice(), data.as_slice());
    }

    #[test]
    fn test_public_key_hash_hash160_wrong_length_errors() {
        use platform_value::BinaryData;
        // Non-ECDSA hash variants route through Bytes20::from_vec, which should reject != 20.
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            data: BinaryData::new(vec![0u8; 19]),
            read_only: false,
            disabled_at: None,
        };
        assert!(key.public_key_hash().is_err());
    }

    // -- validate_private_key_bytes: BIP13 is unsupported and always errors --
    #[test]
    fn test_validate_private_key_bytes_bip13_script_hash_is_unsupported() {
        use platform_value::BinaryData;
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::BIP13_SCRIPT_HASH,
            data: BinaryData::new(vec![0u8; 20]),
            read_only: false,
            disabled_at: None,
        };
        let err = key
            .validate_private_key_bytes(&[0u8; 32], Network::Testnet)
            .unwrap_err();
        match err {
            ProtocolError::NotSupported(msg) => {
                assert!(msg.contains("script hash"));
            }
            other => panic!("expected NotSupported, got {:?}", other),
        }
    }

    // -- validate_private_key_bytes for ECDSA: bad secret key bytes are handled (Ok(false)) --
    #[test]
    fn test_validate_private_key_bytes_ecdsa_secret_key_parse_error_returns_false() {
        use platform_value::BinaryData;
        // All-zeroes is not a valid secp256k1 secret key; the code maps that
        // to Ok(false) rather than Err.
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            // The actual stored public key is irrelevant here because we never get past
            // the secret-key parse step.
            data: BinaryData::new(vec![0u8; 33]),
            read_only: false,
            disabled_at: None,
        };
        let ok = key
            .validate_private_key_bytes(&[0u8; 32], Network::Testnet)
            .unwrap();
        assert!(!ok);
    }

    #[test]
    fn test_validate_private_key_bytes_ecdsa_hash160_secret_key_parse_error_returns_false() {
        use platform_value::BinaryData;
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            data: BinaryData::new(vec![0u8; 20]),
            read_only: false,
            disabled_at: None,
        };
        let ok = key
            .validate_private_key_bytes(&[0u8; 32], Network::Testnet)
            .unwrap();
        assert!(!ok);
    }

    #[cfg(all(feature = "random-public-keys", feature = "ed25519-dalek"))]
    #[test]
    fn test_validate_private_key_bytes_with_random_keys_eddsa_25519_hash160() {
        let platform_version = PlatformVersion::latest();
        let mut rng = StdRng::from_entropy();

        // Test for EDDSA_25519_HASH160
        let key_type = KeyType::EDDSA_25519_HASH160;
        let (public_key_data, private_key_data) = key_type
            .random_public_and_private_key_data(&mut rng, platform_version)
            .expect("expected to generate random keys");

        let identity_public_key = IdentityPublicKeyV0 {
            id: 3,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type,
            data: public_key_data.into(),
            read_only: false,
            disabled_at: None,
        };

        // Validate that the private key matches the public key
        assert!(identity_public_key
            .validate_private_key_bytes(&private_key_data, Network::Testnet)
            .unwrap());

        // Test with an invalid private key
        let invalid_private_key_bytes = [0u8; 32];
        assert!(!identity_public_key
            .validate_private_key_bytes(&invalid_private_key_bytes, Network::Testnet)
            .unwrap());
    }
}
