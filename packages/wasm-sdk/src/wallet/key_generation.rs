//! Key generation functionality for wallets
//!
//! Provides key generation and address derivation without full HD wallet support

use dash_sdk::dpp::dashcore::hashes::{sha256, Hash};
use dash_sdk::dpp::dashcore::secp256k1::{Secp256k1, SecretKey};
use dash_sdk::dpp::dashcore::{Address, Network, PrivateKey, PublicKey};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use crate::error::new_structured_error;
use serde_json::json;

/// Key pair information
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Generate a new random key pair
#[wasm_bindgen]
pub fn generate_key_pair(network: &str) -> Result<JsValue, JsError> {
    let net = match network {
        "mainnet" => Network::Dash,
        "testnet" => Network::Testnet,
        _ => return Err(new_structured_error(
            "Invalid network",
            "E_INVALID_ARGUMENT",
            "argument",
            Some(json!({"field":"network","allowed":["mainnet","testnet"]})),
            Some(false),
        )),
    };

    // Generate random 32 bytes
    let mut key_bytes = [0u8; 32];
    getrandom::getrandom(&mut key_bytes)
        .map_err(|e| new_structured_error(&format!("Failed to generate random bytes: {}", e), "E_INTERNAL", "internal", None, Some(false)))?;

    // Create private key
    let private_key = PrivateKey::from_byte_array(&key_bytes, net)
        .map_err(|e| new_structured_error(&format!("Failed to create private key: {}", e), "E_INVALID_ARGUMENT", "argument", None, Some(false)))?;

    // Get public key
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&key_bytes)
        .map_err(|e| new_structured_error(&format!("Invalid secret key: {}", e), "E_INVALID_ARGUMENT", "argument", None, Some(false)))?;
    let public_key =
        dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let public_key_bytes = public_key.serialize();

    // Get address
    let address = Address::p2pkh(
        &PublicKey::from_slice(&public_key_bytes)
            .map_err(|e| new_structured_error(&format!("Failed to create public key: {}", e), "E_INVALID_ARGUMENT", "argument", None, Some(false)))?,
        net,
    );

    let key_pair = KeyPair {
        private_key_wif: private_key.to_wif(),
        private_key_hex: hex::encode(&key_bytes),
        public_key: hex::encode(&public_key_bytes),
        address: address.to_string(),
        network: network.to_string(),
    };

    serde_wasm_bindgen::to_value(&key_pair)
        .map_err(|e| new_structured_error(&format!("Failed to serialize key pair: {}", e), "E_INTERNAL", "internal", None, Some(false)))
}

/// Generate multiple key pairs
#[wasm_bindgen]
pub fn generate_key_pairs(network: &str, count: u32) -> Result<Vec<JsValue>, JsError> {
    if count == 0 || count > 100 {
        return Err(new_structured_error(
            "Count must be between 1 and 100",
            "E_INVALID_ARGUMENT",
            "argument",
            Some(json!({"field":"count","min":1,"max":100})),
            Some(false),
        ));
    }

    let mut pairs = Vec::new();
    for _ in 0..count {
        pairs.push(generate_key_pair(network)?);
    }
    Ok(pairs)
}

/// Create key pair from private key WIF
#[wasm_bindgen]
pub fn key_pair_from_wif(private_key_wif: &str) -> Result<JsValue, JsError> {
    let private_key = PrivateKey::from_wif(private_key_wif)
        .map_err(|e| new_structured_error(&format!("Invalid WIF: {}", e), "E_INVALID_ARGUMENT", "argument", Some(json!({"field":"privateKeyWif"})), Some(false)))?;

    let network = match private_key.network {
        Network::Dash => "mainnet",
        Network::Testnet => "testnet",
        _ => return Err(new_structured_error(
            "Unsupported network",
            "E_UNSUPPORTED",
            "unsupported",
            None,
            Some(false),
        )),
    };

    // Get public key
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&private_key.inner.secret_bytes())
        .map_err(|e| new_structured_error(&format!("Invalid secret key: {}", e), "E_INVALID_ARGUMENT", "argument", None, Some(false)))?;
    let public_key =
        dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let public_key_bytes = public_key.serialize();

    // Get address
    let address = Address::p2pkh(
        &PublicKey::from_slice(&public_key_bytes)
        .map_err(|e| new_structured_error(&format!("Failed to create public key: {}", e), "E_INVALID_ARGUMENT", "argument", None, Some(false)))?,
        private_key.network,
    );

    let key_pair = KeyPair {
        private_key_wif: private_key_wif.to_string(),
        private_key_hex: hex::encode(&private_key.inner.secret_bytes()),
        public_key: hex::encode(&public_key_bytes),
        address: address.to_string(),
        network: network.to_string(),
    };

    serde_wasm_bindgen::to_value(&key_pair)
        .map_err(|e| new_structured_error(&format!("Failed to serialize key pair: {}", e), "E_INTERNAL", "internal", None, Some(false)))
}

