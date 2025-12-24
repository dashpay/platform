use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
// PathQuery is re-exported through drive
use drive::query::{PathQuery, QueryItem};
use js_sys::{Array, Object, Reflect, Uint8Array};
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TokenTransitionPathQueryResult {
    path_query: JsValue,
}

#[wasm_bindgen]
impl TokenTransitionPathQueryResult {
    #[wasm_bindgen(getter)]
    pub fn path_query(&self) -> JsValue {
        self.path_query.clone()
    }
}

#[wasm_bindgen(js_name = "tokenTransitionIntoPathQuery")]
pub fn token_transition_into_path_query(
    token_transition_js: &JsValue,
    contract_js: &JsValue,
    owner_id: &Uint8Array,
    platform_version_number: u32,
) -> Result<TokenTransitionPathQueryResult, JsValue> {
    // Parse token transition from JS
    let _token_transition: TokenTransition = from_value(token_transition_js.clone())
        .map_err(|e| JsValue::from_str(&format!("Failed to parse token transition: {:?}", e)))?;

    // Parse contract from JS
    let _contract: DataContract = from_value(contract_js.clone())
        .map_err(|e| JsValue::from_str(&format!("Failed to parse contract: {:?}", e)))?;

    // Parse owner ID
    let owner_id_bytes: [u8; 32] = owner_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid owner_id length. Expected 32 bytes."))?;
    let _owner_identifier = Identifier::from(owner_id_bytes);

    let _platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    // Note: Since we can't directly use the trait method from rs-drive here,
    // we would need to implement the path query logic based on the token transition type
    // For now, return an error indicating this needs implementation
    Err(JsValue::from_str(
        "Token transition path query conversion not yet implemented in WASM bindings",
    ))
}

fn convert_path_query_to_js(path_query: &PathQuery) -> Result<JsValue, JsValue> {
    let obj = Object::new();

    // Convert path
    let path_array = Array::new();
    for segment in path_query.path.as_slice() {
        let segment_array = Uint8Array::from(segment.as_slice());
        path_array.push(&segment_array);
    }
    Reflect::set(&obj, &JsValue::from_str("path"), &path_array)
        .map_err(|_| JsValue::from_str("Failed to set path"))?;

    // Convert query (this is a simplified version - real implementation would need to handle all query types)
    let query_obj = Object::new();

    // Handle different query types
    if !path_query.query.query.items.is_empty() {
        let items_array = Array::new();
        for item in &path_query.query.query.items {
            let item_js = serialize_query_item(item)?;
            items_array.push(&item_js);
        }
        Reflect::set(&query_obj, &JsValue::from_str("items"), &items_array)
            .map_err(|_| JsValue::from_str("Failed to set items"))?;
    }

    // Range queries are handled through the QueryItem serialization above
    // Each QueryItem contains its own range information

    if let Some(limit) = path_query.query.limit {
        Reflect::set(
            &query_obj,
            &JsValue::from_str("limit"),
            &JsValue::from(limit),
        )
        .map_err(|_| JsValue::from_str("Failed to set limit"))?;
    }

    if let Some(offset) = path_query.query.offset {
        Reflect::set(
            &query_obj,
            &JsValue::from_str("offset"),
            &JsValue::from(offset),
        )
        .map_err(|_| JsValue::from_str("Failed to set offset"))?;
    }

    Reflect::set(&obj, &JsValue::from_str("query"), &query_obj)
        .map_err(|_| JsValue::from_str("Failed to set query"))?;

    Ok(obj.into())
}

// Helper functions for creating specific queries (matching the rs-drive implementation)

#[wasm_bindgen(js_name = "tokenBalanceForIdentityIdQuery")]
pub fn token_balance_for_identity_id_query(
    token_id: &Uint8Array,
    identity_id: &Uint8Array,
) -> Result<TokenTransitionPathQueryResult, JsValue> {
    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

    let path_query = Drive::token_balance_for_identity_id_query(token_id_bytes, identity_id_bytes);
    let path_query_js = convert_path_query_to_js(&path_query)?;

    Ok(TokenTransitionPathQueryResult {
        path_query: path_query_js,
    })
}

