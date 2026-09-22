//! `VerifiedDataContract` proof-result wrapper.

use super::helpers::{js_obj, json_safe_credits};
use crate::DataContractWasm;
use crate::IdentifierWasm;
use crate::PlatformVersionLikeJs;
use crate::data_contract::{
    ContractModerationReasonJs, ContractWarningsJs, DataContractJSONJs, DataContractObjectJs,
    moderation_reason_to_js, moderation_warnings_to_js,
};
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use crate::serialization::conversions::normalize_js_value_for_json;
use dpp::data_contract::config::moderation::{ContractModerationReason, ContractWarning};
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
/// on the lists a moderation transition touched (every barring list for a ban, the edited one
/// otherwise). A list the proof does not cover is unknown: `banned` is undefined unless
/// `lists` includes `banlist`, and `warnings` unless it includes `warnings`.
#[wasm_bindgen(js_name = "VerifiedContractModerationListStatuses")]
#[derive(Clone)]
pub struct VerifiedContractModerationListStatusesWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "contractId")]
    pub contract_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "identityId")]
    pub identity_id: IdentifierWasm,
    /// The lists the proof covers: `banlist` and `suspensions` for a ban, else the one edited
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
    /// When `lists` includes `warnings`: the identity's warnings, oldest first, empty when it
    /// carries none
    #[wasm_bindgen(skip)]
    pub warnings: Option<Vec<ContractWarning>>,
}

impl VerifiedContractModerationListStatusesWasm {
    fn warnings_or_undefined(&self, as_json: bool) -> JsValue {
        self.warnings
            .as_deref()
            .map(|warnings| moderation_warnings_to_js(warnings, as_json))
            .unwrap_or(JsValue::UNDEFINED)
    }

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

    /// When `lists` includes `warnings`: the identity's warnings, oldest first
    #[wasm_bindgen(getter = "warnings")]
    pub fn warnings(&self) -> Option<ContractWarningsJs> {
        self.warnings
            .as_deref()
            .map(|warnings| moderation_warnings_to_js(warnings, false).into())
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
            ("warnings", self.warnings_or_undefined(false)),
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
            ("warnings", self.warnings_or_undefined(true)),
        ]))
    }
}

impl_wasm_type_info!(
    VerifiedContractModerationListStatusesWasm,
    VerifiedContractModerationListStatuses
);

/// `VerifiedContractFeeClaim` proof-result wrapper: the pot a contract fee claim paid out (the
/// contract, the pot, its last claim, the credits left in it) and the balance of every identity
/// the claim paid, after the claim. A pot is paid out at most once per epoch, so while
/// `lastClaimEpoch` is the epoch of the claim the proof is of that claim, and `lastClaimantId`
/// and `lastClaimTimeMs` are its own.
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
    pub(super) last_claim_time_ms: u64,
    /// The identity that signed the last claim of the pot
    #[wasm_bindgen(getter_with_clone, js_name = "lastClaimantId")]
    pub last_claimant_id: IdentifierWasm,
    pub(super) remaining_credits: u64,
    pub(super) balances: Map, // Map<string(base58), BigInt>
}

