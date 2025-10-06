use crate::utils::{IntoWasm, get_class_type, identifier_from_js_value};
use dpp::platform_value::string_encoding::Encoding::{Base58, Base64, Hex};
use dpp::platform_value::string_encoding::decode;
use dpp::prelude::Identifier;
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
        identifier.clone().into()
    }
}

impl TryFrom<&[u8]> for IdentifierWasm {
    type Error = JsValue;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 32 {
            return Err(JsValue::from("Identifier must be 32 bytes length"));
        }

        let norm_slice: [u8; 32] = value
            .try_into()
            .map_err(|_| JsValue::from("Cannot parse identifier"))?;

        Ok(IdentifierWasm(Identifier::new(norm_slice)))
    }
}

impl TryFrom<JsValue> for IdentifierWasm {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        match value.is_object() {
            true => match get_class_type(&value) {
                Ok(class_type) => match class_type.as_str() {
                    "Identifier" => Ok(value.to_wasm::<IdentifierWasm>("Identifier")?.clone()),
                    "" => Ok(identifier_from_js_value(&value)?.into()),
                    _ => Err(Self::Error::from_str(&format!(
                        "Invalid type of data for identifier (passed {})",
                        class_type
                    ))),
                },
                Err(_) => Ok(identifier_from_js_value(&value)?.into()),
            },
            false => match value.is_string() {
                false => Ok(identifier_from_js_value(&value)?.into()),
                true => {
                    let id_str = value.as_string().unwrap();
                    match id_str.len() == 64 {
                        true => {
                            let bytes = decode(value.as_string().unwrap().as_str(), Hex)
                                .map_err(|err| JsValue::from(err.to_string()))?;

                            Ok(IdentifierWasm::try_from(bytes.as_slice())?)
                        }
                        false => Ok(identifier_from_js_value(&value)?.into()),
                    }
                }
            },
        }
    }
}

impl TryFrom<&JsValue> for IdentifierWasm {
    type Error = JsValue;
    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        IdentifierWasm::try_from(value.clone())
    }
}

#[wasm_bindgen(js_class = Identifier)]
impl IdentifierWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "Identifier".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "Identifier".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(js_identifier: &JsValue) -> Result<IdentifierWasm, JsValue> {
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
    pub fn from_base58(base58: String) -> Result<IdentifierWasm, JsValue> {
        let identitfier = Identifier::from_string(base58.as_str(), Base58)
            .map_err(|err| JsValue::from(err.to_string()))?;

        Ok(IdentifierWasm(identitfier))
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> Result<IdentifierWasm, JsValue> {
        let identitfier = Identifier::from_string(base64.as_str(), Base64)
            .map_err(|err| JsValue::from(err.to_string()))?;

        Ok(IdentifierWasm(identitfier))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> Result<IdentifierWasm, JsValue> {
        let identitfier = Identifier::from_string(hex.as_str(), Hex)
            .map_err(|err| JsValue::from(err.to_string()))?;

        Ok(IdentifierWasm(identitfier))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<IdentifierWasm, JsValue> {
        let identifier =
            Identifier::from_vec(bytes).map_err(|err| JsValue::from(err.to_string()))?;

        Ok(IdentifierWasm(identifier))
    }
}

impl IdentifierWasm {
    pub fn to_slice(&self) -> [u8; 32] {
        self.0.as_bytes().clone()
    }
}
