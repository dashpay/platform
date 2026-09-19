use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options, try_to_u16, try_to_u32, try_to_u64};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::v0::{
    ContractUserModerationAction, ContractUserModerationTransitionV0,
};
use dpp::state_transition::{
    StateTransition, StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned,
    StateTransitionSingleSigned,
};
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const CONTRACT_USER_MODERATION_TS: &str = r#"
/**
 * What a moderation transition does to one identity on a contract. The wire shape of the
 * action, as the transition's `action` field carries it.
 */
export type ContractUserModerationActionJSON =
  | { $type: "ban"; identityId: string }
  | { $type: "unban"; identityId: string }
  | { $type: "suspend"; identityId: string; until: number | string }
  | { $type: "unsuspend"; identityId: string };

/**
 * Bans, unbans, suspends or unsuspends one identity on a moderated data contract (protocol
 * version 14). Signed by the contract owner or a moderator its config names, with a CRITICAL
 * authentication key, under the signer's contract-scoped nonce.
 */
export interface ContractUserModerationTransitionOptions {
    /** The moderator that signs */
    ownerId: IdentifierLike;
    /** The moderated contract */
    dataContractId: IdentifierLike;
    /** The signer's nonce for the contract */
    identityContractNonce: bigint;
    /** What is done */
    action: "ban" | "unban" | "suspend" | "unsuspend";
    /** The identity the action targets */
    identityId: IdentifierLike;
    /** For a suspend: the block time, in milliseconds, at which the suspension lapses */
    until?: bigint;
    userFeeIncrease?: number;
}

/**
 * ContractUserModeration serialized as a plain object.
 */
export interface ContractUserModerationObject {
    ownerId: Uint8Array;
    dataContractId: Uint8Array;
    identityContractNonce: bigint;
    action: { $type: string; identityId: Uint8Array; until?: bigint };
    userFeeIncrease: number;
    signature?: Uint8Array;
    signaturePublicKeyId?: number;
}

/**
 * ContractUserModeration serialized as JSON.
 */
export interface ContractUserModerationJSON {
    ownerId: string;
    dataContractId: string;
    identityContractNonce: number | string;
    action: ContractUserModerationActionJSON;
    userFeeIncrease: number;
    signature?: string;
    signaturePublicKeyId?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContractUserModerationTransitionOptions")]
    pub type ContractUserModerationTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "ContractUserModerationObject")]
    pub type ContractUserModerationObjectJs;

    #[wasm_bindgen(typescript_type = "ContractUserModerationJSON")]
    pub type ContractUserModerationJSONJs;
}

/// Serde struct for the primitive fields of the options
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractUserModerationOptionsInput {
    identity_contract_nonce: u64,
    action: String,
    #[serde(default)]
    until: Option<u64>,
    /// `undefined` reaches serde as a unit value, so the fee is read as an option and defaulted
    #[serde(default)]
    user_fee_increase: Option<UserFeeIncrease>,
}

#[wasm_bindgen(js_name = "ContractUserModeration")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ContractUserModerationWasm(ContractUserModerationTransition);

impl From<ContractUserModerationTransition> for ContractUserModerationWasm {
    fn from(val: ContractUserModerationTransition) -> Self {
        ContractUserModerationWasm(val)
    }
}

impl From<ContractUserModerationWasm> for ContractUserModerationTransition {
    fn from(val: ContractUserModerationWasm) -> Self {
        val.0
    }
}

/// The action for a name and a target, `until` given for a suspend.
pub fn moderation_action_from_parts(
    action: &str,
    identity_id: dpp::prelude::Identifier,
    until: Option<u64>,
) -> WasmDppResult<ContractUserModerationAction> {
    match action {
        "ban" => Ok(ContractUserModerationAction::Ban { identity_id }),
        "unban" => Ok(ContractUserModerationAction::Unban { identity_id }),
        "suspend" => {
            let until = until
                .ok_or_else(|| WasmDppError::invalid_argument("a suspend action needs `until`"))?;
            Ok(ContractUserModerationAction::Suspend { identity_id, until })
        }
        "unsuspend" => Ok(ContractUserModerationAction::Unsuspend { identity_id }),
        other => Err(WasmDppError::invalid_argument(format!(
            "unknown moderation action `{other}`: expected ban, unban, suspend or unsuspend"
        ))),
    }
}

