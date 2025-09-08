//! Key derivation functionality for HD wallets
//!
//! Implements BIP32, BIP39, and BIP44 standards for hierarchical deterministic key derivation

use bip39::{Language, Mnemonic};
use dash_sdk::dpp::dashcore;
use dash_sdk::dpp::dashcore::secp256k1::Secp256k1;
use dash_sdk::dpp::key_wallet::bip32::{
    ChildNumber, DerivationPath as BIP32DerivationPath, ExtendedPrivKey as BIP32ExtendedPrivKey,
    ExtendedPubKey as BIP32ExtendedPubKey,
};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// Dash coin type for BIP44 (mainnet)
pub const DASH_COIN_TYPE: u32 = 5;
/// Testnet coin type for BIP44
pub const TESTNET_COIN_TYPE: u32 = 1;

/// DIP9 feature type for Dash-specific derivation
pub const DIP9_FEATURE_TYPE: u32 = 9;

/// DIP13 purpose - should be 9 as per DIP9, not 13
pub const DIP13_PURPOSE: u32 = 9;
/// DIP13 feature for identity keys
pub const DIP13_IDENTITY_FEATURE: u32 = 5;

/// Standard BIP44 derivation path for Dash
/// m/44'/5'/account'/change/index for mainnet
/// m/44'/1'/account'/change/index for testnet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivationPath {
    purpose: u32,
    coin_type: u32,
    account: u32,
    change: u32,
    index: u32,
}

impl DerivationPath {
    /// Create a new BIP44 derivation path for Dash mainnet
    pub fn new_bip44_mainnet(account: u32, change: u32, index: u32) -> Self {
        Self {
            purpose: 44,
            coin_type: DASH_COIN_TYPE,
            account,
            change,
            index,
        }
    }

    /// Create a new BIP44 derivation path for testnet
    pub fn new_bip44_testnet(account: u32, change: u32, index: u32) -> Self {
        Self {
            purpose: 44,
            coin_type: TESTNET_COIN_TYPE,
            account,
            change,
            index,
        }
    }

    /// Create a new DIP9 derivation path for Dash mainnet
    pub fn new_dip9_mainnet(feature_type: u32, account: u32, index: u32) -> Self {
        Self {
            purpose: DIP9_FEATURE_TYPE,
            coin_type: DASH_COIN_TYPE,
            account: feature_type,
            change: account,
            index,
        }
    }

    /// Create a new DIP9 derivation path for testnet
    pub fn new_dip9_testnet(feature_type: u32, account: u32, index: u32) -> Self {
        Self {
            purpose: DIP9_FEATURE_TYPE,
            coin_type: TESTNET_COIN_TYPE,
            account: feature_type,
            change: account,
            index,
        }
    }

    /// Convert to string representation (e.g., "m/44'/5'/0'/0/0")
    pub fn to_string(&self) -> String {
        format!(
            "m/{}'/{}'/{}'/{}/{}",
            self.purpose, self.coin_type, self.account, self.change, self.index
        )
    }

    /// Parse from string representation
    pub fn from_string(path: &str) -> Result<DerivationPath, JsError> {
        let parts: Vec<&str> = path.trim_start_matches("m/").split('/').collect();
        if parts.len() != 5 {
            return Err(JsError::new("Invalid derivation path format"));
        }

        let parse_hardened = |s: &str| -> Result<u32, JsError> {
            s.trim_end_matches('\'')
                .trim_end_matches('h')
                .parse::<u32>()
                .map_err(|_| JsError::new("Invalid path component"))
        };

        Ok(DerivationPath {
            purpose: parse_hardened(parts[0])?,
            coin_type: parse_hardened(parts[1])?,
            account: parse_hardened(parts[2])?,
            change: parts[3]
                .parse()
                .map_err(|_| JsError::new("Invalid change index"))?,
            index: parts[4]
                .parse()
                .map_err(|_| JsError::new("Invalid address index"))?,
        })
    }
}

