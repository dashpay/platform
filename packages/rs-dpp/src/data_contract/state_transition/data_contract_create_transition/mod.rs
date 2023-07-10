use std::collections::BTreeMap;
use std::convert::TryInto;

use crate::serialization_traits::PlatformSerializable;
use platform_serialization::PlatformSignable;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

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

use crate::serialization_traits::{PlatformDeserializable, Signable};

use bincode::{config, Decode, Encode};

mod action;
pub mod apply_data_contract_create_transition_factory;
pub mod builder;
mod serialize_for_signing;
pub mod validation;

pub use action::{
    DataContractCreateTransitionAction, DATA_CONTRACT_CREATE_TRANSITION_ACTION_VERSION,
};

pub mod property_names {
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const DATA_CONTRACT: &str = "dataContract";
    pub const DATA_CONTRACT_ID: &str = "dataContract.$id";
    pub const DATA_CONTRACT_OWNER_ID: &str = "dataContract.ownerId";
    pub const DATA_CONTRACT_ENTROPY: &str = "dataContract.entropy";
    pub const ENTROPY: &str = "entropy";
    pub const DATA_CONTRACT_PROTOCOL_VERSION: &str = "dataContract.protocolVersion";
    pub const SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
    pub const SIGNATURE: &str = "signature";
}

pub const IDENTIFIER_FIELDS: [&str; 2] = [
    property_names::DATA_CONTRACT_ID,
    property_names::DATA_CONTRACT_OWNER_ID,
];
pub const BINARY_FIELDS: [&str; 3] = [
    property_names::ENTROPY,
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
pub struct DataContractCreateTransition {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    pub data_contract: DataContract,
    pub entropy: Bytes32,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl std::default::Default for DataContractCreateTransition {
    fn default() -> Self {
        DataContractCreateTransition {
            protocol_version: Default::default(),
            transition_type: StateTransitionType::DataContractCreate,
            entropy: Bytes32::default(),
            signature_public_key_id: 0,
            signature: BinaryData::default(),
            data_contract: Default::default(),
        }
    }
}

impl DataContractCreateTransition {
    pub fn from_raw_object(
        mut raw_object: Value,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        Ok(DataContractCreateTransition {
            protocol_version: raw_object.get_integer(PROTOCOL_VERSION)?,
            signature: raw_object
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_object
                .get_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            entropy: raw_object
                .remove_optional_bytes_32(ENTROPY)
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
        mut raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        Ok(DataContractCreateTransition {
            protocol_version: raw_data_contract_create_transition
                .get_integer(PROTOCOL_VERSION)
                .map_err(ProtocolError::ValueError)?,
            signature: raw_data_contract_create_transition
                .remove_optional_binary_data(SIGNATURE)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            signature_public_key_id: raw_data_contract_create_transition
                .remove_optional_integer(SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            entropy: raw_data_contract_create_transition
                .remove_optional_bytes_32(ENTROPY)
                .map_err(ProtocolError::ValueError)?
                .unwrap_or_default(),
            data_contract: DataContract::from_raw_object(
                raw_data_contract_create_transition
                    .remove(DATA_CONTRACT)
                    .ok_or(ProtocolError::DecodingError(
                        "data contract missing on state transition".to_string(),
                    ))?,
            )?,
            ..Default::default()
        })
    }

    pub fn get_data_contract(&self) -> &DataContract {
        &self.data_contract
    }

    pub fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }

    pub fn set_data_contract(&mut self, data_contract: DataContract) {
        self.data_contract = data_contract;
    }

    /// Returns ID of the created contract
    pub fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract.id]
    }

    pub fn clean_value(value: &mut Value) -> Result<(), platform_value::Error> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }
}

impl StateTransitionIdentitySigned for DataContractCreateTransition {
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

impl StateTransitionLike for DataContractCreateTransition {
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

impl StateTransitionConvert for DataContractCreateTransition {
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
    use crate::data_contract::CreatedDataContract;
    use integer_encoding::VarInt;

    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::util::json_value::JsonValueExt;
    use crate::version;

    use super::*;

    struct TestData {
        state_transition: DataContractCreateTransition,
        created_data_contract: CreatedDataContract,
    }

    fn get_test_data() -> TestData {
        let created_data_contract = get_data_contract_fixture(None);

        let state_transition = DataContractCreateTransition::from_raw_object(Value::from([
            (PROTOCOL_VERSION, version::LATEST_VERSION.into()),
            (ENTROPY, created_data_contract.entropy_used.into()),
            (
                DATA_CONTRACT,
                created_data_contract.data_contract.to_object().unwrap(),
            ),
        ]))
        .expect("state transition should be created without errors");

        TestData {
            created_data_contract,
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
            StateTransitionType::DataContractCreate,
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
            data.created_data_contract
                .data_contract
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
            0,
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

        assert_eq!(
            <Bytes32 as Into<String>>::into(data.created_data_contract.entropy_used),
            json_object
                .remove_into::<String>(ENTROPY)
                .expect("the entropy should be present")
        )
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
            &data.created_data_contract.data_contract.owner_id,
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

    mod platform_serializable {
        use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
        use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable};

        #[test]
        fn should_serialize_config() {
            let mut data = super::get_test_data();
            data.state_transition.data_contract.config.keeps_history = true;
            let state_transition_bytes = data
                .state_transition
                .serialize()
                .expect("state transition should be serialized");

            assert!(data.state_transition.data_contract.config.keeps_history);

            let restored = DataContractCreateTransition::deserialize(&state_transition_bytes)
                .expect("state transition should be deserialized");

            assert!(restored.data_contract.config.keeps_history);
        }
    }
}
