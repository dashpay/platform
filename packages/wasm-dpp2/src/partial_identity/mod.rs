use crate::identifier::IdentifierWASM;
use crate::identity_public_key::IdentityPublicKeyWASM;
use crate::utils::IntoWasm;
use dpp::fee::Credits;
use dpp::identity::{IdentityPublicKey, KeyID, PartialIdentity};
use dpp::prelude::Revision;
use js_sys::{Array, Object, Reflect};
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = "PartialIdentityWASM")]
pub struct PartialIdentityWASM(PartialIdentity);

impl From<PartialIdentity> for PartialIdentityWASM {
    fn from(value: PartialIdentity) -> Self {
        Self(value)
    }
}

#[wasm_bindgen]
impl PartialIdentityWASM {
    #[wasm_bindgen(constructor)]
    pub fn new(
        js_id: &JsValue,
        js_loaded_public_keys: &JsValue,
        balance: Option<Credits>,
        revision: Option<Revision>,
        js_not_found_public_keys: Option<Array>,
    ) -> Result<Self, JsValue> {
        let id = IdentifierWASM::try_from(js_id)?;
        let loaded_public_keys = js_value_to_loaded_public_keys(js_loaded_public_keys)?;

        let not_found_public_keys: BTreeSet<KeyID> =
            option_array_to_not_found(js_not_found_public_keys)?;

        Ok(PartialIdentityWASM(PartialIdentity {
            id: id.into(),
            loaded_public_keys,
            balance,
            revision,
            not_found_public_keys,
        }))
    }

    #[wasm_bindgen(getter = "id")]
    pub fn id(&self) -> IdentifierWASM {
        self.0.id.into()
    }

    #[wasm_bindgen(getter = "loadedPublicKeys")]
    pub fn loaded_public_keys(&self) -> Result<Object, JsValue> {
        let obj = Object::new();

        for (k, v) in self.0.loaded_public_keys.clone() {
            Reflect::set(
                &obj,
                &k.to_string().into(),
                &IdentityPublicKeyWASM::from(v).into(),
            )?;
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
    pub fn set_id(&mut self, js_id: &JsValue) -> Result<(), JsValue> {
        let identifier: IdentifierWASM = IdentifierWASM::try_from(js_id)?;

        self.0.id = identifier.into();

        Ok(())
    }

    #[wasm_bindgen(setter = "loadedPublicKeys")]
    pub fn set_loaded_public_keys(&mut self, loaded_public_keys: &JsValue) -> Result<(), JsValue> {
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
    pub fn set_not_found_public_keys(&mut self, keys: Option<Array>) -> Result<(), JsValue> {
        self.0.not_found_public_keys = option_array_to_not_found(keys)?;

        Ok(())
    }
}

pub fn js_value_to_loaded_public_keys(
    js_loaded_public_keys: &JsValue,
) -> Result<BTreeMap<KeyID, IdentityPublicKey>, JsValue> {
    match js_loaded_public_keys.is_object() {
        false => Err(JsValue::from("loaded_public_keys must be an object")),
        true => {
            let mut map = BTreeMap::new();

            let pub_keys_object = Object::from(js_loaded_public_keys.clone());
            let keys = Object::keys(&pub_keys_object);

            for key in keys.iter() {
                if key.as_f64().unwrap() > u32::MAX as f64 {
                    return Err(JsValue::from_str(&format!(
                        "Key id '{:?}' exceeds the maximum limit for u32.",
                        key.as_string()
                    )));
                }

                let key_id = KeyID::from(key.as_f64().unwrap() as u32);

                let js_key = Reflect::get(&pub_keys_object, &key)?;

                let key = js_key
                    .to_wasm::<IdentityPublicKeyWASM>("IdentityPublicKeyWASM")?
                    .clone();

                map.insert(key_id, IdentityPublicKey::from(key));
            }

            Ok(map)
        }
    }
}

pub fn option_array_to_not_found(
    js_not_found_public_keys: Option<Array>,
) -> Result<BTreeSet<KeyID>, JsValue> {
    match js_not_found_public_keys {
        None => Ok::<BTreeSet<KeyID>, JsValue>(BTreeSet::new()),
        Some(keys) => {
            let keys_iter: Vec<KeyID> = keys
                .to_vec()
                .iter()
                .map(|key| {
                    if key.as_f64().unwrap() > u32::MAX as f64 {
                        return Err(JsValue::from_str(&format!(
                            "Key id '{:?}' exceeds the maximum limit for u32.",
                            key.as_string()
                        )))?;
                    }

                    Ok(key.as_f64().unwrap() as KeyID)
                })
                .collect::<Result<Vec<KeyID>, JsValue>>()?;

            Ok(BTreeSet::from_iter(keys_iter.iter().map(|id| id.clone()))
                .iter()
                .map(|key| key.clone())
                .collect())
        }
    }
}
