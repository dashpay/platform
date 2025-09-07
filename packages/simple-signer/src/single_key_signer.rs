use dpp::dashcore;
use dpp::dashcore::signer;
use dpp::dashcore::Network;
use dpp::dashcore::PrivateKey;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, KeyType};
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use tracing::{debug, warn};

/// A simple signer that uses a single private key
/// This is designed for WASM and other single-key use cases
#[derive(Debug, Clone)]
pub struct SingleKeySigner {
    private_key: PrivateKey,
}

impl SingleKeySigner {
    /// Create a new SingleKeySigner from a WIF-encoded private key
    pub fn new(private_key_wif: &str) -> Result<Self, String> {
        let private_key = PrivateKey::from_wif(private_key_wif)
            .map_err(|e| format!("Invalid WIF private key: {}", e))?;
        Ok(Self { private_key })
    }

    pub fn new_from_slice(private_key_data: &[u8], network: Network) -> Result<Self, String> {
        let private_key = PrivateKey::from_slice(private_key_data, network)
            .map_err(|e| format!("Invalid private key: {}", e))?;
        Ok(Self { private_key })
    }

    /// Create a new SingleKeySigner from a hex-encoded private key
    pub fn from_hex(private_key_hex: &str, network: dashcore::Network) -> Result<Self, String> {
        if private_key_hex.len() != 64 {
            return Err("Private key hex must be exactly 64 characters".to_string());
        }

        let key_bytes =
            hex::decode(private_key_hex).map_err(|e| format!("Invalid hex private key: {}", e))?;

        if key_bytes.len() != 32 {
            return Err("Private key must be 32 bytes".to_string());
        }

        let private_key = PrivateKey::from_slice(&key_bytes, network)
            .map_err(|e| format!("Invalid private key bytes: {}", e))?;

        Ok(Self { private_key })
    }

    /// Create a new SingleKeySigner from a private key
    pub fn from_private_key(private_key: PrivateKey) -> Self {
        Self { private_key }
    }

    /// Create from a hex or WIF string (auto-detect format)
    pub fn from_string(private_key_str: &str, network: dashcore::Network) -> Result<Self, String> {
        // Try hex first if it looks like hex, then WIF
        if private_key_str.len() == 64 && private_key_str.chars().all(|c| c.is_ascii_hexdigit()) {
            Self::from_hex(private_key_str, network)
        } else {
            Self::new(private_key_str)
        }
    }

    /// Get the private key
    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }
}

impl Signer for SingleKeySigner {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        // Only support ECDSA keys for now
        match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                // Do not log private key material. Log data fingerprint only.
                debug!(data_hex = %hex::encode(data), "SingleKeySigner: signing data");
                let signature = signer::sign(data, &self.private_key.inner.secret_bytes())?;
                Ok(signature.to_vec().into())
            }
            _ => {
                warn!(key_type = ?identity_public_key.key_type(), "SingleKeySigner: unsupported key type");
                Err(ProtocolError::Generic(format!(
                    "SingleKeySigner only supports ECDSA keys, got {:?}",
                    identity_public_key.key_type()
                )))
            }
        }
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        // Check if the public key matches our private key
        match identity_public_key.key_type() {
            KeyType::ECDSA_SECP256K1 => {
                // Compare full public key
                let secp = dashcore::secp256k1::Secp256k1::new();
                let secret_key = match dashcore::secp256k1::SecretKey::from_byte_array(
                    &self.private_key.inner.secret_bytes(),
                ) {
                    Ok(sk) => sk,
                    Err(_) => return false,
                };
                let public_key =
                    dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
                let public_key_bytes = public_key.serialize();

                identity_public_key.data().as_slice() == public_key_bytes
            }
            KeyType::ECDSA_HASH160 => {
                // Compare hash160 of public key
                use dpp::dashcore::hashes::{hash160, Hash};

                let secp = dashcore::secp256k1::Secp256k1::new();
                let secret_key = match dashcore::secp256k1::SecretKey::from_byte_array(
                    &self.private_key.inner.secret_bytes(),
                ) {
                    Ok(sk) => sk,
                    Err(_) => return false,
                };
                let public_key =
                    dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
                let public_key_bytes = public_key.serialize();
                let public_key_hash160 = hash160::Hash::hash(&public_key_bytes)
                    .to_byte_array()
                    .to_vec();

                identity_public_key.data().as_slice() == public_key_hash160.as_slice()
            }
            _ => false, // We only support ECDSA keys
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::Network;

    #[test]
    fn test_single_key_signer_from_wif() {
        // Create a valid testnet WIF
        let private_key = PrivateKey::from_slice(
            &[0x01; 32], // Valid 32-byte private key
            Network::Testnet,
        )
        .unwrap();
        let wif = private_key.to_wif();

        let signer = SingleKeySigner::new(&wif).unwrap();
        assert!(signer.private_key().to_wif().starts_with('c')); // Testnet WIF
        assert_eq!(signer.private_key().to_wif(), wif);
    }

    #[test]
    fn test_single_key_signer_from_hex() {
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let signer = SingleKeySigner::from_hex(hex, Network::Testnet).unwrap();
        assert_eq!(signer.private_key().inner.secret_bytes().len(), 32);
    }

    #[test]
    fn test_single_key_signer_auto_detect() {
        // Test hex detection
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let signer = SingleKeySigner::from_string(hex, Network::Testnet).unwrap();
        assert_eq!(signer.private_key().inner.secret_bytes().len(), 32);

        // Test WIF detection
        let private_key = PrivateKey::from_slice(
            &[0x02; 32], // Valid 32-byte private key
            Network::Testnet,
        )
        .unwrap();
        let wif = private_key.to_wif();

        let signer = SingleKeySigner::from_string(&wif, Network::Testnet).unwrap();
        assert!(signer.private_key().to_wif().starts_with('c'));
        assert_eq!(signer.private_key().to_wif(), wif);
    }
}
