use crate::error::{WasmDppError, WasmDppResult};
use crate::utils::IntoWasm;
use dpp::platform_value::string_encoding::Encoding::{Base58, Base64, Hex};
use dpp::platform_value::string_encoding::decode;
use dpp::prelude::Identifier;
use serde::de::{self, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value as JsonValue;
use std::fmt;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[derive(Copy, Clone)]
#[wasm_bindgen(js_name = "Identifier")]
pub struct IdentifierWasm(Identifier);

impl From<IdentifierWasm> for Identifier {
    fn from(identifier: IdentifierWasm) -> Self {
        identifier.0
    }
}

impl From<Identifier> for IdentifierWasm {
    fn from(identifier: Identifier) -> Self {
        IdentifierWasm(identifier)
    }
}

impl From<[u8; 32]> for IdentifierWasm {
    fn from(identifier: [u8; 32]) -> Self {
        IdentifierWasm(Identifier::new(identifier))
    }
}

impl From<&IdentifierWasm> for Identifier {
    fn from(identifier: &IdentifierWasm) -> Self {
        (*identifier).into()
    }
}

impl TryFrom<&[u8]> for IdentifierWasm {
    type Error = WasmDppError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 32 {
            return Err(WasmDppError::invalid_argument(
                "Identifier must be 32 bytes length",
            ));
        }

        let norm_slice: [u8; 32] = value
            .try_into()
            .map_err(|_| WasmDppError::invalid_argument("Cannot parse identifier"))?;

        Ok(IdentifierWasm(Identifier::new(norm_slice)))
    }
}

impl TryFrom<JsValue> for IdentifierWasm {
    type Error = WasmDppError;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        if value.is_undefined() || value.is_null() {
            return Err(WasmDppError::invalid_argument(
                "the identifier cannot be null or undefined",
            ));
        }

        if let Ok(existing) = value.clone().to_wasm::<IdentifierWasm>("Identifier") {
            return Ok(*existing);
        }

        if let Some(string) = value.as_string() {
            return IdentifierWasm::try_from_str(&string);
        }

        if value.is_instance_of::<js_sys::Uint8Array>() || value.is_array() || value.is_object() {
            let uint8_array = Uint8Array::from(value.clone());
            let bytes = uint8_array.to_vec();

            return Identifier::from_bytes(&bytes)
                .map(Into::into)
                .map_err(|err| WasmDppError::invalid_argument(err.to_string()));
        }

        Err(WasmDppError::invalid_argument(
            "Invalid identifier. Expected Identifier, Uint8Array or string",
        ))
    }
}

impl TryFrom<&JsValue> for IdentifierWasm {
    type Error = WasmDppError;
    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        IdentifierWasm::try_from(value.clone())
    }
}

struct IdentifierWasmVisitor;

impl<'de> Visitor<'de> for IdentifierWasmVisitor {
    type Value = IdentifierWasm;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Identifier or compatible representation")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        IdentifierWasm::try_from_str(value).map_err(|err| E::custom(err.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&value)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut bytes: Vec<u8> = Vec::new();
        while let Some(byte) = seq.next_element::<u8>()? {
            bytes.push(byte);
        }
        Identifier::from_vec(bytes)
            .map(IdentifierWasm)
            .map_err(|err| A::Error::custom(err.to_string()))
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Identifier::from_vec(value.to_vec())
            .map(IdentifierWasm)
            .map_err(|err| E::custom(err.to_string()))
    }

    fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let value = JsonValue::deserialize(de::value::MapAccessDeserializer::new(map))
            .map_err(M::Error::custom)?;
        let js_value = serde_wasm_bindgen::to_value(&value).map_err(M::Error::custom)?;
        IdentifierWasm::try_from(&js_value).map_err(|err| M::Error::custom(err.to_string()))
    }
}

impl<'de> Deserialize<'de> for IdentifierWasm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(IdentifierWasmVisitor)
    }
}

#[wasm_bindgen(js_class = Identifier)]
impl IdentifierWasm {
    fn try_from_str(value: &str) -> Result<Self, WasmDppError> {
        if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()) {
            let bytes = decode(value, Hex)
                .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;
            return IdentifierWasm::try_from(bytes.as_slice());
        }

        Identifier::from_string(value, Base58)
            .map(Into::into)
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))
    }

    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "Identifier".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "Identifier".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_identifier: &JsValue,
    ) -> WasmDppResult<IdentifierWasm> {
        IdentifierWasm::try_from(js_identifier)
    }

    #[wasm_bindgen(js_name = "base58")]
    pub fn get_base58(&self) -> String {
        self.0.to_string(Base58)
    }

    #[wasm_bindgen(js_name = "base64")]
    pub fn get_base64(&self) -> String {
        self.0.to_string(Base64)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> String {
        self.0.to_string(Hex)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    #[wasm_bindgen(js_name = "fromBase58")]
    pub fn from_base58(base58: String) -> WasmDppResult<IdentifierWasm> {
        let identitfier = Identifier::from_string(base58.as_str(), Base58)
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        Ok(IdentifierWasm(identitfier))
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentifierWasm> {
        let identitfier = Identifier::from_string(base64.as_str(), Base64)
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        Ok(IdentifierWasm(identitfier))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentifierWasm> {
        let identitfier = Identifier::from_string(hex.as_str(), Hex)
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        Ok(IdentifierWasm(identitfier))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentifierWasm> {
        let identifier = Identifier::from_vec(bytes)
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;

        Ok(IdentifierWasm(identifier))
    }
}

impl IdentifierWasm {
    pub fn to_slice(&self) -> [u8; 32] {
        *self.0.as_bytes()
    }
}
