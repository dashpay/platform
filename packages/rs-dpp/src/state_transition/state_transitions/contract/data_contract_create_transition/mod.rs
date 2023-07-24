mod fields;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod serialize;
mod state_transition_like;
mod v0;
mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;

use fields::*;

use crate::data_contract::property_names::ENTROPY;

use crate::data_contract::DataContract;
use crate::identity::KeyID;
use crate::serialization_traits::PlatformDeserializable;
use crate::serialization_traits::{PlatformSerializable, Signable};
use crate::state_transition::{
    StateTransitionFieldTypes, StateTransitionLike, StateTransitionType,
};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::{BinaryData, Bytes32, Identifier, Value};
use platform_versioning::{PlatformSerdeVersionedDeserialize, PlatformVersioned};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use v0::*;
use crate::data_contract::created_data_contract::CreatedDataContract;

pub type DataContractCreateTransitionLatest = DataContractCreateTransitionV0;

#[derive(
    Debug,
    Clone,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    Decode,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, PlatformSerdeVersionedDeserialize),
    serde(untagged)
)]
#[platform_error_type(ProtocolError)]
#[platform_serialize(platform_version_path = "state_transitions.contract_create_state_transition")]
pub enum DataContractCreateTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", versioned(0))]
    V0(DataContractCreateTransitionV0),
}

impl DataContractCreateTransition {
    pub fn try_from(value: CreatedDataContract, platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version.dpp.state_transition_serialization_versions.contract_create_state_transition.default_current_version {
            0 => {
                let data_contract_create_transition : DataContractCreateTransitionV0 = value.into();
                Ok(data_contract_create_transition.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractCreateTransition::try_from(CreatedDataContract)".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl From<DataContract> for DataContractCreateTransition {
    fn from(value: DataContract) -> Self {
        DataContractCreateTransitionV0::from(value).into()
    }
}

impl StateTransitionFieldTypes for DataContractCreateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, ENTROPY]
    }
}

impl DataContractCreateTransition {
    pub fn state_transition_version(&self) -> u16 {
        match self {
            DataContractCreateTransition::V0(_) => 0,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::data_contract::created_data_contract::CreatedDataContract;
    use integer_encoding::VarInt;
    use platform_value::Bytes32;

    use super::*;
    use crate::data_contract::conversion::platform_value_conversion::v0::DataContractValueConversionMethodsV0;
    use crate::state_transition::state_transitions::common_fields::property_names;
    use crate::state_transition::{StateTransitionType, StateTransitionValueConvert};
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::util::json_value::JsonValueExt;
    use crate::version::LATEST_PLATFORM_VERSION;
    use crate::{version, Convertible};

    pub(crate) struct TestData {
        pub(crate) state_transition: DataContractCreateTransition,
        pub(crate) created_data_contract: CreatedDataContract,
    }

    pub(crate) fn get_test_data() -> TestData {
        let created_data_contract = get_data_contract_fixture(None, 1);

        let state_transition = DataContractCreateTransition::from_object(Value::from([
            (
                STATE_TRANSITION_PROTOCOL_VERSION,
                LATEST_PLATFORM_VERSION
                    .state_transitions
                    .contract_create_state_transition
                    .default_current_version
                    .into(),
            ),
            (
                property_names::ENTROPY,
                created_data_contract.entropy_used().into(),
            ),
            (
                DATA_CONTRACT,
                created_data_contract.data_contract().to_object().unwrap(),
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
