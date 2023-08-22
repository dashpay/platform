use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use platform_value::Bytes32;

pub trait DataContractCreateTransitionAccessorsV0 {
    fn data_contract(&self) -> &DataContractInSerializationFormat;

    fn entropy(&self) -> &Bytes32;

    fn set_data_contract(&mut self, data_contract: DataContractInSerializationFormat);
}
