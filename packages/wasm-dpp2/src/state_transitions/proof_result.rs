//! Typed WASM wrappers for `StateTransitionProofResult` variants.
//!
//! Each variant of the Rust `StateTransitionProofResult` enum gets its own
//! `#[wasm_bindgen]` struct with typed getters.  On the TypeScript side they
//! are combined into a discriminated union `StateTransitionProofResultType`
//! (discriminated by the `__type` getter added via `impl_wasm_type_info!`).

use crate::DataContractWasm;
use crate::DocumentWasm;
use crate::IdentifierWasm;
use crate::IdentityTokenInfoWasm;
use crate::IdentityWasm;
use crate::PartialIdentityWasm;
use crate::PlatformAddressWasm;
use crate::PlatformVersionLikeJs;
use crate::TokenStatusWasm;
use crate::VoteWasm;
use crate::data_contract::{DataContractJSONJs, DataContractObjectJs};
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_pricing_schedule::TokenPricingScheduleWasm;
use crate::utils::JsMapExt;
use dpp::document::Document;
use dpp::platform_value::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use js_sys::{BigInt, Map};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

// ============================================================================
// TypeScript union type
// ============================================================================

#[wasm_bindgen(typescript_custom_section)]
const TS_PROOF_RESULT_TYPE: &str = r#"
export type StateTransitionProofResultType =
  | VerifiedDataContract
  | VerifiedIdentity
  | VerifiedTokenBalanceAbsence
  | VerifiedTokenBalance
  | VerifiedTokenIdentityInfo
  | VerifiedTokenPricingSchedule
  | VerifiedTokenStatus
  | VerifiedTokenIdentitiesBalances
  | VerifiedPartialIdentity
  | VerifiedBalanceTransfer
  | VerifiedDocuments
  | VerifiedTokenActionWithDocument
  | VerifiedTokenGroupActionWithDocument
  | VerifiedTokenGroupActionWithTokenBalance
  | VerifiedTokenGroupActionWithTokenIdentityInfo
  | VerifiedTokenGroupActionWithTokenPricingSchedule
  | VerifiedMasternodeVote
  | VerifiedNextDistribution
  | VerifiedAddressInfos
  | VerifiedIdentityFullWithAddressInfos
  | VerifiedIdentityWithAddressInfos;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "StateTransitionProofResultType")]
    pub type StateTransitionProofResultTypeJs;
}

// ============================================================================
// Helper: build a plain JS object from key-value pairs
// ============================================================================

fn js_obj(entries: &[(&str, JsValue)]) -> JsValue {
    let obj = js_sys::Object::new();
    for (key, val) in entries {
        js_sys::Reflect::set(&obj, &(*key).into(), val).unwrap();
    }
    obj.into()
}

// ============================================================================
// Variant structs
// ============================================================================

// --- VerifiedDataContract ---

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

// --- VerifiedIdentity ---

#[wasm_bindgen(js_name = "VerifiedIdentity")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedIdentityWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub identity: IdentityWasm,
}

impl_wasm_type_info!(VerifiedIdentityWasm, VerifiedIdentity);
impl_wasm_conversions!(VerifiedIdentityWasm, VerifiedIdentity);

// --- VerifiedTokenBalanceAbsence ---

#[wasm_bindgen(js_name = "VerifiedTokenBalanceAbsence")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenBalanceAbsenceWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "tokenId")]
    pub token_id: IdentifierWasm,
}

impl_wasm_type_info!(VerifiedTokenBalanceAbsenceWasm, VerifiedTokenBalanceAbsence);
impl_wasm_conversions!(VerifiedTokenBalanceAbsenceWasm, VerifiedTokenBalanceAbsence);

// --- VerifiedTokenBalance ---

#[wasm_bindgen(js_name = "VerifiedTokenBalance")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenBalanceWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "tokenId")]
    pub token_id: IdentifierWasm,
    balance: u64,
}

#[wasm_bindgen(js_class = VerifiedTokenBalance)]
impl VerifiedTokenBalanceWasm {
    #[wasm_bindgen(getter)]
    pub fn balance(&self) -> JsValue {
        BigInt::from(self.balance).into()
    }
}

