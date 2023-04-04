use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::identity::signer::Signer;
use crate::identity::{KeyID, PartialIdentity};
use crate::prelude::DataContract;
use crate::state_transition::StateTransitionConvert;
use crate::state_transition::StateTransitionType::DataContractCreate;
use crate::version::LATEST_VERSION;
use crate::{NonConsensusError, ProtocolError};

impl DataContractCreateTransition {
    pub fn new_from_data_contract<S: Signer>(
        data_contract: DataContract,
        identity: &PartialIdentity,
        key_id: KeyID,
        signer: &S,
    ) -> Result<Self, ProtocolError> {
        let mut transition = DataContractCreateTransition {
            protocol_version: LATEST_VERSION,
            transition_type: DataContractCreate,
            data_contract,
            entropy: Default::default(),
            signature_public_key_id: key_id,
            signature: Default::default(),
        };
        let value = transition.to_cleaned_object(true)?;
        let public_key =
            identity
                .loaded_public_keys
                .get(&key_id)
                .ok_or(ProtocolError::NonConsensusError(
                    NonConsensusError::StateTransitionCreationError(
                        "public key did not exist".to_string(),
                    ),
                ))?;
        transition.signature = signer.sign(public_key, &value)?.into();
        Ok(transition)
    }
}
