use super::{PlatformAddressLikeJs, PlatformAddressWasm};
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use crate::utils::{IntoWasm, try_to_u64};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use js_sys::BigInt;
use serde::Deserialize;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

/// Represents an input address for address-based state transitions.
///
/// An input specifies a Platform address that will spend credits,
/// along with its current nonce and the amount to spend.
#[wasm_bindgen(js_name = "PlatformAddressInput")]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAddressInputWasm {
    address: PlatformAddressWasm,
    nonce: AddressNonce,
    amount: Credits,
}

impl_wasm_type_info!(PlatformAddressInputWasm, PlatformAddressInput);

#[wasm_bindgen(js_class = PlatformAddressInput)]
impl PlatformAddressInputWasm {
    /// Creates a new PlatformAddressInput.
    ///
    /// @param address - The Platform address (PlatformAddress, Uint8Array, or bech32m string)
    /// @param nonce - The current nonce of the address (will be incremented for the transaction)
    /// @param amount - The amount of credits to spend from this address
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        address: PlatformAddressLikeJs,
        nonce: u32,
        amount: BigInt,
    ) -> WasmDppResult<PlatformAddressInputWasm> {
        let platform_address: PlatformAddressWasm = address.try_into()?;
        let amount_u64 = try_to_u64(&amount.into(), "amount")?;

        Ok(PlatformAddressInputWasm {
            address: platform_address,
            nonce,
            amount: amount_u64,
        })
    }

    /// Returns the Platform address.
    #[wasm_bindgen(getter)]
    pub fn address(&self) -> PlatformAddressWasm {
        self.address
    }

    /// Returns the nonce.
    #[wasm_bindgen(getter)]
    pub fn nonce(&self) -> u32 {
        self.nonce
    }

    /// Returns the amount.
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> BigInt {
        BigInt::from(self.amount)
    }
}

impl PlatformAddressInputWasm {
    pub fn new(address: PlatformAddress, nonce: AddressNonce, amount: Credits) -> Self {
        Self {
            address: address.into(),
            nonce,
            amount,
        }
    }

    /// Returns the inner values as a tuple suitable for BTreeMap insertion.
    pub fn into_inner(self) -> (PlatformAddress, (AddressNonce, Credits)) {
        (self.address.into(), (self.nonce, self.amount))
    }
}

/// Represents an output address for address-based state transitions.
///
/// An output specifies a Platform address that will receive credits,
/// along with an optional amount to receive. When amount is None,
/// the system distributes funds automatically (used for asset lock funding).
// TODO: Add nonce; see [WasmSdk::identity_create_from_addresses] notes.
#[wasm_bindgen(js_name = "PlatformAddressOutput")]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAddressOutputWasm {
    address: PlatformAddressWasm,
    #[serde(default)]
    amount: Option<Credits>,
}

impl_wasm_type_info!(PlatformAddressOutputWasm, PlatformAddressOutput);

#[wasm_bindgen(js_class = PlatformAddressOutput)]
impl PlatformAddressOutputWasm {
    /// Creates a new PlatformAddressOutput with a specific amount.
    ///
    /// @param address - The Platform address (PlatformAddress, Uint8Array, or bech32m string)
    /// @param amount - The amount of credits to send to this address (optional for asset lock funding)
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        address: PlatformAddressLikeJs,
        amount: Option<BigInt>,
    ) -> WasmDppResult<PlatformAddressOutputWasm> {
        let platform_address: PlatformAddressWasm = address.try_into()?;
        let amount_u64 = amount
            .map(|v| try_to_u64(&v.into(), "amount"))
            .transpose()?;

        Ok(PlatformAddressOutputWasm {
            address: platform_address,
            amount: amount_u64,
        })
    }

    /// Returns the Platform address.
    #[wasm_bindgen(getter)]
    pub fn address(&self) -> PlatformAddressWasm {
        self.address
    }

    /// Returns the amount, or undefined if not specified.
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> Option<BigInt> {
        self.amount.map(BigInt::from)
    }
}

impl PlatformAddressOutputWasm {
    pub fn new(address: PlatformAddress, amount: Credits) -> Self {
        Self {
            address: address.into(),
            amount: Some(amount),
        }
    }

    pub fn new_optional(address: PlatformAddress, amount: Option<Credits>) -> Self {
        Self {
            address: address.into(),
            amount,
        }
    }

