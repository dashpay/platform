use crate::data_contract::generate_data_contract_id;
use crate::data_contract::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0,
};
use crate::data_contract::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0,
};
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::prelude::DataContract;
use crate::serialization_traits::Signable;
use crate::state_transition::StateTransitionLike;
use crate::{NonConsensusError, ProtocolError};

impl DataContractCreateTransition {
    pub fn new_from_data_contract<S: Signer>(
        mut data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
    ) -> Result<Self, ProtocolError> {
        data_contract.owner_id = identity.id;
        data_contract.id = generate_data_contract_id(identity.id, data_contract.entropy);
        let mut transition = DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
            data_contract,
            entropy: Default::default(),
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

impl DataContractUpdateTransition {
    pub fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
    ) -> Result<Self, ProtocolError> {
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
