use crate::data_contract::DataContract;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::serialization::Signable;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0,
};
use crate::state_transition::StateTransitionLike;
use crate::version::FeatureVersion;
use crate::{NonConsensusError, ProtocolError};

impl DataContractUpdateTransitionMethodsV0 for DataContractUpdateTransitionV0 {
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
}
