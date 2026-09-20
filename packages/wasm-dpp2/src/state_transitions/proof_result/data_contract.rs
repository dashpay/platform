//! `VerifiedDataContract` proof-result wrapper.

use super::helpers::js_obj;
use crate::DataContractWasm;
use crate::IdentifierWasm;
use crate::PlatformVersionLikeJs;
use crate::data_contract::{
    ContractModerationReasonJs, DataContractJSONJs, DataContractObjectJs, moderation_reason_to_js,
};
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use crate::serialization::conversions::normalize_js_value_for_json;
use dpp::data_contract::config::moderation::ContractModerationReason;
use js_sys::{BigInt, Map};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "VerifiedDataContract")]
#[derive(Clone)]
pub struct VerifiedDataContractWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "dataContract")]
    pub data_contract: DataContractWasm,
}

#[wasm_bindgen(js_class = VerifiedDataContract)]
impl VerifiedDataContractWasm {
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(
        &self,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<JsValue> {
        let dc = self.data_contract.to_object(platform_version)?;
        Ok(js_obj(&[("dataContract", dc.into())]))
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(
        &self,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<JsValue> {
        let dc = self.data_contract.to_json(platform_version)?;
        Ok(js_obj(&[("dataContract", dc.into())]))
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(
        value: JsValue,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<VerifiedDataContractWasm> {
        let dc_val = js_sys::Reflect::get(&value, &"dataContract".into())
            .map_err(|_| WasmDppError::generic("Missing property: dataContract"))?;
        let data_contract = DataContractWasm::from_object(
            dc_val.unchecked_into::<DataContractObjectJs>(),
            false,
            platform_version,
        )?;
        Ok(VerifiedDataContractWasm { data_contract })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(
        value: JsValue,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<VerifiedDataContractWasm> {
        let dc_val = js_sys::Reflect::get(&value, &"dataContract".into())
            .map_err(|_| WasmDppError::generic("Missing property: dataContract"))?;
        let data_contract = DataContractWasm::from_json(
            dc_val.unchecked_into::<DataContractJSONJs>(),
            false,
            platform_version,
        )?;
        Ok(VerifiedDataContractWasm { data_contract })
    }
}

impl_wasm_type_info!(VerifiedDataContractWasm, VerifiedDataContract);

/// `VerifiedContractModerationListStatuses` proof-result wrapper: the target identity's status
/// on the lists a moderation transition touched (both for a ban, the edited one otherwise).
/// A list the proof does not cover is unknown: `banned` is undefined unless `lists` includes
/// `banlist`.
#[wasm_bindgen(js_name = "VerifiedContractModerationListStatuses")]
#[derive(Clone)]
pub struct VerifiedContractModerationListStatusesWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "contractId")]
    pub contract_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "identityId")]
    pub identity_id: IdentifierWasm,
    /// The lists the proof covers: `banlist`, `suspensions`, or both
    #[wasm_bindgen(getter_with_clone)]
    pub lists: Vec<String>,
    /// When `lists` includes `banlist`: the identity is on the banlist
    pub banned: Option<bool>,
    #[wasm_bindgen(skip)]
    pub ban_reason: Option<ContractModerationReason>,
    /// When `lists` includes `suspensions`: the block time, in milliseconds, until which the
    /// identity is suspended
    #[wasm_bindgen(js_name = "suspendedUntil")]
    pub suspended_until: Option<u64>,
    #[wasm_bindgen(skip)]
    pub suspension_reason: Option<ContractModerationReason>,
}

impl VerifiedContractModerationListStatusesWasm {
    fn reason_or_undefined(reason: &Option<ContractModerationReason>) -> JsValue {
        reason
            .as_ref()
            .map(moderation_reason_to_js)
            .unwrap_or(JsValue::UNDEFINED)
    }
}

#[wasm_bindgen(js_class = VerifiedContractModerationListStatuses)]
impl VerifiedContractModerationListStatusesWasm {
    /// When the identity is banned: why
    #[wasm_bindgen(getter = "banReason")]
    pub fn ban_reason(&self) -> Option<ContractModerationReasonJs> {
        self.ban_reason
            .as_ref()
            .map(|reason| moderation_reason_to_js(reason).into())
    }

    /// When the identity is suspended: why
    #[wasm_bindgen(getter = "suspensionReason")]
    pub fn suspension_reason(&self) -> Option<ContractModerationReasonJs> {
        self.suspension_reason
            .as_ref()
            .map(|reason| moderation_reason_to_js(reason).into())
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        Ok(js_obj(&[
            ("contractId", self.contract_id.into()),
            ("identityId", self.identity_id.into()),
            (
                "lists",
                self.lists
                    .iter()
                    .map(|list| JsValue::from_str(list))
                    .collect::<js_sys::Array>()
                    .into(),
            ),
            (
                "banned",
                self.banned
                    .map(JsValue::from_bool)
                    .unwrap_or(JsValue::UNDEFINED),
            ),
            ("banReason", Self::reason_or_undefined(&self.ban_reason)),
            (
                "suspendedUntil",
                self.suspended_until
                    .map(|until| JsValue::from(js_sys::BigInt::from(until)))
                    .unwrap_or(JsValue::UNDEFINED),
            ),
            (
                "suspensionReason",
                Self::reason_or_undefined(&self.suspension_reason),
            ),
        ]))
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        Ok(js_obj(&[
            (
                "contractId",
                JsValue::from_str(&self.contract_id.to_base58()),
            ),
            (
                "identityId",
                JsValue::from_str(&self.identity_id.to_base58()),
            ),
            (
                "lists",
                self.lists
                    .iter()
                    .map(|list| JsValue::from_str(list))
                    .collect::<js_sys::Array>()
                    .into(),
            ),
            (
                "banned",
                self.banned
                    .map(JsValue::from_bool)
                    .unwrap_or(JsValue::UNDEFINED),
            ),
            ("banReason", Self::reason_or_undefined(&self.ban_reason)),
            (
                "suspendedUntil",
                self.suspended_until
                    .map(|until| JsValue::from_f64(until as f64))
                    .unwrap_or(JsValue::UNDEFINED),
            ),
            (
                "suspensionReason",
                Self::reason_or_undefined(&self.suspension_reason),
            ),
        ]))
    }
}

impl_wasm_type_info!(
    VerifiedContractModerationListStatusesWasm,
    VerifiedContractModerationListStatuses
);

/// `VerifiedContractFeeClaim` proof-result wrapper: the pot a contract fee claim paid out (the
/// contract, the pot, the epoch the pot was last claimed in, the credits left in it) and the
/// balance of every identity the claim paid, after the claim. A pot is paid out at most once
/// per epoch, so while `lastClaimEpoch` is the epoch of the claim the proof is of that claim.
#[wasm_bindgen(js_name = "VerifiedContractFeeClaim")]
#[derive(Clone)]
pub struct VerifiedContractFeeClaimWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "contractId")]
    pub contract_id: IdentifierWasm,
    /// The pot that was paid out: `owner` or `moderators`
    #[wasm_bindgen(getter_with_clone)]
    pub pot: String,
    /// The epoch the pot was last claimed in: this claim's own, unless the pot was claimed
    /// again since
    #[wasm_bindgen(js_name = "lastClaimEpoch")]
    pub last_claim_epoch: u16,
    pub(super) remaining_credits: u64,
    pub(super) balances: Map, // Map<string(base58), BigInt>
}