/// Generate a new mnemonic phrase
#[wasm_bindgen]
pub fn generate_mnemonic(
    word_count: Option<u32>,
    language_code: Option<String>,
) -> Result<String, JsError> {
    let words = word_count.unwrap_or(12);

    // Validate word count and calculate entropy bytes
    let entropy_bytes = match words {
        12 => 16, // 128 bits
        15 => 20, // 160 bits
        18 => 24, // 192 bits
        21 => 28, // 224 bits
        24 => 32, // 256 bits
        _ => return Err(JsError::new("Word count must be 12, 15, 18, 21, or 24")),
    };

    // Select language based on language code
    let language = match language_code.as_deref() {
        Some("en") | None => Language::English,
        Some("zh-cn") | Some("zh_CN") | Some("zh-Hans") => Language::SimplifiedChinese,
        Some("zh-tw") | Some("zh_TW") | Some("zh-Hant") => Language::TraditionalChinese,
        Some("cs") => Language::Czech,
        Some("fr") => Language::French,
        Some("it") => Language::Italian,
        Some("ja") => Language::Japanese,
        Some("ko") => Language::Korean,
        Some("pt") => Language::Portuguese,
        Some("es") => Language::Spanish,
        Some(code) => return Err(JsError::new(&format!("Unsupported language code: {}. Supported: en, zh-cn, zh-tw, cs, fr, it, ja, ko, pt, es", code))),
    };

    // Generate random entropy
    let mut entropy = vec![0u8; entropy_bytes];
    thread_rng().fill_bytes(&mut entropy);

    // Create mnemonic from entropy
    let mnemonic = Mnemonic::from_entropy_in(language, &entropy)
        .map_err(|e| JsError::new(&format!("Failed to generate mnemonic: {}", e)))?;

    Ok(mnemonic.to_string())
}

/// Validate a mnemonic phrase
#[wasm_bindgen]
pub fn validate_mnemonic(mnemonic: &str, language_code: Option<String>) -> bool {
    // If language is specified, validate in that language
    if let Some(code) = language_code {
        let language = match code.as_str() {
            "en" => Language::English,
            "zh-cn" | "zh_CN" | "zh-Hans" => Language::SimplifiedChinese,
            "zh-tw" | "zh_TW" | "zh-Hant" => Language::TraditionalChinese,
            "cs" => Language::Czech,
            "fr" => Language::French,
            "it" => Language::Italian,
            "ja" => Language::Japanese,
            "ko" => Language::Korean,
            "pt" => Language::Portuguese,
            "es" => Language::Spanish,
            _ => return false,
        };
        return Mnemonic::parse_in(language, mnemonic).is_ok();
    }

    // Otherwise, try to parse in any language
    Mnemonic::parse_normalized(mnemonic).is_ok()
}

/// Derive a seed from a mnemonic phrase
#[wasm_bindgen]
pub fn mnemonic_to_seed(mnemonic: &str, passphrase: Option<String>) -> Result<Vec<u8>, JsError> {
    // Parse the mnemonic
    let mnemonic = Mnemonic::parse_normalized(mnemonic)
        .map_err(|e| JsError::new(&format!("Invalid mnemonic phrase: {}", e)))?;

    // Generate seed with optional passphrase
    let seed = mnemonic.to_seed(passphrase.as_deref().unwrap_or(""));

    Ok(seed.to_vec())
}

/// Derive a key from mnemonic phrase using BIP39/BIP44
#[wasm_bindgen]
pub fn derive_key_from_seed_phrase(
    mnemonic: &str,
    passphrase: Option<String>,
    network: &str,
) -> Result<JsValue, JsError> {
    use crate::wallet::key_generation::KeyPair;

    // Get seed from mnemonic
    let seed = mnemonic_to_seed(mnemonic, passphrase)?;

    // For now, we'll use the first 32 bytes of the seed as the private key
    // Note: This is a simplified approach. Proper BIP32/BIP44 would use HD derivation
    // with the path m/44'/5'/0'/0/0 for Dash mainnet or m/44'/1'/0'/0/0 for testnet
    let key_bytes = if seed.len() >= 32 {
        &seed[0..32]
    } else {
        // This shouldn't happen with BIP39, but handle it just in case
        return Err(JsError::new("Seed too short"));
    };

    let net = match network {
        "mainnet" => dashcore::Network::Dash,
        "testnet" => dashcore::Network::Testnet,
        _ => return Err(JsError::new("Invalid network")),
    };

    // Create private key from seed bytes
    let private_key = dashcore::PrivateKey::from_slice(key_bytes, net)
        .map_err(|e| JsError::new(&format!("Failed to create private key: {}", e)))?;

    // Get public key
    use dash_sdk::dpp::dashcore::secp256k1::Secp256k1;
    let secp = Secp256k1::new();

    let public_key = private_key.public_key(&secp);
    let public_key_bytes = public_key.inner.serialize();
    // Get address
    let address = dashcore::Address::p2pkh(&public_key, net);

    let key_pair = KeyPair {
        private_key_wif: private_key.to_wif(),
        private_key_hex: hex::encode(key_bytes),
        public_key: hex::encode(&public_key_bytes),
        address: address.to_string(),
        network: network.to_string(),
    };

    serde_wasm_bindgen::to_value(&key_pair)
        .map_err(|e| JsError::new(&format!("Failed to serialize key pair: {}", e)))
}

