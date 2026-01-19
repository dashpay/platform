use crate::asset_lock_proof::AssetLockProofWasm;
use crate::version::PlatformVersionWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::identity::transitions::public_key_in_creation::IdentityPublicKeyInCreationWasm;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::utils::IntoWasm;
use dpp::identity::state_transition::AssetLockProved;
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::{StateTransition, StateTransitionLike, StateTransitionSingleSigned};
use js_sys::{Array, Object, Reflect};
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreateTransitionOptions {
    #[serde(default)]
    signature: Option<Vec<u8>>,
    #[serde(default)]
    user_fee_increase: Option<UserFeeIncrease>,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
export interface IdentityCreateTransitionOptions {
    publicKeys: IdentityPublicKeyInCreation[];
    assetLock: AssetLockProof;
    signature?: Uint8Array;
    userFeeIncrease?: number;
}
"#;

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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(unchecked_param_type = "IdentityCreateTransitionOptions")] options: JsValue,
    ) -> WasmDppResult<IdentityCreateTransitionWasm> {
        let object = Object::from(options.clone());

        // Extract publicKeys (required array)
        let js_public_keys = Reflect::get(&object, &JsValue::from_str("publicKeys"))
            .map_err(|e| WasmDppError::invalid_argument(format!("Missing publicKeys: {:?}", e)))?;
        let js_public_keys_array = Array::from(&js_public_keys);
        let public_keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_js_value(&js_public_keys_array)?;

        // Extract assetLock (required)
        let js_asset_lock = Reflect::get(&object, &JsValue::from_str("assetLock"))
            .map_err(|e| WasmDppError::invalid_argument(format!("Missing assetLock: {:?}", e)))?;
        let asset_lock = js_asset_lock
            .to_wasm::<AssetLockProofWasm>("AssetLockProof")?
            .clone();

        // Extract simple fields via serde
        let opts: IdentityCreateTransitionOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(IdentityCreateTransitionWasm(IdentityCreateTransition::V0(
            IdentityCreateTransitionV0 {
                public_keys: public_keys.iter().map(|key| key.clone().into()).collect(),
                asset_lock_proof: asset_lock.clone().into(),
                user_fee_increase: opts.user_fee_increase.unwrap_or(0),
                signature: BinaryData::from(opts.signature.unwrap_or_default()),
                identity_id: asset_lock.create_identifier()?.into(),
            },
        )))
    }

    #[wasm_bindgen(js_name = "default")]
    pub fn default(
        #[wasm_bindgen(unchecked_param_type = "PlatformVersionLike")]
        platform_version: JsValue,
    ) -> WasmDppResult<IdentityCreateTransitionWasm> {
        let platform_version = PlatformVersionWasm::try_from(platform_version)?;

        IdentityCreateTransition::default_versioned(&platform_version.into())
            .map_err(|err| WasmDppError::generic(err.to_string()))
            .map(Into::into)
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityCreateTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityCreateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentityCreateTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityCreateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityCreateTransitionWasm> {
        let rs_transition = IdentityCreateTransition::deserialize_from_bytes(bytes.as_slice())?;

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

    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
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
    pub fn get_signable_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.signable_bytes()?)
    }

    #[wasm_bindgen(getter = "assetLock")]
    pub fn get_asset_lock_proof(&self) -> AssetLockProofWasm {
        AssetLockProofWasm::from(self.0.asset_lock_proof().clone())
    }

    #[wasm_bindgen(setter = "publicKeys")]
    pub fn set_public_keys(&mut self, js_public_keys: &js_sys::Array) -> WasmDppResult<()> {
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
    pub fn set_asset_lock_proof(&mut self, proof: AssetLockProofWasm) -> WasmDppResult<()> {
        self.0.set_asset_lock_proof(proof.into())?;
        Ok(())
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::IdentityCreate(self.clone().0))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<IdentityCreateTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::IdentityCreate(st) => Ok(IdentityCreateTransitionWasm(st)),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl_wasm_conversions!(IdentityCreateTransitionWasm, IdentityCreateTransition);
impl_wasm_type_info!(IdentityCreateTransitionWasm, IdentityCreateTransition);
