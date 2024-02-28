use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::prelude::IdentityNonce;

pub trait DataContractCreateTransitionAccessorsV0 {
    fn data_contract(&self) -> &DataContractInSerializationFormat;

    fn identity_nonce(&self) -> IdentityNonce;

    fn set_data_contract(&mut self, data_contract: DataContractInSerializationFormat);
}
