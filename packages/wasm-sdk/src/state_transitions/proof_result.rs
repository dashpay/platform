//! Typed WASM wrappers for `StateTransitionProofResult` variants.
//!
//! Each variant of the Rust `StateTransitionProofResult` enum gets its own
//! `#[wasm_bindgen]` struct with typed getters.  On the TypeScript side they
//! are combined into a discriminated union `StateTransitionProofResultType`
//! (discriminated by the `__type` getter added via `impl_wasm_type_info!`).

use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use js_sys::{BigInt, Map};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_dpp2::impl_wasm_type_info;
use wasm_dpp2::state_transitions::batch::token_pricing_schedule::TokenPricingScheduleWasm;
use wasm_dpp2::utils::JsMapExt;
use wasm_dpp2::DataContractWasm;
use wasm_dpp2::DocumentWasm;
use wasm_dpp2::IdentifierWasm;
use wasm_dpp2::IdentityTokenInfoWasm;
use wasm_dpp2::IdentityWasm;
use wasm_dpp2::PartialIdentityWasm;
use wasm_dpp2::PlatformAddressWasm;
use wasm_dpp2::TokenStatusWasm;
use wasm_dpp2::VoteWasm;

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
// Variant structs
// ============================================================================

// --- VerifiedDataContract ---

#[wasm_bindgen(js_name = "VerifiedDataContract")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedDataContractWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "dataContract")]
    pub data_contract: DataContractWasm,
}

impl_wasm_type_info!(VerifiedDataContractWasm, VerifiedDataContract);
impl_wasm_serde_conversions!(VerifiedDataContractWasm, VerifiedDataContract);

// --- VerifiedIdentity ---

#[wasm_bindgen(js_name = "VerifiedIdentity")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedIdentityWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub identity: IdentityWasm,
}

impl_wasm_type_info!(VerifiedIdentityWasm, VerifiedIdentity);
impl_wasm_serde_conversions!(VerifiedIdentityWasm, VerifiedIdentity);

// --- VerifiedTokenBalanceAbsence ---

#[wasm_bindgen(js_name = "VerifiedTokenBalanceAbsence")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenBalanceAbsenceWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "tokenId")]
    pub token_id: IdentifierWasm,
}

impl_wasm_type_info!(VerifiedTokenBalanceAbsenceWasm, VerifiedTokenBalanceAbsence);
impl_wasm_serde_conversions!(VerifiedTokenBalanceAbsenceWasm, VerifiedTokenBalanceAbsence);

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
impl_wasm_serde_conversions!(VerifiedTokenBalanceWasm, VerifiedTokenBalance);

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
impl_wasm_serde_conversions!(VerifiedTokenIdentityInfoWasm, VerifiedTokenIdentityInfo);

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
impl_wasm_serde_conversions!(
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
impl_wasm_serde_conversions!(VerifiedTokenStatusWasm, VerifiedTokenStatus);

// --- VerifiedTokenIdentitiesBalances ---

#[wasm_bindgen(js_name = "VerifiedTokenIdentitiesBalances")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedTokenIdentitiesBalancesWasm {
    #[serde(skip)]
    balances: Option<Map>, // Map<IdentifierWasm, BigInt>
}

