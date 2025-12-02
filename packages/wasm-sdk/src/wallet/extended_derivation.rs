//! Extended key derivation for DIP14/DIP15 support
//!
//! Implements 256-bit derivation paths for DashPay contact keys

use crate::error::WasmSdkError;
use crate::impl_wasm_object_json;
use crate::queries::utils::deserialize_required_query;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::dashcore;
use dash_sdk::dpp::dashcore::secp256k1::Secp256k1;
use dash_sdk::dpp::key_wallet::{bip32, DerivationPath, ExtendedPrivKey};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::debug;
use wasm_bindgen::prelude::*;
use wasm_dpp2::identifier::IdentifierWasm;

// TypeScript option bags (module scope) for extended derivation helpers
#[wasm_bindgen(typescript_custom_section)]
const DERIVE_FROM_EXTENDED_PATH_OPTS_TS: &'static str = r#"
export interface DeriveKeyFromSeedWithExtendedPathParams {
  mnemonic: string;
  passphrase?: string;
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
    sender_identity_id: IdentifierWasm,
    #[serde(rename = "receiverIdentityId")]
    receiver_identity_id: IdentifierWasm,
    account: u32,
    #[serde(rename = "addressIndex")]
    address_index: u32,
    network: String,
}

#[wasm_bindgen(typescript_custom_section)]
const DERIVE_DASHPAY_CONTACT_KEY_OPTS_TS: &'static str = r#"
export interface DeriveDashpayContactKeyParams {
  mnemonic: string;
  passphrase?: string;
  senderIdentityId: IdentifierLike;
  receiverIdentityId: IdentifierLike;
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

// Common internal result to avoid duplicating derivation logic/output wiring
struct CommonDerivation {
    path: String,
    private_key_wif: String,
    private_key_hex: String,
    public_key_hex: String,
    address: String,
    network: String,
    xprv: String,
    xpub: String,
}

fn derive_common_from_mnemonic(
    mnemonic: &str,
    passphrase: Option<String>,
    network: &str,
    path: &str,
) -> Result<CommonDerivation, WasmSdkError> {
    // Get seed from mnemonic
    let seed = WasmSdk::mnemonic_to_seed(mnemonic, passphrase)?;

    let net = match network {
        "mainnet" => dashcore::Network::Dash,
        "testnet" => dashcore::Network::Testnet,
        _ => return Err(WasmSdkError::invalid_argument("Invalid network")),
    };

    // Create master extended private key from seed
    let master_key = ExtendedPrivKey::new_master(net, &seed)
        .map_err(|e| WasmSdkError::generic(format!("Failed to create master key: {}", e)))?;

    // Parse path and derive
    let derivation_path = DerivationPath::from_str(path)
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid derivation path: {}", e)))?;

    let secp = Secp256k1::new();
    let derived_key = master_key
        .derive_priv(&secp, &derivation_path)
        .map_err(|e| WasmSdkError::generic(format!("Failed to derive key: {}", e)))?;

    let xpub = bip32::ExtendedPubKey::from_priv(&secp, &derived_key);
    let private_key = dashcore::PrivateKey::new(derived_key.private_key, net);
    let public_key = private_key.public_key(&secp);
    let address = dashcore::Address::p2pkh(&public_key, net);

    Ok(CommonDerivation {
        path: path.to_string(),
        private_key_wif: private_key.to_wif(),
        private_key_hex: hex::encode(private_key.inner.secret_bytes()),
        public_key_hex: hex::encode(public_key.to_bytes()),
        address: address.to_string(),
        network: network.to_string(),
        xprv: derived_key.to_string(),
        xpub: xpub.to_string(),
    })
}

#[wasm_bindgen(js_name = "DerivedKeyInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivedKeyInfoWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub path: String,
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
    #[wasm_bindgen(getter_with_clone)]
    pub xprv: String,
    #[wasm_bindgen(getter_with_clone)]
    pub xpub: String,
}

