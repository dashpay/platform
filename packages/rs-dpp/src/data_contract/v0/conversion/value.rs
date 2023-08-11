use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::v0::conversion::json::DATA_CONTRACT_IDENTIFIER_FIELDS_V0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};
use platform_version::TryFromPlatformVersioned;

impl DataContractValueConversionMethodsV0 for DataContractV0 {
    fn from_value(
        mut value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        value.replace_at_paths(
            DATA_CONTRACT_IDENTIFIER_FIELDS_V0,
            ReplacementType::Identifier,
        )?;

        let data_contract_data: DataContractInSerializationFormatV0 =
            platform_value::from_value(value).map_err(ProtocolError::ValueError)?;

        DataContractV0::try_from_platform_versioned(data_contract_data, platform_version)
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
