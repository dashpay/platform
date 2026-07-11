//! Shielded pool WASM wrappers for `StateTransitionProofResult` variants.
//!
//! These types were extracted from `proof_result` to keep shielded-specific
//! code in its own module.

use super::helpers::{js_obj, read_map_property};
use crate::IdentityWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_conversions_serde;
use crate::impl_wasm_type_info;
use crate::serialization::conversions::normalize_js_value_for_json;
use js_sys::{BigInt, Map};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

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

// --- VerifiedAssetLockConsumedWithAddressInfos ---

#[wasm_bindgen(js_name = "VerifiedAssetLockConsumedWithAddressInfos")]
#[derive(Clone)]
pub struct VerifiedAssetLockConsumedWithAddressInfosWasm {
    status: String,
    initial_credit_value: Option<u64>,
    remaining_credit_value: Option<u64>,
    address_infos: Map,
}

#[wasm_bindgen(js_class = VerifiedAssetLockConsumedWithAddressInfos)]
impl VerifiedAssetLockConsumedWithAddressInfosWasm {
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> String {
        self.status.clone()
    }

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

    #[wasm_bindgen(getter = "addressInfos")]
    pub fn address_infos(&self) -> Map {
        self.address_infos.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> JsValue {
        js_obj(&[
            ("status", self.status.clone().into()),
            ("initialCreditValue", self.initial_credit_value()),
            ("remainingCreditValue", self.remaining_credit_value()),
            ("addressInfos", self.address_infos.clone().into()),
        ])
    }

    /// Returns a `JSON.stringify`-friendly form: the `Map` is normalised to a
    /// plain object so its entries survive serialisation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        normalize_js_value_for_json(&self.to_object())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(
        value: JsValue,
    ) -> WasmDppResult<VerifiedAssetLockConsumedWithAddressInfosWasm> {
        let status = js_sys::Reflect::get(&value, &"status".into())
            .ok()
            .and_then(|v| v.as_string())
            .ok_or_else(|| WasmDppError::generic("Missing property: status".to_string()))?;
        // A credit value may arrive as a BigInt (`toObject`), a base-10 string (`toJSON`
        // normalizes BigInt to a string so it survives `JSON.stringify`), or a plain number
        // (a hand-built object). Accept all three so `fromJSON(JSON.parse(JSON.stringify(...)))`
        // round-trips. Absent / null / undefined means "no surplus" (`None`); a PRESENT value
        // that cannot be cleanly read as u64 is an error — silently mapping it to `None`
        // would conflate "absent" with "garbage".
        let read_opt_u64 = |name: &str| -> WasmDppResult<Option<u64>> {
            let v = js_sys::Reflect::get(&value, &name.into()).unwrap_or(JsValue::UNDEFINED);
            if v.is_undefined() || v.is_null() {
                return Ok(None);
            }
            if let Ok(b) = u64::try_from(v.clone()) {
                return Ok(Some(b));
            }
            if let Some(s) = v.as_string() {
                return s.parse::<u64>().map(Some).map_err(|e| {
                    WasmDppError::invalid_argument(format!(
                        "{} is not a valid u64 string: {}",
                        name, e
                    ))
                });
            }
            if let Some(n) = v.as_f64() {
                if n >= 0.0 && n.fract() == 0.0 && n <= u64::MAX as f64 {
                    return Ok(Some(n as u64));
                }
                return Err(WasmDppError::invalid_argument(format!(
                    "{} must be a non-negative integer within u64 range, got {}",
                    name, n
                )));
            }
            Err(WasmDppError::invalid_argument(format!(
                "{} must be a BigInt, base-10 string, or integer number",
                name
            )))
        };
        Ok(VerifiedAssetLockConsumedWithAddressInfosWasm {
            status,
            initial_credit_value: read_opt_u64("initialCreditValue")?,
            remaining_credit_value: read_opt_u64("remainingCreditValue")?,
            address_infos: read_map_property(&value, "addressInfos")?,
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(
        value: JsValue,
    ) -> WasmDppResult<VerifiedAssetLockConsumedWithAddressInfosWasm> {
        Self::from_object(value)
    }
}

impl VerifiedAssetLockConsumedWithAddressInfosWasm {
    pub fn new(
        status: String,
        initial_credit_value: Option<u64>,
        remaining_credit_value: Option<u64>,
        address_infos: Map,
    ) -> Self {
        Self {
            status,
            initial_credit_value,
            remaining_credit_value,
            address_infos,
        }
    }
}

impl_wasm_type_info!(
    VerifiedAssetLockConsumedWithAddressInfosWasm,
    VerifiedAssetLockConsumedWithAddressInfos
);

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

    /// Returns a `JSON.stringify`-friendly form: the `Map` is normalised to a
    /// plain object so its entries survive serialisation (otherwise
    /// `JSON.stringify({nullifiers: <Map>})` produces `{"nullifiers":{}}`).
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        normalize_js_value_for_json(&self.to_object())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(value: JsValue) -> WasmDppResult<VerifiedShieldedNullifiersWasm> {
        Ok(VerifiedShieldedNullifiersWasm {
            nullifiers: read_map_property(&value, "nullifiers")?,
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedShieldedNullifiersWasm> {
        Self::from_object(value)
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

    /// Returns a `JSON.stringify`-friendly form: the `Map` instances are
    /// normalised to plain objects so their entries survive serialisation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        normalize_js_value_for_json(&self.to_object())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(
        value: JsValue,
    ) -> WasmDppResult<VerifiedShieldedNullifiersWithAddressInfosWasm> {
        Ok(VerifiedShieldedNullifiersWithAddressInfosWasm {
            nullifiers: read_map_property(&value, "nullifiers")?,
            address_infos: read_map_property(&value, "addressInfos")?,
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(
        value: JsValue,
    ) -> WasmDppResult<VerifiedShieldedNullifiersWithAddressInfosWasm> {
        Self::from_object(value)
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

    /// Returns a `JSON.stringify`-friendly form: the `Map` instances are
    /// normalised to plain objects so their entries survive serialisation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        normalize_js_value_for_json(&self.to_object())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(
        value: JsValue,
    ) -> WasmDppResult<VerifiedShieldedNullifiersWithWithdrawalDocumentWasm> {
        Ok(VerifiedShieldedNullifiersWithWithdrawalDocumentWasm {
            nullifiers: read_map_property(&value, "nullifiers")?,
            documents: read_map_property(&value, "documents")?,
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(
        value: JsValue,
    ) -> WasmDppResult<VerifiedShieldedNullifiersWithWithdrawalDocumentWasm> {
        Self::from_object(value)
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

// --- VerifiedIdentityWithShieldedNullifiers ---

/// Returned by `IdentityCreateFromShieldedPool`: the newly-created identity plus the presence of
/// each spent funding nullifier, proven together in a single STRICT merged GroveDB proof.
#[wasm_bindgen(js_name = "VerifiedIdentityWithShieldedNullifiers")]
#[derive(Clone)]
pub struct VerifiedIdentityWithShieldedNullifiersWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub identity: IdentityWasm,
    nullifiers: Map,
}

#[wasm_bindgen(js_class = VerifiedIdentityWithShieldedNullifiers)]
impl VerifiedIdentityWithShieldedNullifiersWasm {
    #[wasm_bindgen(getter)]
    pub fn nullifiers(&self) -> Map {
        self.nullifiers.clone()
    }

    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> WasmDppResult<JsValue> {
        // Use the identity's own `toObject` so consumers get a plain JS object (not the exported
        // `IdentityWasm` class instance), matching the address-funded sibling wrapper.
        let id = self.identity.to_object()?;
        let nullifiers_js: JsValue = self.nullifiers.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"identity".into(), &id.into()).unwrap();
        js_sys::Reflect::set(&obj, &"nullifiers".into(), &nullifiers_js).unwrap();
        Ok(obj.into())
    }

    /// Returns a `JSON.stringify`-friendly form: the `Map` is normalised to a plain object so its
    /// entries survive serialisation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmDppResult<JsValue> {
        let id = self.identity.to_json()?;
        let nullifiers_js: JsValue = self.nullifiers.clone().into();
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"identity".into(), &id.into()).unwrap();
        js_sys::Reflect::set(&obj, &"nullifiers".into(), &nullifiers_js).unwrap();
        normalize_js_value_for_json(&obj.into())
    }

    #[wasm_bindgen(js_name = fromObject)]
    pub fn from_object(
        value: JsValue,
    ) -> WasmDppResult<VerifiedIdentityWithShieldedNullifiersWasm> {
        let identity_val = js_sys::Reflect::get(&value, &"identity".into())
            .map_err(|_| WasmDppError::generic("Missing property: identity"))?;
        let identity: IdentityWasm = crate::serialization::conversions::from_object(identity_val)?;
        // `toJSON` normalizes the `Map` to a plain object so it survives `JSON.stringify`; rebuild a
        // real `Map` (accepting either form) so `nullifiers()` behaves like a Map after a
        // `JSON.parse(JSON.stringify(...))` round-trip — same boundary the sibling wrappers handle.
        let nullifiers = read_map_property(&value, "nullifiers")?;
        Ok(VerifiedIdentityWithShieldedNullifiersWasm {
            identity,
            nullifiers,
        })
    }

    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> WasmDppResult<VerifiedIdentityWithShieldedNullifiersWasm> {
        let identity_val = js_sys::Reflect::get(&value, &"identity".into())
            .map_err(|_| WasmDppError::generic("Missing property: identity"))?;
        let identity: IdentityWasm = crate::serialization::conversions::from_json(identity_val)?;
        // `toJSON` normalizes the `Map` to a plain object so it survives `JSON.stringify`; rebuild a
        // real `Map` (accepting either form) so `nullifiers()` behaves like a Map after a
        // `JSON.parse(JSON.stringify(...))` round-trip — same boundary the sibling wrappers handle.
        let nullifiers = read_map_property(&value, "nullifiers")?;
        Ok(VerifiedIdentityWithShieldedNullifiersWasm {
            identity,
            nullifiers,
        })
    }
}

impl VerifiedIdentityWithShieldedNullifiersWasm {
    pub fn new(identity: IdentityWasm, nullifiers: Map) -> Self {
        Self {
            identity,
            nullifiers,
        }
    }
}

impl_wasm_type_info!(
    VerifiedIdentityWithShieldedNullifiersWasm,
    VerifiedIdentityWithShieldedNullifiers
);
