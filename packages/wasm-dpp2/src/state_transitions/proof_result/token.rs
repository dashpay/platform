//! Token-related `StateTransitionProofResult` wrappers.

use super::helpers::js_obj;
use crate::DocumentWasm;
use crate::IdentifierWasm;
use crate::IdentityTokenInfoWasm;
use crate::TokenStatusWasm;
use crate::error::WasmDppResult;
use crate::impl_wasm_conversions_serde;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_pricing_schedule::TokenPricingScheduleWasm;
use js_sys::{BigInt, Map};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

// --- VerifiedTokenBalanceAbsence ---

#[wasm_bindgen(js_name = "VerifiedTokenBalanceAbsence")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenBalanceAbsenceWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "tokenId")]
    pub token_id: IdentifierWasm,
}

impl_wasm_type_info!(VerifiedTokenBalanceAbsenceWasm, VerifiedTokenBalanceAbsence);
impl_wasm_conversions_serde!(VerifiedTokenBalanceAbsenceWasm, VerifiedTokenBalanceAbsence);

// --- VerifiedTokenBalance ---

#[dpp_json_convertible_derive::json_safe_fields(crate = "dpp")]
#[wasm_bindgen(js_name = "VerifiedTokenBalance")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenBalanceWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "tokenId")]
    pub token_id: IdentifierWasm,
    pub(super) balance: u64,
}

#[wasm_bindgen(js_class = VerifiedTokenBalance)]
impl VerifiedTokenBalanceWasm {
    #[wasm_bindgen(getter)]
    pub fn balance(&self) -> JsValue {
        BigInt::from(self.balance).into()
    }
}

impl_wasm_type_info!(VerifiedTokenBalanceWasm, VerifiedTokenBalance);
impl_wasm_conversions_serde!(VerifiedTokenBalanceWasm, VerifiedTokenBalance);

// --- VerifiedTokenIdentityInfo ---

#[wasm_bindgen(js_name = "VerifiedTokenIdentityInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenIdentityInfoWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "tokenId")]
    pub token_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "tokenInfo")]
    pub token_info: IdentityTokenInfoWasm,
}

impl_wasm_type_info!(VerifiedTokenIdentityInfoWasm, VerifiedTokenIdentityInfo);
impl_wasm_conversions_serde!(VerifiedTokenIdentityInfoWasm, VerifiedTokenIdentityInfo);

// --- VerifiedTokenPricingSchedule ---

#[wasm_bindgen(js_name = "VerifiedTokenPricingSchedule")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenPricingScheduleWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "tokenId")]
    pub token_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "pricingSchedule")]
    pub pricing_schedule: Option<TokenPricingScheduleWasm>,
}

impl_wasm_type_info!(
    VerifiedTokenPricingScheduleWasm,
    VerifiedTokenPricingSchedule
);
impl_wasm_conversions_serde!(
    VerifiedTokenPricingScheduleWasm,
    VerifiedTokenPricingSchedule
);

// --- VerifiedTokenStatus ---

#[wasm_bindgen(js_name = "VerifiedTokenStatus")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenStatusWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "tokenStatus")]
    pub token_status: TokenStatusWasm,
}

impl_wasm_type_info!(VerifiedTokenStatusWasm, VerifiedTokenStatus);
impl_wasm_conversions_serde!(VerifiedTokenStatusWasm, VerifiedTokenStatus);

// --- VerifiedTokenIdentitiesBalances ---

#[wasm_bindgen(js_name = "VerifiedTokenIdentitiesBalances")]
#[derive(Clone)]
pub struct VerifiedTokenIdentitiesBalancesWasm {
    pub(super) balances: Map, // Map<string(base58), BigInt>
}

#[wasm_bindgen(js_class = VerifiedTokenIdentitiesBalances)]
impl VerifiedTokenIdentitiesBalancesWasm {
    #[wasm_bindgen(getter)]
    pub fn balances(&self) -> Map {
        self.balances.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[("balances", self.balances.clone().into())])
    }

    /// Returns a `JSON.stringify`-friendly form: the `Map` is normalised to a
    /// plain object so its entries survive serialisation (otherwise
    /// `JSON.stringify({balances: <Map>})` produces `{"balances":{}}`).
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        crate::serialization::conversions::normalize_js_value_for_json(&self.to_object())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedTokenIdentitiesBalancesWasm> {
        let balances = super::helpers::read_map_property(&value, "balances")?;
        Ok(VerifiedTokenIdentitiesBalancesWasm { balances })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedTokenIdentitiesBalancesWasm> {
        Self::from_object(value)
    }
}

