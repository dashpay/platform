//! Key generation functionality for wallets
//!
//! Provides key generation and address derivation without full HD wallet support

use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::dashcore::hashes::{sha256, Hash};
use dash_sdk::dpp::dashcore::secp256k1::{Secp256k1, SecretKey};
use dash_sdk::dpp::dashcore::{Address, Network, PrivateKey, PublicKey};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// Key pair information
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// Private key in WIF format
    pub private_key_wif: String,
    /// Private key in hex format
    pub private_key_hex: String,
    /// Public key in hex format
    pub public_key: String,
    /// Address for the key
    pub address: String,
    /// Network (mainnet/testnet)
    pub network: String,
}

#[wasm_bindgen(js_name = "KeyPair")]
#[derive(Clone)]
pub struct KeyPairWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "privateKeyWif")]
    pub private_key_wif: String,
    #[wasm_bindgen(getter_with_clone, js_name = "privateKeyHex")]
    pub private_key_hex: String,
    #[wasm_bindgen(getter_with_clone, js_name = "publicKey")]
    pub public_key: String,
    #[wasm_bindgen(getter_with_clone)]
    pub address: String,
    #[wasm_bindgen(getter_with_clone)]
    pub network: String,
}

impl From<KeyPair> for KeyPairWasm {
    fn from(value: KeyPair) -> Self {
        Self {
            private_key_wif: value.private_key_wif,
            private_key_hex: value.private_key_hex,
            public_key: value.public_key,
            address: value.address,
            network: value.network,
        }
    }
}

#[wasm_bindgen]
impl WasmSdk {
    fn build_key_pair(
        private_key: &PrivateKey,
        network: Network,
        network_label: &str,
    ) -> Result<KeyPair, WasmSdkError> {
        let secp = Secp256k1::new();
        let public_key = private_key.public_key(&secp);
        let public_key_bytes = public_key.inner.serialize();
        let address = Address::p2pkh(&public_key, network);

        Ok(KeyPair {
            private_key_wif: private_key.to_wif(),
            private_key_hex: hex::encode(private_key.inner.secret_bytes()),
            public_key: hex::encode(public_key_bytes),
            address: address.to_string(),
            network: network_label.to_string(),
        })
    }

    /// Generate a new random key pair
    #[wasm_bindgen(js_name = "generateKeyPair")]
    pub fn generate_key_pair(network: &str) -> Result<KeyPairWasm, WasmSdkError> {
        let net = match network {
            "mainnet" => Network::Dash,
            "testnet" => Network::Testnet,
            _ => {
                return Err(WasmSdkError::invalid_argument(
                    "Invalid network. Use 'mainnet' or 'testnet'",
                ));
            }
        };

        // Generate random 32 bytes
        let mut key_bytes = [0u8; 32];
        getrandom::getrandom(&mut key_bytes).map_err(|e| {
            WasmSdkError::generic(format!("Failed to generate random bytes: {}", e))
        })?;

        // Create private key
        let private_key = PrivateKey::from_byte_array(&key_bytes, net)
            .map_err(|e| WasmSdkError::generic(format!("Failed to create private key: {}", e)))?;

        // Ensure secret key is valid before building info
        SecretKey::from_slice(&key_bytes)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;

        let key_pair = Self::build_key_pair(&private_key, net, network)?;
        Ok(KeyPairWasm::from(key_pair))
    }

    /// Generate multiple key pairs
    #[wasm_bindgen(js_name = "generateKeyPairs")]
    pub fn generate_key_pairs(network: &str, count: u32) -> Result<Vec<KeyPairWasm>, WasmSdkError> {
        if count == 0 || count > 100 {
            return Err(WasmSdkError::invalid_argument(
                "Count must be between 1 and 100",
            ));
        }

        let mut pairs = Vec::new();
        for _ in 0..count {
            pairs.push(Self::generate_key_pair(network)?);
        }
        Ok(pairs)
    }