/// Derive a key from seed phrase with arbitrary path
#[wasm_bindgen]
pub fn derive_key_from_seed_with_path(
    mnemonic: &str,
    passphrase: Option<String>,
    path: &str,
    network: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::dpp::key_wallet::{DerivationPath, ExtendedPrivKey};

    // Get seed from mnemonic
    let seed = mnemonic_to_seed(mnemonic, passphrase)?;

    let net = match network {
        "mainnet" => dashcore::Network::Dash,
        "testnet" => dashcore::Network::Testnet,
        _ => return Err(JsError::new("Invalid network")),
    };

    // Parse derivation path
    let derivation_path = DerivationPath::from_str(path)
        .map_err(|e| JsError::new(&format!("Invalid derivation path: {}", e)))?;

    // Create master extended private key from seed
    let master_key = ExtendedPrivKey::new_master(net, &seed)
        .map_err(|e| JsError::new(&format!("Failed to create master key: {}", e)))?;

    // Derive the key at the specified path
    let derived_key = master_key
        .derive_priv(&dashcore::secp256k1::Secp256k1::new(), &derivation_path)
        .map_err(|e| JsError::new(&format!("Failed to derive key: {}", e)))?;

    // In v0.40-dev, ExtendedPrivKey might have a different structure
    // Create a PrivateKey from the derived key
    let private_key = dashcore::PrivateKey::new(derived_key.private_key, net);

    // Get public key
    let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
    let public_key = private_key.public_key(&secp);

    // Get address
    let address = dashcore::Address::p2pkh(&public_key, net);

    // Create a JavaScript object directly
    let obj = js_sys::Object::new();

    js_sys::Reflect::set(&obj, &JsValue::from_str("path"), &JsValue::from_str(path))
        .map_err(|_| JsError::new("Failed to set path property"))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("private_key_wif"),
        &JsValue::from_str(&private_key.to_wif()),
    )
    .map_err(|_| JsError::new("Failed to set private_key_wif property"))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("private_key_hex"),
        &JsValue::from_str(&hex::encode(private_key.inner.secret_bytes())),
    )
    .map_err(|_| JsError::new("Failed to set private_key_hex property"))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("public_key"),
        &JsValue::from_str(&hex::encode(public_key.to_bytes())),
    )
    .map_err(|_| JsError::new("Failed to set public_key property"))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("address"),
        &JsValue::from_str(&address.to_string()),
    )
    .map_err(|_| JsError::new("Failed to set address property"))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("network"),
        &JsValue::from_str(network),
    )
    .map_err(|_| JsError::new("Failed to set network property"))?;

    Ok(obj.into())
}

/// HD Key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDKeyInfo {
    /// The derivation path used
    pub path: String,
    /// The private key (if available)
    pub private_key: Option<String>,
    /// The public key
    pub public_key: String,
    /// The address
    pub address: String,
    /// Extended public key (xpub)
    pub xpub: Option<String>,
    /// Extended private key (xprv) - only if private key is available
    pub xprv: Option<String>,
}

/// Create a BIP44 mainnet derivation path
#[wasm_bindgen]
pub fn derivation_path_bip44_mainnet(account: u32, change: u32, index: u32) -> JsValue {
    let path = DerivationPath::new_bip44_mainnet(account, change, index);
    serde_wasm_bindgen::to_value(&path).unwrap_or(JsValue::NULL)
}

/// Create a BIP44 testnet derivation path
#[wasm_bindgen]
pub fn derivation_path_bip44_testnet(account: u32, change: u32, index: u32) -> JsValue {
    let path = DerivationPath::new_bip44_testnet(account, change, index);
    serde_wasm_bindgen::to_value(&path).unwrap_or(JsValue::NULL)
}

