use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{BinaryData, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
    Convertible, ProtocolError,
};

use super::property_names::*;

mod action;
pub mod apply_data_contract_update_transition_factory;
mod serialize_for_signing;
pub mod validation;

pub use action::{
    DataContractUpdateTransitionAction, DATA_CONTRACT_UPDATE_TRANSITION_ACTION_VERSION,
};

pub mod property_names {
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const DATA_CONTRACT: &str = "dataContract";
    pub const DATA_CONTRACT_ID: &str = "dataContract.$id";
    pub const DATA_CONTRACT_OWNER_ID: &str = "dataContract.ownerId";
    pub const DATA_CONTRACT_ENTROPY: &str = "dataContract.entropy";
    pub const DATA_CONTRACT_PROTOCOL_VERSION: &str = "dataContract.protocolVersion";
    pub const SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
    pub const SIGNATURE: &str = "signature";
}

pub const IDENTIFIER_FIELDS: [&str; 2] = [
    property_names::DATA_CONTRACT_ID,
    property_names::DATA_CONTRACT_OWNER_ID,
];
pub const BINARY_FIELDS: [&str; 2] = [
    property_names::DATA_CONTRACT_ENTROPY,
    property_names::SIGNATURE,
];
pub const U32_FIELDS: [&str; 2] = [
    property_names::PROTOCOL_VERSION,
    property_names::DATA_CONTRACT_PROTOCOL_VERSION,
];

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PartialEq,
    PlatformSignable,
)]
#[serde(rename_all = "camelCase")]
#[platform_error_type(ProtocolError)]
pub struct DataContractUpdateTransition {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    pub data_contract: DataContract,
    #[exclude_from_sig_hash]
    pub signature_public_key_id: KeyID,
    #[exclude_from_sig_hash]
    pub signature: BinaryData,
}

impl std::default::Default for DataContractUpdateTransition {
    fn default() -> Self {
        DataContractUpdateTransition {
            protocol_version: Default::default(),
            transition_type: StateTransitionType::DataContractUpdate,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
            data_contract: Default::default(),
        }
    }
}

impl DataContractUpdateTransition {
    pub fn from_raw_object(
        mut raw_object: Value,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        Ok(DataContractUpdateTransition {
            protocol_version: raw_object.get_integer(PROTOCOL_VERSION)?,
            signature: raw_object
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_object
                .get_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            data_contract: DataContract::from_raw_object(
                raw_object.remove(DATA_CONTRACT).map_err(|_| {
                    ProtocolError::DecodingError(
                        "data contract missing on state transition".to_string(),
                    )
                })?,
            )?,
            ..Default::default()
        })
    }

    pub fn from_value_map(
        mut raw_data_contract_update_transition: BTreeMap<String, Value>,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        Ok(DataContractUpdateTransition {
            protocol_version: raw_data_contract_update_transition
                .get_integer(PROTOCOL_VERSION)
                .map_err(ProtocolError::ValueError)?,
            signature: raw_data_contract_update_transition
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_data_contract_update_transition
                .remove_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            data_contract: DataContract::from_raw_object(
                raw_data_contract_update_transition
                    .remove(DATA_CONTRACT)
                    .ok_or(ProtocolError::DecodingError(
                        "data contract missing on state transition".to_string(),
                    ))?,
            )?,
            ..Default::default()
        })
    }

    pub fn clean_value(value: &mut Value) -> Result<(), platform_value::Error> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }

    pub fn get_data_contract(&self) -> &DataContract {
        &self.data_contract
    }

    pub fn set_data_contract(&mut self, data_contract: DataContract) {
        self.data_contract = data_contract;
    }
}

impl StateTransitionIdentitySigned for DataContractUpdateTransition {
    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier {
        &self.data_contract.owner_id
    }

    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        Some(self.signature_public_key_id)
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::KeyID) {
        self.signature_public_key_id = key_id
    }
}

impl StateTransitionLike for DataContractUpdateTransition {
    /// Returns ID of the created contract
    fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract.id]
    }

    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        self.transition_type
    }
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &BinaryData {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = signature
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = BinaryData::new(signature)
    }
}

