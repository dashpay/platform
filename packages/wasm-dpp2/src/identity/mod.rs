use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::identity_public_key::IdentityPublicKeyWasm;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::{self, Identity, KeyID};
use dpp::platform_value::ReplacementType;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::IdentityPublicKey;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, ValueConvertible};
use dpp::version::PlatformVersion;
use serde_json::Value as JsonValue;
use serde_wasm_bindgen::to_value;
use std::collections::BTreeMap;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = "Identity")]
pub struct IdentityWasm(Identity);

impl From<Identity> for IdentityWasm {
    fn from(identity: Identity) -> Self {
        Self(identity)
    }
}

#[wasm_bindgen(js_class = Identity)]
impl IdentityWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "Identity".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "Identity".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(js_identifier: &JsValue) -> WasmDppResult<IdentityWasm> {
        let identifier = IdentifierWasm::try_from(js_identifier)?;

        let identity =
            Identity::create_basic_identity(identifier.into(), PlatformVersion::first())?;

        Ok(IdentityWasm(identity))
    }

    #[wasm_bindgen(setter = "id")]
    pub fn set_id(&mut self, js_identifier: &JsValue) -> WasmDppResult<()> {
        let identifier = IdentifierWasm::try_from(js_identifier)?;
        self.0.set_id(identifier.into());
        Ok(())
    }

    #[wasm_bindgen(setter = "balance")]
    pub fn set_balance(&mut self, balance: u64) {
        self.0.set_balance(balance);
    }

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: u64) {
        self.0.set_revision(revision);
    }

    #[wasm_bindgen(js_name = "addPublicKey")]
    pub fn add_public_key(&mut self, public_key: &IdentityPublicKeyWasm) {
        self.0.add_public_key(public_key.clone().into());
    }

    // GETTERS

    #[wasm_bindgen(getter = "id")]
    pub fn get_id(&self) -> IdentifierWasm {
        self.0.id().into()
    }

    #[wasm_bindgen(getter = "balance")]
    pub fn get_balance(&self) -> u64 {
        self.0.balance()
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn get_revision(&self) -> u64 {
        self.0.revision()
    }

    #[wasm_bindgen(js_name = "getPublicKeyById")]
    pub fn get_public_key_by_id(&self, key_id: KeyID) -> IdentityPublicKeyWasm {
        let identity_public_key = self.0.get_public_key_by_id(key_id);
        IdentityPublicKeyWasm::from(identity_public_key.unwrap().clone())
    }

    #[wasm_bindgen(js_name = "getPublicKeys")]
    pub fn get_public_keys(&self) -> Vec<IdentityPublicKeyWasm> {
        let keys = self
            .0
            .public_keys()
            .iter()
            .map(|(_index, key)| IdentityPublicKeyWasm::from(key.clone()))
            .collect();

        keys
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentityWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "base64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Base64))
    }

    fn cleaned_json_value(&self) -> WasmDppResult<JsonValue> {
        let mut value = self.0.to_object()?;

        value
            .replace_at_paths(
                identity::IDENTIFIER_FIELDS_RAW_OBJECT,
                ReplacementType::TextBase58,
            )
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        if let Some(public_keys) = value
            .get_optional_array_mut_ref(identity::property_names::PUBLIC_KEYS)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?
        {
            for key in public_keys.iter_mut() {
                key.replace_at_paths(
                    identity::identity_public_key::BINARY_DATA_FIELDS,
                    ReplacementType::TextBase64,
                )
                .map_err(|e| WasmDppError::serialization(e.to_string()))?;
            }
        }

        value
            .try_into_validating_json()
            .map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let json_value = self.cleaned_json_value()?;
        to_value(&json_value).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        self.to_object()
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityWasm> {
        let identity = Identity::deserialize_from_bytes(bytes.as_slice())?;
        Ok(IdentityWasm(identity))
    }
}

impl IdentityWasm {
    pub fn get_rs_public_keys(&self) -> BTreeMap<KeyID, IdentityPublicKey> {
        self.0.public_keys().clone()
    }
}
