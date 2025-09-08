//! Key generation functionality for wallets
//! 
//! Provides key generation and address derivation without full HD wallet support

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use dash_sdk::dpp::dashcore::{Network, PrivateKey, PublicKey, Address};
use dash_sdk::dpp::dashcore::secp256k1::{Secp256k1, SecretKey};
use dash_sdk::dpp::dashcore::hashes::{Hash, sha256};
use std::str::FromStr;
use dash_sdk::dpp::dashcore;

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
        _ => return Err(JsError::new("Invalid network. Use 'mainnet' or 'testnet'")),
    };

    // Generate random 32 bytes
    let mut key_bytes = [0u8; 32];
    getrandom::getrandom(&mut key_bytes)
        .map_err(|e| JsError::new(&format!("Failed to generate random bytes: {}", e)))?;

    // Create private key
    let private_key = PrivateKey::from_byte_array(&key_bytes, net)
        .map_err(|e| JsError::new(&format!("Failed to create private key: {}", e)))?;

    // Get public key
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&key_bytes)
        .map_err(|e| JsError::new(&format!("Invalid secret key: {}", e)))?;
    let public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let public_key_bytes = public_key.serialize();

    // Get address
    let address = Address::p2pkh(&PublicKey::from_slice(&public_key_bytes)
        .map_err(|e| JsError::new(&format!("Failed to create public key: {}", e)))?, net);

    let key_pair = KeyPair {
        private_key_wif: private_key.to_wif(),
        private_key_hex: hex::encode(&key_bytes),
        public_key: hex::encode(&public_key_bytes),
        address: address.to_string(),
        network: network.to_string(),
    };

    serde_wasm_bindgen::to_value(&key_pair)
        .map_err(|e| JsError::new(&format!("Failed to serialize key pair: {}", e)))
}

/// Generate multiple key pairs
#[wasm_bindgen]
pub fn generate_key_pairs(network: &str, count: u32) -> Result<Vec<JsValue>, JsError> {
    if count == 0 || count > 100 {
        return Err(JsError::new("Count must be between 1 and 100"));
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
        .map_err(|e| JsError::new(&format!("Invalid WIF: {}", e)))?;

    let network = match private_key.network {
        Network::Dash => "mainnet",
        Network::Testnet => "testnet",
        _ => return Err(JsError::new("Unsupported network")),
    };

    // Get public key
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&private_key.inner.secret_bytes())
        .map_err(|e| JsError::new(&format!("Invalid secret key: {}", e)))?;
    let public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let public_key_bytes = public_key.serialize();

    // Get address
    let address = Address::p2pkh(&PublicKey::from_slice(&public_key_bytes)
        .map_err(|e| JsError::new(&format!("Failed to create public key: {}", e)))?, 
        private_key.network);

    let key_pair = KeyPair {
        private_key_wif: private_key_wif.to_string(),
        private_key_hex: hex::encode(&private_key.inner.secret_bytes()),
        public_key: hex::encode(&public_key_bytes),
        address: address.to_string(),
        network: network.to_string(),
    };

    serde_wasm_bindgen::to_value(&key_pair)
        .map_err(|e| JsError::new(&format!("Failed to serialize key pair: {}", e)))
}

/// Create key pair from private key hex
#[wasm_bindgen]
pub fn key_pair_from_hex(private_key_hex: &str, network: &str) -> Result<JsValue, JsError> {
    if private_key_hex.len() != 64 {
        return Err(JsError::new("Private key hex must be exactly 64 characters"));
    }

    let net = match network {
        "mainnet" => Network::Dash,
        "testnet" => Network::Testnet,
        _ => return Err(JsError::new("Invalid network. Use 'mainnet' or 'testnet'")),
    };

    let key_bytes = hex::decode(private_key_hex)
        .map_err(|e| JsError::new(&format!("Invalid hex: {}", e)))?;

    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| JsError::new("Private key bytes must be 32 bytes"))?;
    let private_key = PrivateKey::from_byte_array(&key_array, net)
        .map_err(|e| JsError::new(&format!("Failed to create private key: {}", e)))?;

    key_pair_from_wif(&private_key.to_wif())
}

/// Get address from public key
#[wasm_bindgen]
pub fn pubkey_to_address(pubkey_hex: &str, network: &str) -> Result<String, JsError> {
    let net = match network {
        "mainnet" => Network::Dash,
        "testnet" => Network::Testnet,
        _ => return Err(JsError::new("Invalid network. Use 'mainnet' or 'testnet'")),
    };

    let pubkey_bytes = hex::decode(pubkey_hex)
        .map_err(|e| JsError::new(&format!("Invalid hex: {}", e)))?;

    let public_key = PublicKey::from_slice(&pubkey_bytes)
        .map_err(|e| JsError::new(&format!("Invalid public key: {}", e)))?;

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
        .map_err(|e| JsError::new(&format!("Invalid WIF: {}", e)))?;

    // Create message hash
    let message_bytes = message.as_bytes();
    let hash = sha256::Hash::hash(message_bytes);

    // Sign the hash
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&private_key.inner.secret_bytes())
        .map_err(|e| JsError::new(&format!("Invalid secret key: {}", e)))?;
    
    let message_hash = dash_sdk::dpp::dashcore::secp256k1::Message::from_digest(hash.to_byte_array());
    let signature = secp.sign_ecdsa(&message_hash, &secret_key);

    Ok(hex::encode(signature.serialize_compact()))
}
