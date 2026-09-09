use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::utils::{
    IntoWasm, try_from_options_with, try_to_array, try_to_bytes, try_to_fixed_bytes,
};
use dpp::shielded::SerializedAction;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const SERIALIZED_ORCHARD_ACTION_TS_TYPES: &str = r#"
/**
 * Options for constructing a SerializedOrchardAction.
 */
export interface SerializedOrchardActionOptions {
    nullifier: Uint8Array;
    rk: Uint8Array;
    cmx: Uint8Array;
    encryptedNote: Uint8Array;
    cvNet: Uint8Array;
    spendAuthSig: Uint8Array;
}

/**
 * A serialized Orchard action (spend-output pair) in Object form.
 */
export interface SerializedOrchardActionObject {
    nullifier: Uint8Array;
    rk: Uint8Array;
    cmx: Uint8Array;
    encryptedNote: Uint8Array;
    cvNet: Uint8Array;
    spendAuthSig: Uint8Array;
}

/**
 * A serialized Orchard action (spend-output pair) in JSON form.
 */
export interface SerializedOrchardActionJSON {
    nullifier: string;
    rk: string;
    cmx: string;
    encryptedNote: string;
    cvNet: string;
    spendAuthSig: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "SerializedOrchardActionOptions")]
    pub type SerializedOrchardActionOptionsJs;

    #[wasm_bindgen(typescript_type = "SerializedOrchardActionObject")]
    pub type SerializedOrchardActionObjectJs;

    #[wasm_bindgen(typescript_type = "SerializedOrchardActionJSON")]
    pub type SerializedOrchardActionJSONJs;
}

/// A serialized Orchard action: the on-chain representation of a spend-output pair.
///
/// Each action consumes a previously created note (revealing its `nullifier`) while
/// creating a new note (publishing its commitment `cmx`). Privacy is preserved by
/// the zero-knowledge proof; observers cannot link spent notes to their commitments.
#[wasm_bindgen(js_name = "SerializedOrchardAction")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SerializedOrchardActionWasm(SerializedAction);

impl From<SerializedAction> for SerializedOrchardActionWasm {
    fn from(action: SerializedAction) -> Self {
        Self(action)
    }
}

impl From<SerializedOrchardActionWasm> for SerializedAction {
    fn from(wasm: SerializedOrchardActionWasm) -> Self {
        wasm.0
    }
}

#[wasm_bindgen(js_class = SerializedOrchardAction)]
impl SerializedOrchardActionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: SerializedOrchardActionOptionsJs,
    ) -> WasmDppResult<SerializedOrchardActionWasm> {
        let opts: &JsValue = options.as_ref();

        let nullifier: [u8; 32] = try_from_options_with(opts, "nullifier", |v| {
            try_to_fixed_bytes::<32>(v.clone(), "nullifier")
        })?;
        let rk: [u8; 32] =
            try_from_options_with(opts, "rk", |v| try_to_fixed_bytes::<32>(v.clone(), "rk"))?;
        let cmx: [u8; 32] =
            try_from_options_with(opts, "cmx", |v| try_to_fixed_bytes::<32>(v.clone(), "cmx"))?;
        // No size check — DPP's structural validation enforces
        // `encrypted_note.len() == ENCRYPTED_NOTE_SIZE` (216 bytes); wasm-dpp2
        // is a thin TS convenience wrapper and doesn't duplicate DPP logic.
        let encrypted_note: Vec<u8> = try_from_options_with(opts, "encryptedNote", |v| {
            try_to_bytes(v.clone(), "encryptedNote")
        })?;
        let cv_net: [u8; 32] = try_from_options_with(opts, "cvNet", |v| {
            try_to_fixed_bytes::<32>(v.clone(), "cvNet")
        })?;
        let spend_auth_sig: [u8; 64] = try_from_options_with(opts, "spendAuthSig", |v| {
            try_to_fixed_bytes::<64>(v.clone(), "spendAuthSig")
        })?;

        Ok(SerializedOrchardActionWasm(SerializedAction {
            nullifier,
            rk,
            cmx,
            encrypted_note,
            cv_net,
            spend_auth_sig,
        }))
    }

    /// Returns the 32-byte nullifier (note-spend tag).
    #[wasm_bindgen(getter)]
    pub fn nullifier(&self) -> Vec<u8> {
        self.0.nullifier.to_vec()
    }

    /// Returns the 32-byte randomized spend validating key.
    #[wasm_bindgen(getter)]
    pub fn rk(&self) -> Vec<u8> {
        self.0.rk.to_vec()
    }

    /// Returns the 32-byte note commitment for the new output note.
    #[wasm_bindgen(getter)]
    pub fn cmx(&self) -> Vec<u8> {
        self.0.cmx.to_vec()
    }

    /// Returns the 216-byte encrypted note ciphertext.
    #[wasm_bindgen(getter = encryptedNote)]
    pub fn encrypted_note(&self) -> Vec<u8> {
        self.0.encrypted_note.clone()
    }

    /// Returns the 32-byte value commitment.
    #[wasm_bindgen(getter = cvNet)]
    pub fn cv_net(&self) -> Vec<u8> {
        self.0.cv_net.to_vec()
    }

    /// Returns the 64-byte RedPallas spend authorization signature.
    #[wasm_bindgen(getter = spendAuthSig)]
    pub fn spend_auth_sig(&self) -> Vec<u8> {
        self.0.spend_auth_sig.to_vec()
    }
}

impl_try_from_js_value!(SerializedOrchardActionWasm, "SerializedOrchardAction");
impl_wasm_type_info!(SerializedOrchardActionWasm, SerializedOrchardAction);
impl_wasm_conversions_inner!(
    SerializedOrchardActionWasm,
    SerializedAction,
    SerializedOrchardAction,
    SerializedOrchardActionObjectJs,
    SerializedOrchardActionJSONJs
);

/// Extract a `Vec<SerializedOrchardActionWasm>` from a JS options object property.
///
/// Reads the named property as a JS array, then extracts each element as a
/// `SerializedOrchardAction` wasm-bindgen object via its internal pointer.
pub fn actions_from_js_options(
    options: &JsValue,
    field_name: &str,
) -> WasmDppResult<Vec<SerializedOrchardActionWasm>> {
    let array = try_from_options_with(options, field_name, |v| try_to_array(v, field_name))?;
    array
        .iter()
        .enumerate()
        .map(|(i, item)| {
            item.to_wasm::<SerializedOrchardActionWasm>("SerializedOrchardAction")
                .map(|r| (*r).clone())
                .map_err(|_| {
                    WasmDppError::invalid_argument(format!(
                        "{}[{}] is not a SerializedOrchardAction",
                        field_name, i
                    ))
                })
        })
        .collect()
}
