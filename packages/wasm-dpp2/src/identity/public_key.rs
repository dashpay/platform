use crate::core::network::NetworkLikeJs;
use crate::data_contract::contract_bounds::ContractBoundsWasm;
use crate::enums::keys::key_type::{KeyTypeLikeJs, KeyTypeWasm};
use crate::enums::keys::purpose::{PurposeLikeJs, PurposeWasm};
use crate::enums::keys::security_level::{SecurityLevelLikeJs, SecurityLevelWasm};
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_try_from_options;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::utils::{IntoWasm, get_required_property};
use dpp::dashcore::Network;
use dpp::dashcore::secp256k1::hashes::hex::{Case, DisplayHex};
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::identity::identity_public_key::conversion::json::IdentityPublicKeyJsonConversionMethodsV0;
use dpp::identity::identity_public_key::conversion::platform_value::IdentityPublicKeyPlatformValueConversionMethodsV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel, TimestampMillis};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::version::PlatformVersion;
use hex;
use js_sys::{Object, Reflect};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use serde_wasm_bindgen::from_value as serde_from_value;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityPublicKeyOptions {
    key_id: u32,
    #[serde(default)]
    is_read_only: bool,
    data: String,
    #[serde(default)]
    disabled_at: Option<TimestampMillis>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
export type PurposeLike = Purpose | string | number;
export type SecurityLevelLike = SecurityLevel | string | number;
export type KeyTypeLike = KeyType | string | number;

export interface IdentityPublicKeyOptions {
    keyId: number;
    purpose: PurposeLike;
    securityLevel: SecurityLevelLike;
    keyType: KeyTypeLike;
    isReadOnly?: boolean;
    data: string; // hex encoded
    disabledAt?: number;
    contractBounds?: ContractBounds;
}

/**
 * IdentityPublicKey serialized as a plain object.
 */
export interface IdentityPublicKeyObject {
    $version: string;
    id: number;
    purpose: number;
    securityLevel: number;
    contractBounds?: ContractBounds;
    type: number;
    readOnly: boolean;
    data: Uint8Array;
    disabledAt?: number;
}

/**
 * IdentityPublicKey serialized as JSON.
 */
export interface IdentityPublicKeyJSON {
    $version: string;
    id: number;
    purpose: number;
    securityLevel: number;
    contractBounds?: object;
    type: number;
    readOnly: boolean;
    data: string;
    disabledAt?: number;
}

/**
 * PublicKeyHash input type - accepts hex string or Uint8Array (20 bytes for HASH160)
 */
export type PublicKeyHashLike = string | Uint8Array;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityPublicKeyOptions")]
    pub type IdentityPublicKeyOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityPublicKeyObject")]
    pub type IdentityPublicKeyObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityPublicKeyJSON")]
    pub type IdentityPublicKeyJSONJs;

    #[wasm_bindgen(typescript_type = "PublicKeyHashLike")]
    pub type PublicKeyHashLikeJs;
}

/// Convert PublicKeyHashLikeJs to Vec<u8>
/// Accepts hex string or Uint8Array
pub fn public_key_hash_from_js(value: PublicKeyHashLikeJs) -> WasmDppResult<Vec<u8>> {
    let js_value: JsValue = value.into();

    if let Some(hex_str) = js_value.as_string() {
        hex::decode(&hex_str).map_err(|e| {
            WasmDppError::invalid_argument(format!("Invalid hex string for public key hash: {}", e))
        })
    } else {
        // Try to convert as Uint8Array
        let array = js_sys::Uint8Array::new(&js_value);
        Ok(array.to_vec())
    }
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "IdentityPublicKey")]
pub struct IdentityPublicKeyWasm(IdentityPublicKey);

impl From<IdentityPublicKey> for IdentityPublicKeyWasm {
    fn from(value: IdentityPublicKey) -> Self {
        IdentityPublicKeyWasm(value)
    }
}

impl From<IdentityPublicKeyWasm> for IdentityPublicKey {
    fn from(value: IdentityPublicKeyWasm) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_class = IdentityPublicKey)]
impl IdentityPublicKeyWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: IdentityPublicKeyOptionsJs) -> WasmDppResult<Self> {
        let options: JsValue = options.into();
        let object = Object::from(options.clone());

        // Extract purpose (required, complex type)
        let js_purpose = get_required_property(&object, "purpose")?;
        let purpose = PurposeWasm::try_from(js_purpose)?;

        // Extract securityLevel (required, complex type)
        let js_security_level = get_required_property(&object, "securityLevel")?;
        let security_level = SecurityLevelWasm::try_from(js_security_level)?;

        // Extract keyType (required, complex type)
        let js_key_type = get_required_property(&object, "keyType")?;
        let key_type = KeyTypeWasm::try_from(js_key_type)?;

