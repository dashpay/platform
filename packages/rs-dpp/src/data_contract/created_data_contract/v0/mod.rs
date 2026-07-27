use crate::data_contract::DataContract;
use bincode::{Decode, Encode};

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::prelude::IdentityNonce;

// TODO: Decide on what we need ExtendedDataContract with metadata or CreatedDataContract or both.
#[derive(Clone, Debug, PartialEq)]
pub struct CreatedDataContractV0 {
    pub data_contract: DataContract,
    pub identity_nonce: IdentityNonce,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct CreatedDataContractInSerializationFormatV0 {
    pub data_contract: DataContractInSerializationFormat,
    pub identity_nonce: IdentityNonce,
}