impl From<CommonDerivation> for DerivedKeyInfoWasm {
    fn from(c: CommonDerivation) -> Self {
        Self {
            path: c.path,
            private_key_wif: c.private_key_wif,
            private_key_hex: c.private_key_hex,
            public_key: c.public_key_hex,
            address: c.address,
            network: c.network,
            xprv: c.xprv,
            xpub: c.xpub,
        }
    }
}

// Field getters are generated via getter_with_clone annotations above
impl_wasm_object_json!(DerivedKeyInfoWasm);

#[wasm_bindgen(js_name = "DashpayContactKeyInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashpayContactKeyInfoWasm {
    // common
    #[wasm_bindgen(getter_with_clone)]
    pub path: String,
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
    #[wasm_bindgen(getter_with_clone)]
    pub xprv: String,
    #[wasm_bindgen(getter_with_clone)]
    pub xpub: String,
    // extras
    #[wasm_bindgen(getter_with_clone, js_name = "dipStandard")]
    pub dip_standard: String,
    #[wasm_bindgen(getter_with_clone)]
    pub purpose: String,
    #[wasm_bindgen(getter_with_clone, js_name = "senderIdentity")]
    pub sender_identity: String,
    #[wasm_bindgen(getter_with_clone, js_name = "receiverIdentity")]
    pub receiver_identity: String,
    #[wasm_bindgen(getter_with_clone)]
    pub account: u32,
    #[wasm_bindgen(getter_with_clone, js_name = "addressIndex")]
    pub address_index: u32,
}

impl DashpayContactKeyInfoWasm {
    fn from_common(
        c: CommonDerivation,
        sender_identity: String,
        receiver_identity: String,
        account: u32,
        address_index: u32,
    ) -> Self {
        Self {
            path: c.path,
            private_key_wif: c.private_key_wif,
            private_key_hex: c.private_key_hex,
            public_key: c.public_key_hex,
            address: c.address,
            network: c.network,
            xprv: c.xprv,
            xpub: c.xpub,
            dip_standard: "DIP15".to_string(),
            purpose: "DashPay Contact Payment".to_string(),
            sender_identity,
            receiver_identity,
            account,
            address_index,
        }
    }
}

// Field getters are generated via getter_with_clone annotations above
impl_wasm_object_json!(DashpayContactKeyInfoWasm);
#[wasm_bindgen]
impl WasmSdk {
    /// Derive a key from seed phrase with extended path supporting 256-bit indices
    /// This supports DIP14/DIP15 paths with identity IDs
    #[wasm_bindgen(js_name = "deriveKeyFromSeedWithExtendedPath")]
    pub fn derive_key_from_seed_with_extended_path(
        params: DeriveKeyFromSeedWithExtendedPathParamsJs,
    ) -> Result<DerivedKeyInfoWasm, WasmSdkError> {
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
        let common = derive_common_from_mnemonic(&mnemonic, passphrase, &network, &path)?;
        Ok(DerivedKeyInfoWasm::from(common))
    }

    /// Derive a DashPay contact key using DIP15 with full identity IDs
    #[wasm_bindgen(js_name = "deriveDashpayContactKey")]
    pub fn derive_dashpay_contact_key(
        params: DeriveDashpayContactKeyParamsJs,
    ) -> Result<DashpayContactKeyInfoWasm, WasmSdkError> {
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
        let sender_id_hex = {
            let id: dash_sdk::dpp::prelude::Identifier = sender_identity_id.into();
            hex::encode(id.as_bytes())
        };
        let receiver_id_hex = {
            let id: dash_sdk::dpp::prelude::Identifier = receiver_identity_id.into();
            hex::encode(id.as_bytes())
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
            "m/9'/{}'/{}'/{}'/0x{}/0x{}/{}",
            coin_type, 15, account, sender_id_hex, receiver_id_hex, address_index
        );
        debug!(target : "wasm_sdk", path = % path, "DashPay contact path");

        // Reuse common derivation logic
        let common = derive_common_from_mnemonic(&mnemonic, passphrase, &network, &path)?;
        Ok(DashpayContactKeyInfoWasm::from_common(
            common,
            sender_id_hex,
            receiver_id_hex,
            account,
            address_index,
        ))
    }
}