impl_wasm_type_info!(VerifiedTokenBalanceWasm, VerifiedTokenBalance);
impl_wasm_conversions!(VerifiedTokenBalanceWasm, VerifiedTokenBalance);

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
impl_wasm_conversions!(VerifiedTokenIdentityInfoWasm, VerifiedTokenIdentityInfo);

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
impl_wasm_conversions!(
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
impl_wasm_conversions!(VerifiedTokenStatusWasm, VerifiedTokenStatus);

// --- VerifiedTokenIdentitiesBalances ---

#[wasm_bindgen(js_name = "VerifiedTokenIdentitiesBalances")]
#[derive(Clone)]
pub struct VerifiedTokenIdentitiesBalancesWasm {
    balances: Map, // Map<IdentifierWasm, BigInt>
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

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        self.to_object()
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedTokenIdentitiesBalancesWasm> {
        let map_val = js_sys::Reflect::get(&value, &"balances".into())
            .map_err(|_| WasmDppError::generic("Missing property: balances"))?;
        Ok(VerifiedTokenIdentitiesBalancesWasm {
            balances: map_val.unchecked_into(),
        })
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

// --- VerifiedPartialIdentity ---

#[wasm_bindgen(js_name = "VerifiedPartialIdentity")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedPartialIdentityWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "partialIdentity")]
    pub partial_identity: PartialIdentityWasm,
}

impl_wasm_type_info!(VerifiedPartialIdentityWasm, VerifiedPartialIdentity);
impl_wasm_conversions!(VerifiedPartialIdentityWasm, VerifiedPartialIdentity);

// --- VerifiedBalanceTransfer ---

#[wasm_bindgen(js_name = "VerifiedBalanceTransfer")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedBalanceTransferWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub sender: PartialIdentityWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub recipient: PartialIdentityWasm,
}

impl_wasm_type_info!(VerifiedBalanceTransferWasm, VerifiedBalanceTransfer);
impl_wasm_conversions!(VerifiedBalanceTransferWasm, VerifiedBalanceTransfer);

// --- VerifiedDocuments ---

#[wasm_bindgen(js_name = "VerifiedDocuments")]
#[derive(Clone)]
pub struct VerifiedDocumentsWasm {
    documents: Map, // Map<IdentifierWasm, DocumentWasm | undefined>
}

#[wasm_bindgen(js_class = VerifiedDocuments)]
impl VerifiedDocumentsWasm {
    #[wasm_bindgen(getter)]
    pub fn documents(&self) -> Map {
        self.documents.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[("documents", self.documents.clone().into())])
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        self.to_object()
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedDocumentsWasm> {
        let map_val = js_sys::Reflect::get(&value, &"documents".into())
            .map_err(|_| WasmDppError::generic("Missing property: documents"))?;
        Ok(VerifiedDocumentsWasm {
            documents: map_val.unchecked_into(),
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedDocumentsWasm> {
        Self::from_object(value)
    }
}

impl_wasm_type_info!(VerifiedDocumentsWasm, VerifiedDocuments);

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
impl_wasm_conversions!(
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
impl_wasm_conversions!(
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
    balance: Option<u64>,
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
impl_wasm_conversions!(
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
impl_wasm_conversions!(
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
impl_wasm_conversions!(
    VerifiedTokenGroupActionWithTokenPricingScheduleWasm,
    VerifiedTokenGroupActionWithTokenPricingSchedule
);

// --- VerifiedMasternodeVote ---

#[wasm_bindgen(js_name = "VerifiedMasternodeVote")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedMasternodeVoteWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub vote: VoteWasm,
}

impl_wasm_type_info!(VerifiedMasternodeVoteWasm, VerifiedMasternodeVote);
impl_wasm_conversions!(VerifiedMasternodeVoteWasm, VerifiedMasternodeVote);

// --- VerifiedNextDistribution ---

#[wasm_bindgen(js_name = "VerifiedNextDistribution")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedNextDistributionWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub vote: VoteWasm,
}

impl_wasm_type_info!(VerifiedNextDistributionWasm, VerifiedNextDistribution);
impl_wasm_conversions!(VerifiedNextDistributionWasm, VerifiedNextDistribution);

// --- VerifiedAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedAddressInfos")]
#[derive(Clone)]
pub struct VerifiedAddressInfosWasm {
    address_infos: Map, // Map<string(hex), { address: PlatformAddress, nonce: number, credits: BigInt } | undefined>
}

#[wasm_bindgen(js_class = VerifiedAddressInfos)]
impl VerifiedAddressInfosWasm {
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[("addressInfos", self.address_infos.clone().into())])
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        self.to_object()
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedAddressInfosWasm> {
        let map_val = js_sys::Reflect::get(&value, &"addressInfos".into())
            .map_err(|_| WasmDppError::generic("Missing property: addressInfos"))?;
        Ok(VerifiedAddressInfosWasm {
            address_infos: map_val.unchecked_into(),
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedAddressInfosWasm> {
        Self::from_object(value)
    }
}

impl_wasm_type_info!(VerifiedAddressInfosWasm, VerifiedAddressInfos);

// --- VerifiedIdentityFullWithAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedIdentityFullWithAddressInfos")]
#[derive(Clone)]
pub struct VerifiedIdentityFullWithAddressInfosWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub identity: IdentityWasm,
    address_infos: Map,
}

#[wasm_bindgen(js_class = VerifiedIdentityFullWithAddressInfos)]
impl VerifiedIdentityFullWithAddressInfosWasm {
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let id = self.identity.to_object()?;
        let map_js: JsValue = self.address_infos.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"identity".into(), &id.into()).unwrap();
        js_sys::Reflect::set(&obj, &"addressInfos".into(), &map_js).unwrap();
        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let id = self.identity.to_json()?;
        let map_js: JsValue = self.address_infos.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"identity".into(), &id.into()).unwrap();
        js_sys::Reflect::set(&obj, &"addressInfos".into(), &map_js).unwrap();
        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedIdentityFullWithAddressInfosWasm> {
        let identity_val = js_sys::Reflect::get(&value, &"identity".into())
            .map_err(|_| WasmDppError::generic("Missing property: identity"))?;
        let identity: IdentityWasm = crate::serialization::conversions::from_object(identity_val)?;
        let map_val = js_sys::Reflect::get(&value, &"addressInfos".into())
            .map_err(|_| WasmDppError::generic("Missing property: addressInfos"))?;
        Ok(VerifiedIdentityFullWithAddressInfosWasm {
            identity,
            address_infos: map_val.unchecked_into(),
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedIdentityFullWithAddressInfosWasm> {
        let identity_val = js_sys::Reflect::get(&value, &"identity".into())
            .map_err(|_| WasmDppError::generic("Missing property: identity"))?;
        let identity: IdentityWasm = crate::serialization::conversions::from_json(identity_val)?;
        let map_val = js_sys::Reflect::get(&value, &"addressInfos".into())
            .map_err(|_| WasmDppError::generic("Missing property: addressInfos"))?;
        Ok(VerifiedIdentityFullWithAddressInfosWasm {
            identity,
            address_infos: map_val.unchecked_into(),
        })
    }
}

impl_wasm_type_info!(
    VerifiedIdentityFullWithAddressInfosWasm,
    VerifiedIdentityFullWithAddressInfos
);

// --- VerifiedIdentityWithAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedIdentityWithAddressInfos")]
#[derive(Clone)]
pub struct VerifiedIdentityWithAddressInfosWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "partialIdentity")]
    pub partial_identity: PartialIdentityWasm,
    address_infos: Map,
}

#[wasm_bindgen(js_class = VerifiedIdentityWithAddressInfos)]
impl VerifiedIdentityWithAddressInfosWasm {
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        let pi = self.partial_identity.to_object()?;
        let map_js: JsValue = self.address_infos.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"partialIdentity".into(), &pi.into()).unwrap();
        js_sys::Reflect::set(&obj, &"addressInfos".into(), &map_js).unwrap();
        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let pi = self.partial_identity.to_json()?;
        let map_js: JsValue = self.address_infos.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"partialIdentity".into(), &pi.into()).unwrap();
        js_sys::Reflect::set(&obj, &"addressInfos".into(), &map_js).unwrap();
        Ok(obj.into())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedIdentityWithAddressInfosWasm> {
        let pi_val = js_sys::Reflect::get(&value, &"partialIdentity".into())
            .map_err(|_| WasmDppError::generic("Missing property: partialIdentity"))?;
        let partial_identity: PartialIdentityWasm =
            crate::serialization::conversions::from_object(pi_val)?;
        let map_val = js_sys::Reflect::get(&value, &"addressInfos".into())
            .map_err(|_| WasmDppError::generic("Missing property: addressInfos"))?;
        Ok(VerifiedIdentityWithAddressInfosWasm {
            partial_identity,
            address_infos: map_val.unchecked_into(),
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedIdentityWithAddressInfosWasm> {
        let pi_val = js_sys::Reflect::get(&value, &"partialIdentity".into())
            .map_err(|_| WasmDppError::generic("Missing property: partialIdentity"))?;
        let partial_identity: PartialIdentityWasm =
            crate::serialization::conversions::from_json(pi_val)?;
        let map_val = js_sys::Reflect::get(&value, &"addressInfos".into())
            .map_err(|_| WasmDppError::generic("Missing property: addressInfos"))?;
        Ok(VerifiedIdentityWithAddressInfosWasm {
            partial_identity,
            address_infos: map_val.unchecked_into(),
        })
    }
}

impl_wasm_type_info!(
    VerifiedIdentityWithAddressInfosWasm,
    VerifiedIdentityWithAddressInfos
);

// ============================================================================
// Conversion function
// ============================================================================

/// Wrap a raw `Document` into `DocumentWasm`.
///
/// `DocumentWasm` requires metadata (contract ID, type name) that a bare
/// `Document` does not carry.  When converting proof-result documents we
/// use empty defaults — the actual document data (id, owner_id, properties,
/// revision, timestamps) is fully preserved.
fn doc_to_wasm(doc: Document) -> DocumentWasm {
    DocumentWasm::new(doc, Identifier::default(), String::new(), None)
}

/// Helper to build `Map<string, { address: PlatformAddress, nonce: number, credits: BigInt } | undefined>`
/// from the Rust address-info BTreeMap.  Shared by three variants.
///
/// Keys are hex-encoded PlatformAddress bytes so that JS consumers can
/// look up entries by string (JS Map uses reference equality for object keys).
fn build_address_infos_map(
    map: std::collections::BTreeMap<dpp::address_funds::PlatformAddress, Option<(u32, u64)>>,
) -> Map {
    Map::from_entries(map.into_iter().map(|(address, info)| {
        let address_wasm = PlatformAddressWasm::from(address);
        let key: JsValue = address_wasm.to_hex().into();
        let val: JsValue = match info {
            Some((nonce, credits)) => {
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"address".into(), &address_wasm.into()).unwrap();
                js_sys::Reflect::set(&obj, &"nonce".into(), &nonce.into()).unwrap();
                js_sys::Reflect::set(&obj, &"credits".into(), &BigInt::from(credits).into())
                    .unwrap();
                obj.into()
            }
            None => JsValue::undefined(),
        };
        (key, val)
    }))
}

fn action_status_to_string(status: dpp::group::group_action_status::GroupActionStatus) -> String {
    match status {
        dpp::group::group_action_status::GroupActionStatus::ActionActive => {
            "ActionActive".to_string()
        }
        dpp::group::group_action_status::GroupActionStatus::ActionClosed => {
            "ActionClosed".to_string()
        }
    }
}

/// Convert a Rust `StateTransitionProofResult` into the corresponding typed
/// WASM wrapper, ready to be returned to JavaScript.
pub fn convert_proof_result(
    result: StateTransitionProofResult,
) -> WasmDppResult<StateTransitionProofResultTypeJs> {
    let js_value: JsValue = match result {
        StateTransitionProofResult::VerifiedDataContract(dc) => VerifiedDataContractWasm {
            data_contract: dc.into(),
        }
        .into(),

        StateTransitionProofResult::VerifiedIdentity(identity) => VerifiedIdentityWasm {
            identity: identity.into(),
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenBalanceAbsence(id) => {
            VerifiedTokenBalanceAbsenceWasm {
                token_id: id.into(),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenBalance(id, amount) => VerifiedTokenBalanceWasm {
            token_id: id.into(),
            balance: amount,
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenIdentityInfo(id, info) => {
            VerifiedTokenIdentityInfoWasm {
                token_id: id.into(),
                token_info: info.into(),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenPricingSchedule(id, schedule) => {
            VerifiedTokenPricingScheduleWasm {
                token_id: id.into(),
                pricing_schedule: schedule.map(Into::into),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenStatus(status) => VerifiedTokenStatusWasm {
            token_status: status.into(),
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenIdentitiesBalances(balances) => {
            let map = Map::from_entries(balances.into_iter().map(|(id, amount)| {
                let key: JsValue = IdentifierWasm::from(id).into();
                let val: JsValue = BigInt::from(amount).into();
                (key, val)
            }));
            VerifiedTokenIdentitiesBalancesWasm { balances: map }.into()
        }

        StateTransitionProofResult::VerifiedPartialIdentity(pi) => VerifiedPartialIdentityWasm {
            partial_identity: pi.into(),
        }
        .into(),

        StateTransitionProofResult::VerifiedBalanceTransfer(from, to) => {
            VerifiedBalanceTransferWasm {
                sender: from.into(),
                recipient: to.into(),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedDocuments(docs) => {
            let map = Map::from_entries(docs.into_iter().map(|(id, maybe_doc)| {
                let key: JsValue = IdentifierWasm::from(id).into();
                let val: JsValue = match maybe_doc {
                    Some(doc) => doc_to_wasm(doc).into(),
                    None => JsValue::undefined(),
                };
                (key, val)
            }));
            VerifiedDocumentsWasm { documents: map }.into()
        }

        StateTransitionProofResult::VerifiedTokenActionWithDocument(doc) => {
            VerifiedTokenActionWithDocumentWasm {
                document: doc_to_wasm(doc),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, maybe_doc) => {
            VerifiedTokenGroupActionWithDocumentWasm {
                group_power: power,
                document: maybe_doc.map(doc_to_wasm),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(
            power,
            status,
            maybe_balance,
        ) => VerifiedTokenGroupActionWithTokenBalanceWasm {
            group_power: power,
            action_status: action_status_to_string(status),
            balance: maybe_balance,
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenGroupActionWithTokenIdentityInfo(
            power,
            status,
            maybe_info,
        ) => VerifiedTokenGroupActionWithTokenIdentityInfoWasm {
            group_power: power,
            action_status: action_status_to_string(status),
            token_info: maybe_info.map(Into::into),
        }
        .into(),

        StateTransitionProofResult::VerifiedTokenGroupActionWithTokenPricingSchedule(
            power,
            status,
            maybe_schedule,
        ) => VerifiedTokenGroupActionWithTokenPricingScheduleWasm {
            group_power: power,
            action_status: action_status_to_string(status),
            pricing_schedule: maybe_schedule.map(Into::into),
        }
        .into(),

        StateTransitionProofResult::VerifiedMasternodeVote(vote) => {
            VerifiedMasternodeVoteWasm { vote: vote.into() }.into()
        }

        StateTransitionProofResult::VerifiedNextDistribution(vote) => {
            VerifiedNextDistributionWasm { vote: vote.into() }.into()
        }

        StateTransitionProofResult::VerifiedAddressInfos(infos) => VerifiedAddressInfosWasm {
            address_infos: build_address_infos_map(infos),
        }
        .into(),

        StateTransitionProofResult::VerifiedIdentityFullWithAddressInfos(identity, infos) => {
            VerifiedIdentityFullWithAddressInfosWasm {
                identity: identity.into(),
                address_infos: build_address_infos_map(infos),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedIdentityWithAddressInfos(pi, infos) => {
            VerifiedIdentityWithAddressInfosWasm {
                partial_identity: pi.into(),
                address_infos: build_address_infos_map(infos),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedAssetLockConsumed(_)
        | StateTransitionProofResult::VerifiedShieldedNullifiers(_)
        | StateTransitionProofResult::VerifiedShieldedNullifiersWithAddressInfos(_, _)
        | StateTransitionProofResult::VerifiedShieldedNullifiersWithWithdrawalDocument(_, _) => {
            todo!("shielded proof results not yet implemented in wasm")
        }
    };

    Ok(js_value.into())
}
