//! Extended key derivation for DIP14/DIP15 support
//!
//! Implements 256-bit derivation paths for DashPay contact keys

use crate::error::WasmSdkError;
use crate::queries::utils::deserialize_required_query;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::dashcore;
use dash_sdk::dpp::dashcore::secp256k1::Secp256k1;
use dash_sdk::dpp::key_wallet::{bip32, DerivationPath, ExtendedPrivKey};
use std::str::FromStr;
use tracing::debug;
use wasm_bindgen::prelude::*;

// TypeScript option bags (module scope) for extended derivation helpers
#[wasm_bindgen(typescript_custom_section)]
const DERIVE_FROM_EXTENDED_PATH_OPTS_TS: &'static str = r#"
export interface DeriveKeyFromSeedWithExtendedPathParams {
  mnemonic: string;
  passphrase?: string | null;
  path: string;
  network: string;
}
"#;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DeriveKeyFromSeedWithExtendedPathParams")]
    pub type DeriveKeyFromSeedWithExtendedPathParamsJs;
}

// Inputs parsed from options (module scope)
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeriveFromExtendedPathInput {
    mnemonic: String,
    #[serde(default)]
    passphrase: Option<String>,
    path: String,
    network: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeriveDashpayContactKeyInput {
    mnemonic: String,
    #[serde(default)]
    passphrase: Option<String>,
    #[serde(rename = "senderIdentityId")]
    sender_identity_id: String,
    #[serde(rename = "receiverIdentityId")]
    receiver_identity_id: String,
    account: u32,
    #[serde(rename = "addressIndex")]
    address_index: u32,
    network: String,
}

#[wasm_bindgen(typescript_custom_section)]
const DERIVE_DASHPAY_CONTACT_KEY_OPTS_TS: &'static str = r#"
export interface DeriveDashpayContactKeyParams {
  mnemonic: string;
  passphrase?: string | null;
  senderIdentityId: string;
  receiverIdentityId: string;
  account: number;
  addressIndex: number;
  network: string;
}
"#;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DeriveDashpayContactKeyParams")]
    pub type DeriveDashpayContactKeyParamsJs;
}
#[wasm_bindgen]
impl WasmSdk {
    /// Derive a key from seed phrase with extended path supporting 256-bit indices
    /// This supports DIP14/DIP15 paths with identity IDs
    #[wasm_bindgen(js_name = "deriveKeyFromSeedWithExtendedPath")]
    pub fn derive_key_from_seed_with_extended_path(
        #[wasm_bindgen(unchecked_param_type = "DeriveKeyFromSeedWithExtendedPathParams")]
        params: JsValue,
    ) -> Result<JsValue, WasmSdkError> {
        let DeriveFromExtendedPathInput {
            mnemonic,
            passphrase,
            path,
            network,
        } = deserialize_required_query(
            params,
            "Options object is required",
            "deriveKeyFromSeedWithExtendedPath options",
        )?;
        // Debug: Log the path being processed
        debug!(target: "wasm_sdk", path, "Processing extended path");

        // Get seed from mnemonic
        let seed = Self::mnemonic_to_seed(&mnemonic, passphrase)?;

        let net = match network.as_str() {
            "mainnet" => dashcore::Network::Dash,
            "testnet" => dashcore::Network::Testnet,
            _ => return Err(WasmSdkError::invalid_argument("Invalid network")),
        };

        // Create master extended private key from seed
        let master_key = ExtendedPrivKey::new_master(net, &seed)
            .map_err(|e| WasmSdkError::generic(format!("Failed to create master key: {}", e)))?;

        // Parse the derivation path using dashcore's built-in parser
        // This already supports 256-bit hex values like 0x775d3854...
        let derivation_path = DerivationPath::from_str(&path).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid derivation path: {}", e))
        })?;

        // Use dashcore's built-in derive_priv method which handles DIP14
        let secp = Secp256k1::new();
        let derived_key = master_key
            .derive_priv(&secp, &derivation_path)
            .map_err(|e| WasmSdkError::generic(format!("Failed to derive key: {}", e)))?;

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

        js_sys::Reflect::set(&obj, &JsValue::from_str("path"), &JsValue::from_str(&path))
            .map_err(|_| WasmSdkError::generic("Failed to set path property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("private_key_wif"),
            &JsValue::from_str(&private_key.to_wif()),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set private_key_wif property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("private_key_hex"),
            &JsValue::from_str(&hex::encode(private_key.inner.secret_bytes())),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set private_key_hex property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("public_key"),
            &JsValue::from_str(&hex::encode(public_key.to_bytes())),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set public_key property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("address"),
            &JsValue::from_str(&address.to_string()),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set address property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("network"),
            &JsValue::from_str(&network),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set network property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("xprv"),
            &JsValue::from_str(&derived_key.to_string()),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set xprv property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("xpub"),
            &JsValue::from_str(&xpub.to_string()),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set xpub property"))?;

        Ok(obj.into())
    }

    /// Derive a DashPay contact key using DIP15 with full identity IDs
    // TODO: Implement typed response object
    #[wasm_bindgen(js_name = "deriveDashpayContactKey")]
    pub fn derive_dashpay_contact_key(
        #[wasm_bindgen(unchecked_param_type = "DeriveDashpayContactKeyParams")] params: JsValue,
    ) -> Result<JsValue, WasmSdkError> {
        let DeriveDashpayContactKeyInput {
            mnemonic,
            passphrase,
            sender_identity_id,
            receiver_identity_id,
            account,
            address_index,
            network,
        } = deserialize_required_query(
            params,
            "Options object is required",
            "deriveDashpayContactKey options",
        )?;
        use bs58;

        // Convert base58 identity IDs to hex format if needed
        let sender_id_formatted = if sender_identity_id.starts_with("0x") {
            sender_identity_id.to_string()
        } else {
            // Decode base58 to bytes, then convert to hex
            let bytes = bs58::decode(&sender_identity_id).into_vec().map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid sender identity ID: {}", e))
            })?;
            format!("0x{}", hex::encode(bytes))
        };

        let receiver_id_formatted = if receiver_identity_id.starts_with("0x") {
            receiver_identity_id.to_string()
        } else {
            // Decode base58 to bytes, then convert to hex
            let bytes = bs58::decode(&receiver_identity_id)
                .into_vec()
                .map_err(|e| {
                    WasmSdkError::invalid_argument(format!("Invalid receiver identity ID: {}", e))
                })?;
            format!("0x{}", hex::encode(bytes))
        };

        // Build the DIP15 path
        // m / 9' / coin_type' / 15' / account' / sender_id / receiver_id / index
        let coin_type = match network.as_str() {
            "mainnet" => 5,
            "testnet" => 1,
            _ => return Err(WasmSdkError::invalid_argument("Invalid network")),
        };
        // Build the DIP15 path
        // m / 9' / coin_type' / 15' / account' / sender_id / receiver_id / index
        let path = format!(
            "m/9'/{}'/{}'/{}'/{}/{}/{}",
            coin_type, 15, account, sender_id_formatted, receiver_id_formatted, address_index
        );
        debug!(target : "wasm_sdk", path = % path, "DashPay contact path");

        // Perform extended derivation directly to avoid re-deserialization
        let seed = Self::mnemonic_to_seed(&mnemonic, passphrase)?;

        let net = match network.as_str() {
            "mainnet" => dashcore::Network::Dash,
            "testnet" => dashcore::Network::Testnet,
            _ => return Err(WasmSdkError::invalid_argument("Invalid network")),
        };

        let master_key = ExtendedPrivKey::new_master(net, &seed)
            .map_err(|e| WasmSdkError::generic(format!("Failed to create master key: {}", e)))?;

        let derivation_path = DerivationPath::from_str(&path).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid derivation path: {}", e))
        })?;

        let secp = Secp256k1::new();
        let derived_key = master_key
            .derive_priv(&secp, &derivation_path)
            .map_err(|e| WasmSdkError::generic(format!("Failed to derive key: {}", e)))?;

        let xpub = bip32::ExtendedPubKey::from_priv(&secp, &derived_key);
        let private_key = dashcore::PrivateKey::new(derived_key.private_key, net);
        let public_key = private_key.public_key(&secp);
        let address = dashcore::Address::p2pkh(&public_key, net);

        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &JsValue::from_str("path"), &JsValue::from_str(&path))
            .map_err(|_| WasmSdkError::generic("Failed to set path property"))?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("private_key_wif"),
            &JsValue::from_str(&private_key.to_wif()),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set private_key_wif property"))?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("private_key_hex"),
            &JsValue::from_str(&hex::encode(private_key.inner.secret_bytes())),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set private_key_hex property"))?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("public_key"),
            &JsValue::from_str(&hex::encode(public_key.to_bytes())),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set public_key property"))?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("address"),
            &JsValue::from_str(&address.to_string()),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set address property"))?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("network"),
            &JsValue::from_str(&network),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set network property"))?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("xprv"),
            &JsValue::from_str(&derived_key.to_string()),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set xprv property"))?;
        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("xpub"),
            &JsValue::from_str(&xpub.to_string()),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set xpub property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("dipStandard"),
            &JsValue::from_str("DIP15"),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set dipStandard property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("purpose"),
            &JsValue::from_str("DashPay Contact Payment"),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set purpose property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("senderIdentity"),
            &JsValue::from_str(&sender_identity_id),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set senderIdentity property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("receiverIdentity"),
            &JsValue::from_str(&receiver_identity_id),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set receiverIdentity property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("account"),
            &JsValue::from_f64(account as f64),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set account property"))?;

        js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("addressIndex"),
            &JsValue::from_f64(address_index as f64),
        )
        .map_err(|_| WasmSdkError::generic("Failed to set addressIndex property"))?;

        Ok(obj.into())
    }
}
