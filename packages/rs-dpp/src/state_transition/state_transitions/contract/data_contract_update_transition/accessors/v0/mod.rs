use crate::data_contract::DataContract;

pub trait DataContractUpdateTransitionAccessorsV0 {
    fn data_contract(&self) -> &DataContract;
    fn set_data_contract(&mut self, data_contract: DataContract);
}
