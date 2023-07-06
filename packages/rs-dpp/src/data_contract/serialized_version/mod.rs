use bincode::{Decode, Encode};
use crate::data_contract::serialized_version::v0::DataContractSerializationFormatV0;

mod v0;

#[derive(Encode, Decode)]
pub enum DataContractSerializationFormat {
    V0(DataContractSerializationFormatV0)
}