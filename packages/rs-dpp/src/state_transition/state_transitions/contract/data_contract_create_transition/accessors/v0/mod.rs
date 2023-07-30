use crate::data_contract::DataContract;
use platform_value::Bytes32;

pub trait DataContractCreateTransitionAccessorsV0 {
    fn data_contract(&self) -> &DataContract;

    fn entropy(&self) -> &Bytes32;

    fn set_data_contract(&mut self, data_contract: DataContract);
}
