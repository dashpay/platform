mod action;
pub mod apply_data_contract_create_transition_factory;
mod v0;
mod v0_action;

use crate::data_contract::property_names::ENTROPY;
use crate::data_contract::state_transition::data_contract_create_transition::property_names::{
    DATA_CONTRACT, SIGNATURE, SIGNATURE_PUBLIC_KEY_ID,
};

use crate::data_contract::DataContract;
use crate::document::document_transition::document_base_transition::JsonValue;
use crate::identity::KeyID;
use crate::serialization_traits::PlatformDeserializable;
use crate::serialization_traits::{PlatformSerializable, Signable};
use crate::state_transition::{
    StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike, StateTransitionType,
};
use crate::version::FeatureVersion;
use crate::{Convertible, ProtocolError};
pub use action::DataContractCreateTransitionAction;
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{BinaryData, Bytes32, Identifier, Value};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt;
pub use v0::*;
pub use v0_action::*;

pub mod property_names {
    pub const STATE_TRANSITION_PROTOCOL_VERSION: &str = "version";
    pub const DATA_CONTRACT: &str = "dataContract";
    pub const DATA_CONTRACT_ID: &str = "dataContract.$id";
    pub const DATA_CONTRACT_OWNER_ID: &str = "dataContract.ownerId";
    pub const DATA_CONTRACT_ENTROPY: &str = "dataContract.entropy";
    pub const DATA_CONTRACT_PROTOCOL_VERSION: &str = "dataContract.protocolVersion";
    pub const ENTROPY: &str = "entropy";
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
    property_names::STATE_TRANSITION_PROTOCOL_VERSION,
    property_names::DATA_CONTRACT_PROTOCOL_VERSION,
];

pub type DataContractCreateTransitionLatest = DataContractCreateTransitionV0;

#[derive(Debug, Clone, PlatformDeserialize, PlatformSerialize, Encode, Decode, From, PartialEq)]
#[platform_error_type(ProtocolError)]
pub enum DataContractCreateTransition {
    V0(DataContractCreateTransitionV0),
}

impl Serialize for DataContractCreateTransition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(None)?;

        match *self {
            DataContractCreateTransition::V0(ref v0) => {
                state.serialize_entry("type", &StateTransitionType::DataContractCreate)?;
                state.serialize_entry("version", &0u16)?;
                state.serialize_entry("dataContract", &v0.data_contract)?;
                state.serialize_entry("entropy", &v0.entropy)?;
                state.serialize_entry("signaturePublicKeyId", &v0.signature_public_key_id)?;
                state.serialize_entry("signature", &v0.signature)?;
            }
        }

        state.end()
    }
}

struct DataContractCreateTransitionVisitor;

impl<'de> Visitor<'de> for DataContractCreateTransitionVisitor {
    type Value = DataContractCreateTransition;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map representing a DataContractCreateTransition")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut version: Option<u16> = None;
        let mut data_contract: Option<DataContract> = None;
        let mut entropy: Option<Bytes32> = None;
        let mut signature_public_key_id: Option<KeyID> = None;
        let mut signature: Option<BinaryData> = None;

        while let Some(key) = map.next_key()? {
            match key {
                "version" => {
                    version = Some(map.next_value()?);
                }
                "dataContract" => {
                    data_contract = Some(map.next_value()?);
                }
                "entropy" => {
                    entropy = Some(map.next_value()?);
                }
                "signaturePublicKeyId" => {
                    signature_public_key_id = Some(map.next_value()?);
                }
                "signature" => {
                    signature = Some(map.next_value()?);
                }
                _ => {}
            }
        }

        let version = version.ok_or_else(|| serde::de::Error::missing_field("version"))?;
        let data_contract =
            data_contract.ok_or_else(|| serde::de::Error::missing_field("dataContract"))?;
        let entropy = entropy.ok_or_else(|| serde::de::Error::missing_field("entropy"))?;
        let signature_public_key_id = signature_public_key_id
            .ok_or_else(|| serde::de::Error::missing_field("signaturePublicKeyId"))?;
        let signature = signature.ok_or_else(|| serde::de::Error::missing_field("signature"))?;

        match version {
            0 => Ok(DataContractCreateTransition::V0(
                DataContractCreateTransitionV0 {
                    data_contract,
                    entropy,
                    signature_public_key_id,
                    signature,
                },
            )),
            _ => Err(serde::de::Error::unknown_variant(
                &format!("{}", version),
                &[],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for DataContractCreateTransition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(DataContractCreateTransitionVisitor)
    }
}

impl From<DataContract> for DataContractCreateTransition {
    fn from(value: DataContract) -> Self {
        DataContractCreateTransitionV0::from(value).into()
    }
}

impl Signable for DataContractCreateTransition {
    fn signable_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        match self {
            DataContractCreateTransition::V0(transition) => transition.signable_bytes(),
        }
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
        object.insert(
            String::from(DATA_CONTRACT),
            self.data_contract().to_object()?,
        )?;
        Ok(object)
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object(skip_signature)
            .and_then(|value| value.try_into().map_err(ProtocolError::ValueError))
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
            self.data_contract().to_cleaned_object()?,
        )?;
        Ok(object)
    }
}

