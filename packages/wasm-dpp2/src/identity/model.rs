use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::identity::public_key::IdentityPublicKeyWasm;
use crate::utils::{IntoWasm, JsValueExt};
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::fields::IDENTIFIER_FIELDS_RAW_OBJECT;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::fields::BINARY_DATA_FIELDS;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{self, Identity, KeyID};
use dpp::platform_value::ReplacementType;
use dpp::platform_value::Value;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{Identifier, IdentityPublicKey};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, ValueConvertible};
use dpp::version::PlatformVersion;
use js_sys::{Array, BigInt, Error as JsError, Object, RangeError, Reflect};
use serde_json::Value as JsonValue;
use serde_wasm_bindgen::{from_value, to_value};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use wasm_bindgen::JsCast;
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
    pub fn get_public_key_by_id(&self, key_id: KeyID) -> IdentityPublicKeyWasm {
        let identity_public_key = self.0.get_public_key_by_id(key_id);
        IdentityPublicKeyWasm::from(identity_public_key.unwrap().clone())
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
        let object = Object::new();

        Reflect::set(
            &object,
            &JsValue::from_str("id"),
            &JsValue::from(IdentifierWasm::from(self.0.id())),
        )
        .map_err(|err| {
            WasmDppError::serialization(format!(
                "unable to set identity id: {}",
                err.error_message()
            ))
        })?;

        let public_keys = Array::new();
        for public_key in self.0.public_keys().values() {
            public_keys.push(&JsValue::from(IdentityPublicKeyWasm::from(
                public_key.clone(),
            )));
        }
        Reflect::set(
            &object,
            &JsValue::from_str("publicKeys"),
            &public_keys.into(),
        )
        .map_err(|err| {
            WasmDppError::serialization(format!(
                "unable to set identity publicKeys: {}",
                err.error_message()
            ))
        })?;

        let balance = big_int_from_u64(self.0.balance())?;
        Reflect::set(&object, &JsValue::from_str("balance"), &balance.into()).map_err(|err| {
            WasmDppError::serialization(format!(
                "unable to set identity balance: {}",
                err.error_message()
            ))
        })?;

        let revision = big_int_from_u64(self.0.revision())?;
        Reflect::set(&object, &JsValue::from_str("revision"), &revision.into()).map_err(|err| {
            WasmDppError::serialization(format!(
                "unable to set identity revision: {}",
                err.error_message()
            ))
        })?;

        Ok(object.into())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let json_value = self.cleaned_json_value()?;
        to_value(&json_value).map_err(|e| WasmDppError::serialization(e.to_string()))
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(js_value: JsValue) -> WasmDppResult<IdentityWasm> {
        let json_value: JsonValue =
            from_value(js_value).map_err(|err| WasmDppError::serialization(err.to_string()))?;

        let value = Value::from(json_value);

        identity_from_platform_value(value)
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(js_value: JsValue) -> WasmDppResult<IdentityWasm> {
        let object = IdentityWasm::from_js_value(js_value)?;

        Ok(object)
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityWasm> {
        let identity = Identity::deserialize_from_bytes(bytes.as_slice())?;
        Ok(IdentityWasm(identity))
    }
}

fn identity_from_platform_value(mut value: Value) -> WasmDppResult<IdentityWasm> {
    value
        .replace_at_paths(IDENTIFIER_FIELDS_RAW_OBJECT, ReplacementType::Identifier)
        .map_err(|err| WasmDppError::serialization(err.to_string()))?;

    if let Some(public_keys) = value
        .get_optional_array_mut_ref(identity::property_names::PUBLIC_KEYS)
        .map_err(|err| WasmDppError::serialization(err.to_string()))?
    {
        for public_key in public_keys.iter_mut() {
            public_key
                .replace_at_paths(BINARY_DATA_FIELDS, ReplacementType::BinaryBytes)
                .map_err(|err| WasmDppError::serialization(err.to_string()))?;
        }
    }

    let identity_v0 = IdentityV0::try_from(value).map_err(WasmDppError::from)?;

    Ok(IdentityWasm(Identity::from(identity_v0)))
}

fn big_int_from_u64(value: u64) -> WasmDppResult<BigInt> {
    BigInt::new(&JsValue::from_str(&value.to_string())).map_err(|err: JsError| {
        WasmDppError::serialization(format!(
            "unable to convert number to BigInt: {}",
            err.message()
        ))
    })
}

fn js_big_int_to_u64(value: &JsValue) -> WasmDppResult<u64> {
    let bigint: BigInt = value.clone().dyn_into().map_err(|_| {
        WasmDppError::invalid_argument("expected balance/revision to be a BigInt".to_string())
    })?;

    let js_string = BigInt::to_string(&bigint, 10).map_err(|err: RangeError| {
        WasmDppError::serialization(format!("unable to stringify BigInt: {}", err.message()))
    })?;

    let string = js_string.as_string().ok_or_else(|| {
        WasmDppError::serialization("unable to convert BigInt to string".to_string())
    })?;

    string
        .parse::<u64>()
        .map_err(|err| WasmDppError::serialization(err.to_string()))
}

impl IdentityWasm {
    pub fn get_rs_public_keys(&self) -> BTreeMap<KeyID, IdentityPublicKey> {
        self.0.public_keys().clone()
    }

    fn from_js_value(js_value: JsValue) -> WasmDppResult<IdentityWasm> {
        if let Ok(identity_wasm) = js_value.to_wasm::<IdentityWasm>("Identity") {
            return Ok(identity_wasm.clone());
        }

        if js_value.is_object() {
            let object = Object::from(js_value.clone());

            let id_js = Reflect::get(&object, &JsValue::from_str("id")).map_err(|err| {
                WasmDppError::invalid_argument(format!(
                    "unable to read identity id: {}",
                    err.error_message()
                ))
            })?;
            let identifier: Identifier = IdentifierWasm::try_from(&id_js)?.into();

            let js_public_keys =
                Reflect::get(&object, &JsValue::from_str("publicKeys")).map_err(|err| {
                    WasmDppError::invalid_argument(format!(
                        "unable to read identity publicKeys: {}",
                        err.error_message()
                    ))
                })?;
            let mut public_keys: BTreeMap<KeyID, IdentityPublicKey> = BTreeMap::new();
            if !js_public_keys.is_undefined() && !js_public_keys.is_null() {
                let array = Array::from(&js_public_keys);
                for value in array.iter() {
                    let public_key_wasm = value
                        .to_wasm::<IdentityPublicKeyWasm>("IdentityPublicKey")
                        .map_err(|err| {
                            WasmDppError::invalid_argument(format!(
                                "identity publicKeys must contain IdentityPublicKey instances: {}",
                                err
                            ))
                        })?;
                    let public_key: IdentityPublicKey = public_key_wasm.clone().into();
                    public_keys.insert(public_key.id(), public_key);
                }
            }

            let balance_js =
                Reflect::get(&object, &JsValue::from_str("balance")).map_err(|err| {
                    WasmDppError::invalid_argument(format!(
                        "unable to read identity balance: {}",
                        err.error_message()
                    ))
                })?;
            let balance = if balance_js.is_undefined() || balance_js.is_null() {
                0
            } else {
                js_big_int_to_u64(&balance_js)?
            };

            let revision_js =
                Reflect::get(&object, &JsValue::from_str("revision")).map_err(|err| {
                    WasmDppError::invalid_argument(format!(
                        "unable to read identity revision: {}",
                        err.error_message()
                    ))
                })?;
            let revision = if revision_js.is_undefined() || revision_js.is_null() {
                0
            } else {
                js_big_int_to_u64(&revision_js)?
            };

            let identity_v0 = IdentityV0 {
                id: identifier,
                public_keys,
                balance,
                revision,
            };

            return Ok(IdentityWasm(Identity::from(identity_v0)));
        }

        let json_value: JsonValue =
            from_value(js_value).map_err(|err| WasmDppError::serialization(err.to_string()))?;

        let value = Value::from(json_value);

        identity_from_platform_value(value)
    }
}
