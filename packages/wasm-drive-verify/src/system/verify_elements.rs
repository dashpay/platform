use drive::verify::RootHash;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;
use js_sys::{Uint8Array, Array, Object, Reflect};
use serde_wasm_bindgen::to_value;
use std::collections::BTreeMap;
use grovedb::Element;

#[wasm_bindgen]
pub struct VerifyElementsResult {
    root_hash: Vec<u8>,
    elements: JsValue,
}

#[wasm_bindgen]
impl VerifyElementsResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn elements(&self) -> JsValue {
        self.elements.clone()
    }
}

#[wasm_bindgen(js_name = "verifyElements")]
pub fn verify_elements(
    proof: &Uint8Array,
    path: &JsValue,
    keys: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyElementsResult, JsValue> {
    let proof_vec = proof.to_vec();
    
    // Parse path from JS array of Uint8Arrays
    let path_array: Array = path.clone().dyn_into()
        .map_err(|_| JsValue::from_str("path must be an array"))?;
    
    let mut path_vec = Vec::new();
    for i in 0..path_array.length() {
        let item = path_array.get(i);
        let item_uint8: Uint8Array = item.dyn_into()
            .map_err(|_| JsValue::from_str("Each path item must be a Uint8Array"))?;
        path_vec.push(item_uint8.to_vec());
    }

    // Parse keys from JS array of Uint8Arrays
    let keys_array: Array = keys.clone().dyn_into()
        .map_err(|_| JsValue::from_str("keys must be an array"))?;
    
    let mut keys_vec = Vec::new();
    for i in 0..keys_array.length() {
        let item = keys_array.get(i);
        let item_uint8: Uint8Array = item.dyn_into()
            .map_err(|_| JsValue::from_str("Each key must be a Uint8Array"))?;
        keys_vec.push(item_uint8.to_vec());
    }

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, elements_map) = drive::verify::system::verify_elements(
        &proof_vec,
        path_vec,
        keys_vec,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert BTreeMap<Vec<u8>, Option<Element>> to JS object
    let js_obj = Object::new();
    for (key, element_option) in elements_map {
        let hex_key = hex::encode(&key);
        
        let element_js = match element_option {
            Some(element) => {
                let element_json = match element {
                    Element::Item(data, _) => {
                        serde_json::json!({
                            "type": "Item",
                            "data": hex::encode(&data)
                        })
                    },
                    Element::Reference(reference, _, _) => {
                        serde_json::json!({
                            "type": "Reference",
                            "reference": reference.as_slice().iter().map(|path| hex::encode(path)).collect::<Vec<_>>()
                        })
                    },
                    Element::Tree(root_hash, _) => {
                        serde_json::json!({
                            "type": "Tree",
                            "rootHash": root_hash.map(hex::encode)
                        })
                    },
                    Element::SumItem(value, _) => {
                        serde_json::json!({
                            "type": "SumItem",
                            "value": value
                        })
                    },
                    Element::SumTree(root_hash, sum, _) => {
                        serde_json::json!({
                            "type": "SumTree",
                            "rootHash": root_hash.map(hex::encode),
                            "sum": sum
                        })
                    },
                };
                to_value(&element_json)
                    .map_err(|e| JsValue::from_str(&format!("Failed to convert element to JsValue: {:?}", e)))?
            }
            None => JsValue::NULL,
        };
        
        Reflect::set(&js_obj, &JsValue::from_str(&hex_key), &element_js)
            .map_err(|_| JsValue::from_str("Failed to set element in result object"))?;
    }

    Ok(VerifyElementsResult {
        root_hash: root_hash.to_vec(),
        elements: js_obj.into(),
    })
}