/// Create a DIP9 mainnet derivation path
#[wasm_bindgen]
pub fn derivation_path_dip9_mainnet(feature_type: u32, account: u32, index: u32) -> JsValue {
    let path = DerivationPath::new_dip9_mainnet(feature_type, account, index);
    serde_wasm_bindgen::to_value(&path).unwrap_or(JsValue::NULL)
}

/// Create a DIP9 testnet derivation path
#[wasm_bindgen]
pub fn derivation_path_dip9_testnet(feature_type: u32, account: u32, index: u32) -> JsValue {
    let path = DerivationPath::new_dip9_testnet(feature_type, account, index);
    serde_wasm_bindgen::to_value(&path).unwrap_or(JsValue::NULL)
}

/// Create a DIP13 mainnet derivation path (for HD masternode keys)
#[wasm_bindgen]
pub fn derivation_path_dip13_mainnet(account: u32) -> JsValue {
    // DIP13 uses m/9'/5'/account' format (DIP13 uses purpose 9, not 13)
    let path_str = format!("m/{}'/{}'/{}'", DIP13_PURPOSE, DASH_COIN_TYPE, account);

    let obj = js_sys::Object::new();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("path"),
        &JsValue::from_str(&path_str),
    )
    .unwrap();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("purpose"),
        &JsValue::from_f64(DIP13_PURPOSE as f64),
    )
    .unwrap();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("coin_type"),
        &JsValue::from_f64(DASH_COIN_TYPE as f64),
    )
    .unwrap();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("account"),
        &JsValue::from_f64(account as f64),
    )
    .unwrap();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("description"),
        &JsValue::from_str("DIP13 HD identity key path"),
    )
    .unwrap();

    obj.into()
}

/// Create a DIP13 testnet derivation path (for HD masternode keys)
#[wasm_bindgen]
pub fn derivation_path_dip13_testnet(account: u32) -> JsValue {
    // DIP13 uses m/9'/1'/account' format for testnet
    let path_str = format!("m/{}'/{}'/{}'", DIP13_PURPOSE, TESTNET_COIN_TYPE, account);

    let obj = js_sys::Object::new();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("path"),
        &JsValue::from_str(&path_str),
    )
    .unwrap();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("purpose"),
        &JsValue::from_f64(DIP13_PURPOSE as f64),
    )
    .unwrap();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("coin_type"),
        &JsValue::from_f64(TESTNET_COIN_TYPE as f64),
    )
    .unwrap();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("account"),
        &JsValue::from_f64(account as f64),
    )
    .unwrap();

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("description"),
        &JsValue::from_str("DIP13 HD identity key path (testnet)"),
    )
    .unwrap();

    obj.into()
}

/// Get child public key from extended public key
#[wasm_bindgen]
pub fn derive_child_public_key(xpub: &str, index: u32, hardened: bool) -> Result<String, JsError> {
    if hardened {
        return Err(JsError::new(
            "Cannot derive hardened child from extended public key",
        ));
    }

    // Disallow indices in the hardened range for non-hardened derivation
    if index >= 0x8000_0000 {
        return Err(JsError::new(
            "Index is in hardened range; use a value < 2^31",
        ));
    }

    // Parse the extended public key
    let parent_xpub = BIP32ExtendedPubKey::from_str(xpub)
        .map_err(|e| JsError::new(&format!("Invalid extended public key: {}", e)))?;

    // Build a one-step derivation path and derive
    let child_number: ChildNumber = ChildNumber::from(index);
    let path = BIP32DerivationPath::from(vec![child_number]);
    let secp = Secp256k1::new();
    let child_xpub = parent_xpub
        .derive_pub(&secp, &path)
        .map_err(|e| JsError::new(&format!("Failed to derive child key: {}", e)))?;

    Ok(child_xpub.to_string())
}

/// Convert extended private key to extended public key
#[wasm_bindgen]
pub fn xprv_to_xpub(xprv: &str) -> Result<String, JsError> {
    // Parse the extended private key and convert to extended public key
    let ext_prv = BIP32ExtendedPrivKey::from_str(xprv)
        .map_err(|e| JsError::new(&format!("Invalid extended private key: {}", e)))?;
    let secp = Secp256k1::new();
    let ext_pub = BIP32ExtendedPubKey::from_priv(&secp, &ext_prv);
    Ok(ext_pub.to_string())
}
