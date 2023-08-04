use crate::serialization::PlatformDeserializable;
use crate::serialization::PlatformSerializable;
use crate::serialization::Signable;
use crate::state_transition::{
    StateTransitionFieldTypes, StateTransitionLike, StateTransitionType,
};
use crate::{Convertible, ProtocolError};
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::{BinaryData, Identifier, Value};
use platform_versioning::{PlatformSerdeVersionedDeserialize, PlatformVersioned};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod accessors;
mod fields;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
pub mod methods;
mod serialize;
mod state_transition_like;
mod v0;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

pub use fields::*;
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};
use crate::version::PlatformVersionCurrentVersion;
use crate::state_transition::StateTransitionValueConvert;

use crate::data_contract::DataContract;
use crate::version::FeatureVersion;
pub use v0::*;

pub type DataContractUpdateTransitionLatest = DataContractUpdateTransitionV0;

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, PlatformSerdeVersionedDeserialize),
    serde(untagged)
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.contract_update_state_transition"
)]
pub enum DataContractUpdateTransition {
    #[cfg_attr(feature = "state-transition-serde-conversion", versioned(0))]
    V0(DataContractUpdateTransitionV0),
}

impl TryFromPlatformVersioned<DataContract> for DataContractUpdateTransition {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .state_transition_serialization_versions
            .contract_update_state_transition
            .default_current_version
        {
            0 => {
                let data_contract_update_transition: DataContractUpdateTransitionV0 =
                    value.try_into_platform_versioned(platform_version)?;
                Ok(data_contract_update_transition.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractUpdateTransition::try_from_platform_versioned(DataContract)"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
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
}

#[cfg(test)]
mod test {
    use crate::util::json_value::JsonValueExt;
    use integer_encoding::VarInt;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::convert::TryInto;

    use crate::data_contract::conversion::json_conversion::DataContractJsonConversionMethodsV0;
    use crate::data_contract::DataContract;
    use crate::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
    use crate::state_transition::{
        JsonStateTransitionSerializationOptions, StateTransitionJsonConvert,
        StateTransitionValueConvert,
    };
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::version::LATEST_PLATFORM_VERSION;
    use crate::{version, Convertible};

    use super::*;

    struct TestData {
        state_transition: DataContractUpdateTransition,
        data_contract: DataContract,
    }

    fn get_test_data() -> TestData {
        let platform_version = PlatformVersion::first();
        let data_contract = get_data_contract_fixture(None, platform_version.protocol_version)
            .data_contract_owned();

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

        let state_transition =
            DataContractUpdateTransition::from_value_map(value_map, platform_version)
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
            .to_json(JsonStateTransitionSerializationOptions {
                skip_signature: false,
                into_validating_json: false,
            })
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
            data.state_transition.owner_id()
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
