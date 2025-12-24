use crate::data_contract::DataContractWasm;
use crate::enums::platform::PlatformVersionWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::state_transitions::StateTransitionWasm;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{DataContract, IdentityNonce};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::StateTransition;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::version::{FeatureVersion, ProtocolVersion, TryFromPlatformVersioned};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "DataContractUpdateTransition")]
pub struct DataContractUpdateTransitionWasm(DataContractUpdateTransition);

#[wasm_bindgen(js_class = DataContractUpdateTransition)]
impl DataContractUpdateTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DataContractUpdateTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DataContractUpdateTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        data_contract: &DataContractWasm,
        identity_nonce: IdentityNonce,
        js_platform_version: JsValue,
    ) -> WasmDppResult<DataContractUpdateTransitionWasm> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let rs_data_contract_update_transition =
            DataContractUpdateTransition::try_from_platform_versioned(
                (DataContract::from(data_contract.clone()), identity_nonce),
                &platform_version.into(),
            )?;

        Ok(DataContractUpdateTransitionWasm(
            rs_data_contract_update_transition,
        ))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<DataContractUpdateTransitionWasm> {
        let rs_data_contract_update_transition: DataContractUpdateTransition =
            DataContractUpdateTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(DataContractUpdateTransitionWasm(
            rs_data_contract_update_transition,
        ))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<DataContractUpdateTransitionWasm> {
        let bytes = decode(hex.as_str(), Hex)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        DataContractUpdateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<DataContractUpdateTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        DataContractUpdateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        self.0.serialize_to_bytes().map_err(Into::into)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "base64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Base64))
    }

    #[wasm_bindgen(getter = "featureVersion")]
    pub fn get_feature_version(&self) -> FeatureVersion {
        self.0.feature_version()
    }

    #[wasm_bindgen(js_name = "verifyProtocolVersion")]
    pub fn verify_protocol_version(
        &self,
        protocol_version: ProtocolVersion,
    ) -> WasmDppResult<bool> {
        self.0
            .verify_protocol_version(protocol_version)
            .map_err(Into::into)
    }

    #[wasm_bindgen(js_name = "setDataContract")]
    pub fn set_data_contract(
        &mut self,
        data_contract: &DataContractWasm,
        js_platform_version: JsValue,
    ) -> WasmDppResult<()> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let data_contract_serialization_format =
            DataContractInSerializationFormat::try_from_platform_versioned(
                DataContract::from(data_contract.clone()),
                &platform_version.into(),
            )?;

        self.0.set_data_contract(data_contract_serialization_format);

        Ok(())
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn get_identity_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(js_name = "getDataContract")]
    pub fn get_data_contract(
        &self,
        full_validation: Option<bool>,
        js_platform_version: JsValue,
    ) -> WasmDppResult<DataContractWasm> {
        let platform_version = match js_platform_version.is_undefined() {
            true => PlatformVersionWasm::default(),
            false => PlatformVersionWasm::try_from(js_platform_version)?,
        };

        let data_contract_serialization_format = self.0.data_contract();

        let mut validation_operations: Vec<ProtocolValidationOperation> = Vec::new();

        let rs_data_contract = DataContract::try_from_platform_versioned(
            data_contract_serialization_format.clone(),
            full_validation.unwrap_or(false),
            &mut validation_operations,
            &platform_version.into(),
        )?;

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
    ) -> WasmDppResult<DataContractUpdateTransitionWasm> {
        let rs_transition = StateTransition::from(state_transition.clone());

        match rs_transition {
            StateTransition::DataContractUpdate(state_transition) => {
                Ok(DataContractUpdateTransitionWasm(state_transition))
            }
            _ => Err(WasmDppError::invalid_argument("Incorrect transition type")),
        }
    }
}
