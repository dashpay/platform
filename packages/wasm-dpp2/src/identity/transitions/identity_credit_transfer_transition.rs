use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options, try_to_u16, try_to_u32, try_to_u64};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use dpp::state_transition::{
    StateTransition, StateTransitionIdentitySigned, StateTransitionLike,
    StateTransitionSingleSigned,
};
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const CREDIT_TRANSFER_OPTIONS_TS: &str = r#"
export interface IdentityCreditTransferOptions {
    amount: bigint;
    senderId: IdentifierLike;
    recipientId: IdentifierLike;
    nonce: bigint;
    userFeeIncrease?: number;
}

/**
 * IdentityCreditTransfer serialized as a plain object.
 */
export interface IdentityCreditTransferObject {
    amount: bigint;
    senderId: Uint8Array;
    recipientId: Uint8Array;
    nonce: bigint;
    userFeeIncrease: number;
    signature?: Uint8Array;
    signaturePublicKeyId?: number;
}

/**
 * IdentityCreditTransfer serialized as JSON.
 */
export interface IdentityCreditTransferJSON {
    amount: string;
    senderId: string;
    recipientId: string;
    nonce: string;
    userFeeIncrease: number;
    signature?: string;
    signaturePublicKeyId?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityCreditTransferOptions")]
    pub type IdentityCreditTransferOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityCreditTransferObject")]
    pub type IdentityCreditTransferObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityCreditTransferJSON")]
    pub type IdentityCreditTransferJSONJs;
}

/// Serde struct for IdentityCreditTransferOptions (primitives only)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreditTransferOptionsInput {
    amount: u64,
    nonce: u64,
    #[serde(default)]
    user_fee_increase: UserFeeIncrease,
}

#[wasm_bindgen(js_name = "IdentityCreditTransfer")]
#[derive(Clone)]
pub struct IdentityCreditTransferWasm(IdentityCreditTransferTransition);

#[wasm_bindgen(js_class = IdentityCreditTransfer)]
impl IdentityCreditTransferWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: IdentityCreditTransferOptionsJs,
    ) -> WasmDppResult<IdentityCreditTransferWasm> {
        // Extract complex types first (borrows &options)
        let sender_id: IdentifierWasm = try_from_options(&options, "senderId")?;
        let recipient_id: IdentifierWasm = try_from_options(&options, "recipientId")?;

        // Deserialize primitive fields via serde last (consumes options)
        let input: IdentityCreditTransferOptionsInput =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(IdentityCreditTransferWasm(
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0 {
                identity_id: sender_id.into(),
                recipient_id: recipient_id.into(),
                amount: input.amount,
                nonce: input.nonce,
                user_fee_increase: input.user_fee_increase,
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
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityCreditTransferWasm> {
        let rs_transition =
            IdentityCreditTransferTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(IdentityCreditTransferWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityCreditTransferWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityCreditTransferWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(hex: String) -> WasmDppResult<IdentityCreditTransferWasm> {
        let bytes =
            decode(hex.as_str(), Base64).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityCreditTransferWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(setter = "recipientId")]
    pub fn set_recipient_id(&mut self, recipient: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_recipient_id(recipient.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "senderId")]
    pub fn set_sender_id(&mut self, sender: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_identity_id(sender.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "amount")]
    pub fn set_amount(&mut self, amount: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0.set_amount(try_to_u64(amount, "amount")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "nonce")]
    pub fn set_nonce(&mut self, nonce: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0.set_nonce(try_to_u64(nonce, "nonce")?);
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

    #[wasm_bindgen(js_name = "getSignableBytes")]
    pub fn get_signable_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.signable_bytes()?)
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn signature_public_key_id(&self) -> u32 {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "recipientId")]
    pub fn recipient_id(&self) -> IdentifierWasm {
        self.0.recipient_id().into()
    }

    #[wasm_bindgen(getter = "senderId")]
    pub fn sender_id(&self) -> IdentifierWasm {
        self.0.identity_id().into()
    }

    #[wasm_bindgen(getter = "amount")]
    pub fn amount(&self) -> u64 {
        self.0.amount()
    }

    #[wasm_bindgen(getter = "nonce")]
    pub fn nonce(&self) -> u64 {
        self.0.nonce()
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<IdentityCreditTransferWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::IdentityCreditTransfer(st) => Ok(IdentityCreditTransferWasm(st)),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl IdentityCreditTransferWasm {
    pub fn set_signature_binary_data(&mut self, data: BinaryData) {
        self.0.set_signature(data)
    }
}

impl_wasm_conversions!(
    IdentityCreditTransferWasm,
    IdentityCreditTransferTransition,
    IdentityCreditTransferObjectJs,
    IdentityCreditTransferJSONJs
);

impl_wasm_type_info!(IdentityCreditTransferWasm, IdentityCreditTransfer);
