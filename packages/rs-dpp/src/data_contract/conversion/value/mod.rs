pub mod v0;

use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::v1::DataContractV1;
use crate::data_contract::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

impl DataContractValueConversionMethodsV0 for DataContract {
    fn from_value_validated(
        raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(DataContractV0::from_value_validated(raw_object, platform_version)?.into()),
            1 => Ok(DataContractV1::from_value_validated(raw_object, platform_version)?.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_value_validated".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
