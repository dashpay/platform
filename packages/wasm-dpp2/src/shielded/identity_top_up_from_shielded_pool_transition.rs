use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::try_vec_to_fixed_bytes;
use crate::{impl_wasm_conversions_inner, impl_wasm_type_info};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::accessors::IdentityTopUpFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::v0::IdentityTopUpFromShieldedPoolTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing an IdentityTopUpFromShieldedPoolTransition (shielded pool to an
 * existing identity's balance). The bundle is an Orchard spend whose binding signature commits
 * to identityId and topUpAmount; there is no platform signature.
 */
export interface IdentityTopUpFromShieldedPoolTransitionOptions {
    identityId: IdentifierLike;
    actions: SerializedOrchardAction[];
    topUpAmount: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

export interface IdentityTopUpFromShieldedPoolTransitionObject {
    $formatVersion: string;
    identityId: Uint8Array;
    actions: SerializedOrchardActionObject[];
    topUpAmount: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

export interface IdentityTopUpFromShieldedPoolTransitionJSON {
    $formatVersion: string;
    identityId: string;
    actions: SerializedOrchardActionJSON[];
    topUpAmount: number | string;
    anchor: string;
    proof: string;
    bindingSignature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityTopUpFromShieldedPoolTransitionOptions")]
    pub type IdentityTopUpFromShieldedPoolTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityTopUpFromShieldedPoolTransitionObject")]
    pub type IdentityTopUpFromShieldedPoolTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityTopUpFromShieldedPoolTransitionJSON")]
    pub type IdentityTopUpFromShieldedPoolTransitionJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityTopUpFromShieldedPoolTransitionSimpleFields {
    top_up_amount: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = IdentityTopUpFromShieldedPoolTransition)]
pub struct IdentityTopUpFromShieldedPoolTransitionWasm(IdentityTopUpFromShieldedPoolTransition);

impl From<IdentityTopUpFromShieldedPoolTransition> for IdentityTopUpFromShieldedPoolTransitionWasm {
    fn from(v: IdentityTopUpFromShieldedPoolTransition) -> Self {
        IdentityTopUpFromShieldedPoolTransitionWasm(v)
    }
}

impl From<IdentityTopUpFromShieldedPoolTransitionWasm> for IdentityTopUpFromShieldedPoolTransition {
    fn from(v: IdentityTopUpFromShieldedPoolTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = IdentityTopUpFromShieldedPoolTransition)]
impl IdentityTopUpFromShieldedPoolTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: IdentityTopUpFromShieldedPoolTransitionOptionsJs,
    ) -> WasmDppResult<IdentityTopUpFromShieldedPoolTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        let identity_id: IdentifierWasm = crate::utils::try_from_options(&options, "identityId")?;
        let actions = actions_from_js_options(js_opts, "actions")?;

        let fields: IdentityTopUpFromShieldedPoolTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(IdentityTopUpFromShieldedPoolTransitionWasm(
            IdentityTopUpFromShieldedPoolTransition::V0(
                IdentityTopUpFromShieldedPoolTransitionV0 {
                    identity_id: identity_id.into(),
                    actions: actions.into_iter().map(Into::into).collect(),
                    top_up_amount: fields.top_up_amount,
                    anchor,
                    proof: fields.proof,
                    binding_signature,
                },
            ),
        ))
    }

    /// The identity whose balance receives the top-up.
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
        self.0.identity_id().into()
    }

    #[wasm_bindgen(setter = "identityId")]
    pub fn set_identity_id(&mut self, identity_id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_identity_id(identity_id.try_into()?);
        Ok(())
    }

    /// Returns the serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        self.0
            .actions()
            .iter()
            .cloned()
            .map(SerializedOrchardActionWasm::from)
            .collect()
    }

    /// Gross credits leaving the pool; the identity receives this minus the flat fee.
    #[wasm_bindgen(getter = "topUpAmount")]
    pub fn top_up_amount(&self) -> u64 {
        self.0.top_up_amount()
    }

    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        self.0.anchor().to_vec()
    }

    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        self.0.proof().to_vec()
    }

    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
        self.0.binding_signature().to_vec()
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
            &StateTransition::IdentityTopUpFromShieldedPool(self.0.clone()),
        )?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(
        bytes: Vec<u8>,
    ) -> WasmDppResult<IdentityTopUpFromShieldedPoolTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(&bytes)?;
        match st {
            StateTransition::IdentityTopUpFromShieldedPool(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected IdentityTopUpFromShieldedPool",
            )),
        }
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityTopUpFromShieldedPoolTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityTopUpFromShieldedPoolTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(
        base64: String,
    ) -> WasmDppResult<IdentityTopUpFromShieldedPoolTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityTopUpFromShieldedPoolTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransition::IdentityTopUpFromShieldedPool(self.0.clone()).into()
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<IdentityTopUpFromShieldedPoolTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();
        match rs_st {
            StateTransition::IdentityTopUpFromShieldedPool(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected IdentityTopUpFromShieldedPool",
            )),
        }
    }
}

impl_wasm_conversions_inner!(
    IdentityTopUpFromShieldedPoolTransitionWasm,
    IdentityTopUpFromShieldedPoolTransition,
    IdentityTopUpFromShieldedPoolTransition,
    IdentityTopUpFromShieldedPoolTransitionObjectJs,
    IdentityTopUpFromShieldedPoolTransitionJSONJs
);

impl_wasm_type_info!(
    IdentityTopUpFromShieldedPoolTransitionWasm,
    IdentityTopUpFromShieldedPoolTransition
);
