use crate::data_contract::DataContract;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Bytes32, Error, Value};

#[derive(Clone, Debug)]
pub struct CreatedDataContract {
    pub data_contract: DataContract,
    pub entropy_used: Bytes32,
}

impl CreatedDataContract {
    pub fn from_raw_object(raw_object: Value) -> Result<Self, ProtocolError> {
        let mut raw_map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let raw_data_contract = raw_map.remove(st_prop::DATA_CONTRACT).ok_or_else(|| {
            Error::StructureError("unable to remove property dataContract".to_string())
        })?;

        let raw_entropy = raw_map
            .remove_bytes(st_prop::ENTROPY)
            .map_err(ProtocolError::ValueError)?;

        let data_contract = DataContract::from_raw_object(raw_data_contract)?;

        Ok(Self {
            data_contract,
            entropy_used: Bytes32::from_vec(raw_entropy)?,
        })
    }
}
