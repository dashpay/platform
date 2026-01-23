use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::identity::public_key::IdentityPublicKeyWasm;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::utils::{IntoWasm, JsValueExt, get_required_property, try_to_object, try_to_u64};
use dpp::fee::Credits;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use dpp::platform_value;
use dpp::prelude::Revision;
use js_sys::{Array, Object, Reflect};
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const PARTIAL_IDENTITY_OPTIONS_TS: &'static str = r#"
export interface PartialIdentityOptions {
    id: IdentifierLike;
    loadedPublicKeys: Record<number, IdentityPublicKey>;
    balance?: bigint | number;
    revision?: bigint | number;
    notFoundPublicKeys?: number[];
}

/**
 * PartialIdentity serialized as a plain object.
 */
export interface PartialIdentityObject {
    id: Uint8Array;
    loadedPublicKeys: Record<string, IdentityPublicKeyObject>;
    balance?: bigint;
    revision?: bigint;
    notFoundPublicKeys: number[];
}

/**
 * PartialIdentity serialized as JSON.
 * Note: JSON uses null for missing optional fields (JSON standard).
 */
export interface PartialIdentityJSON {
    id: string;
    loadedPublicKeys: Record<string, IdentityPublicKeyJSON>;
    balance: number | null;
    revision: number | null;
    notFoundPublicKeys: number[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PartialIdentityOptions")]
    pub type PartialIdentityOptionsJs;

    #[wasm_bindgen(typescript_type = "PartialIdentityObject")]
    pub type PartialIdentityObjectJs;

    #[wasm_bindgen(typescript_type = "PartialIdentityJSON")]
    pub type PartialIdentityJSONJs;
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "PartialIdentity")]
pub struct PartialIdentityWasm(PartialIdentity);

impl From<PartialIdentity> for PartialIdentityWasm {
    fn from(value: PartialIdentity) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class = PartialIdentity)]
impl PartialIdentityWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(options: PartialIdentityOptionsJs) -> WasmDppResult<Self> {
        let options_obj = try_to_object(options.into(), "options")?;

        let id_js = get_required_property(&options_obj, "id")?;
        let id = IdentifierWasm::try_from(&id_js)?.into();

        let loaded_public_keys_js = get_required_property(&options_obj, "loadedPublicKeys")?;
        let loaded_public_keys = value_to_loaded_public_keys(&loaded_public_keys_js)?;

        let balance_js =
            Reflect::get(&options_obj, &"balance".into()).unwrap_or(JsValue::UNDEFINED);
        let balance: Option<Credits> = if balance_js.is_undefined() {
            None
        } else {
            Some(try_to_u64(balance_js)?)
        };

        let revision_js =
            Reflect::get(&options_obj, &"revision".into()).unwrap_or(JsValue::UNDEFINED);
        let revision: Option<Revision> = if revision_js.is_undefined() {
            None
        } else {
            Some(try_to_u64(revision_js)?)
        };

        let not_found_public_keys_js =
            Reflect::get(&options_obj, &"notFoundPublicKeys".into()).unwrap_or(JsValue::UNDEFINED);
        let not_found_public_keys: Option<Array> = if not_found_public_keys_js.is_undefined() {
            None
        } else {
            Some(Array::from(&not_found_public_keys_js))
        };
        let not_found_keys: BTreeSet<KeyID> = option_array_to_not_found(not_found_public_keys)?;

