use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0,
};
use crate::version::FeatureVersion;
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier};

pub trait DataContractCreateTransitionMethodsV0 {
    /// Creates a new instance of the DataContractCreateTransition from the provided data contract.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - A mutable `DataContract` instance, to be used in the transition.
    /// * `entropy` - A `Bytes32` value providing additional randomness.
    /// * `identity` - A reference to a `PartialIdentity` object.
    /// * `key_id` - A `KeyID` identifier for the public key used for signing the transition.
    /// * `signer` - A reference to an object implementing the `Signer` trait.
    ///
    /// # Returns
    ///
    /// If successful, returns a `Result<Self, ProtocolError>` containing a `DataContractCreateTransition`
    /// object. Otherwise, returns `ProtocolError`.
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        entropy: Bytes32,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        version: FeatureVersion,
    ) -> Result<DataContractCreateTransition, ProtocolError>;

    fn modified_data_ids(&self) -> Vec<Identifier>;
}
