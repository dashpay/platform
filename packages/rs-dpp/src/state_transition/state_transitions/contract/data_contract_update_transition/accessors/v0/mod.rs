use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::prelude::IdentityNonce;

pub trait DataContractUpdateTransitionAccessorsV0 {
    /// Returns the data contract for V0 transitions, None for V1+
    fn data_contract(&self) -> Option<&DataContractInSerializationFormat>;
    /// Sets the data contract for V0 transitions, returns false for V1+
    fn set_data_contract(&mut self, data_contract: DataContractInSerializationFormat) -> bool;

    fn identity_contract_nonce(&self) -> IdentityNonce;
}