        Ok(PartialIdentityWasm(PartialIdentity {
            id,
            loaded_public_keys,
            balance,
            revision,
            not_found_public_keys: not_found_keys,
        }))
    }

    #[wasm_bindgen(getter = "id")]
    pub fn id(&self) -> IdentifierWasm {
        self.0.id.into()
    }

    #[wasm_bindgen(getter = "loadedPublicKeys")]
    pub fn loaded_public_keys(&self) -> WasmDppResult<Object> {
        let obj = Object::new();

        for (k, v) in self.0.loaded_public_keys.clone() {
            Reflect::set(
                &obj,
                &k.to_string().into(),
                &IdentityPublicKeyWasm::from(v).into(),
            )
            .map_err(|err| {
                let message = err.error_message();
                WasmDppError::generic(format!(
                    "failed to write loaded public key '{}' into JS object: {}",
                    k, message
                ))
            })?;
        }

        Ok(obj)
    }

    #[wasm_bindgen(getter = "balance")]
    pub fn balance(&self) -> Option<Credits> {
        self.0.balance
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn revision(&self) -> Option<Revision> {
        self.0.revision
    }

    #[wasm_bindgen(getter = "notFoundPublicKeys")]
    pub fn not_found_public_keys(&self) -> Array {
        let arr = Array::new();

        for v in self.0.not_found_public_keys.clone() {
            arr.push(&v.into());
        }

        arr
    }

    #[wasm_bindgen(setter = "id")]
    pub fn set_id(&mut self, id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.id = id.try_into()?;
        Ok(())
    }

    #[wasm_bindgen(setter = "loadedPublicKeys")]
    pub fn set_loaded_public_keys(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Record<number, IdentityPublicKey>")]
        loaded_public_keys: &JsValue,
    ) -> WasmDppResult<()> {
        self.0.loaded_public_keys = value_to_loaded_public_keys(loaded_public_keys)?;

        Ok(())
    }

    #[wasm_bindgen(setter = "balance")]
    pub fn set_balance(&mut self, balance: Option<Credits>) {
        self.0.balance = balance;
    }

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: Option<Revision>) {
        self.0.revision = revision;
    }

    #[wasm_bindgen(setter = "notFoundPublicKeys")]
    pub fn set_not_found_public_keys(&mut self, keys: Option<Array>) -> WasmDppResult<()> {
        self.0.not_found_public_keys = option_array_to_not_found(keys)?;

        Ok(())
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> WasmDppResult<PartialIdentityJSONJs> {
        // Convert to platform_value, which handles integer map keys by converting to strings
        let value = platform_value::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(format!("toJSON: {}", e)))?;
        // platform_value_to_json converts Identifier to base58, bytes to base64
        let js_value = serialization::platform_value_to_json(&value)?;
        Ok(js_value.into())
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<PartialIdentityObjectJs> {
        // Convert to platform_value, which handles integer map keys by converting to strings
        let value = platform_value::to_value(&self.0)
            .map_err(|e| WasmDppError::serialization(format!("toObject: {}", e)))?;
        // platform_value_to_object keeps bytes as Uint8Array, u64 as BigInt
        let js_value = serialization::platform_value_to_object(&value)?;
        Ok(js_value.into())
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(obj: PartialIdentityObjectJs) -> WasmDppResult<PartialIdentityWasm> {
        let obj_val: JsValue = obj.into();
        let options_obj = Object::from(obj_val);

        // id - can be Uint8Array or Identifier
        let id_js = get_required_property(&options_obj, "id")?;
        let id = IdentifierWasm::try_from(&id_js)?.into();

        // loadedPublicKeys - values are plain objects
        let loaded_public_keys_js = get_required_property(&options_obj, "loadedPublicKeys")?;
        let loaded_public_keys = value_to_loaded_public_keys_from_object(&loaded_public_keys_js)?;

        // balance - can be BigInt, number, or undefined
        let balance_js =
            Reflect::get(&options_obj, &"balance".into()).unwrap_or(JsValue::UNDEFINED);
        let balance: Option<Credits> = if balance_js.is_undefined() || balance_js.is_null() {
            None
        } else {
            Some(try_to_u64(balance_js)?)
        };

        // revision - can be BigInt, number, or undefined
        let revision_js =
            Reflect::get(&options_obj, &"revision".into()).unwrap_or(JsValue::UNDEFINED);
        let revision: Option<Revision> = if revision_js.is_undefined() || revision_js.is_null() {
            None
        } else {
            Some(try_to_u64(revision_js)?)
        };

        // notFoundPublicKeys
        let not_found_public_keys_js =
            Reflect::get(&options_obj, &"notFoundPublicKeys".into()).unwrap_or(JsValue::UNDEFINED);
        let not_found_public_keys: Option<Array> = if not_found_public_keys_js.is_undefined() {
            None
        } else {
            Some(Array::from(&not_found_public_keys_js))
        };
        let not_found_keys: BTreeSet<KeyID> = option_array_to_not_found(not_found_public_keys)?;

        Ok(PartialIdentityWasm(PartialIdentity {
            id,
            loaded_public_keys,
            balance,
            revision,
            not_found_public_keys: not_found_keys,
        }))
    }

    #[wasm_bindgen(js_name = "fromJSON")]
    pub fn from_json(json: PartialIdentityJSONJs) -> WasmDppResult<PartialIdentityWasm> {
        let obj_val: JsValue = json.into();
        let options_obj = Object::from(obj_val);

        // id - base58 string
        let id_js = get_required_property(&options_obj, "id")?;
        let id = IdentifierWasm::try_from(&id_js)?.into();

        // loadedPublicKeys - values are JSON objects
        let loaded_public_keys_js = get_required_property(&options_obj, "loadedPublicKeys")?;
        let loaded_public_keys = value_to_loaded_public_keys_from_json(&loaded_public_keys_js)?;

        // balance - number or null
        let balance_js =
            Reflect::get(&options_obj, &"balance".into()).unwrap_or(JsValue::UNDEFINED);
        let balance: Option<Credits> = if balance_js.is_undefined() || balance_js.is_null() {
            None
        } else {
            Some(try_to_u64(balance_js)?)
        };

        // revision - number or null
        let revision_js =
            Reflect::get(&options_obj, &"revision".into()).unwrap_or(JsValue::UNDEFINED);
        let revision: Option<Revision> = if revision_js.is_undefined() || revision_js.is_null() {
            None
        } else {
            Some(try_to_u64(revision_js)?)
        };

        // notFoundPublicKeys
        let not_found_public_keys_js =
            Reflect::get(&options_obj, &"notFoundPublicKeys".into()).unwrap_or(JsValue::UNDEFINED);
        let not_found_public_keys: Option<Array> = if not_found_public_keys_js.is_undefined() {
            None
        } else {
            Some(Array::from(&not_found_public_keys_js))
        };
        let not_found_keys: BTreeSet<KeyID> = option_array_to_not_found(not_found_public_keys)?;

        Ok(PartialIdentityWasm(PartialIdentity {
            id,
            loaded_public_keys,
            balance,
            revision,
            not_found_public_keys: not_found_keys,
        }))
    }
}

impl_wasm_type_info!(PartialIdentityWasm, PartialIdentity);

pub fn value_to_loaded_public_keys(
    loaded_public_keys: &JsValue,
) -> WasmDppResult<BTreeMap<KeyID, IdentityPublicKey>> {
    match loaded_public_keys.is_object() {
        false => Err(WasmDppError::invalid_argument(
            "loaded_public_keys must be an object",
        )),
        true => {
            let mut map = BTreeMap::new();

            let pub_keys_object = Object::from(loaded_public_keys.clone());
            let keys = Object::keys(&pub_keys_object);

            for key in keys.iter() {
                // Object.keys() returns strings, so we need to parse them
                let key_str = key.as_string().ok_or_else(|| {
                    WasmDppError::invalid_argument("Key identifier must be a string")
                })?;
                let key_val: f64 = key_str.parse().map_err(|_| {
                    WasmDppError::invalid_argument(format!(
                        "Key identifier '{}' must be numeric",
                        key_str
                    ))
                })?;

                if key_val > u32::MAX as f64 {
                    return Err(WasmDppError::invalid_argument(format!(
                        "Key id '{:?}' exceeds the maximum limit for u32.",
                        key.as_string()
                    )));
                }

                let key_id = KeyID::from(key_val as u32);

                let js_key = Reflect::get(&pub_keys_object, &key).map_err(|err| {
                    let message = err.error_message();
                    WasmDppError::invalid_argument(format!(
                        "unable to access loaded public key '{}': {}",
                        key_val as u32, message
                    ))
                })?;

                let key = js_key
                    .to_wasm::<IdentityPublicKeyWasm>("IdentityPublicKey")?
                    .clone();

                map.insert(key_id, IdentityPublicKey::from(key));
            }

            Ok(map)
        }
    }
}

pub fn option_array_to_not_found(
    not_found_public_keys: Option<Array>,
) -> WasmDppResult<BTreeSet<KeyID>> {
    match not_found_public_keys {
        None => Ok(BTreeSet::new()),
        Some(keys) => {
            let keys_iter: Vec<KeyID> = keys
                .to_vec()
                .iter()
                .map(|key| {
                    let key_val = key.as_f64().ok_or_else(|| {
                        WasmDppError::invalid_argument("Key id must be a numeric value")
                    })?;

                    if key_val > u32::MAX as f64 {
                        return Err(WasmDppError::invalid_argument(format!(
                            "Key id '{:?}' exceeds the maximum limit for u32.",
                            key.as_string()
                        )));
                    }

                    Ok(key_val as KeyID)
                })
                .collect::<WasmDppResult<Vec<KeyID>>>()?;

            Ok(keys_iter.into_iter().collect())
        }
    }
}

/// Parse loaded public keys from an object (where values are plain objects from toObject)
pub fn value_to_loaded_public_keys_from_object(
    loaded_public_keys: &JsValue,
) -> WasmDppResult<BTreeMap<KeyID, IdentityPublicKey>> {
    if !loaded_public_keys.is_object() {
        return Err(WasmDppError::invalid_argument(
            "loaded_public_keys must be an object",
        ));
    }

    let mut map = BTreeMap::new();
    let pub_keys_object = Object::from(loaded_public_keys.clone());
    let keys = Object::keys(&pub_keys_object);

    for key in keys.iter() {
        let key_str = key.as_string().ok_or_else(|| {
            WasmDppError::invalid_argument("Key identifier must be a string")
        })?;
        let key_val: f64 = key_str.parse().map_err(|_| {
            WasmDppError::invalid_argument(format!(
                "Key identifier '{}' must be numeric",
                key_str
            ))
        })?;

        if key_val > u32::MAX as f64 {
            return Err(WasmDppError::invalid_argument(format!(
                "Key id '{}' exceeds the maximum limit for u32.",
                key_str
            )));
        }

        let key_id = KeyID::from(key_val as u32);

        let js_key = Reflect::get(&pub_keys_object, &key).map_err(|err| {
            let message = err.error_message();
            WasmDppError::invalid_argument(format!(
                "unable to access loaded public key '{}': {}",
                key_val as u32, message
            ))
        })?;

        // fromObject receives plain objects, use IdentityPublicKeyWasm::from_object
        let pub_key = IdentityPublicKeyWasm::from_object(js_key.into())?;
        map.insert(key_id, IdentityPublicKey::from(pub_key));
    }

    Ok(map)
}

/// Parse loaded public keys from JSON (where values are JSON objects from toJSON)
pub fn value_to_loaded_public_keys_from_json(
    loaded_public_keys: &JsValue,
) -> WasmDppResult<BTreeMap<KeyID, IdentityPublicKey>> {
    if !loaded_public_keys.is_object() {
        return Err(WasmDppError::invalid_argument(
            "loaded_public_keys must be an object",
        ));
    }

    let mut map = BTreeMap::new();
    let pub_keys_object = Object::from(loaded_public_keys.clone());
    let keys = Object::keys(&pub_keys_object);

    for key in keys.iter() {
        let key_str = key.as_string().ok_or_else(|| {
            WasmDppError::invalid_argument("Key identifier must be a string")
        })?;
        let key_val: f64 = key_str.parse().map_err(|_| {
            WasmDppError::invalid_argument(format!(
                "Key identifier '{}' must be numeric",
                key_str
            ))
        })?;

        if key_val > u32::MAX as f64 {
            return Err(WasmDppError::invalid_argument(format!(
                "Key id '{}' exceeds the maximum limit for u32.",
                key_str
            )));
        }

        let key_id = KeyID::from(key_val as u32);

        let js_key = Reflect::get(&pub_keys_object, &key).map_err(|err| {
            let message = err.error_message();
            WasmDppError::invalid_argument(format!(
                "unable to access loaded public key '{}': {}",
                key_val as u32, message
            ))
        })?;

        // fromJSON receives JSON objects, use IdentityPublicKeyWasm::from_json
        let pub_key = IdentityPublicKeyWasm::from_json(js_key.into())?;
        map.insert(key_id, IdentityPublicKey::from(pub_key));
    }

    Ok(map)
}

