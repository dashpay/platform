
use crate::data_contract::serialized_version::DataContractInSerializationFormat;

pub trait DataContractUpdateTransitionAccessorsV0 {
    fn data_contract(&self) -> &DataContractInSerializationFormat;
    fn set_data_contract(&mut self, data_contract: DataContractInSerializationFormat);
}
