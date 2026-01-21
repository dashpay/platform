use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::identity::public_key::IdentityPublicKeyWasm;
use crate::impl_wasm_type_info;
use crate::utils::{IntoWasm, JsValueExt, try_to_u64};
use dpp::fee::Credits;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use dpp::prelude::Revision;
use js_sys::{Array, Object, Reflect};
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

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
    id: Identifier;
    loadedPublicKeys: Record<number, IdentityPublicKey>;
    balance: bigint | null;
    revision: bigint | null;
    notFoundPublicKeys: number[];
}

/**
 * PartialIdentity serialized as JSON.
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
        let options_obj = Object::from(JsValue::from(options));

        let id_js = Reflect::get(&options_obj, &"id".into())
            .map_err(|_| WasmDppError::invalid_argument("id is required"))?;
        let id = IdentifierWasm::try_from(&id_js)?.into();

        let loaded_public_keys_js = Reflect::get(&options_obj, &"loadedPublicKeys".into())
            .map_err(|_| WasmDppError::invalid_argument("loadedPublicKeys is required"))?;
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
        let obj = Object::new();

        Reflect::set(&obj, &"id".into(), &self.id().to_base58().into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        // Serialize loaded public keys as object with string keys and JSON values
        let loaded_keys_obj = Object::new();
        for (k, v) in self.0.loaded_public_keys.clone() {
            let key_wasm = IdentityPublicKeyWasm::from(v);
            let key_json: JsValue = key_wasm.to_json()?.into();
            Reflect::set(&loaded_keys_obj, &k.to_string().into(), &key_json)
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;
        }
        Reflect::set(&obj, &"loadedPublicKeys".into(), &loaded_keys_obj.into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        match self.0.balance {
            Some(b) => Reflect::set(&obj, &"balance".into(), &JsValue::from(b as f64))
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?,
            None => Reflect::set(&obj, &"balance".into(), &JsValue::NULL)
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?,
        };

        match self.0.revision {
            Some(r) => Reflect::set(&obj, &"revision".into(), &JsValue::from(r as f64))
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?,
            None => Reflect::set(&obj, &"revision".into(), &JsValue::NULL)
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?,
        };

        let not_found_arr = Array::new();
        for k in self.0.not_found_public_keys.iter() {
            not_found_arr.push(&JsValue::from(*k));
        }
        Reflect::set(&obj, &"notFoundPublicKeys".into(), &not_found_arr.into())
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        Ok(obj.unchecked_into())
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> WasmDppResult<PartialIdentityObjectJs> {
        let obj = Object::new();

        Reflect::set(&obj, &"id".into(), &JsValue::from(self.id()))
            .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        Reflect::set(
            &obj,
            &"loadedPublicKeys".into(),
            &self.loaded_public_keys()?.into(),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        match self.0.balance {
            Some(b) => Reflect::set(&obj, &"balance".into(), &js_sys::BigInt::from(b).into())
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?,
            None => Reflect::set(&obj, &"balance".into(), &JsValue::NULL)
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?,
        };

        match self.0.revision {
            Some(r) => Reflect::set(&obj, &"revision".into(), &js_sys::BigInt::from(r).into())
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?,
            None => Reflect::set(&obj, &"revision".into(), &JsValue::NULL)
                .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?,
        };

        Reflect::set(
            &obj,
            &"notFoundPublicKeys".into(),
            &self.not_found_public_keys().into(),
        )
        .map_err(|e| WasmDppError::serialization(format!("{:?}", e)))?;

        Ok(obj.unchecked_into())
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
                let key_val = key.as_f64().ok_or_else(|| {
                    WasmDppError::invalid_argument("Key identifier must be numeric")
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
