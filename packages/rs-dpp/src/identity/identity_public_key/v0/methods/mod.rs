use crate::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::KeyType;
use crate::util::hash::ripemd160_sha256;
use crate::ProtocolError;
use anyhow::anyhow;
use dashcore::hashes::Hash;
use dashcore::key::Secp256k1;
use dashcore::secp256k1::SecretKey;
use dashcore::{Network, PublicKey as ECDSAPublicKey};
use platform_value::Bytes20;

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
        private_key_bytes: &[u8],
        network: Network,
    ) -> Result<bool, ProtocolError> {
        match self.key_type {
            KeyType::ECDSA_SECP256K1 => {
                let secp = Secp256k1::new();
                let secret_key = match SecretKey::from_slice(private_key_bytes) {
                    Ok(secret_key) => secret_key,
                    Err(_) => return Ok(false),
                };
                let private_key = dashcore::PrivateKey::new(secret_key, network);

                Ok(private_key.public_key(&secp).to_bytes() == self.data.as_slice())
            }
            KeyType::BLS12_381 => {
                #[cfg(feature = "bls-signatures")]
                {
                    let private_key =
                        match bls_signatures::PrivateKey::from_bytes(private_key_bytes, false) {
                            Ok(secret_key) => secret_key,
                            Err(_) => return Ok(false),
                        };
                    let public_key_bytes = private_key
                        .g1_element()
                        .expect("expected to get a public key from a bls private key")
                        .to_bytes()
                        .to_vec();
                    Ok(public_key_bytes == self.data.as_slice())
                }
                #[cfg(not(feature = "bls-signatures"))]
                return Err(ProtocolError::NotSupported(
                    "Converting a private key to a bls public key is not supported without the bls-signatures feature".to_string(),
                ));
            }
            KeyType::ECDSA_HASH160 => {
                let secp = Secp256k1::new();
                let secret_key = match SecretKey::from_slice(private_key_bytes) {
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
                    let secret_key = match private_key_bytes.try_into() {
                        Ok(secret_key) => secret_key,
                        Err(_) => return Ok(false),
                    };
                    let key_pair = ed25519_dalek::SigningKey::from_bytes(&secret_key);
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
            KeyType::BIP13_SCRIPT_HASH => {
                return Err(ProtocolError::NotSupported(
                    "Converting a private key to a script hash is not supported".to_string(),
                ));
            }
        }
    }
}
