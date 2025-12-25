use crate::error::{WasmDppError, WasmDppResult};
use super::PlatformAddressWasm;
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

/// Helper to convert BigInt to u64
fn bigint_to_u64(value: BigInt) -> Result<u64, WasmDppError> {
    value
        .try_into()
        .map_err(|_| WasmDppError::invalid_argument("value must be a valid u64"))
}

#[wasm_bindgen(js_class = PlatformAddressInput)]
impl PlatformAddressInputWasm {
    /// Creates a new PlatformAddressInput.
    ///
    /// @param address - The Platform address (PlatformAddress, Uint8Array, or bech32m string)
    /// @param nonce - The current nonce of the address (will be incremented for the transaction)
    /// @param amount - The amount of credits to spend from this address
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "PlatformAddressLike")] address: &JsValue,
        nonce: u32,
        amount: BigInt,
    ) -> WasmDppResult<PlatformAddressInputWasm> {
        let platform_address = PlatformAddressWasm::try_from(address)?;
        let amount_u64 = bigint_to_u64(amount)?;

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
    /// Returns the inner values as a tuple suitable for BTreeMap insertion.
    pub fn into_inner(self) -> (PlatformAddress, (AddressNonce, Credits)) {
        (self.address.into(), (self.nonce, self.amount))
    }

    /// Returns a reference to the inner PlatformAddress.
    pub fn platform_address(&self) -> PlatformAddress {
        self.address.into()
    }

    /// Returns the nonce value.
    pub fn nonce_value(&self) -> AddressNonce {
        self.nonce
    }

    /// Returns the amount value.
    pub fn amount_value(&self) -> Credits {
        self.amount
    }
}

/// Represents an output address for address-based state transitions.
///
/// An output specifies a Platform address that will receive credits,
/// along with the amount to receive.
#[wasm_bindgen(js_name = "PlatformAddressOutput")]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAddressOutputWasm {
    address: PlatformAddressWasm,
    amount: Credits,
}

#[wasm_bindgen(js_class = PlatformAddressOutput)]
impl PlatformAddressOutputWasm {
    /// Creates a new PlatformAddressOutput.
    ///
    /// @param address - The Platform address (PlatformAddress, Uint8Array, or bech32m string)
    /// @param amount - The amount of credits to send to this address
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "PlatformAddressLike")] address: &JsValue,
        amount: BigInt,
    ) -> WasmDppResult<PlatformAddressOutputWasm> {
        let platform_address = PlatformAddressWasm::try_from(address)?;
        let amount_u64 = bigint_to_u64(amount)?;

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

    /// Returns the amount.
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> BigInt {
        BigInt::from(self.amount)
    }
}

impl PlatformAddressOutputWasm {
    /// Returns the inner values as a tuple suitable for BTreeMap insertion.
    pub fn into_inner(self) -> (PlatformAddress, Credits) {
        (self.address.into(), self.amount)
    }

    /// Returns a reference to the inner PlatformAddress.
    pub fn platform_address(&self) -> PlatformAddress {
        self.address.into()
    }

    /// Returns the amount value.
    pub fn amount_value(&self) -> Credits {
        self.amount
    }
}

/// Converts a vector of PlatformAddressInput into a BTreeMap.
pub fn inputs_to_btree_map(
    inputs: Vec<PlatformAddressInputWasm>,
) -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
    inputs.into_iter().map(|i| i.into_inner()).collect()
}

/// Converts a vector of PlatformAddressOutput into a BTreeMap.
pub fn outputs_to_btree_map(
    outputs: Vec<PlatformAddressOutputWasm>,
) -> BTreeMap<PlatformAddress, Credits> {
    outputs.into_iter().map(|o| o.into_inner()).collect()
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
        .map(|o| {
            let (addr, amount) = o.into_inner();
            (addr, Some(amount))
        })
        .collect()
}

/// Extracts addresses from a slice of PlatformAddressOutput.
pub fn extract_addresses(outputs: &[PlatformAddressOutputWasm]) -> Vec<PlatformAddress> {
    outputs.iter().map(|o| o.platform_address()).collect()
}

/// Extracts amounts from a slice of PlatformAddressOutput.
pub fn extract_amounts(outputs: &[PlatformAddressOutputWasm]) -> Vec<Credits> {
    outputs.iter().map(|o| o.amount_value()).collect()
}
