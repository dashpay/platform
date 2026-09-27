use crate::data_contract::DataContractWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::version::{PlatformVersionLikeJs, PlatformVersionWasm};
use dpp::contract_group::{ContractGroupMembership, ContractGroupRegistration};
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{DataContract, IdentityNonce};
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use dpp::state_transition::StateTransition;
use dpp::state_transition::data_contract_create_transition::accessors::{
    DataContractCreateTransitionAccessorsV0, DataContractCreateTransitionAccessorsV1,
};
use dpp::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0, DataContractCreateTransitionV1,
};
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::version::{
    FeatureVersion, PlatformVersion, ProtocolVersion, TryFromPlatformVersioned,
    TryIntoPlatformVersioned,
};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Registration of a contract group, carried by a version 1 DataContractCreateTransition.
 * The owner is the transition's signer and is not part of the object; `admins` are the
 * identities that may add members alongside the owner.
 */
export interface ContractGroupRegistrationObject {
    admins?: Uint8Array[];
    name?: string;
    description?: string;
}

/**
 * ContractGroupRegistrationObject serialized as JSON (identifiers as base58 strings).
 */
export interface ContractGroupRegistrationJSON {
    admins?: string[];
    name?: string;
    description?: string;
}

/**
 * Which part of the created contract joins a contract group.
 */
export type ContractGroupMemberObject = "contract" | { documentType: string } | { token: number };

/**
 * A declaration that a part of the created contract joins a contract group.
 */
export interface ContractGroupMembershipObject {
    contractGroupId: Uint8Array;
    member: ContractGroupMemberObject;
}

/**
 * ContractGroupMembershipObject serialized as JSON (identifiers as base58 strings).
 */
export interface ContractGroupMembershipJSON {
    contractGroupId: string;
    member: ContractGroupMemberObject;
}

/**
 * DataContractCreateTransition serialized as a plain object. Version 1 (protocol version 14)
 * adds `contractGroup` and `contractGroupMemberships`; both are absent on version 0 and
 * optional on version 1.
 */
export interface DataContractCreateTransitionObject {
    $formatVersion?: string;
    dataContract: DataContractObject;
    identityNonce: bigint;
    contractGroup?: ContractGroupRegistrationObject | null;
    contractGroupMemberships?: ContractGroupMembershipObject[];
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature?: Uint8Array;
}

/**
 * DataContractCreateTransition serialized as JSON.
 */
export interface DataContractCreateTransitionJSON {
    $formatVersion?: string;
    dataContract: DataContractJSON;
    identityNonce: string;
    contractGroup?: ContractGroupRegistrationJSON | null;
    contractGroupMemberships?: ContractGroupMembershipJSON[];
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DataContractCreateTransitionObject")]
    pub type DataContractCreateTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "DataContractCreateTransitionJSON")]
    pub type DataContractCreateTransitionJSONJs;
}

#[wasm_bindgen(js_name = "DataContractCreateTransition")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct DataContractCreateTransitionWasm(DataContractCreateTransition);

