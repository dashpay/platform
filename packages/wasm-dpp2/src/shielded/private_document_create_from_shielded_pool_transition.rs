use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::try_vec_to_fixed_bytes;
use crate::{impl_wasm_conversions_inner, impl_wasm_type_info};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::accessors::PrivateDocumentCreateFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::v0::PrivateDocumentCreateFromShieldedPoolTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use js_sys::{Array, Uint8Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing a PrivateDocumentCreateFromShieldedPoolTransition (entries appended to
 * a private document type's store, paid from the credit shielded pool). The bundle is an Orchard
 * spend whose value balance is the fee and whose binding signature commits to the contract, the
 * type name and every entry; there is no platform signature.
 */
export interface PrivateDocumentCreateFromShieldedPoolTransitionOptions {
    dataContractId: IdentifierLike;
    documentTypeName: string;
    entries: Uint8Array[];
    actions: SerializedOrchardAction[];
    valueBalance: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

export interface PrivateDocumentCreateFromShieldedPoolTransitionObject {
    $formatVersion: string;
    dataContractId: Uint8Array;
    documentTypeName: string;
    entries: Uint8Array[];
    actions: SerializedOrchardActionObject[];
    valueBalance: bigint;
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

export interface PrivateDocumentCreateFromShieldedPoolTransitionJSON {
    $formatVersion: string;
    dataContractId: string;
    documentTypeName: string;
    entries: string[];
    actions: SerializedOrchardActionJSON[];
    valueBalance: number | string;
    anchor: string;
    proof: string;
    bindingSignature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PrivateDocumentCreateFromShieldedPoolTransitionOptions")]
    pub type PrivateDocumentCreateFromShieldedPoolTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "PrivateDocumentCreateFromShieldedPoolTransitionObject")]
    pub type PrivateDocumentCreateFromShieldedPoolTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "PrivateDocumentCreateFromShieldedPoolTransitionJSON")]
    pub type PrivateDocumentCreateFromShieldedPoolTransitionJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrivateDocumentCreateFromShieldedPoolTransitionSimpleFields {
    document_type_name: String,
    /// Each a `Uint8Array`, which the serde-wasm-bindgen fork reads as bytes.
    entries: Vec<Vec<u8>>,
    value_balance: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = PrivateDocumentCreateFromShieldedPoolTransition)]
pub struct PrivateDocumentCreateFromShieldedPoolTransitionWasm(
    PrivateDocumentCreateFromShieldedPoolTransition,
);

impl From<PrivateDocumentCreateFromShieldedPoolTransition>
    for PrivateDocumentCreateFromShieldedPoolTransitionWasm
{
    fn from(v: PrivateDocumentCreateFromShieldedPoolTransition) -> Self {
        PrivateDocumentCreateFromShieldedPoolTransitionWasm(v)
    }
}

impl From<PrivateDocumentCreateFromShieldedPoolTransitionWasm>
    for PrivateDocumentCreateFromShieldedPoolTransition
{
    fn from(v: PrivateDocumentCreateFromShieldedPoolTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = PrivateDocumentCreateFromShieldedPoolTransition)]
impl PrivateDocumentCreateFromShieldedPoolTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: PrivateDocumentCreateFromShieldedPoolTransitionOptionsJs,
    ) -> WasmDppResult<PrivateDocumentCreateFromShieldedPoolTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        let data_contract_id: IdentifierWasm =
            crate::utils::try_from_options(&options, "dataContractId")?;
        let actions = actions_from_js_options(js_opts, "actions")?;

        let fields: PrivateDocumentCreateFromShieldedPoolTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(PrivateDocumentCreateFromShieldedPoolTransitionWasm(
            PrivateDocumentCreateFromShieldedPoolTransition::V0(
                PrivateDocumentCreateFromShieldedPoolTransitionV0 {
                    data_contract_id: data_contract_id.into(),
                    document_type_name: fields.document_type_name,
                    entries: fields.entries.into_iter().map(BinaryData::new).collect(),
                    actions: actions.into_iter().map(Into::into).collect(),
                    value_balance: fields.value_balance,
                    anchor,
                    proof: fields.proof,
                    binding_signature,
                },
            ),
        ))
    }

    /// The contract that defines the private document type. Read-only, like every field the
    /// Orchard binding signature commits to: a wrapper that changed it would produce a
    /// transition consensus must reject.
    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    /// The private document type the entries are appended to.
    #[wasm_bindgen(getter = "documentTypeName")]
    pub fn document_type_name(&self) -> String {
        self.0.document_type_name().to_string()
    }

    /// The entries, in the order they are appended.
    #[wasm_bindgen(getter = "entries")]
    pub fn entries(&self) -> Array {
        self.0
            .entries()
            .iter()
            .map(|entry| Uint8Array::from(entry.as_slice()))
            .collect()
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

    /// Credits leaving the credit pool: the bundle's value balance, which is the fee.
    #[wasm_bindgen(getter = "valueBalance")]
    pub fn value_balance(&self) -> u64 {
        self.0.value_balance()
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
            &StateTransition::PrivateDocumentCreateFromShieldedPool(self.0.clone()),
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
    ) -> WasmDppResult<PrivateDocumentCreateFromShieldedPoolTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes_untrusted(&bytes)?;
        match st {
            StateTransition::PrivateDocumentCreateFromShieldedPool(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected PrivateDocumentCreateFromShieldedPool",
            )),
        }
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(
        hex: String,
    ) -> WasmDppResult<PrivateDocumentCreateFromShieldedPoolTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        PrivateDocumentCreateFromShieldedPoolTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(
        base64: String,
    ) -> WasmDppResult<PrivateDocumentCreateFromShieldedPoolTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        PrivateDocumentCreateFromShieldedPoolTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransition::PrivateDocumentCreateFromShieldedPool(self.0.clone()).into()
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<PrivateDocumentCreateFromShieldedPoolTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();
        match rs_st {
            StateTransition::PrivateDocumentCreateFromShieldedPool(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected PrivateDocumentCreateFromShieldedPool",
            )),
        }
    }
}

impl_wasm_conversions_inner!(
    PrivateDocumentCreateFromShieldedPoolTransitionWasm,
    PrivateDocumentCreateFromShieldedPoolTransition,
    PrivateDocumentCreateFromShieldedPoolTransition,
    PrivateDocumentCreateFromShieldedPoolTransitionObjectJs,
    PrivateDocumentCreateFromShieldedPoolTransitionJSONJs
);

impl_wasm_type_info!(
    PrivateDocumentCreateFromShieldedPoolTransitionWasm,
    PrivateDocumentCreateFromShieldedPoolTransition
);
