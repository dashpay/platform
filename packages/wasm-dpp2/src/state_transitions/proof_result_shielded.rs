//! Shielded pool WASM wrappers for `StateTransitionProofResult` variants.
//!
//! These types were extracted from `proof_result` to keep shielded-specific
//! code in its own module.

use crate::impl_wasm_conversions_serde;
use crate::impl_wasm_type_info;
use js_sys::{BigInt, Map};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

use super::proof_result::js_obj;

// --- VerifiedShieldedPoolState ---

#[wasm_bindgen(js_name = "VerifiedShieldedPoolState")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedShieldedPoolStateWasm {
    pool_balance: Option<u64>,
}

impl_wasm_type_info!(VerifiedShieldedPoolStateWasm, VerifiedShieldedPoolState);
impl_wasm_conversions_serde!(VerifiedShieldedPoolStateWasm, VerifiedShieldedPoolState);

#[wasm_bindgen(js_class = VerifiedShieldedPoolState)]
impl VerifiedShieldedPoolStateWasm {
    #[wasm_bindgen(getter, js_name = "poolBalance")]
    pub fn pool_balance(&self) -> JsValue {
        match self.pool_balance {
            Some(b) => BigInt::from(b).into(),
            None => JsValue::undefined(),
        }
    }
}

// --- VerifiedAssetLockConsumed ---

#[wasm_bindgen(js_name = "VerifiedAssetLockConsumed")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedAssetLockConsumedWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub status: String,
    initial_credit_value: Option<u64>,
    remaining_credit_value: Option<u64>,
}

#[wasm_bindgen(js_class = VerifiedAssetLockConsumed)]
impl VerifiedAssetLockConsumedWasm {
    #[wasm_bindgen(getter, js_name = "initialCreditValue")]
    pub fn initial_credit_value(&self) -> JsValue {
        match self.initial_credit_value {
            Some(v) => BigInt::from(v).into(),
            None => JsValue::undefined(),
        }
    }

    #[wasm_bindgen(getter, js_name = "remainingCreditValue")]
    pub fn remaining_credit_value(&self) -> JsValue {
        match self.remaining_credit_value {
            Some(v) => BigInt::from(v).into(),
            None => JsValue::undefined(),
        }
    }
}

impl VerifiedAssetLockConsumedWasm {
    pub fn new(
        status: String,
        initial_credit_value: Option<u64>,
        remaining_credit_value: Option<u64>,
    ) -> Self {
        Self {
            status,
            initial_credit_value,
            remaining_credit_value,
        }
    }
}

impl_wasm_type_info!(VerifiedAssetLockConsumedWasm, VerifiedAssetLockConsumed);
impl_wasm_conversions_serde!(VerifiedAssetLockConsumedWasm, VerifiedAssetLockConsumed);

// --- VerifiedShieldedNullifiers ---

#[wasm_bindgen(js_name = "VerifiedShieldedNullifiers")]
#[derive(Clone)]
pub struct VerifiedShieldedNullifiersWasm {
    nullifiers: Map, // Map<hex(nullifier), boolean>
}

#[wasm_bindgen(js_class = VerifiedShieldedNullifiers)]
impl VerifiedShieldedNullifiersWasm {
    #[wasm_bindgen(getter)]
    pub fn nullifiers(&self) -> Map {
        self.nullifiers.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[("nullifiers", self.nullifiers.clone().into())])
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        self.to_object()
    }
}

impl VerifiedShieldedNullifiersWasm {
    pub fn from_map(nullifiers: Map) -> Self {
        Self { nullifiers }
    }
}

impl_wasm_type_info!(VerifiedShieldedNullifiersWasm, VerifiedShieldedNullifiers);

// --- VerifiedShieldedNullifiersWithAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedShieldedNullifiersWithAddressInfos")]
#[derive(Clone)]
pub struct VerifiedShieldedNullifiersWithAddressInfosWasm {
    nullifiers: Map,
    address_infos: Map,
}

#[wasm_bindgen(js_class = VerifiedShieldedNullifiersWithAddressInfos)]
impl VerifiedShieldedNullifiersWithAddressInfosWasm {
    #[wasm_bindgen(getter)]
    pub fn nullifiers(&self) -> Map {
        self.nullifiers.clone()
    }

    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[
            ("nullifiers", self.nullifiers.clone().into()),
            ("addressInfos", self.address_infos.clone().into()),
        ])
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        self.to_object()
    }
}

impl VerifiedShieldedNullifiersWithAddressInfosWasm {
    pub fn new(nullifiers: Map, address_infos: Map) -> Self {
        Self {
            nullifiers,
            address_infos,
        }
    }
}

impl_wasm_type_info!(
    VerifiedShieldedNullifiersWithAddressInfosWasm,
    VerifiedShieldedNullifiersWithAddressInfos
);

// --- VerifiedShieldedNullifiersWithWithdrawalDocument ---

#[wasm_bindgen(js_name = "VerifiedShieldedNullifiersWithWithdrawalDocument")]
#[derive(Clone)]
pub struct VerifiedShieldedNullifiersWithWithdrawalDocumentWasm {
    nullifiers: Map,
    documents: Map,
}

#[wasm_bindgen(js_class = VerifiedShieldedNullifiersWithWithdrawalDocument)]
impl VerifiedShieldedNullifiersWithWithdrawalDocumentWasm {
    #[wasm_bindgen(getter)]
    pub fn nullifiers(&self) -> Map {
        self.nullifiers.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn documents(&self) -> Map {
        self.documents.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[
            ("nullifiers", self.nullifiers.clone().into()),
            ("documents", self.documents.clone().into()),
        ])
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        self.to_object()
    }
}

impl VerifiedShieldedNullifiersWithWithdrawalDocumentWasm {
    pub fn new(nullifiers: Map, documents: Map) -> Self {
        Self {
            nullifiers,
            documents,
        }
    }
}

impl_wasm_type_info!(
    VerifiedShieldedNullifiersWithWithdrawalDocumentWasm,
    VerifiedShieldedNullifiersWithWithdrawalDocument
);