#[wasm_bindgen(js_class = DataContractCreateTransition)]
impl DataContractCreateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(js_name = "dataContract")] data_contract: &DataContractWasm,
        #[wasm_bindgen(js_name = "identityNonce")] identity_nonce: IdentityNonce,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractCreateTransitionWasm> {
        let rs_data_contract: DataContract = data_contract.clone().into();

        let platform_version = PlatformVersion::try_from(platform_version)?;

        let data_contract = rs_data_contract.try_into_platform_versioned(&platform_version)?;

        // The platform version decides the transition version, as the Rust SDK does: version 1
        // exists from protocol version 14 and carries the contract group fields.
        let rs_data_contract_transition = match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_create_state_transition
            .default_current_version
        {
            0 => DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
                data_contract,
                identity_nonce,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
            1 => DataContractCreateTransition::V1(DataContractCreateTransitionV1 {
                data_contract,
                identity_nonce,
                contract_group: None,
                contract_group_memberships: vec![],
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
            version => {
                return Err(WasmDppError::invalid_argument(format!(
                    "unknown data contract create transition version {}",
                    version
                )));
            }
        };

        Ok(DataContractCreateTransitionWasm(
            rs_data_contract_transition,
        ))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<DataContractCreateTransitionWasm> {
        let rs_data_contract_create_transition: DataContractCreateTransition =
            DataContractCreateTransition::deserialize_from_bytes_untrusted(bytes.as_slice())?;

        Ok(DataContractCreateTransitionWasm(
            rs_data_contract_create_transition,
        ))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<DataContractCreateTransitionWasm> {
        let bytes = decode(hex.as_str(), Hex)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        DataContractCreateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<DataContractCreateTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        DataContractCreateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        self.0.serialize_to_bytes().map_err(Into::into)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Base64))
    }

    #[wasm_bindgen(getter = "featureVersion")]
    pub fn feature_version(&self) -> FeatureVersion {
        self.0.feature_version()
    }

    /// The contract group this transition registers, as a `ContractGroupRegistrationObject`,
    /// or `undefined` when it registers none (a version 0 transition never does).
    #[wasm_bindgen(getter = "contractGroup")]
    pub fn contract_group(&self) -> WasmDppResult<JsValue> {
        match self.0.contract_group() {
            Some(registration) => serde_wasm_bindgen::to_value(registration)
                .map_err(|err| WasmDppError::serialization(err.to_string())),
            None => Ok(JsValue::UNDEFINED),
        }
    }

    /// Registers a contract group with this transition, or clears the registration with
    /// `undefined`. Needs a version 1 transition (protocol version 14).
    #[wasm_bindgen(js_name = "setContractGroup")]
    pub fn set_contract_group(&mut self, registration: JsValue) -> WasmDppResult<()> {
        let registration: Option<ContractGroupRegistration> =
            if registration.is_undefined() || registration.is_null() {
                None
            } else {
                Some(
                    serde_wasm_bindgen::from_value(registration)
                        .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?,
                )
            };
        match &mut self.0 {
            DataContractCreateTransition::V1(transition) => {
                transition.contract_group = registration;
                Ok(())
            }
            DataContractCreateTransition::V0(_) => Err(WasmDppError::invalid_argument(
                "contract groups need a version 1 data contract create transition (protocol version 14)",
            )),
        }
    }

    /// The contract group memberships the created contract declares, as
    /// `ContractGroupMembershipObject[]`; empty on a version 0 transition.
    #[wasm_bindgen(getter = "contractGroupMemberships")]
    pub fn contract_group_memberships(&self) -> WasmDppResult<JsValue> {
        serde_wasm_bindgen::to_value(self.0.contract_group_memberships())
            .map_err(|err| WasmDppError::serialization(err.to_string()))
    }

    /// Declares which contract groups the created contract, its document types or its tokens
    /// join. Needs a version 1 transition (protocol version 14).
    #[wasm_bindgen(js_name = "setContractGroupMemberships")]
    pub fn set_contract_group_memberships(&mut self, memberships: JsValue) -> WasmDppResult<()> {
        let memberships: Vec<ContractGroupMembership> = serde_wasm_bindgen::from_value(memberships)
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))?;
        match &mut self.0 {
            DataContractCreateTransition::V1(transition) => {
                transition.contract_group_memberships = memberships;
                Ok(())
            }
            DataContractCreateTransition::V0(_) => Err(WasmDppError::invalid_argument(
                "contract groups need a version 1 data contract create transition (protocol version 14)",
            )),
        }
    }

    #[wasm_bindgen(js_name = "verifyProtocolVersion")]
    pub fn verify_protocol_version(
        &self,
        #[wasm_bindgen(js_name = "protocolVersion")] protocol_version: ProtocolVersion,
    ) -> WasmDppResult<bool> {
        self.0
            .verify_protocol_version(protocol_version)
            .map_err(Into::into)
    }

    #[wasm_bindgen(js_name = "setDataContract")]
    pub fn set_data_contract(
        &mut self,
        #[wasm_bindgen(js_name = "dataContract")] data_contract: &DataContractWasm,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<()> {
        let platform_version = PlatformVersionWasm::try_from(platform_version)?;

        let data_contract_serialization_format =
            DataContractInSerializationFormat::try_from_platform_versioned(
                DataContract::from(data_contract.clone()),
                &platform_version.into(),
            )?;

        self.0.set_data_contract(data_contract_serialization_format);

        Ok(())
    }

    #[wasm_bindgen(getter = "identityNonce")]
    pub fn identity_nonce(&self) -> IdentityNonce {
        self.0.identity_nonce()
    }

    #[wasm_bindgen(js_name = "getDataContract")]
    pub fn get_data_contract(
        &self,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
        #[wasm_bindgen(js_name = "fullValidation")] full_validation: Option<bool>,
    ) -> WasmDppResult<DataContractWasm> {
        let platform_version = PlatformVersionWasm::try_from(platform_version)?;

        let rs_data_contract_serialization_format = self.0.data_contract();

        let mut validation_operations: Vec<ProtocolValidationOperation> = Vec::new();

        let rs_data_contract = DataContract::try_from_platform_versioned(
            rs_data_contract_serialization_format.clone(),
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
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
    ) -> WasmDppResult<DataContractCreateTransitionWasm> {
        let rs_transition = StateTransition::from(state_transition.clone());

        match rs_transition {
            StateTransition::DataContractCreate(state_transition) => {
                Ok(DataContractCreateTransitionWasm(state_transition))
            }
            _ => Err(WasmDppError::invalid_argument("Incorrect transition type")),
        }
    }
}

impl_wasm_conversions_inner!(
    DataContractCreateTransitionWasm,
    DataContractCreateTransition,
    DataContractCreateTransition,
    DataContractCreateTransitionObjectJs,
    DataContractCreateTransitionJSONJs
);
impl_wasm_type_info!(
    DataContractCreateTransitionWasm,
    DataContractCreateTransition
);