/// Create key pair from private key hex
#[wasm_bindgen]
pub fn key_pair_from_hex(private_key_hex: &str, network: &str) -> Result<JsValue, JsError> {
    if private_key_hex.len() != 64 {
        return Err(new_structured_error(
            "Private key hex must be exactly 64 characters",
            "E_INVALID_ARGUMENT",
            "argument",
            Some(json!({"field":"privateKeyHex","length":64})),
            Some(false),
        ));
    }

    let net = match network {
        "mainnet" => Network::Dash,
        "testnet" => Network::Testnet,
        _ => return Err(new_structured_error(
            "Invalid network",
            "E_INVALID_ARGUMENT",
            "argument",
            Some(json!({"field":"network","allowed":["mainnet","testnet"]})),
            Some(false),
        )),
    };

    let key_bytes =
        hex::decode(private_key_hex).map_err(|e| new_structured_error(&format!("Invalid hex: {}", e), "E_INVALID_ARGUMENT", "argument", Some(json!({"field":"privateKeyHex"})), Some(false)))?;

    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| new_structured_error("Private key bytes must be 32 bytes", "E_INVALID_ARGUMENT", "argument", Some(json!({"field":"privateKeyHex","bytes":32})), Some(false)))?;
    let private_key = PrivateKey::from_byte_array(&key_array, net)
        .map_err(|e| new_structured_error(&format!("Failed to create private key: {}", e), "E_INVALID_ARGUMENT", "argument", None, Some(false)))?;

    key_pair_from_wif(&private_key.to_wif())
}

/// Get address from public key
#[wasm_bindgen]
pub fn pubkey_to_address(pubkey_hex: &str, network: &str) -> Result<String, JsError> {
    let net = match network {
        "mainnet" => Network::Dash,
        "testnet" => Network::Testnet,
        _ => return Err(new_structured_error(
            "Invalid network",
            "E_INVALID_ARGUMENT",
            "argument",
            Some(json!({"field":"network","allowed":["mainnet","testnet"]})),
            Some(false),
        )),
    };

    let pubkey_bytes =
        hex::decode(pubkey_hex).map_err(|e| new_structured_error(&format!("Invalid hex: {}", e), "E_INVALID_ARGUMENT", "argument", Some(json!({"field":"pubkeyHex"})), Some(false)))?;

    let public_key = PublicKey::from_slice(&pubkey_bytes)
        .map_err(|e| new_structured_error(&format!("Invalid public key: {}", e), "E_INVALID_ARGUMENT", "argument", None, Some(false)))?;

    let address = Address::p2pkh(&public_key, net);
    Ok(address.to_string())
}

/// Validate a Dash address
#[wasm_bindgen]
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
#[wasm_bindgen]
pub fn sign_message(message: &str, private_key_wif: &str) -> Result<String, JsError> {
    let private_key = PrivateKey::from_wif(private_key_wif)
        .map_err(|e| new_structured_error(&format!("Invalid WIF: {}", e), "E_INVALID_ARGUMENT", "argument", Some(json!({"field":"privateKeyWif"})), Some(false)))?;

    // Create message hash
    let message_bytes = message.as_bytes();
    let hash = sha256::Hash::hash(message_bytes);

    // Sign the hash
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&private_key.inner.secret_bytes())
        .map_err(|e| new_structured_error(&format!("Invalid secret key: {}", e), "E_INVALID_ARGUMENT", "argument", None, Some(false)))?;

    let message_hash =
        dash_sdk::dpp::dashcore::secp256k1::Message::from_digest(hash.to_byte_array());
    let signature = secp.sign_ecdsa(&message_hash, &secret_key);

    Ok(hex::encode(signature.serialize_compact()))
}
