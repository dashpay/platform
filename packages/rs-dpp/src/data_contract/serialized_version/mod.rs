use crate::data_contract::serialized_version::v0::DataContractSerializationFormatV0;
use bincode::{Decode, Encode};

mod v0;

#[derive(Encode, Decode)]
pub enum DataContractSerializationFormat {
    V0(DataContractSerializationFormatV0),
}
