//! Identity-related `StateTransitionProofResult` wrappers.
//!
//! Contains `VerifiedIdentity`, `VerifiedPartialIdentity`, and
//! `VerifiedBalanceTransfer`.

use crate::IdentityWasm;
use crate::PartialIdentityWasm;
use crate::impl_wasm_conversions_serde;
use crate::impl_wasm_type_info;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// --- VerifiedIdentity ---

#[wasm_bindgen(js_name = "VerifiedIdentity")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedIdentityWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub identity: IdentityWasm,
}

impl_wasm_type_info!(VerifiedIdentityWasm, VerifiedIdentity);
impl_wasm_conversions_serde!(VerifiedIdentityWasm, VerifiedIdentity);

// --- VerifiedPartialIdentity ---

#[wasm_bindgen(js_name = "VerifiedPartialIdentity")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedPartialIdentityWasm {
    #[wasm_bindgen(getter_with_clone, js_name = "partialIdentity")]
    pub partial_identity: PartialIdentityWasm,
}

impl_wasm_type_info!(VerifiedPartialIdentityWasm, VerifiedPartialIdentity);
impl_wasm_conversions_serde!(VerifiedPartialIdentityWasm, VerifiedPartialIdentity);

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
impl_wasm_conversions_serde!(VerifiedBalanceTransferWasm, VerifiedBalanceTransfer);
