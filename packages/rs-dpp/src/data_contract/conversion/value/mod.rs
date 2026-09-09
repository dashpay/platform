pub mod v0;

use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::v1::DataContractV1;
use crate::data_contract::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

impl DataContractValueConversionMethodsV0 for DataContract {
    fn from_value(
        raw_object: Value,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(
                DataContractV0::from_value(raw_object, full_validation, platform_version)?.into(),
            ),
            1 => Ok(
                DataContractV1::from_value(raw_object, full_validation, platform_version)?.into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_value".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Pins for the explicit-version `to_value` path: the serialization format
    //! must be selected by the *passed* platform version, never by the
    //! process-global current version (which parallel tests may mutate — see
    //! the serde module doc for the race this guards against).
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::tests::fixtures::get_data_contract_fixture;

    /// A protocol version whose contract serialization default is format 0
    /// (frozen behavior; latest defaults to format 1).
    const FORMAT_0_PROTOCOL_VERSION: u32 = 8;

    fn format_0_platform_version() -> &'static PlatformVersion {
        let platform_version =
            PlatformVersion::get(FORMAT_0_PROTOCOL_VERSION).expect("expected platform version 8");
        assert_eq!(
            platform_version
                .dpp
                .contract_versions
                .contract_serialization_version
                .default_current_version,
            0,
            "test premise: protocol version 8 serializes contracts in format 0"
        );
        platform_version
    }

    #[test]
    fn to_value_selects_format_by_explicit_platform_version() {
        let contract =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract()
                .clone();

        let old_value = contract
            .to_value(format_0_platform_version())
            .expect("to_value at format-0 version");
        assert_eq!(old_value.get_str("$formatVersion"), Ok("0"));

        let new_value = contract
            .to_value(PlatformVersion::latest())
            .expect("to_value at latest");
        assert_eq!(new_value.get_str("$formatVersion"), Ok("1"));
    }

    #[test]
    fn concrete_v0_and_v1_to_value_select_format_by_explicit_platform_version() {
        let old_platform_version = format_0_platform_version();

        let DataContract::V0(v0) = get_data_contract_fixture(None, 0, FORMAT_0_PROTOCOL_VERSION)
            .data_contract()
            .clone()
        else {
            panic!("fixture at protocol version 8 should be a DataContractV0");
        };
        let DataContract::V1(v1) =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
                .data_contract()
                .clone()
        else {
            panic!("fixture at latest should be a DataContractV1");
        };

        let v0_old = v0.to_value(old_platform_version).expect("v0 at format 0");
        assert_eq!(v0_old.get_str("$formatVersion"), Ok("0"));
        let v0_new = v0
            .to_value(PlatformVersion::latest())
            .expect("v0 at latest");
        assert_eq!(v0_new.get_str("$formatVersion"), Ok("1"));

        let v1_old = v1.to_value(old_platform_version).expect("v1 at format 0");
        assert_eq!(v1_old.get_str("$formatVersion"), Ok("0"));
        let v1_new = v1
            .to_value(PlatformVersion::latest())
            .expect("v1 at latest");
        assert_eq!(v1_new.get_str("$formatVersion"), Ok("1"));
    }

    #[test]
    fn to_value_round_trips_through_from_value_at_the_same_version() {
        for platform_version in [format_0_platform_version(), PlatformVersion::latest()] {
            let original = get_data_contract_fixture(None, 0, platform_version.protocol_version)
                .data_contract()
                .clone();

            let value = original.to_value(platform_version).expect("to_value");
            let recovered = DataContract::from_value(value, true, platform_version)
                .expect("from_value at the same version");

            assert_eq!(original.id(), recovered.id());
            assert_eq!(original.owner_id(), recovered.owner_id());
            assert_eq!(original.version(), recovered.version());
        }
    }
}