impl_wasm_type_info!(
    VerifiedTokenIdentitiesBalancesWasm,
    VerifiedTokenIdentitiesBalances
);

// --- VerifiedTokenActionWithDocument ---

#[wasm_bindgen(js_name = "VerifiedTokenActionWithDocument")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenActionWithDocumentWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub document: DocumentWasm,
}

impl_wasm_type_info!(
    VerifiedTokenActionWithDocumentWasm,
    VerifiedTokenActionWithDocument
);
impl_wasm_conversions_serde!(
    VerifiedTokenActionWithDocumentWasm,
    VerifiedTokenActionWithDocument
);

// --- VerifiedTokenGroupActionWithDocument ---

#[wasm_bindgen(js_name = "VerifiedTokenGroupActionWithDocument")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenGroupActionWithDocumentWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "groupPower")]
    pub group_power: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub document: Option<DocumentWasm>,
}

impl_wasm_type_info!(
    VerifiedTokenGroupActionWithDocumentWasm,
    VerifiedTokenGroupActionWithDocument
);
impl_wasm_conversions_serde!(
    VerifiedTokenGroupActionWithDocumentWasm,
    VerifiedTokenGroupActionWithDocument
);

// --- VerifiedTokenGroupActionWithTokenBalance ---

#[wasm_bindgen(js_name = "VerifiedTokenGroupActionWithTokenBalance")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenGroupActionWithTokenBalanceWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "groupPower")]
    pub group_power: u32,
    #[wasm_bindgen(getter_with_clone, js_name = "actionStatus")]
    pub action_status: String,
    pub(super) balance: Option<u64>,
}

#[wasm_bindgen(js_class = VerifiedTokenGroupActionWithTokenBalance)]
impl VerifiedTokenGroupActionWithTokenBalanceWasm {
    #[wasm_bindgen(getter)]
    pub fn balance(&self) -> JsValue {
        match self.balance {
            Some(b) => BigInt::from(b).into(),
            None => JsValue::undefined(),
        }
    }
}

impl_wasm_type_info!(
    VerifiedTokenGroupActionWithTokenBalanceWasm,
    VerifiedTokenGroupActionWithTokenBalance
);
impl_wasm_conversions_serde!(
    VerifiedTokenGroupActionWithTokenBalanceWasm,
    VerifiedTokenGroupActionWithTokenBalance
);

// --- VerifiedTokenGroupActionWithTokenIdentityInfo ---

#[wasm_bindgen(js_name = "VerifiedTokenGroupActionWithTokenIdentityInfo")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenGroupActionWithTokenIdentityInfoWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "groupPower")]
    pub group_power: u32,
    #[wasm_bindgen(getter_with_clone, js_name = "actionStatus")]
    pub action_status: String,
    #[wasm_bindgen(getter_with_clone, js_name = "tokenInfo")]
    pub token_info: Option<IdentityTokenInfoWasm>,
}

impl_wasm_type_info!(
    VerifiedTokenGroupActionWithTokenIdentityInfoWasm,
    VerifiedTokenGroupActionWithTokenIdentityInfo
);
impl_wasm_conversions_serde!(
    VerifiedTokenGroupActionWithTokenIdentityInfoWasm,
    VerifiedTokenGroupActionWithTokenIdentityInfo
);

// --- VerifiedTokenGroupActionWithTokenPricingSchedule ---

#[wasm_bindgen(js_name = "VerifiedTokenGroupActionWithTokenPricingSchedule")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenGroupActionWithTokenPricingScheduleWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "groupPower")]
    pub group_power: u32,
    #[wasm_bindgen(getter_with_clone, js_name = "actionStatus")]
    pub action_status: String,
    #[wasm_bindgen(getter_with_clone, js_name = "pricingSchedule")]
    pub pricing_schedule: Option<TokenPricingScheduleWasm>,
}

impl_wasm_type_info!(
    VerifiedTokenGroupActionWithTokenPricingScheduleWasm,
    VerifiedTokenGroupActionWithTokenPricingSchedule
);
impl_wasm_conversions_serde!(
    VerifiedTokenGroupActionWithTokenPricingScheduleWasm,
    VerifiedTokenGroupActionWithTokenPricingSchedule
);
