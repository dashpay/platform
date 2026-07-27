use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::platform_address::{
    PlatformAddressOutputWasm, outputs_from_js_options, outputs_to_btree_map,
};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_to_u16, try_to_u32, try_to_u64};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
use dpp::state_transition::{
    StateTransition, StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned,
    StateTransitionSingleSigned,
};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const CREDIT_TRANSFER_TO_ADDRESSES_OPTIONS_TS: &str = r#"
export interface IdentityCreditTransferToAddressesOptions {
    recipientAddresses: PlatformAddressOutput[];
    senderId: IdentifierLike;
    nonce: bigint;
    userFeeIncrease?: number;
}

/**
 * IdentityCreditTransferToAddresses serialized as a plain object.
 */
export interface IdentityCreditTransferToAddressesObject {
    recipientAddresses: PlatformAddressOutputObject[];
    senderId: Uint8Array;
    nonce: bigint;
    userFeeIncrease: number;
    signature?: Uint8Array;
    signaturePublicKeyId?: number;
}

/**
 * IdentityCreditTransferToAddresses serialized as JSON.
 */
export interface IdentityCreditTransferToAddressesJSON {
    recipientAddresses: PlatformAddressOutputJSON[];
    senderId: string;
    nonce: string;
    userFeeIncrease: number;
    signature?: string;
    signaturePublicKeyId?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityCreditTransferToAddressesOptions")]
    pub type IdentityCreditTransferToAddressesOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityCreditTransferToAddressesObject")]
    pub type IdentityCreditTransferToAddressesObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityCreditTransferToAddressesJSON")]
    pub type IdentityCreditTransferToAddressesJSONJs;
}

#[wasm_bindgen(js_name = "IdentityCreditTransferToAddresses")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct IdentityCreditTransferToAddressesTransitionWasm(
    IdentityCreditTransferToAddressesTransition,
);

#[wasm_bindgen(js_class = IdentityCreditTransferToAddresses)]
impl IdentityCreditTransferToAddressesTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: IdentityCreditTransferToAddressesOptionsJs,
    ) -> WasmDppResult<IdentityCreditTransferToAddressesTransitionWasm> {
        let js_opts: &wasm_bindgen::JsValue = options.as_ref();

        // Extract complex wasm-bindgen types manually
        let sender_id: IdentifierWasm = crate::utils::try_from_options(&options, "senderId")?;
        let recipient_outputs = outputs_from_js_options(js_opts, "recipientAddresses")?;

        // Extract simple fields
        let nonce: u64 =
            crate::utils::try_from_options_with(js_opts, "nonce", |v| try_to_u64(v, "nonce"))?;
        let user_fee_increase: UserFeeIncrease =
            crate::utils::try_from_options_optional_with(js_opts, "userFeeIncrease", |v| {
                crate::utils::try_to_u16(v, "userFeeIncrease")
            })?
            .unwrap_or(0);

        let recipient_addresses = outputs_to_btree_map(recipient_outputs)?;

        Ok(IdentityCreditTransferToAddressesTransitionWasm(
            IdentityCreditTransferToAddressesTransition::V0(
                IdentityCreditTransferToAddressesTransitionV0 {
                    identity_id: sender_id.into(),
                    recipient_addresses,
                    nonce,
                    user_fee_increase,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                },
            ),
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
    pub fn from_bytes(
        bytes: Vec<u8>,
    ) -> WasmDppResult<IdentityCreditTransferToAddressesTransitionWasm> {
        let rs_transition =
            IdentityCreditTransferToAddressesTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(IdentityCreditTransferToAddressesTransitionWasm(
            rs_transition,
        ))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityCreditTransferToAddressesTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityCreditTransferToAddressesTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(
        base64: String,
    ) -> WasmDppResult<IdentityCreditTransferToAddressesTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityCreditTransferToAddressesTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(getter = "recipientAddresses")]
    pub fn recipient_addresses(&self) -> Vec<PlatformAddressOutputWasm> {
        self.0
            .recipient_addresses()
            .iter()
            .map(|(address, amount)| PlatformAddressOutputWasm::new(*address, *amount))
            .collect()
    }

    #[wasm_bindgen(setter = "recipientAddresses")]
    pub fn set_recipient_addresses(
        &mut self,
        outputs: Vec<PlatformAddressOutputWasm>,
    ) -> WasmDppResult<()> {
        self.0
            .set_recipient_addresses(outputs_to_btree_map(outputs)?);
        Ok(())
    }

    #[wasm_bindgen(setter = "senderId")]
    pub fn set_sender_id(&mut self, sender: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_identity_id(sender.try_into()?);
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

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn signature_public_key_id(&self) -> u32 {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "senderId")]
    pub fn sender_id(&self) -> IdentifierWasm {
        self.0.identity_id().into()
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
    ) -> WasmDppResult<IdentityCreditTransferToAddressesTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::IdentityCreditTransferToAddresses(st) => {
                Ok(IdentityCreditTransferToAddressesTransitionWasm(st))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl IdentityCreditTransferToAddressesTransitionWasm {
    pub fn set_signature_binary_data(&mut self, data: BinaryData) {
        self.0.set_signature(data)
    }
}

impl_wasm_conversions_inner!(
    IdentityCreditTransferToAddressesTransitionWasm,
    IdentityCreditTransferToAddressesTransition,
    IdentityCreditTransferToAddresses,
    IdentityCreditTransferToAddressesObjectJs,
    IdentityCreditTransferToAddressesJSONJs
);

impl_wasm_type_info!(
    IdentityCreditTransferToAddressesTransitionWasm,
    IdentityCreditTransferToAddresses
);
