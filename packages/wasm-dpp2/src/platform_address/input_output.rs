use super::{PlatformAddressLikeJs, PlatformAddressWasm};
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::utils::{IntoWasm, try_from_options_with, try_to_array, try_to_u64};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use js_sys::BigInt;
use serde::Deserialize;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const PLATFORM_ADDRESS_INPUT_OUTPUT_TS_TYPES: &str = r#"
/**
 * Input address spending credits — Object form (output of a transition's `toObject()`).
 *
 * `address` is the 21-byte PlatformAddress (1 type byte + 20-byte hash) as a Uint8Array.
 */
export interface PlatformAddressInputObject {
    address: Uint8Array;
    nonce: number;
    amount: bigint;
}

/**
 * Input address spending credits — JSON form (output of a transition's `toJSON()`).
 *
 * `address` is hex-encoded; `amount` may be a string when above `Number.MAX_SAFE_INTEGER`.
 */
export interface PlatformAddressInputJSON {
    address: string;
    nonce: number;
    amount: number | string;
}

/**
 * Output address receiving credits — Object form.
 *
 * `amount` is `null` only for asset-lock funding transitions, where exactly one
 * output (acting as the change recipient) absorbs the asset-lock remainder. For
 * all other transitions (transfer / withdrawal / identity flows / credit transfer)
 * the amount is always present.
 */
export interface PlatformAddressOutputObject {
    address: Uint8Array;
    amount: bigint | null;
}

/**
 * Output address receiving credits — JSON form.
 */
export interface PlatformAddressOutputJSON {
    address: string;
    amount: number | string | null;
}
"#;

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
impl_try_from_js_value!(PlatformAddressOutputWasm, "PlatformAddressOutput");

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

/// Converts a vector of `PlatformAddressInputWasm` into a `BTreeMap`,
/// returning an error if any address appears more than once.
///
/// `BTreeMap::collect()` on its own would silently overwrite duplicates,
/// which is dangerous: a JS caller could pass two entries for the same
/// address with different amounts and only the last one would survive.
pub fn inputs_to_btree_map(
    inputs: Vec<PlatformAddressInputWasm>,
) -> WasmDppResult<BTreeMap<PlatformAddress, (AddressNonce, Credits)>> {
    let mut map = BTreeMap::new();
    for input in inputs {
        let (address, value) = input.into_inner();
        if map.insert(address, value).is_some() {
            return Err(WasmDppError::invalid_argument(format!(
                "duplicate input address: {}",
                hex::encode(address.to_bytes())
            )));
        }
    }
    Ok(map)
}

/// Converts a vector of PlatformAddressOutput into a BTreeMap.
/// Returns an error if any output has no amount set or if an address is duplicated.
pub fn outputs_to_btree_map(
    outputs: Vec<PlatformAddressOutputWasm>,
) -> WasmDppResult<BTreeMap<PlatformAddress, Credits>> {
    let mut map = BTreeMap::new();
    for output in outputs {
        let (address, amount) = output.try_into_inner()?;
        if map.insert(address, amount).is_some() {
            return Err(WasmDppError::invalid_argument(format!(
                "duplicate output address: {}",
                hex::encode(address.to_bytes())
            )));
        }
    }
    Ok(map)
}

/// Converts a vector of PlatformAddressOutput into a BTreeMap with optional amounts.
///
/// Used for asset lock funding where the amount is optional (None means
/// the system distributes the asset lock funds automatically). Returns an
/// error if an address is duplicated.
pub fn outputs_to_optional_btree_map(
    outputs: Vec<PlatformAddressOutputWasm>,
) -> WasmDppResult<BTreeMap<PlatformAddress, Option<Credits>>> {
    let mut map = BTreeMap::new();
    for output in outputs {
        let (address, amount) = output.into_inner_optional();
        if map.insert(address, amount).is_some() {
            return Err(WasmDppError::invalid_argument(format!(
                "duplicate output address: {}",
                hex::encode(address.to_bytes())
            )));
        }
    }
    Ok(map)
}

/// Extract a Vec<PlatformAddressInputWasm> from a JS options object property.
///
/// Reads the named property as a JS array, then extracts each element
/// as a PlatformAddressInput wasm-bindgen object via its __wbg_ptr.
pub fn inputs_from_js_options(
    options: &JsValue,
    field_name: &str,
) -> WasmDppResult<Vec<PlatformAddressInputWasm>> {
    let array = try_from_options_with(options, field_name, |v| try_to_array(v, field_name))?;
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
    let array = try_from_options_with(options, field_name, |v| try_to_array(v, field_name))?;
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