#[wasm_bindgen(js_class = VerifiedTokenIdentitiesBalances)]
impl VerifiedTokenIdentitiesBalancesWasm {
    #[wasm_bindgen(getter)]
    pub fn balances(&self) -> Option<Map> {
        self.balances.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        let obj = js_sys::Object::new();
        if let Some(ref map) = self.balances {
            js_sys::Reflect::set(&obj, &"balances".into(), map).unwrap();
        }
        obj.into()
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        self.to_object()
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
impl_wasm_serde_conversions!(VerifiedPartialIdentityWasm, VerifiedPartialIdentity);

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
impl_wasm_serde_conversions!(VerifiedBalanceTransferWasm, VerifiedBalanceTransfer);

// --- VerifiedDocuments ---

#[wasm_bindgen(js_name = "VerifiedDocuments")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedDocumentsWasm {
    #[serde(skip)]
    documents: Option<Map>, // Map<IdentifierWasm, DocumentWasm | null>
}

#[wasm_bindgen(js_class = VerifiedDocuments)]
impl VerifiedDocumentsWasm {
    #[wasm_bindgen(getter)]
    pub fn documents(&self) -> Option<Map> {
        self.documents.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        let obj = js_sys::Object::new();
        if let Some(ref map) = self.documents {
            js_sys::Reflect::set(&obj, &"documents".into(), map).unwrap();
        }
        obj.into()
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        self.to_object()
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
impl_wasm_serde_conversions!(
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
impl_wasm_serde_conversions!(
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
            None => JsValue::NULL,
        }
    }
}

impl_wasm_type_info!(
    VerifiedTokenGroupActionWithTokenBalanceWasm,
    VerifiedTokenGroupActionWithTokenBalance
);
impl_wasm_serde_conversions!(
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
impl_wasm_serde_conversions!(
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
impl_wasm_serde_conversions!(
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
impl_wasm_serde_conversions!(VerifiedMasternodeVoteWasm, VerifiedMasternodeVote);

// --- VerifiedNextDistribution ---

#[wasm_bindgen(js_name = "VerifiedNextDistribution")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedNextDistributionWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub vote: VoteWasm,
}

impl_wasm_type_info!(VerifiedNextDistributionWasm, VerifiedNextDistribution);
impl_wasm_serde_conversions!(VerifiedNextDistributionWasm, VerifiedNextDistribution);

// --- VerifiedAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedAddressInfos")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedAddressInfosWasm {
    #[serde(skip)]
    address_infos: Option<Map>, // Map<PlatformAddressWasm, { nonce: number, credits: BigInt } | null>
}

#[wasm_bindgen(js_class = VerifiedAddressInfos)]
impl VerifiedAddressInfosWasm {
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Option<Map> {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        let obj = js_sys::Object::new();
        if let Some(ref map) = self.address_infos {
            js_sys::Reflect::set(&obj, &"addressInfos".into(), map).unwrap();
        }
        obj.into()
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        self.to_object()
    }
}

impl_wasm_type_info!(VerifiedAddressInfosWasm, VerifiedAddressInfos);

// --- VerifiedIdentityFullWithAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedIdentityFullWithAddressInfos")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedIdentityFullWithAddressInfosWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub identity: IdentityWasm,
    #[serde(skip)]
    address_infos: Option<Map>,
}

#[wasm_bindgen(js_class = VerifiedIdentityFullWithAddressInfos)]
impl VerifiedIdentityFullWithAddressInfosWasm {
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Option<Map> {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, WasmSdkError> {
        let obj = wasm_dpp2::serialization::to_object(self)
            .map_err(WasmSdkError::from)?;
        if let Some(ref map) = self.address_infos {
            js_sys::Reflect::set(&obj, &"addressInfos".into(), map).unwrap();
        }
        Ok(obj)
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, WasmSdkError> {
        self.to_object()
    }
}

impl_wasm_type_info!(
    VerifiedIdentityFullWithAddressInfosWasm,
    VerifiedIdentityFullWithAddressInfos
);

// --- VerifiedIdentityWithAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedIdentityWithAddressInfos")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedIdentityWithAddressInfosWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "partialIdentity")]
    pub partial_identity: PartialIdentityWasm,
    #[serde(skip)]
    address_infos: Option<Map>,
}

#[wasm_bindgen(js_class = VerifiedIdentityWithAddressInfos)]
impl VerifiedIdentityWithAddressInfosWasm {
    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Option<Map> {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, WasmSdkError> {
        let obj = wasm_dpp2::serialization::to_object(self)
            .map_err(WasmSdkError::from)?;
        if let Some(ref map) = self.address_infos {
            js_sys::Reflect::set(&obj, &"addressInfos".into(), map).unwrap();
        }
        Ok(obj)
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, WasmSdkError> {
        self.to_object()
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

/// Helper to build `Map<PlatformAddressWasm, { nonce: number, credits: BigInt } | null>`
/// from the Rust address-info BTreeMap.  Shared by three variants.
fn build_address_infos_map(
    map: std::collections::BTreeMap<
        dash_sdk::dpp::address_funds::PlatformAddress,
        Option<(u32, u64)>,
    >,
) -> Map {
    Map::from_entries(map.into_iter().map(|(address, info)| {
        let key: JsValue = PlatformAddressWasm::from(address).into();
        let val: JsValue = match info {
            Some((nonce, credits)) => {
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"nonce".into(), &nonce.into()).unwrap();
                js_sys::Reflect::set(&obj, &"credits".into(), &BigInt::from(credits).into())
                    .unwrap();
                obj.into()
            }
            None => JsValue::NULL,
        };
        (key, val)
    }))
}

fn action_status_to_string(
    status: dash_sdk::dpp::group::group_action_status::GroupActionStatus,
) -> String {
    match status {
        dash_sdk::dpp::group::group_action_status::GroupActionStatus::ActionActive => {
            "ActionActive".to_string()
        }
        dash_sdk::dpp::group::group_action_status::GroupActionStatus::ActionClosed => {
            "ActionClosed".to_string()
        }
    }
}

/// Convert a Rust `StateTransitionProofResult` into the corresponding typed
/// WASM wrapper, ready to be returned to JavaScript.
pub fn convert_proof_result(
    result: StateTransitionProofResult,
) -> Result<StateTransitionProofResultTypeJs, WasmSdkError> {
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
            VerifiedTokenIdentitiesBalancesWasm {
                balances: Some(map),
            }
            .into()
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
                    None => JsValue::NULL,
                };
                (key, val)
            }));
            VerifiedDocumentsWasm {
                documents: Some(map),
            }
            .into()
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

        StateTransitionProofResult::VerifiedMasternodeVote(vote) => VerifiedMasternodeVoteWasm {
            vote: vote.into(),
        }
        .into(),

        StateTransitionProofResult::VerifiedNextDistribution(vote) => {
            VerifiedNextDistributionWasm {
                vote: vote.into(),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedAddressInfos(infos) => VerifiedAddressInfosWasm {
            address_infos: Some(build_address_infos_map(infos)),
        }
        .into(),

        StateTransitionProofResult::VerifiedIdentityFullWithAddressInfos(identity, infos) => {
            VerifiedIdentityFullWithAddressInfosWasm {
                identity: identity.into(),
                address_infos: Some(build_address_infos_map(infos)),
            }
            .into()
        }

        StateTransitionProofResult::VerifiedIdentityWithAddressInfos(pi, infos) => {
            VerifiedIdentityWithAddressInfosWasm {
                partial_identity: pi.into(),
                address_infos: Some(build_address_infos_map(infos)),
            }
            .into()
        }
    };

    Ok(js_value.into())
}
