use crate::identifier::IdentifierWASM;
use crate::state_transition::StateTransitionWASM;
use crate::utils::WithJsError;
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{Identifier, UserFeeIncrease};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned, StateTransitionLike};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen(js_name = IdentityCreditTransferWASM)]
#[derive(Clone)]
pub struct IdentityCreditTransferWASM(IdentityCreditTransferTransition);

#[wasm_bindgen]
impl IdentityCreditTransferWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "IdentityCreditTransferWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "IdentityCreditTransferWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        amount: u64,
        js_sender: &JsValue,
        js_recipient: &JsValue,
        nonce: u64,
        user_fee_increase: Option<UserFeeIncrease>,
    ) -> Result<IdentityCreditTransferWASM, JsValue> {
        let sender: Identifier = IdentifierWASM::try_from(js_sender)?.into();

        let recipient: Identifier = IdentifierWASM::try_from(js_recipient)?.into();

        Ok(IdentityCreditTransferWASM(
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0 {
                identity_id: sender,
                recipient_id: recipient,
                amount,
                nonce,
                user_fee_increase: user_fee_increase.unwrap_or(0),
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
        ))
    }

    #[wasm_bindgen(js_name = "bytes")]
    pub fn to_bytes(&self) -> Result<Vec<u8>, JsValue> {
        self.0.serialize_to_bytes().with_js_error()
    }

    #[wasm_bindgen(js_name = "hex")]
    pub fn to_hex(&self) -> Result<String, JsValue> {
        Ok(encode(
            self.0.serialize_to_bytes().with_js_error()?.as_slice(),
            Hex,
        ))
    }

    #[wasm_bindgen(js_name = "base64")]
    pub fn to_base64(&self) -> Result<String, JsValue> {
        Ok(encode(
            self.0.serialize_to_bytes().with_js_error()?.as_slice(),
            Base64,
        ))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<IdentityCreditTransferWASM, JsValue> {
        let rs_transition =
            IdentityCreditTransferTransition::deserialize_from_bytes(bytes.as_slice())
                .with_js_error()?;

        Ok(IdentityCreditTransferWASM(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> Result<IdentityCreditTransferWASM, JsValue> {
        IdentityCreditTransferWASM::from_bytes(decode(hex.as_str(), Hex).map_err(JsError::from)?)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(hex: String) -> Result<IdentityCreditTransferWASM, JsValue> {
        IdentityCreditTransferWASM::from_bytes(decode(hex.as_str(), Base64).map_err(JsError::from)?)
    }

    #[wasm_bindgen(setter = "recipientId")]
    pub fn set_recipient_id(&mut self, js_recipient: &JsValue) -> Result<(), JsValue> {
        let recipient: Identifier = IdentifierWASM::try_from(js_recipient)?.into();

        Ok(self.0.set_recipient_id(recipient))
    }

    #[wasm_bindgen(setter = "senderId")]
    pub fn set_sender_id(&mut self, js_sender: &JsValue) -> Result<(), JsValue> {
        let sender: Identifier = IdentifierWASM::try_from(js_sender)?.into();

        Ok(self.0.set_identity_id(sender))
    }

    #[wasm_bindgen(setter = "amount")]
    pub fn set_amount(&mut self, amount: u64) {
        self.0.set_amount(amount)
    }

    #[wasm_bindgen(setter = "nonce")]
    pub fn set_nonce(&mut self, nonce: u64) {
        self.0.set_nonce(nonce)
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(&mut self, public_key_id: u32) {
        self.0.set_signature_public_key_id(public_key_id)
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, amount: u16) {
        self.0.set_user_fee_increase(amount)
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(js_name = "getSignableBytes")]
    pub fn get_signable_bytes(&self) -> Result<Vec<u8>, JsValue> {
        self.0.signable_bytes().with_js_error()
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn get_signature_public_key_id(&self) -> u32 {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn get_user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "recipientId")]
    pub fn get_recipient_id(&self) -> IdentifierWASM {
        self.0.recipient_id().into()
    }

    #[wasm_bindgen(getter = "senderId")]
    pub fn get_identity_id(&self) -> IdentifierWASM {
        self.0.identity_id().into()
    }

    #[wasm_bindgen(getter = "amount")]
    pub fn get_amount(&self) -> u64 {
        self.0.amount()
    }

    #[wasm_bindgen(getter = "nonce")]
    pub fn get_nonce(&self) -> u64 {
        self.0.nonce()
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWASM {
        StateTransitionWASM::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWASM,
    ) -> Result<IdentityCreditTransferWASM, JsValue> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::IdentityCreditTransfer(st) => Ok(IdentityCreditTransferWASM(st)),
            _ => Err(JsValue::from_str(&"Invalid state transition type)")),
        }
    }
}

impl IdentityCreditTransferWASM {
    pub fn set_signature_binary_data(&mut self, data: BinaryData) {
        self.0.set_signature(data)
    }
}
