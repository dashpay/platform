use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::identity::transitions::public_key_in_creation::IdentityPublicKeyInCreationWasm;
use crate::platform_address::PlatformAddressWasm;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::utils::try_vec_to_fixed_bytes;
use crate::utils::{try_from_options, try_from_options_with, try_to_array};
use crate::{impl_wasm_conversions_inner, impl_wasm_type_info};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use dpp::state_transition::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
use dpp::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing an IdentityCreateFromShieldedPoolTransition.
 * Uses WASM instance types for complex fields. The new identity's id is
 * derived from the action nullifiers, so it is not part of the options.
 */
export interface IdentityCreateFromShieldedPoolTransitionOptions {
    publicKeys: IdentityPublicKeyInCreation[];
    denomination: bigint;
    actions: SerializedOrchardAction[];
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    sendToAddressOnCreationFailure: PlatformAddressLike;
}

/**
 * IdentityCreateFromShieldedPoolTransition serialized as a plain object.
 *
 * `sendToAddressOnCreationFailure` is the raw 21 bytes of a PlatformAddress
 * (type byte + 20-byte hash); the JSON form (below) carries the same value as
 * a hex string.
 */
export interface IdentityCreateFromShieldedPoolTransitionObject {
    $formatVersion: string;
    publicKeys: IdentityPublicKeyInCreationObject[];
    denomination: bigint;
    actions: SerializedOrchardActionObject[];
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    sendToAddressOnCreationFailure: Uint8Array;
    identityId: Uint8Array;
}

/**
 * IdentityCreateFromShieldedPoolTransition serialized as JSON (human-readable).
 */
export interface IdentityCreateFromShieldedPoolTransitionJSON {
    $formatVersion: string;
    publicKeys: IdentityPublicKeyInCreationJSON[];
    denomination: number | string;
    actions: SerializedOrchardActionJSON[];
    anchor: string;
    proof: string;
    bindingSignature: string;
    sendToAddressOnCreationFailure: string;
    identityId: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityCreateFromShieldedPoolTransitionOptions")]
    pub type IdentityCreateFromShieldedPoolTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityCreateFromShieldedPoolTransitionObject")]
    pub type IdentityCreateFromShieldedPoolTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityCreateFromShieldedPoolTransitionJSON")]
    pub type IdentityCreateFromShieldedPoolTransitionJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
///
/// The complex fields (`publicKeys`, `actions`, `sendToAddressOnCreationFailure`)
/// are extracted separately as WASM class instances.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreateFromShieldedPoolSimpleFields {
    denomination: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = IdentityCreateFromShieldedPoolTransition)]
pub struct IdentityCreateFromShieldedPoolTransitionWasm(IdentityCreateFromShieldedPoolTransition);

impl From<IdentityCreateFromShieldedPoolTransition>
    for IdentityCreateFromShieldedPoolTransitionWasm
{
    fn from(v: IdentityCreateFromShieldedPoolTransition) -> Self {
        IdentityCreateFromShieldedPoolTransitionWasm(v)
    }
}

impl From<IdentityCreateFromShieldedPoolTransitionWasm>
    for IdentityCreateFromShieldedPoolTransition
{
    fn from(v: IdentityCreateFromShieldedPoolTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = IdentityCreateFromShieldedPoolTransition)]
impl IdentityCreateFromShieldedPoolTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: IdentityCreateFromShieldedPoolTransitionOptionsJs,
    ) -> WasmDppResult<IdentityCreateFromShieldedPoolTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        // Extract WASM class instances (borrow &options)
        let js_public_keys_array =
            try_from_options_with(js_opts, "publicKeys", |v| try_to_array(v, "publicKeys"))?;
        let public_keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_array(&js_public_keys_array)?;
        let actions = actions_from_js_options(js_opts, "actions")?;
        let send_to_address: PlatformAddressWasm =
            try_from_options(js_opts, "sendToAddressOnCreationFailure")?;

        // Extract simple fields via serde (consumes options)
        let fields: IdentityCreateFromShieldedPoolSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        let actions: Vec<dpp::shielded::SerializedAction> =
            actions.into_iter().map(Into::into).collect();
        // The id is fully determined by the spend nullifiers (consensus re-derives
        // and checks it), so it is derived here rather than taken from options.
        let identity_id = derive_identity_id_from_actions(&actions);

        Ok(IdentityCreateFromShieldedPoolTransitionWasm(
            IdentityCreateFromShieldedPoolTransition::V0(
                IdentityCreateFromShieldedPoolTransitionV0 {
                    public_keys: public_keys.into_iter().map(Into::into).collect(),
                    denomination: fields.denomination,
                    actions,
                    anchor,
                    proof: fields.proof,
                    binding_signature,
                    send_to_address_on_creation_failure: send_to_address.into(),
                    identity_id,
                },
            ),
        ))
    }

    /// Returns the public keys of the new identity.
    #[wasm_bindgen(getter = "publicKeys")]
    pub fn public_keys(&self) -> Vec<IdentityPublicKeyInCreationWasm> {
        match &self.0 {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0
                .public_keys
                .iter()
                .cloned()
                .map(IdentityPublicKeyInCreationWasm::from)
                .collect(),
        }
    }

    /// Returns the fixed exit denomination (credits leaving the shielded pool).
    #[wasm_bindgen(getter = "denomination")]
    pub fn denomination(&self) -> u64 {
        match &self.0 {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.denomination,
        }
    }

    /// Returns the serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        match &self.0 {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0
                .actions
                .iter()
                .cloned()
                .map(SerializedOrchardActionWasm::from)
                .collect(),
        }
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        match &self.0 {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.anchor.to_vec(),
        }
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        match &self.0 {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.proof.clone(),
        }
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
        match &self.0 {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.binding_signature.to_vec(),
        }
    }

    /// Returns the fallback platform address credited if identity creation
    /// fails a stateful check.
    #[wasm_bindgen(getter = "sendToAddressOnCreationFailure")]
    pub fn send_to_address_on_creation_failure(&self) -> PlatformAddressWasm {
        match &self.0 {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => {
                PlatformAddressWasm::from(v0.send_to_address_on_creation_failure)
            }
        }
    }

    /// Returns the new identity's id (derived from the spend nullifiers).
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
        match &self.0 {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.identity_id.into(),
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
            &StateTransition::IdentityCreateFromShieldedPool(self.0.clone()),
        )?)
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(
        bytes: Vec<u8>,
    ) -> WasmDppResult<IdentityCreateFromShieldedPoolTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(&bytes)?;
        match st {
            StateTransition::IdentityCreateFromShieldedPool(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected IdentityCreateFromShieldedPool",
            )),
        }
    }

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> crate::state_transitions::base::StateTransitionWasm {
        StateTransition::IdentityCreateFromShieldedPool(self.0.clone()).into()
    }
}

impl_wasm_conversions_inner!(
    IdentityCreateFromShieldedPoolTransitionWasm,
    IdentityCreateFromShieldedPoolTransition,
    IdentityCreateFromShieldedPoolTransition,
    IdentityCreateFromShieldedPoolTransitionObjectJs,
    IdentityCreateFromShieldedPoolTransitionJSONJs
);

impl_wasm_type_info!(
    IdentityCreateFromShieldedPoolTransitionWasm,
    IdentityCreateFromShieldedPoolTransition
);
