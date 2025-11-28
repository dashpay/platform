use crate::asset_lock_proof::AssetLockProofWasm;
use crate::core_script::CoreScriptWasm;
use crate::enums::keys::purpose::PurposeWasm;
use crate::enums::withdrawal::PoolingWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::state_transitions::StateTransitionWasm;
use crate::utils::IntoWasm;
use dpp::identity::KeyID;
use dpp::identity::core_script::CoreScript;
use dpp::identity::state_transition::OptionallyAssetLockProved;
use dpp::platform_value::Identifier;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned, StateTransitionLike};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "IdentityCreditWithdrawalTransition")]
pub struct IdentityCreditWithdrawalTransitionWasm(IdentityCreditWithdrawalTransition);

#[wasm_bindgen(js_class = IdentityCreditWithdrawalTransition)]
impl IdentityCreditWithdrawalTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "IdentityCreditWithdrawalTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "IdentityCreditWithdrawalTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_identity_id: JsValue,
        amount: u64,
        core_fee_per_byte: u32,
        js_pooling: JsValue,
        js_output_script: &JsValue,
        nonce: Option<IdentityNonce>,
        user_fee_increase: Option<UserFeeIncrease>,
    ) -> WasmDppResult<IdentityCreditWithdrawalTransitionWasm> {
        let pooling = PoolingWasm::try_from(js_pooling)?;
        let identity_id: Identifier = IdentifierWasm::try_from(&js_identity_id)?.into();

        let output_script: Option<CoreScript> = match js_output_script.is_undefined() {
            true => None,
            false => Some(
                js_output_script
                    .to_wasm::<CoreScriptWasm>("CoreScript")?
                    .clone()
                    .into(),
            ),
        };

        Ok(IdentityCreditWithdrawalTransitionWasm(
            IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                amount,
                identity_id,
                output_script,
                core_fee_per_byte,
                pooling: pooling.into(),
                nonce: nonce.unwrap_or(0),
                user_fee_increase: user_fee_increase.unwrap_or(0),
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
        ))
    }

    #[wasm_bindgen(getter = "outputScript")]
    pub fn get_output_script(&self) -> Option<CoreScriptWasm> {
        self.0.output_script().map(|script| script.into())
    }

    #[wasm_bindgen(getter = "pooling")]
    pub fn get_pooling(&self) -> String {
        PoolingWasm::from(self.0.pooling()).into()
    }

    #[wasm_bindgen(getter = "identityId")]
    pub fn get_identity_id(&self) -> IdentifierWasm {
        IdentifierWasm::from(self.0.identity_id())
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn get_user_fee_increase(&self) -> UserFeeIncrease {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "nonce")]
    pub fn get_nonce(&self) -> IdentityNonce {
        self.0.nonce()
    }

    #[wasm_bindgen(getter = "amount")]
    pub fn get_amount(&self) -> u64 {
        self.0.amount()
    }

    #[wasm_bindgen(js_name = "getPurposeRequirement")]
    pub fn get_purpose_requirement(&self) -> Vec<String> {
        self.0
            .purpose_requirement()
            .iter()
            .map(|purpose| PurposeWasm::from(*purpose).into())
            .collect()
    }

    #[wasm_bindgen(js_name = "getModifiedDataIds")]
    pub fn get_modified_data_ids(&self) -> Vec<IdentifierWasm> {
        self.0
            .modified_data_ids()
            .iter()
            .map(|id| (*id).into())
            .collect()
    }

    #[wasm_bindgen(js_name = "getOptionalAssetLockProof")]
    pub fn get_optional_asset_lock_proof(&self) -> JsValue {
        match self.0.optional_asset_lock_proof() {
            Some(asset_lock) => JsValue::from(AssetLockProofWasm::from(asset_lock.clone())),
            None => JsValue::null(),
        }
    }

    #[wasm_bindgen(setter = "outputScript")]
    pub fn set_output_script(&mut self, js_script: &JsValue) -> WasmDppResult<()> {
        if js_script.is_undefined() {
            self.0.set_output_script(None);
        } else {
            let script: CoreScriptWasm = js_script.to_wasm::<CoreScriptWasm>("CoreScript")?.clone();
            self.0.set_output_script(Some(script.clone().into()));
        }

        Ok(())
    }

    #[wasm_bindgen(setter = "pooling")]
    pub fn set_pooling(&mut self, js_pooling: JsValue) -> WasmDppResult<()> {
        let pooling: PoolingWasm = PoolingWasm::try_from(js_pooling)?;
        self.0.set_pooling(pooling.into());
        Ok(())
    }

    #[wasm_bindgen(setter = "identityId")]
    pub fn set_identity_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_identity_id: JsValue,
    ) -> WasmDppResult<()> {
        let identity_id = IdentifierWasm::try_from(&js_identity_id)?.into();

        self.0.set_identity_id(identity_id);
        Ok(())
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.0.set_user_fee_increase(user_fee_increase);
    }

    #[wasm_bindgen(setter = "nonce")]
    pub fn set_nonce(&mut self, nonce: IdentityNonce) {
        self.0.set_nonce(nonce)
    }

    #[wasm_bindgen(setter = "amount")]
    pub fn set_amount(&mut self, amount: u64) {
        self.0.set_amount(amount)
    }

    #[wasm_bindgen(setter = "coreFeePerByte")]
    pub fn set_core_fee_per_byte(&mut self, fee_per_byte: u32) {
        self.0.set_core_fee_per_byte(fee_per_byte)
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(getter = "coreFeePerByte")]
    pub fn get_core_fee_per_byte(&self) -> u32 {
        self.0.core_fee_per_byte()
    }

    #[wasm_bindgen(js_name = "getSignableBytes")]
    pub fn get_signable_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.signable_bytes()?)
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn get_signature_public_key_id(&self) -> KeyID {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(&mut self, signature_public_key_id: KeyID) {
        self.0.set_signature_public_key_id(signature_public_key_id)
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityCreditWithdrawalTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityCreditWithdrawalTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentityCreditWithdrawalTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityCreditWithdrawalTransitionWasm::from_bytes(bytes)
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

    #[wasm_bindgen(js_name = "base64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityCreditWithdrawalTransitionWasm> {
        let rs_transition =
            IdentityCreditWithdrawalTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(IdentityCreditWithdrawalTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<IdentityCreditWithdrawalTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::IdentityCreditWithdrawal(st) => {
                Ok(IdentityCreditWithdrawalTransitionWasm(st))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}
