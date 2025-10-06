use crate::asset_lock_proof::AssetLockProofWasm;
use crate::enums::platform::PlatformVersionWasm;
use crate::identifier::IdentifierWasm;
use crate::identity_transitions::public_key_in_creation::IdentityPublicKeyInCreationWasm;
use crate::state_transition::StateTransitionWasm;
use crate::utils::WithJsError;
use dpp::identity::state_transition::AssetLockProved;
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::decode;
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen(js_name = "IdentityCreateTransition")]
#[derive(Clone)]
pub struct IdentityCreateTransitionWasm(IdentityCreateTransition);

impl From<IdentityCreateTransition> for IdentityCreateTransitionWasm {
    fn from(val: IdentityCreateTransition) -> Self {
        IdentityCreateTransitionWasm(val)
    }
}

impl From<IdentityCreateTransitionWasm> for IdentityCreateTransition {
    fn from(val: IdentityCreateTransitionWasm) -> Self {
        val.0
    }
}

#[wasm_bindgen(js_class = IdentityCreateTransition)]
impl IdentityCreateTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "IdentityCreateTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "IdentityCreateTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        js_public_keys: &js_sys::Array,
        asset_lock: &AssetLockProofWasm,
        signature: Option<Vec<u8>>,
        user_fee_increase: Option<UserFeeIncrease>,
    ) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let public_keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_js_value(js_public_keys)?;

        Ok(IdentityCreateTransitionWasm(IdentityCreateTransition::V0(
            IdentityCreateTransitionV0 {
                public_keys: public_keys.iter().map(|key| key.clone().into()).collect(),
                asset_lock_proof: asset_lock.clone().into(),
                user_fee_increase: user_fee_increase.unwrap_or(0),
                signature: BinaryData::from(signature.unwrap_or(vec![])),
                identity_id: asset_lock.create_identifier()?.into(),
            },
        )))
    }

    #[wasm_bindgen(js_name = "default")]
    pub fn default(js_platform_version: JsValue) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let platform_version = PlatformVersionWasm::try_from(js_platform_version)?;

        IdentityCreateTransition::default_versioned(&platform_version.into())
            .map_err(|err| JsValue::from_str(&*err.to_string()))
            .map(Into::into)
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let bytes = decode(hex.as_str(), Hex).map_err(JsError::from)?;

        IdentityCreateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let bytes = decode(base64.as_str(), Base64).map_err(JsError::from)?;

        IdentityCreateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> Result<Vec<u8>, JsValue> {
        self.0.serialize_to_bytes().with_js_error()
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let rs_transition =
            IdentityCreateTransition::deserialize_from_bytes(bytes.as_slice()).with_js_error()?;

        Ok(IdentityCreateTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(getter = "publicKeys")]
    pub fn get_public_keys(&self) -> Vec<IdentityPublicKeyInCreationWasm> {
        self.0
            .public_keys()
            .iter()
            .map(|key| IdentityPublicKeyInCreationWasm::from(key.clone()))
            .collect()
    }

    #[wasm_bindgen(js_name = "getIdentifier")]
    pub fn get_identity_id(&self) -> IdentifierWasm {
        self.0.identity_id().into()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn get_user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(js_name = "getSignableBytes")]
    pub fn get_signable_bytes(&self) -> Result<Vec<u8>, JsValue> {
        self.0.signable_bytes().with_js_error()
    }

    #[wasm_bindgen(getter = "assetLock")]
    pub fn get_asset_lock_proof(&self) -> AssetLockProofWasm {
        AssetLockProofWasm::from(self.0.asset_lock_proof().clone())
    }

    #[wasm_bindgen(setter = "publicKeys")]
    pub fn set_public_keys(&mut self, js_public_keys: &js_sys::Array) -> Result<(), JsValue> {
        let public_keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_js_value(js_public_keys)?;

        self.0.set_public_keys(
            public_keys
                .iter()
                .map(|key| IdentityPublicKeyInCreation::from(key.clone()))
                .collect(),
        );

        Ok(())
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, amount: u16) {
        self.0.set_user_fee_increase(amount)
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(setter = "assetLock")]
    pub fn set_asset_lock_proof(&mut self, proof: AssetLockProofWasm) -> Result<(), JsValue> {
        self.0.set_asset_lock_proof(proof.into()).with_js_error()
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::IdentityCreate(self.clone().0))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::IdentityCreate(st) => Ok(IdentityCreateTransitionWasm(st)),
            _ => Err(JsValue::from_str(
                &"Invalid state document_transition type)",
            )),
        }
    }
}