        // Extract contractBounds (optional)
        let js_contract_bounds = Reflect::get(&object, &JsValue::from_str("contractBounds"))
            .unwrap_or(JsValue::UNDEFINED);
        let contract_bounds: Option<ContractBounds> =
            match js_contract_bounds.is_undefined() | js_contract_bounds.is_null() {
                true => None,
                false => Some(
                    js_contract_bounds
                        .to_wasm::<ContractBoundsWasm>("ContractBounds")?
                        .clone()
                        .into(),
                ),
            };

        // Extract simple fields via serde
        let opts: IdentityPublicKeyOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(IdentityPublicKeyWasm(IdentityPublicKey::from(
            IdentityPublicKeyV0 {
                id: opts.key_id,
                purpose: Purpose::from(purpose),
                security_level: SecurityLevel::from(security_level),
                contract_bounds,
                key_type: KeyType::from(key_type),
                read_only: opts.is_read_only,
                data: BinaryData::from_string(&opts.data, Hex)
                    .map_err(|e| WasmDppError::serialization(e.to_string()))?,
                disabled_at: opts.disabled_at,
            },
        )))
    }
}

#[wasm_bindgen(js_class = IdentityPublicKey)]
impl IdentityPublicKeyWasm {
    #[wasm_bindgen(js_name = "validatePrivateKey")]
    pub fn validate_private_key(
        &self,
        private_key_bytes_input: Vec<u8>,
        network: NetworkLikeJs,
    ) -> WasmDppResult<bool> {
        if private_key_bytes_input.len() != 32 {
            return Err(WasmDppError::invalid_argument(format!(
                "Private key must be exactly 32 bytes, got {}",
                private_key_bytes_input.len()
            )));
        }
        let mut private_key_bytes = [0u8; 32];
        private_key_bytes.copy_from_slice(&private_key_bytes_input);

        let network: Network = network.try_into()?;

        let is_valid = self
            .0
            .validate_private_key_bytes(&private_key_bytes, network)?;

        Ok(is_valid)
    }

    #[wasm_bindgen(getter = "contractBounds")]
    pub fn contract_bounds(&self) -> Option<ContractBoundsWasm> {
        self.0
            .contract_bounds()
            .map(|b| ContractBoundsWasm::from(b.clone()))
    }

    #[wasm_bindgen(getter = keyId)]
    pub fn get_key_id(&self) -> u32 {
        self.0.id()
    }

    #[wasm_bindgen(getter = purpose)]
    pub fn get_purpose(&self) -> String {
        PurposeWasm::from(self.0.purpose()).into()
    }

    #[wasm_bindgen(getter = purposeNumber)]
    pub fn get_purpose_number(&self) -> PurposeWasm {
        PurposeWasm::from(self.0.purpose())
    }

    #[wasm_bindgen(getter = securityLevel)]
    pub fn get_security_level(&self) -> String {
        SecurityLevelWasm::from(self.0.security_level()).into()
    }

    #[wasm_bindgen(getter = securityLevelNumber)]
    pub fn get_security_level_number(&self) -> SecurityLevelWasm {
        SecurityLevelWasm::from(self.0.security_level())
    }

    #[wasm_bindgen(getter = keyType)]
    pub fn get_key_type(&self) -> String {
        KeyTypeWasm::from(self.0.key_type()).into()
    }

    #[wasm_bindgen(getter = keyTypeNumber)]
    pub fn get_key_type_number(&self) -> KeyTypeWasm {
        KeyTypeWasm::from(self.0.key_type())
    }

    #[wasm_bindgen(getter = "isReadOnly")]
    pub fn is_read_only(&self) -> bool {
        self.0.read_only()
    }

    #[wasm_bindgen(getter = data)]
    pub fn get_data(&self) -> String {
        self.0.data().to_string(Hex)
    }

    #[wasm_bindgen(getter = disabledAt)]
    pub fn get_disabled_at(&self) -> Option<u64> {
        self.0.disabled_at()
    }

    #[wasm_bindgen(setter = keyId)]
    pub fn set_key_id(&mut self, key_id: u32) {
        self.0.set_id(key_id)
    }

    #[wasm_bindgen(setter = purpose)]
    pub fn set_purpose(&mut self, purpose: PurposeLikeJs) -> WasmDppResult<()> {
        let purpose: Purpose = purpose.try_into()?;
        self.0.set_purpose(purpose);
        Ok(())
    }

    #[wasm_bindgen(setter = securityLevel)]
    pub fn set_security_level(&mut self, security_level: SecurityLevelLikeJs) -> WasmDppResult<()> {
        let security_level: SecurityLevel = security_level.try_into()?;
        self.0.set_security_level(security_level);
        Ok(())
    }

