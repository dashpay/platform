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