    /// Create key pair from private key WIF
    #[wasm_bindgen(js_name = "keyPairFromWif")]
    pub fn key_pair_from_wif(
        #[wasm_bindgen(js_name = "privateKeyWif")] private_key_wif: &str,
    ) -> Result<KeyPairWasm, WasmSdkError> {
        let private_key = PrivateKey::from_wif(private_key_wif)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid WIF: {}", e)))?;

        let network = match private_key.network {
            Network::Dash => "mainnet",
            Network::Testnet => "testnet",
            _ => return Err(WasmSdkError::invalid_argument("Unsupported network")),
        };

        let key_pair = Self::build_key_pair(&private_key, private_key.network, network)?;
        Ok(KeyPairWasm::from(key_pair))
    }

    /// Create key pair from private key hex
    #[wasm_bindgen(js_name = "keyPairFromHex")]
    pub fn key_pair_from_hex(
        #[wasm_bindgen(js_name = "privateKeyHex")] private_key_hex: &str,
        network: &str,
    ) -> Result<KeyPairWasm, WasmSdkError> {
        if private_key_hex.len() != 64 {
            return Err(WasmSdkError::invalid_argument(
                "Private key hex must be exactly 64 characters",
            ));
        }

        let net = match network {
            "mainnet" => Network::Dash,
            "testnet" => Network::Testnet,
            _ => {
                return Err(WasmSdkError::invalid_argument(
                    "Invalid network. Use 'mainnet' or 'testnet'",
                ));
            }
        };

        let key_bytes = hex::decode(private_key_hex)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid hex: {}", e)))?;

        let key_array: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| WasmSdkError::invalid_argument("Private key bytes must be 32 bytes"))?;
        let private_key = PrivateKey::from_byte_array(&key_array, net)
            .map_err(|e| WasmSdkError::generic(format!("Failed to create private key: {}", e)))?;

        let key_pair = Self::build_key_pair(&private_key, net, network)?;
        Ok(KeyPairWasm::from(key_pair))
    }

    /// Get address from public key
    #[wasm_bindgen(js_name = "pubkeyToAddress")]
    pub fn pubkey_to_address(
        #[wasm_bindgen(js_name = "pubkeyHex")] pubkey_hex: &str,
        network: &str,
    ) -> Result<String, WasmSdkError> {
        let net = match network {
            "mainnet" => Network::Dash,
            "testnet" => Network::Testnet,
            _ => {
                return Err(WasmSdkError::invalid_argument(
                    "Invalid network. Use 'mainnet' or 'testnet'",
                ));
            }
        };

        let pubkey_bytes = hex::decode(pubkey_hex)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid hex: {}", e)))?;

        let public_key = PublicKey::from_slice(&pubkey_bytes)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid public key: {}", e)))?;

        let address = Address::p2pkh(&public_key, net);
        Ok(address.to_string())
    }

    /// Validate a Dash address
    #[wasm_bindgen(js_name = "validateAddress")]
    pub fn validate_address(address: &str, network: &str) -> bool {
        let net = match network {
            "mainnet" => Network::Dash,
            "testnet" => Network::Testnet,
            _ => return false,
        };

        Address::from_str(address)
            .map(|addr| *addr.network() == net)
            .unwrap_or(false)
    }

    /// Sign a message with a private key
    #[wasm_bindgen(js_name = "signMessage")]
    pub fn sign_message(
        message: &str,
        #[wasm_bindgen(js_name = "privateKeyWif")] private_key_wif: &str,
    ) -> Result<String, WasmSdkError> {
        let private_key = PrivateKey::from_wif(private_key_wif)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid WIF: {}", e)))?;

        // Create message hash
        let message_bytes = message.as_bytes();
        let hash = sha256::Hash::hash(message_bytes);

        // Sign the hash
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&private_key.inner.secret_bytes())
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid secret key: {}", e)))?;

        let message_hash =
            dash_sdk::dpp::dashcore::secp256k1::Message::from_digest(hash.to_byte_array());
        let signature = secp.sign_ecdsa(&message_hash, &secret_key);

        Ok(hex::encode(signature.serialize_compact()))
    }
}
