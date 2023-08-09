use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::DataContract;

#[cfg(feature = "data-contract-value-conversions")]
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::created_data_contract::fields::property_names::{DATA_CONTRACT, ENTROPY};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Bytes32, Error, Value};

// TODO: Decide on what we need ExtendedDataContract with metadata or CreatedDataContract or both.
#[derive(Clone, Debug)]
pub struct CreatedDataContractV0 {
    pub data_contract: DataContract,
    pub entropy_used: Bytes32,
}

impl CreatedDataContractV0 {
    #[cfg(feature = "data-contract-value-conversion")]
    pub fn from_object(
        raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut raw_map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let raw_data_contract = raw_map.remove(DATA_CONTRACT).ok_or_else(|| {
            Error::StructureError("unable to remove property dataContract".to_string())
        })?;

        let entropy_used = raw_map
            .remove_bytes_32(ENTROPY)
            .map_err(ProtocolError::ValueError)?;

        let data_contract = DataContract::from_object(raw_data_contract, platform_version)?;

        Ok(Self {
            data_contract,
            entropy_used,
        })
    }
}
