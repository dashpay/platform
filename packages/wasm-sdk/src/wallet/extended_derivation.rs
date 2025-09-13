//! Extended key derivation for DIP14/DIP15 support
//!
//! Implements 256-bit derivation paths for DashPay contact keys

use crate::wallet::key_derivation::mnemonic_to_seed;
use dash_sdk::dpp::dashcore;
use dash_sdk::dpp::dashcore::secp256k1::Secp256k1;
use dash_sdk::dpp::key_wallet::{bip32, DerivationPath, ExtendedPrivKey};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use crate::error::new_structured_error;
use serde_json::json;
use web_sys;

/// Derive a key from seed phrase with extended path supporting 256-bit indices
/// This supports DIP14/DIP15 paths with identity IDs
#[wasm_bindgen]
pub fn derive_key_from_seed_with_extended_path(
    mnemonic: &str,
    passphrase: Option<String>,
    path: &str,
    network: &str,
) -> Result<JsValue, JsError> {
    // Debug: Log the path being processed
    web_sys::console::log_1(&format!("Processing extended path: {}", path).into());

    // Get seed from mnemonic
    let seed = mnemonic_to_seed(mnemonic, passphrase)?;

    let net = match network {
        "mainnet" => dashcore::Network::Dash,
        "testnet" => dashcore::Network::Testnet,
        _ => return Err(new_structured_error(
            "Invalid network",
            "E_INVALID_ARGUMENT",
            "argument",
            Some(json!({"field":"network","allowed":["mainnet","testnet"]})),
            Some(false),
        )),
    };

    // Create master extended private key from seed
    let master_key = ExtendedPrivKey::new_master(net, &seed)
        .map_err(|e| new_structured_error(&format!("Failed to create master key: {}", e), "E_INTERNAL", "internal", None, Some(false)))?;

    // Parse the derivation path using dashcore's built-in parser
    // This already supports 256-bit hex values like 0x775d3854...
    let derivation_path = DerivationPath::from_str(path)
        .map_err(|e| new_structured_error(&format!("Invalid derivation path: {}", e), "E_INVALID_ARGUMENT", "argument", Some(json!({"field":"path"})), Some(false)))?;

    // Use dashcore's built-in derive_priv method which handles DIP14
    let secp = Secp256k1::new();
    let derived_key = master_key
        .derive_priv(&secp, &derivation_path)
        .map_err(|e| new_structured_error(&format!("Failed to derive key: {}", e), "E_INTERNAL", "internal", None, Some(false)))?;

    // Get the extended public key
    let xpub = bip32::ExtendedPubKey::from_priv(&secp, &derived_key);

    // Get the private key
    let private_key = dashcore::PrivateKey::new(derived_key.private_key, net);

    // Get public key
    let public_key = private_key.public_key(&secp);

    // Get address
    let address = dashcore::Address::p2pkh(&public_key, net);

    // Create result object
    let obj = js_sys::Object::new();

    js_sys::Reflect::set(&obj, &JsValue::from_str("path"), &JsValue::from_str(path))
        .map_err(|_| new_structured_error("Failed to set path property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("private_key_wif"),
        &JsValue::from_str(&private_key.to_wif()),
    )
    .map_err(|_| new_structured_error("Failed to set private_key_wif property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("private_key_hex"),
        &JsValue::from_str(&hex::encode(private_key.inner.secret_bytes())),
    )
    .map_err(|_| new_structured_error("Failed to set private_key_hex property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("public_key"),
        &JsValue::from_str(&hex::encode(public_key.to_bytes())),
    )
    .map_err(|_| new_structured_error("Failed to set public_key property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("address"),
        &JsValue::from_str(&address.to_string()),
    )
    .map_err(|_| new_structured_error("Failed to set address property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("network"),
        &JsValue::from_str(network),
    )
    .map_err(|_| new_structured_error("Failed to set network property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("xprv"),
        &JsValue::from_str(&derived_key.to_string()),
    )
    .map_err(|_| new_structured_error("Failed to set xprv property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("xpub"),
        &JsValue::from_str(&xpub.to_string()),
    )
    .map_err(|_| new_structured_error("Failed to set xpub property", "E_INTERNAL", "internal", None, Some(false)))?;

    Ok(obj.into())
}

/// Derive a DashPay contact key using DIP15 with full identity IDs
#[wasm_bindgen]
pub fn derive_dashpay_contact_key(
    mnemonic: &str,
    passphrase: Option<String>,
    sender_identity_id: &str,
    receiver_identity_id: &str,
    account: u32,
    address_index: u32,
    network: &str,
) -> Result<JsValue, JsError> {
    use bs58;

    // Convert base58 identity IDs to hex format if needed
    let sender_id_formatted = if sender_identity_id.starts_with("0x") {
        sender_identity_id.to_string()
    } else {
        // Decode base58 to bytes, then convert to hex
        let bytes = bs58::decode(sender_identity_id)
            .into_vec()
            .map_err(|e| new_structured_error(&format!("Invalid sender identity ID: {}", e), "E_INVALID_ARGUMENT", "argument", Some(json!({"field":"senderIdentityId"})), Some(false)))?;
        format!("0x{}", hex::encode(bytes))
    };

    let receiver_id_formatted = if receiver_identity_id.starts_with("0x") {
        receiver_identity_id.to_string()
    } else {
        // Decode base58 to bytes, then convert to hex
        let bytes = bs58::decode(receiver_identity_id)
            .into_vec()
            .map_err(|e| new_structured_error(&format!("Invalid receiver identity ID: {}", e), "E_INVALID_ARGUMENT", "argument", Some(json!({"field":"receiverIdentityId"})), Some(false)))?;
        format!("0x{}", hex::encode(bytes))
    };

    // Build the DIP15 path
    // m / 9' / coin_type' / 15' / account' / sender_id / receiver_id / index
    let coin_type = match network {
        "mainnet" => 5,
        "testnet" => 1,
        _ => return Err(new_structured_error("Invalid network", "E_INVALID_ARGUMENT", "argument", Some(json!({"field":"network","allowed":["mainnet","testnet"]})), Some(false))),
    };

    let path = format!(
        "m/9'/{}'/{}'/{}'/{}/{}/{}",
        coin_type,
        15, // DIP15 feature
        account,
        sender_id_formatted,
        receiver_id_formatted,
        address_index
    );

    web_sys::console::log_1(&format!("DashPay contact path: {}", path).into());

    // Use the extended derivation function
    let result = derive_key_from_seed_with_extended_path(mnemonic, passphrase, &path, network)?;

    // Add DIP15-specific metadata
    let obj = result
        .dyn_into::<js_sys::Object>()
        .map_err(|_| new_structured_error("Failed to cast result to object", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("dipStandard"),
        &JsValue::from_str("DIP15"),
    )
    .map_err(|_| new_structured_error("Failed to set dipStandard property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("purpose"),
        &JsValue::from_str("DashPay Contact Payment"),
    )
    .map_err(|_| new_structured_error("Failed to set purpose property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("senderIdentity"),
        &JsValue::from_str(sender_identity_id),
    )
    .map_err(|_| new_structured_error("Failed to set senderIdentity property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("receiverIdentity"),
        &JsValue::from_str(receiver_identity_id),
    )
    .map_err(|_| new_structured_error("Failed to set receiverIdentity property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("account"),
        &JsValue::from_f64(account as f64),
    )
    .map_err(|_| new_structured_error("Failed to set account property", "E_INTERNAL", "internal", None, Some(false)))?;

    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("addressIndex"),
        &JsValue::from_f64(address_index as f64),
    )
    .map_err(|_| new_structured_error("Failed to set addressIndex property", "E_INTERNAL", "internal", None, Some(false)))?;

    Ok(obj.into())
}