#[wasm_bindgen(js_name = "tokenBalancesForIdentityIdsQuery")]
pub fn token_balances_for_identity_ids_query(
    token_id: &Uint8Array,
    identity_ids_js: &JsValue,
) -> Result<TokenTransitionPathQueryResult, JsValue> {
    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    // Parse identity IDs array
    let identity_ids_array: Array = identity_ids_js
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("identity_ids must be an array"))?;

    let mut identity_ids: Vec<[u8; 32]> = Vec::new();
    for i in 0..identity_ids_array.length() {
        let id_uint8array: Uint8Array = identity_ids_array
            .get(i)
            .dyn_into()
            .map_err(|_| JsValue::from_str("Each identity ID must be a Uint8Array"))?;

        let id_bytes: [u8; 32] = id_uint8array
            .to_vec()
            .try_into()
            .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

        identity_ids.push(id_bytes);
    }

    let path_query = Drive::token_balances_for_identity_ids_query(token_id_bytes, &identity_ids);
    let path_query_js = convert_path_query_to_js(&path_query)?;

    Ok(TokenTransitionPathQueryResult {
        path_query: path_query_js,
    })
}

#[wasm_bindgen(js_name = "tokenInfoForIdentityIdQuery")]
pub fn token_info_for_identity_id_query(
    token_id: &Uint8Array,
    identity_id: &Uint8Array,
) -> Result<TokenTransitionPathQueryResult, JsValue> {
    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

    let path_query = Drive::token_info_for_identity_id_query(token_id_bytes, identity_id_bytes);
    let path_query_js = convert_path_query_to_js(&path_query)?;

    Ok(TokenTransitionPathQueryResult {
        path_query: path_query_js,
    })
}

#[wasm_bindgen(js_name = "tokenDirectPurchasePriceQuery")]
pub fn token_direct_purchase_price_query(
    token_id: &Uint8Array,
) -> Result<TokenTransitionPathQueryResult, JsValue> {
    let token_id_bytes: [u8; 32] = token_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid token_id length. Expected 32 bytes."))?;

    let path_query = Drive::token_direct_purchase_price_query(token_id_bytes);
    let path_query_js = convert_path_query_to_js(&path_query)?;

    Ok(TokenTransitionPathQueryResult {
        path_query: path_query_js,
    })
}

#[wasm_bindgen(js_name = "groupActiveAndClosedActionSingleSignerQuery")]
pub fn group_active_and_closed_action_single_signer_query(
    contract_id: &Uint8Array,
    group_contract_position: u16,
    action_id: &Uint8Array,
    identity_id: &Uint8Array,
) -> Result<TokenTransitionPathQueryResult, JsValue> {
    let contract_id_bytes: [u8; 32] = contract_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid contract_id length. Expected 32 bytes."))?;

    let action_id_bytes: [u8; 32] = action_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid action_id length. Expected 32 bytes."))?;

    let identity_id_bytes: [u8; 32] = identity_id
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?;

    let path_query = Drive::group_active_and_closed_action_single_signer_query(
        contract_id_bytes,
        group_contract_position,
        action_id_bytes,
        identity_id_bytes,
    );
    let path_query_js = convert_path_query_to_js(&path_query)?;

    Ok(TokenTransitionPathQueryResult {
        path_query: path_query_js,
    })
}

