use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::identity::public_key::IdentityPublicKeyWasm;
use crate::serialization;
use crate::utils::IntoWasm;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::{Identity, KeyID};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::Identifier;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, ValueConvertible};
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
use std::convert::TryFrom;
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

impl From<IdentityWasm> for Identity {
    fn from(wasm: IdentityWasm) -> Self {
        wasm.0
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
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_identifier: &JsValue,
    ) -> WasmDppResult<IdentityWasm> {
        let identifier: Identifier = IdentifierWasm::try_from(js_identifier)?.into();

        let identity = Identity::create_basic_identity(identifier, PlatformVersion::first())?;

        Ok(IdentityWasm(identity))
    }

    #[wasm_bindgen(setter = "id")]
    pub fn set_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_identifier: &JsValue,
    ) -> WasmDppResult<()> {
        let identifier: Identifier = IdentifierWasm::try_from(js_identifier)?.into();
        self.0.set_id(identifier);
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
    pub fn get_public_key_by_id(&self, key_id: KeyID) -> Option<IdentityPublicKeyWasm> {
        self.0
            .get_public_key_by_id(key_id)
            .map(|key| IdentityPublicKeyWasm::from(key.clone()))
    }

    #[wasm_bindgen(js_name = "getPublicKeys")]
    pub fn get_public_keys(&self) -> Vec<IdentityPublicKeyWasm> {
        self.0
            .public_keys()
            .values()
            .map(|key| IdentityPublicKeyWasm::from(key.clone()))
            .collect()
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

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        // Use platform_value conversion which handles BigInt for balance/revision
        // and outputs id as Uint8Array, publicKeys as plain objects
        let value = self.0.to_object()?;
        serialization::platform_value_to_object(&value)
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        serialization::to_json(&self.0)
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<IdentityWasm> {
        serialization::from_json(js_value).map(IdentityWasm)
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<IdentityWasm> {
        let value = serialization::js_value_to_platform_value(&js_value)?;
        let identity = Identity::try_from_platform_versioned(value, PlatformVersion::latest())?;
        Ok(IdentityWasm(identity))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityWasm> {
        let identity = Identity::deserialize_from_bytes(bytes.as_slice())?;
        Ok(IdentityWasm(identity))
    }
}

impl IdentityWasm {
    /// Try to extract an Identity from an options object field.
    ///
    /// This helper reads the specified field from an options object and converts it
    /// to an IdentityWasm.
    pub fn try_from_options(options: &JsValue, field_name: &str) -> WasmDppResult<Self> {
        let identity_js =
            js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
                WasmDppError::invalid_argument(format!("Missing '{}' field", field_name))
            })?;

        if identity_js.is_undefined() || identity_js.is_null() {
            return Err(WasmDppError::invalid_argument(format!(
                "'{}' is required",
                field_name
            )));
        }

        identity_js
            .to_wasm::<IdentityWasm>("Identity")
            .map(|boxed| (*boxed).clone())
    }
}
