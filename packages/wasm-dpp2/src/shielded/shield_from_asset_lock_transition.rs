use crate::asset_lock_proof::AssetLockProofWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::shielded::{MAX_HALO2_PROOF_BYTES, MAX_RECOVERABLE_ECDSA_SIGNATURE_BYTES};
use crate::utils::{check_max_len, try_from_options, try_vec_to_fixed_bytes};
use crate::{impl_wasm_conversions_serde, impl_wasm_type_info};
use dpp::platform_value::BinaryData;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use dpp::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Non-WASM-instance fields extracted from the constructor options via serde.
///
/// `actions` and `assetLockProof` are extracted separately as WASM class instances.
/// `signature` is optional because transitions are typically constructed unsigned.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShieldFromAssetLockTransitionSimpleFields {
    value_balance: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
    #[serde(default)]
    signature: Vec<u8>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing a ShieldFromAssetLockTransition.
 * Uses WASM instance types for complex fields like AssetLockProof.
 */
export interface ShieldFromAssetLockTransitionOptions {
    assetLockProof: AssetLockProof;
    actions: SerializedOrchardAction[];
    valueBalance: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    signature: Uint8Array;
}

/**
 * ShieldFromAssetLockTransition serialized as a plain object.
 */
export interface ShieldFromAssetLockTransitionObject {
    $formatVersion: string;
    assetLockProof: AssetLockProofObject;
    actions: SerializedOrchardActionObject[];
    valueBalance: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    signature: Uint8Array;
}

/**
 * ShieldFromAssetLockTransition serialized as JSON (human-readable).
 */
export interface ShieldFromAssetLockTransitionJSON {
    $formatVersion: string;
    assetLockProof: AssetLockProofJSON;
    actions: SerializedOrchardActionJSON[];
    valueBalance: number | string;
    anchor: string;
    proof: string;
    bindingSignature: string;
    signature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ShieldFromAssetLockTransitionOptions")]
    pub type ShieldFromAssetLockTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "ShieldFromAssetLockTransitionObject")]
    pub type ShieldFromAssetLockTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "ShieldFromAssetLockTransitionJSON")]
    pub type ShieldFromAssetLockTransitionJSONJs;
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = ShieldFromAssetLockTransition)]
pub struct ShieldFromAssetLockTransitionWasm(ShieldFromAssetLockTransition);

impl From<ShieldFromAssetLockTransition> for ShieldFromAssetLockTransitionWasm {
    fn from(v: ShieldFromAssetLockTransition) -> Self {
        ShieldFromAssetLockTransitionWasm(v)
    }
}

impl From<ShieldFromAssetLockTransitionWasm> for ShieldFromAssetLockTransition {
    fn from(v: ShieldFromAssetLockTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = ShieldFromAssetLockTransition)]
impl ShieldFromAssetLockTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: ShieldFromAssetLockTransitionOptionsJs,
    ) -> WasmDppResult<ShieldFromAssetLockTransitionWasm> {
        // Extract WASM class instances (borrow &options)
        let asset_lock: AssetLockProofWasm = try_from_options(&options, "assetLockProof")?;
        let actions = actions_from_js_options(options.as_ref(), "actions")?;

        // Extract remaining simple fields via serde (consumes options)
        let fields: ShieldFromAssetLockTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;
        check_max_len(&fields.proof, MAX_HALO2_PROOF_BYTES, "proof")?;
        check_max_len(
            &fields.signature,
            MAX_RECOVERABLE_ECDSA_SIGNATURE_BYTES,
            "signature",
        )?;

        Ok(ShieldFromAssetLockTransitionWasm(
            ShieldFromAssetLockTransition::V0(ShieldFromAssetLockTransitionV0 {
                asset_lock_proof: asset_lock.into(),
                actions: actions.into_iter().map(Into::into).collect(),
                value_balance: fields.value_balance,
                anchor,
                proof: fields.proof,
                binding_signature,
                signature: BinaryData::from(fields.signature),
            }),
        ))
    }

    /// Returns the asset lock proof.
    #[wasm_bindgen(getter = "assetLockProof")]
    pub fn asset_lock_proof(&self) -> AssetLockProofWasm {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => {
                AssetLockProofWasm::from(v0.asset_lock_proof.clone())
            }
        }
    }

    /// Returns the serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0
                .actions
                .iter()
                .cloned()
                .map(SerializedOrchardActionWasm::from)
                .collect(),
        }
    }

    /// Returns the net value balance.
    #[wasm_bindgen(getter = "valueBalance")]
    pub fn value_balance(&self) -> u64 {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.value_balance,
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.binding_signature.to_vec(),
        }
    }

    /// Returns the ECDSA signature.
    #[wasm_bindgen(getter = "signature")]
    pub fn signature(&self) -> Vec<u8> {
        match &self.0 {
            ShieldFromAssetLockTransition::V0(v0) => v0.signature.to_vec(),
        }
    }

    #[wasm_bindgen(js_name = getModifiedDataIds)]
    pub fn modified_data_ids(&self) -> Vec<IdentifierWasm> {
        self.0
            .modified_data_ids()
            .into_iter()
            .map(IdentifierWasm::from)
            .collect()
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(PlatformSerializable::serialize_to_bytes(
            &StateTransition::ShieldFromAssetLock(self.0.clone()),
        )?)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ShieldFromAssetLockTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(&bytes)?;
        match st {
            StateTransition::ShieldFromAssetLock(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected ShieldFromAssetLock",
            )),
        }
    }

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> crate::state_transitions::base::StateTransitionWasm {
        StateTransition::ShieldFromAssetLock(self.0.clone()).into()
    }
}

impl_wasm_conversions_serde!(
    ShieldFromAssetLockTransitionWasm,
    ShieldFromAssetLockTransition,
    ShieldFromAssetLockTransitionObjectJs,
    ShieldFromAssetLockTransitionJSONJs
);

impl_wasm_type_info!(
    ShieldFromAssetLockTransitionWasm,
    ShieldFromAssetLockTransition
);
