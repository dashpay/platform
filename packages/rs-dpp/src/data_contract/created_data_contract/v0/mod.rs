use crate::data_contract::DataContract;
use bincode::{Decode, Encode};

#[cfg(feature = "data-contract-value-conversion")]
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::created_data_contract::fields::property_names::{DATA_CONTRACT, ENTROPY};
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Bytes32, Error, Value};

// TODO: Decide on what we need ExtendedDataContract with metadata or CreatedDataContract or both.
#[derive(Clone, Debug, PartialEq)]
pub struct CreatedDataContractV0 {
    pub data_contract: DataContract,
    pub entropy_used: Bytes32,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct CreatedDataContractInSerializationFormatV0 {
    pub data_contract: DataContractInSerializationFormat,
    pub entropy_used: Bytes32,
}

impl CreatedDataContractV0 {
    #[cfg(feature = "data-contract-value-conversion")]
    pub fn from_object(
        raw_object: Value,
        validate: bool,
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

        let data_contract =
            DataContract::from_value(raw_data_contract, validate, platform_version)?;

        Ok(Self {
            data_contract,
            entropy_used,
        })
    }
}