    #[wasm_bindgen(setter = keyType)]
    pub fn set_key_type(&mut self, key_type: KeyTypeLikeJs) -> WasmDppResult<()> {
        let key_type: KeyType = key_type.try_into()?;
        self.0.set_key_type(key_type);
        Ok(())
    }

    #[wasm_bindgen(setter = "isReadOnly")]
    pub fn set_is_read_only(&mut self, is_read_only: bool) {
        self.0.set_read_only(is_read_only)
    }

    #[wasm_bindgen(setter = data)]
    pub fn set_data(&mut self, binary_data: &str) -> WasmDppResult<()> {
        let data = BinaryData::from_string(binary_data, Hex)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        self.0.set_data(data);
        Ok(())
    }

    #[wasm_bindgen(setter = disabledAt)]
    pub fn set_disabled_at(&mut self, disabled_at: u64) {
        self.0.set_disabled_at(disabled_at)
    }

    #[wasm_bindgen(js_name = "getPublicKeyHash")]
    pub fn public_key_hash(&self) -> WasmDppResult<String> {
        let hash = self
            .0
            .public_key_hash()?
            .to_vec()
            .to_hex_string(Case::Lower);

        Ok(hash)
    }

    #[wasm_bindgen(getter = "isMaster")]
    pub fn is_master(&self) -> bool {
        self.0.is_master()
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "hex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        Ok(encode(self.0.serialize_to_bytes()?.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "base64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        Ok(encode(self.0.serialize_to_bytes()?.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityPublicKeyWasm> {
        let public_key = IdentityPublicKey::deserialize_from_bytes(bytes.as_slice())?;

        Ok(IdentityPublicKeyWasm(public_key))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityPublicKeyWasm> {
        let bytes =
            decode(&hex, Hex).map_err(|err| WasmDppError::serialization(err.to_string()))?;

        let public_key = IdentityPublicKey::deserialize_from_bytes(bytes.as_slice())?;

        Ok(IdentityPublicKeyWasm(public_key))
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(hex: String) -> WasmDppResult<IdentityPublicKeyWasm> {
        let bytes =
            decode(&hex, Base64).map_err(|err| WasmDppError::serialization(err.to_string()))?;

        let public_key = IdentityPublicKey::deserialize_from_bytes(bytes.as_slice())?;

        Ok(IdentityPublicKeyWasm(public_key))
    }

    /// Serialize to JS object (non-human-readable).
    ///
    /// Uses platform_value conversion which properly handles the tagged enum
    /// and removes None fields like disabledAt.
    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<IdentityPublicKeyObjectJs> {
        let value = self.0.to_cleaned_object().map_err(WasmDppError::from)?;
        let js_value = serialization::platform_value_to_object(&value)?;
        Ok(js_value.into())
    }

    /// Deserialize from JS object (non-human-readable).
    ///
    /// Uses platform_value conversion which properly handles the tagged enum.
    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(value: IdentityPublicKeyObjectJs) -> WasmDppResult<IdentityPublicKeyWasm> {
        let platform_value = serialization::platform_value_from_object(value.into())?;
        let platform_version = PlatformVersion::latest();
        let key = IdentityPublicKey::from_object(platform_value, platform_version)
            .map_err(WasmDppError::from)?;
        Ok(IdentityPublicKeyWasm(key))
    }

    /// Serialize to JSON-compatible JS object (human-readable).
    ///
    /// Uses serde_json conversion which properly handles the tagged enum
    /// and serializes binary data as base64 strings.
    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<IdentityPublicKeyJSONJs> {
        let json_value = self.0.to_json_object().map_err(WasmDppError::from)?;
        let js_value = serialization::to_json(&json_value)?;
        Ok(js_value.into())
    }

    /// Deserialize from JSON-compatible JS object (human-readable).
    ///
    /// Uses serde_json conversion which properly handles the tagged enum
    /// and deserializes base64 strings to binary data.
    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(value: IdentityPublicKeyJSONJs) -> WasmDppResult<IdentityPublicKeyWasm> {
        let json_value: JsonValue = serde_from_value(value.into()).map_err(|err| {
            WasmDppError::serialization(format!(
                "IdentityPublicKey.fromJSON: unable to parse JSON: {}",
                err
            ))
        })?;

        let platform_version = PlatformVersion::latest();
        let key = IdentityPublicKey::from_json_object(json_value, platform_version)
            .map_err(WasmDppError::from)?;

        Ok(IdentityPublicKeyWasm(key))
    }
}

impl_try_from_js_value!(IdentityPublicKeyWasm, "IdentityPublicKey");
impl_try_from_options!(IdentityPublicKeyWasm);
impl_wasm_type_info!(IdentityPublicKeyWasm, IdentityPublicKey);
