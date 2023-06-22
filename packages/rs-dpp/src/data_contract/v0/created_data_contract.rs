use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::state_transition::property_names::{DATA_CONTRACT, ENTROPY};
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Bytes32, Error, Value};
use crate::data_contract::DataContract;

#[derive(Clone, Debug)]
pub struct CreatedDataContractV0 {
    pub data_contract: DataContract,
    pub entropy_used: Bytes32,
}

impl CreatedDataContractV0 {
    #[cfg(feature = "platform-value")]
    pub fn from_raw_object(raw_object: Value) -> Result<Self, ProtocolError> {
        let mut raw_map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let raw_data_contract = raw_map.remove(DATA_CONTRACT).ok_or_else(|| {
            Error::StructureError("unable to remove property dataContract".to_string())
        })?;

        let entropy_used = raw_map
            .remove_bytes_32(ENTROPY)
            .map_err(ProtocolError::ValueError)?;

        let data_contract = DataContractV0::from_raw_object(raw_data_contract)?;

        Ok(Self {
            data_contract,
            entropy_used,
        })
    }
}
