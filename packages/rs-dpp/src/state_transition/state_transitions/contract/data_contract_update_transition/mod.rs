use crate::data_contract::DataContract;
use crate::document::document_transition::document_base_transition::JsonValue;
use crate::identity::KeyID;
use crate::serialization_traits::PlatformDeserializable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::Signable;
use crate::state_transition::{
    StateTransitionFieldTypes, StateTransitionLike, StateTransitionType,
};
use crate::{Convertible, ProtocolError};
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{BinaryData, Identifier, Value};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt;
use platform_versioning::PlatformVersioned;

mod action;

mod v0;
mod v0_action;
mod fields;
#[cfg(feature = "json-object")]
mod json_conversion;
mod state_transition_like;
mod v0_methods;
#[cfg(feature = "platform-value")]
mod value_conversion;
mod identity_signed;

pub use fields::*;

use crate::version::FeatureVersion;
pub use action::DataContractUpdateTransitionAction;
pub use v0::*;
pub use v0_action::DataContractUpdateTransitionActionV0;


pub type DataContractUpdateTransitionLatest = DataContractUpdateTransitionV0;

#[derive(Debug, Clone, PlatformDeserialize, PlatformSerialize, PlatformVersioned, Encode, Decode, From, PartialEq)]
#[platform_error_type(ProtocolError)]
#[platform_version_path(state_transitions.contract_update_state_transition)]
pub enum DataContractUpdateTransition {
    V0(DataContractUpdateTransitionV0),
}

impl Serialize for DataContractUpdateTransition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(None)?;

        match *self {
            DataContractUpdateTransition::V0(ref v0) => {
                state.serialize_entry("type", &StateTransitionType::DataContractUpdate)?;
                state.serialize_entry("version", &0u16)?;
                state.serialize_entry("dataContract", &v0.data_contract)?;
                state.serialize_entry("signaturePublicKeyId", &v0.signature_public_key_id)?;
                state.serialize_entry("signature", &v0.signature)?;
            }
        }

        state.end()
    }
}
struct DataContractUpdateTransitionVisitor;

impl<'de> Visitor<'de> for DataContractUpdateTransitionVisitor {
    type Value = DataContractUpdateTransition;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map representing a DataContractUpdateTransition")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut version: Option<u16> = None;
        let mut data_contract: Option<DataContract> = None;
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
        let signature_public_key_id = signature_public_key_id
            .ok_or_else(|| serde::de::Error::missing_field("signaturePublicKeyId"))?;
        let signature = signature.ok_or_else(|| serde::de::Error::missing_field("signature"))?;

        match version {
            0 => Ok(DataContractUpdateTransition::V0(
                DataContractUpdateTransitionV0 {
                    data_contract,
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

impl<'de> Deserialize<'de> for DataContractUpdateTransition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(DataContractUpdateTransitionVisitor)
    }
}

impl Signable for DataContractUpdateTransition {
    fn signable_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        match self {
            DataContractUpdateTransition::V0(transition) => transition.signable_bytes(),
        }
    }
}

impl StateTransitionFieldTypes for DataContractUpdateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
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

impl StateTransitionLike for DataContractUpdateTransition {
    /// Returns ID of the created contract
    fn modified_data_ids(&self) -> Vec<Identifier> {
        match self {
            DataContractUpdateTransition::V0(transition) => transition.modified_data_ids(),
        }
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        match self {
            DataContractUpdateTransition::V0(_) => 0,
        }
    }
    /// returns the type of State Transition
    fn state_transition_type(&self) -> StateTransitionType {
        match self {
            DataContractUpdateTransition::V0(transition) => transition.state_transition_type(),
        }
    }
    /// returns the signature as a byte-array
    fn signature(&self) -> &BinaryData {
        match self {
            DataContractUpdateTransition::V0(transition) => transition.signature(),
        }
    }
    /// set a new signature
    fn set_signature(&mut self, signature: BinaryData) {
        match self {
            DataContractUpdateTransition::V0(transition) => transition.set_signature(signature),
        }
    }

    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        match self {
            DataContractUpdateTransition::V0(transition) => {
                transition.set_signature_bytes(signature)
            }
        }
    }
}

impl StateTransitionIdentitySignedV0 for DataContractUpdateTransition {
    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier {
        &self.data_contract().owner_id
    }

    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        match self {
            DataContractUpdateTransition::V0(transition) => {
                Some(transition.signature_public_key_id)
            }
        }
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        match self {
            DataContractUpdateTransition::V0(transition) => {
                transition.signature_public_key_id = key_id
            }
        }
    }
}