impl StateTransitionConvert for DataContractUpdateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, ENTROPY]
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object(skip_signature)
            .and_then(|value| value.try_into().map_err(ProtocolError::ValueError))
    }

    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut object: Value = platform_value::to_value(self)?;
        if skip_signature {
            Self::signature_property_paths()
                .into_iter()
                .try_for_each(|path| {
                    object
                        .remove_values_matching_path(path)
                        .map_err(ProtocolError::ValueError)
                        .map(|_| ())
                })?;
        }
        object.insert(String::from(DATA_CONTRACT), self.data_contract.to_object()?)?;
        Ok(object)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut object: Value = platform_value::to_value(self)?;
        if skip_signature {
            Self::signature_property_paths()
                .into_iter()
                .try_for_each(|path| {
                    object
                        .remove_values_matching_path(path)
                        .map_err(ProtocolError::ValueError)
                        .map(|_| ())
                })?;
        }
        object.insert(
            String::from(DATA_CONTRACT),
            self.data_contract.to_cleaned_object()?,
        )?;
        Ok(object)
    }
}

#[cfg(test)]
mod test {
    use crate::util::json_value::JsonValueExt;
    use integer_encoding::VarInt;
    use std::convert::TryInto;

    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::version;

    use super::*;

    struct TestData {
        state_transition: DataContractUpdateTransition,
        data_contract: DataContract,
    }

    fn get_test_data() -> TestData {
        let data_contract = get_data_contract_fixture(None);

        let value_map = BTreeMap::from([
            (
                PROTOCOL_VERSION.to_string(),
                Value::U32(version::LATEST_VERSION),
            ),
            (
                DATA_CONTRACT.to_string(),
                data_contract.clone().try_into().unwrap(),
            ),
        ]);

        let state_transition = DataContractUpdateTransition::from_value_map(value_map)
            .expect("state transition should be created without errors");

        TestData {
            data_contract,
            state_transition,
        }
    }

    #[test]
    fn should_return_protocol_version() {
        let data = get_test_data();
        assert_eq!(
            version::LATEST_VERSION,
            data.state_transition.get_protocol_version()
        )
    }

    #[test]
    fn should_return_transition_type() {
        let data = get_test_data();
        assert_eq!(
            StateTransitionType::DataContractUpdate,
            data.state_transition.get_type()
        );
    }

    #[test]
    fn should_return_data_contract() {
        let data = get_test_data();

        assert_eq!(
            data.state_transition
                .get_data_contract()
                .to_json_object()
                .expect("conversion to object shouldn't fail"),
            data.data_contract
                .to_json_object()
                .expect("conversion to object shouldn't fail")
        );
    }

    #[test]
    fn should_return_state_transition_in_json_format() {
        let data = get_test_data();
        let mut json_object = data
            .state_transition
            .to_json(false)
            .expect("conversion to JSON shouldn't fail");

        assert_eq!(
            version::LATEST_VERSION,
            json_object
                .get_u64(PROTOCOL_VERSION)
                .expect("the protocol version should be present") as u32
        );

        assert_eq!(
            4,
            json_object
                .get_u64(TRANSITION_TYPE)
                .expect("the transition type should be present") as u8
        );
        assert_eq!(
            0,
            json_object
                .get_u64(SIGNATURE_PUBLIC_KEY_ID)
                .expect("default public key id should be defined"),
        );
        assert_eq!(
            "",
            json_object
                .remove_into::<String>(SIGNATURE)
                .expect("default string value for signature should be present")
        );
    }

    #[test]
    fn should_return_serialized_state_transition_to_buffer() {
        let data = get_test_data();
        let state_transition_bytes = data
            .state_transition
            .to_cbor_buffer(false)
            .expect("state transition should be converted to buffer");
        let (protocol_version, _) =
            u32::decode_var(state_transition_bytes.as_ref()).expect("expected to decode");
        assert_eq!(version::LATEST_VERSION, protocol_version)
    }

    #[test]
    fn should_return_owner_id() {
        let data = get_test_data();
        assert_eq!(
            &data.data_contract.owner_id,
            data.state_transition.get_owner_id()
        );
    }

    #[test]
    fn is_data_contract_state_transition() {
        let data = get_test_data();
        assert!(data.state_transition.is_data_contract_state_transition());
        assert!(!data.state_transition.is_document_state_transition());
        assert!(!data.state_transition.is_identity_state_transition());
    }
}