#[wasm_bindgen(js_class = VerifiedContractFeeClaim)]
impl VerifiedContractFeeClaimWasm {
    /// The credits left in the pot after the claim
    #[wasm_bindgen(getter = "remainingCredits")]
    pub fn remaining_credits(&self) -> JsValue {
        BigInt::from(self.remaining_credits).into()
    }

    /// The balance of every identity the claim paid, after the claim
    #[wasm_bindgen(getter)]
    pub fn balances(&self) -> Map {
        self.balances.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        Ok(js_obj(&[
            ("contractId", self.contract_id.into()),
            ("pot", JsValue::from_str(&self.pot)),
            (
                "lastClaimEpoch",
                JsValue::from_f64(self.last_claim_epoch as f64),
            ),
            (
                "remainingCredits",
                BigInt::from(self.remaining_credits).into(),
            ),
            ("balances", self.balances.clone().into()),
        ]))
    }

    /// Returns a `JSON.stringify`-friendly form: the `balances` `Map` is normalised to a plain
    /// object so its entries survive serialisation (otherwise `JSON.stringify({balances: <Map>})`
    /// produces `{"balances":{}}`).
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        Ok(js_obj(&[
            (
                "contractId",
                JsValue::from_str(&self.contract_id.to_base58()),
            ),
            ("pot", JsValue::from_str(&self.pot)),
            (
                "lastClaimEpoch",
                JsValue::from_f64(self.last_claim_epoch as f64),
            ),
            (
                "remainingCredits",
                json_safe_credits(self.remaining_credits),
            ),
            (
                "balances",
                normalize_js_value_for_json(&self.balances.clone().into())?,
            ),
        ]))
    }
}

/// Credits in JSON: a number while it is exact in JavaScript, a decimal string past
/// `Number.MAX_SAFE_INTEGER`, where a number would silently round.
fn json_safe_credits(credits: u64) -> JsValue {
    const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;
    if credits <= MAX_SAFE_INTEGER {
        JsValue::from_f64(credits as f64)
    } else {
        JsValue::from_str(&credits.to_string())
    }
}

impl_wasm_type_info!(VerifiedContractFeeClaimWasm, VerifiedContractFeeClaim);
