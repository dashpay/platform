use crate::data_contract::DataContractWasm;
use crate::enums::platform::PlatformVersionWasm;
use crate::state_transition::StateTransitionWasm;
use crate::utils::WithJsError;
use dpp::ProtocolError;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{DataContract, IdentityNonce};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::StateTransition;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0,
};
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::version::{
    FeatureVersion, ProtocolVersion, TryFromPlatformVersioned, TryIntoPlatformVersioned,
};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen(js_name = "DataContractCreateTransition")]
pub struct DataContractCreateTransitionWasm(DataContractCreateTransition);

#[wasm_bindgen(js_class = DataContractCreateTransition)]
impl DataContractCreateTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DataContractCreateTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DataContractCreateTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        data_contract: &DataContractWasm,
        identity_nonce: IdentityNonce,
        js_platform_version: JsValue,
    ) -> Result<DataContractCreateTransitionWasm, JsValue> {
        let rs_data_contract: DataContract = data_contract.clone().into();

        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let rs_data_contract_in_serialized: Result<
            DataContractInSerializationFormat,
            ProtocolError,
        > = rs_data_contract.try_into_platform_versioned(&platform_version.into());

        let rs_data_contract_create_transition_v0: DataContractCreateTransitionV0 =
            DataContractCreateTransitionV0 {
                data_contract: rs_data_contract_in_serialized.with_js_error()?,
                identity_nonce,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            };

        let rs_data_contract_transition =
            DataContractCreateTransition::V0(rs_data_contract_create_transition_v0);

        Ok(DataContractCreateTransitionWasm(
            rs_data_contract_transition,
        ))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<DataContractCreateTransitionWasm, JsValue> {
        let rs_data_contract_create_transition: DataContractCreateTransition =
            DataContractCreateTransition::deserialize_from_bytes(bytes.as_slice())
                .with_js_error()?;

        Ok(DataContractCreateTransitionWasm(
            rs_data_contract_create_transition,
        ))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> Result<DataContractCreateTransitionWasm, JsValue> {
        let bytes = decode(hex.as_str(), Hex).map_err(JsError::from)?;

        DataContractCreateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> Result<DataContractCreateTransitionWasm, JsValue> {
        let bytes = decode(base64.as_str(), Base64).map_err(JsError::from)?;

        DataContractCreateTransitionWasm::from_bytes(bytes)
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

    #[wasm_bindgen(getter = "featureVersion")]
    pub fn get_feature_version(&self) -> FeatureVersion {
        self.0.feature_version()
    }

    #[wasm_bindgen(js_name = "verifyProtocolVersion")]
    pub fn verify_protocol_version(
        &self,
        protocol_version: ProtocolVersion,
    ) -> Result<bool, JsValue> {
        self.0
            .verify_protocol_version(protocol_version)
            .with_js_error()
    }

    #[wasm_bindgen(js_name = "setDataContract")]
    pub fn set_data_contract(
        &mut self,
        data_contract: &DataContractWasm,
        js_platform_version: JsValue,
    ) -> Result<(), JsValue> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let data_contract_serialization_format =
            DataContractInSerializationFormat::try_from_platform_versioned(
                DataContract::from(data_contract.clone()),
                &platform_version.into(),
            )
            .with_js_error()?;

        self.0.set_data_contract(data_contract_serialization_format);

        Ok(())
    }

    #[wasm_bindgen(getter = "identityNonce")]
    pub fn get_identity_nonce(&self) -> IdentityNonce {
        self.0.identity_nonce()
    }

    #[wasm_bindgen(js_name = "getDataContract")]
    pub fn get_data_contract(
        &self,
        js_platform_version: JsValue,
        full_validation: Option<bool>,
    ) -> Result<DataContractWasm, JsValue> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let rs_data_contract_serialization_format = self.0.data_contract();

        let mut validation_operations: Vec<ProtocolValidationOperation> = Vec::new();

        let rs_data_contract = DataContract::try_from_platform_versioned(
            rs_data_contract_serialization_format.clone(),
            full_validation.unwrap_or(false),
            &mut validation_operations,
            &platform_version.into(),
        )
        .with_js_error()?;

        Ok(DataContractWasm::from(rs_data_contract))
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        let rs_state_transition = StateTransition::from(self.0.clone());

        StateTransitionWasm::from(rs_state_transition)
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        state_transition: &StateTransitionWasm,
    ) -> Result<DataContractCreateTransitionWasm, JsValue> {
        let rs_transition = StateTransition::from(state_transition.clone());

        match rs_transition {
            StateTransition::DataContractCreate(state_transition) => {
                Ok(DataContractCreateTransitionWasm(state_transition))
            }
            _ => Err(JsValue::from("Incorrect transition type")),
        }
    }
}
