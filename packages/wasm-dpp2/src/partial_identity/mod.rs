use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::identity_public_key::IdentityPublicKeyWasm;
use crate::utils::IntoWasm;
use dpp::fee::Credits;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use dpp::prelude::Revision;
use js_sys::{Array, Object, Reflect};
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

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
    pub fn new(
        js_id: &JsValue,
        js_loaded_public_keys: &JsValue,
        balance: Option<Credits>,
        revision: Option<Revision>,
        js_not_found_public_keys: Option<Array>,
    ) -> WasmDppResult<Self> {
        let id = IdentifierWasm::try_from(js_id)?;
        let loaded_public_keys = js_value_to_loaded_public_keys(js_loaded_public_keys)?;

        let not_found_public_keys: BTreeSet<KeyID> =
            option_array_to_not_found(js_not_found_public_keys)?;

        Ok(PartialIdentityWasm(PartialIdentity {
            id: id.into(),
            loaded_public_keys,
            balance,
            revision,
            not_found_public_keys,
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
            .map_err(|err| WasmDppError::from_js_value(err))?;
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

        arr.into()
    }

    #[wasm_bindgen(setter = "id")]
    pub fn set_id(&mut self, js_id: &JsValue) -> WasmDppResult<()> {
        let identifier: IdentifierWasm = IdentifierWasm::try_from(js_id)?;

        self.0.id = identifier.into();

        Ok(())
    }

    #[wasm_bindgen(setter = "loadedPublicKeys")]
    pub fn set_loaded_public_keys(&mut self, loaded_public_keys: &JsValue) -> WasmDppResult<()> {
        self.0.loaded_public_keys = js_value_to_loaded_public_keys(loaded_public_keys)?;

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
}

pub fn js_value_to_loaded_public_keys(
    js_loaded_public_keys: &JsValue,
) -> WasmDppResult<BTreeMap<KeyID, IdentityPublicKey>> {
    match js_loaded_public_keys.is_object() {
        false => Err(WasmDppError::invalid_argument(
            "loaded_public_keys must be an object",
        )),
        true => {
            let mut map = BTreeMap::new();

            let pub_keys_object = Object::from(js_loaded_public_keys.clone());
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

                let js_key =
                    Reflect::get(&pub_keys_object, &key).map_err(WasmDppError::from_js_value)?;

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
    js_not_found_public_keys: Option<Array>,
) -> WasmDppResult<BTreeSet<KeyID>> {
    match js_not_found_public_keys {
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