impl StateTransitionLike for DataContractCreateTransition {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            DataContractCreateTransition::V0(transition) => transition.get_modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            DataContractCreateTransition::V0(_) => 0,
        }
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            DataContractCreateTransition::V0(transition) => transition.state_transition_type(),
        }
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        match self {
            DataContractCreateTransition::V0(transition) => transition.signature(),
        }
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            DataContractCreateTransition::V0(transition) => transition.set_signature(signature),
        }
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        match self {
            DataContractCreateTransition::V0(transition) => {
                transition.set_signature_bytes(signature)
            }
        }
    }
}

impl StateTransitionIdentitySigned for DataContractCreateTransition {
    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier {
        &self.data_contract().owner_id
    }

    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        match self {
            DataContractCreateTransition::V0(transition) => {
                Some(transition.signature_public_key_id)
            }
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            DataContractCreateTransition::V0(transition) => {
                transition.signature_public_key_id = key_id
            }
        }
    }
}

impl DataContractCreateTransition {
    pub fn from_raw_object(
        mut raw_object: Value,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        let version: u8 = raw_object
            .remove_integer(property_names::STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;
        match version {
            0 => Ok(DataContractCreateTransitionV0::from_raw_object(raw_object)?.into()),
            n => Err(ProtocolError::UnknownProtocolVersionError(format!(
                "Unknown DataContractCreateTransition version {n}"
            ))),
        }
    }

    pub fn from_value_map(
        mut raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        let version: u8 = raw_data_contract_create_transition
            .remove_integer(property_names::STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;

        match version {
            0 => Ok(DataContractCreateTransitionV0::from_value_map(
                raw_data_contract_create_transition,
            )?
            .into()),
            n => Err(ProtocolError::UnknownProtocolVersionError(format!(
                "Unknown DataContractCreateTransition version {n}"
            ))),
        }
    }

    pub fn data_contract(&self) -> &DataContract {
        match self {
            DataContractCreateTransition::V0(transition) => &transition.data_contract,
        }
    }

    pub fn set_data_contract(&mut self, data_contract: DataContract) {
        match self {
            DataContractCreateTransition::V0(transition) => {
                transition.data_contract = data_contract
            }
        }
    }

    pub fn entropy(&self) -> Bytes32 {
        match self {
            DataContractCreateTransition::V0(transition) => transition.entropy,
        }
    }

    pub fn entropy_ref(&self) -> &Bytes32 {
        match self {
            DataContractCreateTransition::V0(transition) => &transition.entropy,
        }
    }

    pub fn set_entropy(&mut self, entropy: Bytes32) {
        match self {
            DataContractCreateTransition::V0(transition) => transition.entropy = entropy,
        }
    }

    pub fn state_transition_version(&self) -> u16 {
        match self {
            DataContractCreateTransition::V0(_) => 0,
        }
    }

    /// Returns ID of the created contract
    pub fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract().id]
    }

    pub fn clean_value(value: &mut Value) -> Result<(), platform_value::Error> {
        DataContractCreateTransitionLatest::clean_value(value)
    }
}

#[cfg(test)]
mod test {
    use crate::data_contract::state_transition::property_names::TRANSITION_TYPE;
    use crate::data_contract::CreatedDataContract;
    use integer_encoding::VarInt;
    use platform_value::Bytes32;

    use crate::data_contract::state_transition::data_contract_create_transition::property_names::{
        ENTROPY, SIGNATURE, SIGNATURE_PUBLIC_KEY_ID, STATE_TRANSITION_PROTOCOL_VERSION,
    };
    use crate::state_transition::StateTransitionType;
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::util::json_value::JsonValueExt;
    use crate::version::LATEST_PLATFORM_VERSION;
    use crate::{version, Convertible};

    use super::*;

    struct TestData {
        state_transition: DataContractCreateTransition,
        created_data_contract: CreatedDataContract,
    }

    fn get_test_data() -> TestData {
        let created_data_contract = get_data_contract_fixture(None);

        let state_transition = DataContractCreateTransition::from_raw_object(Value::from([
            (
                property_names::STATE_TRANSITION_PROTOCOL_VERSION,
                LATEST_PLATFORM_VERSION
                    .state_transitions
                    .contract_create_state_transition
                    .default_current_version
                    .into(),
            ),
            (
                property_names::ENTROPY,
                created_data_contract.entropy_used.into(),
            ),
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
            LATEST_PLATFORM_VERSION
                .state_transitions
                .contract_create_state_transition
                .default_current_version,
            data.state_transition.state_transition_protocol_version()
        )
    }

    #[test]
    fn should_return_transition_type() {
        let data = get_test_data();
        assert_eq!(
            StateTransitionType::DataContractCreate,
            data.state_transition.state_transition_type()
        );
    }

    #[test]
    fn should_return_data_contract() {
        let data = get_test_data();

        assert_eq!(
            data.state_transition
                .data_contract()
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
                .get_u64(STATE_TRANSITION_PROTOCOL_VERSION)
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
}
