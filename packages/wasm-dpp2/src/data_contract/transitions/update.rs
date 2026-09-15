use crate::data_contract::DataContractWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::version::{PlatformVersionLikeJs, PlatformVersionWasm};
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{DataContract, IdentityNonce};
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use dpp::state_transition::StateTransition;
use dpp::state_transition::StateTransitionOwned;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::version::{FeatureVersion, ProtocolVersion, TryFromPlatformVersioned};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * What a delta-based (V1) DataContractUpdateTransition does to the description.
 */
export type DataContractDescriptionUpdate = "keep" | "clear" | { set: string };

/**
 * A full-contract (V0) DataContractUpdateTransition serialized as a plain object.
 */
export interface DataContractUpdateTransitionV0Object {
    $formatVersion: "0";
    dataContract: DataContractObject;
    "$identity-contract-nonce": bigint;
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature?: Uint8Array;
}

/**
 * A delta-based (V1) DataContractUpdateTransition serialized as a plain object.
 * It carries only what changed, keyed by the contract, its owner and the
 * version the update produces.
 */
export interface DataContractUpdateTransitionV1Object {
    $formatVersion: "1";
    "$identity-contract-nonce": bigint;
    dataContractId: Identifier;
    ownerId: Identifier;
    version: number;
    config?: DataContractConfig;
    updatedSchemaDefs?: Record<string, object>;
    newSchemaDefs?: Record<string, object>;
    updatedDocumentSchemas?: Record<string, object>;
    newDocumentSchemas?: Record<string, object>;
    newGroups?: Record<number, Group>;
    newTokens?: Record<number, TokenConfiguration>;
    addKeywords?: string[];
    removeKeywords?: string[];
    description?: DataContractDescriptionUpdate;
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature?: Uint8Array;
}

/**
 * DataContractUpdateTransition serialized as a plain object.
 */
export type DataContractUpdateTransitionObject =
    | DataContractUpdateTransitionV0Object
    | DataContractUpdateTransitionV1Object;

/**
 * A full-contract (V0) DataContractUpdateTransition serialized as JSON.
 */
export interface DataContractUpdateTransitionV0JSON {
    $formatVersion: "0";
    dataContract: DataContractJSON;
    "$identity-contract-nonce": string;
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature?: string;
}

/**
 * A delta-based (V1) DataContractUpdateTransition serialized as JSON
 * (with string identifiers).
 */
export interface DataContractUpdateTransitionV1JSON {
    $formatVersion: "1";
    "$identity-contract-nonce": string;
    dataContractId: string;
    ownerId: string;
    version: number;
    config?: DataContractConfig;
    updatedSchemaDefs?: Record<string, object>;
    newSchemaDefs?: Record<string, object>;
    updatedDocumentSchemas?: Record<string, object>;
    newDocumentSchemas?: Record<string, object>;
    newGroups?: Record<number, object>;
    newTokens?: Record<number, object>;
    addKeywords?: string[];
    removeKeywords?: string[];
    description?: DataContractDescriptionUpdate;
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature?: string;
}

/**
 * DataContractUpdateTransition serialized as JSON.
 */
export type DataContractUpdateTransitionJSON =
    | DataContractUpdateTransitionV0JSON
    | DataContractUpdateTransitionV1JSON;
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DataContractUpdateTransitionObject")]
    pub type DataContractUpdateTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "DataContractUpdateTransitionJSON")]
    pub type DataContractUpdateTransitionJSONJs;
}

#[wasm_bindgen(js_name = "DataContractUpdateTransition")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct DataContractUpdateTransitionWasm(DataContractUpdateTransition);

#[wasm_bindgen(js_class = DataContractUpdateTransition)]
impl DataContractUpdateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(js_name = "dataContract")] data_contract: &DataContractWasm,
        #[wasm_bindgen(js_name = "identityNonce")] identity_nonce: IdentityNonce,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractUpdateTransitionWasm> {
        let platform_version = PlatformVersionWasm::try_from(platform_version)?;

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
            DataContractUpdateTransition::deserialize_from_bytes_untrusted(bytes.as_slice())?;

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

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Base64))
    }

    #[wasm_bindgen(getter = "featureVersion")]
    pub fn feature_version(&self) -> FeatureVersion {
        self.0.feature_version()
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

        self.0
            .set_data_contract(data_contract_serialization_format)?;

        Ok(())
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    /// The contract the update targets: embedded in a V0 transition, named by a V1 delta.
    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "ownerId")]
    pub fn owner_id(&self) -> IdentifierWasm {
        self.0.owner_id().into()
    }

    #[wasm_bindgen(js_name = "getDataContract")]
    pub fn get_data_contract(
        &self,
        #[wasm_bindgen(js_name = "fullValidation")] full_validation: Option<bool>,
        #[wasm_bindgen(js_name = "platformVersion")] platform_version: PlatformVersionLikeJs,
    ) -> WasmDppResult<DataContractWasm> {
        let platform_version = PlatformVersionWasm::try_from(platform_version)?;

        let data_contract_serialization_format = self.0.data_contract().ok_or_else(|| {
            WasmDppError::invalid_argument(
                "a delta-based data contract update transition carries no full contract",
            )
        })?;

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
        #[wasm_bindgen(js_name = "stateTransition")] state_transition: &StateTransitionWasm,
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

impl_wasm_conversions_inner!(
    DataContractUpdateTransitionWasm,
    DataContractUpdateTransition,
    DataContractUpdateTransition,
    DataContractUpdateTransitionObjectJs,
    DataContractUpdateTransitionJSONJs
);
impl_wasm_type_info!(
    DataContractUpdateTransitionWasm,
    DataContractUpdateTransition
);
