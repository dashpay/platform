use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::property_names;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::TryFromPlatformVersioned;

impl DataContractValueConversionMethodsV0 for DataContractV0 {
    fn from_object(
        mut raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        raw_object
            .remove(property_names::SCHEMA)
            .map_err(ProtocolError::ValueError)?;

        let data_contract_data: DataContractInSerializationFormatV0 =
            platform_value::from_value(raw_object).map_err(ProtocolError::ValueError)?;

        DataContractV0::try_from_platform_versioned(data_contract_data, platform_version)
    }

    fn to_object(&self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError> {
        let data_contract_data =
            DataContractInSerializationFormat::try_from_platform_versioned(self, platform_version)?;

        let value =
            platform_value::to_value(data_contract_data).map_err(ProtocolError::ValueError)?;

        Ok(value)
    }

    fn into_object(self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError> {
        let data_contract_data =
            DataContractInSerializationFormat::try_from_platform_versioned(self, platform_version)?;

        let value =
            platform_value::to_value(data_contract_data).map_err(ProtocolError::ValueError)?;

        Ok(value)
    }
}