fn serialize_query_item(item: &QueryItem) -> Result<JsValue, JsValue> {
    let obj = Object::new();

    match item {
        QueryItem::Key(key) => {
            Reflect::set(&obj, &JsValue::from_str("type"), &JsValue::from_str("Key"))
                .map_err(|_| JsValue::from_str("Failed to set type"))?;
            let key_array = Uint8Array::from(key.as_slice());
            Reflect::set(&obj, &JsValue::from_str("key"), &key_array)
                .map_err(|_| JsValue::from_str("Failed to set key"))?;
        }
        QueryItem::Range(range) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("Range"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            let start_array = Uint8Array::from(range.start.as_slice());
            let end_array = Uint8Array::from(range.end.as_slice());
            Reflect::set(&obj, &JsValue::from_str("start"), &start_array)
                .map_err(|_| JsValue::from_str("Failed to set start"))?;
            Reflect::set(&obj, &JsValue::from_str("end"), &end_array)
                .map_err(|_| JsValue::from_str("Failed to set end"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("startInclusive"),
                &JsValue::from_bool(true),
            )
            .map_err(|_| JsValue::from_str("Failed to set startInclusive"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("endInclusive"),
                &JsValue::from_bool(false),
            )
            .map_err(|_| JsValue::from_str("Failed to set endInclusive"))?;
        }
        QueryItem::RangeInclusive(range) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("Range"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            let start_array = Uint8Array::from(range.start().as_slice());
            let end_array = Uint8Array::from(range.end().as_slice());
            Reflect::set(&obj, &JsValue::from_str("start"), &start_array)
                .map_err(|_| JsValue::from_str("Failed to set start"))?;
            Reflect::set(&obj, &JsValue::from_str("end"), &end_array)
                .map_err(|_| JsValue::from_str("Failed to set end"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("startInclusive"),
                &JsValue::from_bool(true),
            )
            .map_err(|_| JsValue::from_str("Failed to set startInclusive"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("endInclusive"),
                &JsValue::from_bool(true),
            )
            .map_err(|_| JsValue::from_str("Failed to set endInclusive"))?;
        }
        QueryItem::RangeFull(_) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("RangeFull"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
        }
        QueryItem::RangeFrom(range) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("RangeFrom"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            let start_array = Uint8Array::from(range.start.as_slice());
            Reflect::set(&obj, &JsValue::from_str("start"), &start_array)
                .map_err(|_| JsValue::from_str("Failed to set start"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("startInclusive"),
                &JsValue::from_bool(true),
            )
            .map_err(|_| JsValue::from_str("Failed to set startInclusive"))?;
        }
        QueryItem::RangeTo(range) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("RangeTo"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            let end_array = Uint8Array::from(range.end.as_slice());
            Reflect::set(&obj, &JsValue::from_str("end"), &end_array)
                .map_err(|_| JsValue::from_str("Failed to set end"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("endInclusive"),
                &JsValue::from_bool(false),
            )
            .map_err(|_| JsValue::from_str("Failed to set endInclusive"))?;
        }
        QueryItem::RangeToInclusive(range) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("RangeTo"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            let end_array = Uint8Array::from(range.end.as_slice());
            Reflect::set(&obj, &JsValue::from_str("end"), &end_array)
                .map_err(|_| JsValue::from_str("Failed to set end"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("endInclusive"),
                &JsValue::from_bool(true),
            )
            .map_err(|_| JsValue::from_str("Failed to set endInclusive"))?;
        }
        QueryItem::RangeAfter(range) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("RangeAfter"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            let start_array = Uint8Array::from(range.start.as_slice());
            Reflect::set(&obj, &JsValue::from_str("start"), &start_array)
                .map_err(|_| JsValue::from_str("Failed to set start"))?;
        }
        QueryItem::RangeAfterTo(range) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("RangeAfterTo"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            let start_array = Uint8Array::from(range.start.as_slice());
            let end_array = Uint8Array::from(range.end.as_slice());
            Reflect::set(&obj, &JsValue::from_str("start"), &start_array)
                .map_err(|_| JsValue::from_str("Failed to set start"))?;
            Reflect::set(&obj, &JsValue::from_str("end"), &end_array)
                .map_err(|_| JsValue::from_str("Failed to set end"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("endInclusive"),
                &JsValue::from_bool(false),
            )
            .map_err(|_| JsValue::from_str("Failed to set endInclusive"))?;
        }
        QueryItem::RangeAfterToInclusive(range) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("RangeAfterTo"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;
            let start_array = Uint8Array::from(range.start().as_slice());
            let end_array = Uint8Array::from(range.end().as_slice());
            Reflect::set(&obj, &JsValue::from_str("start"), &start_array)
                .map_err(|_| JsValue::from_str("Failed to set start"))?;
            Reflect::set(&obj, &JsValue::from_str("end"), &end_array)
                .map_err(|_| JsValue::from_str("Failed to set end"))?;
            Reflect::set(
                &obj,
                &JsValue::from_str("endInclusive"),
                &JsValue::from_bool(true),
            )
            .map_err(|_| JsValue::from_str("Failed to set endInclusive"))?;
        }
    }

    Ok(obj.into())
}