impl DataContractUpdateTransition {
    #[cfg(feature = "platform-value")]
    pub fn from_raw_object(
        mut raw_object: Value,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        let version: u8 = raw_object
            .remove_integer(property_names::STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;
        match version {
            0 => Ok(DataContractUpdateTransitionV0::from_raw_object(raw_object)?.into()),
            n => Err(ProtocolError::UnknownProtocolVersionError(format!(
                "Unknown DataContractUpdateTransition version {n}"
            ))),
        }
    }

    #[cfg(feature = "platform-value")]
    pub fn from_value_map(
        mut raw_data_contract_create_transition: BTreeMap<String, Value>,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        let version: u8 = raw_data_contract_create_transition
            .remove_integer(property_names::STATE_TRANSITION_PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?;

        match version {
            0 => Ok(DataContractUpdateTransitionV0::from_value_map(
                raw_data_contract_create_transition,
            )?
            .into()),
            n => Err(ProtocolError::UnknownProtocolVersionError(format!(
                "Unknown DataContractUpdateTransition version {n}"
            ))),
        }
    }

    pub fn data_contract(&self) -> &DataContract {
        match self {
            DataContractUpdateTransition::V0(transition) => &transition.data_contract,
        }
    }

    pub fn set_data_contract(&mut self, data_contract: DataContract) {
        match self {
            DataContractUpdateTransition::V0(transition) => {
                transition.data_contract = data_contract
            }
        }
    }

    pub fn state_transition_version(&self) -> u16 {
        match self {
            DataContractUpdateTransition::V0(_) => 0,
        }
    }

    /// Returns ID of the created contract
    pub fn get_modified_data_ids(&self) -> Vec<Identifier> {
        vec![self.data_contract().id]
    }

    #[cfg(feature = "platform-value")]
    pub fn clean_value(value: &mut Value) -> Result<(), platform_value::Error> {
        DataContractUpdateTransitionLatest::clean_value(value)
    }
}

#[cfg(test)]
mod test {
    use crate::util::json_value::JsonValueExt;
    use integer_encoding::VarInt;
    use std::convert::TryInto;

    use crate::data_contract::state_transition::data_contract_update_transition::property_names::STATE_TRANSITION_PROTOCOL_VERSION;
    use crate::data_contract::state_transition::property_names::TRANSITION_TYPE;
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::version::LATEST_PLATFORM_VERSION;
    use crate::{version, Convertible};

    use super::*;

    struct TestData {
        state_transition: DataContractUpdateTransition,
        data_contract: DataContract,
    }

    fn get_test_data() -> TestData {
        let data_contract = get_data_contract_fixture(None).data_contract;

        let value_map = BTreeMap::from([
            (
                STATE_TRANSITION_PROTOCOL_VERSION.to_string(),
                Value::U16(
                    LATEST_PLATFORM_VERSION
                        .state_transitions
                        .contract_create_state_transition
                        .default_current_version,
                ),
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
            LATEST_PLATFORM_VERSION
                .state_transitions
                .contract_update_state_transition
                .default_current_version,
            data.state_transition.state_transition_protocol_version()
        )
    }

    #[test]
    fn should_return_transition_type() {
        let data = get_test_data();
        assert_eq!(
            StateTransitionType::DataContractUpdate,
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
                .get_u64(STATE_TRANSITION_PROTOCOL_VERSION)
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