#[wasm_bindgen(js_class = VerifiedContractFeeClaim)]
impl VerifiedContractFeeClaimWasm {
    /// The time, in milliseconds, of the block the pot was last paid out in
    #[wasm_bindgen(getter = "lastClaimTimeMs")]
    pub fn last_claim_time_ms(&self) -> JsValue {
        BigInt::from(self.last_claim_time_ms).into()
    }

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
                "lastClaimTimeMs",
                BigInt::from(self.last_claim_time_ms).into(),
            ),
            ("lastClaimantId", self.last_claimant_id.into()),
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
            // A block time in milliseconds stays exact as a JavaScript number.
            (
                "lastClaimTimeMs",
                JsValue::from_f64(self.last_claim_time_ms as f64),
            ),
            (
                "lastClaimantId",
                JsValue::from_str(&self.last_claimant_id.to_base58()),
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

impl_wasm_type_info!(VerifiedContractFeeClaimWasm, VerifiedContractFeeClaim);

/// `VerifiedContractDocumentRemoval` proof-result wrapper: the record a moderator's document
/// deletion left under the contract, as a deletion or a restore leaves it. After a deletion
/// the document is gone and the record says whose it was, who removed it, why, when and what
/// it was (its hash); after a restore the document is live again and the record also says who
/// brought it back and when.
#[wasm_bindgen(js_name = "VerifiedContractDocumentRemoval")]
#[derive(Clone)]
pub struct VerifiedContractDocumentRemovalWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "contractId")]
    pub contract_id: IdentifierWasm,
    #[wasm_bindgen(getter_with_clone, js_name = "documentTypeName")]
    pub document_type_name: String,
    #[wasm_bindgen(getter_with_clone, js_name = "documentId")]
    pub document_id: IdentifierWasm,
    /// The identity that owned the document when it was removed
    #[wasm_bindgen(getter_with_clone, js_name = "documentOwnerId")]
    pub document_owner_id: IdentifierWasm,
    /// The contract owner or moderator that removed it
    #[wasm_bindgen(getter_with_clone, js_name = "moderatorId")]
    pub moderator_id: IdentifierWasm,
    #[wasm_bindgen(skip)]
    pub reason: ContractModerationReason,
    /// The time of the block that removed it, in milliseconds
    #[wasm_bindgen(js_name = "removedAt")]
    pub removed_at: u64,
    #[wasm_bindgen(skip)]
    pub document_hash: [u8; 32],
    /// The contract owner or moderator that restored the document, undefined while the
    /// removal stands
    #[wasm_bindgen(getter_with_clone, js_name = "restoredBy")]
    pub restored_by: Option<IdentifierWasm>,
    /// The time of the block that restored it, in milliseconds, undefined while the removal
    /// stands
    #[wasm_bindgen(js_name = "restoredAt")]
    pub restored_at: Option<u64>,
}

#[wasm_bindgen(js_class = VerifiedContractDocumentRemoval)]
impl VerifiedContractDocumentRemovalWasm {
    /// Why the moderator removed the document: the text may be empty
    #[wasm_bindgen(getter = "reason")]
    pub fn reason(&self) -> ContractModerationReasonJs {
        moderation_reason_to_js(&self.reason).into()
    }

    /// A double SHA-256 of the document as it was serialized under its type when it was
    /// removed: what a restore must bring back byte for byte, as 64 hex characters
    #[wasm_bindgen(getter = "documentHash")]
    pub fn document_hash(&self) -> String {
        hex::encode(self.document_hash)
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        Ok(js_obj(&[
            ("contractId", self.contract_id.into()),
            (
                "documentTypeName",
                JsValue::from_str(&self.document_type_name),
            ),
            ("documentId", self.document_id.into()),
            ("documentOwnerId", self.document_owner_id.into()),
            ("moderatorId", self.moderator_id.into()),
            ("reason", moderation_reason_to_js(&self.reason)),
            (
                "removedAt",
                JsValue::from(js_sys::BigInt::from(self.removed_at)),
            ),
            (
                "documentHash",
                JsValue::from_str(&hex::encode(self.document_hash)),
            ),
            (
                "restoredBy",
                self.restored_by
                    .map_or(JsValue::UNDEFINED, |restored_by| restored_by.into()),
            ),
            (
                "restoredAt",
                self.restored_at.map_or(JsValue::UNDEFINED, |restored_at| {
                    JsValue::from(js_sys::BigInt::from(restored_at))
                }),
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
                "documentTypeName",
                JsValue::from_str(&self.document_type_name),
            ),
            (
                "documentId",
                JsValue::from_str(&self.document_id.to_base58()),
            ),
            (
                "documentOwnerId",
                JsValue::from_str(&self.document_owner_id.to_base58()),
            ),
            (
                "moderatorId",
                JsValue::from_str(&self.moderator_id.to_base58()),
            ),
            ("reason", moderation_reason_to_js(&self.reason)),
            ("removedAt", JsValue::from_f64(self.removed_at as f64)),
            (
                "documentHash",
                JsValue::from_str(&hex::encode(self.document_hash)),
            ),
            (
                "restoredBy",
                self.restored_by.map_or(JsValue::UNDEFINED, |restored_by| {
                    JsValue::from_str(&restored_by.to_base58())
                }),
            ),
            (
                "restoredAt",
                self.restored_at.map_or(JsValue::UNDEFINED, |restored_at| {
                    JsValue::from_f64(restored_at as f64)
                }),
            ),
        ]))
    }
}

impl_wasm_type_info!(
    VerifiedContractDocumentRemovalWasm,
    VerifiedContractDocumentRemoval
);
