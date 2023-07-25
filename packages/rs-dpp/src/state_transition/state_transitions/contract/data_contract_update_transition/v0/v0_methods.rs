use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::serialization_traits::Signable;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0,
};
use crate::state_transition::StateTransitionLike;
use crate::version::FeatureVersion;
use crate::{NonConsensusError, ProtocolError};

pub trait DataContractUpdateTransitionV0Methods {
    /// Creates a new instance of `DataContractUpdateTransition` from the given `data_contract`.
    ///
    /// # Arguments
    ///
    /// * `data_contract` - The `DataContract` to be used in the transition.
    /// * `identity` - A reference to the `PartialIdentity` containing the public keys.
    /// * `key_id` - The `KeyID` (public key identifier) to be used for signing the transition.
    /// * `signer` - A reference to the `Signer` object that will sign the transition.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - If successful, returns an instance of `DataContractUpdateTransition`.
    ///   In case of any error, a relevant `ProtocolError` is returned.
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        version: FeatureVersion,
    ) -> Result<DataContractUpdateTransition, ProtocolError>;
    fn get_data_contract(&self) -> &DataContract;
    fn set_data_contract(&mut self, data_contract: DataContract);
}

impl DataContractUpdateTransitionV0Methods for DataContractUpdateTransitionV0 {
    fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
        _version: FeatureVersion,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        let mut transition = DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
            data_contract,
            signature_public_key_id: key_id,
            signature: Default::default(),
        });
        let value = transition.signable_bytes()?;
        let public_key =
            identity
                .loaded_public_keys
                .get(&key_id)
                .ok_or(ProtocolError::NonConsensusError(
                    NonConsensusError::StateTransitionCreationError(
                        "public key did not exist".to_string(),
                    ),
                ))?;
        transition.set_signature(signer.sign(public_key, &value)?.into());
        Ok(transition)
    }

    fn get_data_contract(&self) -> &DataContract {
        &self.data_contract
    }

    fn set_data_contract(&mut self, data_contract: DataContract) {
        self.data_contract = data_contract;
    }
}
