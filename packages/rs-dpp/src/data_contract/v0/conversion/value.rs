use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::serialized_version::v0::{
    property_names, DataContractInSerializationFormatV0,
};
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};
use platform_version::TryFromPlatformVersioned;

pub const DATA_CONTRACT_IDENTIFIER_FIELDS_V0: [&str; 2] =
    [property_names::ID, property_names::OWNER_ID];

impl DataContractValueConversionMethodsV0 for DataContractV0 {
    fn from_value(
        mut value: Value,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        value.replace_at_paths(
            DATA_CONTRACT_IDENTIFIER_FIELDS_V0,
            ReplacementType::Identifier,
        )?;
        let format_version = value.get_str("$format_version")?;
        match format_version {
            "0" => {
                let data_contract_data: DataContractInSerializationFormatV0 =
                    platform_value::from_value(value).map_err(ProtocolError::ValueError)?;

                DataContractV0::try_from_platform_versioned(
                    data_contract_data.into(),
                    validate,
                    &mut vec![], // this is not used in consensus code
                    platform_version,
                )
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractV0::from_value".to_string(),
                known_versions: vec![0],
                received: version
                    .parse()
                    .map_err(|_| ProtocolError::Generic("Conversion error".to_string()))?,
            }),
        }
    }

    fn to_value(&self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError> {
        let data_contract_data =
            DataContractInSerializationFormat::try_from_platform_versioned(self, platform_version)?;

        let value =
            platform_value::to_value(data_contract_data).map_err(ProtocolError::ValueError)?;

        Ok(value)
    }

    fn into_value(self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError> {
        let data_contract_data =
            DataContractInSerializationFormat::try_from_platform_versioned(self, platform_version)?;

        let value =
            platform_value::to_value(data_contract_data).map_err(ProtocolError::ValueError)?;

        Ok(value)
    }
}