#[wasm_bindgen(js_class = ContractUserModeration)]
impl ContractUserModerationWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: ContractUserModerationTransitionOptionsJs,
    ) -> WasmDppResult<ContractUserModerationWasm> {
        // Extract complex types first (borrows &options)
        let owner_id: IdentifierWasm = try_from_options(&options, "ownerId")?;
        let data_contract_id: IdentifierWasm = try_from_options(&options, "dataContractId")?;
        let identity_id: IdentifierWasm = try_from_options(&options, "identityId")?;

        // Deserialize primitive fields via serde last (consumes options)
        let input: ContractUserModerationOptionsInput =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let action = moderation_action_from_parts(&input.action, identity_id.into(), input.until)?;

        Ok(ContractUserModerationWasm(
            ContractUserModerationTransition::V0(ContractUserModerationTransitionV0 {
                owner_id: owner_id.into(),
                data_contract_id: data_contract_id.into(),
                identity_contract_nonce: input.identity_contract_nonce,
                action,
                user_fee_increase: input.user_fee_increase.unwrap_or_default(),
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
        ))
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ContractUserModerationWasm> {
        let rs_transition =
            ContractUserModerationTransition::deserialize_from_bytes_untrusted(bytes.as_slice())?;

        Ok(ContractUserModerationWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<ContractUserModerationWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        ContractUserModerationWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<ContractUserModerationWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        ContractUserModerationWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(setter = "ownerId")]
    pub fn set_owner_id(&mut self, owner_id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_owner_id(owner_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "dataContractId")]
    pub fn set_data_contract_id(&mut self, id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_data_contract_id(id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0
            .set_identity_contract_nonce(try_to_u64(nonce, "identityContractNonce")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(
        &mut self,
        #[wasm_bindgen(js_name = "publicKeyId")] public_key_id: &js_sys::Number,
    ) -> WasmDppResult<()> {
        self.0
            .set_signature_public_key_id(try_to_u32(public_key_id, "signaturePublicKeyId")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, amount: &js_sys::Number) -> WasmDppResult<()> {
        self.0
            .set_user_fee_increase(try_to_u16(amount, "userFeeIncrease")?);
        Ok(())
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn signature_public_key_id(&self) -> u32 {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "ownerId")]
    pub fn owner_id(&self) -> IdentifierWasm {
        use dpp::state_transition::StateTransitionOwned;
        self.0.owner_id().into()
    }

    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn identity_contract_nonce(&self) -> u64 {
        self.0.identity_contract_nonce()
    }

    /// The action's name: ban, unban, suspend or unsuspend
    #[wasm_bindgen(getter = "action")]
    pub fn action(&self) -> String {
        self.0.action().name().to_string()
    }

    /// The identity the action targets
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
        self.0.target_identity_id().into()
    }

    /// For a suspend, the block time in milliseconds at which the suspension lapses
    #[wasm_bindgen(getter = "until")]
    pub fn until(&self) -> Option<u64> {
        self.0.action().until()
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<ContractUserModerationWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::ContractUserModeration(st) => Ok(ContractUserModerationWasm(st)),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl ContractUserModerationWasm {
    pub fn set_signature_binary_data(&mut self, data: BinaryData) {
        self.0.set_signature(data)
    }
}

impl_wasm_conversions_inner!(
    ContractUserModerationWasm,
    ContractUserModerationTransition,
    ContractUserModeration,
    ContractUserModerationObjectJs,
    ContractUserModerationJSONJs
);

impl_wasm_type_info!(ContractUserModerationWasm, ContractUserModeration);
