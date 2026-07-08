use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::utils::{IntoWasm, try_from_options_with, try_to_array};
use dpp::address_funds::AddressWitness;
use dpp::platform_value::BinaryData;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const ADDRESS_WITNESS_TS_TYPES: &str = r#"
/**
 * Address witness for P2PKH spending in Object form.
 */
export interface AddressWitnessP2pkhObject {
    $type: "p2pkh";
    signature: Uint8Array;
}

/**
 * Address witness for P2SH spending in Object form.
 */
export interface AddressWitnessP2shObject {
    $type: "p2sh";
    signatures: Uint8Array[];
    redeemScript: Uint8Array;
}

/**
 * Address witness (P2PKH or P2SH) in Object form.
 */
export type AddressWitnessObject = AddressWitnessP2pkhObject | AddressWitnessP2shObject;

/**
 * Address witness for P2PKH spending in JSON form.
 */
export interface AddressWitnessP2pkhJSON {
    $type: "p2pkh";
    signature: string;
}

/**
 * Address witness for P2SH spending in JSON form.
 */
export interface AddressWitnessP2shJSON {
    $type: "p2sh";
    signatures: string[];
    redeemScript: string;
}

/**
 * Address witness (P2PKH or P2SH) in JSON form.
 */
export type AddressWitnessJSON = AddressWitnessP2pkhJSON | AddressWitnessP2shJSON;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "AddressWitnessObject")]
    pub type AddressWitnessObjectJs;

    #[wasm_bindgen(typescript_type = "AddressWitnessJSON")]
    pub type AddressWitnessJSONJs;
}

/// The input witness data required to spend from a PlatformAddress.
///
/// Captures the different spending patterns for P2PKH (recoverable signature only)
/// and P2SH (signatures + redeem script) addresses.
#[wasm_bindgen(js_name = "AddressWitness")]
#[derive(Clone, Debug)]
pub struct AddressWitnessWasm(AddressWitness);

impl From<AddressWitness> for AddressWitnessWasm {
    fn from(w: AddressWitness) -> Self {
        Self(w)
    }
}

impl From<AddressWitnessWasm> for AddressWitness {
    fn from(w: AddressWitnessWasm) -> Self {
        w.0
    }
}

#[wasm_bindgen(js_class = AddressWitness)]
impl AddressWitnessWasm {
    /// Creates a P2PKH witness from a recoverable ECDSA signature (typically 65 bytes
    /// including the recovery byte prefix).
    #[wasm_bindgen(js_name = "p2pkh")]
    pub fn p2pkh(signature: Vec<u8>) -> AddressWitnessWasm {
        AddressWitnessWasm(AddressWitness::P2pkh {
            signature: BinaryData::new(signature),
        })
    }

    /// Creates a P2SH witness from a list of signatures and the redeem script.
    ///
    /// For a 2-of-3 multisig, `signatures` would be `[OP_0, sig1, sig2]` and
    /// `redeemScript` would be `OP_2 <pub1> <pub2> <pub3> OP_3 OP_CHECKMULTISIG`.
    ///
    /// Each entry in `signatures` must be a `Uint8Array`. The signature count is
    /// validated by DPP (`MAX_P2SH_SIGNATURES = 17`) on serialization; this
    /// constructor does not duplicate that check.
    #[wasm_bindgen(js_name = "p2sh")]
    pub fn p2sh(
        #[wasm_bindgen(unchecked_param_type = "Uint8Array[]")] signatures: js_sys::Array,
        #[wasm_bindgen(js_name = "redeemScript")] redeem_script: Vec<u8>,
    ) -> WasmDppResult<AddressWitnessWasm> {
        let len = signatures.length() as usize;
        let mut converted = Vec::with_capacity(len);
        for (i, value) in signatures.iter().enumerate() {
            let bytes: js_sys::Uint8Array = value.dyn_into().map_err(|_| {
                WasmDppError::invalid_argument(format!(
                    "p2sh signatures[{}] must be a Uint8Array",
                    i
                ))
            })?;
            converted.push(BinaryData::new(bytes.to_vec()));
        }
        Ok(AddressWitnessWasm(AddressWitness::P2sh {
            signatures: converted,
            redeem_script: BinaryData::new(redeem_script),
        }))
    }

    /// Returns the witness kind: "p2pkh" or "p2sh".
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        match self.0 {
            AddressWitness::P2pkh { .. } => "p2pkh".to_string(),
            AddressWitness::P2sh { .. } => "p2sh".to_string(),
        }
    }

    /// Returns true if this is a P2PKH witness.
    #[wasm_bindgen(js_name = "isP2pkh", getter)]
    pub fn is_p2pkh(&self) -> bool {
        self.0.is_p2pkh()
    }

    /// Returns true if this is a P2SH witness.
    #[wasm_bindgen(js_name = "isP2sh", getter)]
    pub fn is_p2sh(&self) -> bool {
        self.0.is_p2sh()
    }

    /// Returns the signature bytes for a P2PKH witness, or `null` for P2SH.
    #[wasm_bindgen(getter)]
    pub fn signature(&self) -> Option<Vec<u8>> {
        match &self.0 {
            AddressWitness::P2pkh { signature } => Some(signature.to_vec()),
            AddressWitness::P2sh { .. } => None,
        }
    }

    /// Returns the signatures for a P2SH witness, or `null` for P2PKH.
    #[wasm_bindgen(getter)]
    pub fn signatures(&self) -> Option<Vec<js_sys::Uint8Array>> {
        match &self.0 {
            AddressWitness::P2sh { signatures, .. } => Some(
                signatures
                    .iter()
                    .map(|sig| js_sys::Uint8Array::from(sig.as_slice()))
                    .collect(),
            ),
            AddressWitness::P2pkh { .. } => None,
        }
    }

    /// Returns the redeem script for a P2SH witness, or `null` for P2PKH.
    #[wasm_bindgen(js_name = "redeemScript", getter)]
    pub fn redeem_script(&self) -> Option<Vec<u8>> {
        self.0.redeem_script().map(|s| s.to_vec())
    }
}

impl_try_from_js_value!(AddressWitnessWasm, "AddressWitness");
impl_wasm_type_info!(AddressWitnessWasm, AddressWitness);

impl_wasm_conversions_inner!(
    AddressWitnessWasm,
    AddressWitness,
    AddressWitness,
    AddressWitnessObjectJs,
    AddressWitnessJSONJs
);

/// Extract a `Vec<AddressWitnessWasm>` from a JS options object property.
///
/// Reads the named property as a JS array, then extracts each element as an
/// `AddressWitness` wasm-bindgen object via its internal pointer.
pub fn input_witnesses_from_js_options(
    options: &JsValue,
    field_name: &str,
) -> WasmDppResult<Vec<AddressWitnessWasm>> {
    let array = try_from_options_with(options, field_name, |v| try_to_array(v, field_name))?;
    array
        .iter()
        .enumerate()
        .map(|(i, item)| {
            item.to_wasm::<AddressWitnessWasm>("AddressWitness")
                .map(|r| (*r).clone())
                .map_err(|_| {
                    WasmDppError::invalid_argument(format!(
                        "{}[{}] is not an AddressWitness",
                        field_name, i
                    ))
                })
        })
        .collect()
}