    /// Returns the inner values as a tuple, or an error if amount is None.
    pub fn try_into_inner(self) -> WasmDppResult<(PlatformAddress, Credits)> {
        let amount = self.amount.ok_or_else(|| {
            WasmDppError::invalid_argument("PlatformAddressOutput: amount is required")
        })?;
        Ok((self.address.into(), amount))
    }

    /// Returns the inner values with optional amount.
    pub fn into_inner_optional(self) -> (PlatformAddress, Option<Credits>) {
        (self.address.into(), self.amount)
    }
}

/// Converts a vector of PlatformAddressOutput into a BTreeMap.
/// Returns an error if any output has no amount set.
pub fn outputs_to_btree_map(
    outputs: Vec<PlatformAddressOutputWasm>,
) -> WasmDppResult<BTreeMap<PlatformAddress, Credits>> {
    outputs.into_iter().map(|o| o.try_into_inner()).collect()
}

/// Converts a vector of PlatformAddressOutput into a BTreeMap with optional amounts.
///
/// Used for asset lock funding where the amount is optional (None means
/// the system distributes the asset lock funds automatically).
pub fn outputs_to_optional_btree_map(
    outputs: Vec<PlatformAddressOutputWasm>,
) -> BTreeMap<PlatformAddress, Option<Credits>> {
    outputs
        .into_iter()
        .map(|o| o.into_inner_optional())
        .collect()
}

/// Extract a Vec<PlatformAddressInputWasm> from a JS options object property.
///
/// Reads the named property as a JS array, then extracts each element
/// as a PlatformAddressInput wasm-bindgen object via its __wbg_ptr.
pub fn inputs_from_js_options(
    options: &JsValue,
    field_name: &str,
) -> WasmDppResult<Vec<PlatformAddressInputWasm>> {
    let value = js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
        WasmDppError::invalid_argument(format!("failed to read '{}' from options", field_name))
    })?;
    if value.is_undefined() || value.is_null() {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' is required",
            field_name
        )));
    }
    if !js_sys::Array::is_array(&value) {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be an array",
            field_name
        )));
    }
    let array = js_sys::Array::from(&value);
    array
        .iter()
        .enumerate()
        .map(|(i, item)| {
            item.to_wasm::<PlatformAddressInputWasm>("PlatformAddressInput")
                .map(|r| (*r).clone())
                .map_err(|_| {
                    WasmDppError::invalid_argument(format!(
                        "{}[{}] is not a PlatformAddressInput",
                        field_name, i
                    ))
                })
        })
        .collect()
}

/// Extract a Vec<PlatformAddressOutputWasm> from a JS options object property.
///
/// Reads the named property as a JS array, then extracts each element
/// as a PlatformAddressOutput wasm-bindgen object via its __wbg_ptr.
pub fn outputs_from_js_options(
    options: &JsValue,
    field_name: &str,
) -> WasmDppResult<Vec<PlatformAddressOutputWasm>> {
    let value = js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
        WasmDppError::invalid_argument(format!("failed to read '{}' from options", field_name))
    })?;
    if value.is_undefined() || value.is_null() {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' is required",
            field_name
        )));
    }
    if !js_sys::Array::is_array(&value) {
        return Err(WasmDppError::invalid_argument(format!(
            "'{}' must be an array",
            field_name
        )));
    }
    let array = js_sys::Array::from(&value);
    array
        .iter()
        .enumerate()
        .map(|(i, item)| {
            item.to_wasm::<PlatformAddressOutputWasm>("PlatformAddressOutput")
                .map(|r| (*r).clone())
                .map_err(|_| {
                    WasmDppError::invalid_argument(format!(
                        "{}[{}] is not a PlatformAddressOutput",
                        field_name, i
                    ))
                })
        })
        .collect()
}

/// Extract an optional PlatformAddressOutputWasm from a JS options object property.
///
/// Returns None if the property is undefined or null.
pub fn optional_output_from_js_options(
    options: &JsValue,
    field_name: &str,
) -> WasmDppResult<Option<PlatformAddressOutputWasm>> {
    let value = js_sys::Reflect::get(options, &JsValue::from_str(field_name)).map_err(|_| {
        WasmDppError::invalid_argument(format!("failed to read '{}' from options", field_name))
    })?;
    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }
    value
        .to_wasm::<PlatformAddressOutputWasm>("PlatformAddressOutput")
        .map(|r| Some((*r).clone()))
        .map_err(|_| {
            WasmDppError::invalid_argument(format!(
                "'{}' is not a PlatformAddressOutput",
                field_name
            ))
        })
}
